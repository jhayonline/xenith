//! # Module System Module
//!
//! Handles module loading, caching, and resolution.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use crate::context::Context;
use crate::interpreter::Interpreter;
use crate::lexer::Lexer;
use crate::nodes::Node;
use crate::parser::Parser;
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
    pub ast: Node,
}

impl ModuleRegistry {
    pub fn new(current_file: &str) -> Self {
        Self {
            modules: HashMap::new(),
            current_file: PathBuf::from(current_file),
        }
    }

    /// Resolve a module path to a file
    pub fn resolve_path(&self, module_path: &str) -> Option<PathBuf> {
        // Handle std:: prefix
        if module_path.starts_with("std::") {
            let stdlib_path = module_path.strip_prefix("std::").unwrap();
            return self.resolve_stdlib(stdlib_path);
        }

        // Handle local modules (mod::math or just math)
        let clean_path = module_path.strip_prefix("mod::").unwrap_or(module_path);
        self.resolve_local(clean_path)
    }

    fn resolve_stdlib(&self, path: &str) -> Option<PathBuf> {
        // Replace :: with path separator
        let file_path = path.replace("::", "/");

        // Look in built-in stdlib directory (relative to executable)
        let exe_path = std::env::current_exe().ok()?;
        let stdlib_dir = exe_path.parent()?.join("stdlib");

        let candidates = vec![
            stdlib_dir.join(&file_path).with_extension("xen"),
            stdlib_dir.join(&file_path).join("mod.xen"),
        ];

        candidates.into_iter().find(|p| p.exists())
    }

    fn resolve_local(&self, path: &str) -> Option<PathBuf> {
        // Replace :: with OS path separator
        let file_path = path.replace("::", "/");

        // Get directory of current file
        let current_dir = self.current_file.parent()?;

        let candidates = vec![
            current_dir.join(&file_path).with_extension("xen"),
            current_dir.join(&file_path).join("mod.xen"),
        ];

        candidates.into_iter().find(|p| p.exists())
    }

    /// Load a module (with caching)
    pub fn load_module(
        &mut self,
        module_path: &str,
        interpreter: &mut Interpreter,
    ) -> Result<Module, String> {
        // Check cache first
        if let Some(module) = self.modules.get(module_path) {
            return Ok(module.clone());
        }

        // Resolve file path
        let file_path = self
            .resolve_path(module_path)
            .ok_or_else(|| format!("Module '{}' not found", module_path))?;

        // Read and parse file
        let source = fs::read_to_string(&file_path)
            .map_err(|e| format!("Failed to read module '{}': {}", module_path, e))?;

        let mut lexer = Lexer::new(file_path.to_string_lossy().to_string(), source);
        let tokens = lexer.make_tokens().map_err(|e| e.base.as_string())?;

        let mut parser = Parser::new(tokens);
        let parse_result = parser.parse();

        if let Some(error) = parse_result.error {
            return Err(error.as_string());
        }

        let ast = parse_result.node.unwrap();

        // Create module context and execute
        let mut module_context = Context::new(&module_path, None, None);

        // Track exports during execution
        let exec_result = interpreter.visit(&ast, &mut module_context);

        if let Some(error) = exec_result.error {
            return Err(error.as_string());
        }

        // Collect exports from the module's symbol table
        let exports = module_context.get_exports().clone();

        let module = Module {
            name: module_path.to_string(),
            exports,
            ast,
        };

        // Cache the module
        self.modules.insert(module_path.to_string(), module.clone());

        Ok(module)
    }
}
