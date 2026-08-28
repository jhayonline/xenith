//! `Chunk` as text.
//!
//! Register allocation bugs produce wrong answers rather than crashes, so the
//! disassembler is written before the VM loop rather than after it. Every
//! compiler test asserts on this output.

use std::fmt::Write;

use crate::values::Value;
use crate::vm::chunk::{Chunk, Instr};

impl Chunk {
    /// One line per instruction, preceded by the constants and frame size,
    /// followed by every nested body.
    pub fn disassemble(&self) -> String {
        let mut out = String::new();
        self.write_into(&mut out, "");
        out
    }

    fn write_into(&self, out: &mut String, prefix: &str) {
        // Printed only when there is one, so that a chunk with no captures
        // disassembles exactly as it did in phase 3.
        if !self.upvalues.is_empty() {
            out.push_str("upvalues:\n");
            for (i, up) in self.upvalues.iter().enumerate() {
                let _ = if up.in_parent_locals {
                    writeln!(out, "  u{}  parent local r{}", i, up.index)
                } else {
                    writeln!(out, "  u{}  parent upvalue u{}", i, up.index)
                };
            }
        }

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

        // After the code that makes them, not before: reading a disassembly
        // top to bottom should meet a proto at the `CLOSURE` that refers to
        // it, and the nesting can go deeper than a page.
        for (i, proto) in self.protos.iter().enumerate() {
            let name = proto.name.as_deref().unwrap_or("<anonymous>");
            let label = if prefix.is_empty() {
                format!("p{}", i)
            } else {
                format!("{}.{}", prefix, i)
            };
            let _ = writeln!(out, "\nproto {}  {}/{}", label, name, proto.arity);
            proto.write_into(out, &label);
        }
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

        Instr::AddI { dst, a, b } => binary("ADD_I", dst, a, b),
        Instr::SubI { dst, a, b } => binary("SUB_I", dst, a, b),
        Instr::MulI { dst, a, b } => binary("MUL_I", dst, a, b),
        Instr::DivI { dst, a, b } => binary("DIV_I", dst, a, b),
        Instr::RemI { dst, a, b } => binary("REM_I", dst, a, b),
        Instr::LtI { dst, a, b } => binary("LT_I", dst, a, b),
        Instr::GtI { dst, a, b } => binary("GT_I", dst, a, b),
        Instr::LeI { dst, a, b } => binary("LE_I", dst, a, b),
        Instr::GeI { dst, a, b } => binary("GE_I", dst, a, b),
        Instr::EqI { dst, a, b } => binary("EQ_I", dst, a, b),
        Instr::NeI { dst, a, b } => binary("NE_I", dst, a, b),

        Instr::AddF { dst, a, b } => binary("ADD_F", dst, a, b),
        Instr::SubF { dst, a, b } => binary("SUB_F", dst, a, b),
        Instr::MulF { dst, a, b } => binary("MUL_F", dst, a, b),
        Instr::DivF { dst, a, b } => binary("DIV_F", dst, a, b),
        Instr::LtF { dst, a, b } => binary("LT_F", dst, a, b),
        Instr::GtF { dst, a, b } => binary("GT_F", dst, a, b),
        Instr::LeF { dst, a, b } => binary("LE_F", dst, a, b),
        Instr::GeF { dst, a, b } => binary("GE_F", dst, a, b),
        Instr::EqF { dst, a, b } => binary("EQ_F", dst, a, b),
        Instr::NeF { dst, a, b } => binary("NE_F", dst, a, b),

        Instr::AddIK { dst, a, k } => binary_k("ADD_IK", dst, a, k),
        Instr::SubIK { dst, a, k } => binary_k("SUB_IK", dst, a, k),
        Instr::MulIK { dst, a, k } => binary_k("MUL_IK", dst, a, k),
        Instr::LtIK { dst, a, k } => binary_k("LT_IK", dst, a, k),
        Instr::GtIK { dst, a, k } => binary_k("GT_IK", dst, a, k),
        Instr::LeIK { dst, a, k } => binary_k("LE_IK", dst, a, k),
        Instr::GeIK { dst, a, k } => binary_k("GE_IK", dst, a, k),
        Instr::EqIK { dst, a, k } => binary_k("EQ_IK", dst, a, k),
        Instr::NeIK { dst, a, k } => binary_k("NE_IK", dst, a, k),

        Instr::Neg { dst, src } => format!("{:<12} r{}, r{}", "NEG", dst, src),
        Instr::Not { dst, src } => format!("{:<12} r{}, r{}", "NOT", dst, src),

        Instr::Jump { to } => format!("{:<12} @{:04}", "JUMP", to),
        Instr::JumpIfFalse { cond, to } => {
            format!("{:<12} r{}, @{:04}", "JUMP_IF_FALSE", cond, to)
        }
        Instr::JumpIfTrue { cond, to } => {
            format!("{:<12} r{}, @{:04}", "JUMP_IF_TRUE", cond, to)
        }

        Instr::Closure { dst, proto } => format!("{:<12} r{}, p{}", "CLOSURE", dst, proto),
        Instr::Ret { src } => format!("{:<12} r{}", "RET", src),
        Instr::Call { dst, callee, argc } => {
            format!("{:<12} r{}, r{}, {}", "CALL", dst, callee, argc)
        }
        Instr::GetUpval { dst, idx } => format!("{:<12} r{}, u{}", "GET_UPVAL", dst, idx),
        Instr::SetUpval { idx, src } => format!("{:<12} u{}, r{}", "SET_UPVAL", idx, src),
        Instr::CloseUpvals { from } => format!("{:<12} r{}", "CLOSE_UPVALS", from),

        Instr::Echo { src } => format!("{:<12} r{}", "ECHO", src),
        Instr::Halt { src } => format!("{:<12} r{}", "HALT", src),
    }
}

fn binary(name: &str, dst: &u8, a: &u8, b: &u8) -> String {
    format!("{:<12} r{}, r{}, r{}", name, dst, a, b)
}

/// `NAME  rDST, rA, kK` -- a register and a constant.
fn binary_k(name: &str, dst: &u8, a: &u8, k: &u16) -> String {
    format!("{:<12} r{}, r{}, k{}", name, dst, a, k)
}
