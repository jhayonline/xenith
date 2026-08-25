//! Compiler tests, asserted against disassembly text.
//!
//! Text rather than an `Instr` vector on purpose: when one of these fails, the
//! message shows the code that was actually emitted, which is the thing you
//! need in order to see a register allocation bug.

use xenith::values::Value;
use xenith::vm::chunk::{Chunk, Instr};

#[test]
fn a_chunk_disassembles() {
    let mut chunk = Chunk::new();
    let k = chunk.add_constant(Value::int(7));
    chunk.push(Instr::LoadConst { dst: 0, k });
    chunk.push(Instr::LoadConst { dst: 1, k });
    chunk.push(Instr::Add { dst: 2, a: 0, b: 1 });
    chunk.push(Instr::Halt { src: 2 });
    chunk.registers = 3;

    assert_eq!(
        chunk.disassemble(),
        "\
constants:
  k0  int 7
registers: 3
code:
  0000  LOAD_CONST   r0, k0
  0001  LOAD_CONST   r1, k0
  0002  ADD          r2, r0, r1
  0003  HALT         r2
"
    );
}

#[test]
fn an_identical_constant_is_interned_once() {
    let mut chunk = Chunk::new();
    let a = chunk.add_constant(Value::int(7));
    let b = chunk.add_constant(Value::int(7));
    assert_eq!(a, b);
    assert_eq!(chunk.constants.len(), 1);
}

#[test]
fn an_int_and_a_float_are_not_the_same_constant() {
    // `1` and `1.0` are different values in Xenith -- mixing them is an error,
    // not a promotion -- so they must not share a constant slot.
    let mut chunk = Chunk::new();
    let i = chunk.add_constant(Value::int(1));
    let f = chunk.add_constant(Value::float(1.0));
    assert_ne!(i, f);
}

#[test]
fn a_jump_target_is_shown_as_an_address() {
    let mut chunk = Chunk::new();
    chunk.push(Instr::LoadBool { dst: 0, value: true });
    chunk.push(Instr::JumpIfFalse { cond: 0, to: 3 });
    chunk.push(Instr::Jump { to: 0 });
    chunk.push(Instr::Halt { src: 0 });
    chunk.registers = 1;

    assert_eq!(
        chunk.disassemble(),
        "\
constants:
  (none)
registers: 1
code:
  0000  LOAD_BOOL    r0, true
  0001  JUMP_IF_FALSE r0, @0003
  0002  JUMP         @0000
  0003  HALT         r0
"
    );
}
