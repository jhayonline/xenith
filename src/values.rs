// values.rs
//! # Runtime Values Module
//!
//! Defines the runtime object system for Xenith including Numbers, Strings,
//! Lists, and Functions. Implements operations between values (arithmetic,
//! comparison, logical) and provides the foundation for the interpreter's
//! execution semantics.

use std::collections::HashMap;
use std::io::{self, Write};

use crate::context::Context;
use crate::error::{Error, RuntimeError};
use crate::interpreter::Interpreter;
use crate::json::Json;
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
    Map(Map),
    Struct(Struct),
    Bool(bool),
    Json(Json),
}

impl Value {
    /// Creates a number value
    pub fn number(f: f64) -> Self {
        Value::Number(Number::new(f))
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
            Value::Number(n) => n.value != 0.0,
            Value::String(s) => !s.value.is_empty(),
            Value::List(l) => !l.elements.is_empty(),
            Value::Map(m) => !m.pairs.is_empty(),
            Value::Struct(s) => !s.fields.is_empty(),
            Value::Function(_) => true,
            Value::BuiltInFunction(_) => true,
            Value::Bool(b) => *b,
            Value::Json(j) => !j.is_null(),
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

    /// Addition operation
    pub fn add(&self, other: &Value) -> Result<Value, Error> {
        match (self, other) {
            (Value::Number(a), Value::Number(b)) => {
                Ok(Value::Number(Number::new(a.value + b.value)))
            }
            (Value::String(a), Value::String(b)) => {
                let mut new = a.value.clone();
                new.push_str(&b.value);
                Ok(Value::String(XenithString::new(new)))
            }
            (Value::String(a), Value::Number(b)) => {
                let mut new = a.value.clone();
                new.push_str(&b.value.to_string());
                Ok(Value::String(XenithString::new(new)))
            }
            (Value::Number(a), Value::String(b)) => {
                let mut new = a.value.to_string();
                new.push_str(&b.value);
                Ok(Value::String(XenithString::new(new)))
            }
            (Value::List(a), Value::List(b)) => {
                let mut new = a.clone();
                new.elements.extend(b.elements.clone());
                Ok(Value::List(new))
            }
            (Value::List(a), b) => {
                let mut new = a.clone();
                new.elements.push(b.clone());
                Ok(Value::List(new))
            }
            _ => Err(RuntimeError::new(
                Self::dummy_pos(),
                Self::dummy_pos(),
                "Cannot add these types",
                None,
            )
            .base),
        }
    }

    /// Subtraction operation
    pub fn subtract(&self, other: &Value) -> Result<Value, Error> {
        match (self, other) {
            (Value::Number(a), Value::Number(b)) => {
                Ok(Value::Number(Number::new(a.value - b.value)))
            }
            _ => Err(RuntimeError::new(
                Self::dummy_pos(),
                Self::dummy_pos(),
                "Cannot subtract these types",
                None,
            )
            .base),
        }
    }

    /// Multiplication operation
    pub fn multiply(&self, other: &Value) -> Result<Value, Error> {
        match (self, other) {
            (Value::Number(a), Value::Number(b)) => {
                Ok(Value::Number(Number::new(a.value * b.value)))
            }
            (Value::String(a), Value::Number(b)) => {
                let repeated = a.value.repeat(b.value as usize);
                Ok(Value::String(XenithString::new(repeated)))
            }
            (Value::List(a), Value::List(b)) => {
                let mut new = a.clone();
                new.elements.extend(b.elements.clone());
                Ok(Value::List(new))
            }
            _ => Err(RuntimeError::new(
                Self::dummy_pos(),
                Self::dummy_pos(),
                "Cannot multiply these types",
                None,
            )
            .base),
        }
    }

    /// Division operation
    pub fn divide(&self, other: &Value) -> Result<Value, Error> {
        match (self, other) {
            (Value::Number(a), Value::Number(b)) => {
                if b.value == 0.0 {
                    return Err(RuntimeError::new(
                        Self::dummy_pos(),
                        Self::dummy_pos(),
                        "Division by zero",
                        None,
                    )
                    .base);
                }
                Ok(Value::Number(Number::new(a.value / b.value)))
            }
            (Value::List(a), Value::Number(b)) => {
                let idx = b.value as usize;
                if idx >= a.elements.len() {
                    return Err(RuntimeError::new(
                        Self::dummy_pos(),
                        Self::dummy_pos(),
                        "List index out of bounds",
                        None,
                    )
                    .base);
                }
                Ok(a.elements[idx].clone())
            }
            _ => Err(RuntimeError::new(
                Self::dummy_pos(),
                Self::dummy_pos(),
                "Cannot divide these types",
                None,
            )
            .base),
        }
    }

    /// Power operation
    pub fn power(&self, other: &Value) -> Result<Value, Error> {
        match (self, other) {
            (Value::Number(a), Value::Number(b)) => {
                Ok(Value::Number(Number::new(a.value.powf(b.value))))
            }
            _ => Err(RuntimeError::new(
                Self::dummy_pos(),
                Self::dummy_pos(),
                "Cannot raise to power these types",
                None,
            )
            .base),
        }
    }

    /// Equality comparison
    pub fn equals(&self, other: &Value) -> Result<Value, Error> {
        let result = match (self, other) {
            (Value::Number(a), Value::Number(b)) => (a.value - b.value).abs() < 1e-10,
            (Value::String(a), Value::String(b)) => a.value == b.value,
            (Value::List(a), Value::List(b)) => {
                if a.elements.len() != b.elements.len() {
                    false
                } else {
                    a.elements
                        .iter()
                        .zip(b.elements.iter())
                        .all(|(x, y)| match x.equals(y) {
                            Ok(Value::Number(n)) => n.value != 0.0,
                            _ => false,
                        })
                }
            }
            (Value::Bool(a), Value::Bool(b)) => a == b,
            _ => false,
        };
        Ok(Value::Number(Number::new(if result { 1.0 } else { 0.0 })))
    }

    /// Not equals comparison
    pub fn not_equals(&self, other: &Value) -> Result<Value, Error> {
        let eq = self.equals(other)?;
        match eq {
            Value::Number(n) => Ok(Value::Number(Number::new(if n.value == 0.0 {
                1.0
            } else {
                0.0
            }))),
            _ => Ok(Value::Number(Number::new(1.0))),
        }
    }

    /// Less than comparison
    pub fn less_than(&self, other: &Value) -> Result<Value, Error> {
        match (self, other) {
            (Value::Number(a), Value::Number(b)) => {
                Ok(Value::Number(Number::new(if a.value < b.value {
                    1.0
                } else {
                    0.0
                })))
            }
            (Value::String(a), Value::String(b)) => {
                Ok(Value::Number(Number::new(if a.value < b.value {
                    1.0
                } else {
                    0.0
                })))
            }
            _ => Err(RuntimeError::new(
                Self::dummy_pos(),
                Self::dummy_pos(),
                "Cannot compare these types",
                None,
            )
            .base),
        }
    }

    /// Greater than comparison
    pub fn greater_than(&self, other: &Value) -> Result<Value, Error> {
        match (self, other) {
            (Value::Number(a), Value::Number(b)) => {
                Ok(Value::Number(Number::new(if a.value > b.value {
                    1.0
                } else {
                    0.0
                })))
            }
            (Value::String(a), Value::String(b)) => {
                Ok(Value::Number(Number::new(if a.value > b.value {
                    1.0
                } else {
                    0.0
                })))
            }
            _ => Err(RuntimeError::new(
                Self::dummy_pos(),
                Self::dummy_pos(),
                "Cannot compare these types",
                None,
            )
            .base),
        }
    }

    /// Less than or equal comparison
    pub fn less_than_or_equal(&self, other: &Value) -> Result<Value, Error> {
        match (self, other) {
            (Value::Number(a), Value::Number(b)) => {
                Ok(Value::Number(Number::new(if a.value <= b.value {
                    1.0
                } else {
                    0.0
                })))
            }
            (Value::String(a), Value::String(b)) => {
                Ok(Value::Number(Number::new(if a.value <= b.value {
                    1.0
                } else {
                    0.0
                })))
            }
            _ => Err(RuntimeError::new(
                Self::dummy_pos(),
                Self::dummy_pos(),
                "Cannot compare these types",
                None,
            )
            .base),
        }
    }

    /// Greater than or equal comparison
    pub fn greater_than_or_equal(&self, other: &Value) -> Result<Value, Error> {
        match (self, other) {
            (Value::Number(a), Value::Number(b)) => Ok(Value::Bool(a.value >= b.value)),
            (Value::String(a), Value::String(b)) => Ok(Value::Bool(a.value >= b.value)),
            _ => Err(RuntimeError::new(
                Self::dummy_pos(),
                Self::dummy_pos(),
                "Cannot compare these types",
                None,
            )
            .base),
        }
    }

    /// Logical NOT
    pub fn logical_not(&self) -> Result<Value, Error> {
        match self {
            Value::Bool(b) => Ok(Value::Bool(!b)),
            Value::Number(n) => Ok(Value::Bool(n.value == 0.0)),
            _ => Ok(Value::Bool(!self.is_true())),
        }
    }

    /// Negative value
    pub fn negative(&self) -> Result<Value, Error> {
        match self {
            Value::Number(n) => Ok(Value::Number(Number::new(-n.value))),
            _ => Err(RuntimeError::new(
                Self::dummy_pos(),
                Self::dummy_pos(),
                "Cannot negate non-number",
                None,
            )
            .base),
        }
    }

    /// Logical AND operation
    pub fn anded_by(&self, other: &Value) -> Result<Value, Error> {
        let left_true = self.is_true();
        let right_true = other.is_true();
        Ok(Value::Number(Number::new(if left_true && right_true {
            1.0
        } else {
            0.0
        })))
    }

    /// Logical OR operation
    pub fn ored_by(&self, other: &Value) -> Result<Value, Error> {
        let left_true = self.is_true();
        let right_true = other.is_true();
        Ok(Value::Number(Number::new(if left_true || right_true {
            1.0
        } else {
            0.0
        })))
    }
}

/// Number runtime value
#[derive(Debug, Clone)]
pub struct Number {
    pub value: f64,
}

impl Number {
    pub fn new(value: f64) -> Self {
        Self { value }
    }

    pub fn null() -> Self {
        Self { value: 0.0 }
    }

    pub fn false_val() -> Self {
        Self { value: 0.0 }
    }

    pub fn true_val() -> Self {
        Self { value: 1.0 }
    }

    pub fn math_pi() -> Self {
        Self {
            value: std::f64::consts::PI,
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
    pub body_node: Box<Node>,
    pub arg_names: Vec<String>,
    pub param_types: Vec<Type>,
    pub should_auto_return: bool,
}

impl Function {
    pub fn new(
        name: Option<String>,
        body_node: Node,
        arg_names: Vec<String>,
        param_types: Vec<Type>,
        should_auto_return: bool,
    ) -> Self {
        Self {
            name,
            body_node: Box::new(body_node),
            arg_names,
            param_types,
            should_auto_return,
        }
    }

    pub fn execute(
        &self,
        args: Vec<Value>,
        context: Context,
        interpreter: &mut Interpreter,
        call_position: Position, // NEW
    ) -> RuntimeResult {
        let mut result = RuntimeResult::new();

        // Check argument count
        if args.len() != self.arg_names.len() {
            return RuntimeResult::new().failure(Error::too_few_arguments(
                self.arg_names.len(),
                args.len(),
                call_position.clone(),
                call_position,
            ));
        }

        // Check argument types
        for (i, (arg, expected_type)) in args.iter().zip(self.param_types.iter()).enumerate() {
            if !Self::value_matches_type(arg, expected_type) {
                return RuntimeResult::new().failure(Error::type_mismatch(
                    &expected_type.to_string(),
                    &Self::get_type_name(arg),
                    call_position.clone(),
                    call_position.clone(),
                ));
            }
        }

        // Create child context
        let mut func_context = context.create_child(
            self.name.as_deref().unwrap_or("<anonymous>"),
            call_position.clone(),
        );

        // Bind arguments in the function's local scope
        for (i, arg_name) in self.arg_names.iter().enumerate() {
            func_context
                .symbol_table
                .set_local(arg_name.clone(), args[i].clone());
        }

        // Execute function body
        let exec_result = interpreter.visit(&self.body_node, &mut func_context);

        // Handle return value
        if let Some(err) = exec_result.error {
            return RuntimeResult::new().failure(err);
        }

        if self.should_auto_return {
            if let Some(val) = exec_result.value {
                return RuntimeResult::new().success(val);
            }
        }

        if let Some(ret_val) = exec_result.func_return_value {
            return RuntimeResult::new().success(ret_val);
        }

        RuntimeResult::new().success(Value::Number(Number::null()))
    }

    pub fn value_matches_type(value: &Value, expected_type: &Type) -> bool {
        match (value, expected_type) {
            (Value::Number(n), Type::Int) => n.value.fract() == 0.0,
            (Value::Number(_), Type::Float) => true,
            (Value::String(_), Type::String) => true,
            (Value::Bool(_), Type::Bool) => true,
            // Permissive collection matching - type params are hints only
            (Value::List(_), Type::List(_)) => true,
            (Value::List(_), Type::Struct(name, _)) if name == "list" => true,
            (Value::Map(_), Type::Map(_, _)) => true,
            (Value::Map(_), Type::Struct(name, _)) if name == "map" => true,
            (Value::Struct(s), Type::Struct(name, _)) => &s.name == name,
            (Value::Json(_), Type::Json) => true,
            _ => false,
        }
    }

    pub fn get_type_name(value: &Value) -> String {
        match value {
            Value::Number(n) => {
                if n.value.fract() == 0.0 {
                    "int".to_string()
                } else {
                    "float".to_string()
                }
            }
            Value::String(_) => "string".to_string(),
            Value::Bool(_) => "bool".to_string(),
            Value::List(_) => "list".to_string(),
            Value::Map(_) => "map".to_string(),
            Value::Struct(s) => format!("struct {}", s.name),
            Value::Function(_) => "function".to_string(),
            Value::BuiltInFunction(_) => "builtin".to_string(),
            Value::Json(_) => "json".to_string(),
        }
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

    pub fn execute(
        &self,
        args: Vec<Value>,
        interpreter: &mut Interpreter,
        call_pos: Position,
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
            "format" => crate::builtins::format::format(args, interpreter, call_pos),

            // std::fs
            "__fs_read" => crate::builtins::fs::read(args, call_pos),
            "__fs_write" => crate::builtins::fs::write(args, call_pos),
            "__fs_append" => crate::builtins::fs::append(args, call_pos),
            "__fs_exists" => crate::builtins::fs::exists(args, call_pos),
            "__fs_is_file" => crate::builtins::fs::is_file(args, call_pos),
            "__fs_is_dir" => crate::builtins::fs::is_dir(args, call_pos),
            "__fs_mkdir" => crate::builtins::fs::mkdir(args, call_pos),
            "__fs_mkdir_all" => crate::builtins::fs::mkdir_all(args, call_pos),
            "__fs_remove" => crate::builtins::fs::remove(args, call_pos),
            "__fs_remove_all" => crate::builtins::fs::remove_all(args, call_pos),
            "__fs_list_dir" => crate::builtins::fs::list_dir(args, call_pos),
            "__fs_copy" => crate::builtins::fs::copy(args, call_pos),

            // std::path
            "__path_join" => crate::builtins::path::join(args, call_pos),
            "__path_basename" => crate::builtins::path::basename(args, call_pos),
            "__path_dirname" => crate::builtins::path::dirname(args, call_pos),
            "__path_extension" => crate::builtins::path::extension(args, call_pos),
            "__path_stem" => crate::builtins::path::stem(args, call_pos),
            "__path_is_absolute" => crate::builtins::path::is_absolute(args, call_pos),
            "__path_is_relative" => crate::builtins::path::is_relative(args, call_pos),
            "__path_absolute" => crate::builtins::path::absolute(args, call_pos),
            "__path_normalize" => crate::builtins::path::normalize(args, call_pos),
            "__path_components" => crate::builtins::path::components(args, call_pos),
            "__path_parent" => crate::builtins::path::parent(args, call_pos),

            // std::time
            "__time_timestamp" => crate::builtins::time::timestamp(args, call_pos),
            "__time_timestamp_ms" => crate::builtins::time::timestamp_ms(args, call_pos),
            "__time_sleep" => crate::builtins::time::sleep(args, call_pos),
            "__time_sleep_sec" => crate::builtins::time::sleep_sec(args, call_pos),
            "__time_duration_secs" => crate::builtins::time::duration_secs(args, call_pos),
            "__time_duration_ms" => crate::builtins::time::duration_ms(args, call_pos),

            // std::math
            "__math_sqrt" => crate::builtins::math::sqrt(args, call_pos),
            "__math_pow" => crate::builtins::math::pow(args, call_pos),
            "__math_sin" => crate::builtins::math::sin(args, call_pos),
            "__math_cos" => crate::builtins::math::cos(args, call_pos),
            "__math_tan" => crate::builtins::math::tan(args, call_pos),
            "__math_asin" => crate::builtins::math::asin(args, call_pos),
            "__math_acos" => crate::builtins::math::acos(args, call_pos),
            "__math_atan" => crate::builtins::math::atan(args, call_pos),
            "__math_atan2" => crate::builtins::math::atan2(args, call_pos),
            "__math_log" => crate::builtins::math::log(args, call_pos),
            "__math_log10" => crate::builtins::math::log10(args, call_pos),
            "__math_abs" => crate::builtins::math::abs(args, call_pos),
            "__math_min" => crate::builtins::math::min(args, call_pos),
            "__math_max" => crate::builtins::math::max(args, call_pos),
            "__math_clamp" => crate::builtins::math::clamp(args, call_pos),
            "__math_round" => crate::builtins::math::round(args, call_pos),
            "__math_floor" => crate::builtins::math::floor(args, call_pos),
            "__math_ceil" => crate::builtins::math::ceil(args, call_pos),
            "__math_trunc" => crate::builtins::math::trunc(args, call_pos),
            "__math_fract" => crate::builtins::math::fract(args, call_pos),
            "__math_radians" => crate::builtins::math::radians(args, call_pos),
            "__math_degrees" => crate::builtins::math::degrees(args, call_pos),
            "__math_sum" => crate::builtins::math::sum(args, call_pos),
            "__math_average" => crate::builtins::math::average(args, call_pos),

            // std::string
            "__string_split" => crate::builtins::string::split(args, call_pos),
            "__string_join" => crate::builtins::string::join(args, call_pos),
            "__string_trim" => crate::builtins::string::trim(args, call_pos),
            "__string_trim_start" => crate::builtins::string::trim_start(args, call_pos),
            "__string_trim_end" => crate::builtins::string::trim_end(args, call_pos),
            "__string_replace" => crate::builtins::string::replace(args, call_pos),
            "__string_contains" => crate::builtins::string::contains(args, call_pos),
            "__string_starts_with" => crate::builtins::string::starts_with(args, call_pos),
            "__string_ends_with" => crate::builtins::string::ends_with(args, call_pos),
            "__string_to_upper" => crate::builtins::string::to_upper(args, call_pos),
            "__string_to_lower" => crate::builtins::string::to_lower(args, call_pos),
            "__string_reverse" => crate::builtins::string::reverse(args, call_pos),

            // random
            "__rand_int" => crate::builtins::random::rand_int(args, call_pos),
            "__rand_int_range" => crate::builtins::random::rand_int_range(args, call_pos),
            "__rand_float" => crate::builtins::random::rand_float(args, call_pos),
            "__rand_float_range" => crate::builtins::random::rand_float_range(args, call_pos),
            "__rand_bool" => crate::builtins::random::rand_bool(args, call_pos),
            "__rand_choice" => crate::builtins::random::choice(args, call_pos),
            "__rand_shuffle" => crate::builtins::random::shuffle(args, call_pos),
            "__rand_uuid" => crate::builtins::random::uuid(args, call_pos),

            // std::json
            "__json_parse" => crate::builtins::json::parse(args, call_pos),
            "__json_stringify" => crate::builtins::json::stringify(args, call_pos),
            "__json_stringify_pretty" => crate::builtins::json::stringify_pretty(args, call_pos),
            "__json_get" => crate::builtins::json::get(args, call_pos),
            "__json_set" => crate::builtins::json::set(args, call_pos),
            "__json_has_key" => crate::builtins::json::has_key(args, call_pos),
            "__json_from_map" => crate::builtins::json::from_map(args, call_pos),
            "__json_null" => crate::builtins::json::null_value(args, call_pos),

            // std::dotenv
            "__dotenv_load" => crate::builtins::dotenv::load(args, call_pos),
            "__dotenv_load_file" => crate::builtins::dotenv::load_file(args, call_pos),
            "__dotenv_get" => crate::builtins::dotenv::get(args, call_pos),
            "__dotenv_get_or_default" => crate::builtins::dotenv::get_or_default(args, call_pos),
            "__dotenv_has" => crate::builtins::dotenv::has(args, call_pos),
            "__dotenv_set" => crate::builtins::dotenv::set(args, call_pos),
            "__dotenv_unset" => crate::builtins::dotenv::unset(args, call_pos),
            "__dotenv_vars" => crate::builtins::dotenv::vars(args, call_pos),

            // std::http
            "__http_get" => crate::builtins::http::get(args, call_pos),
            "__http_post" => crate::builtins::http::post(args, call_pos),
            "__http_put" => crate::builtins::http::put(args, call_pos),
            "__http_delete" => crate::builtins::http::delete(args, call_pos),
            "__http_patch" => crate::builtins::http::patch(args, call_pos),
            "__http_set_timeout" => crate::builtins::http::set_timeout(args, call_pos),
            "__http_set_user_agent" => crate::builtins::http::set_user_agent(args, call_pos),

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
                Value::Number(n) => print!("{}", n.value),
                Value::String(s) => print!("{}", s.value),
                Value::List(l) => {
                    print!("[");
                    for (i, elem) in l.elements.iter().enumerate() {
                        if i > 0 {
                            print!(", ");
                        }
                        match elem {
                            Value::Number(n) => print!("{}", n.value),
                            Value::String(s) => print!("\"{}\"", s.value),
                            _ => print!("?"),
                        }
                    }
                    print!("]");
                }
                _ => print!("?"),
            }
        }
        println!();
        io::stdout().flush().unwrap();
        RuntimeResult::new().success(Value::Number(Number::null()))
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
                return RuntimeResult::new().success(Value::Number(Number::new(num as f64)));
            }
            println!("'{}' must be an integer. Try again!", input.trim());
        }
    }

    fn clear(&self, _call_pos: Position) -> RuntimeResult {
        print!("\x1B[2J\x1B[1;1H");
        io::stdout().flush().unwrap();
        RuntimeResult::new().success(Value::Number(Number::null()))
    }

    fn is_num(&self, args: Vec<Value>, _call_pos: Position) -> RuntimeResult {
        let result = matches!(args.first(), Some(Value::Number(_)));
        RuntimeResult::new().success(Value::Number(Number::new(if result { 1.0 } else { 0.0 })))
    }

    fn is_str(&self, args: Vec<Value>, _call_pos: Position) -> RuntimeResult {
        let result = matches!(args.first(), Some(Value::String(_)));
        RuntimeResult::new().success(Value::Number(Number::new(if result { 1.0 } else { 0.0 })))
    }

    fn is_list(&self, args: Vec<Value>, _call_pos: Position) -> RuntimeResult {
        let result = matches!(args.first(), Some(Value::List(_)));
        RuntimeResult::new().success(Value::Number(Number::new(if result { 1.0 } else { 0.0 })))
    }

    fn is_fun(&self, args: Vec<Value>, _call_pos: Position) -> RuntimeResult {
        let result = matches!(
            args.first(),
            Some(Value::Function(_)) | Some(Value::BuiltInFunction(_))
        );
        RuntimeResult::new().success(Value::Number(Number::new(if result { 1.0 } else { 0.0 })))
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
                let idx_usize = idx.value as usize;
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

    fn len(&self, args: Vec<Value>, call_pos: Position) -> RuntimeResult {
        if args.len() != 1 {
            return RuntimeResult::new().failure(
                RuntimeError::new(call_pos.clone(), call_pos, "len expects 1 argument", None).base,
            );
        }

        match &args[0] {
            Value::List(list) => {
                RuntimeResult::new().success(Value::Number(Number::new(list.elements.len() as f64)))
            }
            Value::String(s) => {
                RuntimeResult::new().success(Value::Number(Number::new(s.value.len() as f64)))
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
                Ok(_) => RuntimeResult::new().success(Value::Number(Number::null())),
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

#[derive(Debug, Clone)]
pub struct CaughtError {
    pub message: String,
    pub error: Option<Box<Error>>,
}

impl CaughtError {
    pub fn from_error(error: Error) -> Self {
        let message = format!("{}: {}", error.error_name, error.details);
        Self {
            message,
            error: Some(Box::new(error)),
        }
    }

    pub fn from_message(message: String) -> Self {
        Self {
            message,
            error: None,
        }
    }

    pub fn to_string(&self) -> String {
        if let Some(error) = &self.error {
            error.as_string()
        } else {
            self.message.clone()
        }
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
