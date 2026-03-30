//////////////////////////////////////////
// ERROR HANDLING MODULE
// Defines the error hierarchy for Xenith including lexical errors,
// syntax errors, and runtime errors with traceback support.
// Provides formatted error messages with arrow pointers to source code.
//////////////////////////////////////////

use crate::position::Position;
use crate::utils::string_with_arrows;

#[derive(Debug, Clone)]
pub struct Error {
    pub position_start: Position,
    pub position_end: Position,
    pub error_name: String,
    pub details: String,
}

impl Error {
    pub fn new(
        position_start: Position,
        position_end: Position,
        error_name: &str,
        details: &str,
    ) -> Self {
        Self {
            position_start,
            position_end,
            error_name: error_name.to_string(),
            details: details.to_string(),
        }
    }

    pub fn as_string(&self) -> String {
        let mut result = format!("{}: {}\n", self.error_name, self.details);
        result += &format!(
            "File {}, line {}\n\n",
            self.position_start.file_name,
            self.position_start.line + 1
        );
        result += &string_with_arrows(
            &self.position_start.file_text,
            &self.position_start,
            &self.position_end,
        );
        result
    }
}

// ---------------------------
// Specialized Errors
// ---------------------------
#[derive(Debug, Clone)]
pub struct IllegalCharError {
    pub base: Error,
}

impl IllegalCharError {
    pub fn new(position_start: Position, position_end: Position, details: &str) -> Self {
        Self {
            base: Error::new(position_start, position_end, "Illegal Character", details),
        }
    }

    pub fn as_string(&self) -> String {
        self.base.as_string()
    }
}

#[derive(Debug, Clone)]
pub struct ExpectedCharError {
    pub base: Error,
}

impl ExpectedCharError {
    pub fn new(position_start: Position, position_end: Position, details: &str) -> Self {
        Self {
            base: Error::new(position_start, position_end, "Expected Character", details),
        }
    }

    pub fn as_string(&self) -> String {
        self.base.as_string()
    }
}

#[derive(Debug, Clone)]
pub struct InvalidSyntaxError {
    pub base: Error,
}

impl InvalidSyntaxError {
    pub fn new(position_start: Position, position_end: Position, details: &str) -> Self {
        Self {
            base: Error::new(position_start, position_end, "Invalid Syntax", details),
        }
    }

    pub fn as_string(&self) -> String {
        self.base.as_string()
    }
}

// ---------------------------
// Runtime Error with Context
// ---------------------------
#[derive(Debug, Clone)]
pub struct RTError {
    pub base: Error,
    pub context: Option<Box<Context>>,
}

impl RTError {
    pub fn new(
        position_start: Position,
        position_end: Position,
        details: &str,
        context: Option<Context>,
    ) -> Self {
        Self {
            base: Error::new(position_start, position_end, "Runtime Error", details),
            context: context.map(Box::new),
        }
    }

    pub fn as_string(&self) -> String {
        let mut result = self.generate_traceback();
        result += &format!("{}: {}\n\n", self.base.error_name, self.base.details);
        result += &string_with_arrows(
            &self.base.position_start.file_text,
            &self.base.position_start,
            &self.base.position_end,
        );
        result
    }

    pub fn generate_traceback(&self) -> String {
        let mut result = String::new();
        let mut position = self.base.position_start.clone();
        let mut context: Option<&Context> = self.context.as_deref();

        while let Some(c) = context {
            result = format!(
                "  File {}, line {}, in {}\n{}",
                position.file_name,
                position.line + 1,
                c.display_name,
                result
            );
            if let Some(parent_position) = &c.parent_entry_position {
                position = parent_position.clone();
            }
            context = c.parent.as_ref().map(|b| &**b);
        }

        format!("Traceback (most recent call last):\n{}", result)
    }
}

// ---------------------------
// Context Struct (needed for RTError)
// ---------------------------
#[derive(Debug, Clone)]
pub struct Context {
    pub display_name: String,
    pub parent: Option<Box<Context>>,
    pub parent_entry_position: Option<Position>,
}

impl Context {
    pub fn new(
        display_name: &str,
        parent: Option<Context>,
        parent_entry_position: Option<Position>,
    ) -> Self {
        Self {
            display_name: display_name.to_string(),
            parent: parent.map(Box::new),
            parent_entry_position,
        }
    }
}
