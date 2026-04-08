//! # Error Handling Module
//!
//! Defines the error hierarchy for Xenith including lexical errors,
//! syntax errors, and runtime errors with traceback support.
//! Provides formatted error messages with arrow pointers to source code.

use crate::context::Context;
use crate::position::Position;

/// Base error structure containing common error information
#[derive(Debug, Clone)]
pub struct Error {
    pub code: String,
    pub position_start: Position,
    pub position_end: Position,
    pub error_name: String,
    pub details: String,
    pub note: Option<String>,
    pub help: Option<String>,
    pub cause: Option<Box<Error>>,
}

impl Error {
    pub fn new(
        position_start: Position,
        position_end: Position,
        error_name: &str,
        details: &str,
    ) -> Self {
        Self {
            code: "XEN000".to_string(),
            position_start,
            position_end,
            error_name: error_name.to_string(),
            details: details.to_string(),
            note: None,
            help: None,
            cause: None,
        }
    }

    pub fn with_code(mut self, code: &str) -> Self {
        self.code = code.to_string();
        self
    }

    pub fn with_note(mut self, note: &str) -> Self {
        self.note = Some(note.to_string());
        self
    }

    pub fn with_help(mut self, help: &str) -> Self {
        self.help = Some(help.to_string());
        self
    }

    pub fn with_cause(mut self, cause: Error) -> Self {
        self.cause = Some(Box::new(cause));
        self
    }

    /// Type mismatch error (XEN001)
    pub fn type_mismatch(
        expected: &str,
        found: &str,
        pos_start: Position,
        pos_end: Position,
    ) -> Self {
        Self::new(
            pos_start,
            pos_end,
            "Type Mismatch",
            &format!("expected `{}`, found `{}`", expected, found),
        )
        .with_code("XEN001")
        .with_note(&format!(
            "cannot assign `{}` to variable of type `{}`",
            found, expected
        ))
        .with_help(&format!("use type conversion: `value as {}`", expected))
    }

    /// Undefined variable error (XEN002)
    pub fn undefined_variable(name: &str, pos_start: Position, pos_end: Position) -> Self {
        Self::new(
            pos_start,
            pos_end,
            "Undefined Variable",
            &format!("`{}` is not defined", name),
        )
        .with_code("XEN002")
        .with_note("variables must be declared with `spawn` before use")
        .with_help("check spelling or declare the variable first")
    }

    /// Division by zero error (XEN003)
    pub fn division_by_zero(pos_start: Position, pos_end: Position) -> Self {
        Self::new(
            pos_start,
            pos_end,
            "Division by Zero",
            "cannot divide by zero",
        )
        .with_code("XEN003")
        .with_note("division by zero is not allowed")
        .with_help("check if denominator is zero before dividing")
    }

    /// Index out of bounds error (XEN004)
    pub fn index_out_of_bounds(
        index: usize,
        len: usize,
        pos_start: Position,
        pos_end: Position,
    ) -> Self {
        Self::new(
            pos_start,
            pos_end,
            "Index Out of Bounds",
            &format!("index `{}` out of bounds", index),
        )
        .with_code("XEN004")
        .with_note(&format!(
            "list length is `{}`, but index `{}` was requested",
            len, index
        ))
        .with_help(&format!("valid indices are `0` to `{}`", len - 1))
    }

    /// File not found error (XEN005)
    pub fn file_not_found(path: &str, pos_start: Position, pos_end: Position) -> Self {
        Self::new(
            pos_start,
            pos_end,
            "File Not Found",
            &format!("file `{}` not found", path),
        )
        .with_code("XEN005")
        .with_note(&format!("attempted to open: `{}`", path))
        .with_help("check if the file exists and the path is correct")
    }

    /// Invalid JSON error (XEN006)
    pub fn invalid_json(msg: &str, pos_start: Position, pos_end: Position) -> Self {
        Self::new(pos_start, pos_end, "Invalid JSON", msg)
            .with_code("XEN006")
            .with_note("the provided string is not valid JSON")
            .with_help("check the JSON syntax")
    }

    /// Environment variable not found error (XEN007)
    pub fn env_not_found(key: &str, pos_start: Position, pos_end: Position) -> Self {
        Self::new(
            pos_start,
            pos_end,
            "Environment Variable Not Found",
            &format!("`{}` not found", key),
        )
        .with_code("XEN007")
        .with_note(&format!("environment variable `{}` is not set", key))
        .with_help("check if the variable exists or provide a default value")
    }

    /// Method not found error (XEN008)
    pub fn method_not_found(
        struct_name: &str,
        method_name: &str,
        pos_start: Position,
        pos_end: Position,
    ) -> Self {
        Self::new(
            pos_start,
            pos_end,
            "Method Not Found",
            &format!(
                "method `{}` not found for struct `{}`",
                method_name, struct_name
            ),
        )
        .with_code("XEN008")
        .with_note(&format!(
            "the struct `{}` has no method named `{}`",
            struct_name, method_name
        ))
        .with_help("check the method name spelling or define the method in an `impl` block")
    }

    /// Struct field not found error (XEN009)
    pub fn field_not_found(
        struct_name: &str,
        field_name: &str,
        pos_start: Position,
        pos_end: Position,
    ) -> Self {
        Self::new(
            pos_start,
            pos_end,
            "Field Not Found",
            &format!(
                "field `{}` not found for struct `{}`",
                field_name, struct_name
            ),
        )
        .with_code("XEN009")
        .with_note(&format!(
            "the struct `{}` has no field named `{}`",
            struct_name, field_name
        ))
        .with_help("check the field name spelling")
    }

    /// Permission denied error (XEN010)
    pub fn permission_denied(path: &str, pos_start: Position, pos_end: Position) -> Self {
        Self::new(
            pos_start,
            pos_end,
            "Permission Denied",
            &format!("cannot access `{}`", path),
        )
        .with_code("XEN010")
        .with_note("insufficient permissions to access the file or directory")
        .with_help("check file permissions or run with appropriate privileges")
    }

    /// Invalid type conversion error (XEN011)
    pub fn invalid_conversion(
        from: &str,
        to: &str,
        pos_start: Position,
        pos_end: Position,
    ) -> Self {
        Self::new(
            pos_start,
            pos_end,
            "Invalid Type Conversion",
            &format!("cannot convert `{}` to `{}`", from, to),
        )
        .with_code("XEN011")
        .with_note(&format!(
            "conversion from `{}` to `{}` is not supported",
            from, to
        ))
        .with_help("use a different conversion or check the value format")
    }

    /// Module not found error (XEN012)
    pub fn module_not_found(name: &str, pos_start: Position, pos_end: Position) -> Self {
        Self::new(
            pos_start,
            pos_end,
            "Module Not Found",
            &format!("module `{}` not found", name),
        )
        .with_code("XEN012")
        .with_note(&format!("could not locate module `{}`", name))
        .with_help("check the module path or ensure the file exists")
    }

    /// Unexpected token error (XEN013)
    pub fn unexpected_token(
        token: &str,
        expected: &str,
        pos_start: Position,
        pos_end: Position,
    ) -> Self {
        Self::new(
            pos_start,
            pos_end,
            "Unexpected Token",
            &format!("expected `{}`, found `{}`", expected, token),
        )
        .with_code("XEN013")
        .with_note("the parser encountered an unexpected token")
        .with_help("check the syntax near this location")
    }

    /// Missing return value error (XEN014)
    pub fn missing_return(method_name: &str, pos_start: Position, pos_end: Position) -> Self {
        Self::new(
            pos_start,
            pos_end,
            "Missing Return Value",
            &format!("method `{}` must return a value", method_name),
        )
        .with_code("XEN014")
        .with_note("the method has a return type but no `release` statement")
        .with_help("add a `release` statement with a value")
    }

    /// Too many arguments error (XEN015)
    pub fn too_many_arguments(
        expected: usize,
        found: usize,
        pos_start: Position,
        pos_end: Position,
    ) -> Self {
        Self::new(
            pos_start,
            pos_end,
            "Too Many Arguments",
            &format!("expected `{}` arguments, got `{}`", expected, found),
        )
        .with_code("XEN015")
        .with_note(&format!(
            "the method takes `{}` arguments but `{}` were provided",
            expected, found
        ))
        .with_help("check the method signature and remove extra arguments")
    }

    /// Too few arguments error (XEN016)
    pub fn too_few_arguments(
        expected: usize,
        found: usize,
        pos_start: Position,
        pos_end: Position,
    ) -> Self {
        Self::new(
            pos_start,
            pos_end,
            "Too Few Arguments",
            &format!("expected `{}` arguments, got `{}`", expected, found),
        )
        .with_code("XEN016")
        .with_note(&format!(
            "the method takes `{}` arguments but only `{}` were provided",
            expected, found
        ))
        .with_help("check the method signature and add missing arguments")
    }

    pub fn as_string(&self) -> String {
        let mut result = String::new();

        // Error header
        result.push_str(&format!("Error {}: {}\n", self.code, self.error_name));
        result.push_str(&format!(
            "  → {}:{}:{}\n",
            self.position_start.file_name,
            self.position_start.line + 1,
            self.position_start.column + 1
        ));

        // Get the line content
        let line_content = self.get_line_content();
        if !line_content.is_empty() {
            result.push_str(&format!("  │\n"));
            result.push_str(&format!("  │ {}\n", line_content));

            // Create arrow pointing to the error
            let mut arrow = String::new();
            let start_col = self.position_start.column;
            let end_col = self.position_end.column;

            for i in 0..line_content.len() {
                if i >= start_col && i < end_col {
                    arrow.push('^');
                } else if i < start_col {
                    arrow.push(' ');
                }
            }

            if !arrow.is_empty() {
                result.push_str(&format!("  │ {}\n", arrow));
            }
        }

        result.push_str(&format!("  │\n"));

        // Note
        if let Some(note) = &self.note {
            result.push_str(&format!("  = note: {}\n", note));
        }

        // Help
        if let Some(help) = &self.help {
            result.push_str(&format!("  = help: {}\n", help));
        }

        // Cause chain
        if let Some(cause) = &self.cause {
            result.push_str("\nCaused by:\n");
            result.push_str(&cause.as_string());
        }

        result
    }

    fn get_line_content(&self) -> String {
        if self.position_start.file_text.is_empty() {
            return String::new();
        }

        let lines: Vec<&str> = self.position_start.file_text.lines().collect();
        if self.position_start.line < lines.len() {
            lines[self.position_start.line].to_string()
        } else {
            String::new()
        }
    }

    fn get_arrow(&self) -> String {
        let line = self.get_line_content();
        let start = self.position_start.column;
        let end = self.position_end.column;
        let mut arrow = String::new();
        for i in 0..line.len() {
            if i >= start && i < end {
                arrow.push('^');
            } else if i < start {
                arrow.push(' ');
            }
        }
        arrow
    }
}

/// Error for illegal characters in source code
#[derive(Debug, Clone)]
pub struct IllegalCharError {
    pub base: Error,
}

impl IllegalCharError {
    pub fn new(position_start: Position, position_end: Position, details: &str) -> Self {
        Self {
            base: Error::new(position_start, position_end, "Illegal Character", details)
                .with_code("XEN100")
                .with_note(&format!("character `{}` is not allowed", details))
                .with_help("remove the illegal character or use a valid one"),
        }
    }
}

/// Error for missing expected characters
#[derive(Debug, Clone)]
pub struct ExpectedCharError {
    pub base: Error,
}

impl ExpectedCharError {
    pub fn new(position_start: Position, position_end: Position, details: &str) -> Self {
        Self {
            base: Error::new(position_start, position_end, "Expected Character", details)
                .with_code("XEN101")
                .with_note("the parser expected a specific character")
                .with_help(&format!("add the missing character: `{}`", details)),
        }
    }
}

/// Error for invalid syntax
#[derive(Debug, Clone)]
pub struct InvalidSyntaxError {
    pub base: Error,
}

impl InvalidSyntaxError {
    pub fn new(position_start: Position, position_end: Position, details: &str) -> Self {
        Self {
            base: Error::new(position_start, position_end, "Invalid Syntax", details)
                .with_code("XEN102")
                .with_note("the code does not follow Xenith syntax rules")
                .with_help("review the syntax near this location"),
        }
    }
}

/// Runtime error with execution context traceback
#[derive(Debug, Clone)]
pub struct RuntimeError {
    pub base: Error,
    pub context: Option<Box<Context>>,
}

impl RuntimeError {
    pub fn new(
        position_start: Position,
        position_end: Position,
        details: &str,
        context: Option<Context>,
    ) -> Self {
        Self {
            base: Error::new(position_start, position_end, "Runtime Error", details)
                .with_code("XEN200"),
            context: context.map(Box::new),
        }
    }

    pub fn with_code(mut self, code: &str) -> Self {
        self.base = self.base.with_code(code);
        self
    }

    pub fn with_note(mut self, note: &str) -> Self {
        self.base = self.base.with_note(note);
        self
    }

    pub fn with_help(mut self, help: &str) -> Self {
        self.base = self.base.with_help(help);
        self
    }

    pub fn as_string(&self) -> String {
        let mut result = self.generate_traceback();
        result.push_str(&self.base.as_string());
        result
    }

    fn generate_traceback(&self) -> String {
        let mut result = String::new();
        let mut position = self.base.position_start.clone();
        let mut context: Option<&Context> = self.context.as_deref();

        while let Some(ctx) = context {
            result = format!(
                "  File {}, line {}, in {}\n{}",
                position.file_name,
                position.line + 1,
                ctx.display_name,
                result
            );
            if let Some(parent_position) = &ctx.parent_entry_position {
                position = parent_position.clone();
            }
            context = ctx.parent.as_ref().map(|b| &**b);
        }

        if !result.is_empty() {
            format!("Traceback (most recent call last):\n{}", result)
        } else {
            String::new()
        }
    }
}

impl From<ExpectedCharError> for IllegalCharError {
    fn from(err: ExpectedCharError) -> Self {
        IllegalCharError { base: err.base }
    }
}
