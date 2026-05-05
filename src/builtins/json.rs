//! JSON built-in functions
//! These are called by the std::json wrapper module

use crate::error::{Error, RuntimeError};
use crate::json::Json;
use crate::position::Position;
use crate::runtime_result::RuntimeResult;
use crate::values::{List, Map, Number, Value, XenithString};
use std::collections::HashMap;

// Convert Xenith Value to Json
fn value_to_json(value: &Value, call_pos: Position) -> Result<Json, Error> {
    match value {
        Value::Number(n) => {
            if n.value == 0.0 {
                Ok(Json::Null)
            } else {
                Ok(Json::Number(n.value))
            }
        }
        Value::String(s) => Ok(Json::String(s.value.clone())),
        Value::Bool(b) => Ok(Json::Bool(*b)),
        Value::List(l) => {
            let mut arr = Vec::new();
            for elem in &l.elements {
                arr.push(value_to_json(elem, call_pos.clone())?);
            }
            Ok(Json::Array(arr))
        }
        Value::Map(m) => {
            let mut obj = HashMap::new();
            for (k, v) in &m.pairs {
                obj.insert(k.clone(), value_to_json(v, call_pos.clone())?);
            }
            Ok(Json::Object(obj))
        }
        Value::Json(j) => Ok(j.clone()),
        _ => Err(Error::invalid_conversion(
            "value",
            "json",
            call_pos.clone(),
            call_pos,
        )),
    }
}

// Convert Json to Xenith Value
fn json_to_value(json: &Json) -> Value {
    match json {
        Json::Null => Value::Number(Number::null()),
        Json::Bool(b) => Value::Bool(*b),
        Json::Number(n) => Value::Number(Number::new(*n)),
        Json::String(s) => Value::String(XenithString::new(s.clone())),
        Json::Array(arr) => {
            let elements: Vec<Value> = arr.iter().map(json_to_value).collect();
            Value::List(List::new(elements))
        }
        Json::Object(obj) => {
            let mut map = Map::new();
            for (k, v) in obj {
                map.set(k.clone(), json_to_value(v));
            }
            Value::Map(map)
        }
    }
}

pub fn parse(args: Vec<Value>, call_pos: Position) -> RuntimeResult {
    if args.len() != 1 {
        return RuntimeResult::new().failure(
            Error::new(
                call_pos.clone(),
                call_pos,
                "Argument Error",
                "__json_parse expects 1 argument",
            )
            .with_code("XEN100"),
        );
    }

    match &args[0] {
        Value::String(s) => {
            let json_val: serde_json::Value = match serde_json::from_str(&s.value) {
                Ok(v) => v,
                Err(e) => {
                    return RuntimeResult::new().failure(Error::invalid_json(
                        &e.to_string(),
                        call_pos.clone(),
                        call_pos,
                    ));
                }
            };
            RuntimeResult::new().success(Value::Json(Json::from(json_val)))
        }
        Value::Map(m) => match value_to_json(&Value::Map(m.clone()), call_pos.clone()) {
            Ok(json_val) => RuntimeResult::new().success(Value::Json(json_val)),
            Err(e) => RuntimeResult::new().failure(e),
        },
        _ => RuntimeResult::new().failure(Error::type_mismatch(
            "string or map",
            "other",
            call_pos.clone(),
            call_pos,
        )),
    }
}

pub fn stringify(args: Vec<Value>, call_pos: Position) -> RuntimeResult {
    if args.len() != 1 {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                call_pos.clone(),
                call_pos,
                "__json_stringify expects 1 argument",
                None,
            )
            .base,
        );
    }

    let json_val = match &args[0] {
        Value::Json(j) => j.clone(),
        other => match value_to_json(other, call_pos.clone()) {
            Ok(j) => j,
            Err(e) => return RuntimeResult::new().failure(e),
        },
    };

    RuntimeResult::new().success(Value::String(XenithString::new(json_val.to_string())))
}

pub fn stringify_pretty(args: Vec<Value>, call_pos: Position) -> RuntimeResult {
    if args.len() != 1 {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                call_pos.clone(),
                call_pos,
                "__json_stringify_pretty expects 1 argument",
                None,
            )
            .base,
        );
    }

    let json_val = match &args[0] {
        Value::Json(j) => j.clone(),
        other => match value_to_json(other, call_pos.clone()) {
            Ok(j) => j,
            Err(e) => return RuntimeResult::new().failure(e),
        },
    };

    RuntimeResult::new().success(Value::String(XenithString::new(
        json_val.to_string_pretty(0),
    )))
}

pub fn from_map(args: Vec<Value>, call_pos: Position) -> RuntimeResult {
    if args.len() != 1 {
        return RuntimeResult::new().failure(
            Error::new(
                call_pos.clone(),
                call_pos,
                "Argument Error",
                "__json_from_map expects 1 argument (map)",
            )
            .with_code("XEN100"),
        );
    }

    match &args[0] {
        Value::Map(m) => match value_to_json(&Value::Map(m.clone()), call_pos.clone()) {
            Ok(json_val) => RuntimeResult::new().success(Value::Json(json_val)),
            Err(e) => RuntimeResult::new().failure(e),
        },
        _ => RuntimeResult::new().failure(Error::type_mismatch(
            "map",
            "other",
            call_pos.clone(),
            call_pos,
        )),
    }
}

pub fn get(args: Vec<Value>, call_pos: Position) -> RuntimeResult {
    if args.len() != 3 {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                call_pos.clone(),
                call_pos,
                "__json_get expects 3 arguments (json, key, default)",
                None,
            )
            .base,
        );
    }

    let json_val = match &args[0] {
        Value::Json(j) => j,
        _ => {
            return RuntimeResult::new().failure(
                RuntimeError::new(
                    call_pos.clone(),
                    call_pos,
                    "__json_get: first argument must be json",
                    None,
                )
                .base,
            );
        }
    };

    let key = match &args[1] {
        Value::String(s) => &s.value,
        _ => {
            return RuntimeResult::new().failure(
                RuntimeError::new(
                    call_pos.clone(),
                    call_pos,
                    "__json_get: second argument must be a string",
                    None,
                )
                .base,
            );
        }
    };

    match json_val {
        Json::Object(obj) => {
            if let Some(value) = obj.get(key) {
                RuntimeResult::new().success(Value::Json(value.clone()))
            } else {
                match value_to_json(&args[2], call_pos.clone()) {
                    Ok(default_json) => RuntimeResult::new().success(Value::Json(default_json)),
                    Err(e) => RuntimeResult::new().failure(e),
                }
            }
        }
        _ => RuntimeResult::new().failure(
            RuntimeError::new(
                call_pos.clone(),
                call_pos,
                "__json_get: cannot get key from non-object json",
                None,
            )
            .base,
        ),
    }
}

pub fn set(args: Vec<Value>, call_pos: Position) -> RuntimeResult {
    if args.len() != 3 {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                call_pos.clone(),
                call_pos,
                "__json_set expects 3 arguments (json, key, value)",
                None,
            )
            .base,
        );
    }

    let json_val = match &args[0] {
        Value::Json(j) => j.clone(),
        _ => {
            return RuntimeResult::new().failure(
                RuntimeError::new(
                    call_pos.clone(),
                    call_pos,
                    "__json_set: first argument must be json",
                    None,
                )
                .base,
            );
        }
    };

    let key = match &args[1] {
        Value::String(s) => s.value.clone(),
        _ => {
            return RuntimeResult::new().failure(
                RuntimeError::new(
                    call_pos.clone(),
                    call_pos,
                    "__json_set: second argument must be a string",
                    None,
                )
                .base,
            );
        }
    };

    let value = match value_to_json(&args[2], call_pos.clone()) {
        Ok(v) => v,
        Err(e) => return RuntimeResult::new().failure(e),
    };

    match json_val {
        Json::Object(mut obj) => {
            obj.insert(key, value);
            RuntimeResult::new().success(Value::Json(Json::Object(obj)))
        }
        _ => RuntimeResult::new().failure(
            RuntimeError::new(
                call_pos.clone(),
                call_pos,
                "__json_set: cannot set key on non-object json",
                None,
            )
            .base,
        ),
    }
}

pub fn has_key(args: Vec<Value>, call_pos: Position) -> RuntimeResult {
    if args.len() != 2 {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                call_pos.clone(),
                call_pos,
                "__json_has_key expects 2 arguments (json, key)",
                None,
            )
            .base,
        );
    }

    let json_val = match &args[0] {
        Value::Json(j) => j,
        _ => {
            return RuntimeResult::new().failure(
                RuntimeError::new(
                    call_pos.clone(),
                    call_pos,
                    "__json_has_key: first argument must be json",
                    None,
                )
                .base,
            );
        }
    };

    let key = match &args[1] {
        Value::String(s) => &s.value,
        _ => {
            return RuntimeResult::new().failure(
                RuntimeError::new(
                    call_pos.clone(),
                    call_pos,
                    "__json_has_key: second argument must be a string",
                    None,
                )
                .base,
            );
        }
    };

    match json_val {
        Json::Object(obj) => RuntimeResult::new().success(Value::Bool(obj.contains_key(key))),
        _ => RuntimeResult::new().success(Value::Bool(false)),
    }
}

pub fn null_value(_args: Vec<Value>, _call_pos: Position) -> RuntimeResult {
    RuntimeResult::new().success(Value::Json(Json::Null))
}
