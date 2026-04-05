//! # Execution Context Module
//!
//! Represents an execution scope with its symbol table and parent context.
//! Enables lexical scoping and proper variable resolution during
//! function calls and block execution.

use crate::position::Position;
use crate::symbol_table::SymbolTable;

/// Represents an execution context with variable scope and parent chain
#[derive(Debug, Clone)]
pub struct Context {
    /// Display name for debugging (e.g., function name)
    pub display_name: String,
    /// Parent context for lexical scoping
    pub parent: Option<Box<Context>>,
    /// Position where this context was entered
    pub parent_entry_position: Option<Position>,
    /// Symbol table for variable storage
    pub symbol_table: SymbolTable,
}

impl Context {
    /// Creates a new execution context
    ///
    /// # Arguments
    /// * `display_name` - Name for debugging (e.g., "<program>", function name)
    /// * `parent` - Optional parent context for lexical scoping
    /// * `parent_entry_position` - Position where this context was entered
    pub fn new(
        display_name: &str,
        parent: Option<Context>,
        parent_entry_position: Option<Position>,
    ) -> Self {
        Self {
            display_name: display_name.to_string(),
            parent: parent.map(Box::new),
            parent_entry_position,
            symbol_table: SymbolTable::new(),
        }
    }

    /// Creates a new child context
    pub fn create_child(&self, display_name: &str, entry_pos: Position) -> Self {
        Self {
            display_name: display_name.to_string(),
            parent: Some(Box::new(self.clone())),
            parent_entry_position: Some(entry_pos),
            symbol_table: SymbolTable::new(),
        }
    }
}
