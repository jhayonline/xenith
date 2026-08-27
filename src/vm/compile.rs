//! The AST, lowered to a chunk.
//!
//! ## Register allocation
//!
//! There is barely any. Locals occupy registers `0..n` in declaration order.
//! Expression temporaries come from a counter above the locals, released at
//! the end of each statement. The high-water mark of that counter is the
//! frame size. A tree walker cannot do this -- it cannot know the shape of a
//! scope before it reaches it, which is the entire reason `SlotCache` exists
//! -- but a compiler does not predict the shape, it assigns it.
//!
//! ## Partial coverage
//!
//! Anything this phase does not handle returns [`Unsupported`], and the caller
//! runs the tree walker. That is what makes a partial VM safe to ship: a
//! program cannot break by failing to compile.

use std::rc::Rc;

use crate::nodes::Node;
use crate::position::Position;
use crate::types::Type;
use crate::values::Value;
use crate::vm::chunk::{Addr, Chunk, Instr, Reg, UpvalDesc};

/// Why the compiler gave up. Not an error: the caller runs the tree walker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unsupported {
    pub what: String,
}

impl Unsupported {
    fn new(what: &str) -> Self {
        Self {
            what: what.to_string(),
        }
    }
}

/// Lowers a whole file.
pub fn compile(ast: &Node) -> Result<Chunk, Unsupported> {
    let mut compiler = Compiler {
        functions: vec![FnState::top_level()],
    };

    let value = compiler.program(ast)?;
    compiler.emit(Instr::Halt { src: value }, ast.position_start(), ast.position_end());

    Ok(compiler.finish_function())
}

/// One enclosing loop, for `break` and `continue` to find.
struct LoopContext {
    /// Where the body jumps back to at the end of a pass.
    start: Addr,
    /// Where `continue` goes. The condition for a `while`; the step for a
    /// classic `for`, because skipping the step would never advance the
    /// induction variable and the loop would not terminate.
    continue_to: Addr,
    /// Jumps out, patched to just past the loop when it closes.
    breaks: Vec<Addr>,
    /// Scope depth outside the loop body, so a `break` from inside a nested
    /// `when` still knows how far to unwind. Nothing reads it until phase 4,
    /// where closing upvalues will need it.
    #[allow(dead_code)]
    depth: usize,
}

/// A local binding, resolved to a register at compile time.
struct Local {
    name: String,
    reg: Reg,
    /// Scope nesting depth, so `end_scope` knows which to drop.
    depth: usize,
    is_constant: bool,
    /// Set when a nested method captures this. Task 10 reads it to decide
    /// whether leaving the scope has to close anything.
    captured: bool,
}

/// One function being compiled.
///
/// A nested method pushes another of these. The fields were `Compiler`'s own
/// until phase 4: a method needs its own registers, its own locals and its
/// own loop stack, and it needs the enclosing function's locals to still be
/// there so it can capture them.
struct FnState {
    chunk: Chunk,
    /// Next free register. Locals sit below it permanently; temporaries above
    /// it are handed out and taken back within a statement.
    reg_top: u16,
    /// The largest `reg_top` ever reached, which is the frame size.
    high_water: u16,
    locals: Vec<Local>,
    depth: usize,
    loops: Vec<LoopContext>,
    /// What this function captures, in the order `GET_UPVAL` indexes them.
    /// Task 9 fills it; `finish_function` moves it onto the chunk either way.
    upvalues: Vec<UpvalDesc>,
}

impl FnState {
    /// The top level, which is a function in every respect except that it
    /// ends in `Halt` rather than `Ret` and captures nothing.
    fn top_level() -> Self {
        Self {
            chunk: Chunk::new(),
            reg_top: 0,
            high_water: 0,
            locals: Vec::new(),
            depth: 0,
            loops: Vec::new(),
            upvalues: Vec::new(),
        }
    }

    /// A nested method body.
    fn for_function(name: Option<String>, param_types: &[Type], arity: u8) -> Self {
        let mut state = Self::top_level();
        state.chunk.name = name;
        state.chunk.arity = arity;
        state.chunk.param_types = Rc::new(param_types.to_vec());
        state
    }
}

struct Compiler {
    /// Innermost last. Never empty: the top level is `functions[0]`.
    functions: Vec<FnState>,
}

impl Compiler {
    /// The function being compiled right now.
    ///
    /// `expect` rather than an `Option`: `compile` pushes the top level
    /// before anything else runs and `finish_function` is the only thing that
    /// pops, so an empty stack is a compiler bug, not a program error.
    fn state(&mut self) -> &mut FnState {
        self.functions
            .last_mut()
            .expect("a function is always being compiled")
    }

    fn state_ref(&self) -> &FnState {
        self.functions
            .last()
            .expect("a function is always being compiled")
    }

    /// Pops the innermost function and hands back its finished chunk.
    fn finish_function(&mut self) -> Chunk {
        let state = self
            .functions
            .pop()
            .expect("a function is always being compiled");
        let mut chunk = state.chunk;
        chunk.registers = state.high_water;
        chunk.upvalues = state.upvalues;
        chunk
    }

    /// Takes the next register.
    fn alloc(&mut self) -> Result<Reg, Unsupported> {
        let state = self.state();
        if state.reg_top >= 256 {
            return Err(Unsupported::new("more than 256 registers in one frame"));
        }
        let reg = state.reg_top as Reg;
        state.reg_top += 1;
        if state.reg_top > state.high_water {
            state.high_water = state.reg_top;
        }
        Ok(reg)
    }

    /// The innermost binding of a name, or `None`.
    ///
    /// Searched from the end so an inner scope shadows an outer one, which is
    /// the same order `SymbolTable` walks -- except that this happens once, at
    /// compile time, rather than on every read.
    fn resolve(&self, name: &str) -> Option<&Local> {
        self.state_ref().locals.iter().rev().find(|local| local.name == name)
    }

    /// Finds `name` as a capture of the function at `level`, adding an
    /// upvalue entry at every level it passes through.
    ///
    /// Lua's algorithm. Either the enclosing function has the name as a
    /// local, in which case this function captures that register directly, or
    /// the enclosing function has to capture it first and this function
    /// captures *that* upvalue. The second case is the recursion, and it is
    /// what lets a method reach a name from two functions out.
    fn resolve_upvalue(&mut self, level: usize, name: &str) -> Result<Option<u8>, Unsupported> {
        if level == 0 {
            return Ok(None);
        }

        // Searched from the end, so an inner scope of the enclosing function
        // shadows an outer one -- the same order `resolve` walks.
        if let Some(at) = self.functions[level - 1]
            .locals
            .iter()
            .rposition(|local| local.name == name)
        {
            self.functions[level - 1].locals[at].captured = true;
            let reg = self.functions[level - 1].locals[at].reg;
            return self
                .add_upvalue(
                    level,
                    UpvalDesc {
                        in_parent_locals: true,
                        index: reg,
                    },
                )
                .map(Some);
        }

        if let Some(index) = self.resolve_upvalue(level - 1, name)? {
            return self
                .add_upvalue(
                    level,
                    UpvalDesc {
                        in_parent_locals: false,
                        index,
                    },
                )
                .map(Some);
        }

        Ok(None)
    }

    /// Interns a capture, so two reads of one name share a cell rather than
    /// opening two that would then disagree after a write.
    fn add_upvalue(&mut self, level: usize, desc: UpvalDesc) -> Result<u8, Unsupported> {
        let upvalues = &mut self.functions[level].upvalues;
        if let Some(existing) = upvalues.iter().position(|u| *u == desc) {
            return Ok(existing as u8);
        }
        if upvalues.len() >= 256 {
            return Err(Unsupported::new("more than 256 captures in one method"));
        }
        upvalues.push(desc);
        Ok((upvalues.len() - 1) as u8)
    }

    /// The innermost binding of a name in *any* enclosing function.
    ///
    /// Only for the questions that are about the binding rather than about
    /// how to reach it -- whether it is a constant, in particular, which is
    /// the tree walker's to report either way.
    fn find_any(&self, name: &str) -> Option<&Local> {
        self.functions
            .iter()
            .rev()
            .find_map(|state| state.locals.iter().rev().find(|local| local.name == name))
    }

    fn begin_scope(&mut self) {
        self.state().depth += 1;
    }

    /// Drops the bindings this scope introduced, and frees their registers.
    fn end_scope(&mut self) {
        while let Some(last) = self.state_ref().locals.last() {
            if last.depth < self.state_ref().depth {
                break;
            }
            let reg = last.reg;
            self.state().locals.pop();
            // Registers are handed out in order, so the last local always
            // holds the highest register and this stays a simple decrement.
            self.state().reg_top = reg as u16;
        }
        self.state().depth -= 1;
    }

    /// Gives back every register above `mark`.
    ///
    /// Called at the end of each statement, which is what keeps a long
    /// function from needing one register per subexpression it ever
    /// evaluates.
    /// The first register no live local occupies.
    ///
    /// Locals are handed out in declaration order and never move, so the last
    /// one holds the highest register. A statement's temporaries must be
    /// released down to here and no further: releasing to a bare statement
    /// mark would hand a local's register back, and the next statement would
    /// overwrite the binding.
    fn locals_floor(&self) -> u16 {
        self.state_ref().locals.last().map_or(0, |local| local.reg as u16 + 1)
    }

    fn free_to(&mut self, mark: u16) {
        self.state().reg_top = mark;
    }

    /// Emits a jump whose target is not known yet, and returns its address so
    /// [`patch`] can fill it in.
    fn emit_jump(&mut self, instr: Instr, start: &Position, end: &Position) -> Addr {
        self.emit(instr, start, end)
    }

    /// Points a previously emitted jump at the next instruction to be emitted.
    fn patch(&mut self, at: Addr) {
        let state = self.state();
        let target = state.chunk.code.len() as Addr;
        match &mut state.chunk.code[at as usize] {
            Instr::Jump { to } | Instr::JumpIfFalse { to, .. } | Instr::JumpIfTrue { to, .. } => {
                *to = target;
            }
            other => unreachable!("patched a {:?}, which is not a jump", other),
        }
    }

    /// Appends an instruction and records the source span it came from.
    ///
    /// Both ends, because a trap's caret must underline the whole expression,
    /// exactly as `visit_binary_op` makes it.
    fn emit(&mut self, instr: Instr, start: &Position, end: &Position) -> Addr {
        let state = self.state();
        let at = state.chunk.push(instr);
        state.chunk.record_position(at, start, end);
        at
    }

    /// The top level: a statement list, whose value is the last statement's.
    fn program(&mut self, ast: &Node) -> Result<Reg, Unsupported> {
        let Node::List(statements) = ast else {
            return Err(Unsupported::new("a top level that is not a statement list"));
        };
        self.block(statements)
    }

    /// A statement list: the top level, or the body of a `when` or a loop.
    ///
    /// Its value is the last statement's. Only that one survives -- releasing
    /// to the mark and re-allocating hands back the same register, so keeping
    /// an earlier statement's value would mean the next statement overwrote
    /// the very thing being held.
    fn block(&mut self, n: &crate::nodes::ListNode) -> Result<Reg, Unsupported> {
        let mut last: Option<Reg> = None;
        let count = n.element_nodes.len();

        for (i, statement) in n.element_nodes.iter().enumerate() {
            let mark = self.state().reg_top;
            let reg = self.stmt(statement)?;

            // A `let` inside this statement took a register permanently, so
            // the mark it started from is now below the locals.
            let mark = mark.max(self.locals_floor());

            if i + 1 != count {
                self.free_to(mark);
                continue;
            }

            last = match reg {
                Some(reg) => {
                    // Moved down to the mark so it outlives the temporaries
                    // the statement used to compute it.
                    self.free_to(mark);
                    let kept = self.alloc()?;
                    if kept != reg {
                        self.emit(
                            Instr::Move { dst: kept, src: reg },
                            statement.position_start(), statement.position_end(),
                        );
                    }
                    Some(kept)
                }
                None => {
                    self.free_to(mark);
                    None
                }
            };
        }

        match last {
            Some(reg) => Ok(reg),
            None => {
                let dst = self.alloc()?;
                self.emit(Instr::LoadNull { dst }, &n.position_start, &n.position_end);
                Ok(dst)
            }
        }
    }

    /// A body whose value is discarded, as a loop body's is.
    ///
    /// `block` ends by moving the last statement's value somewhere it will
    /// survive. In a loop that move runs on every pass to produce a value
    /// nothing can read, so the loop compiles its body through this instead.
    fn body_for_effect(&mut self, node: &Node) -> Result<(), Unsupported> {
        let statements = match node {
            Node::List(n) => &n.element_nodes,
            other => return self.stmt(other).map(|_| ()),
        };

        for statement in statements {
            let mark = self.state().reg_top;
            self.stmt(statement)?;
            // A `let` in the body took a register permanently.
            self.free_to(mark.max(self.locals_floor()));
        }
        Ok(())
    }

    /// The body of a `when` or a loop: a braced block, or a single statement.
    ///
    /// `statements()` in the parser builds a plain `ListNode` for a block --
    /// the same node a list literal uses, with no flag telling them apart. The
    /// call site is the discriminator instead of the contents: a body is
    /// reached only from `block()` or `statement()` in the parser, and a
    /// braceless body cannot be a bare list literal, because `when c [1, 2]`
    /// reads as an index on `c` rather than a condition and a body.
    fn body(&mut self, node: &Node) -> Result<Reg, Unsupported> {
        match node {
            Node::List(n) => self.block(n),
            other => match self.stmt(other)? {
                Some(reg) => Ok(reg),
                None => {
                    let dst = self.alloc()?;
                    self.emit(Instr::LoadNull { dst }, other.position_start(), other.position_end());
                    Ok(dst)
                }
            },
        }
    }

    /// One statement. `Ok(None)` means it produced no value.
    ///
    /// Later tasks add arms here: locals (task 6), `when` (task 8), `while`
    /// (task 9), classic `for` (task 10), `echo` (task 7).
    fn stmt(&mut self, node: &Node) -> Result<Option<Reg>, Unsupported> {
        match node {
            Node::Grab(_) => Err(Unsupported::new("an import")),
            Node::Export(_) => Err(Unsupported::new("an export")),
            Node::StructDef(_) => Err(Unsupported::new("a struct declaration")),
            Node::EnumDef(_) => Err(Unsupported::new("an enum declaration")),
            Node::TypeAlias(_) => Err(Unsupported::new("a type alias")),
            Node::Return(n) => {
                // The top level is not a function. `functions.len() == 1`
                // means nothing has been pushed, so this is the top level and
                // the message the tree walker prints is the one that should
                // be printed.
                if self.functions.len() == 1 {
                    return Err(Unsupported::new("release outside a method"));
                }

                let src = match &n.node_to_return {
                    Some(expr) => self.operand(expr)?,
                    None => {
                        // `release` with nothing after it returns null, which
                        // is what `visit_return` does with a `None` node.
                        let dst = self.alloc()?;
                        self.emit(Instr::LoadNull { dst }, &n.position_start, &n.position_end);
                        dst
                    }
                };

                self.emit(Instr::Ret { src }, &n.position_start, &n.position_end);
                // No `free_to` here, as with `break` and `continue`: the
                // caller frees to its own mark, and this statement has no
                // value for it to keep.
                Ok(None)
            }
            Node::VarAssign(n) => self.var_assign(n).map(Some),
            Node::Break(n) => {
                let Some(context) = self.state().loops.last() else {
                    return Err(Unsupported::new("break outside a loop"));
                };
                let _ = context;
                let jump = self.emit_jump(Instr::Jump { to: 0 }, &n.position_start, &n.position_end);
                self.state().loops
                    .last_mut()
                    .expect("checked above")
                    .breaks
                    .push(jump);
                Ok(None)
            }

            Node::Continue(n) => {
                let Some(context) = self.state().loops.last() else {
                    return Err(Unsupported::new("continue outside a loop"));
                };
                let target = context.continue_to;
                self.emit(Instr::Jump { to: target }, &n.position_start, &n.position_end);
                Ok(None)
            }

            other => self.expr(other).map(Some),
        }
    }

    /// An expression in a read-only operand position.
    ///
    /// A local read this way needs no copy: an instruction reads its operand
    /// registers before it writes its destination, and the destination can
    /// never be a local's register. Every caller allocates its destination
    /// after `free_to(mark)`, and `mark` is `reg_top`, which is never below
    /// `locals_floor` -- so a destination is always strictly above every live
    /// local. That invariant is what makes eliding the copy safe, and it is
    /// why this must not be used where the result is stored or returned.
    ///
    /// Worth five of the thirteen instructions in the counting loop's body.
    fn operand(&mut self, node: &Node) -> Result<Reg, Unsupported> {
        if let Node::VarAccess(n) = node {
            if let Some(name) = n.variable_name_token.value.as_deref() {
                if let Some(local) = self.resolve(name) {
                    return Ok(local.reg);
                }
            }
        }
        self.expr(node)
    }

    /// One expression, into a fresh register.
    ///
    /// Later tasks add arms: names (task 6), operators (task 5), `when` as an
    /// expression (task 8).
    fn expr(&mut self, node: &Node) -> Result<Reg, Unsupported> {
        match node {
            Node::Number(n) => {
                let text = n
                    .token
                    .value
                    .as_ref()
                    .ok_or_else(|| Unsupported::new("a number with no text"))?;

                // The same rule `visit_number` uses, so the two agree on what
                // `1e3` is. Anything that does not parse is left to the tree
                // walker, which reports it with its own message.
                let value = if text.contains('.') || text.contains('e') || text.contains('E') {
                    Value::float(
                        text.parse::<f64>()
                            .map_err(|_| Unsupported::new("a float literal that does not parse"))?,
                    )
                } else {
                    Value::int(
                        text.parse::<i64>()
                            .map_err(|_| Unsupported::new("an int literal that does not parse"))?,
                    )
                };

                let dst = self.alloc()?;
                let k = self.state().chunk.add_constant(value);
                self.emit(Instr::LoadConst { dst, k }, &n.position_start, &n.position_end);
                Ok(dst)
            }

            Node::String(n) => {
                let text = n
                    .token
                    .value
                    .clone()
                    .ok_or_else(|| Unsupported::new("a string with no text"))?;
                let dst = self.alloc()?;
                let k = self.state().chunk.add_constant(Value::string(&text));
                self.emit(Instr::LoadConst { dst, k }, &n.position_start, &n.position_end);
                Ok(dst)
            }

            Node::BoolLiteral(n) => {
                let dst = self.alloc()?;
                self.emit(
                    Instr::LoadBool {
                        dst,
                        value: n.value,
                    },
                    &n.position_start, &n.position_end,
                );
                Ok(dst)
            }

            Node::NullLiteral(n) => {
                let dst = self.alloc()?;
                self.emit(Instr::LoadNull { dst }, &n.position_start, &n.position_end);
                Ok(dst)
            }

            Node::If(n) => {
                // Every branch writes its value to the same register, so the
                // whole `when` is one value however many arms it has. That is
                // what makes `let label = when ... ` work.
                let result = self.alloc()?;
                let mut to_end: Vec<Addr> = Vec::new();

                for (condition, body) in &n.cases {
                    let mark = self.state().reg_top;
                    let cond = self.operand(condition)?;
                    let skip = self.emit_jump(
                        Instr::JumpIfFalse { cond, to: 0 },
                        condition.position_start(), condition.position_end(),
                    );
                    self.free_to(mark);

                    self.begin_scope();
                    let value = self.body(body)?;
                    self.emit(Instr::Move { dst: result, src: value }, body.position_start(), body.position_end());
                    self.end_scope();
                    self.free_to(mark);

                    to_end.push(self.emit_jump(Instr::Jump { to: 0 }, body.position_start(), body.position_end()));
                    self.patch(skip);
                }

                match &n.else_case {
                    Some((body, _)) => {
                        let mark = self.state().reg_top;
                        self.begin_scope();
                        let value = self.body(body)?;
                        self.emit(Instr::Move { dst: result, src: value }, body.position_start(), body.position_end());
                        self.end_scope();
                        self.free_to(mark);
                    }
                    None => {
                        // A `when` with no matching arm is null, as it is today.
                        self.emit(Instr::LoadNull { dst: result }, &n.position_start, &n.position_end);
                    }
                }

                for jump in to_end {
                    self.patch(jump);
                }

                Ok(result)
            }

            Node::While(n) => {
                let mark = self.state().reg_top;

                // Where `continue` goes and where the body jumps back to.
                let start = self.state().chunk.code.len() as Addr;

                let cond = self.operand(&n.condition_node)?;
                let exit = self.emit_jump(
                    Instr::JumpIfFalse { cond, to: 0 },
                    n.condition_node.position_start(),
                    n.condition_node.position_end(),
                );
                self.free_to(mark);

                let depth = self.state_ref().depth;
                self.state().loops.push(LoopContext {
                    start,
                    continue_to: start,
                    breaks: Vec::new(),
                    depth,
                });

                self.begin_scope();
                self.body_for_effect(&n.body_node)?;
                self.end_scope();
                self.free_to(mark);

                self.emit(Instr::Jump { to: start }, &n.position_start, &n.position_end);
                self.patch(exit);

                let context = self.state().loops.pop().expect("pushed above");
                for jump in context.breaks {
                    self.patch(jump);
                }

                // A loop is a statement, not an expression: it evaluates to
                // null. The tree walker returns null here too -- it used to
                // collect every iteration's value into a Vec, which nothing
                // could read, and that was removed.
                let dst = self.alloc()?;
                self.emit(Instr::LoadNull { dst }, &n.position_start, &n.position_end);
                Ok(dst)
            }

            Node::ForClassic(n) => {
                // A scope around the whole loop, so `let i` in the init is
                // local to it.
                self.begin_scope();
                let mark = self.state().reg_top;

                if let Some(init) = &n.init_node {
                    let init_mark = self.state().reg_top;
                    self.stmt(init)?;
                    // A `let` in the init keeps its register; anything else
                    // was a temporary.
                    if !matches!(&**init, Node::VarAssign(assign) if assign.is_declaration) {
                        self.free_to(init_mark);
                    }
                }

                // The step is emitted first, jumped over on the way in, and
                // jumped back to from the end of the body. That gives
                // `continue` a fixed address to aim at without a second pass.
                let enter = self.emit_jump(Instr::Jump { to: 0 }, &n.position_start, &n.position_end);

                let step_at = self.state().chunk.code.len() as Addr;
                if let Some(step) = &n.step_node {
                    let step_mark = self.state().reg_top;
                    self.stmt(step)?;
                    self.free_to(step_mark);
                }

                self.patch(enter);

                let exit = match &n.condition_node {
                    Some(condition) => {
                        let cond_mark = self.state().reg_top;
                        let cond = self.operand(condition)?;
                        let jump = self.emit_jump(
                            Instr::JumpIfFalse { cond, to: 0 },
                            condition.position_start(), condition.position_end(),
                        );
                        self.free_to(cond_mark);
                        Some(jump)
                    }
                    // `for (;;)` runs until a `break`.
                    None => None,
                };

                let depth = self.state_ref().depth;
                self.state().loops.push(LoopContext {
                    start: step_at,
                    continue_to: step_at,
                    breaks: Vec::new(),
                    depth,
                });

                self.begin_scope();
                self.body_for_effect(&n.body_node)?;
                self.end_scope();
                // `mark` is below the init's `let`, which the loop still needs.
                self.free_to(mark.max(self.locals_floor()));

                self.emit(Instr::Jump { to: step_at }, &n.position_start, &n.position_end);

                if let Some(exit) = exit {
                    self.patch(exit);
                }

                let context = self.state().loops.pop().expect("pushed above");
                for jump in context.breaks {
                    self.patch(jump);
                }

                self.end_scope();

                let dst = self.alloc()?;
                self.emit(Instr::LoadNull { dst }, &n.position_start, &n.position_end);
                Ok(dst)
            }

            Node::FuncDef(n) => self.func_def(n),

            Node::List(_) => Err(Unsupported::new("a list literal")),
            Node::Map(_) => Err(Unsupported::new("a map literal")),
            Node::TupleLiteral(_) => Err(Unsupported::new("a tuple literal")),
            Node::InterpolatedString(_) => Err(Unsupported::new("string interpolation")),
            Node::Call(n) => {
                let Node::VarAccess(callee) = &*n.node_to_call else {
                    return Err(Unsupported::new("a call to an expression"));
                };
                let name = callee.variable_name_token.value.as_deref().unwrap_or("");

                // `echo` keeps its own opcode rather than being folded into
                // `CALL`: it is a builtin, not a Xenith method, and the
                // builtin registry does not reach the VM until phase 7.
                if name == "echo" && self.resolve(name).is_none() {
                    if n.argument_nodes.len() != 1 {
                        // `echo()` with no argument prints a blank line in
                        // the tree walker. Rather than reimplement that here,
                        // it is left to the tree walker.
                        return Err(Unsupported::new("echo with other than one argument"));
                    }

                    let mark = self.state_ref().reg_top;
                    let src = self.operand(&n.argument_nodes[0])?;
                    self.emit(Instr::Echo { src }, &n.position_start, &n.position_end);
                    self.free_to(mark);

                    let dst = self.alloc()?;
                    self.emit(Instr::LoadNull { dst }, &n.position_start, &n.position_end);
                    return Ok(dst);
                }

                if n.argument_nodes.len() > u8::MAX as usize {
                    return Err(Unsupported::new("a call with more than 255 arguments"));
                }

                // The callee first, at the bottom of the window. `expr` on a
                // `VarAccess` refuses anything that is not a local or a
                // capture, which is what keeps a builtin or a global out of
                // here without a second check.
                //
                // `base` is a `u16` because `reg_top` is; `expr` returns a
                // `Reg`. `alloc` refuses anything above 256, so the casts
                // below cannot truncate.
                let base = self.state_ref().reg_top;
                let got = self.expr(&n.node_to_call)?;
                if got as u16 != base {
                    self.emit(
                        Instr::Move { dst: base as Reg, src: got },
                        n.node_to_call.position_start(),
                        n.node_to_call.position_end(),
                    );
                }
                self.free_to(base + 1);

                // Then the arguments, each compiled with `reg_top` already at
                // the register it has to end up in. `expr` allocates its
                // destination from `reg_top` upward, so the register it picks
                // is usually the one that was wanted and the `Move` below
                // never runs. Only `operand`'s elision could return something
                // lower, and this is deliberately `expr` and not `operand`:
                // an argument is a value the callee owns, not a read.
                for argument in &n.argument_nodes {
                    let want = self.state_ref().reg_top;
                    let got = self.expr(argument)?;
                    if got as u16 != want {
                        self.emit(
                            Instr::Move { dst: want as Reg, src: got },
                            argument.position_start(),
                            argument.position_end(),
                        );
                    }
                    self.free_to(want + 1);
                }

                let argc = n.argument_nodes.len() as u8;

                // The result lands on the callee, so a call does not climb
                // the frame any more than the widest of its arguments did.
                self.free_to(base);
                let dst = self.alloc()?;
                self.emit(
                    Instr::Call {
                        dst,
                        callee: base as Reg,
                        argc,
                    },
                    &n.position_start,
                    &n.position_end,
                );
                Ok(dst)
            }
            Node::Match(_) => Err(Unsupported::new("a match")),
            Node::VarAccess(n) => {
                let name = n
                    .variable_name_token
                    .value
                    .as_deref()
                    .ok_or_else(|| Unsupported::new("a name with no text"))?;

                if let Some(src) = self.resolve(name).map(|local| local.reg) {
                    let dst = self.alloc()?;
                    self.emit(Instr::Move { dst, src }, &n.position_start, &n.position_end);
                    return Ok(dst);
                }

                let level = self.functions.len() - 1;
                if let Some(idx) = self.resolve_upvalue(level, name)? {
                    let dst = self.alloc()?;
                    self.emit(Instr::GetUpval { dst, idx }, &n.position_start, &n.position_end);
                    return Ok(dst);
                }

                // A builtin, a global, or undefined. Phase 4 has none of
                // those, so the tree walker takes it -- which also means it,
                // not the VM, reports an undefined name, with the message it
                // always used.
                Err(Unsupported::new("a name that is not a local or a capture"))
            }
            Node::BinaryOperator(n) => {
                use crate::tokens::TokenType;

                // `&&` and `||` are not these. They must not evaluate their
                // right side unless the left says to, so they are jumps, and
                // they wait for task 8.
                let is_and = n.operator_token.matches(TokenType::Keyword, Some("&&"));
                let is_or = n.operator_token.matches(TokenType::Keyword, Some("||"));

                if is_and || is_or {
                    // The result is the *truthiness* of whichever side decided
                    // it, as a bool -- which is what the tree walker returns:
                    // `Value::Bool(is_or)` on a short circuit, and
                    // `Value::Bool(right.is_true())` otherwise.
                    let mark = self.state().reg_top;
                    let result = self.alloc()?;

                    let left = self.operand(&n.left_node)?;
                    self.emit(Instr::Not { dst: result, src: left }, &n.position_start, &n.position_end);
                    self.emit(Instr::Not { dst: result, src: result }, &n.position_start, &n.position_end);

                    let decided = if is_and {
                        self.emit_jump(
                            Instr::JumpIfFalse { cond: result, to: 0 },
                            &n.position_start, &n.position_end,
                        )
                    } else {
                        self.emit_jump(
                            Instr::JumpIfTrue { cond: result, to: 0 },
                            &n.position_start, &n.position_end,
                        )
                    };

                    let right = self.operand(&n.right_node)?;
                    self.emit(Instr::Not { dst: result, src: right }, &n.position_start, &n.position_end);
                    self.emit(Instr::Not { dst: result, src: result }, &n.position_start, &n.position_end);

                    self.patch(decided);
                    // `result` sits at `mark`, and everything above it was
                    // scratch for the two sides.
                    self.free_to(mark + 1);
                    return Ok(result);
                }

                // `x = v` is an assignment wearing an operator's clothes; the
                // parser produces it as a binary `Eq`. Task 6 handles it.
                if n.operator_token.kind == TokenType::Eq {
                    return Err(Unsupported::new("an assignment"));
                }

                let mark = self.state().reg_top;
                let a = self.operand(&n.left_node)?;
                let b = self.operand(&n.right_node)?;

                // The result lands where the left operand was, so a chain of
                // operators does not climb the frame.
                self.free_to(mark);
                let dst = self.alloc()?;

                let instr = match n.operator_token.kind {
                    TokenType::Plus => Instr::Add { dst, a, b },
                    TokenType::Minus => Instr::Sub { dst, a, b },
                    TokenType::Mul => Instr::Mul { dst, a, b },
                    TokenType::Div => Instr::Div { dst, a, b },
                    TokenType::Mod => Instr::Rem { dst, a, b },
                    TokenType::Pow => Instr::Pow { dst, a, b },
                    TokenType::Ee => Instr::Eq { dst, a, b },
                    TokenType::Ne => Instr::Ne { dst, a, b },
                    TokenType::Lt => Instr::Lt { dst, a, b },
                    TokenType::Gt => Instr::Gt { dst, a, b },
                    TokenType::Lte => Instr::Le { dst, a, b },
                    TokenType::Gte => Instr::Ge { dst, a, b },
                    // `.` and `[` are field access and indexing: phase 6.
                    _ => return Err(Unsupported::new("this operator")),
                };

                self.emit(instr, &n.position_start, &n.position_end);
                Ok(dst)
            }

            Node::UnaryOp(n) => {
                use crate::tokens::TokenType;

                let mark = self.state().reg_top;
                let src = self.operand(&n.node)?;
                self.free_to(mark);
                let dst = self.alloc()?;

                let instr = if n.operator_token.kind == TokenType::Minus {
                    Instr::Neg { dst, src }
                } else if n.operator_token.matches(TokenType::Keyword, Some("!")) {
                    Instr::Not { dst, src }
                } else {
                    return Err(Unsupported::new("this unary operator"));
                };

                self.emit(instr, &n.position_start, &n.position_end);
                Ok(dst)
            }

            other => Err(Unsupported::new(&format!(
                "{} is not compiled yet",
                node_label(other)
            ))),
        }
    }

    /// A method, declared or written as an expression.
    ///
    /// Both make a closure; a named one also binds it. The binding is made
    /// *before* the body is compiled, which is what lets a method find
    /// itself -- `factorial` in `tests/cases/methods.xen` is the fixture that
    /// needs it, and the capture it resolves to is an open upvalue pointing
    /// at the register `CLOSURE` is about to fill.
    fn func_def(&mut self, n: &crate::nodes::FuncDefNode) -> Result<Reg, Unsupported> {
        let name = n
            .variable_name_token
            .as_ref()
            .and_then(|token| token.value.clone());

        if n.param_names.len() > u8::MAX as usize {
            return Err(Unsupported::new("a method with more than 255 parameters"));
        }
        let arity = n.param_names.len() as u8;

        let dst = self.alloc()?;
        if let Some(name) = &name {
            let depth = self.state_ref().depth;
            self.state().locals.push(Local {
                name: name.clone(),
                reg: dst,
                depth,
                is_constant: false,
                captured: false,
            });
        }

        self.functions
            .push(FnState::for_function(name, &n.param_types, arity));

        // An `Unsupported` from here on abandons the whole compile -- the
        // caller runs the tree walker -- so the pushed state being left
        // behind costs nothing, and `?` stays readable.
        for token in &n.param_names {
            let param = token
                .value
                .clone()
                .ok_or_else(|| Unsupported::new("a parameter with no name"))?;
            let reg = self.alloc()?;
            self.state().locals.push(Local {
                name: param,
                reg,
                depth: 0,
                is_constant: false,
                captured: false,
            });
        }

        if n.is_arrow {
            // `=>` returns its body. `Function::execute` calls this
            // `should_auto_return`.
            let value = self.expr(&n.body_node)?;
            self.emit(
                Instr::Ret { src: value },
                n.body_node.position_start(),
                n.body_node.position_end(),
            );
        } else {
            self.body_for_effect(&n.body_node)?;

            // A body already ending in `RET` needs no second one. Not an
            // optimisation -- the instructions would be unreachable -- but a
            // disassembly with two dead instructions at the end of every
            // method is harder to read than one without.
            let ends_in_ret = matches!(
                self.state_ref().chunk.code.last(),
                Some(Instr::Ret { .. })
            );
            if !ends_in_ret {
                let reg = self.alloc()?;
                self.emit(Instr::LoadNull { dst: reg }, &n.position_end, &n.position_end);
                self.emit(Instr::Ret { src: reg }, &n.position_end, &n.position_end);
            }
        }

        let proto = self.finish_function();

        let index = {
            let state = self.state();
            state.chunk.protos.push(Rc::new(proto));
            state.chunk.protos.len() - 1
        };
        if index > u16::MAX as usize {
            return Err(Unsupported::new("more than 65,536 methods in one method"));
        }

        self.emit(
            Instr::Closure {
                dst,
                proto: index as u16,
            },
            &n.position_start,
            &n.position_end,
        );
        Ok(dst)
    }

    /// `let x: int = 1`, and `x = 2`.
    ///
    /// A declaration takes the next register permanently. A reassignment
    /// writes the register the declaration took.
    fn var_assign(&mut self, n: &crate::nodes::VarAssignNode) -> Result<Reg, Unsupported> {
        let name = n
            .variable_name_token
            .value
            .clone()
            .ok_or_else(|| Unsupported::new("a binding with no name"))?;

        let mark = self.state().reg_top;
        let value = self.operand(&n.value_node)?;

        if n.is_declaration {
            // A redeclaration in the same scope shadows, which is what the
            // symbol table does today. A new register, not a reuse.
            self.free_to(mark);
            let reg = self.alloc()?;
            if reg != value {
                self.emit(Instr::Move { dst: reg, src: value }, &n.position_start, &n.position_end);
            }
            let depth = self.state_ref().depth;
            self.state().locals.push(Local {
                name,
                reg,
                depth,
                is_constant: n.is_constant,
                captured: false,
            });
            return Ok(reg);
        }

        if let Some(binding) = self.find_any(&name) {
            if binding.is_constant {
                // XEN010 territory. The checker reports it and so does the
                // tree walker; the VM must not be the one to decide, or the
                // message would have to be duplicated.
                return Err(Unsupported::new("an assignment to a constant"));
            }
        }

        // The register is copied out of the borrow before `emit` takes
        // `&mut self`.
        if let Some(dst) = self.resolve(&name).map(|local| local.reg) {
            self.emit(Instr::Move { dst, src: value }, &n.position_start, &n.position_end);
            self.free_to(mark);
            return Ok(dst);
        }

        let level = self.functions.len() - 1;
        let Some(idx) = self.resolve_upvalue(level, &name)? else {
            return Err(Unsupported::new("an assignment to an unknown name"));
        };

        self.emit(Instr::SetUpval { idx, src: value }, &n.position_start, &n.position_end);
        self.free_to(mark);

        // The value of an assignment is the value assigned, and it is still
        // in the register it was computed in -- but `free_to` just handed
        // that register back. Re-taking it is the same register, which is why
        // the local case can do the same thing.
        let dst = self.alloc()?;
        if dst != value {
            self.emit(Instr::Move { dst, src: value }, &n.position_start, &n.position_end);
        }
        Ok(dst)
    }
}

/// A human name for a node, for the `Unsupported` message. Only reached for
/// nodes with no arm of their own, so it stays a coarse fallback.
fn node_label(node: &Node) -> &'static str {
    match node {
        Node::Ternary(_) => "a ternary",
        Node::If(_) => "a when",
        Node::While(_) => "a while loop",
        Node::For(_) => "a for-in loop",
        Node::ForClassic(_) => "a classic for loop",
        Node::VarAssign(_) => "an assignment",
        Node::Destructure(_) => "a destructuring",
        Node::Panic(_) => "a panic",
        Node::MethodAccess(_) => "a field access",
        Node::EnumVariant(_) => "an enum variant",
        Node::StructInstantiation(_) => "a struct literal",
        Node::Break(_) => "a break",
        Node::Continue(_) => "a continue",
        _ => "this construct",
    }
}
