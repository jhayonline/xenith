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

use crate::nodes::Node;
use crate::position::Position;
use crate::values::Value;
use crate::vm::chunk::{Addr, Chunk, Instr, Reg};

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
        chunk: Chunk::new(),
        reg_top: 0,
        high_water: 0,
    };

    let value = compiler.program(ast)?;
    compiler.emit(Instr::Halt { src: value }, ast.position_start());

    // Read before the move: taking `compiler.chunk` partially moves
    // `compiler`, and `high_water` cannot be read afterwards.
    let registers = compiler.high_water as u16;
    let mut chunk = compiler.chunk;
    chunk.registers = registers;
    Ok(chunk)
}

struct Compiler {
    chunk: Chunk,
    /// Next free register. Locals sit below it permanently; temporaries above
    /// it are handed out and taken back within a statement.
    reg_top: u16,
    /// The largest `reg_top` ever reached, which is the frame size.
    high_water: u16,
}

impl Compiler {
    /// Takes the next register.
    fn alloc(&mut self) -> Result<Reg, Unsupported> {
        if self.reg_top >= 256 {
            return Err(Unsupported::new("more than 256 registers in one frame"));
        }
        let reg = self.reg_top as Reg;
        self.reg_top += 1;
        if self.reg_top > self.high_water {
            self.high_water = self.reg_top;
        }
        Ok(reg)
    }

    /// Gives back every register above `mark`.
    ///
    /// Called at the end of each statement, which is what keeps a long
    /// function from needing one register per subexpression it ever
    /// evaluates.
    fn free_to(&mut self, mark: u16) {
        self.reg_top = mark;
    }

    fn emit(&mut self, instr: Instr, position: &Position) -> Addr {
        let at = self.chunk.push(instr);
        self.chunk.record_position(at, position);
        at
    }

    /// The top level: a statement list, whose value is the last statement's.
    fn program(&mut self, ast: &Node) -> Result<Reg, Unsupported> {
        let Node::List(statements) = ast else {
            return Err(Unsupported::new("a top level that is not a statement list"));
        };

        let mut last: Option<Reg> = None;
        let count = statements.element_nodes.len();

        for (i, statement) in statements.element_nodes.iter().enumerate() {
            let mark = self.reg_top;
            let reg = self.stmt(statement)?;

            // Only the last statement's value survives -- it is the chunk's
            // value. Every earlier one is released with its temporaries.
            //
            // Keeping an intermediate value would be wrong as well as
            // wasteful: releasing to `mark` and re-allocating hands back the
            // same register, so the next statement would overwrite the very
            // value being held.
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
                        self.emit(Instr::Move { dst: kept, src: reg }, statement.position_start());
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
                let reg = self.alloc()?;
                self.emit(Instr::LoadNull { dst: reg }, ast.position_start());
                Ok(reg)
            }
        }
    }

    /// One statement. `Ok(None)` means it produced no value.
    ///
    /// Later tasks add arms here: locals (task 6), `when` (task 8), `while`
    /// (task 9), classic `for` (task 10), `echo` (task 7).
    fn stmt(&mut self, node: &Node) -> Result<Option<Reg>, Unsupported> {
        match node {
            Node::FuncDef(_) => Err(Unsupported::new("a method declaration")),
            Node::Grab(_) => Err(Unsupported::new("an import")),
            Node::Export(_) => Err(Unsupported::new("an export")),
            Node::StructDef(_) => Err(Unsupported::new("a struct declaration")),
            Node::EnumDef(_) => Err(Unsupported::new("an enum declaration")),
            Node::TypeAlias(_) => Err(Unsupported::new("a type alias")),
            Node::Return(_) => Err(Unsupported::new("release outside a method")),
            other => self.expr(other).map(Some),
        }
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
                let k = self.chunk.add_constant(value);
                self.emit(Instr::LoadConst { dst, k }, &n.position_start);
                Ok(dst)
            }

            Node::String(n) => {
                let text = n
                    .token
                    .value
                    .clone()
                    .ok_or_else(|| Unsupported::new("a string with no text"))?;
                let dst = self.alloc()?;
                let k = self.chunk.add_constant(Value::string(&text));
                self.emit(Instr::LoadConst { dst, k }, &n.position_start);
                Ok(dst)
            }

            Node::BoolLiteral(n) => {
                let dst = self.alloc()?;
                self.emit(
                    Instr::LoadBool {
                        dst,
                        value: n.value,
                    },
                    &n.position_start,
                );
                Ok(dst)
            }

            Node::NullLiteral(n) => {
                let dst = self.alloc()?;
                self.emit(Instr::LoadNull { dst }, &n.position_start);
                Ok(dst)
            }

            Node::List(_) => Err(Unsupported::new("a list literal")),
            Node::Map(_) => Err(Unsupported::new("a map literal")),
            Node::TupleLiteral(_) => Err(Unsupported::new("a tuple literal")),
            Node::InterpolatedString(_) => Err(Unsupported::new("string interpolation")),
            Node::Call(_) => Err(Unsupported::new("a call")),
            Node::Match(_) => Err(Unsupported::new("a match")),
            Node::VarAccess(_) => Err(Unsupported::new("a name")),
            Node::BinaryOperator(n) => {
                use crate::tokens::TokenType;

                // `&&` and `||` are not these. They must not evaluate their
                // right side unless the left says to, so they are jumps, and
                // they wait for task 8.
                if n.operator_token.matches(TokenType::Keyword, Some("&&"))
                    || n.operator_token.matches(TokenType::Keyword, Some("||"))
                {
                    return Err(Unsupported::new("a short-circuiting operator"));
                }

                // `x = v` is an assignment wearing an operator's clothes; the
                // parser produces it as a binary `Eq`. Task 6 handles it.
                if n.operator_token.kind == TokenType::Eq {
                    return Err(Unsupported::new("an assignment"));
                }

                let mark = self.reg_top;
                let a = self.expr(&n.left_node)?;
                let b = self.expr(&n.right_node)?;

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

                self.emit(instr, &n.position_start);
                Ok(dst)
            }

            Node::UnaryOp(n) => {
                use crate::tokens::TokenType;

                let mark = self.reg_top;
                let src = self.expr(&n.node)?;
                self.free_to(mark);
                let dst = self.alloc()?;

                let instr = if n.operator_token.kind == TokenType::Minus {
                    Instr::Neg { dst, src }
                } else if n.operator_token.matches(TokenType::Keyword, Some("!")) {
                    Instr::Not { dst, src }
                } else {
                    return Err(Unsupported::new("this unary operator"));
                };

                self.emit(instr, &n.position_start);
                Ok(dst)
            }

            other => Err(Unsupported::new(&format!(
                "{} is not compiled yet",
                node_label(other)
            ))),
        }
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
