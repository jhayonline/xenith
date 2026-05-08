//! # Type System Module
//!
//! Defines the type system for Xenith including primitive types,
//! collections, and user-defined types.

use std::collections::HashMap;

/// Represents all possible types in Xenith
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    /// Integer (64-bit signed)
    Int,
    /// Float (64-bit)
    Float,
    /// String (UTF-8)
    String,
    /// Boolean (true/false)
    Bool,
    /// Null type
    Null,
    /// List of type T
    List(Box<Type>),
    /// Map with key type K and value type V
    Map(Box<Type>, Box<Type>),
    /// Function type
    Function(FunctionType),
    /// Struct type (user-defined)
    Struct(String, Vec<StructField>),
    /// Type alias reference
    Alias(String, Box<Type>),
    /// Unknown/not yet resolved (for parsing)
    Unknown,
    /// JSON type, can hold mixed types (null, bool, number, string, array, object)
    Json,
    Union(Vec<Type>),
    Tuple(Vec<Type>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionType {
    pub param_types: Vec<Type>,
    pub return_type: Box<Type>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StructField {
    pub name: String,
    pub field_type: Type,
    pub is_constant: bool,
}

impl Type {
    /// Get the default value as a string representation
    pub fn default_value(&self) -> String {
        match self {
            Type::Int => "0".to_string(),
            Type::Float => "0.0".to_string(),
            Type::String => "\"\"".to_string(),
            Type::Bool => "false".to_string(),
            Type::Null => "null".to_string(),
            Type::List(_) => "[]".to_string(),
            Type::Map(_, _) => "{}".to_string(),
            Type::Json => "null".to_string(),
            _ => "null".to_string(),
        }
    }

    /// Check if type is numeric (int or float)
    pub fn is_numeric(&self) -> bool {
        matches!(self, Type::Int | Type::Float)
    }

    /// Check if type is primitive
    pub fn is_primitive(&self) -> bool {
        matches!(
            self,
            Type::Int | Type::Float | Type::String | Type::Bool | Type::Null
        )
    }

    /// Parse type from string (for testing)
    pub fn from_str(s: &str) -> Self {
        match s {
            "int" => Type::Int,
            "float" => Type::Float,
            "string" => Type::String,
            "bool" => Type::Bool,
            "null" => Type::Null,
            "json" => Type::Json,
            "tuple" => Type::Tuple(vec![]),
            _ => Type::Unknown,
        }
    }

    /// Convert type to string representation
    pub fn to_string(&self) -> String {
        match self {
            Type::Int => "int".to_string(),
            Type::Float => "float".to_string(),
            Type::String => "string".to_string(),
            Type::Bool => "bool".to_string(),
            Type::Null => "null".to_string(),
            Type::List(t) => format!("list<{}>", t.to_string()),
            Type::Map(k, v) => format!("map<{}, {}>", k.to_string(), v.to_string()),
            Type::Function(f) => format!(
                "method({}) -> {}",
                f.param_types
                    .iter()
                    .map(|t| t.to_string())
                    .collect::<Vec<_>>()
                    .join(", "),
                f.return_type.to_string()
            ),
            Type::Struct(name, _) => name.clone(),
            Type::Alias(name, _) => name.clone(),
            Type::Unknown => "unknown".to_string(),
            Type::Json => "json".to_string(),
            Type::Union(types) => types
                .iter()
                .map(|t| t.to_string())
                .collect::<Vec<_>>()
                .join(" | "),
            Type::Tuple(types) => {
                format!(
                    "({})",
                    types
                        .iter()
                        .map(|t| t.to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
        }
    }

    pub fn is_union(&self) -> bool {
        matches!(self, Type::Union(_))
    }

    pub fn get_union_types(&self) -> Vec<Type> {
        match self {
            Type::Union(types) => types.clone(),
            _ => vec![self.clone()],
        }
    }

    /// Resolve type aliases to their underlying type
    pub fn resolve(&self, aliases: &HashMap<String, Type>) -> Type {
        match self {
            Type::Alias(name, inner) => {
                if let Some(resolved) = aliases.get(name) {
                    resolved.resolve(aliases)
                } else {
                    inner.resolve(aliases)
                }
            }
            Type::List(inner) => Type::List(Box::new(inner.resolve(aliases))),
            Type::Map(k, v) => {
                Type::Map(Box::new(k.resolve(aliases)), Box::new(v.resolve(aliases)))
            }
            Type::Function(f) => Type::Function(FunctionType {
                param_types: f.param_types.iter().map(|t| t.resolve(aliases)).collect(),
                return_type: Box::new(f.return_type.resolve(aliases)),
            }),
            Type::Struct(name, fields) => Type::Struct(name.clone(), fields.clone()),
            _ => self.clone(),
        }
    }
}
