//! # JSON Type Module
//!
//! Defines the dedicated JSON type for handling JSON data with mixed types.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Dedicated JSON type that can represent any valid JSON value
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Json {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<Json>),
    Object(HashMap<String, Json>),
}

impl Json {
    /// Create a null JSON value
    pub fn null() -> Self {
        Json::Null
    }

    /// Check if this is null
    pub fn is_null(&self) -> bool {
        matches!(self, Json::Null)
    }

    /// Create a boolean JSON value
    pub fn bool(b: bool) -> Self {
        Json::Bool(b)
    }

    /// Create a number JSON value
    pub fn number(n: f64) -> Self {
        Json::Number(n)
    }

    /// Create a string JSON value
    pub fn string(s: String) -> Self {
        Json::String(s)
    }

    /// Create an array JSON value
    pub fn array(arr: Vec<Json>) -> Self {
        Json::Array(arr)
    }

    /// Create an object JSON value
    pub fn object(obj: HashMap<String, Json>) -> Self {
        Json::Object(obj)
    }

    /// Get as bool if applicable
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Json::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Get as number if applicable
    pub fn as_number(&self) -> Option<f64> {
        match self {
            Json::Number(n) => Some(*n),
            _ => None,
        }
    }

    /// Get as string if applicable
    pub fn as_string(&self) -> Option<&str> {
        match self {
            Json::String(s) => Some(s),
            _ => None,
        }
    }

    /// Get as array if applicable
    pub fn as_array(&self) -> Option<&Vec<Json>> {
        match self {
            Json::Array(arr) => Some(arr),
            _ => None,
        }
    }

    /// Get as array mutably if applicable
    pub fn as_array_mut(&mut self) -> Option<&mut Vec<Json>> {
        match self {
            Json::Array(arr) => Some(arr),
            _ => None,
        }
    }

    /// Get as object if applicable
    pub fn as_object(&self) -> Option<&HashMap<String, Json>> {
        match self {
            Json::Object(obj) => Some(obj),
            _ => None,
        }
    }

    /// Get as object mutably if applicable
    pub fn as_object_mut(&mut self) -> Option<&mut HashMap<String, Json>> {
        match self {
            Json::Object(obj) => Some(obj),
            _ => None,
        }
    }

    /// Convert to JSON string
    pub fn to_string(&self) -> String {
        match self {
            Json::Null => "null".to_string(),
            Json::Bool(b) => b.to_string(),
            Json::Number(n) => {
                if n.fract() == 0.0 {
                    format!("{:.0}", n)
                } else {
                    n.to_string()
                }
            }
            Json::String(s) => format!("\"{}\"", s.replace('"', "\\\"")),
            Json::Array(arr) => {
                let elements: Vec<String> = arr.iter().map(|j| j.to_string()).collect();
                format!("[{}]", elements.join(", "))
            }
            Json::Object(obj) => {
                let pairs: Vec<String> = obj
                    .iter()
                    .map(|(k, v)| format!("\"{}\": {}", k, v.to_string()))
                    .collect();
                format!("{{{}}}", pairs.join(", "))
            }
        }
    }

    /// Pretty print JSON
    pub fn to_string_pretty(&self, indent: usize) -> String {
        let spaces = "  ".repeat(indent);
        match self {
            Json::Null => "null".to_string(),
            Json::Bool(b) => b.to_string(),
            Json::Number(n) => {
                if n.fract() == 0.0 {
                    format!("{:.0}", n)
                } else {
                    n.to_string()
                }
            }
            Json::String(s) => format!("\"{}\"", s.replace('"', "\\\"")),
            Json::Array(arr) => {
                if arr.is_empty() {
                    return "[]".to_string();
                }
                let elements: Vec<String> = arr
                    .iter()
                    .map(|j| {
                        format!(
                            "{}{}",
                            "  ".repeat(indent + 1),
                            j.to_string_pretty(indent + 1)
                        )
                    })
                    .collect();
                format!("[\n{}\n{}]", elements.join(",\n"), spaces)
            }
            Json::Object(obj) => {
                if obj.is_empty() {
                    return "{}".to_string();
                }
                let pairs: Vec<String> = obj
                    .iter()
                    .map(|(k, v)| {
                        format!(
                            "{}\"{}\": {}",
                            "  ".repeat(indent + 1),
                            k,
                            v.to_string_pretty(indent + 1)
                        )
                    })
                    .collect();
                format!("{{\n{}\n{}}}", pairs.join(",\n"), spaces)
            }
        }
    }
}

// Helper to convert from serde_json::Value to our Json type
impl From<serde_json::Value> for Json {
    fn from(value: serde_json::Value) -> Self {
        match value {
            serde_json::Value::Null => Json::Null,
            serde_json::Value::Bool(b) => Json::Bool(b),
            serde_json::Value::Number(n) => Json::Number(n.as_f64().unwrap_or(0.0)),
            serde_json::Value::String(s) => Json::String(s),
            serde_json::Value::Array(arr) => Json::Array(arr.into_iter().map(Json::from).collect()),
            serde_json::Value::Object(obj) => {
                let mut map = HashMap::new();
                for (k, v) in obj {
                    map.insert(k, Json::from(v));
                }
                Json::Object(map)
            }
        }
    }
}

// Helper to convert from our Json type to serde_json::Value
impl From<Json> for serde_json::Value {
    fn from(value: Json) -> Self {
        match value {
            Json::Null => serde_json::Value::Null,
            Json::Bool(b) => serde_json::Value::Bool(b),
            Json::Number(n) => {
                // Fix: wrap in Value::Number
                serde_json::Value::Number(
                    serde_json::Number::from_f64(n)
                        .unwrap_or(serde_json::Number::from_f64(0.0).unwrap()),
                )
            }
            Json::String(s) => serde_json::Value::String(s),
            Json::Array(arr) => {
                serde_json::Value::Array(arr.into_iter().map(serde_json::Value::from).collect())
            }
            Json::Object(obj) => {
                let mut map = serde_json::Map::new();
                for (k, v) in obj {
                    map.insert(k, serde_json::Value::from(v));
                }
                serde_json::Value::Object(map)
            }
        }
    }
}
