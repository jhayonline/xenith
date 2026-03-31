use std::collections::HashMap;

use crate::error::{ExpectedCharError, IllegalCharError};
use crate::position::Position;
use crate::tokens::{Token, TokenType};
use crate::utils::*;

#[derive(Debug, Clone)]
pub struct Lexer {
    pub file_name: String,
    pub text: String,
    pub position: Position,
    pub current_character: Option<char>,
}

impl Lexer {
    pub fn new(file_name: String, text: String) -> Self {
        let mut lexer = Self {
            position: Position::new(0, 0, 0, &file_name, &text),
            current_character: None,
            file_name,
            text,
        };

        lexer.advance(); // move to first char
        lexer
    }

    pub fn advance(&mut self) {
        self.position.advance(self.current_character);

        if self.position.index < self.text.len() {
            self.current_character = self.text.chars().nth(self.position.index);
        } else {
            self.current_character = None;
        }
    }

    pub fn skip_comment(&mut self) {
        self.advance();

        while let Some(c) = self.current_character {
            if c == '\n' {
                break;
            }
            self.advance();
        }

        self.advance();
    }

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
                        let token = self.make_string();
                        tokens.push(token);
                    }
                    '+' => {
                        tokens.push(Token::new(
                            TokenType::Plus,
                            None,
                            self.position.clone(),
                            None,
                        ));

                        self.advance();
                    }
                    '-' => {
                        tokens.push(Token::new(
                            TokenType::Minus,
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
                    '!' if self.peek() == Some('=') => {
                        let token = self.make_not_equals();

                        match token {
                            Ok(value) => tokens.push(value),
                            Err(error) => println!("Error: {:?}", error),
                        }

                        self.advance();
                    }
                    '!' if self.peek() != Some('=') => {
                        tokens.push(Token::new(
                            TokenType::Keyword,
                            Some("!".to_string()),
                            self.position.clone(),
                            None,
                        ));
                    }
                    '&' if self.peek() == Some('&') => {
                        let position_start = self.position.copy();

                        self.advance();
                        self.advance();

                        tokens.push(Token::new(
                            TokenType::Keyword,
                            Some("&&".to_string()),
                            position_start.clone(),
                            Some(self.position.clone()),
                        ));
                    }
                    '|' if self.peek() == Some('|') => {
                        let position_start = self.position.copy();

                        self.advance();
                        self.advance();

                        tokens.push(Token::new(
                            TokenType::Keyword,
                            Some("||".to_string()),
                            position_start.clone(),
                            Some(self.position.clone()),
                        ));
                    }
                    '=' => tokens.push(self.make_equals()),
                    '<' => tokens.push(self.make_less_than()),
                    '>' => tokens.push(self.make_greater_than()),
                    ',' => {
                        tokens.push(Token::new(
                            TokenType::Comma,
                            None,
                            self.position.clone(),
                            None,
                        ));

                        self.advance();
                    }
                    _ => {
                        let position_start = self.position.copy();
                        let details = format!("'{}'", self.current_character.unwrap());

                        self.advance();

                        return Err(IllegalCharError::new(
                            position_start,
                            self.position.clone(),
                            details.as_str(),
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

    pub fn peek(&mut self) -> Option<char> {
        let peek_index = self.position.index + 1;

        if peek_index < self.text.len() {
            self.text.chars().nth(peek_index)
        } else {
            None
        }
    }

    pub fn make_number(&mut self) -> Token {
        let mut number_string = String::new();
        let mut dot_count = 0;
        let position_start = self.position.copy();

        while let Some(c) = self.current_character {
            if is_digit(c) || c == '.' {
                if c == '.' {
                    if dot_count == 1 {
                        break;
                    }
                    dot_count += 1;
                }
                number_string.push(c);
                self.advance();
            } else {
                break;
            }
        }

        if dot_count == 0 {
            return Token::new(
                TokenType::Int,
                Some(number_string),
                position_start,
                Some(self.position.clone()),
            );
        }

        Token::new(
            TokenType::Float,
            Some(number_string),
            position_start,
            Some(self.position.clone()),
        )
    }

    pub fn make_identifier(&mut self) -> Token {
        todo!()
    }

    pub fn make_string(&mut self) -> Token {
        let mut string_value = String::new();
        let position_start = self.position.copy();

        // Flag to track if the next character is escaped
        let mut escape_character = false;

        // Skip the opening quote
        self.advance();

        // Map of escape sequences
        let escape_characters: HashMap<char, char> = HashMap::from([('n', '\n'), ('t', '\t')]);

        while let Some(c) = self.current_character {
            if c == '"' && !escape_character {
                // Closing quote found, stop parsing
                break;
            }

            if escape_character {
                // If previous char was a backslash, interpret escape
                let escaped = escape_characters.get(&c).copied().unwrap_or(c);
                string_value.push(escaped);
                escape_character = false; // reset the flag
            } else {
                if c == '\\' {
                    escape_character = true; // next char is escaped
                } else {
                    string_value.push(c);
                }
            }

            self.advance();
        }

        // handle unterminated string; raise an error
        if self.current_character == Some('"') {
            self.advance(); // Skip the closing quote
        }

        Token::new(
            TokenType::String,
            Some(string_value),
            position_start,
            Some(self.position.clone()),
        )
    }

    pub fn make_equals(&mut self) -> Token {
        let mut token_type = TokenType::Eq;
        let position_start = self.position.copy();

        self.advance();

        if let Some(c) = self.current_character {
            if c == '=' {
                self.advance();
                token_type = TokenType::Ee;
            }
        }

        Token::new(
            token_type,
            None,
            position_start,
            Some(self.position.clone()),
        )
    }

    pub fn make_not_equals(&mut self) -> Result<Token, ExpectedCharError> {
        let position_start = self.position.copy();
        self.advance();

        if let Some(c) = self.current_character {
            if c == '=' {
                self.advance();

                let new_token = Token::new(
                    TokenType::Ne,
                    None,
                    position_start,
                    Some(self.position.copy()),
                );

                return Ok(new_token);
            }
        }

        self.advance();

        let err_value =
            ExpectedCharError::new(position_start, self.position.clone(), "'=' (after '!')");

        Err(err_value)
    }

    pub fn make_less_than(&mut self) -> Token {
        let mut token_type = TokenType::Lt;
        let position_start = self.position.copy();

        self.advance();

        if let Some(c) = self.current_character {
            if c == '=' {
                self.advance();
                token_type = TokenType::Lte;
            }
        }

        Token::new(
            token_type,
            None,
            position_start,
            Some(self.position.clone()),
        )
    }

    pub fn make_greater_than(&mut self) -> Token {
        let mut token_type = TokenType::Gt;
        let position_start = self.position.copy();

        self.advance();

        if let Some(c) = self.current_character {
            if c == '=' {
                self.advance();
                token_type = TokenType::Gte;
            }
        }

        Token::new(
            token_type,
            None,
            position_start,
            Some(self.position.clone()),
        )
    }

    pub fn make_minus_or_arrow(&mut self) -> Token {
        let mut token_type = TokenType::Minus;
        let position_start = self.position.copy();

        self.advance();

        if let Some(c) = self.current_character {
            if c == '>' {
                self.advance();
                token_type = TokenType::Arrow;
            }
        }

        Token::new(
            token_type,
            None,
            position_start,
            Some(self.position.clone()),
        )
    }
}

pub fn dummy() {}
