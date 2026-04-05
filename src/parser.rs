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

    fn advance(&mut self) {
        self.token_index += 1;
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
        // Check for variable assignment
        if let Some(tok) = self.current_token() {
            if tok.matches(TokenType::Keyword, Some("spawn")) {
                let mut result = ParseResult::new();
                let pos_start = tok.position_start.clone();
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
                            InvalidSyntaxError::new(
                                Self::dummy_pos(),
                                Self::dummy_pos(),
                                "Expected '='",
                            )
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
            }
        }

        // Parse comparison
        self.comp_expr()
    }

    fn comp_expr(&mut self) -> ParseResult {
        self.arith_expr()
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

        if let Some(tok) = self.current_token() {
            if tok.kind == TokenType::LParen {
                let pos_start = tok.position_start.clone();
                self.advance();

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

                        while let Some(comma) = self.current_token() {
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

                        if let Some(node) = atom {
                            return result.success(Node::Call(Box::new(CallNode {
                                node_to_call: Box::new(node),
                                argument_nodes: arg_nodes,
                                position_start: pos_start,
                                position_end: pos_end,
                            })));
                        }
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
            }
        }

        if let Some(node) = atom {
            result.success(node)
        } else {
            result
        }
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
                TokenType::Identifier => {
                    let node = Node::VarAccess(VarAccessNode {
                        variable_name_token: tok.clone(),
                        position_start: tok.position_start.clone(),
                        position_end: tok.position_end.clone(),
                    });
                    self.advance();
                    return result.success(node);
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
                _ => {}
            },
            None => {}
        }

        result.failure(
            InvalidSyntaxError::new(Self::dummy_pos(), Self::dummy_pos(), "Expected expression")
                .base,
        )
    }

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
            cases.push((Box::new(cond), Box::new(body_node)));
        }

        // Parse else clause
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

                if let Some(body) = else_body {
                    let null_node = Node::Number(NumberNode::new(Token::new(
                        TokenType::Int,
                        Some("0".to_string()),
                        pos_start.clone(),
                        None,
                    )));
                    else_case = Some((Box::new(body), Box::new(null_node)));
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

        // Parse body
        match self.current_token() {
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
                    "Expected '{'",
                )
                .base,
            ),
            None => result.failure(
                InvalidSyntaxError::new(Self::dummy_pos(), Self::dummy_pos(), "Expected '{'").base,
            ),
        }
    }
}
