//! # Utility Functions Module
//!
//! Provides helper functions for character classification and
//! error message formatting with arrow pointers.

use crate::position::Position;
use crate::values::Value; // Add this import

/// Checks if a character is a digit
pub fn is_digit(c: char) -> bool {
    c.is_ascii_digit()
}

/// Checks if a character is a letter or underscore
pub fn is_letter(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}

/// Checks if a character is a letter, digit, or underscore
pub fn is_letter_or_digit(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// Generates a string showing the source code snippet with arrows pointing
/// to the error location.
///
/// # Arguments
/// * `text` - full source code text
/// * `pos_start` - starting position of the error
/// * `pos_end` - ending position of the error
///
/// Returns a `String` containing the snippet with arrows.
pub fn string_with_arrows(
    text: &str,
    position_start: &Position,
    position_end: &Position,
) -> String {
    let mut result = String::new();

    // Find the start of the line containing pos_start
    let index_start = text[..position_start.index]
        .rfind('\n')
        .map(|i| i + 1)
        .unwrap_or(0);

    // Find the end of the line containing pos_start
    let mut index_end = text[index_start..]
        .find('\n')
        .map(|i| index_start + i)
        .unwrap_or(text.len());

    // Number of lines to highlight
    let line_count = position_end.line - position_start.line + 1;

    // Loop through each line that needs highlighting
    let mut current_start = index_start;
    for i in 0..line_count {
        // Get the slice of the current line
        let line = &text[current_start..index_end];

        // Determine which columns to highlight
        let col_start = if i == 0 { position_start.column } else { 0 };
        let col_end = if i == line_count - 1 {
            position_end.column
        } else {
            line.len().saturating_sub(1)
        };

        // Append the line itself
        result.push_str(line);
        result.push('\n');

        // Append spaces and caret symbols '^' to indicate the error
        for _ in 0..col_start {
            result.push(' ');
        }
        for _ in col_start..col_end {
            result.push('^');
        }
        result.push('\n');

        // Move to next line
        current_start = index_end + 1;
        if current_start < text.len() {
            index_end = text[current_start..]
                .find('\n')
                .map(|i| current_start + i)
                .unwrap_or(text.len());
        }
    }

    result.replace('\t', "")
}

/// Converts a Value to its string representation
///
/// # Arguments
/// * `value` - The runtime value to convert
///
/// # Returns
/// A string representation of the value
pub fn value_to_string(value: &Value) -> String {
    match value {
        Value::Number(n) => {
            // Check if this is actually a boolean value
            if n.value == 0.0 {
                "false".to_string()
            } else if n.value == 1.0 {
                "true".to_string()
            } else {
                n.value.to_string()
            }
        }
        Value::String(s) => s.value.clone(),
        Value::List(l) => {
            if l.elements.len() == 1 {
                value_to_string(&l.elements[0])
            } else {
                let mut result = String::from("[");
                for (i, elem) in l.elements.iter().enumerate() {
                    if i > 0 {
                        result.push_str(", ");
                    }
                    result.push_str(&value_to_string(elem));
                }
                result.push(']');
                result
            }
        }
        Value::Function(f) => {
            if let Some(name) = &f.name {
                format!("<function {}>", name)
            } else {
                "<anonymous function>".to_string()
            }
        }
        Value::BuiltInFunction(b) => {
            format!("<built-in function {}>", b.name)
        }
        Value::Map(map) => {
            let mut result = String::from("{");
            for (i, (key, val)) in map.pairs.iter().enumerate() {
                if i > 0 {
                    result.push_str(", ");
                }
                result.push_str(&format!("\"{}\": {}", key, value_to_string(val)));
            }
            result.push('}');
            result
        }
    }
}

/// Converts a Value to its string representation for interpolation
pub fn value_to_interpolated_string(value: &Value) -> String {
    match value {
        Value::Number(n) => {
            // Check if this is actually a boolean value
            if n.value == 0.0 {
                "false".to_string()
            } else if n.value == 1.0 {
                "true".to_string()
            } else {
                n.value.to_string()
            }
        }
        Value::String(s) => s.value.clone(),
        Value::List(l) => {
            let mut result = String::from("[");
            for (i, elem) in l.elements.iter().enumerate() {
                if i > 0 {
                    result.push_str(", ");
                }
                result.push_str(&value_to_interpolated_string(elem));
            }
            result.push(']');
            result
        }
        Value::Function(f) => {
            if let Some(name) = &f.name {
                format!("<function {}>", name)
            } else {
                "<anonymous function>".to_string()
            }
        }
        Value::BuiltInFunction(b) => {
            format!("<built-in function {}>", b.name)
        }
        Value::Map(map) => {
            let mut result = String::from("{");
            for (i, (key, val)) in map.pairs.iter().enumerate() {
                if i > 0 {
                    result.push_str(", ");
                }
                result.push_str(&format!(
                    "\"{}\": {}",
                    key,
                    value_to_interpolated_string(val)
                ));
            }
            result.push('}');
            result
        }
    }
}
