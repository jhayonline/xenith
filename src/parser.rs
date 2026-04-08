//! # Syntax Parser Module
//!
//! Implements recursive descent parsing to transform token streams
//! into an Abstract Syntax Tree (AST). Uses {} block syntax.

use crate::error::InvalidSyntaxError;
use crate::nodes::*;
use crate::parse_result::ParseResult;
use crate::position::Position;
use crate::tokens::{Token, TokenType};
use crate::types::{FunctionType, Type};
use std::collections::HashMap;

/// Recursive descent parser for Xenith
#[derive(Debug)]
pub struct Parser {
    pub tokens: Vec<Token>,
    pub token_index: usize,
    pub struct_registry: HashMap<String, StructInfo>,
}

impl Parser {
    /// Creates a new parser from a token stream
    pub fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            token_index: 0,
            struct_registry: HashMap::new(),
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

    /// Parse a type annotation
    /// Syntax: int | float | string | bool | null | list<T> | map<K, V> | identifier | method(...) -> ...
    fn parse_type(&mut self) -> ParseResult {
        let mut result = ParseResult::new();
        let type_token = match self.current_token() {
            Some(tok) => tok.clone(),
            None => {
                return result.failure(
                    InvalidSyntaxError::new(Self::dummy_pos(), Self::dummy_pos(), "Expected type")
                        .base,
                );
            }
        };

        // Handle function type separately (returns ParseResult directly)
        if type_token.matches(TokenType::Keyword, Some("method")) {
            self.advance(); // consume 'method'

            // Expect '('
            match self.current_token() {
                Some(t) if t.kind == TokenType::LParen => {
                    self.advance();
                }
                Some(t) => {
                    return result.failure(
                        InvalidSyntaxError::new(
                            t.position_start.clone(),
                            t.position_end.clone(),
                            "Expected '(' after 'method'",
                        )
                        .base,
                    );
                }
                None => {
                    return result.failure(
                        InvalidSyntaxError::new(
                            Self::dummy_pos(),
                            Self::dummy_pos(),
                            "Expected '(' after 'method'",
                        )
                        .base,
                    );
                }
            }

            let mut param_types = Vec::new();

            // Parse parameter types (no names, just types)
            // Check if empty parameter list
            if let Some(t) = self.current_token() {
                if t.kind != TokenType::RParen {
                    // Parse first parameter type
                    let param_type = result.register_type(&self.parse_type());
                    if result.error.is_some() {
                        return result;
                    }
                    param_types.push(param_type);

                    // Parse additional parameters separated by commas
                    while let Some(comma) = self.current_token() {
                        if comma.kind == TokenType::Comma {
                            self.advance(); // consume ','

                            let next_param = result.register_type(&self.parse_type());
                            if result.error.is_some() {
                                return result;
                            }
                            param_types.push(next_param);
                        } else {
                            break;
                        }
                    }
                }
            }

            // Expect ')'
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

            // Expect '->'
            match self.current_token() {
                Some(t) if t.kind == TokenType::Arrow => {
                    self.advance();
                }
                Some(t) => {
                    return result.failure(
                        InvalidSyntaxError::new(
                            t.position_start.clone(),
                            t.position_end.clone(),
                            "Expected '->'",
                        )
                        .base,
                    );
                }
                None => {
                    return result.failure(
                        InvalidSyntaxError::new(
                            Self::dummy_pos(),
                            Self::dummy_pos(),
                            "Expected '->'",
                        )
                        .base,
                    );
                }
            }

            // Parse return type
            let return_type = result.register_type(&self.parse_type());
            if result.error.is_some() {
                return result;
            }

            return result.success_type(Type::Function(FunctionType {
                param_types,
                return_type: Box::new(return_type),
            }));
        }

        // Handle all other types (same as before)
        let parsed_type = match type_token.kind {
            TokenType::TypeInt => Type::Int,
            TokenType::TypeFloat => Type::Float,
            TokenType::TypeString => Type::String,
            TokenType::TypeBool => Type::Bool,
            TokenType::TypeNull => Type::Null,
            TokenType::TypeList => {
                // Handle list<T>
                self.advance(); // consume 'list'

                // Check for '<'
                match self.current_token() {
                    Some(tok) if tok.kind == TokenType::Lt => {
                        self.advance(); // consume '<'
                        let inner_type = result.register_type(&self.parse_type());
                        if result.error.is_some() {
                            return result;
                        }

                        // Expect '>'
                        match self.current_token() {
                            Some(tok) if tok.kind == TokenType::Gt => {
                                self.advance();
                            }
                            _ => {
                                return result.failure(
                                    InvalidSyntaxError::new(
                                        Self::dummy_pos(),
                                        Self::dummy_pos(),
                                        "Expected '>'",
                                    )
                                    .base,
                                );
                            }
                        }

                        Type::List(Box::new(inner_type))
                    }
                    _ => {
                        return result.failure(
                            InvalidSyntaxError::new(
                                type_token.position_start.clone(),
                                type_token.position_end.clone(),
                                "Expected '<' for list type",
                            )
                            .base,
                        );
                    }
                }
            }
            TokenType::TypeMap => {
                // Handle map<K, V>
                self.advance(); // consume 'map'

                match self.current_token() {
                    Some(tok) if tok.kind == TokenType::Lt => {
                        self.advance(); // consume '<'
                        let key_type = result.register_type(&self.parse_type());
                        if result.error.is_some() {
                            return result;
                        }

                        // Expect ','
                        match self.current_token() {
                            Some(tok) if tok.kind == TokenType::Comma => {
                                self.advance();
                            }
                            _ => {
                                return result.failure(
                                    InvalidSyntaxError::new(
                                        Self::dummy_pos(),
                                        Self::dummy_pos(),
                                        "Expected ','",
                                    )
                                    .base,
                                );
                            }
                        }

                        let value_type = result.register_type(&self.parse_type());
                        if result.error.is_some() {
                            return result;
                        }

                        // Expect '>'
                        match self.current_token() {
                            Some(tok) if tok.kind == TokenType::Gt => {
                                self.advance();
                            }
                            _ => {
                                return result.failure(
                                    InvalidSyntaxError::new(
                                        Self::dummy_pos(),
                                        Self::dummy_pos(),
                                        "Expected '>'",
                                    )
                                    .base,
                                );
                            }
                        }

                        Type::Map(Box::new(key_type), Box::new(value_type))
                    }
                    _ => {
                        return result.failure(
                            InvalidSyntaxError::new(
                                type_token.position_start.clone(),
                                type_token.position_end.clone(),
                                "Expected '<' for map type",
                            )
                            .base,
                        );
                    }
                }
            }
            TokenType::Identifier => {
                // Could be a struct name or type alias
                let name = type_token.value.clone().unwrap();
                self.advance();

                // Check for generic parameters
                if let Some(tok) = self.current_token() {
                    if tok.kind == TokenType::Lt {
                        return self.parse_generic_type(name);
                    }
                }

                Type::Struct(name, Vec::new()) // Placeholder, will resolve later
            }
            _ => {
                return result.failure(
                    InvalidSyntaxError::new(
                        type_token.position_start.clone(),
                        type_token.position_end.clone(),
                        &format!("Expected type, got {:?}", type_token.kind),
                    )
                    .base,
                );
            }
        };

        // Only advance if we didn't already advance inside the match
        if !matches!(
            type_token.kind,
            TokenType::TypeList
                | TokenType::TypeMap
                | TokenType::Identifier
                | TokenType::TypeStruct
        ) {
            self.advance();
        }

        result.success_type(parsed_type)
    }

    /// Parse generic type: list<T>, map<K, V>
    fn parse_generic_type(&mut self, base_name: String) -> ParseResult {
        let mut result = ParseResult::new();

        // Consume '<'
        self.advance();

        match base_name.as_str() {
            "list" => {
                let inner_type = result.register_type(&self.parse_type());
                if result.error.is_some() {
                    return result;
                }

                // Expect '>'
                match self.current_token() {
                    Some(tok) if tok.kind == TokenType::Gt => {
                        self.advance();
                    }
                    Some(tok) => {
                        return result.failure(
                            InvalidSyntaxError::new(
                                tok.position_start.clone(),
                                tok.position_end.clone(),
                                "Expected '>'",
                            )
                            .base,
                        );
                    }
                    None => {
                        return result.failure(
                            InvalidSyntaxError::new(
                                Self::dummy_pos(),
                                Self::dummy_pos(),
                                "Expected '>'",
                            )
                            .base,
                        );
                    }
                }

                result.success_type(Type::List(Box::new(inner_type)))
            }
            "map" => {
                let key_type = result.register_type(&self.parse_type());
                if result.error.is_some() {
                    return result;
                }

                // Expect ','
                match self.current_token() {
                    Some(tok) if tok.kind == TokenType::Comma => {
                        self.advance();
                    }
                    Some(tok) => {
                        return result.failure(
                            InvalidSyntaxError::new(
                                tok.position_start.clone(),
                                tok.position_end.clone(),
                                "Expected ','",
                            )
                            .base,
                        );
                    }
                    None => {
                        return result.failure(
                            InvalidSyntaxError::new(
                                Self::dummy_pos(),
                                Self::dummy_pos(),
                                "Expected ','",
                            )
                            .base,
                        );
                    }
                }

                let value_type = result.register_type(&self.parse_type());
                if result.error.is_some() {
                    return result;
                }

                // Expect '>'
                match self.current_token() {
                    Some(tok) if tok.kind == TokenType::Gt => {
                        self.advance();
                    }
                    Some(tok) => {
                        return result.failure(
                            InvalidSyntaxError::new(
                                tok.position_start.clone(),
                                tok.position_end.clone(),
                                "Expected '>'",
                            )
                            .base,
                        );
                    }
                    None => {
                        return result.failure(
                            InvalidSyntaxError::new(
                                Self::dummy_pos(),
                                Self::dummy_pos(),
                                "Expected '>'",
                            )
                            .base,
                        );
                    }
                }

                result.success_type(Type::Map(Box::new(key_type), Box::new(value_type)))
            }
            _ => result.failure(
                InvalidSyntaxError::new(
                    Self::dummy_pos(),
                    Self::dummy_pos(),
                    &format!("Unknown generic type: {}", base_name),
                )
                .base,
            ),
        }
    }

    fn statement(&mut self) -> ParseResult {
        let mut result = ParseResult::new();
        let pos_start = self
            .current_token()
            .map(|t| t.position_start.clone())
            .unwrap_or_else(Self::dummy_pos);

        // Check for impl block FIRST
        if let Some(tok) = self.current_token() {
            if tok.kind == TokenType::TypeImpl {
                return self.impl_block();
            }
        }

        // Check for type alias
        if let Some(tok) = self.current_token() {
            if tok.kind == TokenType::TypeAlias {
                return self.type_alias();
            }
        }
        if let Some(tok) = self.current_token() {
            if tok.matches(TokenType::Keyword, Some("match")) {
                return self.match_expr();
            }
        }

        // Check for type alias
        if let Some(tok) = self.current_token() {
            if tok.kind == TokenType::TypeAlias {
                return self.type_alias();
            }
        }

        // Check for struct definition
        if let Some(tok) = self.current_token() {
            if tok.kind == TokenType::TypeStruct || tok.matches(TokenType::Keyword, Some("struct"))
            {
                return self.struct_definition();
            }
        }

        // Check for grab statement
        if let Some(tok) = self.current_token() {
            if tok.matches(TokenType::Keyword, Some("grab")) {
                return self.grab_statement();
            }
        }

        // Check for export statement
        if let Some(tok) = self.current_token() {
            if tok.matches(TokenType::Keyword, Some("export")) {
                return self.export_statement();
            }
        }

        // Check for return statement
        if let Some(tok) = self.current_token() {
            if tok.matches(TokenType::Keyword, Some("release")) {
                self.advance();
                let expr = result.register(&self.expr());
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

    fn type_alias(&mut self) -> ParseResult {
        let mut result = ParseResult::new();
        let pos_start = self.current_token().unwrap().position_start.clone();

        // Consume 'type'
        self.advance();

        // Parse the alias name
        let name = match self.current_token() {
            Some(t) if t.kind == TokenType::Identifier => t.clone(),
            Some(t) => {
                return result.failure(
                    InvalidSyntaxError::new(
                        t.position_start.clone(),
                        t.position_end.clone(),
                        "Expected type alias name",
                    )
                    .base,
                );
            }
            None => {
                return result.failure(
                    InvalidSyntaxError::new(
                        Self::dummy_pos(),
                        Self::dummy_pos(),
                        "Expected type alias name",
                    )
                    .base,
                );
            }
        };
        self.advance();

        // Expect '='
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

        // Parse the target type
        let alias_type = result.register_type(&self.parse_type());
        if result.error.is_some() {
            return result;
        }

        let pos_end = self
            .current_token()
            .map(|t| t.position_end.clone())
            .unwrap_or(pos_start.clone());

        result.success(Node::TypeAlias(Box::new(TypeAliasNode {
            name,
            alias_type,
            position_start: pos_start,
            position_end: pos_end,
        })))
    }

    fn try_parse_field_access(&mut self) -> Option<Node> {
        let start_index = self.token_index;

        // Parse the object (should be an identifier)
        let object = match self.current_token() {
            Some(tok) if tok.kind == TokenType::Identifier => {
                let obj_node = Node::VarAccess(VarAccessNode {
                    variable_name_token: tok.clone(),
                    position_start: tok.position_start.clone(),
                    position_end: tok.position_end.clone(),
                });
                self.advance();
                obj_node
            }
            _ => {
                self.token_index = start_index;
                return None;
            }
        };

        // Check for dot
        match self.current_token() {
            Some(tok) if tok.kind == TokenType::Dot => {
                self.advance();
            }
            _ => {
                self.token_index = start_index;
                return None;
            }
        }

        // Parse the field name
        let field_name = match self.current_token() {
            Some(tok) if tok.kind == TokenType::Identifier => {
                let field = tok.clone();
                self.advance();
                field
            }
            _ => {
                self.token_index = start_index;
                return None;
            }
        };

        // Create a method access node (which we'll treat as field access)
        let method_token = Token::new(
            TokenType::Identifier,
            field_name.value.clone(),
            field_name.position_start.clone(),
            Some(field_name.position_end.clone()),
        );

        Some(Node::MethodAccess(MethodAccessNode {
            object: Box::new(object.clone()), // Clone here to avoid move
            method_name: method_token,
            position_start: object.position_start().clone(),
            position_end: field_name.position_end.clone(),
        }))
    }

    fn expr(&mut self) -> ParseResult {
        // Check for field assignment (e.g., object.field = value)
        let start_index = self.token_index;

        // Check if we have an identifier followed by dot
        if let Some(tok) = self.current_token() {
            if tok.kind == TokenType::Identifier {
                if let Some(dot) = self.peek_token() {
                    if dot.kind == TokenType::Dot {
                        let call_result = self.call();
                        if call_result.error.is_some() {
                            // Reset and fall through
                            self.token_index = start_index;
                        } else if let Some(eq) = self.current_token() {
                            if eq.kind == TokenType::Eq {
                                // Field assignment path
                                let eq_token = eq.clone();
                                self.advance();

                                let value_result = self.expr();
                                if value_result.error.is_some() {
                                    return value_result;
                                }
                                let value = value_result.node.unwrap();
                                let field_node = call_result.node.unwrap();
                                let pos_end = value.position_end().clone();

                                let assign_node =
                                    Node::BinaryOperator(Box::new(BinaryOperatorNode {
                                        left_node: Box::new(field_node),
                                        operator_token: eq_token,
                                        right_node: Box::new(value),
                                        position_start: self.tokens[start_index]
                                            .position_start
                                            .clone(),
                                        position_end: pos_end,
                                    }));

                                return ParseResult::new().success(assign_node);
                            } else {
                                // Not an assignment — reset and fall through to ternary_expr
                                self.token_index = start_index;
                            }
                        } else {
                            // No next token — reset and fall through
                            self.token_index = start_index;
                        }
                    }
                }
            }
        }

        self.token_index = start_index;

        // Try to parse a field access (for simple cases without nested dots)
        if let Some(mut node) = self.try_parse_field_access() {
            // Check if next token is '='
            if let Some(eq) = self.current_token() {
                if eq.kind == TokenType::Eq {
                    let eq_token = eq.clone();
                    self.advance(); // consume '='

                    // Parse the value
                    let value_result = self.expr();
                    if value_result.error.is_some() {
                        return value_result;
                    }
                    let value = value_result.node.unwrap();

                    // Create an assignment node for the field
                    let pos_end = value.position_end().clone();

                    // Create a binary operation node for field assignment
                    let assign_node = Node::BinaryOperator(Box::new(BinaryOperatorNode {
                        left_node: Box::new(node),
                        operator_token: eq_token,
                        right_node: Box::new(value),
                        position_start: self.tokens[start_index].position_start.clone(),
                        position_end: pos_end,
                    }));

                    return ParseResult::new().success(assign_node);
                }
            }
        }
        // If not a field assignment, reset and continue
        self.token_index = start_index;

        // Check for const spawn declaration
        if let Some(tok) = self.current_token() {
            if tok.matches(TokenType::Keyword, Some("const")) {
                return self.var_declaration();
            }
        }

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
                        return self.increment_decrement(true);
                    } else if next_tok.kind == TokenType::MinusMinus {
                        return self.increment_decrement(false);
                    }
                }
            }
        }

        // Parse ternary expression (for non-assignment expressions)
        self.ternary_expr()
    }

    fn try_catch_expr(&mut self) -> ParseResult {
        let mut result = ParseResult::new();
        let pos_start = self.current_token().unwrap().position_start.clone();

        // Consume 'try'
        self.advance();

        // Parse try block (must be { ... })
        let try_block = result.register(&self.block());
        if result.error.is_some() {
            return result;
        }

        // Expect 'catch'
        match self.current_token() {
            Some(t) if t.matches(TokenType::Keyword, Some("catch")) => {
                self.advance();
            }
            Some(t) => {
                return result.failure(
                    InvalidSyntaxError::new(
                        t.position_start.clone(),
                        t.position_end.clone(),
                        "Expected 'catch'",
                    )
                    .base,
                );
            }
            None => {
                return result.failure(
                    InvalidSyntaxError::new(
                        Self::dummy_pos(),
                        Self::dummy_pos(),
                        "Expected 'catch'",
                    )
                    .base,
                );
            }
        }

        // Parse catch variable (identifier)
        let catch_var = match self.current_token() {
            Some(t) if t.kind == TokenType::Identifier => t.clone(),
            Some(t) => {
                return result.failure(
                    InvalidSyntaxError::new(
                        t.position_start.clone(),
                        t.position_end.clone(),
                        "Expected identifier for catch variable",
                    )
                    .base,
                );
            }
            None => {
                return result.failure(
                    InvalidSyntaxError::new(
                        Self::dummy_pos(),
                        Self::dummy_pos(),
                        "Expected identifier for catch variable",
                    )
                    .base,
                );
            }
        };
        self.advance();

        // Parse catch block
        let catch_block = result.register(&self.block());
        if result.error.is_some() {
            return result;
        }

        let pos_end = self
            .current_token()
            .map(|t| t.position_end.clone())
            .unwrap_or(pos_start.clone());

        result.success(Node::TryCatch(Box::new(TryCatchNode {
            try_block: Box::new(try_block.unwrap()),
            catch_var,
            catch_block: Box::new(catch_block.unwrap()),
            position_start: pos_start,
            position_end: pos_end,
        })))
    }

    fn panic_expr(&mut self) -> ParseResult {
        let mut result = ParseResult::new();
        let pos_start = self.current_token().unwrap().position_start.clone();

        // Consume 'panic'
        self.advance();

        // Parse message expression
        let message = result.register(&self.expr());
        if result.error.is_some() {
            return result;
        }

        let pos_end = message
            .as_ref()
            .map(|n| n.position_end().clone())
            .unwrap_or(pos_start.clone());

        result.success(Node::Panic(Box::new(PanicNode {
            message_node: Box::new(message.unwrap()),
            position_start: pos_start,
            position_end: pos_end,
        })))
    }

    /// Parse grab/import statement
    /// Syntax: grab { name, other as alias } from "module"
    ///         grab * as namespace from "module"
    fn grab_statement(&mut self) -> ParseResult {
        let mut result = ParseResult::new();
        let pos_start = self.current_token().unwrap().position_start.clone();

        // Consume 'grab'
        self.advance();

        // Check for namespace import (grab * as name)
        let is_namespace_import = if let Some(tok) = self.current_token() {
            tok.matches(TokenType::Mul, None)
        } else {
            false
        };

        let mut imports = Vec::new();
        let mut namespace_alias = None;

        if is_namespace_import {
            // Consume '*'
            self.advance();

            // Expect 'as'
            match self.current_token() {
                Some(tok) if tok.matches(TokenType::Keyword, Some("as")) => {
                    self.advance();
                }
                Some(tok) => {
                    return result.failure(
                        InvalidSyntaxError::new(
                            tok.position_start.clone(),
                            tok.position_end.clone(),
                            "Expected 'as' after '*'",
                        )
                        .base,
                    );
                }
                None => {
                    return result.failure(
                        InvalidSyntaxError::new(
                            Self::dummy_pos(),
                            Self::dummy_pos(),
                            "Expected 'as' after '*'",
                        )
                        .base,
                    );
                }
            }

            // Parse namespace alias (identifier)
            match self.current_token() {
                Some(tok) if tok.kind == TokenType::Identifier => {
                    namespace_alias = Some(tok.value.clone().unwrap());
                    self.advance();
                }
                Some(tok) => {
                    return result.failure(
                        InvalidSyntaxError::new(
                            tok.position_start.clone(),
                            tok.position_end.clone(),
                            "Expected identifier for namespace alias",
                        )
                        .base,
                    );
                }
                None => {
                    return result.failure(
                        InvalidSyntaxError::new(
                            Self::dummy_pos(),
                            Self::dummy_pos(),
                            "Expected identifier for namespace alias",
                        )
                        .base,
                    );
                }
            }
        } else {
            // Parse named imports: { name, other as alias }
            match self.current_token() {
                Some(tok) if tok.kind == TokenType::LBrace => {
                    self.advance(); // consume '{'

                    // Parse import specifiers
                    loop {
                        // Skip newlines
                        while let Some(t) = self.current_token() {
                            if t.kind == TokenType::Newline {
                                self.advance();
                            } else {
                                break;
                            }
                        }

                        // Check for closing brace
                        if let Some(tok) = self.current_token() {
                            if tok.kind == TokenType::RBrace {
                                self.advance();
                                break;
                            }
                        }

                        // Parse identifier
                        let name_start = self.current_token().unwrap().position_start.clone();
                        let name = match self.current_token() {
                            Some(tok) if tok.kind == TokenType::Identifier => {
                                let name = tok.value.clone().unwrap();
                                self.advance();
                                name
                            }
                            Some(tok) => {
                                return result.failure(
                                    InvalidSyntaxError::new(
                                        tok.position_start.clone(),
                                        tok.position_end.clone(),
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

                        let name_end = self
                            .current_token()
                            .map(|t| t.position_start.clone())
                            .unwrap_or_else(Self::dummy_pos);

                        // Check for 'as' alias
                        let alias = if let Some(tok) = self.current_token() {
                            if tok.matches(TokenType::Keyword, Some("as")) {
                                self.advance(); // consume 'as'

                                match self.current_token() {
                                    Some(alias_tok) if alias_tok.kind == TokenType::Identifier => {
                                        let alias_name = alias_tok.value.clone().unwrap();
                                        self.advance();
                                        Some(alias_name)
                                    }
                                    Some(tok) => {
                                        return result.failure(
                                            InvalidSyntaxError::new(
                                                tok.position_start.clone(),
                                                tok.position_end.clone(),
                                                "Expected identifier after 'as'",
                                            )
                                            .base,
                                        );
                                    }
                                    None => {
                                        return result.failure(
                                            InvalidSyntaxError::new(
                                                Self::dummy_pos(),
                                                Self::dummy_pos(),
                                                "Expected identifier after 'as'",
                                            )
                                            .base,
                                        );
                                    }
                                }
                            } else {
                                None
                            }
                        } else {
                            None
                        };

                        imports.push(ImportSpec {
                            original_name: name,
                            alias,
                            position_start: name_start,
                            position_end: name_end,
                        });

                        // Check for comma
                        if let Some(tok) = self.current_token() {
                            if tok.kind == TokenType::Comma {
                                self.advance();
                                continue;
                            } else if tok.kind == TokenType::RBrace {
                                self.advance();
                                break;
                            } else {
                                break;
                            }
                        } else {
                            break;
                        }
                    }
                }
                Some(tok) => {
                    return result.failure(
                        InvalidSyntaxError::new(
                            tok.position_start.clone(),
                            tok.position_end.clone(),
                            "Expected '{' or '*'",
                        )
                        .base,
                    );
                }
                None => {
                    return result.failure(
                        InvalidSyntaxError::new(
                            Self::dummy_pos(),
                            Self::dummy_pos(),
                            "Expected '{' or '*'",
                        )
                        .base,
                    );
                }
            }
        }

        // Expect 'from'
        match self.current_token() {
            Some(tok) if tok.matches(TokenType::Keyword, Some("from")) => {
                self.advance();
            }
            Some(tok) => {
                return result.failure(
                    InvalidSyntaxError::new(
                        tok.position_start.clone(),
                        tok.position_end.clone(),
                        "Expected 'from'",
                    )
                    .base,
                );
            }
            None => {
                return result.failure(
                    InvalidSyntaxError::new(
                        Self::dummy_pos(),
                        Self::dummy_pos(),
                        "Expected 'from'",
                    )
                    .base,
                );
            }
        }

        // Parse module path (string)
        let module_path = match self.current_token() {
            Some(tok) if tok.kind == TokenType::String => {
                let path = tok.value.clone().unwrap();
                self.advance();
                path
            }
            Some(tok) => {
                return result.failure(
                    InvalidSyntaxError::new(
                        tok.position_start.clone(),
                        tok.position_end.clone(),
                        "Expected module path string",
                    )
                    .base,
                );
            }
            None => {
                return result.failure(
                    InvalidSyntaxError::new(
                        Self::dummy_pos(),
                        Self::dummy_pos(),
                        "Expected module path string",
                    )
                    .base,
                );
            }
        };

        let pos_end = self
            .current_token()
            .map(|t| t.position_end.clone())
            .unwrap_or(pos_start.clone());

        result.success(Node::Grab(Box::new(GrabNode {
            imports,
            from_module: module_path,
            is_namespace_import,
            namespace_alias,
            position_start: pos_start,
            position_end: pos_end,
        })))
    }

    /// Parse export statement
    /// Syntax: export <item>
    fn export_statement(&mut self) -> ParseResult {
        let mut result = ParseResult::new();
        let pos_start = self.current_token().unwrap().position_start.clone();

        // Consume 'export'
        self.advance();

        // Parse the exported item (function, variable, etc.)
        let item = result.register(&self.expr());
        if result.error.is_some() {
            return result;
        }

        let pos_end = item
            .as_ref()
            .map(|n| n.position_end().clone())
            .unwrap_or(pos_start.clone());

        // Get the name of the exported item
        let exported_name = if let Some(node) = &item {
            match node {
                Node::VarAssign(var_assign) => var_assign
                    .variable_name_token
                    .value
                    .as_ref()
                    .unwrap()
                    .clone(),
                Node::FuncDef(func_def) => {
                    if let Some(token) = &func_def.variable_name_token {
                        token.value.as_ref().unwrap().clone()
                    } else {
                        return result.failure(
                            InvalidSyntaxError::new(
                                pos_start.clone(),
                                pos_end.clone(),
                                "Cannot export anonymous function",
                            )
                            .base,
                        );
                    }
                }
                _ => {
                    return result.failure(
                        InvalidSyntaxError::new(
                            pos_start.clone(),
                            pos_end.clone(),
                            "Can only export variables and functions",
                        )
                        .base,
                    );
                }
            }
        } else {
            return result.failure(
                InvalidSyntaxError::new(pos_start.clone(), pos_end, "Expected item to export").base,
            );
        };

        result.success(Node::Export(Box::new(ExportNode {
            exported_name,
            node: Box::new(item.unwrap()),
            position_start: pos_start,
            position_end: pos_end,
        })))
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
                var_type: None,
                value_node: Box::new(bin_op_node),
                is_constant: false,
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
            var_type: None,
            value_node: Box::new(bin_op_node),
            is_constant: false,
            position_start: pos_start,
            position_end: pos_end,
        })))
    }

    // Variable declaration with 'spawn' keyword
    fn var_declaration(&mut self) -> ParseResult {
        let mut result = ParseResult::new();
        let mut is_constant = false;

        // Check for 'const' keyword
        if let Some(tok) = self.current_token() {
            if tok.matches(TokenType::Keyword, Some("const")) {
                is_constant = true;
                self.advance(); // consume 'const'
            }
        }

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

        // Parse type annotation (required)
        let var_type = match self.current_token() {
            Some(t) if t.kind == TokenType::Colon => {
                self.advance(); // consume ':'
                let typ = result.register_type(&self.parse_type());
                if result.error.is_some() {
                    return result;
                }
                Some(typ)
            }
            _ => {
                return result.failure(
                    InvalidSyntaxError::new(
                        var_name.position_start.clone(),
                        var_name.position_end.clone(),
                        "Expected type annotation ':'",
                    )
                    .base,
                );
            }
        };

        // Parse '='
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

        let pos_end = self
            .current_token()
            .map(|t| t.position_end.clone())
            .unwrap_or_else(|| pos_start.clone());

        result.success(Node::VarAssign(Box::new(VarAssignNode {
            variable_name_token: var_name,
            var_type,
            value_node: Box::new(value.unwrap()),
            is_constant,
            position_start: pos_start,
            position_end: pos_end,
        })))
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
                var_type: None,
                value_node: Box::new(val_node),
                is_constant: false,
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
        self.advance();

        // Parse the value to match against - only simple expressions, not full expressions
        // This prevents expr() from consuming the '{' that follows
        let value_node = match self.current_token() {
            Some(t) if t.kind == TokenType::Identifier => {
                let node = Node::VarAccess(VarAccessNode {
                    variable_name_token: t.clone(),
                    position_start: t.position_start.clone(),
                    position_end: t.position_end.clone(),
                });
                self.advance();
                Some(node)
            }
            Some(t) if t.kind == TokenType::String => {
                let node = Node::String(StringNode::new(t.clone()));
                self.advance();
                Some(node)
            }
            Some(t) if t.kind == TokenType::Int || t.kind == TokenType::Float => {
                let node = Node::Number(NumberNode::new(t.clone()));
                self.advance();
                Some(node)
            }
            Some(t) => {
                return result.failure(
                    InvalidSyntaxError::new(
                        t.position_start.clone(),
                        t.position_end.clone(),
                        &format!("Expected expression to match on, got {:?}", t.kind),
                    )
                    .base,
                );
            }
            None => {
                return result.failure(
                    InvalidSyntaxError::new(
                        Self::dummy_pos(),
                        Self::dummy_pos(),
                        "Expected expression to match on",
                    )
                    .base,
                );
            }
        };

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

        let mut arms = Vec::new();

        loop {
            // Skip newlines
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
            }

            // Parse pattern - handle string literals directly
            let pattern_node = match self.current_token() {
                Some(t) if t.kind == TokenType::String => {
                    let node = Node::String(StringNode::new(t.clone()));
                    self.advance();
                    node
                }
                Some(t) if t.kind == TokenType::Underscore => {
                    let node = Node::VarAccess(VarAccessNode {
                        variable_name_token: t.clone(),
                        position_start: t.position_start.clone(),
                        position_end: t.position_end.clone(),
                    });
                    self.advance();
                    node
                }
                Some(t) if t.kind == TokenType::Identifier => {
                    let node = Node::VarAccess(VarAccessNode {
                        variable_name_token: t.clone(),
                        position_start: t.position_start.clone(),
                        position_end: t.position_end.clone(),
                    });
                    self.advance();
                    node
                }
                Some(t) => {
                    return result.failure(
                        InvalidSyntaxError::new(
                            t.position_start.clone(),
                            t.position_end.clone(),
                            &format!("Expected pattern, got {:?}", t.kind),
                        )
                        .base,
                    );
                }
                None => {
                    return result.failure(
                        InvalidSyntaxError::new(
                            Self::dummy_pos(),
                            Self::dummy_pos(),
                            "Expected pattern",
                        )
                        .base,
                    );
                }
            };

            // Skip newlines before '=>'
            while let Some(t) = self.current_token() {
                if t.kind == TokenType::Newline {
                    self.advance();
                } else {
                    break;
                }
            }

            // Expect '=>'
            match self.current_token() {
                Some(t) if t.kind == TokenType::FatArrow => {
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

            // Skip newlines after '=>'
            while let Some(t) = self.current_token() {
                if t.kind == TokenType::Newline {
                    self.advance();
                } else {
                    break;
                }
            }

            // Parse body (block)
            let body_node = match self.current_token() {
                Some(t) if t.kind == TokenType::LBrace => match result.register(&self.block()) {
                    Some(node) => node,
                    None => {
                        return result.failure(
                            InvalidSyntaxError::new(
                                Self::dummy_pos(),
                                Self::dummy_pos(),
                                "Expected block body",
                            )
                            .base,
                        );
                    }
                },
                _ => {
                    return result.failure(
                        InvalidSyntaxError::new(
                            Self::dummy_pos(),
                            Self::dummy_pos(),
                            "Expected '{' for match arm body",
                        )
                        .base,
                    );
                }
            };

            if result.error.is_some() {
                return result;
            }

            // Store pattern and body positions before moving them
            let pattern_pos_start = pattern_node.position_start().clone();
            let pattern_pos_end = pattern_node.position_end().clone();
            let body_pos_end = body_node.position_end().clone();

            arms.push(MatchArm {
                pattern_node: Box::new(pattern_node),
                body_node: Box::new(body_node),
                position_start: pattern_pos_start,
                position_end: body_pos_end,
            });
        }

        let pos_end = self
            .current_token()
            .map(|t| t.position_end.clone())
            .unwrap_or_else(|| pos_start.clone());

        result.success(Node::Match(Box::new(MatchNode {
            value_node: Box::new(value_node.unwrap()),
            arms,
            position_start: pos_start,
            position_end: pos_end,
        })))
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
                let node = Node::VarAccess(VarAccessNode {
                    variable_name_token: t.clone(),
                    position_start: t.position_start.clone(),
                    position_end: t.position_end.clone(),
                });
                self.advance();
                result.success(node)
            }
            Some(t) if t.kind == TokenType::Identifier => {
                // For identifiers in patterns, treat them as variable references
                // (except '_' which we already handled)
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
                    &format!("Expected pattern, got {:?}", t.kind),
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

            // Check for comparison operators
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

            // Check for 'as' type conversion (keyword)
            let is_as = tok.matches(TokenType::Keyword, Some("as"));

            if !is_operator && !is_logical && !is_as {
                break;
            }

            let op = tok.clone();
            self.advance();

            if is_as {
                // For 'as', the right side should be a type name, not a full expression
                // Parse the type name (int, float, string, bool)
                let type_name = match self.current_token() {
                    Some(t) if t.kind == TokenType::TypeInt => "int",
                    Some(t) if t.kind == TokenType::TypeFloat => "float",
                    Some(t) if t.kind == TokenType::TypeString => "string",
                    Some(t) if t.kind == TokenType::TypeBool => "bool",
                    Some(t) => {
                        return result.failure(
                            InvalidSyntaxError::new(
                                t.position_start.clone(),
                                t.position_end.clone(),
                                &format!("Expected type name after 'as', got {:?}", t.kind),
                            )
                            .base,
                        );
                    }
                    None => {
                        return result.failure(
                            InvalidSyntaxError::new(
                                Self::dummy_pos(),
                                Self::dummy_pos(),
                                "Expected type name after 'as'",
                            )
                            .base,
                        );
                    }
                };

                let type_token = self.current_token().unwrap().clone();
                self.advance();

                // Create a string node for the type name
                let right = Node::String(StringNode::new(Token::new(
                    TokenType::String,
                    Some(type_name.to_string()),
                    type_token.position_start.clone(),
                    Some(type_token.position_end.clone()),
                )));

                left = Node::bin_op(left, op, right);
            } else {
                // Regular operator - parse right side as expression
                let right = result.register(&self.arith_expr());
                if result.error.is_some() {
                    return result;
                }
                let right = right.unwrap();
                left = Node::bin_op(left, op, right);
            }
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

                let field_or_method_name = match self.current_token() {
                    Some(t) if t.kind == TokenType::Identifier => t.clone(),
                    Some(t) => {
                        return result.failure(
                            InvalidSyntaxError::new(
                                t.position_start.clone(),
                                t.position_end.clone(),
                                "Expected field or method name",
                            )
                            .base,
                        );
                    }
                    None => {
                        return result.failure(
                            InvalidSyntaxError::new(
                                Self::dummy_pos(),
                                Self::dummy_pos(),
                                "Expected field or method name",
                            )
                            .base,
                        );
                    }
                };
                self.advance();

                // Check if this is a method call (followed by '(') or field access
                let is_method_call =
                    matches!(self.current_token(), Some(t) if t.kind == TokenType::LParen);

                if is_method_call {
                    // Parse method call arguments
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

                    // Create method call node
                    let node_pos_start = node.position_start().clone();
                    let method_pos_end = field_or_method_name.position_end.clone();
                    let method_pos_start = field_or_method_name.position_start.clone();

                    let method_token = Token::new(
                        TokenType::Identifier,
                        field_or_method_name.value.clone(),
                        method_pos_start,
                        Some(method_pos_end.clone()),
                    );

                    let new_node = Node::Call(Box::new(CallNode {
                        node_to_call: Box::new(Node::MethodAccess(MethodAccessNode {
                            object: Box::new(node),
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
                } else {
                    // This is field access - create a binary operation node with dot operator
                    let field_token = Token::new(
                        TokenType::Identifier,
                        field_or_method_name.value.clone(),
                        field_or_method_name.position_start.clone(),
                        Some(field_or_method_name.position_end.clone()),
                    );

                    let field_node = Node::VarAccess(VarAccessNode {
                        variable_name_token: field_token,
                        position_start: field_or_method_name.position_start.clone(),
                        position_end: field_or_method_name.position_end.clone(),
                    });

                    // Create a binary operation node for field access
                    let dot_token = Token::new(
                        TokenType::Dot,
                        None,
                        tok.position_start.clone(),
                        Some(field_or_method_name.position_end.clone()),
                    );

                    // Clone node before moving it into the Box
                    let node_clone = node.clone();

                    node = Node::BinaryOperator(Box::new(BinaryOperatorNode {
                        left_node: Box::new(node_clone),
                        operator_token: dot_token,
                        right_node: Box::new(field_node),
                        position_start: node.position_start().clone(),
                        position_end: field_or_method_name.position_end.clone(),
                    }));
                }
            } else if tok.kind == TokenType::LParen {
                // Regular function call (existing code)
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
                TokenType::BoolTrue => {
                    let node = Node::BoolLiteral(BoolLiteralNode {
                        value: true,
                        position_start: tok.position_start.clone(),
                        position_end: tok.position_end.clone(),
                    });
                    self.advance();
                    return result.success(node);
                }
                TokenType::BoolFalse => {
                    let node = Node::BoolLiteral(BoolLiteralNode {
                        value: false,
                        position_start: tok.position_start.clone(),
                        position_end: tok.position_end.clone(),
                    });
                    self.advance();
                    return result.success(node);
                }
                TokenType::TypeNull => {
                    let node = Node::NullLiteral(NullLiteralNode {
                        position_start: tok.position_start.clone(),
                        position_end: tok.position_end.clone(),
                    });
                    self.advance();
                    return result.success(node);
                }
                TokenType::Identifier => {
                    let struct_name = tok.value.clone().unwrap();
                    let struct_pos_start = tok.position_start.clone();
                    let struct_pos_end = tok.position_end.clone();

                    self.advance();

                    // Check for static method call: Identifier::method
                    if let Some(cc) = self.current_token() {
                        if cc.kind == TokenType::ColonColon {
                            self.advance(); // consume '::'

                            let method_name = match self.current_token() {
                                Some(t) if t.kind == TokenType::Identifier => {
                                    let name = t.value.clone().unwrap();
                                    self.advance();
                                    name
                                }
                                Some(t) => {
                                    return result.failure(
                                        InvalidSyntaxError::new(
                                            t.position_start.clone(),
                                            t.position_end.clone(),
                                            "Expected method name after '::'",
                                        )
                                        .base,
                                    );
                                }
                                None => {
                                    return result.failure(
                                        InvalidSyntaxError::new(
                                            Self::dummy_pos(),
                                            Self::dummy_pos(),
                                            "Expected method name after '::'",
                                        )
                                        .base,
                                    );
                                }
                            };

                            let combined = format!("{}::{}", struct_name, method_name);
                            let node = Node::VarAccess(VarAccessNode {
                                variable_name_token: Token::new(
                                    TokenType::Identifier,
                                    Some(combined),
                                    struct_pos_start.clone(),
                                    Some(
                                        self.current_token()
                                            .map(|t| t.position_start.clone())
                                            .unwrap_or(struct_pos_end.clone()),
                                    ),
                                ),
                                position_start: struct_pos_start,
                                position_end: struct_pos_end,
                            });
                            return result.success(node);
                        }
                    }

                    // Check if this is a struct instantiation (followed by {} ON THE SAME LINE)
                    if let Some(lbrace) = self.current_token() {
                        if lbrace.kind == TokenType::LBrace
                            && lbrace.position_start.line == struct_pos_end.line
                        {
                            // Skip newlines to find the first non-newline token after '{'
                            let mut peek_index = self.token_index + 1;
                            let mut after_brace = self.tokens.get(peek_index);
                            while let Some(t) = after_brace {
                                if t.kind == TokenType::Newline {
                                    peek_index += 1;
                                    after_brace = self.tokens.get(peek_index);
                                } else {
                                    break;
                                }
                            }

                            let is_struct = match after_brace {
                                Some(t) if t.kind == TokenType::RBrace => true, // empty struct
                                Some(t) if t.kind == TokenType::Identifier => {
                                    // Check if token after identifier is ':' (skip newlines)
                                    let mut after_identifier_index = peek_index + 1;
                                    let mut after_identifier =
                                        self.tokens.get(after_identifier_index);
                                    while let Some(t) = after_identifier {
                                        if t.kind == TokenType::Newline {
                                            after_identifier_index += 1;
                                            after_identifier =
                                                self.tokens.get(after_identifier_index);
                                        } else {
                                            break;
                                        }
                                    }
                                    matches!(after_identifier, Some(t2) if t2.kind == TokenType::Colon)
                                }
                                _ => false,
                            };
                            if is_struct {
                                return self.struct_instantiation(struct_name);
                            }
                        }
                    }

                    // Not a struct instantiation, create variable access node
                    let node = Node::VarAccess(VarAccessNode {
                        variable_name_token: Token::new(
                            TokenType::Identifier,
                            Some(struct_name.clone()),
                            struct_pos_start.clone(),
                            Some(struct_pos_end.clone()),
                        ),
                        position_start: struct_pos_start,
                        position_end: struct_pos_end,
                    });

                    // REMOVED: All dot-access handling code
                    // The call() function will handle method calls like object.method()
                    // Just return the variable access node and let call() process any following dots

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
                _ if tok.matches(TokenType::Keyword, Some("try")) => {
                    return self.try_catch_expr();
                }
                _ if tok.matches(TokenType::Keyword, Some("panic")) => {
                    return self.panic_expr();
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

    fn struct_instantiation(&mut self, struct_name: String) -> ParseResult {
        let mut result = ParseResult::new();
        let pos_start = self.current_token().unwrap().position_start.clone();

        // Consume '{'
        self.advance();

        let mut fields = Vec::new();

        // Parse fields
        loop {
            // Skip newlines
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
            }

            // Parse field name
            let field_name = match self.current_token() {
                Some(t) if t.kind == TokenType::Identifier => {
                    let name = t.clone();
                    self.advance();
                    name
                }
                Some(t) => {
                    return result.failure(
                        InvalidSyntaxError::new(
                            t.position_start.clone(),
                            t.position_end.clone(),
                            "Expected field name",
                        )
                        .base,
                    );
                }
                None => {
                    return result.failure(
                        InvalidSyntaxError::new(
                            Self::dummy_pos(),
                            Self::dummy_pos(),
                            "Expected field name",
                        )
                        .base,
                    );
                }
            };

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

            // Parse field value
            let field_value = result.register(&self.expr());
            if result.error.is_some() {
                return result;
            }

            fields.push((field_name, field_value.unwrap()));

            // Check for comma or closing brace
            if let Some(t) = self.current_token() {
                if t.kind == TokenType::Comma {
                    self.advance();
                    continue;
                } else if t.kind == TokenType::RBrace {
                    self.advance();
                    break;
                }
            }
        }

        let pos_end = self
            .current_token()
            .map(|t| t.position_end.clone())
            .unwrap_or(pos_start.clone());

        result.success(Node::StructInstantiation(Box::new(
            StructInstantiationNode {
                struct_name,
                fields,
                position_start: pos_start,
                position_end: pos_end,
            },
        )))
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

        // Parse the variable name(s) - could be a single variable or (key, value) tuple
        let var_name_token = match self.current_token() {
            Some(t) if t.kind == TokenType::Identifier => {
                // Check if this is a tuple pattern (key, value) without parentheses
                let first = t.clone();
                self.advance();

                // Look ahead to see if there's a comma
                if let Some(comma) = self.current_token() {
                    if comma.kind == TokenType::Comma {
                        self.advance(); // consume ','

                        let second = match self.current_token() {
                            Some(tok) if tok.kind == TokenType::Identifier => tok.clone(),
                            _ => {
                                return result.failure(
                                    InvalidSyntaxError::new(
                                        Self::dummy_pos(),
                                        Self::dummy_pos(),
                                        "Expected identifier after comma",
                                    )
                                    .base,
                                );
                            }
                        };
                        self.advance();

                        // Create a special token that encodes the tuple pattern
                        let tuple_name = format!(
                            "({},{})",
                            first.value.as_ref().unwrap(),
                            second.value.as_ref().unwrap()
                        );
                        Token::new(
                            TokenType::Identifier,
                            Some(tuple_name),
                            first.position_start.clone(),
                            Some(second.position_end.clone()),
                        )
                    } else {
                        // Single variable
                        first
                    }
                } else {
                    first
                }
            }
            Some(t) if t.kind == TokenType::LParen => {
                // Tuple pattern with parentheses (key, value)
                self.advance(); // consume '('
                let first = match self.current_token() {
                    Some(t) if t.kind == TokenType::Identifier => t.clone(),
                    _ => {
                        return result.failure(
                            InvalidSyntaxError::new(
                                Self::dummy_pos(),
                                Self::dummy_pos(),
                                "Expected identifier in tuple pattern",
                            )
                            .base,
                        );
                    }
                };
                self.advance();

                match self.current_token() {
                    Some(t) if t.kind == TokenType::Comma => {
                        self.advance();
                    }
                    _ => {
                        return result.failure(
                            InvalidSyntaxError::new(
                                Self::dummy_pos(),
                                Self::dummy_pos(),
                                "Expected ',' in tuple pattern",
                            )
                            .base,
                        );
                    }
                }

                let second = match self.current_token() {
                    Some(t) if t.kind == TokenType::Identifier => t.clone(),
                    _ => {
                        return result.failure(
                            InvalidSyntaxError::new(
                                Self::dummy_pos(),
                                Self::dummy_pos(),
                                "Expected identifier in tuple pattern",
                            )
                            .base,
                        );
                    }
                };
                self.advance();

                match self.current_token() {
                    Some(t) if t.kind == TokenType::RParen => {
                        self.advance();
                    }
                    _ => {
                        return result.failure(
                            InvalidSyntaxError::new(
                                Self::dummy_pos(),
                                Self::dummy_pos(),
                                "Expected ')' in tuple pattern",
                            )
                            .base,
                        );
                    }
                }

                // Create a special token that encodes the tuple pattern
                let tuple_name = format!(
                    "({},{})",
                    first.value.as_ref().unwrap(),
                    second.value.as_ref().unwrap()
                );
                Token::new(
                    TokenType::Identifier,
                    Some(tuple_name),
                    first.position_start.clone(),
                    Some(second.position_end.clone()),
                )
            }
            Some(t) => {
                return result.failure(
                    InvalidSyntaxError::new(
                        t.position_start.clone(),
                        t.position_end.clone(),
                        "Expected identifier or tuple pattern",
                    )
                    .base,
                );
            }
            None => {
                return result.failure(
                    InvalidSyntaxError::new(
                        Self::dummy_pos(),
                        Self::dummy_pos(),
                        "Expected identifier or tuple pattern",
                    )
                    .base,
                );
            }
        };

        // Check if this is a collection iteration (using 'in') or range iteration (using '=')
        let is_collection_iteration = if let Some(tok) = self.current_token() {
            tok.value.as_deref() == Some("in")
        } else {
            false
        };

        if is_collection_iteration {
            // Collection iteration: for item in collection { ... }
            self.advance(); // consume 'in'

            let collection_expr = result.register(&self.expr());
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

            let (collection, body_node) = match (collection_expr, body) {
                (Some(c), Some(b)) => (c, b),
                _ => {
                    return result.failure(
                        InvalidSyntaxError::new(
                            Self::dummy_pos(),
                            Self::dummy_pos(),
                            "Invalid collection iteration",
                        )
                        .base,
                    );
                }
            };

            let pos_end = body_node.position_end().clone();
            return result.success(Node::For(Box::new(ForNode {
                variable_name_token: var_name_token,
                start_value_node: Box::new(collection),
                end_value_node: Box::new(Node::Number(NumberNode::new(Token::new(
                    TokenType::Int,
                    Some("0".to_string()),
                    pos_start.clone(),
                    None,
                )))),
                step_value_node: None,
                body_node: Box::new(body_node),
                should_return_null: false,
                position_start: pos_start,
                position_end: pos_end,
            })));
        }

        // Range iteration: for i = 0 to 5 step 2 { ... }
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
                variable_name_token: var_name_token,
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

        // Skip newlines before body
        while let Some(t) = self.current_token() {
            if t.kind == TokenType::Newline {
                self.advance();
            } else {
                break;
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

        let mut param_names = Vec::new();
        let mut param_types = Vec::new();

        // Parse parameters
        if let Some(t) = self.current_token() {
            if t.kind != TokenType::RParen {
                // Parse first parameter
                let param_name = match self.current_token() {
                    Some(tok) if tok.kind == TokenType::Identifier => {
                        let name = tok.clone();
                        self.advance();
                        name
                    }
                    Some(tok) => {
                        return result.failure(
                            InvalidSyntaxError::new(
                                tok.position_start.clone(),
                                tok.position_end.clone(),
                                "Expected parameter name",
                            )
                            .base,
                        );
                    }
                    None => {
                        return result.failure(
                            InvalidSyntaxError::new(
                                Self::dummy_pos(),
                                Self::dummy_pos(),
                                "Expected parameter name",
                            )
                            .base,
                        );
                    }
                };

                // Parse type annotation
                match self.current_token() {
                    Some(tok) if tok.kind == TokenType::Colon => {
                        self.advance(); // consume ':'
                        let param_type = result.register_type(&self.parse_type());
                        if result.error.is_some() {
                            return result;
                        }
                        param_names.push(param_name);
                        param_types.push(param_type);
                    }
                    _ => {
                        return result.failure(
                            InvalidSyntaxError::new(
                                param_name.position_start.clone(),
                                param_name.position_end.clone(),
                                "Expected type annotation ':' for parameter",
                            )
                            .base,
                        );
                    }
                }

                // Parse remaining parameters
                while let Some(comma) = self.current_token() {
                    if comma.kind != TokenType::Comma {
                        break;
                    }
                    self.advance();

                    let next_name = match self.current_token() {
                        Some(tok) if tok.kind == TokenType::Identifier => {
                            let name = tok.clone();
                            self.advance();
                            name
                        }
                        Some(tok) => {
                            return result.failure(
                                InvalidSyntaxError::new(
                                    tok.position_start.clone(),
                                    tok.position_end.clone(),
                                    "Expected parameter name",
                                )
                                .base,
                            );
                        }
                        None => {
                            return result.failure(
                                InvalidSyntaxError::new(
                                    Self::dummy_pos(),
                                    Self::dummy_pos(),
                                    "Expected parameter name",
                                )
                                .base,
                            );
                        }
                    };

                    match self.current_token() {
                        Some(tok) if tok.kind == TokenType::Colon => {
                            self.advance(); // consume ':'
                            let param_type = result.register_type(&self.parse_type());
                            if result.error.is_some() {
                                return result;
                            }
                            param_names.push(next_name);
                            param_types.push(param_type);
                        }
                        _ => {
                            return result.failure(
                                InvalidSyntaxError::new(
                                    next_name.position_start.clone(),
                                    next_name.position_end.clone(),
                                    "Expected type annotation ':' for parameter",
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

        // Parse return type
        let return_type = match self.current_token() {
            Some(t) if t.kind == TokenType::Arrow => {
                self.advance(); // consume '->'
                let typ = result.register_type(&self.parse_type());
                if result.error.is_some() {
                    return result;
                }
                typ
            }
            _ => Type::Null, // Default return type
        };

        // Parse body - check for arrow function first
        let is_arrow = if let Some(t) = self.current_token() {
            t.kind == TokenType::FatArrow
        } else {
            false
        };

        let body = if is_arrow {
            self.advance(); // consume '=>'
            let expr = result.register(&self.expr());
            if result.error.is_some() {
                return result;
            }
            expr.unwrap()
        } else {
            match self.current_token() {
                Some(t) if t.kind == TokenType::LBrace => {
                    let block = result.register(&self.block());
                    if result.error.is_some() {
                        return result;
                    }
                    block.unwrap()
                }
                Some(t) => {
                    return result.failure(
                        InvalidSyntaxError::new(
                            t.position_start.clone(),
                            t.position_end.clone(),
                            "Expected '=>' or '{'",
                        )
                        .base,
                    );
                }
                None => {
                    return result.failure(
                        InvalidSyntaxError::new(
                            Self::dummy_pos(),
                            Self::dummy_pos(),
                            "Expected '=>' or '{'",
                        )
                        .base,
                    );
                }
            }
        };

        let pos_end = body.position_end().clone();

        result.success(Node::FuncDef(Box::new(FuncDefNode {
            variable_name_token: var_name_tok,
            param_names,
            param_types,
            return_type,
            body_node: Box::new(body),
            is_arrow,
            position_start: pos_start,
            position_end: pos_end,
        })))
    }

    /// Parse struct definition
    /// Syntax: struct Name { field: type, field2: type }
    fn struct_definition(&mut self) -> ParseResult {
        let mut result = ParseResult::new();
        let pos_start = self.current_token().unwrap().position_start.clone();

        // Consume 'struct'
        self.advance();

        // Parse struct name
        let struct_name = match self.current_token() {
            Some(t) if t.kind == TokenType::Identifier => t.clone(),
            Some(t) => {
                return result.failure(
                    InvalidSyntaxError::new(
                        t.position_start.clone(),
                        t.position_end.clone(),
                        "Expected struct name",
                    )
                    .base,
                );
            }
            None => {
                return result.failure(
                    InvalidSyntaxError::new(
                        Self::dummy_pos(),
                        Self::dummy_pos(),
                        "Expected struct name",
                    )
                    .base,
                );
            }
        };
        self.advance();

        let struct_name_str = struct_name.value.as_ref().unwrap().clone();

        // Register the struct (create empty entry)
        self.register_struct(&struct_name_str);

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

        let mut fields = Vec::new();

        // Parse fields
        loop {
            // Skip newlines
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
            }

            // Parse field name
            let field_name = match self.current_token() {
                Some(t) if t.kind == TokenType::Identifier => t.clone(),
                Some(t) => {
                    return result.failure(
                        InvalidSyntaxError::new(
                            t.position_start.clone(),
                            t.position_end.clone(),
                            "Expected field name",
                        )
                        .base,
                    );
                }
                None => {
                    return result.failure(
                        InvalidSyntaxError::new(
                            Self::dummy_pos(),
                            Self::dummy_pos(),
                            "Expected field name",
                        )
                        .base,
                    );
                }
            };
            self.advance();

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

            // Parse field type
            let field_type = result.register_type(&self.parse_type());
            if result.error.is_some() {
                return result;
            }

            let field_name_str = field_name.value.as_ref().unwrap().clone();

            // Now borrow mutably to add the field
            if let Some(struct_info) = self.struct_registry.get_mut(&struct_name_str) {
                if let Err(err) = struct_info.add_field(field_name_str, field_type.clone()) {
                    return result.failure(
                        InvalidSyntaxError::new(
                            field_name.position_start.clone(),
                            field_name.position_end.clone(),
                            &err,
                        )
                        .base,
                    );
                }
            }

            fields.push(StructFieldNode {
                name: field_name.clone(),
                field_type,
                is_constant: false,
                position_start: field_name.position_start.clone(),
                position_end: self
                    .current_token()
                    .map(|t| t.position_end.clone())
                    .unwrap_or(field_name.position_end.clone()),
            });

            // Check for comma or closing brace
            if let Some(t) = self.current_token() {
                if t.kind == TokenType::Comma {
                    self.advance();
                    continue;
                } else if t.kind == TokenType::RBrace {
                    self.advance();
                    break;
                }
            }
        }

        let pos_end = self
            .current_token()
            .map(|t| t.position_end.clone())
            .unwrap_or(pos_start.clone());

        result.success(Node::StructDef(Box::new(StructDefNode {
            name: struct_name,
            fields,
            position_start: pos_start,
            position_end: pos_end,
        })))
    }

    pub fn register_struct(&mut self, struct_name: &str) {
        if !self.struct_registry.contains_key(struct_name) {
            self.struct_registry.insert(
                struct_name.to_string(),
                StructInfo::new(struct_name.to_string()),
            );
        }
    }

    pub fn struct_exists(&self, name: &str) -> bool {
        self.struct_registry.contains_key(name)
    }

    fn impl_block(&mut self) -> ParseResult {
        let mut result = ParseResult::new();
        let pos_start = self.current_token().unwrap().position_start.clone();

        // Consume 'impl'
        self.advance();

        // Parse struct name
        let struct_name = match self.current_token() {
            Some(t) if t.kind == TokenType::Identifier => {
                let name = t.clone();
                self.advance();
                name
            }
            Some(t) => {
                return result.failure(
                    InvalidSyntaxError::new(
                        t.position_start.clone(),
                        t.position_end.clone(),
                        "Expected struct name after 'impl'",
                    )
                    .base,
                );
            }
            None => {
                return result.failure(
                    InvalidSyntaxError::new(
                        Self::dummy_pos(),
                        Self::dummy_pos(),
                        "Expected struct name after 'impl'",
                    )
                    .base,
                );
            }
        };

        let struct_name_str = struct_name.value.as_ref().unwrap().to_string();

        // Check if struct exists (immutable borrow only)
        if !self.struct_exists(&struct_name_str) {
            return result.failure(
                InvalidSyntaxError::new(
                    struct_name.position_start.clone(),
                    struct_name.position_end.clone(),
                    &format!("Struct '{}' not defined before impl block", struct_name_str),
                )
                .base,
            );
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

        let mut methods = Vec::new();

        // Parse methods inside the impl block
        loop {
            // Skip newlines
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
            }

            // Parse method definition (reuse func_def)
            let method_result = self.func_def();
            if method_result.error.is_some() {
                return method_result;
            }

            if let Some(method_node) = method_result.node {
                // Extract the FuncDefNode from the Node enum
                if let Node::FuncDef(func_def) = method_node {
                    // Validate that the first parameter is 'self: Self'
                    if func_def.param_names.is_empty() {
                        return result.failure(
                            InvalidSyntaxError::new(
                                pos_start.clone(),
                                pos_start.clone(),
                                "Impl methods must have 'self: Self' as first parameter",
                            )
                            .base,
                        );
                    }

                    let first_param = &func_def.param_names[0];
                    let first_param_name = first_param.value.as_ref().unwrap();

                    if first_param_name != "self" {
                        return result.failure(
                            InvalidSyntaxError::new(
                                first_param.position_start.clone(),
                                first_param.position_end.clone(),
                                "First parameter of impl method must be 'self'",
                            )
                            .base,
                        );
                    }

                    // Check that the type is Self (or the struct name)
                    if func_def.param_types.is_empty() {
                        return result.failure(
                            InvalidSyntaxError::new(
                                first_param.position_start.clone(),
                                first_param.position_end.clone(),
                                "Parameter 'self' must have type annotation",
                            )
                            .base,
                        );
                    }

                    // Store the method - get mutable borrow just for this operation
                    if let Some(struct_info) = self.struct_registry.get_mut(&struct_name_str) {
                        struct_info.add_method((*func_def).clone());
                    }
                    methods.push(func_def);
                }
            }
        }

        let pos_end = self
            .current_token()
            .map(|t| t.position_end.clone())
            .unwrap_or(pos_start.clone());

        result.success(Node::Impl(Box::new(ImplNode {
            struct_name,
            methods,
            position_start: pos_start,
            position_end: pos_end,
        })))
    }
}

// Struct registry for tracking struct definitions and their methods
#[derive(Debug, Clone)]
pub struct StructInfo {
    pub name: String,
    pub fields: HashMap<String, Type>,
    pub field_order: Vec<String>,
    pub methods: HashMap<String, crate::nodes::FuncDefNode>,
}

impl StructInfo {
    pub fn new(name: String) -> Self {
        Self {
            name,
            fields: HashMap::new(),
            field_order: Vec::new(),
            methods: HashMap::new(),
        }
    }

    pub fn add_field(&mut self, name: String, field_type: Type) -> Result<(), String> {
        if self.fields.contains_key(&name) {
            return Err(format!(
                "Duplicate field '{}' in struct '{}'",
                name, self.name
            ));
        }
        self.fields.insert(name.clone(), field_type);
        self.field_order.push(name);
        Ok(())
    }

    pub fn get_field_type(&self, name: &str) -> Option<&Type> {
        self.fields.get(name)
    }

    pub fn add_method(&mut self, method: crate::nodes::FuncDefNode) {
        if let Some(name) = &method.variable_name_token {
            if let Some(method_name) = &name.value {
                self.methods.insert(method_name.clone(), method);
            }
        }
    }

    pub fn get_method(&self, name: &str) -> Option<&crate::nodes::FuncDefNode> {
        self.methods.get(name)
    }
}
