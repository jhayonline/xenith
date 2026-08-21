//! What the checker inferred, kept so a later pass can use it.
//!
//! The checker's `infer` already computes a type for every expression and
//! throws it away. Recording it is what lets a compiler emit `ADD_I64` where
//! both operands are known to be ints, instead of a generic opcode that
//! matches on a pair of values at run time.
//!
//! The governing rule is the checker's own: an absent or `Unknown` entry is
//! always safe, because the generic path does exactly what the tree walker
//! does today. A *wrong* entry is a wrong opcode, so nothing may be recorded
//! that the checker did not prove.

use crate::nodes::NodeId;
use crate::types::Type;

/// Inferred types, indexed by [`NodeId`].
#[derive(Debug, Clone, Default)]
pub struct TypeTable {
    /// Dense, because ids are handed out consecutively by the parser.
    types: Vec<Type>,
}

impl TypeTable {
    pub fn with_capacity(node_count: u32) -> Self {
        Self {
            types: vec![Type::Unknown; node_count as usize],
        }
    }

    /// The type recorded for a node, or `Unknown` if none was.
    ///
    /// `NodeId::UNSET` and any id past the end read as `Unknown` rather than
    /// panicking, so a caller never has to check first.
    pub fn get(&self, id: NodeId) -> &Type {
        self.types.get(id.0 as usize).unwrap_or(&Type::Unknown)
    }

    /// Records a type. An `UNSET` id is ignored.
    pub fn record(&mut self, id: NodeId, ty: Type) {
        if id == NodeId::UNSET {
            return;
        }
        if let Some(slot) = self.types.get_mut(id.0 as usize) {
            *slot = ty;
        }
    }

    /// How many nodes the table has room for.
    pub fn len(&self) -> usize {
        self.types.len()
    }

    pub fn is_empty(&self) -> bool {
        self.types.is_empty()
    }
}
