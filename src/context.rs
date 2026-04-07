//! # Execution Context Module
//!
//! Represents an execution scope with its symbol table and parent context.
//! Enables lexical scoping and proper variable resolution during
//! function calls and block execution.

use crate::position::Position;
use crate::symbol_table::SymbolTable;
use std::rc::Rc;

#[derive(Debug, Clone)]
pub struct Context {
    pub display_name: String,
    pub parent: Option<Box<Context>>,
    pub parent_entry_position: Option<Position>,
    pub symbol_table: SymbolTable,
}

impl Context {
    pub fn new(
        display_name: &str,
        parent: Option<Context>,
        parent_entry_position: Option<Position>,
    ) -> Self {
        let symbol_table = if let Some(parent_ctx) = &parent {
            SymbolTable::with_parent(Rc::new(parent_ctx.symbol_table.clone()))
        } else {
            SymbolTable::new()
        };

        Self {
            display_name: display_name.to_string(),
            parent: parent.map(Box::new),
            parent_entry_position,
            symbol_table,
        }
    }

    pub fn create_child(&self, display_name: &str, entry_pos: Position) -> Self {
        Self {
            display_name: display_name.to_string(),
            parent: Some(Box::new(self.clone())),
            parent_entry_position: Some(entry_pos),
            symbol_table: SymbolTable::with_parent(Rc::new(self.symbol_table.clone())),
        }
    }
}
