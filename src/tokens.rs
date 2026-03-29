use crate::position::Position;

#[derive(Debug, Clone, PartialEq)]
pub enum TokenType {
    Int,
    Float,
    String,
    Identifier,
    Keyword,

    Plus,
    Minus,
    Mul,
    Div,
    Pow,
    Eq,

    LParen,
    RParen,
    LSquare,
    RSquare,
    LBrace,
    RBrace,

    Question,
    Colon,
    Ee, // ==
    Ne, // !=
    Lt,
    Gt,
    Lte,
    Gte,
    Comma,
    Arrow, // ->
    Newline,
    Eof,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenType,
    pub value: Option<String>,
    pub position_start: Option<Position>,
    pub position_end: Option<Position>,
}

impl Token {
    pub fn new(
        kind: TokenType,
        value: Option<String>,
        position_start: Option<Position>,
        position_end: Option<Position>,
    ) -> Self {
        let pos_start_copy = position_start.clone();
        let pos_end_copy = if let Some(pos_end) = position_end.clone() {
            Some(pos_end)
        } else if let Some(pos_start) = position_start.clone() {
            let mut copy = pos_start.clone();
            copy.advance(None);
            Some(copy)
        } else {
            None
        };

        Self {
            kind,
            value,
            position_start: pos_start_copy,
            position_end: pos_end_copy,
        }
    }

    pub fn matches(&self, kind: TokenType, value: Option<&str>) -> bool {
        self.kind == kind && self.value.as_deref() == value
    }
}

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
];
