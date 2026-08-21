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

use crate::nodes::Node;

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
