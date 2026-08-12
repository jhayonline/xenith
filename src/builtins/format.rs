//! Format built-in function for processing raw strings with interpolation

use crate::error::RuntimeError;
use crate::interpreter::Interpreter;
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::position::Position;
use crate::runtime_result::RuntimeResult;
use crate::values::{Value, XenithString};

pub fn format(
    args: Vec<Value>,
    interpreter: &mut Interpreter,
    call_pos: Position,
) -> RuntimeResult {
    if args.len() != 1 {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                call_pos.clone(),
                call_pos,
                "format expects 1 argument (string)",
                None,
            )
            .base,
        );
    }

    match &args[0] {
        Value::String(s) => {
            // Parse the string as an interpolated string
            // Wrap it in quotes and parse as interpolated string
            let wrapped = format!("\"{}\"", s.value);
            let mut lexer = Lexer::new("<format>".to_string(), wrapped);
            let tokens = match lexer.make_tokens() {
                Ok(t) => t,
                Err(e) => {
                    return RuntimeResult::new().failure(e.base);
                }
            };

            let mut parser = Parser::new(tokens);
            let parse_result = parser.parse_expression();

            if let Some(error) = parse_result.error {
                return RuntimeResult::new().failure(error);
            }

            match parse_result.node {
                Some(node) => {
                    // Create a temporary context
                    let mut temp_context = crate::context::Context::new("<format>", None, None);
                    let result = interpreter.visit(&node, &mut temp_context);

                    if let Some(error) = result.error {
                        return RuntimeResult::new().failure(*error);
                    }

                    if let Some(value) = result.value {
                        // Convert the result to string
                        match value {
                            Value::String(s) => RuntimeResult::new().success(Value::String(s)),
                            _ => RuntimeResult::new().success(Value::String(XenithString::new(
                                crate::utils::value_to_string(&value),
                            ))),
                        }
                    } else {
                        RuntimeResult::new()
                            .success(Value::String(XenithString::new(s.value.clone())))
                    }
                }
                None => {
                    RuntimeResult::new().success(Value::String(XenithString::new(s.value.clone())))
                }
            }
        }
        _ => RuntimeResult::new().failure(
            RuntimeError::new(
                call_pos.clone(),
                call_pos,
                "format expects a string argument",
                None,
            )
            .base,
        ),
    }
}
