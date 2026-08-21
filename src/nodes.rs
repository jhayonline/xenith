//! # Abstract Syntax Tree Nodes Module
//!
//! Defines all AST node types for Xenith language constructs
//! (expressions, statements, control flow, function definitions, etc.).
//! Each node stores relevant tokens and maintains position information
//! for error reporting and code generation.

use crate::position::Position;
use crate::tokens::Token;
use crate::types::Type;

/// All possible AST node types
#[derive(Debug, Clone)]
pub enum Node {
    Number(NumberNode),
    String(StringNode),
    List(ListNode),
    Ternary(Box<TernaryNode>),
    VarAccess(VarAccessNode),
    VarAssign(Box<VarAssignNode>),
    BinaryOperator(Box<BinaryOperatorNode>),
    UnaryOp(Box<UnaryOpNode>),
    If(Box<IfNode>),
    For(Box<ForNode>),
    ForClassic(Box<ForClassicNode>),
    While(Box<WhileNode>),
    FuncDef(Box<FuncDefNode>),
    Call(Box<CallNode>),
    Return(Box<ReturnNode>),
    Continue(ContinueNode),
    Break(BreakNode),
    InterpolatedString(InterpolatedStringNode),
    MethodAccess(MethodAccessNode),
    Map(MapNode),
    Panic(Box<PanicNode>),
    Grab(Box<GrabNode>),
    Export(Box<ExportNode>),
    StructDef(Box<StructDefNode>),
    EnumDef(Box<EnumDefNode>),
    /// `Shape::Circle(1.0)`, or `Shape::Empty` for a variant with no payload.
    EnumVariant(Box<EnumVariantNode>),
    Match(Box<MatchNode>),
    TypeAlias(Box<TypeAliasNode>),
    BoolLiteral(BoolLiteralNode),
    NullLiteral(NullLiteralNode),
    StructInstantiation(Box<StructInstantiationNode>),
    TupleLiteral(TupleLiteralNode),
    Destructure(DestructureNode),
    DestructurePattern(DestructurePatternNode),
}

impl Node {
    /// This node's id, or [`NodeId::UNSET`] where it carries no type.
    pub fn id(&self) -> NodeId {
        match self {
            Node::Number(n) => n.id,
            Node::String(n) => n.id,
            Node::List(n) => n.id,
            Node::Ternary(n) => n.id,
            Node::VarAccess(n) => n.id,
            Node::BinaryOperator(n) => n.id,
            Node::UnaryOp(n) => n.id,
            Node::Call(n) => n.id,
            Node::InterpolatedString(n) => n.id,
            Node::MethodAccess(n) => n.id,
            Node::Map(n) => n.id,
            Node::EnumVariant(n) => n.id,
            Node::Match(n) => n.id,
            Node::BoolLiteral(n) => n.id,
            Node::NullLiteral(n) => n.id,
            Node::StructInstantiation(n) => n.id,
            Node::TupleLiteral(n) => n.id,
            _ => NodeId::UNSET,
        }
    }

    /// Every child node, in evaluation order.
    ///
    /// Used by tests and tooling; the interpreter matches on variants directly
    /// rather than going through this.
    ///
    /// Matched exhaustively on purpose. A new `Node` variant will not compile
    /// until it says whether it has children, because a missing arm here is a
    /// subtree the compiler silently will not walk.
    pub fn children(&self) -> Vec<&Node> {
        match self {
            // Expressions
            Node::BinaryOperator(n) => vec![n.left_node.as_ref(), n.right_node.as_ref()],
            Node::UnaryOp(n) => vec![n.node.as_ref()],
            Node::Ternary(n) => vec![
                n.condition.as_ref(),
                n.true_expression.as_ref(),
                n.false_expression.as_ref(),
            ],
            // Also a statement block: `statements()` builds one of these.
            Node::List(n) => n.element_nodes.iter().map(|e| e.as_ref()).collect(),
            Node::TupleLiteral(n) => n.elements.iter().map(|e| e.as_ref()).collect(),
            Node::Map(n) => n
                .pairs
                .iter()
                .flat_map(|pair| [pair.key_node.as_ref(), pair.value_node.as_ref()])
                .collect(),
            Node::MethodAccess(n) => vec![n.object.as_ref()],
            Node::Call(n) => {
                let mut kids = vec![n.node_to_call.as_ref()];
                kids.extend(n.argument_nodes.iter().map(|a| a.as_ref()));
                kids
            }
            Node::EnumVariant(n) => n.arguments.iter().map(|a| a.as_ref()).collect(),
            Node::StructInstantiation(n) => n.fields.iter().map(|(_, value)| value).collect(),
            Node::InterpolatedString(n) => {
                n.parts.iter().filter_map(|p| p.parsed.as_deref()).collect()
            }
            Node::Match(n) => {
                let mut kids = vec![n.subject.as_ref()];
                for arm in &n.arms {
                    if let Some(guard) = &arm.guard {
                        kids.push(guard.as_ref());
                    }
                    kids.push(arm.body.as_ref());
                }
                kids
            }

            // Statements
            Node::VarAssign(n) => vec![n.value_node.as_ref()],
            Node::Destructure(n) => vec![n.value_node.as_ref()],
            Node::Return(n) => n.node_to_return.iter().map(|v| v.as_ref()).collect(),
            Node::Panic(n) => vec![n.message_node.as_ref()],
            Node::FuncDef(n) => vec![n.body_node.as_ref()],
            Node::Export(n) => vec![n.node.as_ref()],
            Node::If(n) => {
                let mut kids = Vec::new();
                for (condition, body) in &n.cases {
                    kids.push(condition.as_ref());
                    kids.push(body.as_ref());
                }
                if let Some((condition, body)) = &n.else_case {
                    kids.push(condition.as_ref());
                    kids.push(body.as_ref());
                }
                kids
            }
            Node::For(n) => vec![n.iterable_node.as_ref(), n.body_node.as_ref()],
            Node::ForClassic(n) => {
                let mut kids = Vec::new();
                for part in [&n.init_node, &n.condition_node, &n.step_node] {
                    if let Some(node) = part {
                        kids.push(node.as_ref());
                    }
                }
                kids.push(n.body_node.as_ref());
                kids
            }
            Node::While(n) => vec![n.condition_node.as_ref(), n.body_node.as_ref()],

            // Nothing below here holds a child expression.
            Node::Number(_)
            | Node::String(_)
            | Node::VarAccess(_)
            | Node::BoolLiteral(_)
            | Node::NullLiteral(_)
            | Node::Continue(_)
            | Node::Break(_)
            | Node::Grab(_)
            | Node::StructDef(_)
            | Node::EnumDef(_)
            | Node::TypeAlias(_)
            | Node::DestructurePattern(_) => Vec::new(),
        }
    }

    pub fn position_start(&self) -> &Position {
        match self {
            Node::Number(n) => &n.position_start,
            Node::String(n) => &n.position_start,
            Node::List(n) => &n.position_start,
            Node::Ternary(n) => &n.position_start,
            Node::VarAccess(n) => &n.position_start,
            Node::VarAssign(n) => &n.position_start,
            Node::BinaryOperator(n) => &n.position_start,
            Node::UnaryOp(n) => &n.position_start,
            Node::If(n) => &n.position_start,
            Node::For(n) => &n.position_start,
            Node::ForClassic(n) => &n.position_start,
            Node::While(n) => &n.position_start,
            Node::FuncDef(n) => &n.position_start,
            Node::Call(n) => &n.position_start,
            Node::Return(n) => &n.position_start,
            Node::Continue(n) => &n.position_start,
            Node::Break(n) => &n.position_start,
            Node::InterpolatedString(n) => &n.position_start,
            Node::MethodAccess(n) => &n.position_start,
            Node::Map(n) => &n.position_start,
            Node::Panic(n) => &n.position_start,
            Node::Grab(n) => &n.position_start,
            Node::Export(n) => &n.position_start,
            Node::StructDef(n) => &n.position_start,
            Node::EnumDef(n) => &n.position_start,
            Node::EnumVariant(n) => &n.position_start,
            Node::Match(n) => &n.position_start,
            Node::TypeAlias(n) => &n.position_start,
            Node::BoolLiteral(n) => &n.position_start,
            Node::NullLiteral(n) => &n.position_start,
            Node::StructInstantiation(n) => &n.position_start,
            Node::TupleLiteral(n) => &n.position_start,
            Node::Destructure(n) => &n.position_start,
            Node::DestructurePattern(n) => &n.position_start,
        }
    }

    pub fn position_end(&self) -> &Position {
        match self {
            Node::Number(n) => &n.position_end,
            Node::String(n) => &n.position_end,
            Node::List(n) => &n.position_end,
            Node::Ternary(n) => &n.position_end,
            Node::VarAccess(n) => &n.position_end,
            Node::VarAssign(n) => &n.position_end,
            Node::BinaryOperator(n) => &n.position_end,
            Node::UnaryOp(n) => &n.position_end,
            Node::If(n) => &n.position_end,
            Node::For(n) => &n.position_end,
            Node::ForClassic(n) => &n.position_end,
            Node::While(n) => &n.position_end,
            Node::FuncDef(n) => &n.position_end,
            Node::Call(n) => &n.position_end,
            Node::Return(n) => &n.position_end,
            Node::Continue(n) => &n.position_end,
            Node::Break(n) => &n.position_end,
            Node::InterpolatedString(n) => &n.position_end,
            Node::MethodAccess(n) => &n.position_end,
            Node::Map(n) => &n.position_end,
            Node::Panic(n) => &n.position_end,
            Node::Grab(n) => &n.position_end,
            Node::Export(n) => &n.position_end,
            Node::StructDef(n) => &n.position_end,
            Node::EnumDef(n) => &n.position_end,
            Node::EnumVariant(n) => &n.position_end,
            Node::Match(n) => &n.position_end,
            Node::TypeAlias(n) => &n.position_end,
            Node::BoolLiteral(n) => &n.position_end,
            Node::NullLiteral(n) => &n.position_end,
            Node::StructInstantiation(n) => &n.position_end,
            Node::TupleLiteral(n) => &n.position_end,
            Node::Destructure(n) => &n.position_end,
            Node::DestructurePattern(n) => &n.position_end,
        }
    }

    /// Creates a binary operation node
    pub fn bin_op(id: NodeId, left: Node, op_token: Token, right: Node) -> Self {
        Node::BinaryOperator(Box::new(BinaryOperatorNode {
            id,
            position_start: left.position_start().clone(),
            position_end: right.position_end().clone(),
            left_node: Box::new(left),
            operator_token: op_token,
            right_node: Box::new(right),
        }))
    }
}

/// Number literal node
#[derive(Debug, Clone)]
pub struct NumberNode {
    pub id: NodeId,
    pub token: Token,
    pub position_start: Position,
    pub position_end: Position,
}

impl NumberNode {
    pub fn new(id: NodeId, token: Token) -> Self {
        Self {
            id,
            position_start: token.position_start.clone(),
            position_end: token.position_end.clone(),
            token,
        }
    }
}

/// String literal node
#[derive(Debug, Clone)]
pub struct StringNode {
    pub id: NodeId,
    pub token: Token,
    pub position_start: Position,
    pub position_end: Position,
}

impl StringNode {
    pub fn new(id: NodeId, token: Token) -> Self {
        Self {
            id,
            position_start: token.position_start.clone(),
            position_end: token.position_end.clone(),
            token,
        }
    }
}

/// List literal node
#[derive(Debug, Clone)]
pub struct ListNode {
    pub id: NodeId,
    pub element_nodes: Vec<Box<Node>>,
    pub position_start: Position,
    pub position_end: Position,
}

#[derive(Debug, Clone)]
pub struct MethodAccessNode {
    pub id: NodeId,
    pub object: Box<Node>,
    pub method_name: Token,
    pub position_start: Position,
    pub position_end: Position,
}

/// Ternary expression node (condition ? true : false)
#[derive(Debug, Clone)]
pub struct TernaryNode {
    pub id: NodeId,
    pub condition: Box<Node>,
    pub true_expression: Box<Node>,
    pub false_expression: Box<Node>,
    pub position_start: Position,
    pub position_end: Position,
}
/// Identifies a node within one parse, so a later pass can record what it
/// worked out about the node without changing the tree.
///
/// Only nodes that can have a type are numbered. A statement carries
/// [`NodeId::UNSET`], and nothing may be recorded against it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(pub u32);

impl NodeId {
    pub const UNSET: Self = NodeId(u32::MAX);
}


/// Where a name was last found, remembered on the node that reads it.
///
/// Looking a variable up means walking out through the scope chain hashing the
/// name in each scope until one has it. The answer is almost always the same
/// every time that particular line runs, so each reference remembers it: how
/// many scopes out, and which slot.
///
/// A remembered position is never trusted blindly. The name at that slot is
/// compared before the value is used, so a hit in a differently shaped scope
/// simply misses and is looked up properly. That matters because Xenith
/// resolves names against the caller's scope, and the same method body can run
/// against different chains. Getting it wrong costs a lookup, not a wrong
/// answer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SlotCache {
    pub hops: u16,
    pub slot: u32,
    pub valid: bool,
}

impl SlotCache {
    pub const EMPTY: Self = Self {
        hops: 0,
        slot: 0,
        valid: false,
    };

    pub fn get(&self) -> Option<(u16, u32)> {
        self.valid.then_some((self.hops, self.slot))
    }

    pub fn set(hops: u16, slot: u32) -> Self {
        Self {
            hops,
            slot,
            valid: true,
        }
    }
}

/// Variable access node
#[derive(Debug, Clone)]
pub struct VarAccessNode {
    pub id: NodeId,
    pub variable_name_token: Token,
    /// Where this name was found last time. See [`SlotCache`].
    pub cache: std::cell::Cell<SlotCache>,
    pub position_start: Position,
    pub position_end: Position,
}

/// Variable assignment node
#[derive(Debug, Clone)]
pub struct VarAssignNode {
    pub variable_name_token: Token,
    /// Where this name was found last time, for reassignment. See
    /// [`SlotCache`]. Unused for a declaration, which always writes to the
    /// current scope.
    pub cache: std::cell::Cell<SlotCache>,
    pub var_type: Option<Type>,
    pub value_node: Box<Node>,
    pub is_constant: bool,
    /// `let x = ...` declares in the current scope; a bare `x = ...` updates
    /// the binding wherever it was declared. Without this flag the two are
    /// indistinguishable and assignment silently shadows.
    pub is_declaration: bool,
    pub position_start: Position,
    pub position_end: Position,
}

/// Binary operator node
#[derive(Debug, Clone)]
pub struct BinaryOperatorNode {
    pub id: NodeId,
    pub left_node: Box<Node>,
    pub operator_token: Token,
    pub right_node: Box<Node>,
    pub position_start: Position,
    pub position_end: Position,
}

/// Unary operator node
#[derive(Debug, Clone)]
pub struct UnaryOpNode {
    pub id: NodeId,
    pub operator_token: Token,
    pub node: Box<Node>,
    pub position_start: Position,
    pub position_end: Position,
}

/// If/elif/else conditional node
#[derive(Debug, Clone)]
pub struct IfNode {
    pub cases: Vec<(Box<Node>, Box<Node>)>,
    pub else_case: Option<(Box<Node>, Box<Node>)>,
    pub position_start: Position,
    pub position_end: Position,
}

/// For loop node
#[derive(Debug, Clone)]
/// Range loop: `for item in collection { ... }`
///
/// A two-variable pattern (`for k, v in map`) is encoded in
/// `variable_name_token` as the literal text `(k,v)`.
pub struct ForNode {
    pub variable_name_token: Token,
    pub iterable_node: Box<Node>,
    pub body_node: Box<Node>,
    pub should_return_null: bool,
    pub position_start: Position,
    pub position_end: Position,
}

/// C-style counting loop: `for (init; condition; step) { ... }`
#[derive(Debug, Clone)]
pub struct ForClassicNode {
    pub init_node: Option<Box<Node>>,
    pub condition_node: Option<Box<Node>>,
    pub step_node: Option<Box<Node>>,
    pub body_node: Box<Node>,
    pub position_start: Position,
    pub position_end: Position,
}

/// While loop node
#[derive(Debug, Clone)]
pub struct WhileNode {
    pub condition_node: Box<Node>,
    pub body_node: Box<Node>,
    pub should_return_null: bool,
    pub position_start: Position,
    pub position_end: Position,
}

/// Function definition node
#[derive(Debug, Clone)]
pub struct FuncDefNode {
    pub variable_name_token: Option<Token>,
    pub param_names: Vec<Token>,
    pub param_types: Vec<Type>,
    pub return_type: Type,
    pub body_node: Box<Node>,
    pub is_arrow: bool, // true for => syntax
    pub position_start: Position,
    pub position_end: Position,
}

/// Function call node
#[derive(Debug, Clone)]
pub struct CallNode {
    pub id: NodeId,
    pub node_to_call: Box<Node>,
    pub argument_nodes: Vec<Box<Node>>,
    pub position_start: Position,
    pub position_end: Position,
}

/// Return statement node
#[derive(Debug, Clone)]
pub struct ReturnNode {
    pub node_to_return: Option<Box<Node>>,
    pub position_start: Position,
    pub position_end: Position,
}

/// Continue statement node
#[derive(Debug, Clone)]
pub struct ContinueNode {
    pub position_start: Position,
    pub position_end: Position,
}

/// Break statement node
#[derive(Debug, Clone)]
pub struct BreakNode {
    pub position_start: Position,
    pub position_end: Position,
}

#[derive(Debug, Clone)]
pub struct InterpolatedStringNode {
    pub id: NodeId,
    pub parts: Vec<InterpolationPart>,
    pub position_start: Position,
    pub position_end: Position,
}

#[derive(Debug, Clone)]
pub struct InterpolationPart {
    pub is_expression: bool,
    pub content: String, // If text: the literal text, if expression: the expression source
    /// An expression part parsed at parse time.
    ///
    /// Interpolated expressions used to be kept only as source text and lexed
    /// and parsed afresh on every evaluation, which cost a parse per loop
    /// iteration and hid them from the static checker. `None` means the text
    /// did not parse; the interpreter falls back to its old path so the error
    /// surfaces the way it always did.
    pub parsed: Option<Box<Node>>,
}

/// Hides `|` and `\` from the part delimiter used by the interpolation
/// encoding, so an expression containing `||` survives the round trip.
pub fn escape_interpolation_part(content: &str) -> String {
    content.replace('\\', "\\\\").replace('|', "\\p")
}

/// Lexes and parses the text inside one `{}` of an interpolated string.
///
/// Returns `None` when it does not parse, leaving the interpreter to produce
/// the error at evaluation time exactly as it did before.
/// `next_node_id` is the enclosing parse's counter, threaded in and out so the
/// nodes inside `"{x + 1}"` are numbered in the same space as everything around
/// them. A fresh parser would start again at zero and collide.
fn parse_interpolated_expression(
    source: &str,
    origin: &Position,
    next_node_id: &mut u32,
) -> Option<Box<Node>> {
    let mut lexer = crate::lexer::Lexer::new_at(source.to_string(), origin);
    let tokens = lexer.make_tokens().ok()?;

    let mut parser = crate::parser::Parser::new_numbering_from(tokens, *next_node_id);
    let result = parser.parse_expression();

    if result.error.is_some() {
        // The ids it handed out are still spent: the sub-tree is discarded, but
        // reusing them would mean two live nodes with one id if a later parse
        // succeeded.
        *next_node_id = parser.node_count();
        return None;
    }
    *next_node_id = parser.node_count();
    result.node.map(Box::new)
}

/// Inverse of [`escape_interpolation_part`].
pub fn unescape_interpolation_part(content: &str) -> String {
    let mut out = String::with_capacity(content.len());
    let mut chars = content.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('p') => out.push('|'),
            Some('\\') => out.push('\\'),
            // Not one of ours: keep the backslash and whatever followed.
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
            None => out.push('\\'),
        }
    }
    out
}

impl InterpolatedStringNode {
    pub fn new(id: NodeId, next_node_id: &mut u32, token: Token) -> Self {
        // Parse the encoded string
        let mut parts = Vec::new();
        let origin = token.position_start.clone();
        if let Some(encoded) = token.value {
            let content = encoded.trim_start_matches("__INTERPOLATED__");
            for part in content.split('|').skip(1) {
                let mut split = part.splitn(2, ':');
                if let (Some(part_type), Some(content)) = (split.next(), split.next()) {
                    let is_expression = part_type == "expr";
                    let content = unescape_interpolation_part(content);
                    let parsed = if is_expression {
                        parse_interpolated_expression(&content, &origin, next_node_id)
                    } else {
                        None
                    };
                    parts.push(InterpolationPart {
                        is_expression,
                        content,
                        parsed,
                    });
                }
            }
        }

        Self {
            id,
            parts,
            position_start: token.position_start.clone(),
            position_end: token.position_end.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct MapNode {
    pub id: NodeId,
    pub pairs: Vec<MapPair>,
    pub position_start: Position,
    pub position_end: Position,
}

#[derive(Debug, Clone)]
pub struct MapPair {
    pub key_node: Box<Node>,
    pub value_node: Box<Node>,
    pub position_start: Position,
    pub position_end: Position,
}

// Panic node
#[derive(Debug, Clone)]
pub struct PanicNode {
    pub message_node: Box<Node>,
    pub position_start: Position,
    pub position_end: Position,
}

// Grab/import node
#[derive(Debug, Clone)]
pub struct GrabNode {
    pub imports: Vec<ImportSpec>,
    pub from_module: String, // Module path like "std::math" or "math"
    pub is_namespace_import: bool,
    pub namespace_alias: Option<String>, // For "grab * as name"
    pub position_start: Position,
    pub position_end: Position,
}

#[derive(Debug, Clone)]
pub struct ImportSpec {
    pub original_name: String,
    pub alias: Option<String>,
    pub position_start: Position,
    pub position_end: Position,
}

// Export annotation for items
#[derive(Debug, Clone)]
pub struct ExportNode {
    pub exported_name: String,
    pub node: Box<Node>,
    pub position_start: Position,
    pub position_end: Position,
}

#[derive(Debug, Clone)]
pub struct StructDefNode {
    pub name: Token,
    pub fields: Vec<StructFieldNode>,
    pub position_start: Position,
    pub position_end: Position,
}

#[derive(Debug, Clone)]
pub struct StructFieldNode {
    pub name: Token,
    pub field_type: Type,
    pub is_constant: bool,
    pub position_start: Position,
    pub position_end: Position,
}

/// `enum Shape { Circle(float), Rect(float, float), Empty }`
#[derive(Debug, Clone)]
pub struct EnumDefNode {
    pub name: Token,
    pub variants: Vec<EnumVariantDef>,
    pub position_start: Position,
    pub position_end: Position,
}

/// One variant of an enum, with the types it carries. Payloads are positional:
/// `Circle(float)` rather than `Circle { radius: float }`.
#[derive(Debug, Clone)]
pub struct EnumVariantDef {
    pub name: Token,
    pub payload_types: Vec<Type>,
    pub position_start: Position,
    pub position_end: Position,
}

/// Building a value: `Shape::Circle(1.0)`.
///
/// The arguments are held here rather than the node being wrapped in a `Call`,
/// because a variant is not a callable value -- there is nothing to look up in
/// a scope, and `Shape::Circle` on its own with a payload is an error rather
/// than a partially applied constructor.
#[derive(Debug, Clone)]
pub struct EnumVariantNode {
    pub id: NodeId,
    pub enum_name: String,
    pub variant_name: String,
    pub arguments: Vec<Box<Node>>,
    pub position_start: Position,
    pub position_end: Position,
}

/// `match subject { pattern => body ... }`
///
/// An expression, so every arm produces a value. That is what lets a `match`
/// stand on the right of a `let`, which is most of the reason to have one.
#[derive(Debug, Clone)]
pub struct MatchNode {
    pub id: NodeId,
    pub subject: Box<Node>,
    pub arms: Vec<MatchArm>,
    pub position_start: Position,
    pub position_end: Position,
}

#[derive(Debug, Clone)]
pub struct MatchArm {
    /// More than one when the arm was written `A | B`. They bind nothing, so
    /// there is no question of the alternatives disagreeing about names.
    pub patterns: Vec<Pattern>,
    /// `when` guard. An arm with one never counts towards exhaustiveness,
    /// because whether it matches cannot be decided by looking at it.
    pub guard: Option<Box<Node>>,
    pub body: Box<Node>,
    pub position_start: Position,
    pub position_end: Position,
}

#[derive(Debug, Clone)]
pub struct Pattern {
    pub kind: PatternKind,
    pub position_start: Position,
    pub position_end: Position,
}

#[derive(Debug, Clone)]
pub enum PatternKind {
    /// `_`
    Wildcard,
    /// A bare name, which matches anything and binds it.
    Binding(Token),
    /// A literal to compare against: `0`, `"GET"`, `true`, `null`.
    Literal(Box<Node>),
    /// `Shape::Circle(r)`, or `Shape::Empty` with no sub-patterns.
    Variant {
        enum_name: String,
        variant_name: String,
        sub_patterns: Vec<Pattern>,
        /// Distinguishes `Empty` from `Empty()`, so the arity check can tell
        /// "no payload" from "a payload of nothing".
        has_parens: bool,
    },
    /// `(a, b)`
    Tuple(Vec<Pattern>),
}

impl Pattern {
    /// Does this pattern match every possible value of its type?
    ///
    /// Only irrefutable sub-patterns let a variant pattern count towards
    /// exhaustiveness: `Circle(r)` covers every circle, but `Circle(0.0)` does
    /// not, and treating the second as covering `Circle` would let a match with
    /// a real hole in it through the checker.
    pub fn is_irrefutable(&self) -> bool {
        match &self.kind {
            PatternKind::Wildcard | PatternKind::Binding(_) => true,
            PatternKind::Literal(_) => false,
            PatternKind::Variant { .. } => false,
            PatternKind::Tuple(elements) => elements.iter().all(|p| p.is_irrefutable()),
        }
    }

    /// Every name this pattern introduces, in the order they appear.
    pub fn bindings(&self, out: &mut Vec<Token>) {
        match &self.kind {
            PatternKind::Binding(token) => out.push(token.clone()),
            PatternKind::Variant { sub_patterns, .. } => {
                for sub in sub_patterns {
                    sub.bindings(out);
                }
            }
            PatternKind::Tuple(elements) => {
                for element in elements {
                    element.bindings(out);
                }
            }
            PatternKind::Wildcard | PatternKind::Literal(_) => {}
        }
    }
}

#[derive(Debug, Clone)]
pub struct TypeAliasNode {
    pub name: Token,
    pub alias_type: Type,
    pub position_start: Position,
    pub position_end: Position,
}

#[derive(Debug, Clone)]
pub struct BoolLiteralNode {
    pub id: NodeId,
    pub value: bool,
    pub position_start: Position,
    pub position_end: Position,
}

#[derive(Debug, Clone)]
pub struct NullLiteralNode {
    pub id: NodeId,
    pub position_start: Position,
    pub position_end: Position,
}

#[derive(Debug, Clone)]
pub struct StructInstantiationNode {
    pub id: NodeId,
    pub struct_name: String,
    pub fields: Vec<(Token, Node)>,
    pub position_start: Position,
    pub position_end: Position,
}

#[derive(Debug, Clone)]
pub struct TupleLiteralNode {
    pub id: NodeId,
    pub elements: Vec<Box<Node>>,
    pub position_start: Position,
    pub position_end: Position,
}

#[derive(Debug, Clone)]
pub struct DestructureNode {
    pub patterns: Vec<DestructurePattern>,
    pub value_node: Box<Node>,
    pub position_start: Position,
    pub position_end: Position,
}

#[derive(Debug, Clone)]
pub enum DestructurePattern {
    Variable(Token),                // a
    Ignore,                         // _
    Tuple(Vec<DestructurePattern>), // nested (a, b)
}

#[derive(Debug, Clone)]
pub struct DestructurePatternNode {
    pub pattern: DestructurePattern,
    pub position_start: Position,
    pub position_end: Position,
}

impl DestructurePattern {
    pub fn position_start(&self) -> Position {
        match self {
            DestructurePattern::Variable(token) => token.position_start.clone(),
            DestructurePattern::Ignore => Position::new(0, 0, 0, "", ""), // Or store position
            DestructurePattern::Tuple(patterns) => patterns
                .first()
                .map(|p| p.position_start())
                .unwrap_or_else(|| Position::new(0, 0, 0, "", "")),
        }
    }

    pub fn position_end(&self) -> Position {
        match self {
            DestructurePattern::Variable(token) => token.position_end.clone(),
            DestructurePattern::Ignore => Position::new(0, 0, 0, "", ""), // Or store position
            DestructurePattern::Tuple(patterns) => patterns
                .last()
                .map(|p| p.position_end())
                .unwrap_or_else(|| Position::new(0, 0, 0, "", "")),
        }
    }
}
