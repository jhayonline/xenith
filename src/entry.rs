//! Whether a file is a program or a script.
//!
//! A file that defines `method main() -> int` at its top level is a
//! **program**: its top level holds declarations only, statements live in
//! `main`, and `main`'s result is the process exit code.
//!
//! A file that does not is a **script**, and runs its top level, which is what
//! Xenith has always done. Scripts are not a legacy mode; they are what the
//! REPL is, and what a short program should stay.
//!
//! The distinction buys two things beyond taste. The global table becomes
//! entirely static, because nothing is written to it at run time. And
//! undefined names become statically reportable, because the reachability
//! question in `docs/internals/06-checker.md` only exists when a top-level
//! `let` can be captured by a method defined above it.

use crate::error::Error;
use crate::nodes::Node;
use crate::types::Type;

/// The name a program's entry point must have.
pub const MAIN: &str = "main";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgramShape {
    /// Top level is declarations; `main` is the entry point.
    Program,
    /// Top level is the program.
    Script,
}

/// Classifies a parsed file.
///
/// Only a top-level `method main` counts. A `main` nested inside another
/// method is an ordinary closure that happens to share the name, and a
/// `let main` is a binding.
pub fn shape_of(ast: &Node) -> ProgramShape {
    let Node::List(statements) = ast else {
        return ProgramShape::Script;
    };

    for statement in &statements.element_nodes {
        if let Node::FuncDef(func) = &**statement {
            let named_main = func
                .variable_name_token
                .as_ref()
                .and_then(|token| token.value.as_deref())
                == Some(MAIN);

            if named_main {
                return ProgramShape::Program;
            }
        }
    }

    ProgramShape::Script
}

/// `Some` when the file has a top-level `main` whose shape is wrong.
///
/// `main` takes nothing and releases an `int`, which is the process exit code.
/// Arguments live in `std::env`, where they already are.
///
/// Note the ordering against [`shape_of`], which returns `Program` for any
/// top-level `main` including a mistyped one. That is deliberate: a bad entry
/// point is reported, not quietly treated as a script whose `main` never runs.
pub fn check_main_signature(ast: &Node) -> Option<Error> {
    let Node::List(statements) = ast else {
        return None;
    };

    for statement in &statements.element_nodes {
        let Node::FuncDef(func) = &**statement else {
            continue;
        };

        if func
            .variable_name_token
            .as_ref()
            .and_then(|t| t.value.as_deref())
            != Some(MAIN)
        {
            continue;
        }

        if !func.param_names.is_empty() {
            return Some(
                Error::new(
                    func.position_start.clone(),
                    func.position_end.clone(),
                    "Entry Point Error",
                    &format!(
                        "main takes no parameters, but this one takes {}",
                        func.param_names.len()
                    ),
                )
                .with_code("XEN024")
                .with_help("read arguments with std::env instead"),
            );
        }

        if func.return_type != Type::Int {
            return Some(
                Error::new(
                    func.position_start.clone(),
                    func.position_end.clone(),
                    "Entry Point Error",
                    &format!(
                        "main must release an int, the process exit code, but this one releases {}",
                        func.return_type.to_string()
                    ),
                )
                .with_code("XEN024")
                .with_help("write `method main() -> int` and `release 0` for success"),
            );
        }

        return None;
    }

    None
}

/// The declarations-only rule, with no test of what kind of file this is.
///
/// Applied to a program's own top level by [`check_top_level`], and to every
/// module a program imports. A module that runs statements on import writes to
/// globals at run time, which is exactly what the rule exists to prevent --
/// enforcing it on the entry file alone would leave the guarantee hollow.
///
/// `note` is why this file is being held to the rule, which differs between
/// the entry file and a module it imports.
pub fn check_declarations_only(ast: &Node, note: &str) -> Vec<Error> {
    let Node::List(statements) = ast else {
        return Vec::new();
    };

    let mut errors = Vec::new();

    for statement in &statements.element_nodes {
        let permitted = match &**statement {
            Node::Grab(_)
            | Node::Export(_)
            | Node::StructDef(_)
            | Node::EnumDef(_)
            | Node::FuncDef(_)
            | Node::TypeAlias(_) => true,
            // `const let LIMIT: int = 100` is a declaration; a plain `let` is
            // a statement, because it can be reassigned and therefore written
            // to at run time.
            Node::VarAssign(assign) => assign.is_constant && assign.is_declaration,
            _ => false,
        };

        if permitted {
            continue;
        }

        errors.push(
            Error::new(
                statement.position_start().clone(),
                statement.position_end().clone(),
                "Entry Point Error",
                "a program's top level holds declarations only",
            )
            .with_code("XEN025")
            .with_note(note)
            .with_help("move this into main, or make it `const let` if it never changes"),
        );
    }

    errors
}

/// Why the entry file is held to the rule.
pub const PROGRAM_NOTE: &str =
    "this file defines main, so it is a program rather than a script";

/// Why a module a program imports is held to it.
pub const MODULE_NOTE: &str =
    "this module is imported by a program, whose modules are loaded rather than run";

/// Every top-level statement a program is not allowed to have.
///
/// Declarations are permitted: `grab`, `struct`, `enum`, `method`, `type` and
/// `const let`. Everything else belongs in `main`. Two things follow. Nothing
/// is written to a global at run time, so the global table is known and
/// complete before the first instruction executes. And no top-level `let`
/// exists to be captured, which is what lets the checker report an undefined
/// name statically.
///
/// A script gets no errors from this; its top level *is* the program.
pub fn check_top_level(ast: &Node) -> Vec<Error> {
    if shape_of(ast) != ProgramShape::Program {
        return Vec::new();
    }
    check_declarations_only(ast, PROGRAM_NOTE)
}
