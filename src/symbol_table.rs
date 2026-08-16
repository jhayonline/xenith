//! # Symbol Table Module
//!
//! Variable storage, one table per scope, chained to the enclosing one.
//!
//! Bindings live in a `Vec` rather than a map, with a map beside it from name
//! to position. That buys two things:
//!
//! - The resolver can hand the interpreter a slot index, and reading a variable
//!   becomes a bounds-checked vector index with no hashing at all.
//! - A binding's value, declared type and constness sit together in one entry,
//!   so an assignment touches one structure instead of three separate maps.
//!
//! Nothing is ever removed from a table, only overwritten or cleared wholesale,
//! which is what makes a slot index stable for the life of a scope.

use crate::fxhash::FxHashMap;
use crate::types::Type;
use crate::values::Value;
use std::cell::RefCell;
use std::rc::Rc;

/// One binding.
#[derive(Debug, Clone)]
pub struct Entry {
    pub name: Rc<str>,
    pub value: Value,
    pub declared_type: Option<Type>,
    pub is_constant: bool,
}

/// Symbol table for variable storage with parent scoping.
/// Uses RefCell for interior mutability to allow modification through Rc.
#[derive(Debug, Clone)]
pub struct SymbolTable {
    entries: Rc<RefCell<Vec<Entry>>>,
    index: Rc<RefCell<FxHashMap<Rc<str>, u32>>>,
    parent: Option<Rc<SymbolTable>>,
}

impl SymbolTable {
    /// Creates a new symbol table
    pub fn new() -> Self {
        Self {
            entries: Rc::new(RefCell::new(Vec::new())),
            index: Rc::new(RefCell::new(FxHashMap::default())),
            parent: None,
        }
    }

    /// Creates a new symbol table with a parent for scoping
    pub fn with_parent(parent: Rc<SymbolTable>) -> Self {
        Self {
            entries: Rc::new(RefCell::new(Vec::new())),
            index: Rc::new(RefCell::new(FxHashMap::default())),
            parent: Some(parent),
        }
    }

    // -- slot access -------------------------------------------------------

    /// Reads a binding the resolver placed at a known position.
    ///
    /// `hops` is how many scopes out to walk before indexing. `name` is checked
    /// against the entry found there, so a resolver that disagrees with the
    /// interpreter about scope shape costs a fallback lookup rather than
    /// returning the wrong variable.
    pub fn get_slot(&self, hops: u16, slot: u32, name: &str) -> Option<Value> {
        let table = self.ancestor(hops)?;
        let entries = table.entries.borrow();
        let entry = entries.get(slot as usize)?;
        if &*entry.name == name {
            Some(entry.value.clone())
        } else {
            None
        }
    }

    /// Writes to a binding the resolver placed at a known position.
    ///
    /// `Err` hands the value back when the slot does not hold that name, so the
    /// caller can fall back to a search without having lost it.
    pub fn assign_slot(
        &self,
        hops: u16,
        slot: u32,
        name: &str,
        value: Value,
        matches: &dyn Fn(&Value, &Type) -> bool,
    ) -> Result<AssignOutcome, Value> {
        let Some(table) = self.ancestor(hops) else {
            return Err(value);
        };
        let mut entries = table.entries.borrow_mut();
        let Some(entry) = entries.get_mut(slot as usize) else {
            return Err(value);
        };

        if &*entry.name != name {
            return Err(value);
        }
        if entry.is_constant {
            return Ok(AssignOutcome::Constant);
        }
        if let Some(declared) = &entry.declared_type {
            if *declared != Type::Unknown && !matches(&value, declared) {
                return Ok(AssignOutcome::TypeMismatch {
                    expected: declared.clone(),
                    found: Value::get_type_name(&value),
                });
            }
        }

        entry.value = value;
        Ok(AssignOutcome::Stored)
    }

    /// The table `hops` scopes out from this one.
    fn ancestor(&self, hops: u16) -> Option<&SymbolTable> {
        let mut table = self;
        for _ in 0..hops {
            table = table.parent.as_deref()?;
        }
        Some(table)
    }

    // -- name access -------------------------------------------------------

    /// Gets a value from the symbol table (searching parents)
    pub fn get(&self, name: &str) -> Option<Value> {
        if let Some(&slot) = self.index.borrow().get(name) {
            return self.entries.borrow().get(slot as usize).map(|e| e.value.clone());
        }
        match &self.parent {
            Some(parent) => parent.get(name),
            None => None,
        }
    }

    /// Takes a value out of the table, leaving `Null` behind.
    ///
    /// This exists so a mutation can be done in place. `xs.append(x)` reads
    /// `xs`, changes it, and writes it back; if the table still held its own
    /// reference to the elements while the change happened, the copy-on-write
    /// in [`List::elements_mut`](crate::values::List::elements_mut) would see a
    /// shared list and copy the whole thing. Emptying the binding first makes
    /// the caller the only holder, so the append is O(1) and filling a list is
    /// linear rather than quadratic.
    ///
    /// The caller is responsible for putting a value back. The window in
    /// between is not observable from Xenith: nothing else runs during it, and
    /// an error in the middle ends the program.
    pub fn take(&self, name: &str) -> Option<Value> {
        if let Some(&slot) = self.index.borrow().get(name) {
            let mut entries = self.entries.borrow_mut();
            let entry = entries.get_mut(slot as usize)?;
            return Some(std::mem::replace(&mut entry.value, Value::Null));
        }
        match &self.parent {
            Some(parent) => parent.take(name),
            None => None,
        }
    }

    /// Gets a value and says where it was found, so the caller can remember the
    /// position and go straight there next time.
    pub fn locate(&self, name: &str) -> Option<(u16, u32, Value)> {
        let mut table = self;
        let mut hops: u16 = 0;

        loop {
            if let Some(&slot) = table.index.borrow().get(name) {
                let value = table.entries.borrow().get(slot as usize)?.value.clone();
                return Some((hops, slot, value));
            }
            table = table.parent.as_deref()?;
            hops = hops.checked_add(1)?;
        }
    }

    /// Where a name lives, without reading it. Used to prime the cache on an
    /// assignment, where the value is being replaced rather than read.
    pub fn locate_binding(&self, name: &str) -> Option<(u16, u32)> {
        let mut table = self;
        let mut hops: u16 = 0;

        loop {
            if let Some(&slot) = table.index.borrow().get(name) {
                return Some((hops, slot));
            }
            table = table.parent.as_deref()?;
            hops = hops.checked_add(1)?;
        }
    }

    /// Declares or overwrites a binding in this scope.
    fn put(&self, name: &str, value: Value, declared_type: Option<Type>, is_constant: bool) {
        if let Some(&slot) = self.index.borrow().get(name) {
            let mut entries = self.entries.borrow_mut();
            if let Some(entry) = entries.get_mut(slot as usize) {
                entry.value = value;
                if declared_type.is_some() {
                    entry.declared_type = declared_type;
                }
                if is_constant {
                    entry.is_constant = true;
                }
                return;
            }
        }

        let key: Rc<str> = Rc::from(name);
        let mut entries = self.entries.borrow_mut();
        let slot = entries.len() as u32;
        entries.push(Entry {
            name: Rc::clone(&key),
            value,
            declared_type,
            is_constant,
        });
        self.index.borrow_mut().insert(key, slot);
    }

    /// Sets a value in the current symbol table
    pub fn set(&self, name: String, value: Value) {
        self.put(&name, value, None, false);
    }

    /// Sets a value and its declared type in the current symbol table
    pub fn set_with_type(&self, name: String, value: Value, typ: Type) {
        self.put(&name, value, Some(typ), false);
    }

    /// Sets a value only in the local scope (does not traverse parents)
    pub fn set_local(&self, name: String, value: Value) {
        self.put(&name, value, None, false);
    }

    /// Updates a variable in the scope it was originally defined, or sets in
    /// the current scope if it is not found anywhere.
    pub fn set_existing(&self, name: &str, value: Value) {
        if !self.assign_existing(name, value.clone()) {
            self.put(name, value, None, false);
        }
    }

    /// Checks if a variable exists in this scope or any parent
    pub fn contains(&self, name: &str) -> bool {
        if self.index.borrow().contains_key(name) {
            return true;
        }
        match &self.parent {
            Some(parent) => parent.contains(name),
            None => false,
        }
    }

    /// Gets the declared type of a variable (searching parents)
    pub fn get_declared_type(&self, name: &str) -> Option<Type> {
        if let Some(&slot) = self.index.borrow().get(name) {
            return self
                .entries
                .borrow()
                .get(slot as usize)
                .and_then(|e| e.declared_type.clone());
        }
        match &self.parent {
            Some(parent) => parent.get_declared_type(name),
            None => None,
        }
    }

    /// Marks a name as declared with `const let` in the current scope
    pub fn mark_constant(&self, name: String) {
        if let Some(&slot) = self.index.borrow().get(name.as_str()) {
            if let Some(entry) = self.entries.borrow_mut().get_mut(slot as usize) {
                entry.is_constant = true;
            }
        }
    }

    /// Was this name declared constant, here or in any enclosing scope?
    pub fn is_constant(&self, name: &str) -> bool {
        if let Some(&slot) = self.index.borrow().get(name) {
            return self
                .entries
                .borrow()
                .get(slot as usize)
                .is_some_and(|e| e.is_constant);
        }
        match &self.parent {
            Some(parent) => parent.is_constant(name),
            None => false,
        }
    }

    /// Updates an existing binding in the scope that declared it.
    /// Returns false if the name is not bound anywhere in the chain.
    pub fn assign_existing(&self, name: &str, value: Value) -> bool {
        if let Some(&slot) = self.index.borrow().get(name) {
            if let Some(entry) = self.entries.borrow_mut().get_mut(slot as usize) {
                entry.value = value;
                return true;
            }
        }
        match &self.parent {
            Some(parent) => parent.assign_existing(name, value),
            None => false,
        }
    }

    /// Finds the binding, refuses it if constant, checks the value against the
    /// declared type, and stores it. All in one walk of the chain.
    ///
    /// This is the fallback for an assignment the resolver could not place; see
    /// [`SymbolTable::assign_slot`] for the fast path.
    pub fn assign_checked(
        &self,
        name: &str,
        value: Value,
        matches: &dyn Fn(&Value, &Type) -> bool,
    ) -> AssignOutcome {
        let slot = match self.index.borrow().get(name) {
            Some(&slot) => slot,
            None => {
                return match &self.parent {
                    Some(parent) => parent.assign_checked(name, value, matches),
                    None => AssignOutcome::NotDeclared,
                };
            }
        };

        let mut entries = self.entries.borrow_mut();
        let Some(entry) = entries.get_mut(slot as usize) else {
            return AssignOutcome::NotDeclared;
        };

        if entry.is_constant {
            return AssignOutcome::Constant;
        }
        if let Some(declared) = &entry.declared_type {
            if *declared != Type::Unknown && !matches(&value, declared) {
                return AssignOutcome::TypeMismatch {
                    expected: declared.clone(),
                    found: Value::get_type_name(&value),
                };
            }
        }

        entry.value = value;
        AssignOutcome::Stored
    }

    /// Drops every binding declared in this scope, leaving parents untouched.
    /// Lets a loop body reuse one scope across iterations instead of
    /// allocating a fresh symbol table each time round.
    pub fn clear_local(&self) {
        self.entries.borrow_mut().clear();
        self.index.borrow_mut().clear();
    }
}

/// What an assignment did.
pub enum AssignOutcome {
    Stored,
    NotDeclared,
    Constant,
    /// Both sides of the mismatch, built only on this path so a successful
    /// assignment never pays for the strings.
    TypeMismatch { expected: Type, found: String },
}

impl Default for SymbolTable {
    fn default() -> Self {
        Self::new()
    }
}
