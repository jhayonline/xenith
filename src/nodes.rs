//! # Abstract Syntax Tree Nodes Module
//!
//! Defines all AST node types for Xenith language constructs
//! (expressions, statements, control flow, function definitions, etc.).
//! Each node stores relevant tokens and maintains position information
//! for error reporting and code generation.

use crate::position::Position;
use crate::tokens::Token;

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
    While(Box<WhileNode>),
    FuncDef(Box<FuncDefNode>),
    Call(Box<CallNode>),
    Return(Box<ReturnNode>),
    Continue(ContinueNode),
    Break(BreakNode),
    InterpolatedString(InterpolatedStringNode),
    MethodAccess(MethodAccessNode),
}

impl Node {
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
            Node::While(n) => &n.position_start,
            Node::FuncDef(n) => &n.position_start,
            Node::Call(n) => &n.position_start,
            Node::Return(n) => &n.position_start,
            Node::Continue(n) => &n.position_start,
            Node::Break(n) => &n.position_start,
            Node::InterpolatedString(n) => &n.position_start,
            Node::MethodAccess(n) => &n.position_start,
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
            Node::While(n) => &n.position_end,
            Node::FuncDef(n) => &n.position_end,
            Node::Call(n) => &n.position_end,
            Node::Return(n) => &n.position_end,
            Node::Continue(n) => &n.position_end,
            Node::Break(n) => &n.position_end,
            Node::InterpolatedString(n) => &n.position_end,
            Node::MethodAccess(n) => &n.position_end,
        }
    }

    /// Creates a binary operation node
    pub fn bin_op(left: Node, op_token: Token, right: Node) -> Self {
        Node::BinaryOperator(Box::new(BinaryOperatorNode {
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
    pub token: Token,
    pub position_start: Position,
    pub position_end: Position,
}

impl NumberNode {
    pub fn new(token: Token) -> Self {
        Self {
            position_start: token.position_start.clone(),
            position_end: token.position_end.clone(),
            token,
        }
    }
}

/// String literal node
#[derive(Debug, Clone)]
pub struct StringNode {
    pub token: Token,
    pub position_start: Position,
    pub position_end: Position,
}

impl StringNode {
    pub fn new(token: Token) -> Self {
        Self {
            position_start: token.position_start.clone(),
            position_end: token.position_end.clone(),
            token,
        }
    }
}

/// List literal node
#[derive(Debug, Clone)]
pub struct ListNode {
    pub element_nodes: Vec<Box<Node>>,
    pub position_start: Position,
    pub position_end: Position,
}

#[derive(Debug, Clone)]
pub struct MethodAccessNode {
    pub object: Box<Node>,
    pub method_name: Token,
    pub position_start: Position,
    pub position_end: Position,
}

/// Ternary expression node (condition ? true : false)
#[derive(Debug, Clone)]
pub struct TernaryNode {
    pub condition: Box<Node>,
    pub true_expression: Box<Node>,
    pub false_expression: Box<Node>,
    pub position_start: Position,
    pub position_end: Position,
}

/// Variable access node
#[derive(Debug, Clone)]
pub struct VarAccessNode {
    pub variable_name_token: Token,
    pub position_start: Position,
    pub position_end: Position,
}

/// Variable assignment node
#[derive(Debug, Clone)]
pub struct VarAssignNode {
    pub variable_name_token: Token,
    pub value_node: Box<Node>,
    pub position_start: Position,
    pub position_end: Position,
}

/// Binary operator node
#[derive(Debug, Clone)]
pub struct BinaryOperatorNode {
    pub left_node: Box<Node>,
    pub operator_token: Token,
    pub right_node: Box<Node>,
    pub position_start: Position,
    pub position_end: Position,
}

/// Unary operator node
#[derive(Debug, Clone)]
pub struct UnaryOpNode {
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
pub struct ForNode {
    pub variable_name_token: Token,
    pub start_value_node: Box<Node>,
    pub end_value_node: Box<Node>,
    pub step_value_node: Option<Box<Node>>,
    pub body_node: Box<Node>,
    pub should_return_null: bool,
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
    pub arg_name_toks: Vec<Token>,
    pub body_node: Box<Node>,
    pub should_auto_return: bool,
    pub position_start: Position,
    pub position_end: Position,
}

/// Function call node
#[derive(Debug, Clone)]
pub struct CallNode {
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
    pub parts: Vec<InterpolationPart>,
    pub position_start: Position,
    pub position_end: Position,
}

#[derive(Debug, Clone)]
pub struct InterpolationPart {
    pub is_expression: bool,
    pub content: String, // If text: the literal text, if expression: the expression source
}

impl InterpolatedStringNode {
    pub fn new(token: Token) -> Self {
        // Parse the encoded string
        let mut parts = Vec::new();
        if let Some(encoded) = token.value {
            let content = encoded.trim_start_matches("__INTERPOLATED__");
            for part in content.split('|').skip(1) {
                let mut split = part.splitn(2, ':');
                if let (Some(part_type), Some(content)) = (split.next(), split.next()) {
                    parts.push(InterpolationPart {
                        is_expression: part_type == "expr",
                        content: content.to_string(),
                    });
                }
            }
        }

        Self {
            parts,
            position_start: token.position_start.clone(),
            position_end: token.position_end.clone(),
        }
    }
}
