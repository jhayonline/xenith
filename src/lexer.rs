//! # Lexical Analyzer Module
//!
//! Converts raw source code into a stream of tokens.
//! Handles numbers, strings, identifiers, keywords, operators, and comments.
//! Performs character-by-character scanning with position tracking.

use std::collections::HashMap;

use crate::error::{ExpectedCharError, IllegalCharError};
use crate::position::Position;
use crate::tokens::{KEYWORDS, Token, TokenType};
use crate::utils::*;

/// Lexer that converts source code into tokens
#[derive(Debug, Clone)]
pub struct Lexer {
    pub file_name: String,
    pub text: String,
    pub position: Position,
    pub current_character: Option<char>,
}

impl Lexer {
    /// Creates a new lexer for the given source code
    pub fn new(file_name: String, text: String) -> Self {
        let mut lexer = Self {
            position: Position::new(0, 0, 0, &file_name, &text),
            current_character: None,
            file_name,
            text,
        };

        // Set the first character without advancing first
        lexer.current_character = lexer.text.chars().next();

        // Skip UTF-8 BOM if present (U+FEFF)
        if lexer.current_character == Some('\u{feff}') {
            lexer.advance(); // Skip the BOM
            lexer.current_character = lexer.text.chars().nth(lexer.position.index);
        }

        lexer
    }

    /// Advances to the next character in the source
    pub fn advance(&mut self) {
        self.position.advance(self.current_character);
        self.current_character = self.text.chars().nth(self.position.index);
    }

    /// Skips a comment until the end of line
    pub fn skip_comment(&mut self) {
        self.advance(); // Skip the '#'
        while let Some(c) = self.current_character {
            if c == '\n' {
                break;
            }
            self.advance();
        }
    }

    /// Converts the entire source into a vector of tokens
    pub fn make_tokens(&mut self) -> Result<Vec<Token>, IllegalCharError> {
        let mut tokens = Vec::new();

        while let Some(c) = self.current_character {
            if is_digit(c) {
                tokens.push(self.make_number());
            } else if is_letter(c) {
                tokens.push(self.make_identifier());
            } else {
                match c {
                    ' ' | '\t' => {
                        self.advance();
                    }
                    '#' => {
                        self.skip_comment();
                    }
                    ';' | '\n' => {
                        tokens.push(Token::new(
                            TokenType::Newline,
                            None,
                            self.position.clone(),
                            None,
                        ));
                        self.advance();
                    }
                    '"' => {
                        tokens.push(self.make_string());
                    }
                    // In the match statement for characters, add these cases:
                    '+' => {
                        if self.peek() == Some('+') {
                            let pos_start = self.position.copy();
                            self.advance();
                            self.advance();
                            tokens.push(Token::new(
                                TokenType::PlusPlus,
                                None,
                                pos_start,
                                Some(self.position.clone()),
                            ));
                        } else if self.peek() == Some('=') {
                            let pos_start = self.position.copy();
                            self.advance();
                            self.advance();
                            tokens.push(Token::new(
                                TokenType::PlusEqual,
                                None,
                                pos_start,
                                Some(self.position.clone()),
                            ));
                        } else {
                            tokens.push(Token::new(
                                TokenType::Plus,
                                None,
                                self.position.clone(),
                                None,
                            ));
                            self.advance();
                        }
                    }
                    '-' => {
                        if self.peek() == Some('-') {
                            let pos_start = self.position.copy();
                            self.advance();
                            self.advance();
                            tokens.push(Token::new(
                                TokenType::MinusMinus,
                                None,
                                pos_start,
                                Some(self.position.clone()),
                            ));
                        } else if self.peek() == Some('=') {
                            let pos_start = self.position.copy();
                            self.advance();
                            self.advance();
                            tokens.push(Token::new(
                                TokenType::MinusEqual,
                                None,
                                pos_start,
                                Some(self.position.clone()),
                            ));
                        } else if self.peek() == Some('>') {
                            // Arrow
                            let pos_start = self.position.copy();
                            self.advance();
                            self.advance();
                            tokens.push(Token::new(
                                TokenType::Arrow,
                                None,
                                pos_start,
                                Some(self.position.clone()),
                            ));
                        } else {
                            tokens.push(Token::new(
                                TokenType::Minus,
                                None,
                                self.position.clone(),
                                None,
                            ));
                            self.advance();
                        }
                    }
                    '*' => {
                        tokens.push(Token::new(
                            TokenType::Mul,
                            None,
                            self.position.clone(),
                            None,
                        ));
                        self.advance();
                    }
                    '/' => {
                        tokens.push(Token::new(
                            TokenType::Div,
                            None,
                            self.position.clone(),
                            None,
                        ));
                        self.advance();
                    }
                    '^' => {
                        tokens.push(Token::new(
                            TokenType::Pow,
                            None,
                            self.position.clone(),
                            None,
                        ));
                        self.advance();
                    }
                    '(' => {
                        tokens.push(Token::new(
                            TokenType::LParen,
                            None,
                            self.position.clone(),
                            None,
                        ));
                        self.advance();
                    }
                    ')' => {
                        tokens.push(Token::new(
                            TokenType::RParen,
                            None,
                            self.position.clone(),
                            None,
                        ));
                        self.advance();
                    }
                    '[' => {
                        tokens.push(Token::new(
                            TokenType::LSquare,
                            None,
                            self.position.clone(),
                            None,
                        ));
                        self.advance();
                    }
                    ']' => {
                        tokens.push(Token::new(
                            TokenType::RSquare,
                            None,
                            self.position.clone(),
                            None,
                        ));
                        self.advance();
                    }
                    '{' => {
                        tokens.push(Token::new(
                            TokenType::LBrace,
                            None,
                            self.position.clone(),
                            None,
                        ));
                        self.advance();
                    }
                    '}' => {
                        tokens.push(Token::new(
                            TokenType::RBrace,
                            None,
                            self.position.clone(),
                            None,
                        ));
                        self.advance();
                    }
                    '?' => {
                        tokens.push(Token::new(
                            TokenType::Question,
                            None,
                            self.position.clone(),
                            None,
                        ));
                        self.advance();
                    }
                    ':' => {
                        tokens.push(Token::new(
                            TokenType::Colon,
                            None,
                            self.position.clone(),
                            None,
                        ));
                        self.advance();
                    }
                    '!' if self.peek() == Some('=') => match self.make_not_equals() {
                        Ok(token) => tokens.push(token),
                        Err(e) => return Err(e.into()),
                    },
                    '!' => {
                        tokens.push(Token::new(
                            TokenType::Keyword,
                            Some("!".to_string()),
                            self.position.clone(),
                            None,
                        ));
                        self.advance();
                    }
                    '&' if self.peek() == Some('&') => {
                        let pos_start = self.position.copy();
                        self.advance();
                        self.advance();
                        tokens.push(Token::new(
                            TokenType::Keyword,
                            Some("&&".to_string()),
                            pos_start,
                            Some(self.position.clone()),
                        ));
                    }
                    '|' if self.peek() == Some('|') => {
                        let pos_start = self.position.copy();
                        self.advance();
                        self.advance();
                        tokens.push(Token::new(
                            TokenType::Keyword,
                            Some("||".to_string()),
                            pos_start,
                            Some(self.position.clone()),
                        ));
                    }
                    '=' => {
                        if self.peek() == Some('>') {
                            // Arrow token =>
                            let pos_start = self.position.copy();
                            self.advance();
                            self.advance();
                            tokens.push(Token::new(
                                TokenType::Arrow,
                                None,
                                pos_start,
                                Some(self.position.clone()),
                            ));
                        } else {
                            tokens.push(self.make_equals());
                        }
                    }
                    '<' => {
                        tokens.push(self.make_less_than());
                    }
                    '>' => {
                        tokens.push(self.make_greater_than());
                    }
                    ',' => {
                        tokens.push(Token::new(
                            TokenType::Comma,
                            None,
                            self.position.clone(),
                            None,
                        ));
                        self.advance();
                    }
                    '.' => {
                        tokens.push(Token::new(
                            TokenType::Dot,
                            None,
                            self.position.clone(),
                            None,
                        ));
                        self.advance();
                    }
                    '_' => {
                        tokens.push(Token::new(
                            TokenType::Underscore,
                            Some("_".to_string()),
                            self.position.clone(),
                            None,
                        ));
                        self.advance();
                    }
                    _ => {
                        let pos_start = self.position.copy();
                        let details = format!("'{}'", self.current_character.unwrap());
                        self.advance();
                        return Err(IllegalCharError::new(
                            pos_start,
                            self.position.clone(),
                            &details,
                        ));
                    }
                }
            }
        }

        tokens.push(Token::new(
            TokenType::Eof,
            None,
            self.position.clone(),
            None,
        ));

        Ok(tokens)
    }

    /// Looks at the next character without advancing
    pub fn peek(&mut self) -> Option<char> {
        let peek_index = self.position.index + 1;
        if peek_index < self.text.len() {
            self.text.chars().nth(peek_index)
        } else {
            None
        }
    }

    /// Creates a number token (integer or float)
    pub fn make_number(&mut self) -> Token {
        let mut num_str = String::new();
        let mut dot_count = 0;
        let pos_start = self.position.copy();

        while let Some(c) = self.current_character {
            if is_digit(c) || c == '.' {
                if c == '.' {
                    if dot_count == 1 {
                        break;
                    }
                    dot_count += 1;
                }
                num_str.push(c);
                self.advance();
            } else {
                break;
            }
        }

        let kind = if dot_count == 0 {
            TokenType::Int
        } else {
            TokenType::Float
        };

        Token::new(kind, Some(num_str), pos_start, Some(self.position.clone()))
    }

    /// Creates an identifier or keyword token
    pub fn make_identifier(&mut self) -> Token {
        let mut id_str = String::new();
        let pos_start = self.position.copy();

        while let Some(c) = self.current_character {
            if is_letter_or_digit(c) {
                id_str.push(c);
                self.advance();
            } else {
                break;
            }
        }

        // Special handling for "or when"
        if id_str == "or" {
            let mut temp_index = self.position.index;

            // Skip spaces
            while let Some(c) = self.text.chars().nth(temp_index) {
                if c.is_whitespace() {
                    temp_index += 1;
                } else {
                    break;
                }
            }

            // Peek at the next word
            let next_word: String = self.text.chars().skip(temp_index).take(4).collect(); // "when" has 4 letters
            if next_word == "when" {
                // Consume space + "when"
                while self.position.index < temp_index + 4 {
                    self.advance();
                }

                return Token::new(
                    TokenType::Keyword,
                    Some("or when".to_string()),
                    pos_start,
                    Some(self.position.clone()),
                );
            }
        }

        // Normal keyword or identifier
        let kind = if KEYWORDS.contains(&id_str.as_str()) {
            TokenType::Keyword
        } else {
            TokenType::Identifier
        };

        Token::new(kind, Some(id_str), pos_start, Some(self.position.clone()))
    }

    /// Creates a string token with escape sequence handling
    pub fn make_string(&mut self) -> Token {
        let mut string_val = String::new();
        let mut interpolation_parts = Vec::new();
        let pos_start = self.position.copy();
        let mut escape_char = false;
        let mut is_interpolated = false;

        self.advance(); // Skip opening quote

        let escape_map: HashMap<char, char> =
            HashMap::from([('n', '\n'), ('t', '\t'), ('{', '{'), ('}', '}')]);

        while let Some(c) = self.current_character {
            if c == '"' && !escape_char {
                break;
            }

            if !escape_char && c == '{' && self.peek() == Some('{') {
                // Escaped double brace - add single brace
                self.advance(); // skip second {
                string_val.push('{');
                self.advance();
                continue;
            }

            if !escape_char && c == '}' && self.peek() == Some('}') {
                // Escaped double brace - add single brace
                self.advance(); // skip second }
                string_val.push('}');
                self.advance();
                continue;
            }

            if !escape_char && c == '{' {
                // Start of interpolation
                is_interpolated = true;
                // Clone the string_val before pushing
                interpolation_parts.push(("text".to_string(), string_val.clone()));
                string_val.clear();

                // Parse the expression inside braces
                self.advance(); // skip {

                let mut expr_str = String::new();
                let mut brace_count = 1;

                while let Some(expr_c) = self.current_character {
                    if expr_c == '{' {
                        brace_count += 1;
                        expr_str.push(expr_c);
                        self.advance();
                    } else if expr_c == '}' {
                        brace_count -= 1;
                        if brace_count == 0 {
                            self.advance(); // skip closing }
                            break;
                        }
                        expr_str.push(expr_c);
                        self.advance();
                    } else {
                        expr_str.push(expr_c);
                        self.advance();
                    }
                }

                interpolation_parts.push(("expr".to_string(), expr_str));
                continue;
            }

            if escape_char {
                let escaped = escape_map.get(&c).copied().unwrap_or(c);
                string_val.push(escaped);
                escape_char = false;
            } else if c == '\\' {
                escape_char = true;
            } else {
                string_val.push(c);
            }

            self.advance();
        }

        if self.current_character == Some('"') {
            self.advance(); // Skip closing quote
        }

        if is_interpolated {
            // Add the last text part if there is any
            if !string_val.is_empty() {
                interpolation_parts.push(("text".to_string(), string_val));
            }

            // Store interpolation data
            let mut encoded = String::from("__INTERPOLATED__");
            for (part_type, content) in interpolation_parts {
                encoded.push_str(&format!("|{}:{}", part_type, content));
            }
            Token::new(
                TokenType::InterpolatedString,
                Some(encoded),
                pos_start,
                Some(self.position.clone()),
            )
        } else {
            // For regular strings, just use string_val directly (not moved yet)
            Token::new(
                TokenType::String,
                Some(string_val),
                pos_start,
                Some(self.position.clone()),
            )
        }
    }

    /// Creates an equals or double-equals token
    pub fn make_equals(&mut self) -> Token {
        let mut kind = TokenType::Eq;
        let pos_start = self.position.copy();
        self.advance();

        if self.current_character == Some('=') {
            self.advance();
            kind = TokenType::Ee;
        }

        Token::new(kind, None, pos_start, Some(self.position.clone()))
    }

    /// Creates a not-equals token
    pub fn make_not_equals(&mut self) -> Result<Token, ExpectedCharError> {
        let pos_start = self.position.copy();
        self.advance();

        if self.current_character == Some('=') {
            self.advance();
            return Ok(Token::new(
                TokenType::Ne,
                None,
                pos_start,
                Some(self.position.clone()),
            ));
        }

        Err(ExpectedCharError::new(
            pos_start,
            self.position.clone(),
            "'=' (after '!')",
        ))
    }

    /// Creates a less-than or less-than-or-equal token
    pub fn make_less_than(&mut self) -> Token {
        let mut kind = TokenType::Lt;
        let pos_start = self.position.copy();
        self.advance();

        if self.current_character == Some('=') {
            self.advance();
            kind = TokenType::Lte;
        }

        Token::new(kind, None, pos_start, Some(self.position.clone()))
    }

    /// Creates a greater-than or greater-than-or-equal token
    pub fn make_greater_than(&mut self) -> Token {
        let mut kind = TokenType::Gt;
        let pos_start = self.position.copy();
        self.advance();

        if self.current_character == Some('=') {
            self.advance();
            kind = TokenType::Gte;
        }

        Token::new(kind, None, pos_start, Some(self.position.clone()))
    }

    /// Creates a minus or arrow token
    pub fn make_minus_or_arrow(&mut self) -> Token {
        let mut kind = TokenType::Minus;
        let pos_start = self.position.copy();
        self.advance();

        if self.current_character == Some('>') {
            self.advance();
            kind = TokenType::Arrow;
        }

        Token::new(kind, None, pos_start, Some(self.position.clone()))
    }
}
