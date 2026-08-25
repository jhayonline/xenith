//! `Chunk` as text.
//!
//! Register allocation bugs produce wrong answers rather than crashes, so the
//! disassembler is written before the VM loop rather than after it. Every
//! compiler test asserts on this output.

use std::fmt::Write;

use crate::values::Value;
use crate::vm::chunk::{Chunk, Instr};

impl Chunk {
    /// One line per instruction, preceded by the constants and frame size.
    pub fn disassemble(&self) -> String {
        let mut out = String::new();

        out.push_str("constants:\n");
        if self.constants.is_empty() {
            out.push_str("  (none)\n");
        } else {
            for (i, constant) in self.constants.iter().enumerate() {
                let _ = writeln!(out, "  k{}  {}", i, describe_constant(constant));
            }
        }

        let _ = writeln!(out, "registers: {}", self.registers);

        out.push_str("code:\n");
        for (addr, instr) in self.code.iter().enumerate() {
            let _ = writeln!(out, "  {:04}  {}", addr, describe(instr));
        }

        out
    }
}

/// A constant, with its type, so an int and a float are told apart on sight.
fn describe_constant(value: &Value) -> String {
    match value {
        Value::Int(i) => format!("int {}", i),
        Value::Float(_) => format!("float {}", value.as_number().unwrap()),
        Value::String(s) => format!("string {:?}", s.value),
        Value::Bool(b) => format!("bool {}", b),
        Value::Null => "null".to_string(),
        other => format!("{}", Value::get_type_name(other)),
    }
}

fn describe(instr: &Instr) -> String {
    // Mnemonics are padded to 12 so the operand columns line up, which is what
    // makes a register bug visible when you scan the output.
    match instr {
        Instr::LoadConst { dst, k } => format!("{:<12} r{}, k{}", "LOAD_CONST", dst, k),
        Instr::LoadBool { dst, value } => format!("{:<12} r{}, {}", "LOAD_BOOL", dst, value),
        Instr::LoadNull { dst } => format!("{:<12} r{}", "LOAD_NULL", dst),
        Instr::Move { dst, src } => format!("{:<12} r{}, r{}", "MOVE", dst, src),

        Instr::Add { dst, a, b } => binary("ADD", dst, a, b),
        Instr::Sub { dst, a, b } => binary("SUB", dst, a, b),
        Instr::Mul { dst, a, b } => binary("MUL", dst, a, b),
        Instr::Div { dst, a, b } => binary("DIV", dst, a, b),
        Instr::Rem { dst, a, b } => binary("REM", dst, a, b),
        Instr::Pow { dst, a, b } => binary("POW", dst, a, b),
        Instr::Eq { dst, a, b } => binary("EQ", dst, a, b),
        Instr::Ne { dst, a, b } => binary("NE", dst, a, b),
        Instr::Lt { dst, a, b } => binary("LT", dst, a, b),
        Instr::Gt { dst, a, b } => binary("GT", dst, a, b),
        Instr::Le { dst, a, b } => binary("LE", dst, a, b),
        Instr::Ge { dst, a, b } => binary("GE", dst, a, b),

        Instr::Neg { dst, src } => format!("{:<12} r{}, r{}", "NEG", dst, src),
        Instr::Not { dst, src } => format!("{:<12} r{}, r{}", "NOT", dst, src),

        Instr::Jump { to } => format!("{:<12} @{:04}", "JUMP", to),
        Instr::JumpIfFalse { cond, to } => {
            format!("{:<12} r{}, @{:04}", "JUMP_IF_FALSE", cond, to)
        }
        Instr::JumpIfTrue { cond, to } => {
            format!("{:<12} r{}, @{:04}", "JUMP_IF_TRUE", cond, to)
        }

        Instr::Echo { src } => format!("{:<12} r{}", "ECHO", src),
        Instr::Halt { src } => format!("{:<12} r{}", "HALT", src),
    }
}

fn binary(name: &str, dst: &u8, a: &u8, b: &u8) -> String {
    format!("{:<12} r{}, r{}, r{}", name, dst, a, b)
}
