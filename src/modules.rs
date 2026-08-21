//! # Module System Module
//!
//! Handles module loading, caching, and resolution.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use crate::context::Context;
use crate::error::Error;
use crate::interpreter::Interpreter;
use crate::lexer::Lexer;
use crate::nodes::Node;
use crate::parser::Parser;
use crate::type_table::TypeTable;
use crate::types::Type;
use crate::values::Value;

/// Module registry that caches loaded modules
#[derive(Debug, Clone)]
pub struct ModuleRegistry {
    modules: HashMap<String, Module>,
    current_file: PathBuf,
}

#[derive(Debug, Clone)]
pub struct Module {
    pub name: String,
    pub exports: HashMap<String, Value>,
    /// Structs the module marked `export`, with their declared fields. Kept
    /// apart from `exports` because a struct is a type and not a value.
    pub struct_exports: HashMap<String, Vec<(String, crate::types::Type)>>,
    /// Enums the module marked `export`, with their variants.
    pub enum_exports: HashMap<String, Vec<(String, Vec<crate::types::Type>)>>,
    pub ast: Node,
}

/// A module after parsing, before checking.
pub struct ParsedModule {
    pub ast: Node,
    pub aliases: HashMap<String, Type>,
    pub node_count: u32,
}

/// Checks a parsed module against what it imports.
///
/// A module gets the same static checking as a file run directly. Without it
/// an imported method could return the wrong type and the importing program
/// would use the result without a word, which is exactly where the guarantee
/// is worth most.
///
/// `Script`, because a module's top level is where its declarations live and
/// it does not define `main`.
pub fn check_module(
    module_path: &str,
    parsed: &ParsedModule,
    imported: &crate::program::Exports,
) -> Result<TypeTable, ModuleError> {
    let (errors, types) = crate::checker::check_typed(
        &parsed.ast,
        &parsed.aliases,
        parsed.node_count,
        crate::entry::ProgramShape::Script,
        imported,
    );

    if !errors.is_empty() {
        return Err(ModuleError::Failed {
            module: module_path.to_string(),
            errors,
        });
    }

    Ok(types)
}

impl ModuleRegistry {
    pub fn new(current_file: &str) -> Self {
        Self {
            modules: HashMap::new(),
            current_file: PathBuf::from(current_file),
        }
    }

    /// Resolve a module path to a file.
    ///
    /// `std::` modules do not live on disk; see [`ModuleRegistry::locate`].
    pub fn resolve_path(&self, module_path: &str) -> Option<PathBuf> {
        let clean_path = module_path.strip_prefix("mod::").unwrap_or(module_path);
        self.resolve_local(clean_path)
    }

    /// Where a module's source comes from.
    fn locate(&self, module_path: &str) -> Option<(String, String)> {
        // `std::` modules are built into the binary rather than found on disk.
        if let Some(name) = module_path.strip_prefix("std::") {
            let source = crate::stdlib::source(name)?;
            return Some((format!("std::{}", name), source.to_string()));
        }

        let file_path = self.resolve_path(module_path)?;
        let source = fs::read_to_string(&file_path).ok()?;
        Some((file_path.to_string_lossy().to_string(), source))
    }

    fn resolve_local(&self, path: &str) -> Option<PathBuf> {
        // Replace :: with OS path separator
        let file_path = path.replace("::", "/");

        // Get directory of current file
        let current_dir = self.current_file.parent()?;

        // Try multiple locations:
        // 1. Relative to current file's directory
        let candidate1 = current_dir.join(&file_path).with_extension("xen");
        if candidate1.exists() {
            return Some(candidate1);
        }

        // 2. Relative to current file's parent (project root)
        let candidate2 = current_dir.parent()?.join(&file_path).with_extension("xen");
        if candidate2.exists() {
            return Some(candidate2);
        }

        // 3. Just the filename in current directory
        let candidate3 = current_dir.join(&file_path).with_extension("xen");
        if candidate3.exists() {
            return Some(candidate3);
        }

        None
    }

    /// Load a module (with caching)
    /// Finds, parses and checks a module without running it.
    ///
    /// `load_module` is this plus execution. They are separate because a
    /// program's whole import graph is walked before anything runs, and
    /// walking it must not have side effects.
    pub fn parse_and_check(
        &self,
        module_path: &str,
    ) -> Result<(Node, TypeTable, HashMap<String, Type>), ModuleError> {
        let parsed = self.parse_module(module_path)?;
        let types = check_module(module_path, &parsed, &crate::program::Exports::default())?;
        Ok((parsed.ast, types, parsed.aliases))
    }

    /// Finds and parses a module, without checking or running it.
    ///
    /// Split from checking because the graph walk has to read a module's
    /// imports before it can check it: an imported name only has a type once
    /// the module it came from has been checked, so the dependencies go first.
    pub fn parse_module(&self, module_path: &str) -> Result<ParsedModule, ModuleError> {
        // Find the source, on disk or built in
        let Some((name, source)) = self.locate(module_path) else {
            return Err(ModuleError::NotFound(module_path.to_string()));
        };

        let failed = |errors: Vec<Error>| ModuleError::Failed {
            module: module_path.to_string(),
            errors,
        };

        let mut lexer = Lexer::new(name, source);
        let tokens = match lexer.make_tokens() {
            Ok(tokens) => tokens,
            Err(e) => return Err(failed(vec![e.base])),
        };

        let mut parser = Parser::new(tokens);
        let parse_result = parser.parse();

        if let Some(error) = parse_result.error {
            return Err(failed(vec![error]));
        }

        let node_count = parser.node_count();
        Ok(ParsedModule {
            ast: parse_result.node.unwrap(),
            aliases: parser.type_aliases,
            node_count,
        })
    }

    pub fn load_module(
        &mut self,
        module_path: &str,
        interpreter: &mut Interpreter,
    ) -> Result<Module, ModuleError> {
        // Check cache first
        if let Some(module) = self.modules.get(module_path) {
            return Ok(module.clone());
        }

        let failed = |errors: Vec<Error>| ModuleError::Failed {
            module: module_path.to_string(),
            errors,
        };

        let (ast, _types, aliases) = self.parse_and_check(module_path)?;

        // Transfer type aliases from parser to interpreter for this module
        interpreter.type_aliases.extend(aliases);

        // Create module context and execute
        let mut module_context = Context::new(module_path, None, None);

        // Track exports during execution
        let exec_result = interpreter.visit(&ast, &mut module_context);

        if let Some(error) = exec_result.error {
            return Err(failed(vec![*error]));
        }
        // Collect exports from the module's symbol table
        let exports = module_context.get_exports().clone();
        let struct_exports = module_context.get_struct_exports().clone();
        let enum_exports = module_context.get_enum_exports().clone();

        let module = Module {
            name: module_path.to_string(),
            exports,
            struct_exports,
            enum_exports,
            ast,
        };

        // Cache the module
        self.modules.insert(module_path.to_string(), module.clone());

        Ok(module)
    }
}

/// Why a module could not be loaded.
///
/// This used to be a `String`, which meant a nested failure arrived already
/// rendered and the caller had to search it for text to work out what kind of
/// error it was. Keeping the errors themselves lets the caller report them with
/// their own codes and positions.
#[derive(Debug)]
pub enum ModuleError {
    /// No file matched the path.
    NotFound(String),
    /// The file exists but could not be read.
    Unreadable(String, String),
    /// The module was found but lexing, parsing, checking or running it failed.
    Failed { module: String, errors: Vec<Error> },
    /// Modules importing each other, innermost last.
    Circular(Vec<String>),
}
