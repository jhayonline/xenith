//! The interpreter loop.
//!
//! Registers are a flat `Vec<Value>` sized once from `Chunk::registers`, so no
//! instruction allocates to get at its operands. Every arithmetic instruction
//! dispatches to the matching method on `Value`, which is the same code the
//! tree walker calls -- the two cannot disagree about what `+` means, because
//! there is one implementation of it.

use crate::error::Error;
use crate::values::Value;
use crate::vm::chunk::{Chunk, Instr};

/// Runs a chunk to its `Halt` and returns the value it halted on.
pub fn execute(chunk: &Chunk) -> Result<Value, Error> {
    let mut registers: Vec<Value> = vec![Value::Null; chunk.registers as usize];
    let mut ip: usize = 0;

    loop {
        // A chunk always ends in `Halt`, so running off the end is a compiler
        // bug rather than a program error. Reported rather than indexed out of
        // bounds, because a wrong jump target is exactly the kind of mistake
        // this phase is expected to make.
        let Some(instr) = chunk.code.get(ip) else {
            return Err(internal(chunk, ip, "ran past the end of the chunk"));
        };
        ip += 1;
        // The address of the instruction now running, for a trap's span.
        let at = ip - 1;

        match *instr {
            Instr::LoadConst { dst, k } => {
                registers[dst as usize] = chunk.constants[k as usize].clone();
            }
            Instr::LoadBool { dst, value } => {
                registers[dst as usize] = Value::Bool(value);
            }
            Instr::LoadNull { dst } => {
                registers[dst as usize] = Value::Null;
            }
            Instr::Move { dst, src } => {
                registers[dst as usize] = registers[src as usize].clone();
            }

            Instr::Halt { src } => {
                return Ok(registers[src as usize].clone());
            }

            Instr::Add { dst, a, b } => binary(&mut registers, dst, a, b, Value::add, chunk, at)?,
            Instr::Sub { dst, a, b } => binary(&mut registers, dst, a, b, Value::subtract, chunk, at)?,
            Instr::Mul { dst, a, b } => binary(&mut registers, dst, a, b, Value::multiply, chunk, at)?,
            Instr::Div { dst, a, b } => {
                // `visit_binary_op` checks for a zero divisor before calling
                // `Value::divide`, and reports a fuller XEN003 than `divide`'s
                // own zero branch does -- with a note and a help line. The
                // check is not inside `divide` because other callers depend on
                // the barer error, so the VM makes the same check rather than
                // moving it, and the two reports stay byte-identical.
                if let (Some(_), Some(divisor)) = (
                    registers[a as usize].as_number(),
                    registers[b as usize].as_number(),
                ) {
                    if divisor.is_zero() {
                        return Err(with_position(
                            Error::division_by_zero(
                                crate::position::Position::dummy(),
                                crate::position::Position::dummy(),
                            ),
                            chunk,
                            at,
                        ));
                    }
                }
                binary(&mut registers, dst, a, b, Value::divide, chunk, at)?
            }
            Instr::Rem { dst, a, b } => binary(&mut registers, dst, a, b, Value::modulo, chunk, at)?,
            Instr::Pow { dst, a, b } => binary(&mut registers, dst, a, b, Value::power, chunk, at)?,
            Instr::Eq { dst, a, b } => binary(&mut registers, dst, a, b, Value::equals, chunk, at)?,
            Instr::Ne { dst, a, b } => binary(&mut registers, dst, a, b, Value::not_equals, chunk, at)?,
            Instr::Lt { dst, a, b } => binary(&mut registers, dst, a, b, Value::less_than, chunk, at)?,
            Instr::Gt { dst, a, b } => binary(&mut registers, dst, a, b, Value::greater_than, chunk, at)?,
            Instr::Le { dst, a, b } => {
                binary(&mut registers, dst, a, b, Value::less_than_or_equal, chunk, at)?
            }
            Instr::Ge { dst, a, b } => {
                binary(&mut registers, dst, a, b, Value::greater_than_or_equal, chunk, at)?
            }

            Instr::Neg { dst, src } => {
                registers[dst as usize] = registers[src as usize]
                    .negative()
                    .map_err(|e| with_position(e, chunk, at))?;
            }
            Instr::Not { dst, src } => {
                registers[dst as usize] = registers[src as usize]
                    .logical_not()
                    .map_err(|e| with_position(e, chunk, at))?;
            }

            Instr::Echo { src } => {
                // `BuiltInFunction::echo` calls this same function. It is not
                // `utils::value_to_string`: `echo` has its own formatting
                // rules, and a second implementation of them would diverge on
                // the first nested list the differential harness met.
                crate::values::echo_line(Some(&registers[src as usize]));
            }

            Instr::Jump { to } => {
                ip = to as usize;
            }
            Instr::JumpIfFalse { cond, to } => {
                if !registers[cond as usize].is_true() {
                    ip = to as usize;
                }
            }
            Instr::JumpIfTrue { cond, to } => {
                if registers[cond as usize].is_true() {
                    ip = to as usize;
                }
            }

            // Tasks 5 and 6 replace these. Nothing emits them yet, so this is
            // unreachable rather than wrong -- and XEN026 is the honest
            // answer if it ever is reached, since an instruction the loop
            // does not implement is a VM bug and not a program error.
            Instr::Closure { .. } | Instr::Ret { .. } => {
                return Err(internal(chunk, at, "a call instruction before frames exist"));
            }
        }
    }
}

/// Applies one of `Value`'s binary methods, in place.
fn binary(
    registers: &mut [Value],
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
    match op(&registers[a as usize], &registers[b as usize]) {
        Ok(value) => {
            registers[dst as usize] = value;
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
