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
