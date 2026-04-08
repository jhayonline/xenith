//! .env file built-in functions
//! These are called by the std::dotenv wrapper module

use crate::error::RuntimeError;
use crate::position::Position;
use crate::runtime_result::RuntimeResult;
use crate::values::{Map, Number, Value, XenithString};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Mutex;

static ENV_VARS: Lazy<Mutex<HashMap<String, String>>> = Lazy::new(|| Mutex::new(HashMap::new()));

fn dummy_pos() -> Position {
    Position::new(0, 0, 0, "", "")
}

pub fn load(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 0 {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                "__dotenv_load expects 0 arguments",
                None,
            )
            .base,
        );
    }

    match dotenv::dotenv() {
        Ok(path) => {
            // Reload into our cache
            let vars = std::env::vars().collect();
            let mut cache = ENV_VARS.lock().unwrap();
            *cache = vars;
            RuntimeResult::new().success(Value::Number(Number::null()))
        }
        Err(e) => RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                &format!("Failed to load .env: {}", e),
                None,
            )
            .base,
        ),
    }
}

pub fn load_file(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 1 {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                "__dotenv_load_file expects 1 argument (path)",
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
                    "__dotenv_load_file: argument must be a string",
                    None,
                )
                .base,
            );
        }
    };

    match dotenv::from_filename(path) {
        Ok(_) => {
            let vars = std::env::vars().collect();
            let mut cache = ENV_VARS.lock().unwrap();
            *cache = vars;
            RuntimeResult::new().success(Value::Number(Number::null()))
        }
        Err(e) => RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                &format!("Failed to load .env file '{}': {}", path, e),
                None,
            )
            .base,
        ),
    }
}

pub fn get(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 1 {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                "__dotenv_get expects 1 argument (key)",
                None,
            )
            .base,
        );
    }

    let key = match &args[0] {
        Value::String(s) => &s.value,
        _ => {
            return RuntimeResult::new().failure(
                RuntimeError::new(
                    dummy_pos(),
                    dummy_pos(),
                    "__dotenv_get: argument must be a string",
                    None,
                )
                .base,
            );
        }
    };

    let cache = ENV_VARS.lock().unwrap();
    match cache.get(key) {
        Some(value) => {
            RuntimeResult::new().success(Value::String(XenithString::new(value.clone())))
        }
        None => RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                &format!("Environment variable '{}' not found", key),
                None,
            )
            .base,
        ),
    }
}

pub fn get_or_default(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 2 {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                "__dotenv_get_or_default expects 2 arguments (key, default)",
                None,
            )
            .base,
        );
    }

    let key = match &args[0] {
        Value::String(s) => &s.value,
        _ => {
            return RuntimeResult::new().failure(
                RuntimeError::new(
                    dummy_pos(),
                    dummy_pos(),
                    "__dotenv_get_or_default: first argument must be a string",
                    None,
                )
                .base,
            );
        }
    };

    let default = match &args[1] {
        Value::String(s) => s.value.clone(),
        _ => {
            return RuntimeResult::new().failure(
                RuntimeError::new(
                    dummy_pos(),
                    dummy_pos(),
                    "__dotenv_get_or_default: second argument must be a string",
                    None,
                )
                .base,
            );
        }
    };

    let cache = ENV_VARS.lock().unwrap();
    match cache.get(key) {
        Some(value) => {
            RuntimeResult::new().success(Value::String(XenithString::new(value.clone())))
        }
        None => RuntimeResult::new().success(Value::String(XenithString::new(default))),
    }
}

pub fn has(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 1 {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                "__dotenv_has expects 1 argument (key)",
                None,
            )
            .base,
        );
    }

    let key = match &args[0] {
        Value::String(s) => &s.value,
        _ => {
            return RuntimeResult::new().failure(
                RuntimeError::new(
                    dummy_pos(),
                    dummy_pos(),
                    "__dotenv_has: argument must be a string",
                    None,
                )
                .base,
            );
        }
    };

    let cache = ENV_VARS.lock().unwrap();
    RuntimeResult::new().success(Value::Bool(cache.contains_key(key)))
}

pub fn set(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 2 {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                "__dotenv_set expects 2 arguments (key, value)",
                None,
            )
            .base,
        );
    }

    let key = match &args[0] {
        Value::String(s) => s.value.clone(),
        _ => {
            return RuntimeResult::new().failure(
                RuntimeError::new(
                    dummy_pos(),
                    dummy_pos(),
                    "__dotenv_set: first argument must be a string",
                    None,
                )
                .base,
            );
        }
    };

    let value = match &args[1] {
        Value::String(s) => s.value.clone(),
        _ => {
            return RuntimeResult::new().failure(
                RuntimeError::new(
                    dummy_pos(),
                    dummy_pos(),
                    "__dotenv_set: second argument must be a string",
                    None,
                )
                .base,
            );
        }
    };

    unsafe {
        std::env::set_var(&key, &value);
    }
    let mut cache = ENV_VARS.lock().unwrap();
    cache.insert(key, value);
    RuntimeResult::new().success(Value::Number(Number::null()))
}

pub fn unset(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 1 {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                "__dotenv_unset expects 1 argument (key)",
                None,
            )
            .base,
        );
    }

    let key = match &args[0] {
        Value::String(s) => &s.value,
        _ => {
            return RuntimeResult::new().failure(
                RuntimeError::new(
                    dummy_pos(),
                    dummy_pos(),
                    "__dotenv_unset: argument must be a string",
                    None,
                )
                .base,
            );
        }
    };

    unsafe {
        std::env::remove_var(key);
    }
    let mut cache = ENV_VARS.lock().unwrap();
    cache.remove(key);
    RuntimeResult::new().success(Value::Number(Number::null()))
}

pub fn vars(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 0 {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                "__dotenv_vars expects 0 arguments",
                None,
            )
            .base,
        );
    }

    let cache = ENV_VARS.lock().unwrap();
    let mut map = Map::new();
    for (k, v) in cache.iter() {
        map.set(k.clone(), Value::String(XenithString::new(v.clone())));
    }
    RuntimeResult::new().success(Value::Map(map))
}
