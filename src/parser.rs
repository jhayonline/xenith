//! # Syntax Parser Module
//!
//! Implements recursive descent parsing to transform token streams
//! into an Abstract Syntax Tree (AST). Uses {} block syntax.

use crate::error::InvalidSyntaxError;
use crate::nodes::*;
use crate::parse_result::ParseResult;
use crate::position::Position;
use crate::tokens::{Token, TokenType};

/// Recursive descent parser for Xenith
#[derive(Debug)]
pub struct Parser {
    pub tokens: Vec<Token>,
    pub token_index: usize,
}

impl Parser {
    /// Creates a new parser from a token stream
    pub fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            token_index: 0,
        }
    }

    fn current_token(&self) -> Option<&Token> {
        self.tokens.get(self.token_index)
    }

    fn peek_token(&self) -> Option<&Token> {
        self.tokens.get(self.token_index + 1)
    }

    fn peek_token_at(&self, offset: usize) -> Option<&Token> {
        self.tokens.get(self.token_index + offset)
    }

    fn advance(&mut self) {
        self.token_index += 1;
    }

    pub fn parse_expression(&mut self) -> ParseResult {
        // Save the current token index
        let start_index = self.token_index;

        // Try to parse a single expression
        let result = self.ternary_expr();

        // Check if we consumed all tokens
        if let Some(tok) = self.current_token() {
            if tok.kind != TokenType::Eof {
                // If there are remaining tokens, it's not a single expression
                // Reset and try parsing as statements
                self.token_index = start_index;
                return self.statements();
            }
        }

        result
    }

    fn dummy_pos() -> Position {
        Position::new(0, 0, 0, "", "")
    }

    /// Parses the entire token stream into an AST
    pub fn parse(&mut self) -> ParseResult {
        let result = self.statements();

        // Check we consumed all tokens
        if let Some(tok) = self.current_token() {
            if tok.kind != TokenType::Eof {
                return result.failure(
                    InvalidSyntaxError::new(
                        tok.position_start.clone(),
                        tok.position_end.clone(),
                        "Unexpected token",
                    )
                    .base,
                );
            }
        }

        result
    }

    fn statements(&mut self) -> ParseResult {
        let mut result = ParseResult::new();
        let mut statements = Vec::new();
        let pos_start = self
            .current_token()
            .map(|t| t.position_start.clone())
            .unwrap_or_else(Self::dummy_pos);

        // Skip leading newlines
        while let Some(tok) = self.current_token() {
            if tok.kind == TokenType::Newline {
                self.advance();
            } else {
                break;
            }
        }

        // Parse statements
        while let Some(stmt) = result.try_register(&self.statement()) {
            statements.push(Box::new(stmt));

            // Skip newlines between statements
            while let Some(tok) = self.current_token() {
                if tok.kind == TokenType::Newline {
                    self.advance();
                } else {
                    break;
                }
            }
        }

        let pos_end = self
            .current_token()
            .map(|t| t.position_end.clone())
            .unwrap_or_else(|| pos_start.clone());

        result.success(Node::List(ListNode {
            element_nodes: statements,
            position_start: pos_start,
            position_end: pos_end,
        }))
    }

    fn block(&mut self) -> ParseResult {
        let mut result = ParseResult::new();
        let pos_start = self
            .current_token()
            .map(|t| t.position_start.clone())
            .unwrap_or_else(Self::dummy_pos);

        // Expect '{'
        match self.current_token() {
            Some(tok) if tok.kind == TokenType::LBrace => {
                self.advance();
            }
            Some(tok) => {
                return result.failure(
                    InvalidSyntaxError::new(
                        tok.position_start.clone(),
                        tok.position_end.clone(),
                        "Expected '{'",
                    )
                    .base,
                );
            }
            None => {
                return result.failure(
                    InvalidSyntaxError::new(Self::dummy_pos(), Self::dummy_pos(), "Expected '{'")
                        .base,
                );
            }
        }

        // Empty block
        if let Some(t) = self.current_token().cloned() {
            if t.kind == TokenType::RBrace {
                let pos_end = t.position_end.clone();
                self.advance();

                return result.success(Node::List(ListNode {
                    element_nodes: Vec::new(),
                    position_start: pos_start,
                    position_end: pos_end,
                }));
            }
        }

        // Parse statements inside block
        let statements = result.register(&self.statements());
        if result.error.is_some() {
            return result;
        }

        // Expect '}'
        match self.current_token() {
            Some(tok) if tok.kind == TokenType::RBrace => {
                self.advance();
            }
            Some(tok) => {
                return result.failure(
                    InvalidSyntaxError::new(
                        tok.position_start.clone(),
                        tok.position_end.clone(),
                        "Expected '}'",
                    )
                    .base,
                );
            }
            None => {
                return result.failure(
                    InvalidSyntaxError::new(Self::dummy_pos(), Self::dummy_pos(), "Expected '}'")
                        .base,
                );
            }
        }

        if let Some(node) = statements {
            result.success(node)
        } else {
            result.failure(
                InvalidSyntaxError::new(pos_start.clone(), pos_start, "Invalid block").base,
            )
        }
    }

    fn statement(&mut self) -> ParseResult {
        let mut result = ParseResult::new();
        let pos_start = self
            .current_token()
            .map(|t| t.position_start.clone())
            .unwrap_or_else(Self::dummy_pos);

        // Check for return statement
        if let Some(tok) = self.current_token() {
            if tok.matches(TokenType::Keyword, Some("release")) {
                self.advance();
                let expr = result.try_register(&self.expr());
                let pos_end = self
                    .current_token()
                    .map(|t| t.position_end.clone())
                    .unwrap_or_else(|| pos_start.clone());

                return result.success(Node::Return(Box::new(ReturnNode {
                    node_to_return: expr.map(|e| Box::new(e)),
                    position_start: pos_start,
                    position_end: pos_end,
                })));
            }
        }

        // Check for continue statement
        if let Some(tok) = self.current_token() {
            if tok.matches(TokenType::Keyword, Some("skip")) {
                self.advance();
                let pos_end = self
                    .current_token()
                    .map(|t| t.position_end.clone())
                    .unwrap_or_else(|| pos_start.clone());
                return result.success(Node::Continue(ContinueNode {
                    position_start: pos_start,
                    position_end: pos_end,
                }));
            }
        }

        // Check for break statement
        if let Some(tok) = self.current_token() {
            if tok.matches(TokenType::Keyword, Some("stop")) {
                self.advance();
                let pos_end = self
                    .current_token()
                    .map(|t| t.position_end.clone())
                    .unwrap_or_else(|| pos_start.clone());
                return result.success(Node::Break(BreakNode {
                    position_start: pos_start,
                    position_end: pos_end,
                }));
            }
        }

        // Parse expression statement
        let expr = result.register(&self.expr());
        if result.error.is_some() {
            return result;
        }

        if let Some(node) = expr {
            result.success(node)
        } else {
            result
        }
    }

    fn expr(&mut self) -> ParseResult {
        // Check for variable declaration with 'spawn'
        if let Some(tok) = self.current_token() {
            if tok.matches(TokenType::Keyword, Some("spawn")) {
                return self.var_declaration();
            }
        }

        // Check for variable reassignment (identifier followed by '=' without 'spawn')
        if let Some(tok) = self.current_token() {
            if tok.kind == TokenType::Identifier {
                // Peek ahead to see if next token is '=', '+=', '-=', '++', or '--'
                if let Some(next_tok) = self.peek_token() {
                    if next_tok.kind == TokenType::Eq {
                        return self.var_reassignment();
                    } else if next_tok.kind == TokenType::PlusEqual {
                        return self.compound_assignment(TokenType::PlusEqual);
                    } else if next_tok.kind == TokenType::MinusEqual {
                        return self.compound_assignment(TokenType::MinusEqual);
                    } else if next_tok.kind == TokenType::PlusPlus {
                        return self.increment_decrement(true); // true for increment
                    } else if next_tok.kind == TokenType::MinusMinus {
                        return self.increment_decrement(false); // false for decrement
                    }
                }
            }
        }

        // Parse ternary expression (for non-assignment expressions)
        self.ternary_expr()
    }

    // Compound assignment (+=, -=)
    fn compound_assignment(&mut self, op: TokenType) -> ParseResult {
        let mut result = ParseResult::new();

        let var_name = match self.current_token() {
            Some(t) if t.kind == TokenType::Identifier => t.clone(),
            Some(t) => {
                return result.failure(
                    InvalidSyntaxError::new(
                        t.position_start.clone(),
                        t.position_end.clone(),
                        "Expected identifier",
                    )
                    .base,
                );
            }
            None => {
                return result.failure(
                    InvalidSyntaxError::new(
                        Self::dummy_pos(),
                        Self::dummy_pos(),
                        "Expected identifier",
                    )
                    .base,
                );
            }
        };
        let pos_start = var_name.position_start.clone();
        self.advance(); // consume identifier

        let op_token = match self.current_token() {
            Some(t) => t.clone(),
            None => {
                return result.failure(
                    InvalidSyntaxError::new(
                        Self::dummy_pos(),
                        Self::dummy_pos(),
                        "Expected operator",
                    )
                    .base,
                );
            }
        };
        self.advance(); // consume the operator

        let value = result.register(&self.expr());
        if result.error.is_some() {
            return result;
        }

        if let Some(val_node) = value {
            // Store the position before moving val_node
            let val_pos_end = val_node.position_end().clone();

            // Create a binary operation: var = var + value or var = var - value
            let left_node = Node::VarAccess(VarAccessNode {
                variable_name_token: var_name.clone(),
                position_start: pos_start.clone(),
                position_end: pos_start.clone(),
            });

            let binary_op = if op == TokenType::PlusEqual {
                Token::new(TokenType::Plus, None, pos_start.clone(), None)
            } else {
                Token::new(TokenType::Minus, None, pos_start.clone(), None)
            };

            let bin_op_node = Node::BinaryOperator(Box::new(BinaryOperatorNode {
                left_node: Box::new(left_node),
                operator_token: binary_op,
                right_node: Box::new(val_node),
                position_start: pos_start.clone(),
                position_end: val_pos_end,
            }));

            let pos_end = self
                .current_token()
                .map(|t| t.position_end.clone())
                .unwrap_or_else(|| pos_start.clone());

            return result.success(Node::VarAssign(Box::new(VarAssignNode {
                variable_name_token: var_name,
                value_node: Box::new(bin_op_node),
                position_start: pos_start,
                position_end: pos_end,
            })));
        }

        result
    }

    // Increment (++) and decrement (--)
    fn increment_decrement(&mut self, is_increment: bool) -> ParseResult {
        let mut result = ParseResult::new();

        let var_name = match self.current_token() {
            Some(t) if t.kind == TokenType::Identifier => t.clone(),
            Some(t) => {
                return result.failure(
                    InvalidSyntaxError::new(
                        t.position_start.clone(),
                        t.position_end.clone(),
                        "Expected identifier",
                    )
                    .base,
                );
            }
            None => {
                return result.failure(
                    InvalidSyntaxError::new(
                        Self::dummy_pos(),
                        Self::dummy_pos(),
                        "Expected identifier",
                    )
                    .base,
                );
            }
        };
        let pos_start = var_name.position_start.clone();
        self.advance(); // consume identifier

        // Consume the ++ or -- operator
        match self.current_token() {
            Some(t) => {
                self.advance();
            }
            None => {
                return result.failure(
                    InvalidSyntaxError::new(
                        Self::dummy_pos(),
                        Self::dummy_pos(),
                        "Expected increment/decrement operator",
                    )
                    .base,
                );
            }
        }

        // Create: var = var + 1 or var = var - 1
        let left_node = Node::VarAccess(VarAccessNode {
            variable_name_token: var_name.clone(),
            position_start: pos_start.clone(),
            position_end: pos_start.clone(),
        });

        let one_node = Node::Number(NumberNode::new(Token::new(
            TokenType::Int,
            Some("1".to_string()),
            pos_start.clone(),
            None,
        )));

        let binary_op = if is_increment {
            Token::new(TokenType::Plus, None, pos_start.clone(), None)
        } else {
            Token::new(TokenType::Minus, None, pos_start.clone(), None)
        };

        let bin_op_node = Node::BinaryOperator(Box::new(BinaryOperatorNode {
            left_node: Box::new(left_node),
            operator_token: binary_op,
            right_node: Box::new(one_node),
            position_start: pos_start.clone(),
            position_end: pos_start.clone(),
        }));

        let pos_end = self
            .current_token()
            .map(|t| t.position_end.clone())
            .unwrap_or_else(|| pos_start.clone());

        result.success(Node::VarAssign(Box::new(VarAssignNode {
            variable_name_token: var_name,
            value_node: Box::new(bin_op_node),
            position_start: pos_start,
            position_end: pos_end,
        })))
    }

    // Variable declaration with 'spawn' keyword
    fn var_declaration(&mut self) -> ParseResult {
        let mut result = ParseResult::new();
        let pos_start = match self.current_token() {
            Some(t) if t.matches(TokenType::Keyword, Some("spawn")) => t.position_start.clone(),
            _ => {
                return result.failure(
                    InvalidSyntaxError::new(
                        Self::dummy_pos(),
                        Self::dummy_pos(),
                        "Expected 'spawn'",
                    )
                    .base,
                );
            }
        };
        self.advance(); // consume 'spawn'

        let var_name = match self.current_token() {
            Some(t) if t.kind == TokenType::Identifier => t.clone(),
            Some(t) => {
                return result.failure(
                    InvalidSyntaxError::new(
                        t.position_start.clone(),
                        t.position_end.clone(),
                        "Expected identifier",
                    )
                    .base,
                );
            }
            None => {
                return result.failure(
                    InvalidSyntaxError::new(
                        Self::dummy_pos(),
                        Self::dummy_pos(),
                        "Expected identifier",
                    )
                    .base,
                );
            }
        };
        self.advance();

        match self.current_token() {
            Some(t) if t.kind == TokenType::Eq => {
                self.advance();
            }
            Some(t) => {
                return result.failure(
                    InvalidSyntaxError::new(
                        t.position_start.clone(),
                        t.position_end.clone(),
                        "Expected '='",
                    )
                    .base,
                );
            }
            None => {
                return result.failure(
                    InvalidSyntaxError::new(Self::dummy_pos(), Self::dummy_pos(), "Expected '='")
                        .base,
                );
            }
        }

        let value = result.register(&self.expr());
        if result.error.is_some() {
            return result;
        }

        if let Some(val_node) = value {
            let pos_end = self
                .current_token()
                .map(|t| t.position_end.clone())
                .unwrap_or_else(|| pos_start.clone());

            return result.success(Node::VarAssign(Box::new(VarAssignNode {
                variable_name_token: var_name,
                value_node: Box::new(val_node),
                position_start: pos_start,
                position_end: pos_end,
            })));
        }

        result
    }

    // Variable reassignment (without 'spawn')
    fn var_reassignment(&mut self) -> ParseResult {
        let mut result = ParseResult::new();

        let var_name = match self.current_token() {
            Some(t) if t.kind == TokenType::Identifier => t.clone(),
            _ => {
                return result.failure(
                    InvalidSyntaxError::new(
                        Self::dummy_pos(),
                        Self::dummy_pos(),
                        "Expected identifier",
                    )
                    .base,
                );
            }
        };
        let pos_start = var_name.position_start.clone();
        self.advance(); // consume identifier

        match self.current_token() {
            Some(t) if t.kind == TokenType::Eq => {
                self.advance(); // consume '='
            }
            Some(t) => {
                return result.failure(
                    InvalidSyntaxError::new(
                        t.position_start.clone(),
                        t.position_end.clone(),
                        "Expected '='",
                    )
                    .base,
                );
            }
            None => {
                return result.failure(
                    InvalidSyntaxError::new(Self::dummy_pos(), Self::dummy_pos(), "Expected '='")
                        .base,
                );
            }
        }

        let value = result.register(&self.expr());
        if result.error.is_some() {
            return result;
        }

        if let Some(val_node) = value {
            let pos_end = self
                .current_token()
                .map(|t| t.position_end.clone())
                .unwrap_or_else(|| pos_start.clone());

            return result.success(Node::VarAssign(Box::new(VarAssignNode {
                variable_name_token: var_name,
                value_node: Box::new(val_node),
                position_start: pos_start,
                position_end: pos_end,
            })));
        }

        result
    }

    fn match_expr(&mut self) -> ParseResult {
        let mut result = ParseResult::new();

        // Expect 'match' keyword
        let pos_start = match self.current_token() {
            Some(t) if t.matches(TokenType::Keyword, Some("match")) => t.position_start.clone(),
            Some(t) => {
                return result.failure(
                    InvalidSyntaxError::new(
                        t.position_start.clone(),
                        t.position_end.clone(),
                        "Expected 'match'",
                    )
                    .base,
                );
            }
            None => {
                return result.failure(
                    InvalidSyntaxError::new(
                        Self::dummy_pos(),
                        Self::dummy_pos(),
                        "Expected 'match'",
                    )
                    .base,
                );
            }
        };
        self.advance(); // consume 'match'

        // Parse the value to match against
        let value_node = result.register(&self.expr());
        if result.error.is_some() {
            return result;
        }

        // Skip newlines before the '{'
        while let Some(tok) = self.current_token() {
            if tok.kind == TokenType::Newline {
                self.advance();
            } else {
                break;
            }
        }

        // Expect '{'
        match self.current_token() {
            Some(t) if t.kind == TokenType::LBrace => {
                self.advance();
            }
            Some(t) => {
                return result.failure(
                    InvalidSyntaxError::new(
                        t.position_start.clone(),
                        t.position_end.clone(),
                        "Expected '{'",
                    )
                    .base,
                );
            }
            None => {
                return result.failure(
                    InvalidSyntaxError::new(Self::dummy_pos(), Self::dummy_pos(), "Expected '{'")
                        .base,
                );
            }
        }

        // Parse match arms
        let mut arms = Vec::new();

        loop {
            // Skip newlines before each arm
            while let Some(t) = self.current_token() {
                if t.kind == TokenType::Newline {
                    self.advance();
                } else {
                    break;
                }
            }

            // Check for closing brace
            if let Some(t) = self.current_token() {
                if t.kind == TokenType::RBrace {
                    self.advance();
                    break;
                }
            } else {
                break;
            }

            // Parse a match arm - get position before consuming tokens
            let arm_pos_start = match self.current_token() {
                Some(t) => t.position_start.clone(),
                None => break,
            };

            // Parse pattern (can be literal, identifier, or underscore)
            let pattern_node = result.register(&self.match_pattern());
            if result.error.is_some() {
                return result;
            }

            // Skip newlines before '=>'
            while let Some(t) = self.current_token() {
                if t.kind == TokenType::Newline {
                    self.advance();
                } else {
                    break;
                }
            }

            // Expect '=>' (Arrow token)
            match self.current_token() {
                Some(t) if t.kind == TokenType::Arrow => {
                    self.advance();
                }
                Some(t) => {
                    return result.failure(
                        InvalidSyntaxError::new(
                            t.position_start.clone(),
                            t.position_end.clone(),
                            "Expected '=>'",
                        )
                        .base,
                    );
                }
                None => {
                    return result.failure(
                        InvalidSyntaxError::new(
                            Self::dummy_pos(),
                            Self::dummy_pos(),
                            "Expected '=>'",
                        )
                        .base,
                    );
                }
            }

            // Skip newlines after '=>' before body
            while let Some(t) = self.current_token() {
                if t.kind == TokenType::Newline {
                    self.advance();
                } else {
                    break;
                }
            }

            // Parse body (can be a block or a single expression)
            let body_node = if let Some(t) = self.current_token() {
                if t.kind == TokenType::LBrace {
                    result.register(&self.block())
                } else {
                    result.register(&self.statement())
                }
            } else {
                result.register(&self.statement())
            };

            if result.error.is_some() {
                return result;
            }

            let arm_pos_end = self
                .current_token()
                .map(|t| t.position_end.clone())
                .unwrap_or_else(|| arm_pos_start.clone());

            if let (Some(pattern), Some(body)) = (pattern_node, body_node) {
                arms.push(MatchArm {
                    pattern_node: Box::new(pattern),
                    body_node: Box::new(body),
                    position_start: arm_pos_start,
                    position_end: arm_pos_end,
                });
            }
        }

        let pos_end = self
            .current_token()
            .map(|t| t.position_end.clone())
            .unwrap_or_else(|| pos_start.clone());

        if let Some(value) = value_node {
            result.success(Node::Match(Box::new(MatchNode {
                value_node: Box::new(value),
                arms,
                position_start: pos_start,
                position_end: pos_end,
            })))
        } else {
            result
        }
    }

    fn match_pattern(&mut self) -> ParseResult {
        let mut result = ParseResult::new();

        match self.current_token() {
            Some(t) if t.kind == TokenType::Int || t.kind == TokenType::Float => {
                let node = Node::Number(NumberNode::new(t.clone()));
                self.advance();
                result.success(node)
            }
            Some(t) if t.kind == TokenType::String => {
                let node = Node::String(StringNode::new(t.clone()));
                self.advance();
                result.success(node)
            }
            Some(t) if t.kind == TokenType::Underscore => {
                // Create a special variable access for underscore
                let node = Node::VarAccess(VarAccessNode {
                    variable_name_token: t.clone(),
                    position_start: t.position_start.clone(),
                    position_end: t.position_end.clone(),
                });
                self.advance();
                result.success(node)
            }
            Some(t) if t.kind == TokenType::Identifier => {
                let node = Node::VarAccess(VarAccessNode {
                    variable_name_token: t.clone(),
                    position_start: t.position_start.clone(),
                    position_end: t.position_end.clone(),
                });
                self.advance();
                result.success(node)
            }
            Some(t) => result.failure(
                InvalidSyntaxError::new(
                    t.position_start.clone(),
                    t.position_end.clone(),
                    "Expected pattern",
                )
                .base,
            ),
            None => result.failure(
                InvalidSyntaxError::new(Self::dummy_pos(), Self::dummy_pos(), "Expected pattern")
                    .base,
            ),
        }
    }

    fn map_expr(&mut self) -> ParseResult {
        let mut result = ParseResult::new();
        let mut pairs = Vec::new();

        let pos_start = match self.current_token() {
            Some(t) if t.kind == TokenType::LBrace => t.position_start.clone(),
            Some(t) => {
                return result.failure(
                    InvalidSyntaxError::new(
                        t.position_start.clone(),
                        t.position_end.clone(),
                        "Expected '{'",
                    )
                    .base,
                );
            }
            None => {
                return result.failure(
                    InvalidSyntaxError::new(Self::dummy_pos(), Self::dummy_pos(), "Expected '{'")
                        .base,
                );
            }
        };
        self.advance(); // consume '{'

        // Skip newlines after the opening brace
        while let Some(t) = self.current_token() {
            if t.kind == TokenType::Newline {
                self.advance();
            } else {
                break;
            }
        }

        // Check for empty map - use peek to avoid borrowing issues
        let is_empty = if let Some(t) = self.current_token() {
            t.kind == TokenType::RBrace
        } else {
            false
        };

        if is_empty {
            let pos_end = self.current_token().unwrap().position_end.clone();
            self.advance(); // consume '}'
            return result.success(Node::Map(MapNode {
                pairs,
                position_start: pos_start,
                position_end: pos_end,
            }));
        }

        // Parse key-value pairs
        loop {
            // Parse key (must be a string or identifier)
            let key_node = match self.current_token() {
                Some(t) if t.kind == TokenType::String => {
                    let node = Node::String(StringNode::new(t.clone()));
                    self.advance();
                    node
                }
                Some(t) if t.kind == TokenType::Identifier => {
                    let node = Node::String(StringNode::new(Token::new(
                        TokenType::String,
                        t.value.clone(),
                        t.position_start.clone(),
                        Some(t.position_end.clone()),
                    )));
                    self.advance();
                    node
                }
                Some(t) => {
                    return result.failure(
                        InvalidSyntaxError::new(
                            t.position_start.clone(),
                            t.position_end.clone(),
                            "Expected string or identifier as key",
                        )
                        .base,
                    );
                }
                None => {
                    return result.failure(
                        InvalidSyntaxError::new(
                            Self::dummy_pos(),
                            Self::dummy_pos(),
                            "Expected key",
                        )
                        .base,
                    );
                }
            };

            // Skip newlines before ':'
            while let Some(t) = self.current_token() {
                if t.kind == TokenType::Newline {
                    self.advance();
                } else {
                    break;
                }
            }

            // Expect ':'
            match self.current_token() {
                Some(t) if t.kind == TokenType::Colon => {
                    self.advance();
                }
                Some(t) => {
                    return result.failure(
                        InvalidSyntaxError::new(
                            t.position_start.clone(),
                            t.position_end.clone(),
                            "Expected ':'",
                        )
                        .base,
                    );
                }
                None => {
                    return result.failure(
                        InvalidSyntaxError::new(
                            Self::dummy_pos(),
                            Self::dummy_pos(),
                            "Expected ':'",
                        )
                        .base,
                    );
                }
            }

            // Skip newlines after ':'
            while let Some(t) = self.current_token() {
                if t.kind == TokenType::Newline {
                    self.advance();
                } else {
                    break;
                }
            }

            // Parse value
            let value_node = result.register(&self.expr());
            if result.error.is_some() {
                return result;
            }

            let pair_pos_start = key_node.position_start().clone();
            let pair_pos_end = value_node
                .as_ref()
                .map(|n| n.position_end().clone())
                .unwrap_or(pair_pos_start.clone());

            if let Some(val_node) = value_node {
                pairs.push(MapPair {
                    key_node: Box::new(key_node),
                    value_node: Box::new(val_node),
                    position_start: pair_pos_start,
                    position_end: pair_pos_end,
                });
            }

            // Skip newlines before comma or closing brace
            while let Some(t) = self.current_token() {
                if t.kind == TokenType::Newline {
                    self.advance();
                } else {
                    break;
                }
            }

            // Check for comma or closing brace
            match self.current_token() {
                Some(t) if t.kind == TokenType::Comma => {
                    self.advance();
                    // Skip newlines after comma
                    while let Some(tok) = self.current_token() {
                        if tok.kind == TokenType::Newline {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    // Check if next token is '}', if so, trailing comma is allowed
                    if let Some(next) = self.current_token() {
                        if next.kind == TokenType::RBrace {
                            self.advance();
                            break;
                        }
                    }
                    continue;
                }
                Some(t) if t.kind == TokenType::RBrace => {
                    self.advance();
                    break;
                }
                Some(t) => {
                    return result.failure(
                        InvalidSyntaxError::new(
                            t.position_start.clone(),
                            t.position_end.clone(),
                            "Expected ',' or '}'",
                        )
                        .base,
                    );
                }
                None => {
                    return result.failure(
                        InvalidSyntaxError::new(
                            Self::dummy_pos(),
                            Self::dummy_pos(),
                            "Expected ',' or '}'",
                        )
                        .base,
                    );
                }
            }
        }

        let pos_end = self
            .current_token()
            .map(|t| t.position_end.clone())
            .unwrap_or_else(|| pos_start.clone());

        result.success(Node::Map(MapNode {
            pairs,
            position_start: pos_start,
            position_end: pos_end,
        }))
    }

    fn ternary_expr(&mut self) -> ParseResult {
        let mut result = ParseResult::new();

        // Parse the condition
        let condition = result.register(&self.comp_expr());
        if result.error.is_some() {
            return result;
        }

        // Check for ternary operator
        if let Some(tok) = self.current_token() {
            if tok.kind == TokenType::Question {
                let question_pos = tok.position_start.clone();
                self.advance();

                // Parse true expression
                let true_expr = result.register(&self.expr());
                if result.error.is_some() {
                    return result;
                }

                // Check for colon
                match self.current_token() {
                    Some(t) if t.kind == TokenType::Colon => {
                        self.advance();
                    }
                    Some(t) => {
                        return result.failure(
                            InvalidSyntaxError::new(
                                t.position_start.clone(),
                                t.position_end.clone(),
                                "Expected ':' in ternary expression",
                            )
                            .base,
                        );
                    }
                    None => {
                        return result.failure(
                            InvalidSyntaxError::new(
                                question_pos,
                                Self::dummy_pos(),
                                "Expected ':' in ternary expression",
                            )
                            .base,
                        );
                    }
                }

                // Parse false expression
                let false_expr = result.register(&self.ternary_expr());
                if result.error.is_some() {
                    return result;
                }

                if let (Some(cond), Some(true_val), Some(false_val)) =
                    (condition, true_expr, false_expr)
                {
                    let pos_start = cond.position_start().clone();
                    let pos_end = false_val.position_end().clone();

                    // Clone cond here to avoid moving it
                    return result.success(Node::Ternary(Box::new(TernaryNode {
                        condition: Box::new(cond.clone()),
                        true_expression: Box::new(true_val),
                        false_expression: Box::new(false_val),
                        position_start: pos_start,
                        position_end: pos_end,
                    })));
                }

                // If we get here, something went wrong with the ternary parsing
                return result.failure(
                    InvalidSyntaxError::new(
                        Self::dummy_pos(),
                        Self::dummy_pos(),
                        "Invalid ternary expression",
                    )
                    .base,
                );
            }
        }

        // No ternary operator, return the condition (clone it to avoid moving)
        if let Some(cond) = condition {
            result.success(cond)
        } else {
            result
        }
    }

    fn comp_expr(&mut self) -> ParseResult {
        let mut result = ParseResult::new();

        let left = result.register(&self.arith_expr());
        if result.error.is_some() {
            return result;
        }
        let mut left = left.unwrap();

        while let Some(tok) = self.current_token().cloned() {
            let op_type = tok.kind.clone();

            // Check for comparison operators AND logical operators
            let is_operator = matches!(
                op_type,
                TokenType::Ee
                    | TokenType::Ne
                    | TokenType::Lt
                    | TokenType::Gt
                    | TokenType::Lte
                    | TokenType::Gte
            );

            // Check for logical operators (&& and || are keywords)
            let is_logical = tok.matches(TokenType::Keyword, Some("&&"))
                || tok.matches(TokenType::Keyword, Some("||"));

            if !is_operator && !is_logical {
                break;
            }

            let op = tok.clone();
            self.advance();

            let right = result.register(&self.arith_expr());
            if result.error.is_some() {
                return result;
            }
            let right = right.unwrap();

            left = Node::bin_op(left, op, right);
        }

        result.success(left)
    }

    fn arith_expr(&mut self) -> ParseResult {
        let mut result = ParseResult::new();

        let left = result.register(&self.term());
        if result.error.is_some() {
            return result;
        }
        let mut left = left.unwrap();

        while let Some(tok) = self.current_token().cloned() {
            let op_type = tok.kind.clone();
            if op_type != TokenType::Plus && op_type != TokenType::Minus {
                break;
            }

            let op = tok.clone();
            self.advance();

            let right = result.register(&self.term());
            if result.error.is_some() {
                return result;
            }
            let right = right.unwrap();

            left = Node::bin_op(left, op, right);
        }

        result.success(left)
    }

    fn term(&mut self) -> ParseResult {
        let mut result = ParseResult::new();

        let left = result.register(&self.factor());
        if result.error.is_some() {
            return result;
        }
        let mut left = left.unwrap();

        while let Some(tok) = self.current_token().cloned() {
            let op_type = tok.kind.clone();

            if op_type != TokenType::Mul && op_type != TokenType::Div {
                break;
            }

            let op = tok.clone();
            self.advance();

            let right = result.register(&self.factor());
            if result.error.is_some() {
                return result;
            }
            let right = right.unwrap();

            left = Node::bin_op(left, op, right);
        }

        result.success(left)
    }

    fn factor(&mut self) -> ParseResult {
        let mut result = ParseResult::new();

        if let Some(tok) = self.current_token().cloned() {
            // Handle unary plus, minus, and NOT (!)
            if tok.kind == TokenType::Plus || tok.kind == TokenType::Minus {
                let op = tok.clone();
                self.advance();

                let node = result.register(&self.factor());
                if result.error.is_some() {
                    return result;
                }
                let node = node.unwrap();

                let pos_end = self
                    .current_token()
                    .map(|t| t.position_end.clone())
                    .unwrap_or_else(|| tok.position_end.clone());

                return result.success(Node::UnaryOp(Box::new(UnaryOpNode {
                    operator_token: op,
                    node: Box::new(node),
                    position_start: tok.position_start.clone(),
                    position_end: pos_end,
                })));
            }

            // Handle logical NOT (!)
            if tok.matches(TokenType::Keyword, Some("!")) {
                let op = tok.clone();
                self.advance();

                let node = result.register(&self.factor());
                if result.error.is_some() {
                    return result;
                }
                let node = node.unwrap();

                let pos_end = self
                    .current_token()
                    .map(|t| t.position_end.clone())
                    .unwrap_or_else(|| tok.position_end.clone());

                return result.success(Node::UnaryOp(Box::new(UnaryOpNode {
                    operator_token: op,
                    node: Box::new(node),
                    position_start: tok.position_start.clone(),
                    position_end: pos_end,
                })));
            }
        }

        self.power()
    }

    fn power(&mut self) -> ParseResult {
        let mut result = ParseResult::new();

        let left = result.register(&self.call());
        if result.error.is_some() {
            return result;
        }
        let mut left = left.unwrap();

        while let Some(tok) = self.current_token().cloned() {
            if tok.kind != TokenType::Pow {
                break;
            }

            let op = tok.clone();
            self.advance();

            let right = result.register(&self.factor());
            if result.error.is_some() {
                return result;
            }
            let right = right.unwrap();

            left = Node::bin_op(left, op, right);
        }

        result.success(left)
    }

    fn call(&mut self) -> ParseResult {
        let mut result = ParseResult::new();

        let atom = result.register(&self.atom());
        if result.error.is_some() {
            return result;
        }
        let mut node = atom.unwrap();

        // Handle method calls and function calls
        while let Some(tok) = self.current_token().cloned() {
            if tok.kind == TokenType::Dot {
                self.advance(); // consume '.'

                let method_name = match self.current_token() {
                    Some(t) if t.kind == TokenType::Identifier => t.clone(),
                    Some(t) => {
                        return result.failure(
                            InvalidSyntaxError::new(
                                t.position_start.clone(),
                                t.position_end.clone(),
                                "Expected method name",
                            )
                            .base,
                        );
                    }
                    None => {
                        return result.failure(
                            InvalidSyntaxError::new(
                                Self::dummy_pos(),
                                Self::dummy_pos(),
                                "Expected method name",
                            )
                            .base,
                        );
                    }
                };
                self.advance();

                // Parse arguments
                let mut arg_nodes = Vec::new();
                if let Some(tok2) = self.current_token() {
                    if tok2.kind == TokenType::LParen {
                        self.advance(); // consume '('

                        // Parse arguments
                        if let Some(rparen) = self.current_token() {
                            if rparen.kind != TokenType::RParen {
                                let arg = result.register(&self.expr());
                                if result.error.is_some() {
                                    return result;
                                }
                                if let Some(arg_node) = arg {
                                    arg_nodes.push(Box::new(arg_node));
                                }

                                while let Some(comma) = self.current_token().cloned() {
                                    if comma.kind != TokenType::Comma {
                                        break;
                                    }
                                    self.advance();

                                    let next_arg = result.register(&self.expr());
                                    if result.error.is_some() {
                                        return result;
                                    }
                                    if let Some(arg_node) = next_arg {
                                        arg_nodes.push(Box::new(arg_node));
                                    }
                                }
                            }
                        }

                        match self.current_token() {
                            Some(t) if t.kind == TokenType::RParen => {
                                self.advance();
                            }
                            Some(t) => {
                                return result.failure(
                                    InvalidSyntaxError::new(
                                        t.position_start.clone(),
                                        t.position_end.clone(),
                                        "Expected ')'",
                                    )
                                    .base,
                                );
                            }
                            None => {
                                return result.failure(
                                    InvalidSyntaxError::new(
                                        Self::dummy_pos(),
                                        Self::dummy_pos(),
                                        "Expected ')'",
                                    )
                                    .base,
                                );
                            }
                        }
                    }
                }

                // Extract positions with clones BEFORE moving node
                let node_pos_start = node.position_start().clone();
                let node_pos_end = node.position_end().clone();
                let method_pos_end = method_name.position_end.clone();
                let method_pos_start = method_name.position_start.clone();

                // Create a method access node wrapped in a call node
                let method_token = Token::new(
                    TokenType::Identifier,
                    method_name.value.clone(),
                    method_pos_start,
                    Some(method_pos_end.clone()),
                );

                // Build the new node
                let new_node = Node::Call(Box::new(CallNode {
                    node_to_call: Box::new(Node::MethodAccess(MethodAccessNode {
                        object: Box::new(node), // node is moved here
                        method_name: method_token,
                        position_start: node_pos_start.clone(),
                        position_end: method_pos_end.clone(),
                    })),
                    argument_nodes: arg_nodes,
                    position_start: node_pos_start,
                    position_end: self
                        .current_token()
                        .map(|t| t.position_end.clone())
                        .unwrap_or(method_pos_end),
                }));
                node = new_node;
            } else if tok.kind == TokenType::LParen {
                // Regular function call
                let pos_start = tok.position_start.clone();
                self.advance(); // consume '('

                let mut arg_nodes = Vec::new();

                // Parse arguments
                if let Some(rparen) = self.current_token() {
                    if rparen.kind != TokenType::RParen {
                        let arg = result.register(&self.expr());
                        if result.error.is_some() {
                            return result;
                        }
                        if let Some(arg_node) = arg {
                            arg_nodes.push(Box::new(arg_node));
                        }

                        while let Some(comma) = self.current_token().cloned() {
                            if comma.kind != TokenType::Comma {
                                break;
                            }
                            self.advance();

                            let next_arg = result.register(&self.expr());
                            if result.error.is_some() {
                                return result;
                            }
                            if let Some(arg_node) = next_arg {
                                arg_nodes.push(Box::new(arg_node));
                            }
                        }
                    }
                }

                match self.current_token() {
                    Some(t) if t.kind == TokenType::RParen => {
                        let pos_end = t.position_end.clone();
                        self.advance();

                        let new_node = Node::Call(Box::new(CallNode {
                            node_to_call: Box::new(node),
                            argument_nodes: arg_nodes,
                            position_start: pos_start,
                            position_end: pos_end,
                        }));
                        node = new_node;
                    }
                    Some(t) => {
                        return result.failure(
                            InvalidSyntaxError::new(
                                t.position_start.clone(),
                                t.position_end.clone(),
                                "Expected ',' or ')'",
                            )
                            .base,
                        );
                    }
                    None => {
                        return result.failure(
                            InvalidSyntaxError::new(
                                Self::dummy_pos(),
                                Self::dummy_pos(),
                                "Expected ')'",
                            )
                            .base,
                        );
                    }
                }
            } else {
                break;
            }
        }

        result.success(node)
    }

    fn atom(&mut self) -> ParseResult {
        let mut result = ParseResult::new();

        match self.current_token() {
            Some(tok) => match tok.kind {
                TokenType::Int | TokenType::Float => {
                    let node = Node::Number(NumberNode::new(tok.clone()));
                    self.advance();
                    return result.success(node);
                }
                TokenType::String => {
                    let node = Node::String(StringNode::new(tok.clone()));
                    self.advance();
                    return result.success(node);
                }
                TokenType::InterpolatedString => {
                    let node = Node::InterpolatedString(InterpolatedStringNode::new(tok.clone()));
                    self.advance();
                    return result.success(node);
                }
                TokenType::Identifier => {
                    let node = Node::VarAccess(VarAccessNode {
                        variable_name_token: tok.clone(),
                        position_start: tok.position_start.clone(),
                        position_end: tok.position_end.clone(),
                    });
                    self.advance();

                    // Check for dot access (e.g., user.name)
                    // Clone the current token before checking to avoid borrowing issues
                    if let Some(dot) = self.current_token().cloned() {
                        if dot.kind == TokenType::Dot {
                            self.advance(); // consume '.'
                            if let Some(prop) = self.current_token().cloned() {
                                if prop.kind == TokenType::Identifier {
                                    let prop_name = prop.value.clone().unwrap();
                                    self.advance();

                                    // Create a map access: user["name"]
                                    let key_node = Node::String(StringNode::new(Token::new(
                                        TokenType::String,
                                        Some(prop_name),
                                        prop.position_start.clone(),
                                        Some(prop.position_end.clone()),
                                    )));

                                    let index_token = Token::new(
                                        TokenType::Index,
                                        None,
                                        dot.position_start.clone(),
                                        Some(prop.position_end.clone()),
                                    );

                                    // Clone node before moving it into the Box
                                    let node_clone = node.clone();
                                    let access_node =
                                        Node::BinaryOperator(Box::new(BinaryOperatorNode {
                                            left_node: Box::new(node_clone),
                                            operator_token: index_token,
                                            right_node: Box::new(key_node),
                                            position_start: node.position_start().clone(),
                                            position_end: prop.position_end.clone(),
                                        }));

                                    return self.parse_indexing(access_node, result);
                                }
                            }
                        }
                    }

                    // Check for indexing after identifier
                    return self.parse_indexing(node, result);
                }
                TokenType::LParen => {
                    let pos_start = tok.position_start.clone();
                    self.advance();

                    let expr = result.register(&self.expr());
                    if result.error.is_some() {
                        return result;
                    }

                    match self.current_token() {
                        Some(t) if t.kind == TokenType::RParen => {
                            self.advance();
                            if let Some(node) = expr {
                                return result.success(node);
                            }
                        }
                        Some(t) => {
                            return result.failure(
                                InvalidSyntaxError::new(
                                    t.position_start.clone(),
                                    t.position_end.clone(),
                                    "Expected ')'",
                                )
                                .base,
                            );
                        }
                        None => {
                            return result.failure(
                                InvalidSyntaxError::new(
                                    pos_start,
                                    Self::dummy_pos(),
                                    "Expected ')'",
                                )
                                .base,
                            );
                        }
                    }
                }
                TokenType::LBrace => {
                    return self.map_expr();
                }
                TokenType::LSquare => {
                    return self.list_expr();
                }
                _ if tok.matches(TokenType::Keyword, Some("when")) => {
                    return self.if_expr();
                }
                _ if tok.matches(TokenType::Keyword, Some("for")) => {
                    return self.for_expr();
                }
                _ if tok.matches(TokenType::Keyword, Some("while")) => {
                    return self.while_expr();
                }
                _ if tok.matches(TokenType::Keyword, Some("method")) => {
                    return self.func_def();
                }
                _ if tok.matches(TokenType::Keyword, Some("match")) => {
                    return self.match_expr();
                }
                _ => {}
            },
            None => {}
        }

        result.failure(
            InvalidSyntaxError::new(Self::dummy_pos(), Self::dummy_pos(), "Expected expression")
                .base,
        )
    }

    // Method to handle indexing (e.g., fruits[0], matrix[1][0])
    fn parse_indexing(&mut self, mut node: Node, mut result: ParseResult) -> ParseResult {
        while let Some(tok) = self.current_token().cloned() {
            if tok.kind == TokenType::LSquare {
                self.advance(); // consume '['

                let index_expr = result.register(&self.expr());
                if result.error.is_some() {
                    return result;
                }

                // Check for closing bracket
                match self.current_token() {
                    Some(t) if t.kind == TokenType::RSquare => {
                        let close_pos = t.position_end.clone();
                        self.advance(); // consume ']'

                        if let Some(index_node) = index_expr {
                            // Store the position start before moving node
                            let node_pos_start = node.position_start().clone();
                            let node_pos_end = node.position_end().clone();

                            // Create a binary operation node for indexing
                            let index_token = Token::new(
                                TokenType::Index,
                                None,
                                tok.position_start.clone(),
                                Some(close_pos.clone()),
                            );

                            // Rebuild node using the stored positions
                            node = Node::BinaryOperator(Box::new(BinaryOperatorNode {
                                left_node: Box::new(node),
                                operator_token: index_token,
                                right_node: Box::new(index_node),
                                position_start: node_pos_start,
                                position_end: close_pos,
                            }));
                        }
                    }
                    Some(t) => {
                        return result.failure(
                            InvalidSyntaxError::new(
                                t.position_start.clone(),
                                t.position_end.clone(),
                                "Expected ']'",
                            )
                            .base,
                        );
                    }
                    None => {
                        return result.failure(
                            InvalidSyntaxError::new(
                                Self::dummy_pos(),
                                Self::dummy_pos(),
                                "Expected ']'",
                            )
                            .base,
                        );
                    }
                }
            } else {
                break;
            }
        }

        result.success(node)
    }

    // New method to handle indexing (e.g., fruits[0], matrix[1][0])

    fn list_expr(&mut self) -> ParseResult {
        let mut result = ParseResult::new();
        let mut elements = Vec::new();

        let pos_start = match self.current_token() {
            Some(t) if t.kind == TokenType::LSquare => t.position_start.clone(),
            Some(t) => {
                return result.failure(
                    InvalidSyntaxError::new(
                        t.position_start.clone(),
                        t.position_end.clone(),
                        "Expected '['",
                    )
                    .base,
                );
            }
            None => {
                return result.failure(
                    InvalidSyntaxError::new(Self::dummy_pos(), Self::dummy_pos(), "Expected '['")
                        .base,
                );
            }
        };
        self.advance();

        // Empty list
        if let Some(t) = self.current_token().cloned() {
            if t.kind == TokenType::RSquare {
                self.advance();

                return result.success(Node::List(ListNode {
                    element_nodes: Vec::new(),
                    position_start: pos_start,
                    position_end: t.position_end.clone(),
                }));
            }
        }

        // Parse elements
        let elem = result.register(&self.expr());
        if result.error.is_some() {
            return result;
        }
        if let Some(e) = elem {
            elements.push(Box::new(e));
        }

        while let Some(t) = self.current_token() {
            if t.kind != TokenType::Comma {
                break;
            }
            self.advance();

            let next_elem = result.register(&self.expr());
            if result.error.is_some() {
                return result;
            }
            if let Some(e) = next_elem {
                elements.push(Box::new(e));
            }
        }

        match self.current_token() {
            Some(t) if t.kind == TokenType::RSquare => {
                let pos_end = t.position_end.clone();
                self.advance();
                result.success(Node::List(ListNode {
                    element_nodes: elements,
                    position_start: pos_start,
                    position_end: pos_end,
                }))
            }
            Some(t) => result.failure(
                InvalidSyntaxError::new(
                    t.position_start.clone(),
                    t.position_end.clone(),
                    "Expected ',' or ']'",
                )
                .base,
            ),
            None => result.failure(
                InvalidSyntaxError::new(Self::dummy_pos(), Self::dummy_pos(), "Expected ']'").base,
            ),
        }
    }

    fn if_expr(&mut self) -> ParseResult {
        let mut result = ParseResult::new();
        let mut cases = Vec::new();

        // Expect "when" keyword
        let pos_start = match self.current_token() {
            Some(t) if t.matches(TokenType::Keyword, Some("when")) => t.position_start.clone(),
            Some(t) => {
                return result.failure(
                    InvalidSyntaxError::new(
                        t.position_start.clone(),
                        t.position_end.clone(),
                        "Expected 'when'",
                    )
                    .base,
                );
            }
            None => {
                return result.failure(
                    InvalidSyntaxError::new(
                        Self::dummy_pos(),
                        Self::dummy_pos(),
                        "Expected 'when'",
                    )
                    .base,
                );
            }
        };
        self.advance();

        // Parse first condition and body
        let mut condition = result.register(&self.expr());
        if result.error.is_some() {
            return result;
        }

        let mut body = if let Some(t) = self.current_token() {
            if t.kind == TokenType::LBrace {
                result.register(&self.block())
            } else {
                result.register(&self.statement())
            }
        } else {
            result.register(&self.statement())
        };

        if result.error.is_some() {
            return result;
        }

        if let (Some(cond), Some(body_node)) = (condition.take(), body.take()) {
            cases.push((Box::new(cond), Box::new(body_node)));
        }

        // Parse any "or when" chains
        while let Some(tok) = self.current_token() {
            if tok.matches(TokenType::Keyword, Some("or when")) {
                self.advance(); // consume "or when"

                condition = result.register(&self.expr());
                if result.error.is_some() {
                    return result;
                }

                body = if let Some(t) = self.current_token() {
                    if t.kind == TokenType::LBrace {
                        result.register(&self.block())
                    } else {
                        result.register(&self.statement())
                    }
                } else {
                    result.register(&self.statement())
                };

                if result.error.is_some() {
                    return result;
                }

                if let (Some(cond), Some(body_node)) = (condition.take(), body.take()) {
                    cases.push((Box::new(cond), Box::new(body_node)));
                }
            } else {
                break;
            }
        }

        // Parse optional "otherwise" clause
        let mut else_case = None;
        if let Some(t) = self.current_token() {
            if t.matches(TokenType::Keyword, Some("otherwise")) {
                self.advance();

                let else_body = if let Some(tt) = self.current_token() {
                    if tt.kind == TokenType::LBrace {
                        result.register(&self.block())
                    } else {
                        result.register(&self.statement())
                    }
                } else {
                    result.register(&self.statement())
                };

                if result.error.is_some() {
                    return result;
                }

                if let Some(body_node) = else_body {
                    let null_node = Node::Number(NumberNode::new(Token::new(
                        TokenType::Int,
                        Some("0".to_string()),
                        pos_start.clone(),
                        None,
                    )));
                    else_case = Some((Box::new(body_node), Box::new(null_node)));
                }
            }
        }

        let pos_end = self
            .current_token()
            .map(|t| t.position_end.clone())
            .unwrap_or_else(|| pos_start.clone());

        result.success(Node::If(Box::new(IfNode {
            cases,
            else_case,
            position_start: pos_start,
            position_end: pos_end,
        })))
    }

    fn for_expr(&mut self) -> ParseResult {
        let mut result = ParseResult::new();

        let pos_start = match self.current_token() {
            Some(t) if t.matches(TokenType::Keyword, Some("for")) => t.position_start.clone(),
            Some(t) => {
                return result.failure(
                    InvalidSyntaxError::new(
                        t.position_start.clone(),
                        t.position_end.clone(),
                        "Expected 'for'",
                    )
                    .base,
                );
            }
            None => {
                return result.failure(
                    InvalidSyntaxError::new(Self::dummy_pos(), Self::dummy_pos(), "Expected 'for'")
                        .base,
                );
            }
        };
        self.advance();

        let var_name = match self.current_token() {
            Some(t) if t.kind == TokenType::Identifier => t.clone(),
            Some(t) => {
                return result.failure(
                    InvalidSyntaxError::new(
                        t.position_start.clone(),
                        t.position_end.clone(),
                        "Expected identifier",
                    )
                    .base,
                );
            }
            None => {
                return result.failure(
                    InvalidSyntaxError::new(
                        Self::dummy_pos(),
                        Self::dummy_pos(),
                        "Expected identifier",
                    )
                    .base,
                );
            }
        };
        self.advance();

        match self.current_token() {
            Some(t) if t.kind == TokenType::Eq => {
                self.advance();
            }
            Some(t) => {
                return result.failure(
                    InvalidSyntaxError::new(
                        t.position_start.clone(),
                        t.position_end.clone(),
                        "Expected '='",
                    )
                    .base,
                );
            }
            None => {
                return result.failure(
                    InvalidSyntaxError::new(Self::dummy_pos(), Self::dummy_pos(), "Expected '='")
                        .base,
                );
            }
        }

        let start_value = result.register(&self.expr());
        if result.error.is_some() {
            return result;
        }

        match self.current_token() {
            Some(t) if t.matches(TokenType::Keyword, Some("to")) => {
                self.advance();
            }
            Some(t) => {
                return result.failure(
                    InvalidSyntaxError::new(
                        t.position_start.clone(),
                        t.position_end.clone(),
                        "Expected 'to'",
                    )
                    .base,
                );
            }
            None => {
                return result.failure(
                    InvalidSyntaxError::new(Self::dummy_pos(), Self::dummy_pos(), "Expected 'to'")
                        .base,
                );
            }
        }

        let end_value = result.register(&self.expr());
        if result.error.is_some() {
            return result;
        }

        let mut step_value = None;
        if let Some(t) = self.current_token() {
            if t.matches(TokenType::Keyword, Some("step")) {
                self.advance();
                let step = result.register(&self.expr());
                if result.error.is_some() {
                    return result;
                }
                step_value = step.map(|s| Box::new(s));
            }
        }

        let body = if let Some(t) = self.current_token() {
            if t.kind == TokenType::LBrace {
                result.register(&self.block())
            } else {
                result.register(&self.statement())
            }
        } else {
            result.register(&self.statement())
        };

        if result.error.is_some() {
            return result;
        }

        if let (Some(start), Some(end), Some(body_node)) = (start_value, end_value, body) {
            let pos_end = body_node.position_end().clone();
            result.success(Node::For(Box::new(ForNode {
                variable_name_token: var_name,
                start_value_node: Box::new(start),
                end_value_node: Box::new(end),
                step_value_node: step_value,
                body_node: Box::new(body_node),
                should_return_null: false,
                position_start: pos_start,
                position_end: pos_end,
            })))
        } else {
            result
        }
    }

    fn while_expr(&mut self) -> ParseResult {
        let mut result = ParseResult::new();

        let pos_start = match self.current_token() {
            Some(t) if t.matches(TokenType::Keyword, Some("while")) => t.position_start.clone(),
            Some(t) => {
                return result.failure(
                    InvalidSyntaxError::new(
                        t.position_start.clone(),
                        t.position_end.clone(),
                        "Expected 'while'",
                    )
                    .base,
                );
            }
            None => {
                return result.failure(
                    InvalidSyntaxError::new(
                        Self::dummy_pos(),
                        Self::dummy_pos(),
                        "Expected 'while'",
                    )
                    .base,
                );
            }
        };
        self.advance();

        let condition = result.register(&self.expr());
        if result.error.is_some() {
            return result;
        }

        let body = if let Some(t) = self.current_token() {
            if t.kind == TokenType::LBrace {
                result.register(&self.block())
            } else {
                result.register(&self.statement())
            }
        } else {
            result.register(&self.statement())
        };

        if result.error.is_some() {
            return result;
        }

        if let (Some(cond), Some(body_node)) = (condition, body) {
            let pos_end = body_node.position_end().clone();
            result.success(Node::While(Box::new(WhileNode {
                condition_node: Box::new(cond),
                body_node: Box::new(body_node),
                should_return_null: false,
                position_start: pos_start,
                position_end: pos_end,
            })))
        } else {
            result
        }
    }

    fn func_def(&mut self) -> ParseResult {
        let mut result = ParseResult::new();

        let pos_start = match self.current_token() {
            Some(t) if t.matches(TokenType::Keyword, Some("method")) => t.position_start.clone(),
            Some(t) => {
                return result.failure(
                    InvalidSyntaxError::new(
                        t.position_start.clone(),
                        t.position_end.clone(),
                        "Expected 'method'",
                    )
                    .base,
                );
            }
            None => {
                return result.failure(
                    InvalidSyntaxError::new(
                        Self::dummy_pos(),
                        Self::dummy_pos(),
                        "Expected 'method'",
                    )
                    .base,
                );
            }
        };
        self.advance();

        // Optional function name
        let var_name_tok = if let Some(t) = self.current_token() {
            if t.kind == TokenType::Identifier {
                let name = t.clone();
                self.advance();
                Some(name)
            } else {
                None
            }
        } else {
            None
        };

        // Parse parameter list
        match self.current_token() {
            Some(t) if t.kind == TokenType::LParen => {
                self.advance();
            }
            Some(t) => {
                return result.failure(
                    InvalidSyntaxError::new(
                        t.position_start.clone(),
                        t.position_end.clone(),
                        "Expected '('",
                    )
                    .base,
                );
            }
            None => {
                return result.failure(
                    InvalidSyntaxError::new(Self::dummy_pos(), Self::dummy_pos(), "Expected '('")
                        .base,
                );
            }
        }

        let mut arg_name_toks = Vec::new();

        // Parse parameters
        if let Some(t) = self.current_token() {
            if t.kind == TokenType::Identifier {
                arg_name_toks.push(t.clone());
                self.advance();

                while let Some(comma) = self.current_token() {
                    if comma.kind != TokenType::Comma {
                        break;
                    }
                    self.advance();

                    match self.current_token() {
                        Some(next) if next.kind == TokenType::Identifier => {
                            arg_name_toks.push(next.clone());
                            self.advance();
                        }
                        Some(next) => {
                            return result.failure(
                                InvalidSyntaxError::new(
                                    next.position_start.clone(),
                                    next.position_end.clone(),
                                    "Expected identifier",
                                )
                                .base,
                            );
                        }
                        None => {
                            return result.failure(
                                InvalidSyntaxError::new(
                                    Self::dummy_pos(),
                                    Self::dummy_pos(),
                                    "Expected identifier",
                                )
                                .base,
                            );
                        }
                    }
                }
            }
        }

        match self.current_token() {
            Some(t) if t.kind == TokenType::RParen => {
                self.advance();
            }
            Some(t) => {
                return result.failure(
                    InvalidSyntaxError::new(
                        t.position_start.clone(),
                        t.position_end.clone(),
                        "Expected ',' or ')'",
                    )
                    .base,
                );
            }
            None => {
                return result.failure(
                    InvalidSyntaxError::new(Self::dummy_pos(), Self::dummy_pos(), "Expected ')'")
                        .base,
                );
            }
        }

        // Parse body - check for arrow function first
        match self.current_token() {
            Some(t) if t.kind == TokenType::Arrow => {
                // Arrow function (single expression)
                self.advance();

                let body = result.register(&self.expr());
                if result.error.is_some() {
                    return result;
                }

                if let Some(body_node) = body {
                    let pos_end = body_node.position_end().clone();
                    result.success(Node::FuncDef(Box::new(FuncDefNode {
                        variable_name_token: var_name_tok,
                        arg_name_toks,
                        body_node: Box::new(body_node),
                        should_auto_return: true,
                        position_start: pos_start,
                        position_end: pos_end,
                    })))
                } else {
                    result.failure(
                        InvalidSyntaxError::new(
                            Self::dummy_pos(),
                            Self::dummy_pos(),
                            "Expected expression after '->'",
                        )
                        .base,
                    )
                }
            }
            Some(t) if t.kind == TokenType::LBrace => {
                let body = result.register(&self.block());
                if result.error.is_some() {
                    return result;
                }

                if let Some(body_node) = body {
                    let pos_end = body_node.position_end().clone();
                    result.success(Node::FuncDef(Box::new(FuncDefNode {
                        variable_name_token: var_name_tok,
                        arg_name_toks,
                        body_node: Box::new(body_node),
                        should_auto_return: false,
                        position_start: pos_start,
                        position_end: pos_end,
                    })))
                } else {
                    result
                }
            }
            Some(t) => result.failure(
                InvalidSyntaxError::new(
                    t.position_start.clone(),
                    t.position_end.clone(),
                    "Expected '->' or '{'",
                )
                .base,
            ),
            None => result.failure(
                InvalidSyntaxError::new(
                    Self::dummy_pos(),
                    Self::dummy_pos(),
                    "Expected '->' or '{'",
                )
                .base,
            ),
        }
    }
}
