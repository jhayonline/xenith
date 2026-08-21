//! The whole import graph, resolved before anything runs.
//!
//! Modules used to be found, checked and executed one at a time, as each
//! `grab` was evaluated. That is why an imported name is checked as it runs
//! rather than before, and it is why a call into the standard library cannot
//! be given a type: the checker has never seen the module when it reaches the
//! call.
//!
//! Walking the graph from the root first fixes both. It also means module
//! loading is no longer execution, which is what a precompiled standard
//! library needs.

use std::collections::HashMap;

use crate::error::Error;
use crate::modules::{ModuleError, ModuleRegistry};
use crate::nodes::Node;
use crate::type_table::TypeTable;
use crate::types::Type;

/// One module, parsed and checked but not run.
pub struct CompiledModule {
    pub path: String,
    pub ast: Node,
    pub types: TypeTable,
    pub aliases: HashMap<String, Type>,
}

/// Every module a program needs, dependencies before dependents.
pub struct ModuleGraph {
    pub modules: Vec<CompiledModule>,
}

/// Resolves a program's imports.
///
/// The root is the file named on the command line, which is where it always
/// was; nothing about this requires the program to define `main`.
pub fn build(root_path: &str, root_source: &str) -> Result<ModuleGraph, ModuleError> {
    let registry = ModuleRegistry::new(root_path);
    let mut graph = ModuleGraph {
        modules: Vec::new(),
    };
    let mut done: Vec<String> = Vec::new();
    let mut in_progress: Vec<String> = Vec::new();

    visit(
        root_path,
        Some(root_source),
        &registry,
        &mut graph,
        &mut done,
        &mut in_progress,
    )?;

    Ok(graph)
}

/// Depth first, so a module is pushed only after everything it imports.
///
/// `in_progress` is the path from the root to here. Meeting a path already on
/// it is a cycle, which is reported rather than followed.
fn visit(
    path: &str,
    source: Option<&str>,
    registry: &ModuleRegistry,
    graph: &mut ModuleGraph,
    done: &mut Vec<String>,
    in_progress: &mut Vec<String>,
) -> Result<(), ModuleError> {
    if done.iter().any(|d| d == path) {
        return Ok(());
    }

    if in_progress.iter().any(|p| p == path) {
        let mut cycle = in_progress.clone();
        cycle.push(path.to_string());
        return Err(ModuleError::Circular(cycle));
    }

    in_progress.push(path.to_string());

    let (ast, types, aliases) = match source {
        Some(text) => parse_root(path, text)?,
        None => registry.parse_and_check(path)?,
    };

    for imported in imports_of(&ast) {
        visit(&imported, None, registry, graph, done, in_progress)?;
    }

    in_progress.pop();
    done.push(path.to_string());
    graph.modules.push(CompiledModule {
        path: path.to_string(),
        ast,
        types,
        aliases,
    });

    Ok(())
}

/// The root is already in memory, so it does not go through `locate`.
///
/// It is also the only file in the graph that may be a program rather than a
/// script, so the entry-point rules are applied here and nowhere else in the
/// walk.
fn parse_root(
    path: &str,
    source: &str,
) -> Result<(Node, TypeTable, HashMap<String, Type>), ModuleError> {
    let failed = |errors: Vec<Error>| ModuleError::Failed {
        module: path.to_string(),
        errors,
    };

    let mut lexer = crate::lexer::Lexer::new(path.to_string(), source.to_string());
    let tokens = match lexer.make_tokens() {
        Ok(tokens) => tokens,
        Err(e) => return Err(failed(vec![e.base])),
    };

    let mut parser = crate::parser::Parser::new(tokens);
    let parse_result = parser.parse();

    if let Some(error) = parse_result.error {
        return Err(failed(vec![error]));
    }

    let Some(ast) = parse_result.node else {
        return Err(failed(Vec::new()));
    };

    let shape = crate::entry::shape_of(&ast);
    let (mut errors, types) = crate::checker::check_typed(
        &ast,
        &parser.type_aliases,
        parser.node_count(),
        shape,
    );

    errors.extend(crate::entry::check_top_level(&ast));
    if let Some(bad_main) = crate::entry::check_main_signature(&ast) {
        errors.push(bad_main);
    }

    if !errors.is_empty() {
        return Err(failed(errors));
    }

    Ok((ast, types, parser.type_aliases))
}

/// Every module path a file grabs.
fn imports_of(ast: &Node) -> Vec<String> {
    let Node::List(statements) = ast else {
        return Vec::new();
    };

    statements
        .element_nodes
        .iter()
        .filter_map(|statement| match &**statement {
            Node::Grab(grab) => Some(grab.from_module.clone()),
            _ => None,
        })
        .collect()
}
