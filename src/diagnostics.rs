//! # Diagnostics Module
//!
//! Provides beautiful, helpful error messages with suggestions and quick fixes.

use crate::position::Position;
use crate::types::Type;
use crate::values::Value;
use colored::*;
use strsim::levenshtein;

/// Enhanced diagnostic system with suggestions and quick fixes
pub struct Diagnostics {
    // Simplified - we'll use our own formatting instead of codespan
}

impl Diagnostics {
    pub fn new() -> Self {
        Self {}
    }

    /// Create a type mismatch error with suggestions
    pub fn type_mismatch(
        &self,
        expected: &Type,
        found: &Value,
        pos: &Position,
        _context: &str,
    ) -> String {
        let expected_str = expected.to_string();
        let found_str = Self::value_type_to_string(found);

        let mut output = String::new();

        output.push_str(&format!(
            "{}\n",
            "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".bright_red()
        ));
        output.push_str(&format!(
            "{} {}: {}\n",
            "error".bright_red().bold(),
            "XEN001".bright_yellow(),
            "Type Mismatch".bright_red()
        ));

        output.push_str(&format!(
            "  {} {}:{}:{}\n",
            "→".bright_cyan(),
            pos.file_name.bright_cyan(),
            (pos.line + 1).to_string().bright_yellow(),
            (pos.column + 1).to_string().bright_yellow()
        ));

        // Get line content if available
        if !pos.file_text.is_empty() {
            let lines: Vec<&str> = pos.file_text.lines().collect();
            if pos.line < lines.len() {
                let line = lines[pos.line];
                output.push_str(&format!(
                    "\n  {} │ {}\n",
                    format!("{:>4}", pos.line + 1).dimmed(),
                    line
                ));

                // Arrow
                let arrow = " ".repeat(pos.column)
                    + "^"
                        .repeat(std::cmp::max(
                            1,
                            (pos.column + 5).saturating_sub(pos.column),
                        ))
                        .as_str();
                output.push_str(&format!("      {}\n", arrow.bright_red()));
            }
        }

        output.push_str(&format!(
            "\n  {} Cannot assign `{}` to variable of type `{}`\n",
            "note".bright_cyan(),
            found_str,
            expected_str
        ));

        if Self::can_convert(found, expected) {
            output.push_str(&format!(
                "  {} Try converting the value: `{} as {}`\n",
                "💡".bright_green(),
                Self::value_example(found),
                expected_str
            ));
        } else {
            output.push_str(&format!(
                "  {} Expected type `{}` but got `{}`\n",
                "💡".bright_green(),
                expected_str,
                found_str
            ));
            output.push_str(&format!(
                "     Check the value being assigned or change the type annotation\n"
            ));
        }

        output.push_str(&format!(
            "{}\n",
            "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".bright_red()
        ));

        output
    }

    /// Create an undefined variable error with suggestions
    pub fn undefined_variable(
        &self,
        name: &str,
        pos: &Position,
        available_vars: &[String],
    ) -> String {
        let mut output = String::new();

        output.push_str(&format!(
            "{}\n",
            "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".bright_red()
        ));
        output.push_str(&format!(
            "{} {}: {}\n",
            "error".bright_red().bold(),
            "XEN002".bright_yellow(),
            "Undefined Variable".bright_red()
        ));

        output.push_str(&format!(
            "  {} {}:{}:{}\n",
            "→".bright_cyan(),
            pos.file_name.bright_cyan(),
            (pos.line + 1).to_string().bright_yellow(),
            (pos.column + 1).to_string().bright_yellow()
        ));

        // Get line content
        if !pos.file_text.is_empty() {
            let lines: Vec<&str> = pos.file_text.lines().collect();
            if pos.line < lines.len() {
                let line = lines[pos.line];
                output.push_str(&format!(
                    "\n  {} │ {}\n",
                    format!("{:>4}", pos.line + 1).dimmed(),
                    line
                ));

                let arrow = " ".repeat(pos.column) + "^".repeat(name.len()).as_str();
                output.push_str(&format!("      {}\n", arrow.bright_red()));
            }
        }

        output.push_str(&format!(
            "\n  {} Variable `{}` is not defined in this scope\n",
            "note".bright_cyan(),
            name
        ));
        output.push_str(&format!(
            "  {} Variables must be declared with `let` before use\n",
            "💡".bright_green()
        ));

        // Find similar variable names
        let mut suggestions: Vec<&String> = available_vars
            .iter()
            .filter(|v| levenshtein(name, v) <= 3)
            .collect();

        suggestions.sort_by(|a, b| levenshtein(name, a).cmp(&levenshtein(name, b)));

        if !suggestions.is_empty() {
            let suggestion_list = suggestions
                .iter()
                .take(3)
                .map(|s| format!("    - `{}`", s))
                .collect::<Vec<_>>()
                .join("\n");
            output.push_str(&format!(
                "  {} Did you mean one of these?\n{}\n",
                "💡".bright_green(),
                suggestion_list
            ));
        } else {
            output.push_str(&format!(
                "  {} Try declaring it first: `let {} = value`\n",
                "💡".bright_green(),
                name
            ));
        }

        output.push_str(&format!(
            "{}\n",
            "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".bright_red()
        ));

        output
    }

    /// Helper methods
    fn value_type_to_string(value: &Value) -> String {
        match value {
            Value::Number(n) => {
                if n.value.fract() == 0.0 {
                    "int".to_string()
                } else {
                    "float".to_string()
                }
            }
            Value::String(_) => "string".to_string(),
            Value::Bool(_) => "bool".to_string(),
            Value::List(_) => "list".to_string(),
            Value::Map(_) => "map".to_string(),
            Value::Struct(s) => format!("struct {}", s.name),
            Value::Function(_) => "function".to_string(),
            Value::BuiltInFunction(_) => "builtin".to_string(),
            Value::Null => "null".to_string(),
            Value::Json(_) => "json".to_string(),
        }
    }

    fn can_convert(value: &Value, target: &Type) -> bool {
        match (value, target) {
            (Value::Number(_), Type::String) => true,
            (Value::Number(_), Type::Bool) => true,
            (Value::String(_), Type::Int) => true,
            (Value::String(_), Type::Float) => true,
            (Value::String(_), Type::Bool) => true,
            (Value::Bool(_), Type::Int) => true,
            (Value::Bool(_), Type::Float) => true,
            (Value::Bool(_), Type::String) => true,
            _ => false,
        }
    }

    fn value_example(value: &Value) -> String {
        match value {
            Value::Number(n) => format!("{} as target_type", n.value as i64),
            Value::String(s) => format!("\"{}\" as target_type", s.value),
            Value::Bool(b) => format!("{} as target_type", b),
            _ => "value as target_type".to_string(),
        }
    }
}

pub type Diagnostic = String;
