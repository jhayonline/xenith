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
    Number(Number),
    String(XenithString),
    List(List),
    Function(Box<Function>),
    BuiltInFunction(BuiltInFunction),
    /// Boxed: a `Map` holds a `HashMap`, which is 48 bytes inline and made
    /// every `Value` that size whether or not it was a map.
    Map(Box<Map>),
    /// Boxed for the same reason: a `String` plus a `HashMap` is 72 bytes.
    Struct(Box<Struct>),
    Bool(bool),
    Tuple(Vec<Value>),
    Null,
}

impl Value {
    /// Creates an int value
    pub fn int(i: i64) -> Self {
        Value::Number(Number::Int(i))
    }

    /// Creates a float value
    pub fn float(f: f64) -> Self {
        Value::Number(Number::Float(f))
    }

    /// Creates a string value
    pub fn string(s: &str) -> Self {
        Value::String(XenithString::new(s.to_string()))
    }

    /// Creates a list value
    pub fn list(elements: Vec<Value>) -> Self {
        Value::List(List::new(elements))
    }

    /// Checks if the value is truthy
    pub fn is_true(&self) -> bool {
        match self {
            Value::Number(n) => !n.is_zero(),
            Value::String(s) => !s.value.is_empty(),
            Value::List(l) => !l.elements.is_empty(),
            Value::Map(m) => !m.pairs.is_empty(),
            Value::Struct(s) => !s.fields.is_empty(),
            Value::Function(_) => true,
            Value::BuiltInFunction(_) => true,
            Value::Bool(b) => *b,
            Value::Null => false,
            Value::Tuple(t) => !t.is_empty(),
        }
    }

    /// Tries to get as a number
    pub fn as_number(&self) -> Option<&Number> {
        match self {
            Value::Number(n) => Some(n),
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
        Value::Tuple(elements)
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
            (Value::Number(a), Value::Number(b)) => match (a, b) {
                (Number::Int(x), Number::Int(y)) => x
                    .checked_add(*y)
                    .map(Value::int)
                    .ok_or_else(|| Self::overflow_err("addition")),
                (Number::Float(x), Number::Float(y)) => Ok(Value::float(x + y)),
                _ => Err(Self::mixed_err("add", a, b)),
            },
            (Value::String(a), Value::String(b)) => {
                let mut new = a.value.clone();
                new.push_str(&b.value);
                Ok(Value::String(XenithString::new(new)))
            }
            (Value::List(a), Value::List(b)) => {
                let mut new = a.clone();
                new.elements.extend(b.elements.clone());
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
            (Value::Number(a), Value::Number(b)) => match (a, b) {
                (Number::Int(x), Number::Int(y)) => x
                    .checked_sub(*y)
                    .map(Value::int)
                    .ok_or_else(|| Self::overflow_err("subtraction")),
                (Number::Float(x), Number::Float(y)) => Ok(Value::float(x - y)),
                _ => Err(Self::mixed_err("subtract", a, b)),
            },
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
            (Value::Number(a), Value::Number(b)) => match (a, b) {
                (Number::Int(x), Number::Int(y)) => x
                    .checked_mul(*y)
                    .map(Value::int)
                    .ok_or_else(|| Self::overflow_err("multiplication")),
                (Number::Float(x), Number::Float(y)) => Ok(Value::float(x * y)),
                _ => Err(Self::mixed_err("multiply", a, b)),
            },
            (Value::String(a), Value::Number(Number::Int(n))) => {
                if *n < 0 {
                    return Err(Self::arith_err("cannot repeat a string a negative number of times"));
                }
                Ok(Value::String(XenithString::new(a.value.repeat(*n as usize))))
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
            (Value::Number(a), Value::Number(b)) => match (a, b) {
                (Number::Int(_), Number::Int(0)) => Err(RuntimeError::new(
                    Self::dummy_pos(),
                    Self::dummy_pos(),
                    "division by zero",
                    None,
                )
                .with_code("XEN003")
                .with_name("Division by Zero")
                .base),
                (Number::Int(x), Number::Int(y)) => x
                    .checked_div(*y)
                    .map(Value::int)
                    .ok_or_else(|| Self::overflow_err("division")),
                (Number::Float(x), Number::Float(y)) => {
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
                _ => Err(Self::mixed_err("divide", a, b)),
            },
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
            (Value::Number(a), Value::Number(b)) => match (a, b) {
                (Number::Int(_), Number::Int(0)) => Err(RuntimeError::new(
                    Self::dummy_pos(),
                    Self::dummy_pos(),
                    "remainder by zero",
                    None,
                )
                .with_code("XEN003")
                .with_name("Division by Zero")
                .base),
                (Number::Int(x), Number::Int(y)) => x
                    .checked_rem(*y)
                    .map(Value::int)
                    .ok_or_else(|| Self::overflow_err("remainder")),
                (Number::Float(x), Number::Float(y)) => Ok(Value::float(x % y)),
                _ => Err(Self::mixed_err("take the remainder of", a, b)),
            },
            _ => Err(Self::arith_err("cannot take the remainder of these types")),
        }
    }

    /// Power operation
    pub fn power(&self, other: &Value) -> Result<Value, Error> {
        match (self, other) {
            (Value::Number(a), Value::Number(b)) => match (a, b) {
                (Number::Int(x), Number::Int(y)) => {
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
                (Number::Float(x), Number::Float(y)) => Ok(Value::float(x.powf(*y))),
                _ => Err(Self::mixed_err("raise", a, b)),
            },
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

    fn eq_value(&self, other: &Value) -> bool {
        match (self, other) {
            (Value::Number(a), Value::Number(b)) => match (a, b) {
                (Number::Int(x), Number::Int(y)) => x == y,
                (Number::Float(x), Number::Float(y)) => x == y,
                _ => false,
            },
            (Value::String(a), Value::String(b)) => a.value == b.value,
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
            _ => false,
        }
    }

    /// Not equals comparison
    pub fn not_equals(&self, other: &Value) -> Result<Value, Error> {
        Ok(Value::Bool(!self.eq_value(other)))
    }

    fn compare(&self, other: &Value, op: &str) -> Result<std::cmp::Ordering, Error> {
        match (self, other) {
            (Value::Number(a), Value::Number(b)) => match (a, b) {
                (Number::Int(x), Number::Int(y)) => Ok(x.cmp(y)),
                (Number::Float(x), Number::Float(y)) => x
                    .partial_cmp(y)
                    .ok_or_else(|| Self::arith_err("cannot compare NaN")),
                _ => Err(Self::mixed_err("compare", a, b)),
            },
            (Value::String(a), Value::String(b)) => Ok(a.value.cmp(&b.value)),
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
            Value::Number(Number::Int(n)) => n
                .checked_neg()
                .map(Value::int)
                .ok_or_else(|| Self::overflow_err("negation")),
            Value::Number(Number::Float(f)) => Ok(Value::float(-f)),
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
            Type::Int => matches!(value, Value::Number(Number::Int(_))),
            Type::Float => matches!(value, Value::Number(Number::Float(_))),
            Type::String => matches!(value, Value::String(_)),
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
            Type::Struct(name, _) => matches!(value, Value::Struct(s) if &s.name == name),
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
            Value::Number(n) => n.type_name().to_string(),
            Value::String(_) => "string".to_string(),
            Value::Bool(_) => "bool".to_string(),
            Value::List(_) => "list".to_string(),
            Value::Map(_) => "map".to_string(),
            Value::Struct(s) => s.name.clone(),
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
}

impl XenithString {
    pub fn new(value: String) -> Self {
        Self { value }
    }
}

/// List runtime value
#[derive(Debug, Clone)]
pub struct List {
    pub elements: Vec<Value>,
}

impl List {
    pub fn new(elements: Vec<Value>) -> Self {
        Self { elements }
    }

    pub fn append(&mut self, value: Value) {
        self.elements.push(value);
    }

    pub fn pop(&mut self, index: Option<usize>) -> Option<Value> {
        let idx = index.unwrap_or(self.elements.len() - 1);
        if idx < self.elements.len() {
            Some(self.elements.remove(idx))
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
            self.elements[index] = value;
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

/// Built-in function
#[derive(Debug, Clone)]
pub struct BuiltInFunction {
    pub name: String,
}

impl BuiltInFunction {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
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
        match self.name.as_str() {
            "echo" => self.echo(args, call_pos),
            "ret" => self.ret(args, call_pos),
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
            "code_at" => self.code_at(args, call_pos),
            "from_code" => self.from_code(args, call_pos),
            "format" => crate::builtins::format::format(args, interpreter, call_pos, context),
            // =================================================================================
            _ => RuntimeResult::new().failure(
                RuntimeError::new(
                    call_pos.clone(),
                    call_pos,
                    &format!("Unknown built-in function: {}", self.name),
                    None,
                )
                .base,
            ),
        }
    }

    fn echo(&self, args: Vec<Value>, _call_pos: Position) -> RuntimeResult {
        if let Some(arg) = args.first() {
            match arg {
                Value::Number(n) => print!("{}", n),
                Value::String(s) => print!("{}", s.value),
                Value::Bool(b) => print!("{}", b),
                Value::Null => print!("null"),
                Value::List(l) => {
                    print!("[");
                    for (i, elem) in l.elements.iter().enumerate() {
                        if i > 0 {
                            print!(", ");
                        }
                        match elem {
                            Value::Number(n) => print!("{}", n),
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
                            Value::Number(n) => print!("{}", n),
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
                Value::Function(f) => {
                    if let Some(name) = &f.name {
                        print!("<function {}>", name);
                    } else {
                        print!("<anonymous function>");
                    }
                }
                Value::BuiltInFunction(b) => {
                    print!("<built-in function {}>", b.name);
                }
                Value::Tuple(t) => {
                    print!("(");
                    for (i, elem) in t.iter().enumerate() {
                        if i > 0 {
                            print!(", ");
                        }
                        // Recursively print element - would need proper handling
                        match elem {
                            Value::Number(n) => print!("{}", n),
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

    fn ret(&self, args: Vec<Value>, _call_pos: Position) -> RuntimeResult {
        if let Some(arg) = args.first() {
            RuntimeResult::new().success(Value::String(XenithString::new(value_to_string(arg))))
        } else {
            RuntimeResult::new().success(Value::String(XenithString::new("".to_string())))
        }
    }

    fn input(&self, _call_pos: Position) -> RuntimeResult {
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        RuntimeResult::new().success(Value::String(XenithString::new(input.trim().to_string())))
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
        let result = matches!(args.first(), Some(Value::Number(_)));
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
                new_list.elements.push(value.clone());
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
            (Value::List(list), Value::Number(idx)) => {
                let Some(idx_usize) = idx.as_index() else {
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
                let popped = new_list.elements.remove(idx_usize);
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
                new_list.elements.extend(list_b.elements.clone());
                RuntimeResult::new().success(Value::List(new_list))
            }
            _ => RuntimeResult::new().failure(
                RuntimeError::new(call_pos.clone(), call_pos, "extend expects two lists", None)
                    .base,
            ),
        }
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
        let (Value::Number(start), Value::Number(end)) = (&args[1], &args[2]) else {
            return fail("substring expects int positions");
        };

        let characters: Vec<char> = text.value.chars().collect();
        let length = characters.len() as i64;

        let (Some(start), Some(end)) = (start.to_i64(), end.to_i64()) else {
            return fail("substring positions must be whole numbers");
        };
        let from = start.clamp(0, length) as usize;
        let to = end.clamp(0, length) as usize;

        let slice: String = if from >= to {
            String::new()
        } else {
            characters[from..to].iter().collect()
        };

        RuntimeResult::new().success(Value::String(XenithString::new(slice)))
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
        let Value::Number(index) = &args[1] else {
            return fail("code_at expects an int index");
        };

        let Some(position) = index.as_index() else {
            return fail("a string index must be a non-negative int");
        };

        match text.value.chars().nth(position) {
            Some(character) => RuntimeResult::new().success(Value::int(character as i64)),
            None => RuntimeResult::new().failure(Error::index_out_of_bounds(
                position,
                text.value.chars().count(),
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
        let Value::Number(code) = &args[0] else {
            return fail("from_code expects an int");
        };

        let Some(value) = code.to_i64() else {
            return fail("from_code expects a whole number");
        };
        let Some(character) = u32::try_from(value).ok().and_then(char::from_u32) else {
            return fail(&format!("{} is not a Unicode code point", value));
        };

        RuntimeResult::new()
            .success(Value::String(XenithString::new(character.to_string())))
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
                RuntimeResult::new().success(Value::int(s.value.chars().count() as i64))
            }
            _ => RuntimeResult::new().failure(
                RuntimeError::new(
                    call_pos.clone(),
                    call_pos,
                    "len expects a list or string",
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

#[derive(Debug, Clone)]
pub struct Map {
    pub pairs: HashMap<String, Value>,
}

impl Map {
    pub fn new() -> Self {
        Self {
            pairs: HashMap::new(),
        }
    }

    pub fn get(&self, key: &str) -> Option<&Value> {
        self.pairs.get(key)
    }

    pub fn set(&mut self, key: String, value: Value) {
        self.pairs.insert(key, value);
    }

    pub fn remove(&mut self, key: &str) -> Option<Value> {
        self.pairs.remove(key)
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
                    Value::String(XenithString::new(key.clone())),
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
            result.push(Value::String(XenithString::new(key)));
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
