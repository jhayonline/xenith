//! A compiled method, and the cells it captured.

use std::cell::RefCell;
use std::rc::Rc;

use crate::values::Value;
use crate::vm::chunk::Chunk;

/// A proto together with what it captured.
///
/// The proto is shared: every closure `make_adder` returns points at the same
/// compiled body, and only the captured cells differ. That is the whole
/// difference from `values::Function`, which carries an `Rc<Node>` body and
/// an `Rc<Context>` -- a whole symbol table -- per closure.
#[derive(Debug)]
pub struct Closure {
    pub proto: Rc<Chunk>,
    /// One cell per entry in `proto.upvalues`, in the same order, so
    /// `GET_UPVAL 2` indexes this directly.
    pub upvalues: Vec<Rc<RefCell<Upvalue>>>,
}

impl Closure {
    /// The name the method was written under, for a message that has to name
    /// it. `None` for an anonymous method.
    pub fn name(&self) -> Option<&str> {
        self.proto.name.as_deref()
    }
}

/// A captured binding.
///
/// **Open** while the frame owning the register is alive: the capture *is*
/// the register, so a write through the variable is seen through the closure
/// and the other way round. That is not a detail of the implementation, it is
/// what Xenith means -- `tests/cases/closures.xen` assigns `step = 50` after
/// `advance` is written and expects `advance(0)` to answer 50.
///
/// **Closed** once that frame goes away, taking a copy of the value with it,
/// so `make_adder(10)` keeps its own `n` after `make_adder` has returned.
///
/// This is Lua's design. The alternative -- capturing by copy at closure
/// creation -- loses the first case; capturing the whole enclosing scope --
/// what the tree walker does -- keeps both but costs a symbol table per
/// closure.
#[derive(Debug)]
pub enum Upvalue {
    /// An absolute index into the VM's register stack.
    Open(usize),
    Closed(Value),
}
