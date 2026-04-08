//! File system built-in functions
//! These are called by the std::fs wrapper module

use crate::error::RuntimeError;
use crate::position::Position;
use crate::runtime_result::RuntimeResult;
use crate::values::{Value, XenithString};
use std::fs;

fn dummy_pos() -> Position {
    Position::new(0, 0, 0, "", "")
}

pub fn read(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 1 {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                "__fs_read expects 1 argument (path)",
                None,
            )
            .base,
        );
    }

    let path = match &args[0] {
        Value::String(s) => &s.value,
        _ => {
            return RuntimeResult::new().failure(
                RuntimeError::new(
                    dummy_pos(),
                    dummy_pos(),
                    "__fs_read: argument must be a string",
                    None,
                )
                .base,
            );
        }
    };

    match fs::read_to_string(path) {
        Ok(content) => RuntimeResult::new().success(Value::String(XenithString::new(content))),
        Err(e) => RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                &format!("Failed to read file '{}': {}", path, e),
                None,
            )
            .base,
        ),
    }
}

pub fn write(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 2 {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                "__fs_write expects 2 arguments (path, content)",
                None,
            )
            .base,
        );
    }

    let path = match &args[0] {
        Value::String(s) => &s.value,
        _ => {
            return RuntimeResult::new().failure(
                RuntimeError::new(
                    dummy_pos(),
                    dummy_pos(),
                    "__fs_write: first argument must be a string (path)",
                    None,
                )
                .base,
            );
        }
    };

    let content = match &args[1] {
        Value::String(s) => &s.value,
        _ => {
            return RuntimeResult::new().failure(
                RuntimeError::new(
                    dummy_pos(),
                    dummy_pos(),
                    "__fs_write: second argument must be a string (content)",
                    None,
                )
                .base,
            );
        }
    };

    match fs::write(path, content) {
        Ok(_) => RuntimeResult::new().success(Value::Number(crate::values::Number::null())),
        Err(e) => RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                &format!("Failed to write to file '{}': {}", path, e),
                None,
            )
            .base,
        ),
    }
}

pub fn exists(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 1 {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                "__fs_exists expects 1 argument (path)",
                None,
            )
            .base,
        );
    }

    let path = match &args[0] {
        Value::String(s) => &s.value,
        _ => {
            return RuntimeResult::new().failure(
                RuntimeError::new(
                    dummy_pos(),
                    dummy_pos(),
                    "__fs_exists: argument must be a string",
                    None,
                )
                .base,
            );
        }
    };

    let exists = fs::metadata(path).is_ok();
    let value = if exists { 1.0 } else { 0.0 };
    RuntimeResult::new().success(Value::Number(crate::values::Number::new(value)))
}
