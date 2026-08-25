//! A register-based bytecode VM.
//!
//! The tree walker in `src/interpreter.rs` remains the reference
//! implementation, and remains the fallback: [`compile::compile`] returns
//! [`compile::Unsupported`] for anything this phase does not cover, and the
//! caller runs the tree walker instead. Nothing can break by failing to
//! compile -- only by compiling to something that behaves differently, which
//! is what `tests/differential.rs` exists to catch.

pub mod chunk;
pub mod compile;
pub mod disasm;
pub mod run;
