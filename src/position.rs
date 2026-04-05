//! # Position Tracking Module
//!
//! Tracks the current position (index, line, column) in the source code
//! during lexical analysis. Essential for accurate error reporting
//! and maintaining source code context throughout the compilation process.

/// Represents a position in source code for error reporting
#[derive(Debug, Clone)]
pub struct Position {
    pub index: usize,
    pub line: usize,
    pub column: usize,
    pub file_name: String,
    pub file_text: String,
}

impl Position {
    /// Creates a new position
    pub fn new(index: usize, line: usize, column: usize, file_name: &str, file_text: &str) -> Self {
        Self {
            index,
            line,
            column,
            file_name: file_name.to_string(),
            file_text: file_text.to_string(),
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
}
