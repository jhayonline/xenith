//! A register-based bytecode VM.
//!
//! The tree walker in `src/interpreter.rs` remains the reference
//! implementation, and remains the fallback: [`compile::compile`] returns
//! [`compile::Unsupported`] for anything this phase does not cover, and the
//! caller runs the tree walker instead. Nothing can break by failing to
//! compile -- only by compiling to something that behaves differently, which
//! is what `tests/differential.rs` exists to catch.

pub mod chunk;
pub mod closure;
pub mod compile;
pub mod disasm;
pub mod run;

use crate::error::Error;
use crate::nodes::Node;
use crate::values::Value;

/// Compiles and runs, or reports that it could not compile.
///
/// `None` means the program used something this phase does not cover, and the
/// caller should run the tree walker. It is not a failure.
pub fn compile_and_run(ast: &Node) -> Option<Result<Value, Error>> {
    let chunk = compile::compile(ast).ok()?;
    Some(run::execute(std::rc::Rc::new(chunk)))
}
