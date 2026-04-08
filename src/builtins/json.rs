//! JSON built-in functions
//! These are called by the std::json wrapper module

use crate::error::{Error, RuntimeError};
use crate::position::Position;
use crate::runtime_result::RuntimeResult;
use crate::values::{JsonValue, List, Map, Number, Value, XenithString};
use serde_json::Value as SerdeValue;

fn dummy_pos() -> Position {
    Position::new(0, 0, 0, "", "")
}

fn json_to_xenith(json_val: SerdeValue) -> Value {
    match json_val {
        SerdeValue::Null => Value::Number(Number::null()),
        SerdeValue::Bool(b) => Value::Bool(b),
        SerdeValue::Number(n) => {
            if let Some(f) = n.as_f64() {
                Value::Number(Number::new(f))
            } else {
                Value::Number(Number::null())
            }
        }
        SerdeValue::String(s) => Value::String(XenithString::new(s)),
        SerdeValue::Array(arr) => {
            let elements: Vec<Value> = arr.into_iter().map(json_to_xenith).collect();
            Value::List(List::new(elements))
        }
        SerdeValue::Object(obj) => {
            let mut map = Map::new();
            for (k, v) in obj {
                map.set(k, json_to_xenith(v));
            }
            Value::Map(map)
        }
    }
}

fn xenith_to_json(value: &Value) -> SerdeValue {
    match value {
        Value::Number(n) => {
            if n.value.fract() == 0.0 {
                SerdeValue::Number(serde_json::Number::from(n.value as i64))
            } else {
                serde_json::Number::from_f64(n.value).map_or(SerdeValue::Null, SerdeValue::Number)
            }
        }
        Value::String(s) => SerdeValue::String(s.value.clone()),
        Value::Bool(b) => SerdeValue::Bool(*b),
        Value::List(l) => {
            let arr: Vec<SerdeValue> = l.elements.iter().map(xenith_to_json).collect();
            SerdeValue::Array(arr)
        }
        Value::Map(m) => {
            let mut obj = serde_json::Map::new();
            for (k, v) in &m.pairs {
                obj.insert(k.clone(), xenith_to_json(v));
            }
            SerdeValue::Object(obj)
        }
        Value::Json(j) => j.value.clone(),
        _ => SerdeValue::Null,
    }
}

pub fn parse(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 1 {
        return RuntimeResult::new().failure(
            Error::new(
                dummy_pos(),
                dummy_pos(),
                "Argument Error",
                "__json_parse expects 1 argument",
            )
            .with_code("XEN100"),
        );
    }

    if let Value::String(s) = &args[0] {
        match serde_json::from_str(&s.value) {
            Ok(json_val) => RuntimeResult::new().success(Value::Json(JsonValue::new(json_val))),
            Err(e) => RuntimeResult::new().failure(Error::invalid_json(
                &e.to_string(),
                dummy_pos(),
                dummy_pos(),
            )),
        }
    } else if let Value::Map(m) = &args[0] {
        let json_val = xenith_to_json(&Value::Map(m.clone()));
        RuntimeResult::new().success(Value::Json(JsonValue::new(json_val)))
    } else {
        RuntimeResult::new().failure(Error::type_mismatch(
            "string or map",
            "other",
            dummy_pos(),
            dummy_pos(),
        ))
    }
}

pub fn stringify(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 1 {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                "__json_stringify expects 1 argument",
                None,
            )
            .base,
        );
    }

    let json_val = match &args[0] {
        Value::Json(j) => &j.value,
        _ => {
            return RuntimeResult::new().failure(
                RuntimeError::new(
                    dummy_pos(),
                    dummy_pos(),
                    "__json_stringify: argument must be json",
                    None,
                )
                .base,
            );
        }
    };

    match serde_json::to_string(json_val) {
        Ok(s) => RuntimeResult::new().success(Value::String(XenithString::new(s))),
        Err(e) => RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                &format!("Failed to stringify: {}", e),
                None,
            )
            .base,
        ),
    }
}

pub fn stringify_pretty(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 1 {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                "__json_stringify_pretty expects 1 argument",
                None,
            )
            .base,
        );
    }

    let json_val = xenith_to_json(&args[0]);
    match serde_json::to_string_pretty(&json_val) {
        Ok(s) => RuntimeResult::new().success(Value::String(XenithString::new(s))),
        Err(e) => RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                &format!("Failed to stringify: {}", e),
                None,
            )
            .base,
        ),
    }
}

pub fn get(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 3 {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                "__json_get expects 3 arguments (map, key, default)",
                None,
            )
            .base,
        );
    }

    let map = match &args[0] {
        Value::Map(m) => m,
        _ => {
            return RuntimeResult::new().failure(
                RuntimeError::new(
                    dummy_pos(),
                    dummy_pos(),
                    "__json_get: first argument must be a map",
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
                    dummy_pos(),
                    dummy_pos(),
                    "__json_get: second argument must be a string",
                    None,
                )
                .base,
            );
        }
    };

    if let Some(value) = map.get(key) {
        RuntimeResult::new().success(value.clone())
    } else {
        RuntimeResult::new().success(args[2].clone())
    }
}

pub fn set(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 3 {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                "__json_set expects 3 arguments (map, key, value)",
                None,
            )
            .base,
        );
    }

    let map = match &args[0] {
        Value::Map(m) => m.clone(),
        _ => {
            return RuntimeResult::new().failure(
                RuntimeError::new(
                    dummy_pos(),
                    dummy_pos(),
                    "__json_set: first argument must be a map",
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
                    dummy_pos(),
                    dummy_pos(),
                    "__json_set: second argument must be a string",
                    None,
                )
                .base,
            );
        }
    };

    let mut mutable_map = map;
    mutable_map.set(key, args[2].clone());
    RuntimeResult::new().success(Value::Map(mutable_map))
}

pub fn has_key(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 2 {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                "__json_has_key expects 2 arguments (map, key)",
                None,
            )
            .base,
        );
    }

    let map = match &args[0] {
        Value::Map(m) => m,
        _ => {
            return RuntimeResult::new().failure(
                RuntimeError::new(
                    dummy_pos(),
                    dummy_pos(),
                    "__json_has_key: first argument must be a map",
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
                    dummy_pos(),
                    dummy_pos(),
                    "__json_has_key: second argument must be a string",
                    None,
                )
                .base,
            );
        }
    };

    RuntimeResult::new().success(Value::Bool(map.contains_key(key)))
}
