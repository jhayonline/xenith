//! String utility built-in functions
//! These are called by the std::string wrapper module

use crate::error::{Error, RuntimeError};
use crate::position::Position;
use crate::runtime_result::RuntimeResult;
use crate::values::{List, Value, XenithString};

fn dummy_pos() -> Position {
    Position::new(0, 0, 0, "", "")
}

fn get_string_arg(args: &[Value], index: usize) -> Result<String, Error> {
    match &args[index] {
        Value::String(s) => Ok(s.value.clone()),
        _ => Err(Error::type_mismatch(
            "string",
            "other",
            dummy_pos(),
            dummy_pos(),
        )),
    }
}

pub fn split(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 2 {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                "__string_split expects 2 arguments (text, delimiter)",
                None,
            )
            .base,
        );
    }

    let text = match get_string_arg(&args, 0) {
        Ok(s) => s,
        Err(e) => return RuntimeResult::new().failure(e),
    };

    let delimiter = match get_string_arg(&args, 1) {
        Ok(s) => s,
        Err(e) => return RuntimeResult::new().failure(e),
    };

    let parts: Vec<Value> = if delimiter.is_empty() {
        text.chars()
            .map(|c| Value::String(XenithString::new(c.to_string())))
            .collect()
    } else {
        text.split(&delimiter)
            .map(|s| Value::String(XenithString::new(s.to_string())))
            .collect()
    };

    RuntimeResult::new().success(Value::List(List::new(parts)))
}

pub fn join(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 2 {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                "__string_join expects 2 arguments (list, separator)",
                None,
            )
            .base,
        );
    }

    let strings = match &args[0] {
        Value::List(list) => list,
        _ => {
            return RuntimeResult::new().failure(
                RuntimeError::new(
                    dummy_pos(),
                    dummy_pos(),
                    "__string_join: first argument must be a list of strings",
                    None,
                )
                .base,
            );
        }
    };

    let separator = match get_string_arg(&args, 1) {
        Ok(s) => s,
        Err(e) => return RuntimeResult::new().failure(e),
    };

    let mut result = String::new();
    for (i, elem) in strings.elements.iter().enumerate() {
        match elem {
            Value::String(s) => {
                if i > 0 {
                    result.push_str(&separator);
                }
                result.push_str(&s.value);
            }
            _ => {
                return RuntimeResult::new().failure(
                    RuntimeError::new(
                        dummy_pos(),
                        dummy_pos(),
                        "__string_join: list must contain only strings",
                        None,
                    )
                    .base,
                );
            }
        }
    }

    RuntimeResult::new().success(Value::String(XenithString::new(result)))
}

pub fn trim(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 1 {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                "__string_trim expects 1 argument (text)",
                None,
            )
            .base,
        );
    }

    let text = match get_string_arg(&args, 0) {
        Ok(s) => s,
        Err(e) => return RuntimeResult::new().failure(e),
    };

    RuntimeResult::new().success(Value::String(XenithString::new(text.trim().to_string())))
}

pub fn trim_start(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 1 {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                "__string_trim_start expects 1 argument (text)",
                None,
            )
            .base,
        );
    }

    let text = match get_string_arg(&args, 0) {
        Ok(s) => s,
        Err(e) => return RuntimeResult::new().failure(e),
    };

    RuntimeResult::new().success(Value::String(XenithString::new(
        text.trim_start().to_string(),
    )))
}

pub fn trim_end(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 1 {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                "__string_trim_end expects 1 argument (text)",
                None,
            )
            .base,
        );
    }

    let text = match get_string_arg(&args, 0) {
        Ok(s) => s,
        Err(e) => return RuntimeResult::new().failure(e),
    };

    RuntimeResult::new().success(Value::String(XenithString::new(
        text.trim_end().to_string(),
    )))
}

pub fn replace(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 3 {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                "__string_replace expects 3 arguments (text, from, to)",
                None,
            )
            .base,
        );
    }

    let text = match get_string_arg(&args, 0) {
        Ok(s) => s,
        Err(e) => return RuntimeResult::new().failure(e),
    };

    let from = match get_string_arg(&args, 1) {
        Ok(s) => s,
        Err(e) => return RuntimeResult::new().failure(e),
    };

    let to = match get_string_arg(&args, 2) {
        Ok(s) => s,
        Err(e) => return RuntimeResult::new().failure(e),
    };

    RuntimeResult::new().success(Value::String(XenithString::new(text.replace(&from, &to))))
}

pub fn contains(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 2 {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                "__string_contains expects 2 arguments (text, substring)",
                None,
            )
            .base,
        );
    }

    let text = match get_string_arg(&args, 0) {
        Ok(s) => s,
        Err(e) => return RuntimeResult::new().failure(e),
    };

    let substring = match get_string_arg(&args, 1) {
        Ok(s) => s,
        Err(e) => return RuntimeResult::new().failure(e),
    };

    RuntimeResult::new().success(Value::Bool(text.contains(&substring)))
}

pub fn starts_with(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 2 {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                "__string_starts_with expects 2 arguments (text, prefix)",
                None,
            )
            .base,
        );
    }

    let text = match get_string_arg(&args, 0) {
        Ok(s) => s,
        Err(e) => return RuntimeResult::new().failure(e),
    };

    let prefix = match get_string_arg(&args, 1) {
        Ok(s) => s,
        Err(e) => return RuntimeResult::new().failure(e),
    };

    RuntimeResult::new().success(Value::Bool(text.starts_with(&prefix)))
}

pub fn ends_with(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 2 {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                "__string_ends_with expects 2 arguments (text, suffix)",
                None,
            )
            .base,
        );
    }

    let text = match get_string_arg(&args, 0) {
        Ok(s) => s,
        Err(e) => return RuntimeResult::new().failure(e),
    };

    let suffix = match get_string_arg(&args, 1) {
        Ok(s) => s,
        Err(e) => return RuntimeResult::new().failure(e),
    };

    RuntimeResult::new().success(Value::Bool(text.ends_with(&suffix)))
}

pub fn to_upper(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 1 {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                "__string_to_upper expects 1 argument (text)",
                None,
            )
            .base,
        );
    }

    let text = match get_string_arg(&args, 0) {
        Ok(s) => s,
        Err(e) => return RuntimeResult::new().failure(e),
    };

    RuntimeResult::new().success(Value::String(XenithString::new(text.to_uppercase())))
}

pub fn to_lower(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 1 {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                "__string_to_lower expects 1 argument (text)",
                None,
            )
            .base,
        );
    }

    let text = match get_string_arg(&args, 0) {
        Ok(s) => s,
        Err(e) => return RuntimeResult::new().failure(e),
    };

    RuntimeResult::new().success(Value::String(XenithString::new(text.to_lowercase())))
}

pub fn reverse(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 1 {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                "__string_reverse expects 1 argument (text)",
                None,
            )
            .base,
        );
    }

    let text = match get_string_arg(&args, 0) {
        Ok(s) => s,
        Err(e) => return RuntimeResult::new().failure(e),
    };

    let reversed: String = text.chars().rev().collect();
    RuntimeResult::new().success(Value::String(XenithString::new(reversed)))
}
