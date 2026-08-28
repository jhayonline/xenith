//! The instruction set and the container it lives in.

use std::rc::Rc;

use crate::position::Position;
use crate::types::Type;
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

    // Typed operators, chosen by the compiler from what the checker proved.
    //
    // Each is *guarded*: it matches the pair of variants it expects and falls
    // through to the generic opcode's own path on anything else. So a wrong
    // `TypeTable` entry costs a program speed and never correctness, which is
    // the safety argument for emitting these at all.
    //
    // The win is not the skipped tag match -- `Value::add` matches `(Int, Int)`
    // first anyway. It is that the fast path writes its answer straight into a
    // register and never builds the `Result` that would have carried it.
    //
    // `POW` gets no typed form: integer exponentiation has a negative-exponent
    // case and an overflow case whose wording lives in `Value::power`, and it
    // is never on a hot path worth a second copy of them.
    AddI { dst: Reg, a: Reg, b: Reg },
    SubI { dst: Reg, a: Reg, b: Reg },
    MulI { dst: Reg, a: Reg, b: Reg },
    DivI { dst: Reg, a: Reg, b: Reg },
    RemI { dst: Reg, a: Reg, b: Reg },
    LtI { dst: Reg, a: Reg, b: Reg },
    GtI { dst: Reg, a: Reg, b: Reg },
    LeI { dst: Reg, a: Reg, b: Reg },
    GeI { dst: Reg, a: Reg, b: Reg },
    EqI { dst: Reg, a: Reg, b: Reg },
    NeI { dst: Reg, a: Reg, b: Reg },

    // The float half. Not a mirror of the int half, because floats are not
    // ints with a different tag:
    //
    // - Float arithmetic cannot overflow. IEEE saturates to infinity and the
    //   generic opcode allows it, so there is no `checked_*` and no XEN017.
    // - Float division by zero *does* raise. `Number::is_zero` answers true
    //   for `0.0`, so `1.0 / 0.0` is XEN003 in this language and not `inf`.
    // - Float ordering can fail. `compare` goes through `partial_cmp` and
    //   turns `None` into "cannot compare NaN", so `LT_F` cannot be a plain
    //   `<`. Equality is the exception: `eq_value` uses `==`, where NaN is
    //   simply unequal and never an error.
    //
    // There is no `REM_F`: float remainder has its own rounding rule in
    // `Value::modulo` and is not hot.
    AddF { dst: Reg, a: Reg, b: Reg },
    SubF { dst: Reg, a: Reg, b: Reg },
    MulF { dst: Reg, a: Reg, b: Reg },
    DivF { dst: Reg, a: Reg, b: Reg },
    LtF { dst: Reg, a: Reg, b: Reg },
    GtF { dst: Reg, a: Reg, b: Reg },
    LeF { dst: Reg, a: Reg, b: Reg },
    GeF { dst: Reg, a: Reg, b: Reg },
    EqF { dst: Reg, a: Reg, b: Reg },
    NeF { dst: Reg, a: Reg, b: Reg },

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

    /// `dst` = a closure over `protos[proto]`, capturing what its `upvalues`
    /// table says to capture.
    Closure { dst: Reg, proto: u16 },
    /// Returns `src` to the caller. The top level ends in `Halt` instead.
    Ret { src: Reg },

    /// `dst = callee(callee+1 ..= callee+argc)`.
    ///
    /// The callee sits in `callee` and its arguments in the registers
    /// immediately above it, and the callee's frame begins at `callee + 1` --
    /// so the arguments already *are* the callee's parameters and a call
    /// copies nothing. The result is written to `dst` once the callee
    /// returns, which is always `callee` as the compiler emits it; the field
    /// is kept because a future direct-to-destination call is then a compiler
    /// change and not an instruction change.
    Call { dst: Reg, callee: Reg, argc: u8 },

    /// `dst = upvalues[idx]`
    GetUpval { dst: Reg, idx: u8 },
    /// `upvalues[idx] = src`
    SetUpval { idx: u8, src: Reg },
    /// Closes every open upvalue watching `from` or above, so the closures
    /// holding them stop sharing a register that is about to be reused.
    ///
    /// Emitted on scope exit, and on the `break` and `continue` that leave a
    /// scope without reaching its end.
    CloseUpvals { from: Reg },

    /// Stops, with `src` as the chunk's value.
    Halt { src: Reg },
}

/// How a closure finds one of its captures, at the moment it is created.
///
/// Two cases, which is why this is a flag rather than an enum with payloads:
/// the value is either still in the enclosing frame, where it is a register,
/// or the enclosing function had already captured it, where it is one of that
/// function's own upvalues. The chain of the second case is what lets a
/// method capture a name from two functions out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UpvalDesc {
    pub in_parent_locals: bool,
    pub index: u8,
}

/// A compiled unit of code.
#[derive(Debug, Clone, Default)]
pub struct Chunk {
    pub code: Vec<Instr>,
    pub constants: Vec<Value>,
    /// The frame size this chunk needs: the high-water mark of register use.
    pub registers: u16,
    /// Sparse and sorted by instruction index. An entry is recorded only where
    /// the span changes.
    ///
    /// Both ends, not just the start: the tree walker underlines the whole
    /// expression a trap came from, and a report that underlined one character
    /// would not be byte-identical to it.
    ///
    /// A parallel array was rejected: at 56 bytes per `Position`, a 10,000
    /// instruction chunk would carry 560KB of them, on the hot path, to serve
    /// the rare case of a trap firing.
    pub positions: Vec<(Addr, Position, Position)>,
    /// The name this body was written under. `None` for the top level and for
    /// an anonymous method. Read only by XEN019, whose message names the
    /// method whose call was refused.
    pub name: Option<String>,
    /// How many parameters. Checked at run time rather than compile time,
    /// because a callee reached through a variable is not known until it is
    /// in a register.
    pub arity: u8,
    /// The declared parameter types, checked once per call exactly as
    /// `Function::execute` checks them. Shared rather than owned: a proto is
    /// behind an `Rc` already and this must not be copied per call.
    pub param_types: Rc<Vec<Type>>,
    /// What a closure over this proto captures, in the order `GET_UPVAL`
    /// indexes them. Held on the proto rather than emitted as pseudo-
    /// instructions after `CLOSURE`, which is how Lua does it: an `Instr`
    /// that is not an instruction would have to be skipped by every loop that
    /// walks the code, including the disassembler.
    pub upvalues: Vec<UpvalDesc>,
    /// Nested function bodies, indexed by `Instr::Closure`.
    pub protos: Vec<Rc<Chunk>>,
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

    /// Records the span an instruction came from, if it differs from the last
    /// entry. Sparse by construction.
    pub fn record_position(&mut self, at: Addr, start: &Position, end: &Position) {
        if let Some((_, last_start, last_end)) = self.positions.last() {
            if last_start.index == start.index
                && last_end.index == end.index
                && last_start.file_name == start.file_name
            {
                return;
            }
        }
        self.positions.push((at, start.clone(), end.clone()));
    }

    /// The span of the instruction at `at`, or the nearest one before it.
    ///
    /// Binary search over a sparse table, run only when a trap fires.
    pub fn position_at(&self, at: Addr) -> Option<(&Position, &Position)> {
        if self.positions.is_empty() {
            return None;
        }
        let found = match self.positions.binary_search_by_key(&at, |(addr, _, _)| *addr) {
            Ok(exact) => exact,
            // `Err(0)` means the trap is before the first recorded span, which
            // cannot happen for a chunk built by the compiler but is handled
            // rather than panicking.
            Err(0) => return None,
            Err(after) => after - 1,
        };
        let (_, start, end) = &self.positions[found];
        Some((start, end))
    }
}
