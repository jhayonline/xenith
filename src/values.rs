// values.rs
//! # Runtime Values Module
//!
//! Defines the runtime object system for Xenith including Numbers, Strings,
//! Lists, and Functions. Implements operations between values (arithmetic,
//! comparison, logical) and provides the foundation for the interpreter's
//! execution semantics.

use std::collections::HashMap;
use std::rc::Rc;
use std::io::{self, Write};

use crate::context::Context;
use crate::error::{Error, RuntimeError};
use crate::interpreter::Interpreter;
use crate::nodes::Node;
use crate::position::Position;
use crate::runtime_result::RuntimeResult;
use crate::types::Type;
use crate::utils::value_to_string;

/// All possible runtime values in Xenith
#[derive(Debug, Clone)]
pub enum Value {
    /// Held flat rather than as a `Number`, which cost a second tag and made
    /// the payload 16 bytes for 8 bytes of data. `Number` is still the type
    /// the arithmetic speaks; it is just not what a `Value` stores.
    Int(i64),
    Float(f64),
    /// Shared, not owned. Cloning a `Value` happens on every call, every
    /// symbol-table read and every assignment; with the text owned inline, a
    /// document or a request body was copied in full each time, which made
    /// passing one through a few functions cost more than reading it. An `Rc`
    /// makes the clone a refcount bump. Strings are never modified in place --
    /// `a + b` builds a new one -- so there is no copy-on-write to do.
    String(Rc<XenithString>),
    /// Boxed. A `Bytes` holds a `Vec<u8>`, 24 bytes inline, which made every
    /// `Value` that size whether or not it was bytes. Shared rather than owned
    /// so a clone stays a refcount bump, as `List` and `String` already are.
    Bytes(Rc<Bytes>),
    List(List),
    Function(Box<Function>),
    BuiltInFunction(BuiltInFunction),
    /// Boxed: a `Map` holds a `HashMap`, which is 48 bytes inline and made
    /// every `Value` that size whether or not it was a map.
    Map(Box<Map>),
    /// Boxed for the same reason: a `String` plus a `HashMap` is 72 bytes.
    Struct(Box<Struct>),
    /// Boxed for the same reason again: two `String`s and a `Vec` is 72.
    Enum(Box<EnumValue>),
    Bool(bool),
    /// Shared, not owned, for the same reason as `List`: a clone happens on
    /// every symbol-table read and every assignment, and copying the elements
    /// each time made passing a tuple through a few functions cost more than
    /// building it. Nothing mutates a tuple in place -- the language has no
    /// tuple element assignment -- so, as with `String`, there is no
    /// copy-on-write to do and sharing is invisible.
    Tuple(Rc<Vec<Value>>),
    Null,
}

impl Value {
    /// Creates an int value
    pub fn int(i: i64) -> Self {
        Value::Int(i)
    }

    /// Creates a string value from an already-built [`XenithString`].
    pub fn string_of(value: XenithString) -> Self {
        Value::String(Rc::new(value))
    }

    /// Creates a float value
    pub fn float(f: f64) -> Self {
        Value::Float(f)
    }

    /// Creates a string value
    pub fn string(s: &str) -> Self {
        Value::string_of(XenithString::new(s.to_string()))
    }

    /// Creates a bytes value
    pub fn bytes(data: Vec<u8>) -> Self {
        Value::Bytes(Rc::new(Bytes::new(data)))
    }

    /// Creates a list value
    pub fn list(elements: Vec<Value>) -> Self {
        Value::List(List::new(elements))
    }

    /// Checks if the value is truthy
    pub fn is_true(&self) -> bool {
        match self {
            Value::Int(i) => *i != 0,
            Value::Float(f) => *f != 0.0,
            Value::String(s) => !s.value.is_empty(),
            Value::Bytes(b) => !b.data.is_empty(),
            Value::List(l) => !l.elements.is_empty(),
            Value::Map(m) => !m.pairs.is_empty(),
            Value::Struct(s) => !s.fields.is_empty(),
            // A variant is a value that is there, whichever one it is. There is
            // no sensible "empty" enum to call false.
            Value::Enum(_) => true,
            Value::Function(_) => true,
            Value::BuiltInFunction(_) => true,
            Value::Bool(b) => *b,
            Value::Null => false,
            Value::Tuple(t) => !t.is_empty(),
        }
    }

    /// The arithmetic helpers take a `Number`, so this builds one rather than
    /// borrowing one. `Number` is `Copy` and 16 bytes, so that is free.
    pub fn as_number(&self) -> Option<Number> {
        match self {
            Value::Int(i) => Some(Number::Int(*i)),
            Value::Float(f) => Some(Number::Float(*f)),
            _ => None,
        }
    }

    /// Tries to get as a string
    pub fn as_string(&self) -> Option<&XenithString> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }

    /// Tries to get as bytes
    pub fn as_bytes(&self) -> Option<&Bytes> {
        match self {
            Value::Bytes(b) => Some(b),
            _ => None,
        }
    }

    /// Tries to get as a list
    pub fn as_list(&self) -> Option<&List> {
        match self {
            Value::List(l) => Some(l),
            _ => None,
        }
    }

    /// Tries to get as a list (mutable)
    pub fn as_list_mut(&mut self) -> Option<&mut List> {
        match self {
            Value::List(l) => Some(l),
            _ => None,
        }
    }

    fn dummy_pos() -> crate::position::Position {
        crate::position::Position::new(0, 0, 0, "", "")
    }

    pub fn tuple(elements: Vec<Value>) -> Self {
        Value::Tuple(Rc::new(elements))
    }

    // Helper for tuple operations
    pub fn as_tuple(&self) -> Option<&Vec<Value>> {
        match self {
            Value::Tuple(t) => Some(t),
            _ => None,
        }
    }

    // ---------------------------------------------------------------
    // Arithmetic
    //
    // Go semantics: int op int -> int, float op float -> float.
    // Mixing int and float is a type error; use `as` to convert.
    // Integer arithmetic is checked -- overflow is an error, never a
    // silent wrap or a slide into f64 imprecision.
    // ---------------------------------------------------------------

    /// An operation applied to types it is not defined for.
    ///
    /// This is XEN001 like any other type mismatch. It used to fall through to
    /// the generic XEN200, which meant `1 + 2.0` and `"a" + 1` reported
    /// different codes for the same kind of mistake.
    fn arith_err(msg: &str) -> Error {
        RuntimeError::new(Self::dummy_pos(), Self::dummy_pos(), msg, None)
            .with_code("XEN001")
            .with_name("Type Mismatch")
            .with_help("convert explicitly, e.g. `x as float`")
            .base
    }

    fn mixed_err(op: &str, a: &Number, b: &Number) -> Error {
        RuntimeError::new(
            Self::dummy_pos(),
            Self::dummy_pos(),
            &format!(
                "cannot {} {} and {}",
                op,
                a.type_name(),
                b.type_name()
            ),
            None,
        )
        .with_code("XEN001")
        .with_name("Type Mismatch")
        .with_help(&format!(
            "convert explicitly, e.g. `x as {}`",
            if a.is_int() { "float" } else { "int" }
        ))
        .base
    }

    fn overflow_err(op: &str) -> Error {
        RuntimeError::new(
            Self::dummy_pos(),
            Self::dummy_pos(),
            &format!("integer overflow in {}", op),
            None,
        )
        .with_code("XEN017")
        .with_name("Integer Overflow")
        .base
    }

    /// Addition operation
    pub fn add(&self, other: &Value) -> Result<Value, Error> {
        match (self, other) {
            (Value::Int(x), Value::Int(y)) => x
                .checked_add(*y)
                .map(Value::int)
                .ok_or_else(|| Self::overflow_err("addition")),
            (Value::Float(x), Value::Float(y)) => Ok(Value::float(x + y)),
            (Value::Int(_), Value::Float(_)) | (Value::Float(_), Value::Int(_)) => Err(
                Self::mixed_err(
                    "add",
                    &self.as_number().unwrap(),
                    &other.as_number().unwrap(),
                ),
            ),
        
            (Value::String(a), Value::String(b)) => {
                let mut new = a.value.clone();
                new.push_str(&b.value);
                Ok(Value::string_of(XenithString::new(new)))
            }
            (Value::Bytes(a), Value::Bytes(b)) => {
                let mut new = a.data.clone();
                new.extend_from_slice(&b.data);
                Ok(Value::bytes(new))
            }
            (Value::List(a), Value::List(b)) => {
                let mut new = a.clone();
                new.elements_mut().extend(b.elements.iter().cloned());
                Ok(Value::List(new))
            }
            _ => Err(Self::arith_err(&format!(
                "cannot add {} and {}",
                Self::get_type_name(self),
                Self::get_type_name(other)
            ))),
        }
    }

    /// Subtraction operation
    pub fn subtract(&self, other: &Value) -> Result<Value, Error> {
        match (self, other) {
            (Value::Int(x), Value::Int(y)) => x
                .checked_sub(*y)
                .map(Value::int)
                .ok_or_else(|| Self::overflow_err("subtraction")),
            (Value::Float(x), Value::Float(y)) => Ok(Value::float(x - y)),
            (Value::Int(_), Value::Float(_)) | (Value::Float(_), Value::Int(_)) => Err(
                Self::mixed_err(
                    "subtract",
                    &self.as_number().unwrap(),
                    &other.as_number().unwrap(),
                ),
            ),
        
            _ => Err(Self::arith_err(&format!(
                "cannot subtract {} from {}",
                Self::get_type_name(other),
                Self::get_type_name(self)
            ))),
        }
    }

    /// Multiplication operation
    pub fn multiply(&self, other: &Value) -> Result<Value, Error> {
        match (self, other) {
            (Value::Int(x), Value::Int(y)) => x
                .checked_mul(*y)
                .map(Value::int)
                .ok_or_else(|| Self::overflow_err("multiplication")),
            (Value::Float(x), Value::Float(y)) => Ok(Value::float(x * y)),
            (Value::Int(_), Value::Float(_)) | (Value::Float(_), Value::Int(_)) => Err(
                Self::mixed_err(
                    "multiply",
                    &self.as_number().unwrap(),
                    &other.as_number().unwrap(),
                ),
            ),
        
            (Value::String(a), Value::Int(n)) => {
                if *n < 0 {
                    return Err(Self::arith_err("cannot repeat a string a negative number of times"));
                }
                Ok(Value::string_of(XenithString::new(a.value.repeat(*n as usize))))
            }
            _ => Err(Self::arith_err(&format!(
                "cannot multiply {} and {}",
                Self::get_type_name(self),
                Self::get_type_name(other)
            ))),
        }
    }

    /// Division operation. Int / int truncates toward zero, as in Go and C.
    pub fn divide(&self, other: &Value) -> Result<Value, Error> {
        match (self, other) {
            (Value::Int(_), Value::Int(0)) => Err(RuntimeError::new(
                Self::dummy_pos(),
                Self::dummy_pos(),
                "division by zero",
                None,
            )
            .with_code("XEN003")
            .with_name("Division by Zero")
            .base),
            (Value::Int(x), Value::Int(y)) => x
                .checked_div(*y)
                .map(Value::int)
                .ok_or_else(|| Self::overflow_err("division")),
            (Value::Float(x), Value::Float(y)) => {
                if *y == 0.0 {
                    return Err(RuntimeError::new(
                        Self::dummy_pos(),
                        Self::dummy_pos(),
                        "division by zero",
                        None,
                    )
                    .with_code("XEN003")
                    .with_name("Division by Zero")
                    .base);
                }
                Ok(Value::float(x / y))
            }
            (Value::Int(_), Value::Float(_)) | (Value::Float(_), Value::Int(_)) => Err(
                Self::mixed_err(
                    "divide",
                    &self.as_number().unwrap(),
                    &other.as_number().unwrap(),
                ),
            ),
        
            _ => Err(Self::arith_err(&format!(
                "cannot divide {} by {}",
                Self::get_type_name(self),
                Self::get_type_name(other)
            ))),
        }
    }

    /// Remainder operation
    pub fn modulo(&self, other: &Value) -> Result<Value, Error> {
        match (self, other) {
            (Value::Int(_), Value::Int(0)) => Err(RuntimeError::new(
                Self::dummy_pos(),
                Self::dummy_pos(),
                "remainder by zero",
                None,
            )
            .with_code("XEN003")
            .with_name("Division by Zero")
            .base),
            (Value::Int(x), Value::Int(y)) => x
                .checked_rem(*y)
                .map(Value::int)
                .ok_or_else(|| Self::overflow_err("remainder")),
            (Value::Float(x), Value::Float(y)) => Ok(Value::float(x % y)),
            (Value::Int(_), Value::Float(_)) | (Value::Float(_), Value::Int(_)) => Err(
                Self::mixed_err(
                    "take the remainder of",
                    &self.as_number().unwrap(),
                    &other.as_number().unwrap(),
                ),
            ),
        
            _ => Err(Self::arith_err("cannot take the remainder of these types")),
        }
    }

    /// Power operation
    pub fn power(&self, other: &Value) -> Result<Value, Error> {
        match (self, other) {
            (Value::Int(x), Value::Int(y)) => {
                if *y < 0 {
                    return Err(Self::arith_err(
                        "cannot raise an int to a negative power -- convert to float first",
                    ));
                }
                let exp = u32::try_from(*y).map_err(|_| Self::overflow_err("power"))?;
                x.checked_pow(exp)
                    .map(Value::int)
                    .ok_or_else(|| Self::overflow_err("power"))
            }
            (Value::Float(x), Value::Float(y)) => Ok(Value::float(x.powf(*y))),
            (Value::Int(_), Value::Float(_)) | (Value::Float(_), Value::Int(_)) => Err(
                Self::mixed_err(
                    "raise",
                    &self.as_number().unwrap(),
                    &other.as_number().unwrap(),
                ),
            ),
        
            _ => Err(Self::arith_err("cannot raise these types to a power")),
        }
    }

    // ---------------------------------------------------------------
    // Comparison -- always yields a bool, never a 1.0/0.0 number
    // ---------------------------------------------------------------

    /// Equality comparison
    pub fn equals(&self, other: &Value) -> Result<Value, Error> {
        Ok(Value::Bool(self.eq_value(other)))
    }

    /// Structural equality, for interning compile-time constants.
    ///
    /// Deliberately not `PartialEq`: `eq_value` is the language's `==`, which
    /// answers a different question -- it is about values a program compares,
    /// not about whether two constant slots hold the same thing. This one is
    /// stricter, because folding an int constant into a float slot would
    /// change what the program means.
    pub fn eq_for_constants(&self, other: &Value) -> bool {
        match (self, other) {
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => a.to_bits() == b.to_bits(),
            (Value::String(a), Value::String(b)) => a.value == b.value,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Null, Value::Null) => true,
            _ => false,
        }
    }

    fn eq_value(&self, other: &Value) -> bool {
        match (self, other) {
            (Value::Int(x), Value::Int(y)) => x == y,
            (Value::Float(x), Value::Float(y)) => x == y,
            (Value::Int(_), Value::Float(_)) | (Value::Float(_), Value::Int(_)) => false,
        
            (Value::String(a), Value::String(b)) => a.value == b.value,
            (Value::Bytes(a), Value::Bytes(b)) => a.data == b.data,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Null, Value::Null) => true,
            (Value::List(a), Value::List(b)) => {
                a.elements.len() == b.elements.len()
                    && a.elements
                        .iter()
                        .zip(b.elements.iter())
                        .all(|(x, y)| x.eq_value(y))
            }
            (Value::Tuple(a), Value::Tuple(b)) => {
                a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| x.eq_value(y))
            }
            (Value::Struct(a), Value::Struct(b)) => {
                a.name == b.name
                    && a.fields.len() == b.fields.len()
                    && a.fields.iter().all(|(k, v)| {
                        b.fields.get(k).map(|o| v.eq_value(o)).unwrap_or(false)
                    })
            }
            (Value::Enum(a), Value::Enum(b)) => {
                a.enum_name == b.enum_name
                    && a.variant == b.variant
                    && a.payload.len() == b.payload.len()
                    && a.payload
                        .iter()
                        .zip(b.payload.iter())
                        .all(|(x, y)| x.eq_value(y))
            }
            _ => false,
        }
    }

    /// Not equals comparison
    pub fn not_equals(&self, other: &Value) -> Result<Value, Error> {
        Ok(Value::Bool(!self.eq_value(other)))
    }

    fn compare(&self, other: &Value, op: &str) -> Result<std::cmp::Ordering, Error> {
        match (self, other) {
            (Value::Int(x), Value::Int(y)) => Ok(x.cmp(y)),
            (Value::Float(x), Value::Float(y)) => x
                .partial_cmp(y)
                .ok_or_else(|| Self::arith_err("cannot compare NaN")),
            (Value::Int(_), Value::Float(_)) | (Value::Float(_), Value::Int(_)) => Err(
                Self::mixed_err(
                    "compare",
                    &self.as_number().unwrap(),
                    &other.as_number().unwrap(),
                ),
            ),
        
            (Value::String(a), Value::String(b)) => Ok(a.value.cmp(&b.value)),
            (Value::Bytes(a), Value::Bytes(b)) => Ok(a.data.cmp(&b.data)),
            _ => Err(Self::arith_err(&format!(
                "cannot compare {} {} {}",
                Self::get_type_name(self),
                op,
                Self::get_type_name(other)
            ))),
        }
    }

    /// Less than comparison
    pub fn less_than(&self, other: &Value) -> Result<Value, Error> {
        Ok(Value::Bool(self.compare(other, "<")?.is_lt()))
    }

    /// Greater than comparison
    pub fn greater_than(&self, other: &Value) -> Result<Value, Error> {
        Ok(Value::Bool(self.compare(other, ">")?.is_gt()))
    }

    /// Less than or equal comparison
    pub fn less_than_or_equal(&self, other: &Value) -> Result<Value, Error> {
        Ok(Value::Bool(self.compare(other, "<=")?.is_le()))
    }

    /// Greater than or equal comparison
    pub fn greater_than_or_equal(&self, other: &Value) -> Result<Value, Error> {
        Ok(Value::Bool(self.compare(other, ">=")?.is_ge()))
    }

    /// Logical NOT
    pub fn logical_not(&self) -> Result<Value, Error> {
        Ok(Value::Bool(!self.is_true()))
    }

    /// Arithmetic negation
    pub fn negative(&self) -> Result<Value, Error> {
        match self {
            Value::Int(n) => n
                .checked_neg()
                .map(Value::int)
                .ok_or_else(|| Self::overflow_err("negation")),
            Value::Float(f) => Ok(Value::float(-f)),
            _ => Err(Self::arith_err(&format!(
                "cannot negate {}",
                Self::get_type_name(self)
            ))),
        }
    }

    /// Logical AND
    pub fn anded_by(&self, other: &Value) -> Result<Value, Error> {
        Ok(Value::Bool(self.is_true() && other.is_true()))
    }

    /// Logical OR
    pub fn ored_by(&self, other: &Value) -> Result<Value, Error> {
        Ok(Value::Bool(self.is_true() || other.is_true()))
    }

    /// Does a runtime value inhabit the given declared type?
    pub fn value_matches_type(value: &Value, expected_type: &Type) -> bool {
        match expected_type {
            Type::Int => matches!(value, Value::Int(_)),
            Type::Float => matches!(value, Value::Float(_)),
            Type::String => matches!(value, Value::String(_)),
            Type::Bytes => matches!(value, Value::Bytes(_)),
            Type::Bool => matches!(value, Value::Bool(_)),
            Type::Null => matches!(value, Value::Null),
            Type::List(inner) => match value {
                Value::List(l) => l
                    .elements
                    .iter()
                    .all(|e| Self::value_matches_type(e, inner)),
                _ => false,
            },
            Type::Map(_, v) => match value {
                Value::Map(m) => m.pairs.values().all(|e| Self::value_matches_type(e, v)),
                _ => false,
            },
            Type::Tuple(types) => match value {
                Value::Tuple(elems) => {
                    elems.len() == types.len()
                        && elems
                            .iter()
                            .zip(types.iter())
                            .all(|(e, t)| Self::value_matches_type(e, t))
                }
                _ => false,
            },
            // `Type::Struct` is the named user type, which covers enums too;
            // see the note on the variant in `types.rs`.
            Type::Struct(name, _) => match value {
                Value::Struct(s) => &s.name == name,
                Value::Enum(e) => &e.enum_name == name,
                _ => false,
            },
            Type::Function(_) => {
                matches!(value, Value::Function(_) | Value::BuiltInFunction(_))
            }
            Type::Alias(_, inner) => Self::value_matches_type(value, inner),
            Type::Unknown => true,
        }
    }

    /// Name of a value's runtime type, for diagnostics
    pub fn get_type_name(value: &Value) -> String {
        match value {
            Value::Int(_) => "int".to_string(),
            Value::Float(_) => "float".to_string(),
            Value::String(_) => "string".to_string(),
            Value::Bytes(_) => "bytes".to_string(),
            Value::Bool(_) => "bool".to_string(),
            Value::List(_) => "list".to_string(),
            Value::Map(_) => "map".to_string(),
            Value::Struct(s) => s.name.clone(),
            Value::Enum(e) => e.enum_name.clone(),
            Value::Function(_) => "method".to_string(),
            Value::BuiltInFunction(_) => "method".to_string(),
            Value::Null => "null".to_string(),
            Value::Tuple(_) => "tuple".to_string(),
        }
    }
}

/// Numeric runtime value.
///
/// `int` is a real 64-bit signed integer, not an f64 in disguise, so integer
/// values above 2^53 keep every bit of precision.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Number {
    Int(i64),
    Float(f64),
}

impl Number {
    pub fn math_pi() -> Self {
        Number::Float(std::f64::consts::PI)
    }

    /// Name of this number's type, for diagnostics
    pub fn type_name(&self) -> &'static str {
        match self {
            Number::Int(_) => "int",
            Number::Float(_) => "float",
        }
    }

    pub fn is_int(&self) -> bool {
        matches!(self, Number::Int(_))
    }

    pub fn is_zero(&self) -> bool {
        match self {
            Number::Int(i) => *i == 0,
            Number::Float(f) => *f == 0.0,
        }
    }

    /// Widen to f64. Lossy for ints beyond 2^53 -- only use where a float is
    /// genuinely wanted (explicit `as float`, float-typed math).
    pub fn to_f64(&self) -> f64 {
        match self {
            Number::Int(i) => *i as f64,
            Number::Float(f) => *f,
        }
    }

    /// Narrow to i64, truncating toward zero. None if a float is out of range
    /// or not finite.
    pub fn to_i64(&self) -> Option<i64> {
        match self {
            Number::Int(i) => Some(*i),
            Number::Float(f) => {
                if f.is_finite() && *f >= i64::MIN as f64 && *f <= i64::MAX as f64 {
                    Some(*f as i64)
                } else {
                    None
                }
            }
        }
    }

    /// Usable as a collection index?
    pub fn as_index(&self) -> Option<usize> {
        match self {
            Number::Int(i) if *i >= 0 => Some(*i as usize),
            _ => None,
        }
    }
}

impl std::fmt::Display for Number {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Number::Int(i) => write!(f, "{}", i),
            // Floats always show a decimal point so `1.0` never prints as `1`
            Number::Float(v) => {
                if v.fract() == 0.0 && v.is_finite() {
                    write!(f, "{:.1}", v)
                } else {
                    write!(f, "{}", v)
                }
            }
        }
    }
}

/// String runtime value
#[derive(Debug, Clone)]
pub struct XenithString {
    pub value: String,
    /// How many characters, and whether all of them are ASCII. Both are settled
    /// once, when the string is made.
    ///
    /// Xenith counts and indexes strings by character while a `String` holds
    /// UTF-8, so `len()` used to walk the whole string and `text[i]` used to
    /// walk as far as `i`. A loop of the shape `while i < text.len()`, which is
    /// how every scanner in the standard library is written, was therefore
    /// quadratic before it had done anything. Caching the count fixes that one;
    /// the ASCII flag fixes indexing, since a string of only ASCII can be
    /// indexed by byte.
    char_len: usize,
    is_ascii: bool,
}

impl XenithString {
    pub fn new(value: String) -> Self {
        // `is_ascii` is a memchr-style scan and the count comes free with it,
        // so this costs one cheap pass over a string that was just built.
        let is_ascii = value.is_ascii();
        let char_len = if is_ascii {
            value.len()
        } else {
            value.chars().count()
        };
        Self {
            value,
            char_len,
            is_ascii,
        }
    }

    /// The number of characters, which is what `len()` reports in Xenith.
    pub fn char_len(&self) -> usize {
        self.char_len
    }

    /// The character at a position, counted the way [`char_len`] counts.
    ///
    /// [`char_len`]: XenithString::char_len
    pub fn char_at(&self, position: usize) -> Option<char> {
        if self.is_ascii {
            return self.value.as_bytes().get(position).map(|byte| *byte as char);
        }
        self.value.chars().nth(position)
    }

    /// The characters from `start` up to but not including `end`, both clamped.
    pub fn slice(&self, start: usize, end: usize) -> String {
        let end = end.min(self.char_len);
        if start >= end {
            return String::new();
        }
        if self.is_ascii {
            return self.value[start..end].to_string();
        }
        self.value
            .chars()
            .skip(start)
            .take(end - start)
            .collect()
    }
}

/// One value of an enum: which variant, and what it carries.
///
/// The enum's name travels with it so that `value_matches_type` can answer
/// without a table lookup, exactly as `Struct` carries its own name.
#[derive(Debug, Clone)]
pub struct EnumValue {
    pub enum_name: String,
    pub variant: String,
    pub payload: Vec<Value>,
}

impl EnumValue {
    pub fn new(enum_name: String, variant: String, payload: Vec<Value>) -> Self {
        Self {
            enum_name,
            variant,
            payload,
        }
    }
}

/// Raw bytes runtime value.
///
/// Indexing gives an `int` in 0..=255 rather than a one-byte `bytes`, because
/// nearly everything done with a single byte is arithmetic or a comparison
/// against a code, and a one-element container would have to be unwrapped at
/// every use.
#[derive(Debug, Clone)]
pub struct Bytes {
    pub data: Vec<u8>,
}

impl Bytes {
    pub fn new(data: Vec<u8>) -> Self {
        Self { data }
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// The bytes from `start` up to but not including `end`, both clamped, so
    /// this never fails. Same contract as `substring` on a string, and for the
    /// same reason: slicing is constant in byte-handling code and a version
    /// that could error means a bounds check at every call site.
    pub fn slice(&self, start: i64, end: i64) -> Bytes {
        let length = self.data.len() as i64;
        let start = start.clamp(0, length) as usize;
        let end = end.clamp(0, length) as usize;
        if start >= end {
            return Bytes::new(Vec::new());
        }
        Bytes::new(self.data[start..end].to_vec())
    }
}

/// List runtime value.
///
/// The elements are behind an `Rc` so that cloning a list is a refcount bump
/// rather than a deep copy. A list is still a value and still has value
/// semantics: `elements_mut` goes through `Rc::make_mut`, which copies only
/// when somebody else is holding the same elements. That is the difference
/// between `xs.append(x)` costing O(1) and costing O(len(xs)), and so between
/// filling a list being linear and being quadratic.
#[derive(Debug, Clone)]
pub struct List {
    pub elements: Rc<Vec<Value>>,
}

impl List {
    pub fn new(elements: Vec<Value>) -> Self {
        Self {
            elements: Rc::new(elements),
        }
    }

    /// The elements, to write to. Copies them first if this list is not the
    /// only holder, so a shared list is never modified underneath its other
    /// holders.
    pub fn elements_mut(&mut self) -> &mut Vec<Value> {
        Rc::make_mut(&mut self.elements)
    }

    pub fn append(&mut self, value: Value) {
        self.elements_mut().push(value);
    }

    pub fn pop(&mut self, index: Option<usize>) -> Option<Value> {
        let idx = index.unwrap_or(self.elements.len() - 1);
        if idx < self.elements.len() {
            Some(self.elements_mut().remove(idx))
        } else {
            None
        }
    }

    pub fn len(&self) -> usize {
        self.elements.len()
    }

    pub fn get(&self, index: usize) -> Option<&Value> {
        self.elements.get(index)
    }

    pub fn set(&mut self, index: usize, value: Value) -> bool {
        if index < self.elements.len() {
            self.elements_mut()[index] = value;
            true
        } else {
            false
        }
    }
}

/// User-defined function
#[derive(Debug, Clone)]
pub struct Function {
    pub name: Option<String>,
    /// Shared, not owned. Symbol-table reads clone the Value, so a `Box` here
    /// meant every reference to a function deep-copied its entire body AST --
    /// which dominated runtime (most of it in malloc/free).
    pub body_node: Rc<Node>,
    pub arg_names: Rc<Vec<String>>,
    pub param_types: Rc<Vec<Type>>,
    pub should_auto_return: bool,
    /// The scope this method was written in.
    ///
    /// A call runs the body against a child of *this*, not of whoever called
    /// it. That is what makes a method see the names around its own definition:
    /// closures over enclosing locals, and a module's exports reaching its
    /// private helpers.
    ///
    /// Shared rather than copied, so the capture stays live: a method defined
    /// before a name it uses still sees that name once it is declared, and a
    /// named method can see itself for recursion.
    ///
    /// This makes a reference cycle whenever a method is stored in the scope it
    /// captured, which is every named method. `Rc` never frees a cycle, so those
    /// contexts live until the process exits. A `Weak` here would break the case
    /// closures exist for, where the method outlives the scope that made it.
    pub closure: Rc<Context>,
}

impl Function {
    pub fn new(
        name: Option<String>,
        body_node: Node,
        arg_names: Vec<String>,
        param_types: Vec<Type>,
        should_auto_return: bool,
        closure: Rc<Context>,
    ) -> Self {
        Self {
            name,
            body_node: Rc::new(body_node),
            arg_names: Rc::new(arg_names),
            param_types: Rc::new(param_types),
            should_auto_return,
            closure,
        }
    }

    pub fn execute(
        &self,
        args: Vec<Value>,
        context: Context,
        interpreter: &mut Interpreter,
        call_position: Position,
    ) -> RuntimeResult {
        let mut result = RuntimeResult::new();

        // Check argument count
        if args.len() != self.arg_names.len() {
            let error = if args.len() > self.arg_names.len() {
                Error::too_many_arguments(
                    self.arg_names.len(),
                    args.len(),
                    call_position.clone(),
                    call_position,
                )
            } else {
                Error::too_few_arguments(
                    self.arg_names.len(),
                    args.len(),
                    call_position.clone(),
                    call_position,
                )
            };
            return RuntimeResult::new().failure(error);
        }

        // TYPE CHECKING
        for (i, (arg, expected_type)) in args.iter().zip(self.param_types.iter()).enumerate() {
            let matches = Value::value_matches_type(arg, expected_type);

            if !matches {
                // `to_string`, not `{:?}`: the reader wants `int`, not `Int`.
                return RuntimeResult::new().failure(Error::type_mismatch(
                    &expected_type.to_string(),
                    &Value::get_type_name(arg),
                    call_position.clone(),
                    call_position.clone(),
                ));
            }
        }

        // The interpreter recurses on the Rust stack, so runaway recursion has
        // to be caught here -- otherwise it aborts the process with a stack
        // overflow instead of producing a diagnostic.
        if context.depth_exceeded() {
            return RuntimeResult::new().failure(
                RuntimeError::new(
                    call_position.clone(),
                    call_position,
                    &format!(
                        "call depth exceeded {} while calling `{}`",
                        crate::context::MAX_CALL_DEPTH,
                        self.name.as_deref().unwrap_or("<anonymous>")
                    ),
                    Some(context.clone()),
                )
                .with_code("XEN019")
                .with_name("Recursion Limit")
                .with_help("check for a missing base case in the recursion")
                .base,
            );
        }

        // The body runs against the scope the method was written in, not the
        // one it was called from.
        let mut func_context = self.closure.create_child(
            self.name.as_deref().unwrap_or("<anonymous>"),
            call_position.clone(),
        );

        // Depth counts calls, not lexical nesting, so it has to come from the
        // caller. Taking it from the closure would leave it constant for a
        // top level method and the recursion guard would never fire, which is
        // an abort rather than a diagnostic.
        func_context.depth = context.depth + 1;

        // Bind arguments
        for (i, arg_name) in self.arg_names.iter().enumerate() {
            func_context
                .symbol_table
                .set_local(arg_name.clone(), args[i].clone());
        }

        // Execute body
        let exec_result = interpreter.visit(&self.body_node, &mut func_context);

        if let Some(err) = exec_result.error {
            return RuntimeResult::new().failure(*err);
        }

        if self.should_auto_return {
            if let Some(val) = exec_result.value {
                return RuntimeResult::new().success(val);
            }
        }

        if let Some(ret_val) = exec_result.func_return_value {
            return RuntimeResult::new().success(ret_val);
        }

        RuntimeResult::new().success(Value::Null)
    }
}

/// A predefined function, stored as its position in
/// [`crate::builtins::registry::BUILTIN_FUNCTIONS`] rather than by name. The
/// name was a `String`, which made every one of these 24 bytes and allocated
/// on every clone, to hold one of about thirty fixed strings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BuiltInFunction {
    index: u16,
}

impl BuiltInFunction {
    /// `None` when the name is not a builtin.
    pub fn new(name: &str) -> Option<Self> {
        crate::builtins::registry::index_of(name).map(|index| Self { index })
    }

    /// The builtin at a known position in the registry, for the interpreter
    /// seeding the global scope straight from the list.
    pub fn at(index: u16) -> Self {
        Self { index }
    }

    pub fn name(&self) -> &'static str {
        crate::builtins::registry::name_of(self.index)
    }

    /// `context` is the caller's scope. Only `format` uses it, to evaluate the
    /// interpolation in the string it is given, but it is threaded through here
    /// so any future builtin that needs to see the caller's variables can.
    pub fn execute(
        &self,
        args: Vec<Value>,
        interpreter: &mut Interpreter,
        call_pos: Position,
        context: &mut Context,
    ) -> RuntimeResult {
        match self.name() {
            "echo" => self.echo(args, call_pos),
            "input" => self.input(call_pos),
            "input_int" => self.input_int(call_pos),
            "clear" => self.clear(call_pos),
            "is_num" => self.is_num(args, call_pos),
            "is_str" => self.is_str(args, call_pos),
            "is_list" => self.is_list(args, call_pos),
            "is_fun" => self.is_fun(args, call_pos),
            "append" => self.append(args, call_pos),
            "pop" => self.pop(args, call_pos),
            "extend" => self.extend(args, call_pos),
            "len" => self.len(args, call_pos),
            "run" => self.run(args, interpreter, call_pos),
            "substring" => self.substring(args, call_pos),
            "fs_read" => self.fs_read(args, call_pos),
            "fs_write" => self.fs_write(args, call_pos, false),
            "fs_append" => self.fs_write(args, call_pos, true),
            "fs_read_bytes" => self.fs_read_bytes(args, call_pos),
            "fs_write_bytes" => self.fs_write_bytes(args, call_pos, false),
            "fs_append_bytes" => self.fs_write_bytes(args, call_pos, true),
            "fs_remove" => self.fs_remove(args, call_pos),
            "fs_exists" => self.fs_test(args, call_pos, "fs_exists", |p| p.exists()),
            "fs_is_file" => self.fs_test(args, call_pos, "fs_is_file", |p| p.is_file()),
            "fs_is_dir" => self.fs_test(args, call_pos, "fs_is_dir", |p| p.is_dir()),
            "fs_size" => self.fs_size(args, call_pos),
            "fs_list" => self.fs_list(args, call_pos),
            "fs_create_dir" => self.fs_create_dir(args, call_pos),
            "fs_remove_dir" => self.fs_remove_dir(args, call_pos),
            "sin" => self.float_fn("sin", args, call_pos, f64::sin),
            "cos" => self.float_fn("cos", args, call_pos, f64::cos),
            "tan" => self.float_fn("tan", args, call_pos, f64::tan),
            "log" => self.float_fn("log", args, call_pos, f64::ln),
            "log10" => self.float_fn("log10", args, call_pos, f64::log10),
            "exp" => self.float_fn("exp", args, call_pos, f64::exp),
            "atan2" => self.atan2(args, call_pos),
            "code_at" => self.code_at(args, call_pos),
            "from_code" => self.from_code(args, call_pos),
            "bytes_slice" => self.bytes_slice(args, call_pos),
            "bytes_to_string" => self.bytes_to_string(args, call_pos),
            "bytes_to_list" => self.bytes_to_list(args, call_pos),
            "bytes_from_list" => self.bytes_from_list(args, call_pos),
            "env_get" => self.env_get(args, call_pos),
            "env_set" => self.env_set(args, call_pos),
            "env_unset" => self.env_unset(args, call_pos),
            "env_vars" => self.env_vars(),
            "env_args" => self.env_args(),
            "env_cwd" => self.env_cwd(),
            "env_exit" => self.env_exit(args, call_pos),
            "format" => crate::builtins::format::format(args, interpreter, call_pos, context),
            // =================================================================================
            _ => RuntimeResult::new().failure(
                RuntimeError::new(
                    call_pos.clone(),
                    call_pos,
                    &format!("Unknown built-in function: {}", self.name()),
                    None,
                )
                .base,
            ),
        }
    }

    fn echo(&self, args: Vec<Value>, _call_pos: Position) -> RuntimeResult {
        if let Some(arg) = args.first() {
            match arg {
                n @ (Value::Int(_) | Value::Float(_)) => {
                    print!("{}", n.as_number().unwrap())
                }
                Value::String(s) => print!("{}", s.value),
                // Not the contents: bytes are usually not text, and printing
                // them raw would spray control characters at the terminal.
                // `b as string` is how you ask for the contents.
                Value::Bytes(b) => print!("<bytes {}>", b.len()),
                Value::Bool(b) => print!("{}", b),
                Value::Null => print!("null"),
                Value::List(l) => {
                    print!("[");
                    for (i, elem) in l.elements.iter().enumerate() {
                        if i > 0 {
                            print!(", ");
                        }
                        match elem {
                            n @ (Value::Int(_) | Value::Float(_)) => {
                    print!("{}", n.as_number().unwrap())
                }
                            Value::String(s) => print!("\"{}\"", s.value),
                            Value::Bool(b) => print!("{}", b),
                            Value::Null => print!("null"),
                            _ => print!("?"),
                        }
                    }
                    print!("]");
                }
                Value::Map(m) => {
                    print!("{{");
                    for (i, (k, v)) in m.pairs.iter().enumerate() {
                        if i > 0 {
                            print!(", ");
                        }
                        print!("\"{}\": ", k);
                        match v {
                            n @ (Value::Int(_) | Value::Float(_)) => {
                    print!("{}", n.as_number().unwrap())
                }
                            Value::String(s) => print!("\"{}\"", s.value),
                            Value::Bool(b) => print!("{}", b),
                            Value::Null => print!("null"),
                            _ => print!("?"),
                        }
                    }
                    print!("}}");
                }
                Value::Struct(s) => {
                    print!("<struct {}>", s.name);
                }
                // `Shape::Circle(1.0)` -- the text that would have built it.
                Value::Enum(_) => print!("{}", value_to_string(arg)),
                Value::Function(f) => {
                    if let Some(name) = &f.name {
                        print!("<function {}>", name);
                    } else {
                        print!("<anonymous function>");
                    }
                }
                Value::BuiltInFunction(b) => {
                    print!("<built-in function {}>", b.name());
                }
                Value::Tuple(t) => {
                    print!("(");
                    for (i, elem) in t.iter().enumerate() {
                        if i > 0 {
                            print!(", ");
                        }
                        // Recursively print element - would need proper handling
                        match elem {
                            n @ (Value::Int(_) | Value::Float(_)) => {
                    print!("{}", n.as_number().unwrap())
                }
                            Value::String(s) => print!("\"{}\"", s.value),
                            Value::Bool(b) => print!("{}", b),
                            Value::Null => print!("null"),
                            _ => print!("?"),
                        }
                    }
                    print!(")");
                }
            }
        }
        println!();
        io::stdout().flush().unwrap();
        RuntimeResult::new().success(Value::Null)
    }

    fn input(&self, _call_pos: Position) -> RuntimeResult {
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        RuntimeResult::new().success(Value::string_of(XenithString::new(input.trim().to_string())))
    }

    fn input_int(&self, _call_pos: Position) -> RuntimeResult {
        loop {
            let mut input = String::new();
            io::stdin().read_line(&mut input).unwrap();
            if let Ok(num) = input.trim().parse::<i64>() {
                return RuntimeResult::new().success(Value::int(num));
            }
            println!("'{}' must be an integer. Try again!", input.trim());
        }
    }

    fn clear(&self, _call_pos: Position) -> RuntimeResult {
        print!("\x1B[2J\x1B[1;1H");
        io::stdout().flush().unwrap();
        RuntimeResult::new().success(Value::Null)
    }

    fn is_num(&self, args: Vec<Value>, _call_pos: Position) -> RuntimeResult {
        let result = matches!(args.first(), Some(Value::Int(_) | Value::Float(_)));
        RuntimeResult::new().success(Value::Bool(result))
    }

    fn is_str(&self, args: Vec<Value>, _call_pos: Position) -> RuntimeResult {
        let result = matches!(args.first(), Some(Value::String(_)));
        RuntimeResult::new().success(Value::Bool(result))
    }

    fn is_list(&self, args: Vec<Value>, _call_pos: Position) -> RuntimeResult {
        let result = matches!(args.first(), Some(Value::List(_)));
        RuntimeResult::new().success(Value::Bool(result))
    }

    fn is_fun(&self, args: Vec<Value>, _call_pos: Position) -> RuntimeResult {
        let result = matches!(
            args.first(),
            Some(Value::Function(_)) | Some(Value::BuiltInFunction(_))
        );
        RuntimeResult::new().success(Value::Bool(result))
    }

    fn append(&self, args: Vec<Value>, call_pos: Position) -> RuntimeResult {
        if args.len() != 2 {
            return RuntimeResult::new().failure(
                RuntimeError::new(
                    call_pos.clone(),
                    call_pos,
                    "append expects 2 arguments",
                    None,
                )
                .base,
            );
        }

        match (&args[0], &args[1]) {
            (Value::List(list), value) => {
                let mut new_list = list.clone();
                new_list.elements_mut().push(value.clone());
                RuntimeResult::new().success(Value::List(new_list))
            }
            _ => RuntimeResult::new().failure(
                RuntimeError::new(
                    call_pos.clone(),
                    call_pos,
                    "First argument to append must be a list",
                    None,
                )
                .base,
            ),
        }
    }

    fn pop(&self, args: Vec<Value>, call_pos: Position) -> RuntimeResult {
        if args.len() != 2 {
            return RuntimeResult::new().failure(
                RuntimeError::new(call_pos.clone(), call_pos, "pop expects 2 arguments", None).base,
            );
        }

        match (&args[0], &args[1]) {
            (Value::List(list), index @ (Value::Int(_) | Value::Float(_))) => {
                let Some(idx_usize) = index.as_number().unwrap().as_index() else {
                    return RuntimeResult::new().failure(
                        RuntimeError::new(
                            call_pos.clone(),
                            call_pos,
                            "list index must be a non-negative int",
                            None,
                        )
                        .with_code("XEN004")
                        .base,
                    );
                };
                if idx_usize >= list.elements.len() {
                    return RuntimeResult::new().failure(
                        RuntimeError::new(
                            call_pos.clone(),
                            call_pos,
                            "List index out of bounds",
                            None,
                        )
                        .base,
                    );
                }
                let mut new_list = list.clone();
                let popped = new_list.elements_mut().remove(idx_usize);
                RuntimeResult::new().success(popped)
            }
            _ => RuntimeResult::new().failure(
                RuntimeError::new(
                    call_pos.clone(),
                    call_pos,
                    "pop expects a list and an index",
                    None,
                )
                .base,
            ),
        }
    }

    fn extend(&self, args: Vec<Value>, call_pos: Position) -> RuntimeResult {
        if args.len() != 2 {
            return RuntimeResult::new().failure(
                RuntimeError::new(
                    call_pos.clone(),
                    call_pos,
                    "extend expects 2 arguments",
                    None,
                )
                .base,
            );
        }

        match (&args[0], &args[1]) {
            (Value::List(list_a), Value::List(list_b)) => {
                let mut new_list = list_a.clone();
                new_list.elements_mut().extend(list_b.elements.iter().cloned());
                RuntimeResult::new().success(Value::List(new_list))
            }
            _ => RuntimeResult::new().failure(
                RuntimeError::new(call_pos.clone(), call_pos, "extend expects two lists", None)
                    .base,
            ),
        }
    }

    /// Applies a one argument float function.
    ///
    /// A float is required rather than accepted alongside int, because the
    /// language does not convert between them anywhere else and this is not
    /// where it should start. `sqrt(n as float)` says what it does.
    fn float_fn(
        &self,
        name: &str,
        args: Vec<Value>,
        call_pos: Position,
        f: fn(f64) -> f64,
    ) -> RuntimeResult {
        let fail = |detail: String| {
            RuntimeResult::new().failure(
                RuntimeError::new(call_pos.clone(), call_pos.clone(), &detail, None)
                    .with_code("XEN001")
                    .with_name("Type Mismatch")
                    .with_help("convert first if you have an int, e.g. `n as float`")
                    .base,
            )
        };

        if args.len() != 1 {
            return fail(format!("{} expects 1 argument", name));
        }
        let Value::Float(x) = &args[0] else {
            return fail(format!(
                "{} expects a float, found {}",
                name,
                Value::get_type_name(&args[0])
            ));
        };

        RuntimeResult::new().success(Value::float(f(*x)))
    }

    /// The angle to a point, correct in all four quadrants.
    fn atan2(&self, args: Vec<Value>, call_pos: Position) -> RuntimeResult {
        let fail = |detail: &str| {
            RuntimeResult::new().failure(
                RuntimeError::new(call_pos.clone(), call_pos.clone(), detail, None)
                    .with_code("XEN001")
                    .with_name("Type Mismatch")
                    .with_help("convert first if you have an int, e.g. `n as float`")
                    .base,
            )
        };

        if args.len() != 2 {
            return fail("atan2 expects 2 arguments (y, x)");
        }
        let (Value::Float(y), Value::Float(x)) =
            (&args[0], &args[1])
        else {
            return fail("atan2 expects two floats");
        };

        RuntimeResult::new().success(Value::float(y.atan2(*x)))
    }

    // -- filesystem ------------------------------------------------------
    //
    // Every one of these reports failure in a string that is empty when
    // nothing went wrong, rather than by stopping the program. A missing file
    // is an ordinary thing for a program to have to handle.

    /// The one string argument these all start with.
    fn fs_path(&self, name: &str, args: &[Value], call_pos: &Position) -> Result<String, RuntimeResult> {
        let fail = |detail: String| {
            RuntimeResult::new().failure(
                RuntimeError::new(call_pos.clone(), call_pos.clone(), &detail, None)
                    .with_code("XEN001")
                    .with_name("Type Mismatch")
                    .base,
            )
        };

        match args.first() {
            Some(Value::String(path)) => Ok(path.value.clone()),
            Some(other) => Err(fail(format!(
                "{} expects a path as a string, found {}",
                name,
                Value::get_type_name(other)
            ))),
            None => Err(fail(format!("{} expects a path", name))),
        }
    }

    fn ok_pair(value: Value) -> RuntimeResult {
        RuntimeResult::new().success(Value::tuple(vec![value, Value::string("")]))
    }

    fn err_pair(empty: Value, message: String) -> RuntimeResult {
        RuntimeResult::new().success(Value::tuple(vec![
            empty,
            Value::string_of(XenithString::new(message)),
        ]))
    }

    fn outcome(result: std::io::Result<()>) -> RuntimeResult {
        match result {
            Ok(()) => RuntimeResult::new().success(Value::string("")),
            Err(e) => RuntimeResult::new()
                .success(Value::string_of(XenithString::new(e.to_string()))),
        }
    }

    fn fs_read(&self, args: Vec<Value>, call_pos: Position) -> RuntimeResult {
        let path = match self.fs_path("fs_read", &args, &call_pos) {
            Ok(path) => path,
            Err(failure) => return failure,
        };

        match std::fs::read_to_string(&path) {
            Ok(contents) => Self::ok_pair(Value::string_of(XenithString::new(contents))),
            Err(e) => Self::err_pair(Value::string(""), e.to_string()),
        }
    }

    fn fs_write(&self, args: Vec<Value>, call_pos: Position, append: bool) -> RuntimeResult {
        let name = if append { "fs_append" } else { "fs_write" };
        let path = match self.fs_path(name, &args, &call_pos) {
            Ok(path) => path,
            Err(failure) => return failure,
        };

        let Some(Value::String(contents)) = args.get(1) else {
            return RuntimeResult::new().failure(
                RuntimeError::new(
                    call_pos.clone(),
                    call_pos,
                    &format!("{} expects the contents as a string", name),
                    None,
                )
                .with_code("XEN001")
                .with_name("Type Mismatch")
                .base,
            );
        };

        let result = if append {
            use std::io::Write as _;
            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)
                .and_then(|mut file| file.write_all(contents.value.as_bytes()))
        } else {
            std::fs::write(&path, &contents.value)
        };

        Self::outcome(result)
    }

    fn fs_read_bytes(&self, args: Vec<Value>, call_pos: Position) -> RuntimeResult {
        let path = match self.fs_path("fs_read_bytes", &args, &call_pos) {
            Ok(path) => path,
            Err(failure) => return failure,
        };

        match std::fs::read(&path) {
            Ok(contents) => Self::ok_pair(Value::bytes(contents)),
            Err(e) => Self::err_pair(Value::bytes(Vec::new()), e.to_string()),
        }
    }

    fn fs_write_bytes(&self, args: Vec<Value>, call_pos: Position, append: bool) -> RuntimeResult {
        let name = if append { "fs_append_bytes" } else { "fs_write_bytes" };
        let path = match self.fs_path(name, &args, &call_pos) {
            Ok(path) => path,
            Err(failure) => return failure,
        };

        let Some(Value::Bytes(contents)) = args.get(1) else {
            return RuntimeResult::new().failure(
                RuntimeError::new(
                    call_pos.clone(),
                    call_pos,
                    &format!("{} expects the contents as bytes", name),
                    None,
                )
                .with_code("XEN001")
                .with_name("Type Mismatch")
                .with_help("convert text first, e.g. `text as bytes`")
                .base,
            );
        };

        let result = if append {
            use std::io::Write as _;
            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)
                .and_then(|mut file| file.write_all(&contents.data))
        } else {
            std::fs::write(&path, &contents.data)
        };

        Self::outcome(result)
    }

    fn fs_remove(&self, args: Vec<Value>, call_pos: Position) -> RuntimeResult {
        match self.fs_path("fs_remove", &args, &call_pos) {
            Ok(path) => Self::outcome(std::fs::remove_file(path)),
            Err(failure) => failure,
        }
    }

    fn fs_create_dir(&self, args: Vec<Value>, call_pos: Position) -> RuntimeResult {
        match self.fs_path("fs_create_dir", &args, &call_pos) {
            Ok(path) => Self::outcome(std::fs::create_dir_all(path)),
            Err(failure) => failure,
        }
    }

    fn fs_remove_dir(&self, args: Vec<Value>, call_pos: Position) -> RuntimeResult {
        match self.fs_path("fs_remove_dir", &args, &call_pos) {
            Ok(path) => Self::outcome(std::fs::remove_dir(path)),
            Err(failure) => failure,
        }
    }

    fn fs_test(
        &self,
        args: Vec<Value>,
        call_pos: Position,
        name: &str,
        test: fn(&std::path::Path) -> bool,
    ) -> RuntimeResult {
        match self.fs_path(name, &args, &call_pos) {
            Ok(path) => {
                RuntimeResult::new().success(Value::Bool(test(std::path::Path::new(&path))))
            }
            Err(failure) => failure,
        }
    }

    fn fs_size(&self, args: Vec<Value>, call_pos: Position) -> RuntimeResult {
        let path = match self.fs_path("fs_size", &args, &call_pos) {
            Ok(path) => path,
            Err(failure) => return failure,
        };

        match std::fs::metadata(&path) {
            Ok(meta) => Self::ok_pair(Value::int(meta.len() as i64)),
            Err(e) => Self::err_pair(Value::int(0), e.to_string()),
        }
    }

    /// Names in a directory, sorted so a program that walks one behaves the
    /// same on every machine and every run.
    fn fs_list(&self, args: Vec<Value>, call_pos: Position) -> RuntimeResult {
        let path = match self.fs_path("fs_list", &args, &call_pos) {
            Ok(path) => path,
            Err(failure) => return failure,
        };

        let entries = match std::fs::read_dir(&path) {
            Ok(entries) => entries,
            Err(e) => return Self::err_pair(Value::list(Vec::new()), e.to_string()),
        };

        let mut names: Vec<String> = Vec::new();
        for entry in entries {
            match entry {
                Ok(entry) => names.push(entry.file_name().to_string_lossy().to_string()),
                Err(e) => return Self::err_pair(Value::list(Vec::new()), e.to_string()),
            }
        }
        names.sort();

        Self::ok_pair(Value::list(
            names.into_iter().map(|n| Value::string(&n)).collect(),
        ))
    }

    // -- bytes -----------------------------------------------------------

    /// The one bytes argument these start with.
    fn bytes_arg(&self, name: &str, args: &[Value], call_pos: &Position) -> Result<Rc<Bytes>, RuntimeResult> {
        let fail = |detail: String| {
            RuntimeResult::new().failure(
                RuntimeError::new(call_pos.clone(), call_pos.clone(), &detail, None)
                    .with_code("XEN001")
                    .with_name("Type Mismatch")
                    .with_help("convert text first, e.g. `text as bytes`")
                    .base,
            )
        };

        match args.first() {
            Some(Value::Bytes(raw)) => Ok(raw.clone()),
            Some(other) => Err(fail(format!(
                "{} expects bytes, found {}",
                name,
                Value::get_type_name(other)
            ))),
            None => Err(fail(format!("{} expects bytes", name))),
        }
    }

    fn bytes_slice(&self, args: Vec<Value>, call_pos: Position) -> RuntimeResult {
        let raw = match self.bytes_arg("bytes_slice", &args, &call_pos) {
            Ok(raw) => raw,
            Err(failure) => return failure,
        };

        let bound = |index: usize| match args.get(index) {
            Some(Value::Int(n)) => Some(*n),
            _ => None,
        };

        let (Some(start), Some(end)) = (bound(1), bound(2)) else {
            return RuntimeResult::new().failure(
                RuntimeError::new(
                    call_pos.clone(),
                    call_pos,
                    "bytes_slice expects a start and an end, both ints",
                    None,
                )
                .with_code("XEN001")
                .with_name("Type Mismatch")
                .base,
            );
        };

        RuntimeResult::new().success(Value::Bytes(Rc::new(raw.slice(start, end))))
    }

    fn bytes_to_string(&self, args: Vec<Value>, call_pos: Position) -> RuntimeResult {
        let raw = match self.bytes_arg("bytes_to_string", &args, &call_pos) {
            Ok(raw) => raw,
            Err(failure) => return failure,
        };

        // Takes the buffer if this call holds the only reference, copies if the
        // caller still has one.
        let owned = Rc::try_unwrap(raw).unwrap_or_else(|shared| (*shared).clone());

        match String::from_utf8(owned.data) {
            Ok(text) => Self::ok_pair(Value::string_of(XenithString::new(text))),
            Err(e) => Self::err_pair(
                Value::string(""),
                format!(
                    "not valid UTF-8 (at byte {})",
                    e.utf8_error().valid_up_to()
                ),
            ),
        }
    }

    fn bytes_to_list(&self, args: Vec<Value>, call_pos: Position) -> RuntimeResult {
        let raw = match self.bytes_arg("bytes_to_list", &args, &call_pos) {
            Ok(raw) => raw,
            Err(failure) => return failure,
        };

        RuntimeResult::new().success(Value::list(
            raw.data.iter().map(|byte| Value::int(*byte as i64)).collect(),
        ))
    }

    fn bytes_from_list(&self, args: Vec<Value>, call_pos: Position) -> RuntimeResult {
        let Some(Value::List(codes)) = args.first() else {
            return RuntimeResult::new().failure(
                RuntimeError::new(
                    call_pos.clone(),
                    call_pos,
                    "bytes_from_list expects a list of ints",
                    None,
                )
                .with_code("XEN001")
                .with_name("Type Mismatch")
                .base,
            );
        };

        let mut data = Vec::with_capacity(codes.elements.len());
        for (index, element) in codes.elements.iter().enumerate() {
            let Value::Int(code) = element else {
                return Self::err_pair(
                    Value::bytes(Vec::new()),
                    format!(
                        "element {} is a {}, not an int",
                        index,
                        Value::get_type_name(element)
                    ),
                );
            };
            match u8::try_from(*code) {
                Ok(byte) => data.push(byte),
                Err(_) => {
                    return Self::err_pair(
                        Value::bytes(Vec::new()),
                        format!("element {} is {}, outside 0 to 255", index, code),
                    );
                }
            }
        }

        Self::ok_pair(Value::bytes(data))
    }

    // -- environment -----------------------------------------------------

    /// The one string argument the env builtins start with.
    fn env_name(&self, name: &str, args: &[Value], call_pos: &Position) -> Result<String, RuntimeResult> {
        let fail = |detail: String| {
            RuntimeResult::new().failure(
                RuntimeError::new(call_pos.clone(), call_pos.clone(), &detail, None)
                    .with_code("XEN001")
                    .with_name("Type Mismatch")
                    .base,
            )
        };

        match args.first() {
            Some(Value::String(text)) => Ok(text.value.clone()),
            Some(other) => Err(fail(format!(
                "{} expects a variable name as a string, found {}",
                name,
                Value::get_type_name(other)
            ))),
            None => Err(fail(format!("{} expects a variable name", name))),
        }
    }

    /// The value and whether it was set. Unset and empty are different, and a
    /// single string cannot tell them apart -- which is the bug every language
    /// that returns "" for both makes easy to write.
    fn env_get(&self, args: Vec<Value>, call_pos: Position) -> RuntimeResult {
        let name = match self.env_name("env_get", &args, &call_pos) {
            Ok(name) => name,
            Err(failure) => return failure,
        };

        match std::env::var(&name) {
            Ok(value) => RuntimeResult::new().success(Value::tuple(vec![
                Value::string_of(XenithString::new(value)),
                Value::Bool(true),
            ])),
            Err(_) => RuntimeResult::new()
                .success(Value::tuple(vec![Value::string(""), Value::Bool(false)])),
        }
    }

    fn env_set(&self, args: Vec<Value>, call_pos: Position) -> RuntimeResult {
        let name = match self.env_name("env_set", &args, &call_pos) {
            Ok(name) => name,
            Err(failure) => return failure,
        };

        let Some(Value::String(value)) = args.get(1) else {
            return RuntimeResult::new().failure(
                RuntimeError::new(
                    call_pos.clone(),
                    call_pos,
                    "env_set expects the value as a string",
                    None,
                )
                .with_code("XEN001")
                .with_name("Type Mismatch")
                .base,
            );
        };

        // Xenith is single threaded, which is the condition this is unsafe
        // without. Revisit when it is not.
        unsafe { std::env::set_var(&name, &value.value) };
        RuntimeResult::new().success(Value::Null)
    }

    fn env_unset(&self, args: Vec<Value>, call_pos: Position) -> RuntimeResult {
        let name = match self.env_name("env_unset", &args, &call_pos) {
            Ok(name) => name,
            Err(failure) => return failure,
        };

        unsafe { std::env::remove_var(&name) };
        RuntimeResult::new().success(Value::Null)
    }

    /// A name whose value is not valid UTF-8 is dropped rather than reported.
    /// `std::env::vars` skips those already; this is a note that it does.
    ///
    /// No sorting here: `Map` sorts in `keys`, `values`, `items` and iteration,
    /// so every way of walking the result already agrees.
    fn env_vars(&self) -> RuntimeResult {
        let mut map = Map::new();
        for (name, value) in std::env::vars() {
            map.set(name, Value::string_of(XenithString::new(value)));
        }
        RuntimeResult::new().success(Value::Map(Box::new(map)))
    }

    /// The command line as the process received it, minus the interpreter
    /// itself, so `args()[0]` is the program being run and the rest is what the
    /// user typed after it -- which is what a script wants and what `os.Args`
    /// gives in Go.
    fn env_args(&self) -> RuntimeResult {
        let args: Vec<Value> = std::env::args()
            .skip(1)
            .map(|arg| Value::string_of(XenithString::new(arg)))
            .collect();
        RuntimeResult::new().success(Value::list(args))
    }

    fn env_cwd(&self) -> RuntimeResult {
        match std::env::current_dir() {
            Ok(path) => Self::ok_pair(Value::string_of(XenithString::new(
                path.to_string_lossy().to_string(),
            ))),
            Err(e) => Self::err_pair(Value::string(""), e.to_string()),
        }
    }

    fn env_exit(&self, args: Vec<Value>, call_pos: Position) -> RuntimeResult {
        let code = match args.first() {
            Some(Value::Int(code)) => *code,
            Some(other) => {
                return RuntimeResult::new().failure(
                    RuntimeError::new(
                        call_pos.clone(),
                        call_pos,
                        &format!(
                            "env_exit expects an int status, found {}",
                            Value::get_type_name(other)
                        ),
                        None,
                    )
                    .with_code("XEN001")
                    .with_name("Type Mismatch")
                    .base,
                );
            }
            None => 0,
        };

        io::stdout().flush().ok();
        std::process::exit(code as i32);
    }

    /// The characters of `text` from `start` up to but not including `end`.
    ///
    /// Both ends are clamped, and a start past the end gives an empty string,
    /// so this never fails. Slicing is used constantly when writing string code
    /// in the language, and a version that could error would mean a bounds
    /// check at every call site.
    fn substring(&self, args: Vec<Value>, call_pos: Position) -> RuntimeResult {
        let fail = |detail: &str| {
            RuntimeResult::new().failure(
                RuntimeError::new(call_pos.clone(), call_pos.clone(), detail, None)
                    .with_code("XEN001")
                    .with_name("Type Mismatch")
                    .base,
            )
        };

        if args.len() != 3 {
            return fail("substring expects 3 arguments (string, start, end)");
        }
        let Value::String(text) = &args[0] else {
            return fail("substring expects a string as its first argument");
        };
        let (Some(start), Some(end)) = (args[1].as_number(), args[2].as_number()) else {
            return fail("substring expects int positions");
        };

        let length = text.char_len() as i64;

        let (Some(start), Some(end)) = (start.to_i64(), end.to_i64()) else {
            return fail("substring positions must be whole numbers");
        };
        let from = start.clamp(0, length) as usize;
        let to = end.clamp(0, length) as usize;

        // `slice` is a byte range on ASCII, so taking a short piece out of a
        // long string does not walk the whole of it. Collecting every character
        // into a `Vec<char>` first, as this used to, made `substring` cost the
        // length of its input rather than the length of its output.
        let slice: String = if from >= to {
            String::new()
        } else {
            text.slice(from, to)
        };

        RuntimeResult::new().success(Value::string_of(XenithString::new(slice)))
    }

    /// The Unicode code point at an index, which is what lets classification
    /// and case conversion be written in the language.
    fn code_at(&self, args: Vec<Value>, call_pos: Position) -> RuntimeResult {
        let fail = |detail: &str| {
            RuntimeResult::new().failure(
                RuntimeError::new(call_pos.clone(), call_pos.clone(), detail, None)
                    .with_code("XEN001")
                    .with_name("Type Mismatch")
                    .base,
            )
        };

        if args.len() != 2 {
            return fail("code_at expects 2 arguments (string, index)");
        }
        let Value::String(text) = &args[0] else {
            return fail("code_at expects a string as its first argument");
        };
        let Some(index) = args[1].as_number() else {
            return fail("code_at expects an int index");
        };

        let Some(position) = index.as_index() else {
            return fail("a string index must be a non-negative int");
        };

        match text.char_at(position) {
            Some(character) => RuntimeResult::new().success(Value::int(character as i64)),
            None => RuntimeResult::new().failure(Error::index_out_of_bounds(
                position,
                text.char_len(),
                call_pos.clone(),
                call_pos,
            )),
        }
    }

    /// The inverse of `code_at`.
    fn from_code(&self, args: Vec<Value>, call_pos: Position) -> RuntimeResult {
        let fail = |detail: &str| {
            RuntimeResult::new().failure(
                RuntimeError::new(call_pos.clone(), call_pos.clone(), detail, None)
                    .with_code("XEN001")
                    .with_name("Type Mismatch")
                    .base,
            )
        };

        if args.len() != 1 {
            return fail("from_code expects 1 argument (int)");
        }
        let Some(code) = args[0].as_number() else {
            return fail("from_code expects an int");
        };

        let Some(value) = code.to_i64() else {
            return fail("from_code expects a whole number");
        };
        let Some(character) = u32::try_from(value).ok().and_then(char::from_u32) else {
            return fail(&format!("{} is not a Unicode code point", value));
        };

        RuntimeResult::new()
            .success(Value::string_of(XenithString::new(character.to_string())))
    }

    fn len(&self, args: Vec<Value>, call_pos: Position) -> RuntimeResult {
        if args.len() != 1 {
            return RuntimeResult::new().failure(
                RuntimeError::new(call_pos.clone(), call_pos, "len expects 1 argument", None).base,
            );
        }

        match &args[0] {
            Value::List(list) => {
                RuntimeResult::new().success(Value::int(list.elements.len() as i64))
            }
            Value::String(s) => {
                RuntimeResult::new().success(Value::int(s.char_len() as i64))
            }
            // Bytes, not characters. `len` on a string counts characters, and
            // the whole point of holding something as bytes is that its length
            // in bytes is the length that matters.
            Value::Bytes(raw) => RuntimeResult::new().success(Value::int(raw.len() as i64)),
            // The registry has always documented `len` as working on a map; it
            // did not, and `map.len()` was the only way to ask.
            Value::Map(map) => RuntimeResult::new().success(Value::int(map.len() as i64)),
            _ => RuntimeResult::new().failure(
                RuntimeError::new(
                    call_pos.clone(),
                    call_pos,
                    "len expects a list, map, string or bytes",
                    None,
                )
                .base,
            ),
        }
    }

    fn run(
        &self,
        args: Vec<Value>,
        interpreter: &mut Interpreter,
        call_pos: Position,
    ) -> RuntimeResult {
        if args.len() != 1 {
            return RuntimeResult::new().failure(
                RuntimeError::new(call_pos.clone(), call_pos, "run expects 1 argument", None).base,
            );
        }

        let filename = match &args[0] {
            Value::String(s) => &s.value,
            _ => {
                return RuntimeResult::new().failure(
                    RuntimeError::new(
                        call_pos.clone(),
                        call_pos,
                        "run expects a string filename",
                        None,
                    )
                    .base,
                );
            }
        };

        match std::fs::read_to_string(filename) {
            Ok(source) => match crate::run(filename, &source) {
                Ok(_) => RuntimeResult::new().success(Value::Null),
                Err(e) => RuntimeResult::new().failure(e),
            },
            Err(e) => RuntimeResult::new().failure(
                RuntimeError::new(
                    call_pos.clone(),
                    call_pos,
                    &format!("Failed to load script \"{}\": {}", filename, e),
                    None,
                )
                .base,
            ),
        }
    }
}

/// Map runtime value.
///
/// Behind an `Rc` for the same reason as [`List`]: cloning is a refcount bump,
/// and `pairs_mut` copies only when the pairs are shared. Without it,
/// `m[key] = value` copied the whole map every time and filling one was
/// quadratic.
#[derive(Debug, Clone)]
pub struct Map {
    pub pairs: Rc<HashMap<String, Value>>,
}

impl Map {
    pub fn new() -> Self {
        Self {
            pairs: Rc::new(HashMap::new()),
        }
    }

    /// The pairs, to write to. Copies them first if this map is not the only
    /// holder.
    pub fn pairs_mut(&mut self) -> &mut HashMap<String, Value> {
        Rc::make_mut(&mut self.pairs)
    }

    pub fn get(&self, key: &str) -> Option<&Value> {
        self.pairs.get(key)
    }

    pub fn set(&mut self, key: String, value: Value) {
        self.pairs_mut().insert(key, value);
    }

    pub fn remove(&mut self, key: &str) -> Option<Value> {
        self.pairs_mut().remove(key)
    }

    pub fn len(&self) -> usize {
        self.pairs.len()
    }

    pub fn contains_key(&self, key: &str) -> bool {
        self.pairs.contains_key(key)
    }

    pub fn items(&self) -> List {
        let mut pairs: Vec<(&String, &Value)> = self.pairs.iter().collect();
        pairs.sort_by(|a, b| a.0.cmp(b.0));

        let items = pairs
            .into_iter()
            .map(|(key, value)| {
                Value::List(List::new(vec![
                    Value::string_of(XenithString::new(key.clone())),
                    value.clone(),
                ]))
            })
            .collect();

        List::new(items)
    }

    pub fn keys(&self) -> List {
        let mut keys: Vec<String> = self.pairs.keys().cloned().collect();
        keys.sort();
        let mut result = Vec::new();
        for key in keys {
            result.push(Value::string_of(XenithString::new(key)));
        }
        List::new(result)
    }

    pub fn values(&self) -> List {
        let mut values: Vec<(&String, &Value)> = self.pairs.iter().collect();
        values.sort_by(|a, b| a.0.cmp(b.0));
        let mut result = Vec::new();
        for (_, value) in values {
            result.push(value.clone());
        }
        List::new(result)
    }
}

impl Default for Map {
    fn default() -> Self {
        Self::new()
    }
}


/// Struct instance
#[derive(Debug, Clone)]
pub struct Struct {
    pub name: String,
    pub fields: HashMap<String, Value>,
}

impl Struct {
    pub fn new(name: String) -> Self {
        Self {
            name,
            fields: HashMap::new(),
        }
    }

    pub fn get_field(&self, name: &str) -> Option<&Value> {
        self.fields.get(name)
    }

    pub fn set_field(&mut self, name: String, value: Value) {
        self.fields.insert(name, value);
    }
}
