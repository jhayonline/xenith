use core::num;

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
                            Some(self.position.clone()),
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
                            Some(self.position.clone()),
                            None,
                        ));

                        self.advance();
                    }
                    '-' => {
                        tokens.push(Token::new(
                            TokenType::Minus,
                            None,
                            Some(self.position.clone()),
                            None,
                        ));

                        self.advance();
                    }
                    '/' => {
                        tokens.push(Token::new(
                            TokenType::Div,
                            None,
                            Some(self.position.clone()),
                            None,
                        ));

                        self.advance();
                    }
                    '^' => {
                        tokens.push(Token::new(
                            TokenType::Pow,
                            None,
                            Some(self.position.clone()),
                            None,
                        ));

                        self.advance();
                    }
                    '(' => {
                        tokens.push(Token::new(
                            TokenType::LParen,
                            None,
                            Some(self.position.clone()),
                            None,
                        ));

                        self.advance();
                    }
                    ')' => {
                        tokens.push(Token::new(
                            TokenType::RParen,
                            None,
                            Some(self.position.clone()),
                            None,
                        ));

                        self.advance();
                    }
                    '[' => {
                        tokens.push(Token::new(
                            TokenType::LSquare,
                            None,
                            Some(self.position.clone()),
                            None,
                        ));

                        self.advance();
                    }
                    ']' => {
                        tokens.push(Token::new(
                            TokenType::RSquare,
                            None,
                            Some(self.position.clone()),
                            None,
                        ));

                        self.advance();
                    }
                    '{' => {
                        tokens.push(Token::new(
                            TokenType::LBrace,
                            None,
                            Some(self.position.clone()),
                            None,
                        ));

                        self.advance();
                    }
                    '}' => {
                        tokens.push(Token::new(
                            TokenType::RBrace,
                            None,
                            Some(self.position.clone()),
                            None,
                        ));

                        self.advance();
                    }
                    '?' => {
                        tokens.push(Token::new(
                            TokenType::Question,
                            None,
                            Some(self.position.clone()),
                            None,
                        ));

                        self.advance();
                    }
                    ':' => {
                        tokens.push(Token::new(
                            TokenType::Colon,
                            None,
                            Some(self.position.clone()),
                            None,
                        ));

                        self.advance();
                    }
                    '!' if self.peek() == Some('=') => {
                        let token = self.make_not_equals();
                        tokens.push(token);

                        self.advance();
                    }
                    '!' if self.peek() != Some('=') => {
                        tokens.push(Token::new(
                            TokenType::Keyword,
                            Some("!".to_string()),
                            Some(self.position.clone()),
                            None,
                        ));

                        self.advance();
                    }
                    '&' if self.peek() == Some('&') => {
                        let position_start = self.position.copy();

                        self.advance();
                        self.advance();

                        tokens.push(Token::new(
                            TokenType::Keyword,
                            Some("&&".to_string()),
                            Some(position_start.clone()),
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
                            Some(position_start.clone()),
                            Some(self.position.clone()),
                        ));

                        self.advance();
                        self.advance();
                    }
                    '=' => tokens.push(self.make_equals()),
                    '<' => tokens.push(self.make_less_than()),
                    '>' => tokens.push(self.make_greater_than()),
                    ',' => {
                        tokens.push(Token::new(
                            TokenType::Comma,
                            None,
                            Some(self.position.clone()),
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
            Some(self.position.clone()),
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
                Some(position_start),
                Some(self.position.clone()),
            );
        }

        Token::new(
            TokenType::Float,
            Some(number_string),
            Some(position_start),
            Some(self.position.clone()),
        )
    }

    pub fn make_identifier(&mut self) -> Token {
        todo!()
    }

    pub fn make_string(&mut self) -> Token {
        todo!()
    }

    pub fn make_equals(&mut self) -> Token {
        todo!()
    }

    pub fn make_not_equals(&mut self) -> Token {
        todo!()
    }

    pub fn make_less_than(&mut self) -> Token {
        todo!()
    }

    pub fn make_greater_than(&mut self) -> Token {
        todo!()
    }
}

pub fn dummy() {}
