//! # Symbol Table Module
//!
//! Implements variable storage and lookup with support for nested scopes.
//! Manages the mapping between identifiers and their runtime values
//! during program execution.

use crate::types::Type;
use crate::values::Value;
use std::cell::RefCell;
use crate::fxhash::{FxHashMap, FxHashSet};
use std::rc::Rc;

/// What an assignment needs to know about an existing binding
#[derive(Debug, Clone)]
pub struct BindingInfo {
    pub is_constant: bool,
    pub declared_type: Option<Type>,
}

/// Symbol table for variable storage with parent scoping
/// Uses RefCell for interior mutability to allow modification through Rc
#[derive(Debug, Clone)]
pub struct SymbolTable {
    symbols: Rc<RefCell<FxHashMap<String, Value>>>,
    types: Rc<RefCell<FxHashMap<String, Type>>>,
    constants: Rc<RefCell<FxHashSet<String>>>,
    parent: Option<Rc<SymbolTable>>,
}

impl SymbolTable {
    /// Creates a new symbol table
    pub fn new() -> Self {
        Self {
            symbols: Rc::new(RefCell::new(FxHashMap::default())),
            types: Rc::new(RefCell::new(FxHashMap::default())),
            constants: Rc::new(RefCell::new(FxHashSet::default())),
            parent: None,
        }
    }

    /// Creates a new symbol table with a parent for scoping
    pub fn with_parent(parent: Rc<SymbolTable>) -> Self {
        Self {
            symbols: Rc::new(RefCell::new(FxHashMap::default())),
            types: Rc::new(RefCell::new(FxHashMap::default())),
            constants: Rc::new(RefCell::new(FxHashSet::default())),
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

    /// Sets a value and its declared type in the current symbol table
    pub fn set_with_type(&self, name: String, value: Value, typ: Type) {
        self.symbols.borrow_mut().insert(name.clone(), value);
        self.types.borrow_mut().insert(name, typ);
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

    /// Gets the declared type of a variable (searching parents)
    pub fn get_declared_type(&self, name: &str) -> Option<Type> {
        if let Some(typ) = self.types.borrow().get(name) {
            Some(typ.clone())
        } else if let Some(parent) = &self.parent {
            parent.get_declared_type(name)
        } else {
            None
        }
    }

    /// Sets the declared type of a variable in the current scope
    pub fn set_declared_type(&self, name: String, typ: Type) {
        self.types.borrow_mut().insert(name, typ);
    }

    /// Drops every binding declared in this scope, leaving parents untouched.
    /// Lets a loop body reuse one scope across iterations instead of
    /// allocating a fresh symbol table each time round.
    pub fn clear_local(&self) {
        self.symbols.borrow_mut().clear();
        self.types.borrow_mut().clear();
        self.constants.borrow_mut().clear();
    }

    /// Marks a name as declared with `const let` in the current scope
    pub fn mark_constant(&self, name: String) {
        self.constants.borrow_mut().insert(name);
    }

    /// Was this name declared constant, here or in any enclosing scope?
    pub fn is_constant(&self, name: &str) -> bool {
        if self.symbols.borrow().contains_key(name) {
            return self.constants.borrow().contains(name);
        }
        match &self.parent {
            Some(parent) => parent.is_constant(name),
            None => false,
        }
    }

    /// Updates an existing binding in the scope that declared it.
    /// Returns false if the name is not bound anywhere in the chain.
    pub fn assign_existing(&self, name: &str, value: Value) -> bool {
        if self.symbols.borrow().contains_key(name) {
            self.symbols.borrow_mut().insert(name.to_string(), value);
            return true;
        }
        match &self.parent {
            Some(parent) => parent.assign_existing(name, value),
            None => false,
        }
    }

    /// Resolves a name for assignment in a single walk of the scope chain.
    ///
    /// Checking "declared?", "constant?", "declared type?" and then assigning
    /// as four separate calls walked the chain four times on every assignment;
    /// this does it once.
    pub fn resolve_for_assign(&self, name: &str) -> Option<BindingInfo> {
        if self.symbols.borrow().contains_key(name) {
            return Some(BindingInfo {
                is_constant: self.constants.borrow().contains(name),
                declared_type: self.types.borrow().get(name).cloned(),
            });
        }
        match &self.parent {
            Some(parent) => parent.resolve_for_assign(name),
            None => None,
        }
    }
}

impl Default for SymbolTable {
    fn default() -> Self {
        Self::new()
    }
}
