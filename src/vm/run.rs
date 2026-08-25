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

            Instr::Add { dst, a, b } => binary(&mut registers, dst, a, b, Value::add)?,
            Instr::Sub { dst, a, b } => binary(&mut registers, dst, a, b, Value::subtract)?,
            Instr::Mul { dst, a, b } => binary(&mut registers, dst, a, b, Value::multiply)?,
            Instr::Div { dst, a, b } => binary(&mut registers, dst, a, b, Value::divide)?,
            Instr::Rem { dst, a, b } => binary(&mut registers, dst, a, b, Value::modulo)?,
            Instr::Pow { dst, a, b } => binary(&mut registers, dst, a, b, Value::power)?,
            Instr::Eq { dst, a, b } => binary(&mut registers, dst, a, b, Value::equals)?,
            Instr::Ne { dst, a, b } => binary(&mut registers, dst, a, b, Value::not_equals)?,
            Instr::Lt { dst, a, b } => binary(&mut registers, dst, a, b, Value::less_than)?,
            Instr::Gt { dst, a, b } => binary(&mut registers, dst, a, b, Value::greater_than)?,
            Instr::Le { dst, a, b } => {
                binary(&mut registers, dst, a, b, Value::less_than_or_equal)?
            }
            Instr::Ge { dst, a, b } => {
                binary(&mut registers, dst, a, b, Value::greater_than_or_equal)?
            }

            Instr::Neg { dst, src } => {
                registers[dst as usize] = registers[src as usize].negative()?;
            }
            Instr::Not { dst, src } => {
                registers[dst as usize] = registers[src as usize].logical_not()?;
            }

            // Task 5 adds the operators, task 7 `Echo`, tasks 8-10 the jumps.
            other => {
                return Err(internal(
                    chunk,
                    ip - 1,
                    &format!("{:?} is not implemented", other),
                ));
            }
        }
    }
}

/// Applies one of `Value`'s binary methods, in place.
///
/// Cloning both operands is deliberate for now: `Value` is 16 bytes and a
/// clone of an int or a float is a copy, while a string or a list is a
/// refcount bump. Borrowing both out of one `Vec` needs a split, which is
/// worth measuring in phase 5 rather than assuming here.
fn binary(
    registers: &mut [Value],
    dst: u8,
    a: u8,
    b: u8,
    op: fn(&Value, &Value) -> Result<Value, Error>,
) -> Result<(), Error> {
    let left = registers[a as usize].clone();
    let right = registers[b as usize].clone();
    registers[dst as usize] = op(&left, &right)?;
    Ok(())
}

/// A VM bug, not a program error.
///
/// Given a position from the chunk where there is one, so the report at least
/// points at the source line that produced the bad instruction.
fn internal(chunk: &Chunk, at: usize, detail: &str) -> Error {
    let position = chunk
        .position_at(at as u32)
        .cloned()
        .unwrap_or_else(|| crate::position::Position::new(0, 0, 0, "<vm>", ""));

    Error::new(
        position.clone(),
        position,
        "Internal Error",
        &format!("bytecode: {}", detail),
    )
    .with_code("XEN026")
    .with_note("this is a bug in the compiler or the VM, not in the program")
    .with_help("re-run with --dump-bytecode to see the code that was emitted")
}
