//! # Symbol Table Module
//!
//! Implements variable storage and lookup with support for nested scopes.
//! Manages the mapping between identifiers and their runtime values
//! during program execution.

use crate::types::Type;
use crate::values::Value;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// Symbol table for variable storage with parent scoping
/// Uses RefCell for interior mutability to allow modification through Rc
#[derive(Debug, Clone)]
pub struct SymbolTable {
    symbols: Rc<RefCell<HashMap<String, Value>>>,
    types: Rc<RefCell<HashMap<String, Type>>>,
    parent: Option<Rc<SymbolTable>>,
}

impl SymbolTable {
    /// Creates a new symbol table
    pub fn new() -> Self {
        Self {
            symbols: Rc::new(RefCell::new(HashMap::new())),
            types: Rc::new(RefCell::new(HashMap::new())),
            parent: None,
        }
    }

    /// Creates a new symbol table with a parent for scoping
    pub fn with_parent(parent: Rc<SymbolTable>) -> Self {
        Self {
            symbols: Rc::new(RefCell::new(HashMap::new())),
            types: Rc::new(RefCell::new(HashMap::new())),
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

    /// Gets a type from the symbol table (searching parents)
    pub fn get_type(&self, name: &str) -> Option<Type> {
        if let Some(typ) = self.types.borrow().get(name) {
            Some(typ.clone())
        } else if let Some(parent) = &self.parent {
            parent.get_type(name)
        } else {
            None
        }
    }

    /// Sets a type in the current symbol table
    pub fn set_type(&self, name: String, typ: Type) {
        self.types.borrow_mut().insert(name, typ);
    }

    /// Sets a value in the current symbol table
    pub fn set(&self, name: String, value: Value) {
        self.symbols.borrow_mut().insert(name, value);
    }

    /// Removes a value from the current symbol table
    pub fn remove(&self, name: &str) -> Option<Value> {
        self.symbols.borrow_mut().remove(name)
    }

    /// Updates a variable in the scope it was originally defined, or sets in current scope if not found
    pub fn set_existing(&self, name: String, value: Value) {
        // Check current scope first
        if self.symbols.borrow().contains_key(&name) {
            self.symbols.borrow_mut().insert(name, value);
        } else if let Some(parent) = &self.parent {
            // Recursively try to set in parent - no mutation needed here
            parent.set_existing(name, value);
        } else {
            // Not found anywhere, set in current scope
            self.symbols.borrow_mut().insert(name, value);
        }
    }

    /// Sets a value only in the local scope (does not traverse parents)
    pub fn set_local(&self, name: String, value: Value) {
        self.symbols.borrow_mut().insert(name, value);
    }

    /// Checks if this table has a parent
    pub fn has_parent(&self) -> bool {
        self.parent.is_some()
    }

    /// Checks if a variable exists in this scope or any parent
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
