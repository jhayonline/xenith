//! # Parse Result Module
//!
//! Tracks the state during parsing operations including success/failure status,
//! node construction, and error recovery. Enables the parser to attempt
//! multiple parsing strategies and backtrack when necessary.

use crate::error::Error;
use crate::nodes::Node;

/// Result of a parsing operation with error tracking
#[derive(Debug, Clone)]
pub struct ParseResult {
    pub error: Option<Error>,
    pub node: Option<Node>,
    pub last_registered_advance_count: usize,
    pub advance_count: usize,
    pub to_reverse_count: usize,
}

impl ParseResult {
    /// Creates a new empty parse result
    pub fn new() -> Self {
        Self {
            error: None,
            node: None,
            last_registered_advance_count: 0,
            advance_count: 0,
            to_reverse_count: 0,
        }
    }

    /// Registers a token advancement
    pub fn register_advancement(&mut self) {
        self.last_registered_advance_count = 1;
        self.advance_count += 1;
    }

    /// Registers a sub-parse result
    pub fn register(&mut self, res: &ParseResult) -> Option<Node> {
        self.last_registered_advance_count = res.advance_count;
        self.advance_count += res.advance_count;

        if let Some(err) = &res.error {
            self.error = Some(err.clone());
        }

        res.node.clone()
    }

    /// Tries to register a result, storing reverse count on failure
    pub fn try_register(&mut self, res: &ParseResult) -> Option<Node> {
        if res.error.is_some() {
            self.to_reverse_count = res.advance_count;
            None
        } else {
            self.register(res)
        }
    }

    /// Sets a successful result
    pub fn success(mut self, node: Node) -> Self {
        self.node = Some(node);
        self
    }

    /// Sets a failure result
    pub fn failure(mut self, error: Error) -> Self {
        if self.error.is_none() || self.last_registered_advance_count == 0 {
            self.error = Some(error);
        }
        self
    }
}

impl Default for ParseResult {
    fn default() -> Self {
        Self::new()
    }
}
