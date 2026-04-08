//! Path manipulation built-in functions
//! These are called by the std::path wrapper module

use crate::error::RuntimeError;
use crate::position::Position;
use crate::runtime_result::RuntimeResult;
use crate::values::{List, Value, XenithString};
use std::path::{Path, PathBuf};

fn dummy_pos() -> Position {
    Position::new(0, 0, 0, "", "")
}

fn value_to_string(value: &Value) -> Result<String, RuntimeError> {
    match value {
        Value::String(s) => Ok(s.value.clone()),
        _ => Err(RuntimeError::new(
            dummy_pos(),
            dummy_pos(),
            "Expected string argument",
            None,
        )),
    }
}

fn value_to_string_list(value: &Value) -> Result<Vec<String>, RuntimeError> {
    match value {
        Value::List(list) => {
            let mut strings = Vec::new();
            for elem in &list.elements {
                match elem {
                    Value::String(s) => strings.push(s.value.clone()),
                    _ => {
                        return Err(RuntimeError::new(
                            dummy_pos(),
                            dummy_pos(),
                            "Expected list of strings",
                            None,
                        ));
                    }
                }
            }
            Ok(strings)
        }
        _ => Err(RuntimeError::new(
            dummy_pos(),
            dummy_pos(),
            "Expected list argument",
            None,
        )),
    }
}

pub fn join(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 1 {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                "__path_join expects 1 argument (list of parts)",
                None,
            )
            .base,
        );
    }

    let parts = match value_to_string_list(&args[0]) {
        Ok(p) => p,
        Err(e) => return RuntimeResult::new().failure(e.base),
    };

    if parts.is_empty() {
        return RuntimeResult::new().success(Value::String(XenithString::new("".to_string())));
    }

    let mut path = PathBuf::new();
    for part in parts {
        path = path.join(part);
    }

    RuntimeResult::new().success(Value::String(XenithString::new(
        path.to_string_lossy().to_string(),
    )))
}

pub fn basename(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 1 {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                "__path_basename expects 1 argument (path)",
                None,
            )
            .base,
        );
    }

    let path_str = match value_to_string(&args[0]) {
        Ok(s) => s,
        Err(e) => return RuntimeResult::new().failure(e.base),
    };

    let path = Path::new(&path_str);
    let basename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

    RuntimeResult::new().success(Value::String(XenithString::new(basename.to_string())))
}

pub fn dirname(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 1 {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                "__path_dirname expects 1 argument (path)",
                None,
            )
            .base,
        );
    }

    let path_str = match value_to_string(&args[0]) {
        Ok(s) => s,
        Err(e) => return RuntimeResult::new().failure(e.base),
    };

    let path = Path::new(&path_str);
    let dirname = path.parent().and_then(|p| p.to_str()).unwrap_or("");

    RuntimeResult::new().success(Value::String(XenithString::new(dirname.to_string())))
}

pub fn extension(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 1 {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                "__path_extension expects 1 argument (path)",
                None,
            )
            .base,
        );
    }

    let path_str = match value_to_string(&args[0]) {
        Ok(s) => s,
        Err(e) => return RuntimeResult::new().failure(e.base),
    };

    let path = Path::new(&path_str);
    let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");

    RuntimeResult::new().success(Value::String(XenithString::new(extension.to_string())))
}

pub fn stem(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 1 {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                "__path_stem expects 1 argument (path)",
                None,
            )
            .base,
        );
    }

    let path_str = match value_to_string(&args[0]) {
        Ok(s) => s,
        Err(e) => return RuntimeResult::new().failure(e.base),
    };

    let path = Path::new(&path_str);
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");

    RuntimeResult::new().success(Value::String(XenithString::new(stem.to_string())))
}

pub fn is_absolute(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 1 {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                "__path_is_absolute expects 1 argument (path)",
                None,
            )
            .base,
        );
    }

    let path_str = match value_to_string(&args[0]) {
        Ok(s) => s,
        Err(e) => return RuntimeResult::new().failure(e.base),
    };

    let path = Path::new(&path_str);
    RuntimeResult::new().success(Value::Bool(path.is_absolute()))
}

pub fn is_relative(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 1 {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                "__path_is_relative expects 1 argument (path)",
                None,
            )
            .base,
        );
    }

    let path_str = match value_to_string(&args[0]) {
        Ok(s) => s,
        Err(e) => return RuntimeResult::new().failure(e.base),
    };

    let path = Path::new(&path_str);
    RuntimeResult::new().success(Value::Bool(path.is_relative()))
}

pub fn absolute(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 1 {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                "__path_absolute expects 1 argument (path)",
                None,
            )
            .base,
        );
    }

    let path_str = match value_to_string(&args[0]) {
        Ok(s) => s,
        Err(e) => return RuntimeResult::new().failure(e.base),
    };

    let path = Path::new(&path_str);
    match path.canonicalize() {
        Ok(abs_path) => RuntimeResult::new().success(Value::String(XenithString::new(
            abs_path.to_string_lossy().to_string(),
        ))),
        Err(e) => RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                &format!("Failed to get absolute path: {}", e),
                None,
            )
            .base,
        ),
    }
}

pub fn normalize(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 1 {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                "__path_normalize expects 1 argument (path)",
                None,
            )
            .base,
        );
    }

    let path_str = match value_to_string(&args[0]) {
        Ok(s) => s,
        Err(e) => return RuntimeResult::new().failure(e.base),
    };

    let path = Path::new(&path_str);
    let mut components = Vec::new();

    for component in path.components() {
        match component {
            std::path::Component::Normal(c) => components.push(c.to_string_lossy().to_string()),
            std::path::Component::ParentDir => {
                components.pop();
            }
            std::path::Component::CurDir => {}
            std::path::Component::RootDir => {
                components.clear();
                components.push("/".to_string());
            }
            std::path::Component::Prefix(_) => {
                components.push(component.as_os_str().to_string_lossy().to_string());
            }
        }
    }

    let result = if components.is_empty() {
        ".".to_string()
    } else {
        components.join(if cfg!(windows) { "\\" } else { "/" })
    };

    RuntimeResult::new().success(Value::String(XenithString::new(result)))
}

pub fn components(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 1 {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                "__path_components expects 1 argument (path)",
                None,
            )
            .base,
        );
    }

    let path_str = match value_to_string(&args[0]) {
        Ok(s) => s,
        Err(e) => return RuntimeResult::new().failure(e.base),
    };

    let path = Path::new(&path_str);
    let mut components = Vec::new();

    for component in path.components() {
        let comp_str = component.as_os_str().to_string_lossy().to_string();
        if !comp_str.is_empty() {
            components.push(Value::String(XenithString::new(comp_str)));
        }
    }

    RuntimeResult::new().success(Value::List(List::new(components)))
}

pub fn parent(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 1 {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                "__path_parent expects 1 argument (path)",
                None,
            )
            .base,
        );
    }

    let path_str = match value_to_string(&args[0]) {
        Ok(s) => s,
        Err(e) => return RuntimeResult::new().failure(e.base),
    };

    let path = Path::new(&path_str);
    let parent = path.parent().and_then(|p| p.to_str()).unwrap_or("");

    RuntimeResult::new().success(Value::String(XenithString::new(parent.to_string())))
}
