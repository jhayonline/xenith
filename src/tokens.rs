//! # Token Definition Module
//!
//! Defines all token types used in Xenith (operators, keywords, literals, etc.)
//! and the Token class which represents lexical units with position information.
//! Serves as the bridge between the lexer and parser.

use crate::position::Position;

/// All possible token types in the Xenith language
#[derive(Debug, Clone, PartialEq)]
pub enum TokenType {
    // Literals
    Int,
    Float,
    String,
    Identifier,
    Keyword,
    InterpolatedString,
    Index,

    // Operators
    Plus,
    PlusPlus,  // ++
    PlusEqual, // +=
    Minus,
    MinusMinus, // --
    MinusEqual, // -=
    Mul,
    Div,
    Pow,
    Eq,  // =
    Ee,  // ==
    Ne,  // !=
    Lt,  // <
    Gt,  // >
    Lte, // <=
    Gte, // >=

    // Delimiters
    LParen,
    RParen,
    LSquare,
    RSquare,
    LBrace,
    RBrace,
    Question,
    Colon,
    Comma,
    Dot,   // . for method calls
    Arrow, // ->
    Newline,
    Eof,
    Match,
    Underscore,
}

/// A token representing a lexical unit
#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenType,
    pub value: Option<String>,
    pub position_start: Position,
    pub position_end: Position,
}

impl Token {
    /// Creates a new token
    pub fn new(
        kind: TokenType,
        value: Option<String>,
        position_start: Position,
        position_end: Option<Position>,
    ) -> Self {
        let pos_end = if let Some(end) = position_end {
            end
        } else {
            let mut copy = position_start.clone();
            copy.advance(None);
            copy
        };

        Self {
            kind,
            value,
            position_start,
            position_end: pos_end,
        }
    }

    /// Checks if the token matches the given type and optional value
    pub fn matches(&self, kind: TokenType, value: Option<&str>) -> bool {
        self.kind == kind && self.value.as_deref() == value
    }
}

/// Reserved keywords in the Xenith language
pub const KEYWORDS: &[&str] = &[
    "spawn",
    "&&",
    "||",
    "!",
    "when",
    "or when",
    "otherwise",
    "for",
    "to",
    "step",
    "while",
    "method",
    "release",
    "skip",
    "stop",
    "match",
    "in",
    "try",
    "catch",
    "panic",
    "grab",
    "export",
    "as",
    "from",
];
