//! File system built-in functions
//! These are called by the std::fs wrapper module

use crate::error::RuntimeError;
use crate::position::Position;
use crate::runtime_result::RuntimeResult;
use crate::values::{List, Number, Value, XenithString};
use std::fs;
use std::path::Path;

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
        Ok(_) => RuntimeResult::new().success(Value::Number(Number::null())),
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

pub fn append(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 2 {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                "__fs_append expects 2 arguments (path, content)",
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
                    "__fs_append: first argument must be a string (path)",
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
                    "__fs_append: second argument must be a string (content)",
                    None,
                )
                .base,
            );
        }
    };

    match fs::OpenOptions::new().write(true).append(true).open(path) {
        Ok(mut file) => {
            use std::io::Write;
            match file.write_all(content.as_bytes()) {
                Ok(_) => RuntimeResult::new().success(Value::Number(Number::null())),
                Err(e) => RuntimeResult::new().failure(
                    RuntimeError::new(
                        dummy_pos(),
                        dummy_pos(),
                        &format!("Failed to append to file '{}': {}", path, e),
                        None,
                    )
                    .base,
                ),
            }
        }
        Err(e) => RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                &format!("Failed to open file '{}' for appending: {}", path, e),
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

    let exists = Path::new(path).exists();
    RuntimeResult::new().success(Value::Bool(exists))
}

pub fn is_file(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 1 {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                "__fs_is_file expects 1 argument (path)",
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
                    "__fs_is_file: argument must be a string",
                    None,
                )
                .base,
            );
        }
    };

    let is_file = Path::new(path).is_file();
    RuntimeResult::new().success(Value::Bool(is_file))
}

pub fn is_dir(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 1 {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                "__fs_is_dir expects 1 argument (path)",
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
                    "__fs_is_dir: argument must be a string",
                    None,
                )
                .base,
            );
        }
    };

    let is_dir = Path::new(path).is_dir();
    RuntimeResult::new().success(Value::Bool(is_dir))
}

pub fn mkdir(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 1 {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                "__fs_mkdir expects 1 argument (path)",
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
                    "__fs_mkdir: argument must be a string",
                    None,
                )
                .base,
            );
        }
    };

    match fs::create_dir(path) {
        Ok(_) => RuntimeResult::new().success(Value::Number(Number::null())),
        Err(e) => RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                &format!("Failed to create directory '{}': {}", path, e),
                None,
            )
            .base,
        ),
    }
}

pub fn mkdir_all(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 1 {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                "__fs_mkdir_all expects 1 argument (path)",
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
                    "__fs_mkdir_all: argument must be a string",
                    None,
                )
                .base,
            );
        }
    };

    match fs::create_dir_all(path) {
        Ok(_) => RuntimeResult::new().success(Value::Number(Number::null())),
        Err(e) => RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                &format!("Failed to create directory tree '{}': {}", path, e),
                None,
            )
            .base,
        ),
    }
}

pub fn remove(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 1 {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                "__fs_remove expects 1 argument (path)",
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
                    "__fs_remove: argument must be a string",
                    None,
                )
                .base,
            );
        }
    };

    let metadata = match fs::metadata(path) {
        Ok(m) => m,
        Err(e) => {
            return RuntimeResult::new().failure(
                RuntimeError::new(
                    dummy_pos(),
                    dummy_pos(),
                    &format!("Failed to remove '{}': {}", path, e),
                    None,
                )
                .base,
            );
        }
    };

    let result = if metadata.is_dir() {
        fs::remove_dir(path)
    } else {
        fs::remove_file(path)
    };

    match result {
        Ok(_) => RuntimeResult::new().success(Value::Number(Number::null())),
        Err(e) => RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                &format!("Failed to remove '{}': {}", path, e),
                None,
            )
            .base,
        ),
    }
}

pub fn remove_all(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 1 {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                "__fs_remove_all expects 1 argument (path)",
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
                    "__fs_remove_all: argument must be a string",
                    None,
                )
                .base,
            );
        }
    };

    match fs::remove_dir_all(path) {
        Ok(_) => RuntimeResult::new().success(Value::Number(Number::null())),
        Err(e) => RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                &format!("Failed to remove directory tree '{}': {}", path, e),
                None,
            )
            .base,
        ),
    }
}

pub fn list_dir(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 1 {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                "__fs_list_dir expects 1 argument (path)",
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
                    "__fs_list_dir: argument must be a string",
                    None,
                )
                .base,
            );
        }
    };

    match fs::read_dir(path) {
        Ok(entries) => {
            let mut items = Vec::new();
            for entry in entries {
                match entry {
                    Ok(entry) => {
                        if let Some(name) = entry.file_name().to_str() {
                            items.push(Value::String(XenithString::new(name.to_string())));
                        }
                    }
                    Err(e) => {
                        return RuntimeResult::new().failure(
                            RuntimeError::new(
                                dummy_pos(),
                                dummy_pos(),
                                &format!("Failed to read directory entry: {}", e),
                                None,
                            )
                            .base,
                        );
                    }
                }
            }
            RuntimeResult::new().success(Value::List(List::new(items)))
        }
        Err(e) => RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                &format!("Failed to list directory '{}': {}", path, e),
                None,
            )
            .base,
        ),
    }
}

pub fn copy(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 2 {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                "__fs_copy expects 2 arguments (from, to)",
                None,
            )
            .base,
        );
    }

    let from = match &args[0] {
        Value::String(s) => &s.value,
        _ => {
            return RuntimeResult::new().failure(
                RuntimeError::new(
                    dummy_pos(),
                    dummy_pos(),
                    "__fs_copy: first argument must be a string (source)",
                    None,
                )
                .base,
            );
        }
    };

    let to = match &args[1] {
        Value::String(s) => &s.value,
        _ => {
            return RuntimeResult::new().failure(
                RuntimeError::new(
                    dummy_pos(),
                    dummy_pos(),
                    "__fs_copy: second argument must be a string (destination)",
                    None,
                )
                .base,
            );
        }
    };

    match fs::copy(from, to) {
        Ok(_) => RuntimeResult::new().success(Value::Number(Number::null())),
        Err(e) => RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                &format!("Failed to copy '{}' to '{}': {}", from, to, e),
                None,
            )
            .base,
        ),
    }
}
