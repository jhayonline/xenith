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
//! let result = run("example.xen", "let x: int = 5\necho(\"{x}\")");
//! assert!(result.is_ok());
//! ```

pub mod builtins;
pub mod checker;
pub mod context;
pub mod entry;
pub mod error;
pub mod fxhash;
pub mod interpreter;
pub mod lexer;
pub mod modules;
pub mod nodes;
pub mod parse_result;
pub mod parser;
pub mod position;
pub mod program;
pub mod repl;
pub mod runtime_result;
pub mod stdlib;
pub mod symbol_table;
pub mod tokens;
pub mod type_table;
pub mod types;
pub mod utils;
pub mod values;
pub mod vm;

use crate::context::Context;
use crate::error::Error;
use crate::interpreter::Interpreter;
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::values::Value;

use std::rc::Rc;

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
    match crate::program::build(filename, source) {
        Ok(graph) => run_with_graph(filename, source, Some(&graph)),

        // Errors in *this* file are this file's to report: they were found with
        // the imported signatures in hand, which is the whole point, and they
        // already carry positions here.
        Err(crate::modules::ModuleError::Failed { module, mut errors })
            if module == filename && !errors.is_empty() =>
        {
            Err(errors.remove(0))
        }

        // Anything else -- a missing module, a cycle, a module with its own
        // errors -- is left to the interpreter, which reports it from the
        // `grab` that caused it, with that statement's position and the
        // module's own note. Rebuilding those here would mean doing it without
        // a node to point at.
        Err(_) => run_with_graph(filename, source, None),
    }
}

/// [`run`], for a caller that has already built the module graph.
///
/// Building it parses and checks every module the program reaches, which is not
/// free -- `std::json` alone is a few hundred lines. The CLI needs the graph
/// before it runs anything, to report every static error at once, so this lets
/// it hand over the one it has rather than paying for a second.
pub fn run_with_graph(
    filename: &str,
    source: &str,
    graph: Option<&crate::program::ModuleGraph>,
) -> Result<Value, Error> {

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

    // Static checking, before anything runs. Callers that want every error at
    // once should use `check_source` first; this only surfaces the first, so
    // that embedding `run` stays a simple Result.
    // Every module the program needs, dependencies first, so an imported name
    // has a real type by the time the file using it is checked.
    let imported = match graph {
        Some(graph) => {
            // A program's modules are loaded, not run, so they are held to the
            // same declarations-only rule as its entry file. The interpreter
            // will not catch this one: it runs a statementful module quite
            // happily, which is the problem.
            if let Some(first) = graph.check_modules_of_a_program().into_iter().next() {
                return Err(first);
            }
            crate::program::imported_by(&ast, &graph.exports_by_path())
        }
        None => crate::program::Exports::default(),
    };

    let static_errors = crate::checker::check_typed(
        &ast,
        &parser.type_aliases,
        parser.node_count(),
        crate::entry::shape_of(&ast),
        &imported,
    )
    .0;
    if let Some(first) = static_errors.into_iter().next() {
        return Err(first);
    }

    if let Some(bad_main) = crate::entry::check_main_signature(&ast) {
        return Err(bad_main);
    }

    if let Some(first) = crate::entry::check_top_level(&ast).into_iter().next() {
        return Err(first);
    }

    // Interpretation
    let mut interpreter = Interpreter::new();

    // The graph already parsed and checked every module this program reaches.
    // Handing them over means the `grab` that reaches one only has to run it.
    if let Some(graph) = graph {
        let mut registry = crate::modules::ModuleRegistry::new(filename);
        for module in &graph.modules {
            if module.path == filename {
                continue;
            }
            registry.reuse(&module.path, module.ast.clone(), module.aliases.clone());
        }
        interpreter.module_registry = Some(registry);
    }

    // transfer type aliases from parser to interpreter
    interpreter.type_aliases = parser.type_aliases;

    let mut context = crate::context::Context::new("<program>", None, None);
    context.symbol_table = Rc::new(interpreter.global_symbol_table.clone());

    let result = interpreter.visit(&ast, &mut context);

    if let Some(error) = result.error {
        return Err(*error);
    }

    // In program mode the top level only declared things. `main` is what runs.
    if crate::entry::shape_of(&ast) == crate::entry::ProgramShape::Program {
        let Some(main_value) = context.symbol_table.get(crate::entry::MAIN) else {
            return Err(Error::new(
                crate::position::Position::new(0, 0, 0, filename, source),
                crate::position::Position::new(0, 0, 0, filename, source),
                "Internal Error",
                "shape_of found a top-level main, but it is not in scope",
            ));
        };

        let call = interpreter.call_value(
            main_value,
            Vec::new(),
            crate::position::Position::new(0, 0, 0, filename, source),
            crate::position::Position::new(0, 0, 0, filename, source),
            &mut context,
        );

        return match call.error {
            Some(error) => Err(*error),
            None => Ok(call.value.unwrap_or(Value::Null)),
        };
    }

    if let Some(value) = result.value {
        Ok(value)
    } else {
        Ok(Value::Null)
    }
}

/// Lexes, parses and statically checks a program without running it.
///
/// `Err` is a lexing or parsing failure, which stops analysis at the first one.
/// `Ok` is every static error the checker found, which may be empty. This is
/// what the CLI and the language server use, so a file with three type errors
/// reports three rather than one.
pub fn check_source(filename: &str, source: &str) -> Result<Vec<Error>, Error> {
    let mut lexer = Lexer::new(filename.to_string(), source.to_string());
    let tokens = match lexer.make_tokens() {
        Ok(tokens) => tokens,
        Err(e) => return Err(e.base),
    };

    let mut parser = Parser::new(tokens);
    let parse_result = parser.parse();

    if let Some(error) = parse_result.error {
        return Err(error);
    }

    match parse_result.node {
        Some(ast) => Ok(crate::checker::check(&ast, &parser.type_aliases)),
        None => Ok(Vec::new()),
    }
}

/// Lexes, parses and checks a program, returning the errors, the inferred
/// types and the tree they belong to.
///
/// `check_source` is this without the table, and stays the entry point for the
/// CLI and the language server, neither of which has any use for one.
pub fn check_source_typed(
    filename: &str,
    source: &str,
) -> Result<(Vec<Error>, crate::type_table::TypeTable, crate::nodes::Node), Error> {
    let mut lexer = Lexer::new(filename.to_string(), source.to_string());
    let tokens = match lexer.make_tokens() {
        Ok(tokens) => tokens,
        Err(e) => return Err(e.base),
    };

    let mut parser = Parser::new(tokens);
    let parse_result = parser.parse();

    if let Some(error) = parse_result.error {
        return Err(error);
    }

    let Some(ast) = parse_result.node else {
        return Ok((
            Vec::new(),
            crate::type_table::TypeTable::default(),
            crate::nodes::Node::List(crate::nodes::ListNode {
                id: crate::nodes::NodeId::UNSET,
                element_nodes: Vec::new(),
                position_start: crate::position::Position::new(0, 0, 0, filename, source),
                position_end: crate::position::Position::new(0, 0, 0, filename, source),
            }),
        ));
    };

    let shape = crate::entry::shape_of(&ast);
    let (errors, table) = crate::checker::check_typed(
        &ast,
        &parser.type_aliases,
        parser.node_count(),
        shape,
        &crate::program::Exports::default(),
    );
    Ok((errors, table, ast))
}

pub fn run_with_context(
    filename: &str,
    source: &str,
    context: &mut Context,
    interpreter: &mut Interpreter,
) -> Result<Value, Error> {
    let mut lexer = Lexer::new(filename.to_string(), source.to_string());
    let tokens = match lexer.make_tokens() {
        Ok(t) => t,
        Err(e) => return Err(e.base),
    };

    let mut parser = Parser::new(tokens);
    // Pass type aliases from interpreter to parser
    parser.type_aliases = interpreter.type_aliases.clone();

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

    interpreter.type_aliases.extend(parser.type_aliases);
    let result = interpreter.visit(&ast, context);

    if let Some(error) = result.error {
        Err(*error)
    } else if let Some(value) = result.value {
        Ok(value)
    } else {
        Ok(Value::Null)
    }
}

pub use repl::run_repl;
