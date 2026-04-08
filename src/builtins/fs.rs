//! File system built-in functions
//! These are called by the std::fs wrapper module

use crate::error::Error;
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
            Error::new(
                dummy_pos(),
                dummy_pos(),
                "Argument Error",
                "__fs_read expects 1 argument (path)",
            )
            .with_code("XEN100"),
        );
    }

    let path = match &args[0] {
        Value::String(s) => &s.value,
        _ => {
            return RuntimeResult::new().failure(Error::type_mismatch(
                "string",
                "other",
                dummy_pos(),
                dummy_pos(),
            ));
        }
    };

    match fs::read_to_string(path) {
        Ok(content) => RuntimeResult::new().success(Value::String(XenithString::new(content))),
        Err(_) => {
            RuntimeResult::new().failure(Error::file_not_found(path, dummy_pos(), dummy_pos()))
        }
    }
}

pub fn write(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 2 {
        return RuntimeResult::new().failure(
            Error::new(
                dummy_pos(),
                dummy_pos(),
                "Argument Error",
                "__fs_write expects 2 arguments (path, content)",
            )
            .with_code("XEN100"),
        );
    }

    let path = match &args[0] {
        Value::String(s) => &s.value,
        _ => {
            return RuntimeResult::new().failure(Error::type_mismatch(
                "string",
                "other",
                dummy_pos(),
                dummy_pos(),
            ));
        }
    };

    let content = match &args[1] {
        Value::String(s) => &s.value,
        _ => {
            return RuntimeResult::new().failure(Error::type_mismatch(
                "string",
                "other",
                dummy_pos(),
                dummy_pos(),
            ));
        }
    };

    match fs::write(path, content) {
        Ok(_) => RuntimeResult::new().success(Value::Number(Number::null())),
        Err(e) => RuntimeResult::new().failure(
            Error::permission_denied(path, dummy_pos(), dummy_pos()).with_note(&e.to_string()),
        ),
    }
}

pub fn append(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 2 {
        return RuntimeResult::new().failure(
            Error::new(
                dummy_pos(),
                dummy_pos(),
                "Argument Error",
                "__fs_append expects 2 arguments (path, content)",
            )
            .with_code("XEN100"),
        );
    }

    let path = match &args[0] {
        Value::String(s) => &s.value,
        _ => {
            return RuntimeResult::new().failure(Error::type_mismatch(
                "string",
                "other",
                dummy_pos(),
                dummy_pos(),
            ));
        }
    };

    let content = match &args[1] {
        Value::String(s) => &s.value,
        _ => {
            return RuntimeResult::new().failure(Error::type_mismatch(
                "string",
                "other",
                dummy_pos(),
                dummy_pos(),
            ));
        }
    };

    match fs::OpenOptions::new().write(true).append(true).open(path) {
        Ok(mut file) => {
            use std::io::Write;
            match file.write_all(content.as_bytes()) {
                Ok(_) => RuntimeResult::new().success(Value::Number(Number::null())),
                Err(e) => RuntimeResult::new().failure(
                    Error::permission_denied(path, dummy_pos(), dummy_pos())
                        .with_note(&e.to_string()),
                ),
            }
        }
        Err(e) => RuntimeResult::new().failure(
            Error::permission_denied(path, dummy_pos(), dummy_pos()).with_note(&e.to_string()),
        ),
    }
}

pub fn exists(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 1 {
        return RuntimeResult::new().failure(
            Error::new(
                dummy_pos(),
                dummy_pos(),
                "Argument Error",
                "__fs_exists expects 1 argument (path)",
            )
            .with_code("XEN100"),
        );
    }

    let path = match &args[0] {
        Value::String(s) => &s.value,
        _ => {
            return RuntimeResult::new().failure(Error::type_mismatch(
                "string",
                "other",
                dummy_pos(),
                dummy_pos(),
            ));
        }
    };

    let exists = Path::new(path).exists();
    RuntimeResult::new().success(Value::Bool(exists))
}

pub fn is_file(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 1 {
        return RuntimeResult::new().failure(
            Error::new(
                dummy_pos(),
                dummy_pos(),
                "Argument Error",
                "__fs_is_file expects 1 argument (path)",
            )
            .with_code("XEN100"),
        );
    }

    let path = match &args[0] {
        Value::String(s) => &s.value,
        _ => {
            return RuntimeResult::new().failure(Error::type_mismatch(
                "string",
                "other",
                dummy_pos(),
                dummy_pos(),
            ));
        }
    };

    let is_file = Path::new(path).is_file();
    RuntimeResult::new().success(Value::Bool(is_file))
}

pub fn is_dir(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 1 {
        return RuntimeResult::new().failure(
            Error::new(
                dummy_pos(),
                dummy_pos(),
                "Argument Error",
                "__fs_is_dir expects 1 argument (path)",
            )
            .with_code("XEN100"),
        );
    }

    let path = match &args[0] {
        Value::String(s) => &s.value,
        _ => {
            return RuntimeResult::new().failure(Error::type_mismatch(
                "string",
                "other",
                dummy_pos(),
                dummy_pos(),
            ));
        }
    };

    let is_dir = Path::new(path).is_dir();
    RuntimeResult::new().success(Value::Bool(is_dir))
}

pub fn mkdir(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 1 {
        return RuntimeResult::new().failure(
            Error::new(
                dummy_pos(),
                dummy_pos(),
                "Argument Error",
                "__fs_mkdir expects 1 argument (path)",
            )
            .with_code("XEN100"),
        );
    }

    let path = match &args[0] {
        Value::String(s) => &s.value,
        _ => {
            return RuntimeResult::new().failure(Error::type_mismatch(
                "string",
                "other",
                dummy_pos(),
                dummy_pos(),
            ));
        }
    };

    match fs::create_dir(path) {
        Ok(_) => RuntimeResult::new().success(Value::Number(Number::null())),
        Err(e) => RuntimeResult::new().failure(
            Error::permission_denied(path, dummy_pos(), dummy_pos()).with_note(&e.to_string()),
        ),
    }
}

pub fn mkdir_all(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 1 {
        return RuntimeResult::new().failure(
            Error::new(
                dummy_pos(),
                dummy_pos(),
                "Argument Error",
                "__fs_mkdir_all expects 1 argument (path)",
            )
            .with_code("XEN100"),
        );
    }

    let path = match &args[0] {
        Value::String(s) => &s.value,
        _ => {
            return RuntimeResult::new().failure(Error::type_mismatch(
                "string",
                "other",
                dummy_pos(),
                dummy_pos(),
            ));
        }
    };

    match fs::create_dir_all(path) {
        Ok(_) => RuntimeResult::new().success(Value::Number(Number::null())),
        Err(e) => RuntimeResult::new().failure(
            Error::permission_denied(path, dummy_pos(), dummy_pos()).with_note(&e.to_string()),
        ),
    }
}

pub fn remove(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 1 {
        return RuntimeResult::new().failure(
            Error::new(
                dummy_pos(),
                dummy_pos(),
                "Argument Error",
                "__fs_remove expects 1 argument (path)",
            )
            .with_code("XEN100"),
        );
    }

    let path = match &args[0] {
        Value::String(s) => &s.value,
        _ => {
            return RuntimeResult::new().failure(Error::type_mismatch(
                "string",
                "other",
                dummy_pos(),
                dummy_pos(),
            ));
        }
    };

    let metadata = match fs::metadata(path) {
        Ok(m) => m,
        Err(e) => {
            return RuntimeResult::new().failure(
                Error::file_not_found(path, dummy_pos(), dummy_pos()).with_note(&e.to_string()),
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
            Error::permission_denied(path, dummy_pos(), dummy_pos()).with_note(&e.to_string()),
        ),
    }
}

pub fn remove_all(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 1 {
        return RuntimeResult::new().failure(
            Error::new(
                dummy_pos(),
                dummy_pos(),
                "Argument Error",
                "__fs_remove_all expects 1 argument (path)",
            )
            .with_code("XEN100"),
        );
    }

    let path = match &args[0] {
        Value::String(s) => &s.value,
        _ => {
            return RuntimeResult::new().failure(Error::type_mismatch(
                "string",
                "other",
                dummy_pos(),
                dummy_pos(),
            ));
        }
    };

    match fs::remove_dir_all(path) {
        Ok(_) => RuntimeResult::new().success(Value::Number(Number::null())),
        Err(e) => RuntimeResult::new().failure(
            Error::permission_denied(path, dummy_pos(), dummy_pos()).with_note(&e.to_string()),
        ),
    }
}

pub fn list_dir(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 1 {
        return RuntimeResult::new().failure(
            Error::new(
                dummy_pos(),
                dummy_pos(),
                "Argument Error",
                "__fs_list_dir expects 1 argument (path)",
            )
            .with_code("XEN100"),
        );
    }

    let path = match &args[0] {
        Value::String(s) => &s.value,
        _ => {
            return RuntimeResult::new().failure(Error::type_mismatch(
                "string",
                "other",
                dummy_pos(),
                dummy_pos(),
            ));
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
                            Error::permission_denied(path, dummy_pos(), dummy_pos())
                                .with_note(&e.to_string()),
                        );
                    }
                }
            }
            RuntimeResult::new().success(Value::List(List::new(items)))
        }
        Err(e) => RuntimeResult::new().failure(
            Error::file_not_found(path, dummy_pos(), dummy_pos()).with_note(&e.to_string()),
        ),
    }
}

pub fn copy(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 2 {
        return RuntimeResult::new().failure(
            Error::new(
                dummy_pos(),
                dummy_pos(),
                "Argument Error",
                "__fs_copy expects 2 arguments (from, to)",
            )
            .with_code("XEN100"),
        );
    }

    let from = match &args[0] {
        Value::String(s) => &s.value,
        _ => {
            return RuntimeResult::new().failure(Error::type_mismatch(
                "string",
                "other",
                dummy_pos(),
                dummy_pos(),
            ));
        }
    };

    let to = match &args[1] {
        Value::String(s) => &s.value,
        _ => {
            return RuntimeResult::new().failure(Error::type_mismatch(
                "string",
                "other",
                dummy_pos(),
                dummy_pos(),
            ));
        }
    };

    match fs::copy(from, to) {
        Ok(_) => RuntimeResult::new().success(Value::Number(Number::null())),
        Err(e) => RuntimeResult::new().failure(
            Error::file_not_found(from, dummy_pos(), dummy_pos()).with_note(&e.to_string()),
        ),
    }
}
