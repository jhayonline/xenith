//! # Position Tracking Module
//!
//! Tracks the current position (index, line, column) in the source code
//! during lexical analysis. Essential for accurate error reporting
//! and maintaining source code context throughout the compilation process.

use std::sync::Arc;

/// Represents a position in source code for error reporting.
///
/// `file_name` and `file_text` are reference-counted, not owned. Every AST node
/// carries two positions and they are cloned constantly, so owning `String`s
/// here meant each clone memcpy'd the entire source file -- it dominated
/// interpreter runtime. `Arc` rather than `Rc` because the language server
/// shares parsed ASTs across threads.
#[derive(Debug, Clone)]
pub struct Position {
    pub index: usize,
    pub line: usize,
    pub column: usize,
    pub file_name: Arc<str>,
    pub file_text: Arc<str>,
}

fn empty() -> Arc<str> {
    // One shared empty string for every source-less position
    static EMPTY: std::sync::OnceLock<Arc<str>> = std::sync::OnceLock::new();
    EMPTY.get_or_init(|| Arc::from("")).clone()
}

impl Position {
    /// Creates a new position
    pub fn new(index: usize, line: usize, column: usize, file_name: &str, file_text: &str) -> Self {
        Self {
            index,
            line,
            column,
            file_name: if file_name.is_empty() {
                empty()
            } else {
                Arc::from(file_name)
            },
            file_text: if file_text.is_empty() {
                empty()
            } else {
                Arc::from(file_text)
            },
        }
    }

    /// Creates a position sharing already-counted source handles.
    /// Prefer this in hot paths -- it never allocates.
    pub fn with_source(
        index: usize,
        line: usize,
        column: usize,
        file_name: Arc<str>,
        file_text: Arc<str>,
    ) -> Self {
        Self {
            index,
            line,
            column,
            file_name,
            file_text,
        }
    }

    /// A position with no source attached, for internally generated nodes
    pub fn dummy() -> Self {
        Self {
            index: 0,
            line: 0,
            column: 0,
            file_name: empty(),
            file_text: empty(),
        }
    }

    /// Advances the position by one character
    pub fn advance(&mut self, current_char: Option<char>) {
        self.index += 1;
        self.column += 1;

        if current_char == Some('\n') {
            self.line += 1;
            self.column = 0;
        }
    }

    /// Creates a copy of the position
    pub fn copy(&self) -> Self {
        self.clone()
    }

    /// Generate a LSP-compatible location
    pub fn to_lsp_range(&self) -> (usize, usize, usize, usize) {
        (self.line, self.column, self.line, self.column + 1)
    }

    /// Generate a quickfix location
    pub fn to_quickfix_range(&self) -> (usize, usize, usize, usize) {
        (
            self.line,
            self.column,
            self.line,
            self.column.saturating_add(1),
        )
    }
}
