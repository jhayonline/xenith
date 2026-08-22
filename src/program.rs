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
use crate::modules::{ModuleError, ModuleRegistry, ParsedModule};
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

impl ModuleGraph {
    /// The root: the file the walk started from, pushed last because its
    /// dependencies go in front of it.
    pub fn root(&self) -> Option<&CompiledModule> {
        self.modules.last()
    }

    /// Holds every module a *program* imports to the declarations-only rule.
    ///
    /// A program whose own top level declares nothing, but which imports a
    /// module that runs statements on import, still writes to globals at run
    /// time. Checking the entry file alone would leave the guarantee hollow.
    ///
    /// Empty for a script, which imports modules the way it always has.
    pub fn check_modules_of_a_program(&self) -> Vec<Error> {
        let Some(root) = self.root() else {
            return Vec::new();
        };

        if crate::entry::shape_of(&root.ast) != crate::entry::ProgramShape::Program {
            return Vec::new();
        }

        self.modules
            .iter()
            .filter(|module| module.path != root.path)
            .flat_map(|module| {
                // The root is covered by `check_top_level`, with its own note.
                crate::entry::check_declarations_only(&module.ast, crate::entry::MODULE_NOTE)
            })
            .collect()
    }

    /// What each module exports, by the path it is grabbed under.
    ///
    /// Kept per module rather than merged, because two modules may export the
    /// same name and which one a file means depends on which it grabbed.
    pub fn exports_by_path(&self) -> HashMap<String, Exports> {
        self.modules
            .iter()
            .map(|module| (module.path.clone(), exports_of(module)))
            .collect()
    }
}

/// Resolves a program's imports.
///
/// The root is the file named on the command line, which is where it always
/// was; nothing about this requires the program to define `main`.
///
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

    // Parse first, check last. A module's imports have to be read before it can
    // be checked, because an imported name only has a type once the module it
    // came from has been checked -- so every dependency is walked, and checked,
    // in between.
    let parsed = match source {
        Some(text) => parse_root_source(path, text)?,
        None => registry.parse_module(path)?,
    };

    for imported in imports_of(&parsed.ast) {
        visit(&imported, None, registry, graph, done, in_progress)?;
    }

    // Everything this file could be importing is in the graph by now.
    let imported = imported_by(&parsed.ast, &graph.exports_by_path());

    let types = match source {
        Some(_) => check_root(path, &parsed, &imported)?,
        None => crate::modules::check_module(path, &parsed, &imported)?,
    };

    in_progress.pop();
    done.push(path.to_string());
    graph.modules.push(CompiledModule {
        path: path.to_string(),
        ast: parsed.ast,
        types,
        aliases: parsed.aliases,
    });

    Ok(())
}

/// The root is already in memory, so it does not go through `locate`.
fn parse_root_source(path: &str, source: &str) -> Result<ParsedModule, ModuleError> {
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

    let node_count = parser.node_count();
    Ok(ParsedModule {
        ast,
        aliases: parser.type_aliases,
        node_count,
    })
}

/// The root is the only file in the graph that may be a program rather than a
/// script, so the entry-point rules are applied here and nowhere else.
fn check_root(
    path: &str,
    parsed: &ParsedModule,
    imported: &Exports,
) -> Result<TypeTable, ModuleError> {
    let shape = crate::entry::shape_of(&parsed.ast);
    let (mut errors, types) = crate::checker::check_typed(
        &parsed.ast,
        &parsed.aliases,
        parsed.node_count,
        shape,
        imported,
    );

    errors.extend(crate::entry::check_top_level(&parsed.ast));
    if let Some(bad_main) = crate::entry::check_main_signature(&parsed.ast) {
        errors.push(bad_main);
    }

    if !errors.is_empty() {
        return Err(ModuleError::Failed {
            module: path.to_string(),
            errors,
        });
    }

    Ok(types)
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

/// What a module offers the file that grabs it.
///
/// Only `export`ed names. A method that is not exported is invisible, exactly
/// as it is at run time.
#[derive(Debug, Clone, Default)]
pub struct Exports {
    pub methods: HashMap<String, (Vec<Type>, Type)>,
    pub structs: HashMap<String, Vec<(String, Type)>>,
    pub enums: HashMap<String, Vec<(String, Vec<Type>)>>,
    pub aliases: HashMap<String, Type>,
}

impl Exports {
    /// Folds another module's exports in. Later wins, which matches the
    /// interpreter: a second `grab` of the same name shadows the first.
    pub fn extend(&mut self, other: Exports) {
        self.methods.extend(other.methods);
        self.structs.extend(other.structs);
        self.enums.extend(other.enums);
        self.aliases.extend(other.aliases);
    }
}

/// Reads a checked module's exported declarations off its tree.
pub fn exports_of(module: &CompiledModule) -> Exports {
    let mut exports = Exports {
        aliases: module.aliases.clone(),
        ..Exports::default()
    };

    let Node::List(statements) = &module.ast else {
        return exports;
    };

    for statement in &statements.element_nodes {
        let Node::Export(export) = &**statement else {
            continue;
        };

        // `ExportNode` already carries the name it exports under, so it is the
        // key rather than the token inside the declaration.
        let name = export.exported_name.clone();

        match export.node.as_ref() {
            Node::FuncDef(func) => {
                exports
                    .methods
                    .insert(name, (func.param_types.clone(), func.return_type.clone()));
            }
            Node::StructDef(def) => {
                let fields = def
                    .fields
                    .iter()
                    .filter_map(|f| f.name.value.clone().map(|n| (n, f.field_type.clone())))
                    .collect();
                exports.structs.insert(name, fields);
            }
            Node::EnumDef(def) => {
                let variants = def
                    .variants
                    .iter()
                    .filter_map(|v| v.name.value.clone().map(|n| (n, v.payload_types.clone())))
                    .collect();
                exports.enums.insert(name, variants);
            }
            _ => {}
        }
    }

    exports
}

/// The subset of what the graph offers that `ast` actually grabs, keyed by the
/// name the importing file uses.
///
/// Two rules, both of them about not inventing errors.
///
/// Only names the file grabs are seeded. Seeding everything would declare names
/// it never imported, and program mode would then fail to report them as
/// undefined.
///
/// And a name grabbed from more than one module is dropped. `grab` is a
/// statement: a later one rebinds the name for the rest of the file, so which
/// signature is live depends on where in the file you are. Until the checker
/// walks `grab` in statement order it cannot know, so it says nothing rather
/// than picking one and reporting every use of the other as a type error.
pub fn imported_by(ast: &Node, available: &HashMap<String, Exports>) -> Exports {
    let Node::List(statements) = ast else {
        return Exports::default();
    };

    let mut taken = Exports::default();
    let mut ambiguous: Vec<String> = Vec::new();

    for statement in &statements.element_nodes {
        let Node::Grab(grab) = &**statement else {
            continue;
        };

        // `grab * as name` brings everything in under a namespace, which the
        // checker cannot yet describe as a type. Leave it alone rather than
        // guess.
        if grab.is_namespace_import {
            continue;
        }

        let Some(module) = available.get(&grab.from_module) else {
            continue;
        };

        taken.aliases.extend(module.aliases.clone());

        for spec in &grab.imports {
            let local = spec
                .alias
                .clone()
                .unwrap_or_else(|| spec.original_name.clone());

            if let Some(sig) = module.methods.get(&spec.original_name) {
                if taken.methods.insert(local.clone(), sig.clone()).is_some() {
                    ambiguous.push(local.clone());
                }
            }
            // A struct or an enum cannot be renamed on import, so the local
            // name is always the original.
            if let Some(fields) = module.structs.get(&spec.original_name) {
                if taken
                    .structs
                    .insert(spec.original_name.clone(), fields.clone())
                    .is_some()
                {
                    ambiguous.push(spec.original_name.clone());
                }
            }
            if let Some(variants) = module.enums.get(&spec.original_name) {
                if taken
                    .enums
                    .insert(spec.original_name.clone(), variants.clone())
                    .is_some()
                {
                    ambiguous.push(spec.original_name.clone());
                }
            }
        }
    }

    for name in ambiguous {
        taken.methods.remove(&name);
        taken.structs.remove(&name);
        taken.enums.remove(&name);
    }

    taken
}
