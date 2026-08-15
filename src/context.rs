//! # Execution Context Module
//!
//! Represents an execution scope with its symbol table and parent context.
//! Enables lexical scoping and proper variable resolution during
//! function calls and block execution.

use crate::position::Position;
use crate::symbol_table::SymbolTable;
use crate::types::Type;
use crate::values::Value;
use std::collections::HashMap;
use std::rc::Rc;

/// Maximum nesting depth before we stop and report a clean error.
///
/// The interpreter recurses on the Rust stack, so unbounded Xenith recursion
/// would abort the process with a stack overflow instead of a diagnostic.
pub const MAX_CALL_DEPTH: usize = 10_000;

#[derive(Debug, Clone)]
pub struct Context {
    pub display_name: String,
    /// Shared, never deep-copied. Cloning a Context must stay O(1): it happens
    /// on every call and every block, and a Box here made calls O(depth^2).
    pub parent: Option<Rc<Context>>,
    pub parent_entry_position: Option<Position>,
    pub symbol_table: Rc<SymbolTable>,
    pub exports: HashMap<String, Value>,
    /// Structs this module marked `export`, with their declared fields.
    ///
    /// Separate from `exports` because a struct definition is not a value:
    /// there is nothing to put in the map. What an importer needs is the field
    /// list, so that is what travels.
    pub struct_exports: HashMap<String, Vec<(String, Type)>>,
    /// Enums this module marked `export`, with their variants. Here for the
    /// same reason as `struct_exports`: an enum is a type, not a value.
    pub enum_exports: HashMap<String, Vec<(String, Vec<Type>)>>,
    /// Nesting depth of this context, counted from the program root.
    pub depth: usize,
}

impl Context {
    pub fn new(
        display_name: &str,
        parent: Option<Context>,
        parent_entry_position: Option<Position>,
    ) -> Self {
        let symbol_table = if let Some(parent_ctx) = &parent {
            Rc::new(SymbolTable::with_parent(parent_ctx.symbol_table.clone()))
        } else {
            Rc::new(SymbolTable::new())
        };
        let depth = parent.as_ref().map(|p| p.depth + 1).unwrap_or(0);

        Self {
            display_name: display_name.to_string(),
            parent: parent.map(Rc::new),
            parent_entry_position,
            symbol_table,
            exports: HashMap::new(),
            struct_exports: HashMap::new(),
            enum_exports: HashMap::new(),
            depth,
        }
    }

    pub fn create_child(&self, display_name: &str, entry_pos: Position) -> Self {
        Self {
            display_name: display_name.to_string(),
            parent: Some(Rc::new(self.clone())),
            parent_entry_position: Some(entry_pos),
            symbol_table: Rc::new(SymbolTable::with_parent(self.symbol_table.clone())),
            exports: HashMap::new(),
            struct_exports: HashMap::new(),
            enum_exports: HashMap::new(),
            depth: self.depth + 1,
        }
    }

    /// Has nesting gone past the point where the Rust stack is at risk?
    pub fn depth_exceeded(&self) -> bool {
        self.depth >= MAX_CALL_DEPTH
    }

    pub fn add_export(&mut self, name: String, value: Value) {
        self.exports.insert(name, value);
    }

    pub fn get_exports(&self) -> &HashMap<String, Value> {
        &self.exports
    }

    pub fn add_struct_export(&mut self, name: String, fields: Vec<(String, Type)>) {
        self.struct_exports.insert(name, fields);
    }

    pub fn get_struct_exports(&self) -> &HashMap<String, Vec<(String, Type)>> {
        &self.struct_exports
    }

    pub fn add_enum_export(&mut self, name: String, variants: Vec<(String, Vec<Type>)>) {
        self.enum_exports.insert(name, variants);
    }

    pub fn get_enum_exports(&self) -> &HashMap<String, Vec<(String, Vec<Type>)>> {
        &self.enum_exports
    }
}
