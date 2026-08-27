//! The interpreter loop.
//!
//! Registers are one contiguous `Vec<Value>`, sliced into frames: a call moves
//! the base up rather than allocating, so no instruction allocates to get at
//! its operands and no call allocates to get a frame. Every arithmetic
//! instruction
//! dispatches to the matching method on `Value`, which is the same code the
//! tree walker calls -- the two cannot disagree about what `+` means, because
//! there is one implementation of it.

use std::cell::RefCell;
use std::rc::Rc;

use crate::error::Error;
use crate::values::Value;
use crate::vm::chunk::{Chunk, Instr};
use crate::vm::closure::{Closure, Upvalue};

/// One suspended caller.
///
/// Not the running frame: that is the loop's own `current`, `base` and `ip`,
/// held in locals so the hot path never indexes a vector to find out where it
/// is.
struct Frame {
    chunk: Rc<Chunk>,
    /// The closure whose upvalues to go back to. `None` is the top level,
    /// which captures nothing.
    closure: Option<Rc<Closure>>,
    base: usize,
    /// The instruction to resume at.
    ip: usize,
    /// The absolute register the result goes to. Always below the callee's
    /// base, since the callee's frame starts one above the caller's callee
    /// register -- which is why clearing the callee's window on return cannot
    /// touch it.
    result: usize,
}

/// Runs a chunk to its `Halt` and returns the value it halted on.
pub fn execute(chunk: Rc<Chunk>) -> Result<Value, Error> {
    // One contiguous stack. A frame is a slice of it, so a call moves the
    // base rather than allocating -- which is the reason arguments are
    // compiled into the registers just above the callee.
    let mut stack: Vec<Value> = vec![Value::Null; chunk.registers as usize];
    let mut frames: Vec<Frame> = Vec::new();
    // Cells watching a register that is still live. One entry per captured
    // variable currently in scope, not one per closure.
    let mut open: Vec<Rc<RefCell<Upvalue>>> = Vec::new();

    let mut current = chunk;
    let mut current_closure: Option<Rc<Closure>> = None;
    let mut base: usize = 0;
    let mut ip: usize = 0;

    loop {
        // A chunk always ends in `Halt`, so running off the end is a compiler
        // bug rather than a program error. Reported rather than indexed out of
        // bounds, because a wrong jump target is exactly the kind of mistake
        // this phase is expected to make.
        let Some(instr) = current.code.get(ip) else {
            return Err(internal(&current, ip, "ran past the end of the chunk"));
        };
        ip += 1;
        // The address of the instruction now running, for a trap's span.
        let at = ip - 1;

        match *instr {
            Instr::LoadConst { dst, k } => {
                stack[base + dst as usize] = current.constants[k as usize].clone();
            }
            Instr::LoadBool { dst, value } => {
                stack[base + dst as usize] = Value::Bool(value);
            }
            Instr::LoadNull { dst } => {
                stack[base + dst as usize] = Value::Null;
            }
            Instr::Move { dst, src } => {
                stack[base + dst as usize] = stack[base + src as usize].clone();
            }

            Instr::Halt { src } => {
                return Ok(stack[base + src as usize].clone());
            }

            Instr::Add { dst, a, b } => binary(&mut stack, base, dst, a, b, Value::add, &current, at)?,
            Instr::Sub { dst, a, b } => binary(&mut stack, base, dst, a, b, Value::subtract, &current, at)?,
            Instr::Mul { dst, a, b } => binary(&mut stack, base, dst, a, b, Value::multiply, &current, at)?,
            Instr::Div { dst, a, b } => {
                // `visit_binary_op` checks for a zero divisor before calling
                // `Value::divide`, and reports a fuller XEN003 than `divide`'s
                // own zero branch does -- with a note and a help line. The
                // check is not inside `divide` because other callers depend on
                // the barer error, so the VM makes the same check rather than
                // moving it, and the two reports stay byte-identical.
                if let (Some(_), Some(divisor)) = (
                    stack[base + a as usize].as_number(),
                    stack[base + b as usize].as_number(),
                ) {
                    if divisor.is_zero() {
                        return Err(with_position(
                            Error::division_by_zero(
                                crate::position::Position::dummy(),
                                crate::position::Position::dummy(),
                            ),
                            &current,
                            at,
                        ));
                    }
                }
                binary(&mut stack, base, dst, a, b, Value::divide, &current, at)?
            }
            Instr::Rem { dst, a, b } => binary(&mut stack, base, dst, a, b, Value::modulo, &current, at)?,
            Instr::Pow { dst, a, b } => binary(&mut stack, base, dst, a, b, Value::power, &current, at)?,
            Instr::Eq { dst, a, b } => binary(&mut stack, base, dst, a, b, Value::equals, &current, at)?,
            Instr::Ne { dst, a, b } => binary(&mut stack, base, dst, a, b, Value::not_equals, &current, at)?,
            Instr::Lt { dst, a, b } => binary(&mut stack, base, dst, a, b, Value::less_than, &current, at)?,
            Instr::Gt { dst, a, b } => binary(&mut stack, base, dst, a, b, Value::greater_than, &current, at)?,
            Instr::Le { dst, a, b } => {
                binary(&mut stack, base, dst, a, b, Value::less_than_or_equal, &current, at)?
            }
            Instr::Ge { dst, a, b } => {
                binary(&mut stack, base, dst, a, b, Value::greater_than_or_equal, &current, at)?
            }

            Instr::Neg { dst, src } => {
                stack[base + dst as usize] = stack[base + src as usize]
                    .negative()
                    .map_err(|e| with_position(e, &current, at))?;
            }
            Instr::Not { dst, src } => {
                stack[base + dst as usize] = stack[base + src as usize]
                    .logical_not()
                    .map_err(|e| with_position(e, &current, at))?;
            }

            Instr::Echo { src } => {
                // `BuiltInFunction::echo` calls this same function. It is not
                // `utils::value_to_string`: `echo` has its own formatting
                // rules, and a second implementation of them would diverge on
                // the first nested list the differential harness met.
                crate::values::echo_line(Some(&stack[base + src as usize]));
            }

            Instr::Jump { to } => {
                ip = to as usize;
            }
            Instr::JumpIfFalse { cond, to } => {
                if !stack[base + cond as usize].is_true() {
                    ip = to as usize;
                }
            }
            Instr::JumpIfTrue { cond, to } => {
                if stack[base + cond as usize].is_true() {
                    ip = to as usize;
                }
            }

            Instr::Closure { dst, proto } => {
                let Some(proto) = current.protos.get(proto as usize).cloned() else {
                    return Err(internal(&current, at, "a closure over a proto that is not there"));
                };
                // Task 6 fills this in. A proto with no captures needs
                // nothing, which is every closure until then.
                if !proto.upvalues.is_empty() {
                    return Err(internal(&current, at, "a capture before upvalues exist"));
                }
                stack[base + dst as usize] = Value::Closure(Rc::new(Closure {
                    proto,
                    upvalues: Vec::new(),
                }));
            }

            Instr::Call { dst, callee, argc } => {
                let callee_at = base + callee as usize;
                let Value::Closure(closure) = stack[callee_at].clone() else {
                    // The compiler only emits a `CALL` for a callee it
                    // resolved to a method, and nothing else can put a
                    // non-closure there -- so this is a compiler bug, and
                    // XEN026 says so rather than inventing a message the tree
                    // walker does not have.
                    return Err(internal(&current, at, "called a value that is not a compiled method"));
                };

                let proto = Rc::clone(&closure.proto);

                // Arity, then parameter types, then depth. That is the order
                // `Function::execute` checks them in, and a VM that checked
                // depth first would answer XEN019 where the tree walker
                // answers XEN015.
                if argc != proto.arity {
                    return Err(arity_error(&proto, argc, &current, at));
                }

                let new_base = callee_at + 1;

                let needed = new_base + proto.registers as usize;
                if stack.len() < needed {
                    stack.resize(needed, Value::Null);
                }

                // The same check `Function::execute` makes, in the same
                // order, over the same `value_matches_type`. Borrowed, never
                // cloned: an argument is already in the register it will be
                // read from.
                for (i, expected) in proto.param_types.iter().enumerate() {
                    if !Value::value_matches_type(&stack[new_base + i], expected) {
                        return Err(param_type_error(
                            expected,
                            &stack[new_base + i],
                            &current,
                            at,
                        ));
                    }
                }

                // The counter the spec asks for, checked before the frame is
                // pushed. `Function::execute` tests the *caller's* depth,
                // which is the number of frames already on the stack.
                if frames.len() >= crate::context::MAX_CALL_DEPTH {
                    return Err(recursion_limit(closure.name(), &current, at));
                }

                frames.push(Frame {
                    chunk: Rc::clone(&current),
                    closure: current_closure.take(),
                    base,
                    ip,
                    result: base + dst as usize,
                });

                current = proto;
                current_closure = Some(closure);
                base = new_base;
                ip = 0;
            }

            Instr::Ret { src } => {
                // Moved out, not cloned: this is the one value the frame
                // keeps and everything else in the window is about to go.
                let value = std::mem::replace(&mut stack[base + src as usize], Value::Null);

                close_upvalues(&mut open, &mut stack, base);

                let Some(frame) = frames.pop() else {
                    return Err(internal(&current, at, "returned from the top level"));
                };

                // Clear the window. Not tidiness: a dead register holding the
                // last copy of a list keeps its refcount above one, and the
                // next `Rc::make_mut` on that list copies it. `cow_semantics`
                // is the fixture that would notice.
                let width = current.registers as usize;
                for slot in &mut stack[base..base + width] {
                    *slot = Value::Null;
                }

                stack[frame.result] = value;
                current = frame.chunk;
                current_closure = frame.closure;
                base = frame.base;
                ip = frame.ip;
            }

            // Task 6 replaces these. Nothing emits them yet, so this is
            // unreachable rather than wrong -- and XEN026 is the honest
            // answer if it ever is reached, since an instruction the loop
            // does not implement is a VM bug and not a program error.
            Instr::GetUpval { .. } | Instr::SetUpval { .. } | Instr::CloseUpvals { .. } => {
                return Err(internal(&current, at, "a capture instruction before upvalues exist"));
            }
        }
    }
}

/// Applies one of `Value`'s binary methods, in place.
fn binary(
    stack: &mut [Value],
    base: usize,
    dst: u8,
    a: u8,
    b: u8,
    op: fn(&Value, &Value) -> Result<Value, Error>,
    chunk: &Chunk,
    at: usize,
) -> Result<(), Error> {
    // Borrowed, not cloned. Two immutable borrows out of one slice are fine,
    // and the write below happens after both have ended -- so an `ADD` of two
    // ints no longer runs `Value::clone`'s match over thirteen variants twice
    // per dispatch.
    match op(&stack[base + a as usize], &stack[base + b as usize]) {
        Ok(value) => {
            stack[base + dst as usize] = value;
            Ok(())
        }
        Err(error) => Err(with_position(error, chunk, at)),
    }
}

/// Gives a positionless error the span of the instruction that raised it.
///
/// The same rule `visit_binary_op` applies, and deliberately the same
/// condition: the value-level operations in `src/values.rs` build their errors
/// with a dummy position, and only those get overwritten. An error that
/// already knows where it came from keeps its own span.
fn with_position(mut error: Error, chunk: &Chunk, at: usize) -> Error {
    if error.position_start.index == 0 && error.position_end.index == 0 {
        if let Some((start, end)) = chunk.position_at(at as u32) {
            error.position_start = start.clone();
            error.position_end = end.clone();
        }
    }
    error
}

/// A VM bug, not a program error.
///
/// Given a position from the chunk where there is one, so the report at least
/// points at the source line that produced the bad instruction.
fn internal(chunk: &Chunk, at: usize, detail: &str) -> Error {
    let (start, end) = match chunk.position_at(at as u32) {
        Some((start, end)) => (start.clone(), end.clone()),
        None => {
            let dummy = crate::position::Position::new(0, 0, 0, "<vm>", "");
            (dummy.clone(), dummy)
        }
    };

    Error::new(
        start,
        end,
        "Internal Error",
        &format!("bytecode: {}", detail),
    )
    .with_code("XEN026")
    .with_note("this is a bug in the compiler or the VM, not in the program")
    .with_help("re-run with --dump-bytecode to see the code that was emitted")
}

/// Closes every open cell watching `from` or above. Task 6 gives it a body;
/// with no captures there is never anything open.
fn close_upvalues(open: &mut Vec<Rc<RefCell<Upvalue>>>, _stack: &mut [Value], _from: usize) {
    debug_assert!(open.is_empty(), "upvalues arrive in task 6");
}

/// The span the tree walker gives an error raised at a call site.
///
/// `Function::execute` is handed `node.position_start` as `call_position` and
/// clones it into *both* ends of every error it builds -- so an arity error
/// underlines one character, and the VM has to underline the same one or the
/// report is not byte-identical.
fn call_span(chunk: &Chunk, at: usize) -> (crate::position::Position, crate::position::Position) {
    match chunk.position_at(at as u32) {
        Some((start, _)) => (start.clone(), start.clone()),
        None => (
            crate::position::Position::dummy(),
            crate::position::Position::dummy(),
        ),
    }
}

/// XEN015 or XEN016, whichever way the count is wrong.
fn arity_error(proto: &Chunk, argc: u8, chunk: &Chunk, at: usize) -> Error {
    let (start, end) = call_span(chunk, at);
    let expected = proto.arity as usize;
    let found = argc as usize;
    if found > expected {
        Error::too_many_arguments(expected, found, start, end)
    } else {
        Error::too_few_arguments(expected, found, start, end)
    }
}

/// XEN001, built from the same two strings `Function::execute` builds it
/// from: the declared type's `to_string`, and the value's type name.
fn param_type_error(
    expected: &crate::types::Type,
    found: &Value,
    chunk: &Chunk,
    at: usize,
) -> Error {
    let (start, end) = call_span(chunk, at);
    Error::type_mismatch(
        &expected.to_string(),
        &Value::get_type_name(found),
        start,
        end,
    )
}

/// XEN019, to the letter of the one `Function::execute` raises.
///
/// That one is built as a `RuntimeError`, which carries a `Context` for a
/// traceback -- and then returns `.base`, which drops the context. So what is
/// actually reported is an `Error` with this name, code, message and help and
/// nothing else, which the VM can build directly.
fn recursion_limit(name: Option<&str>, chunk: &Chunk, at: usize) -> Error {
    let (start, end) = call_span(chunk, at);
    Error::new(
        start,
        end,
        "Recursion Limit",
        &format!(
            "call depth exceeded {} while calling `{}`",
            crate::context::MAX_CALL_DEPTH,
            name.unwrap_or("<anonymous>")
        ),
    )
    .with_code("XEN019")
    .with_help("check for a missing base case in the recursion")
}
