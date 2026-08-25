//! The instruction set and the container it lives in.

use crate::position::Position;
use crate::values::Value;

/// A register index within one frame.
///
/// `u8` caps a frame at 256 registers, which is locals plus the deepest
/// expression nesting in any one statement. A function needing more is
/// rejected by the compiler as unsupported rather than silently wrapping.
pub type Reg = u8;

/// An index into [`Chunk::constants`].
pub type ConstIdx = u16;

/// An index into [`Chunk::code`], for jump targets.
pub type Addr = u32;

/// One instruction.
///
/// Three-address, in the manner of Lua: every arithmetic instruction names its
/// destination and both operands, so no shuffling is needed around it. That is
/// worth roughly 40% fewer dispatches than a stack machine for the same work.
///
/// Kept to 8 bytes. `tests/layout.rs` enforces it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Instr {
    /// `dst = constants[k]`
    LoadConst { dst: Reg, k: ConstIdx },
    /// `dst = value`. Booleans do not need a constant slot.
    LoadBool { dst: Reg, value: bool },
    /// `dst = null`
    LoadNull { dst: Reg },
    /// `dst = src`
    Move { dst: Reg, src: Reg },

    // Binary operators. Every one of these dispatches to the matching method
    // on `Value`, so the VM and the tree walker cannot disagree about what
    // `+` means -- there is one implementation and both call it.
    Add { dst: Reg, a: Reg, b: Reg },
    Sub { dst: Reg, a: Reg, b: Reg },
    Mul { dst: Reg, a: Reg, b: Reg },
    Div { dst: Reg, a: Reg, b: Reg },
    Rem { dst: Reg, a: Reg, b: Reg },
    Pow { dst: Reg, a: Reg, b: Reg },
    Eq { dst: Reg, a: Reg, b: Reg },
    Ne { dst: Reg, a: Reg, b: Reg },
    Lt { dst: Reg, a: Reg, b: Reg },
    Gt { dst: Reg, a: Reg, b: Reg },
    Le { dst: Reg, a: Reg, b: Reg },
    Ge { dst: Reg, a: Reg, b: Reg },

    /// `dst = -src`
    Neg { dst: Reg, src: Reg },
    /// `dst = !src`
    Not { dst: Reg, src: Reg },

    /// `ip = to`
    Jump { to: Addr },
    /// `if !cond { ip = to }`
    JumpIfFalse { cond: Reg, to: Addr },
    /// `if cond { ip = to }`
    JumpIfTrue { cond: Reg, to: Addr },

    /// Writes `src` to standard output, followed by a newline.
    ///
    /// An opcode rather than a call, because phase 3 has no calling
    /// convention. Phase 4 may fold it back into `Call`; the differential
    /// harness will say whether the output changed.
    Echo { src: Reg },

    /// Stops, with `src` as the chunk's value.
    Halt { src: Reg },
}

/// A compiled unit of code.
#[derive(Debug, Clone, Default)]
pub struct Chunk {
    pub code: Vec<Instr>,
    pub constants: Vec<Value>,
    /// The frame size this chunk needs: the high-water mark of register use.
    pub registers: u16,
    /// Sparse and sorted by instruction index. An entry is recorded only where
    /// the position changes.
    ///
    /// A parallel array was rejected: at 56 bytes per `Position`, a 10,000
    /// instruction chunk would carry 560KB of them, on the hot path, to serve
    /// the rare case of a trap firing.
    pub positions: Vec<(Addr, Position)>,
}

impl Chunk {
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends an instruction and returns its address, for later patching.
    pub fn push(&mut self, instr: Instr) -> Addr {
        self.code.push(instr);
        (self.code.len() - 1) as Addr
    }

    /// Interns a constant, reusing an identical one already present.
    ///
    /// Dedup matters for loops: `while i < 300000` would otherwise add a
    /// constant per compile of the same literal.
    pub fn add_constant(&mut self, value: Value) -> ConstIdx {
        if let Some(existing) = self
            .constants
            .iter()
            .position(|c| c.eq_for_constants(&value))
        {
            return existing as ConstIdx;
        }
        self.constants.push(value);
        (self.constants.len() - 1) as ConstIdx
    }

    /// Records where an instruction came from, if it differs from the last
    /// entry. Sparse by construction.
    pub fn record_position(&mut self, at: Addr, position: &Position) {
        if let Some((_, last)) = self.positions.last() {
            if last.index == position.index && last.file_name == position.file_name {
                return;
            }
        }
        self.positions.push((at, position.clone()));
    }

    /// The position of the instruction at `at`, or the nearest one before it.
    ///
    /// Binary search over a sparse table, run only when a trap fires.
    pub fn position_at(&self, at: Addr) -> Option<&Position> {
        if self.positions.is_empty() {
            return None;
        }
        let found = match self.positions.binary_search_by_key(&at, |(addr, _)| *addr) {
            Ok(exact) => exact,
            // `Err(0)` means the trap is before the first recorded position,
            // which cannot happen for a chunk built by the compiler but is
            // handled rather than panicking.
            Err(0) => return None,
            Err(after) => after - 1,
        };
        Some(&self.positions[found].1)
    }
}
