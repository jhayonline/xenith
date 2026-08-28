//! Typed opcodes, each against the generic opcode it specialises.
//!
//! Every test here builds two chunks that differ in one instruction and
//! asserts they agree. That is the property the whole phase rests on: a typed
//! opcode is an optimisation, so it must be indistinguishable from its generic
//! twin on every input, including the inputs that fail.

use std::rc::Rc;

use xenith::values::Value;
use xenith::vm::chunk::{Chunk, Instr};
use xenith::vm::run::execute;

/// Builds `dst = a <op> b` over two constants and runs it.
fn apply(op: fn(u8, u8, u8) -> Instr, a: Value, b: Value) -> Result<Value, xenith::error::Error> {
    let mut chunk = Chunk::new();
    let ka = chunk.add_constant(a);
    let kb = chunk.add_constant(b);
    chunk.push(Instr::LoadConst { dst: 0, k: ka });
    chunk.push(Instr::LoadConst { dst: 1, k: kb });
    chunk.push(op(2, 0, 1));
    chunk.push(Instr::Halt { src: 2 });
    chunk.registers = 3;
    execute(Rc::new(chunk))
}

/// The typed opcode and its generic twin must be indistinguishable.
fn agree(
    typed: fn(u8, u8, u8) -> Instr,
    generic: fn(u8, u8, u8) -> Instr,
    a: Value,
    b: Value,
) {
    match (apply(typed, a.clone(), b.clone()), apply(generic, a, b)) {
        (Ok(t), Ok(g)) => assert_eq!(
            format!("{:?}", t),
            format!("{:?}", g),
            "typed and generic disagreed on the value"
        ),
        (Err(t), Err(g)) => {
            assert_eq!(t.code, g.code, "error code");
            assert_eq!(t.details, g.details, "error details");
            assert_eq!(t.note, g.note, "error note");
            assert_eq!(t.help, g.help, "error help");
        }
        (t, g) => panic!("one succeeded and the other did not: {:?} / {:?}", t, g),
    }
}

#[test]
fn int_arithmetic_matches_the_generic_opcode() {
    for (typed, generic) in [
        (
            (|dst, a, b| Instr::AddI { dst, a, b }) as fn(u8, u8, u8) -> Instr,
            (|dst, a, b| Instr::Add { dst, a, b }) as fn(u8, u8, u8) -> Instr,
        ),
        (
            |dst, a, b| Instr::SubI { dst, a, b },
            |dst, a, b| Instr::Sub { dst, a, b },
        ),
        (
            |dst, a, b| Instr::MulI { dst, a, b },
            |dst, a, b| Instr::Mul { dst, a, b },
        ),
    ] {
        agree(typed, generic, Value::int(7), Value::int(3));
        agree(typed, generic, Value::int(-7), Value::int(3));
        agree(typed, generic, Value::int(0), Value::int(0));
        // Overflow: the same XEN017, with the same wording per operation.
        agree(typed, generic, Value::int(i64::MAX), Value::int(i64::MAX));
        agree(typed, generic, Value::int(i64::MIN), Value::int(i64::MAX));
    }
}

#[test]
fn int_division_matches_the_generic_opcode() {
    agree(
        |dst, a, b| Instr::DivI { dst, a, b },
        |dst, a, b| Instr::Div { dst, a, b },
        Value::int(7),
        Value::int(3),
    );
    agree(
        |dst, a, b| Instr::DivI { dst, a, b },
        |dst, a, b| Instr::Div { dst, a, b },
        Value::int(-7),
        Value::int(2),
    );
    // The zero divisor is the case that has two different reports in the
    // codebase. The VM's own XEN003, with a note and a help line, is the one
    // both opcodes must give.
    agree(
        |dst, a, b| Instr::DivI { dst, a, b },
        |dst, a, b| Instr::Div { dst, a, b },
        Value::int(7),
        Value::int(0),
    );
    agree(
        |dst, a, b| Instr::RemI { dst, a, b },
        |dst, a, b| Instr::Rem { dst, a, b },
        Value::int(7),
        Value::int(3),
    );
    agree(
        |dst, a, b| Instr::RemI { dst, a, b },
        |dst, a, b| Instr::Rem { dst, a, b },
        Value::int(7),
        Value::int(0),
    );
    // MIN / -1 is the one overflowing division.
    agree(
        |dst, a, b| Instr::DivI { dst, a, b },
        |dst, a, b| Instr::Div { dst, a, b },
        Value::int(i64::MIN),
        Value::int(-1),
    );
}

#[test]
fn int_comparison_matches_the_generic_opcode() {
    for (typed, generic) in [
        (
            (|dst, a, b| Instr::LtI { dst, a, b }) as fn(u8, u8, u8) -> Instr,
            (|dst, a, b| Instr::Lt { dst, a, b }) as fn(u8, u8, u8) -> Instr,
        ),
        (
            |dst, a, b| Instr::GtI { dst, a, b },
            |dst, a, b| Instr::Gt { dst, a, b },
        ),
        (
            |dst, a, b| Instr::LeI { dst, a, b },
            |dst, a, b| Instr::Le { dst, a, b },
        ),
        (
            |dst, a, b| Instr::GeI { dst, a, b },
            |dst, a, b| Instr::Ge { dst, a, b },
        ),
        (
            |dst, a, b| Instr::EqI { dst, a, b },
            |dst, a, b| Instr::Eq { dst, a, b },
        ),
        (
            |dst, a, b| Instr::NeI { dst, a, b },
            |dst, a, b| Instr::Ne { dst, a, b },
        ),
    ] {
        agree(typed, generic, Value::int(1), Value::int(2));
        agree(typed, generic, Value::int(2), Value::int(1));
        agree(typed, generic, Value::int(2), Value::int(2));
        agree(typed, generic, Value::int(i64::MIN), Value::int(i64::MAX));
    }
}

#[test]
fn a_typed_opcode_on_the_wrong_type_falls_back() {
    // This is the safety property of the phase. `ADD_I` is emitted on the
    // strength of a TypeTable entry; if that entry were ever wrong, the
    // instruction meets values it did not expect. It must then do exactly what
    // the generic opcode does -- concatenate two strings, and raise the same
    // XEN001 on a mixed pair -- rather than trap or answer wrongly.
    agree(
        |dst, a, b| Instr::AddI { dst, a, b },
        |dst, a, b| Instr::Add { dst, a, b },
        Value::string("ab"),
        Value::string("cd"),
    );
    agree(
        |dst, a, b| Instr::AddI { dst, a, b },
        |dst, a, b| Instr::Add { dst, a, b },
        Value::int(1),
        Value::float(1.0),
    );
    agree(
        |dst, a, b| Instr::LtI { dst, a, b },
        |dst, a, b| Instr::Lt { dst, a, b },
        Value::string("a"),
        Value::string("b"),
    );
    // A float pair through an int opcode: the fallback must find the float
    // arm of the generic operation, not invent an int one.
    agree(
        |dst, a, b| Instr::MulI { dst, a, b },
        |dst, a, b| Instr::Mul { dst, a, b },
        Value::float(1.5),
        Value::float(2.0),
    );
    // Division by zero through the fallback still raises the caller's XEN003.
    agree(
        |dst, a, b| Instr::DivI { dst, a, b },
        |dst, a, b| Instr::Div { dst, a, b },
        Value::float(1.0),
        Value::float(0.0),
    );
}

#[test]
fn the_mnemonics_disassemble() {
    let mut chunk = Chunk::new();
    chunk.push(Instr::AddI { dst: 0, a: 1, b: 2 });
    chunk.push(Instr::LtI { dst: 0, a: 1, b: 2 });
    chunk.push(Instr::Halt { src: 0 });
    chunk.registers = 3;

    assert_eq!(
        chunk.disassemble(),
        "\
constants:
  (none)
registers: 3
code:
  0000  ADD_I        r0, r1, r2
  0001  LT_I         r0, r1, r2
  0002  HALT         r0
"
    );
}
