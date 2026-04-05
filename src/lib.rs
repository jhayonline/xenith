#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(dead_code)]
#![allow(unused_assignments)]
//! # Xenith Programming Language
//!
//! Xenith is an interpreted programming language with a Python-like syntax.
//! This crate provides the core implementation including lexing, parsing,
//! and interpretation phases.
//!
//! ## Example
//! ```rust
//! use xenith::run;
//!
//! let result = run("test.xen", "spawn x = 5\nPRINT(x)");
//! assert!(result.is_ok());
//! ```

pub mod context;
pub mod error;
pub mod interpreter;
pub mod lexer;
pub mod nodes;
pub mod parse_result;
pub mod parser;
pub mod position;
pub mod runtime_result;
pub mod symbol_table;
pub mod tokens;
pub mod utils;
pub mod values;

use crate::error::Error;
use crate::interpreter::Interpreter;
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::values::Value;

/// Runs a Xenith program from source code.
///
/// # Arguments
/// * `filename` - The source file name (for error reporting)
/// * `source` - The source code string
///
/// # Returns
/// * `Ok(Value)` - The result of program execution
/// * `Err(Error)` - An error occurred during lexing, parsing, or runtime
pub fn run(filename: &str, source: &str) -> Result<Value, Error> {
    // Lexical analysis
    let mut lexer = Lexer::new(filename.to_string(), source.to_string());
    let tokens = match lexer.make_tokens() {
        Ok(t) => t,
        Err(e) => return Err(e.base),
    };

    // Syntax analysis
    let mut parser = Parser::new(tokens);
    let parse_result = parser.parse();

    if let Some(error) = parse_result.error {
        return Err(error);
    }

    let ast = match parse_result.node {
        Some(node) => node,
        None => {
            return Err(Error::new(
                crate::position::Position::new(0, 0, 0, filename, source),
                crate::position::Position::new(0, 0, 0, filename, source),
                "Internal Error",
                "No AST node produced",
            ));
        }
    };

    // Interpretation
    let mut interpreter = Interpreter::new();
    let mut context = crate::context::Context::new("<program>", None, None);
    context.symbol_table = interpreter.global_symbol_table.clone();

    let result = interpreter.visit(&ast, &mut context);

    if let Some(error) = result.error {
        Err(error)
    } else if let Some(value) = result.value {
        Ok(value)
    } else {
        Ok(Value::Number(crate::values::Number::null()))
    }
}
