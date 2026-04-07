//! # Symbol Table Module
//!
//! Implements variable storage and lookup with support for nested scopes.
//! Manages the mapping between identifiers and their runtime values
//! during program execution.

use crate::values::Value;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// Symbol table for variable storage with parent scoping
#[derive(Debug, Clone)]
pub struct SymbolTable {
    symbols: Rc<RefCell<HashMap<String, Value>>>,
    parent: Option<Rc<SymbolTable>>,
}

impl SymbolTable {
    /// Creates a new symbol table
    pub fn new() -> Self {
        Self {
            symbols: Rc::new(RefCell::new(HashMap::new())),
            parent: None,
        }
    }

    /// Creates a new symbol table with a parent for scoping
    pub fn with_parent(parent: Rc<SymbolTable>) -> Self {
        Self {
            symbols: Rc::new(RefCell::new(HashMap::new())),
            parent: Some(parent),
        }
    }

    /// Gets a value from the symbol table (searching parents)
    pub fn get(&self, name: &str) -> Option<Value> {
        if let Some(value) = self.symbols.borrow().get(name) {
            Some(value.clone())
        } else if let Some(parent) = &self.parent {
            parent.get(name)
        } else {
            None
        }
    }

    /// Sets a value in the current symbol table
    pub fn set(&mut self, name: String, value: Value) {
        self.symbols.borrow_mut().insert(name, value);
    }

    /// Removes a value from the current symbol table
    pub fn remove(&mut self, name: &str) -> Option<Value> {
        self.symbols.borrow_mut().remove(name)
    }

    /// Updates a variable in the scope it was originally defined, or sets in current scope if not found
    pub fn set_existing(&mut self, name: String, value: Value) {
        if self.symbols.borrow().contains_key(&name) {
            self.symbols.borrow_mut().insert(name, value);
        } else if let Some(parent) = &mut self.parent {
            // We need to get a mutable reference to the parent
            // Since parent is Rc, we need to use RefCell pattern
            if let Some(parent_mut) = Rc::get_mut(parent) {
                parent_mut.set_existing(name, value);
            } else {
                // Can't get mutable reference, set in current scope
                self.symbols.borrow_mut().insert(name, value);
            }
        } else {
            self.symbols.borrow_mut().insert(name, value);
        }
    }

    pub fn set_local(&mut self, name: String, value: Value) {
        self.symbols.borrow_mut().insert(name, value);
    }

    pub fn has_parent(&self) -> bool {
        self.parent.is_some()
    }

    pub fn contains(&self, name: &str) -> bool {
        if self.symbols.borrow().contains_key(name) {
            true
        } else if let Some(parent) = &self.parent {
            parent.contains(name)
        } else {
            false
        }
    }
}

impl Default for SymbolTable {
    fn default() -> Self {
        Self::new()
    }
}
