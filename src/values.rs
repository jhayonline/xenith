//! # Runtime Values Module
//!
//! Defines the runtime object system for Xenith including Numbers, Strings,
//! Lists, and Functions. Implements operations between values (arithmetic,
//! comparison, logical) and provides the foundation for the interpreter's
//! execution semantics.

use std::io::{self, Write};

use crate::context::Context;
use crate::error::{Error, RuntimeError};
use crate::interpreter::Interpreter;
use crate::nodes::Node;
use crate::runtime_result::RuntimeResult;

/// All possible runtime values in Xenith
#[derive(Debug, Clone)]
pub enum Value {
    Number(Number),
    String(XenithString),
    List(List),
    Function(Box<Function>),
    BuiltInFunction(BuiltInFunction),
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
            Value::Function(_) => true,
            Value::BuiltInFunction(_) => true,
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
            (Value::Number(a), Value::Number(b)) => {
                Ok(Value::Number(Number::new(if a.value >= b.value {
                    1.0
                } else {
                    0.0
                })))
            }
            (Value::String(a), Value::String(b)) => {
                Ok(Value::Number(Number::new(if a.value >= b.value {
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

    /// Logical NOT
    pub fn logical_not(&self) -> Result<Value, Error> {
        Ok(Value::Number(Number::new(if self.is_true() {
            0.0
        } else {
            1.0
        })))
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
}

/// User-defined function
#[derive(Debug, Clone)]
pub struct Function {
    pub name: Option<String>,
    pub body_node: Box<Node>,
    pub arg_names: Vec<String>,
    pub should_auto_return: bool,
}

impl Function {
    pub fn new(
        name: Option<String>,
        body_node: Node,
        arg_names: Vec<String>,
        should_auto_return: bool,
    ) -> Self {
        Self {
            name,
            body_node: Box::new(body_node),
            arg_names,
            should_auto_return,
        }
    }

    pub fn execute(
        &self,
        args: Vec<Value>,
        context: Context,
        interpreter: &mut Interpreter,
    ) -> RuntimeResult {
        let mut result = RuntimeResult::new();

        // Check argument count
        if args.len() != self.arg_names.len() {
            return RuntimeResult::new().failure(
                RuntimeError::new(
                    crate::position::Position::new(0, 0, 0, "", ""),
                    crate::position::Position::new(0, 0, 0, "", ""),
                    &format!(
                        "Expected {} arguments, got {}",
                        self.arg_names.len(),
                        args.len()
                    ),
                    Some(context),
                )
                .base,
            );
        }

        // Create new context for function execution
        let mut func_context = context.create_child(
            self.name.as_deref().unwrap_or("<anonymous>"),
            crate::position::Position::new(0, 0, 0, "", ""),
        );

        // Bind arguments
        for (i, arg_name) in self.arg_names.iter().enumerate() {
            func_context
                .symbol_table
                .set(arg_name.clone(), args[i].clone());
        }

        // Execute function body
        let exec_result = interpreter.visit(&self.body_node, &mut func_context);

        if let Some(err) = exec_result.error {
            return RuntimeResult::new().failure(err);
        }

        // Handle return value
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

    pub fn execute(&self, args: Vec<Value>, interpreter: &mut Interpreter) -> RuntimeResult {
        match self.name.as_str() {
            "echo" => self.echo(args),
            "ret" => self.ret(args),
            "input" => self.input(),
            "input_int" => self.input_int(),
            "clear" => self.clear(),
            "is_num" => self.is_num(args),
            "is_str" => self.is_str(args),
            "is_list" => self.is_list(args),
            "is_fun" => self.is_fun(args),
            "append" => self.append(args),
            "pop" => self.pop(args),
            "extend" => self.extend(args),
            "len" => self.len(args),
            "run" => self.run(args, interpreter),
            _ => RuntimeResult::new().failure(
                RuntimeError::new(
                    crate::position::Position::new(0, 0, 0, "", ""),
                    crate::position::Position::new(0, 0, 0, "", ""),
                    &format!("Unknown built-in function: {}", self.name),
                    None,
                )
                .base,
            ),
        }
    }

    fn echo(&self, args: Vec<Value>) -> RuntimeResult {
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

    fn ret(&self, args: Vec<Value>) -> RuntimeResult {
        if let Some(arg) = args.first() {
            match arg {
                Value::Number(n) => RuntimeResult::new()
                    .success(Value::String(XenithString::new(n.value.to_string()))),
                Value::String(s) => RuntimeResult::new().success(Value::String(s.clone())),
                Value::List(l) => {
                    let mut result = String::from("[");
                    for (i, elem) in l.elements.iter().enumerate() {
                        if i > 0 {
                            result.push_str(", ");
                        }
                        match elem {
                            Value::Number(n) => result.push_str(&n.value.to_string()),
                            Value::String(s) => result.push_str(&format!("\"{}\"", s.value)),
                            _ => result.push('?'),
                        }
                    }
                    result.push(']');
                    RuntimeResult::new().success(Value::String(XenithString::new(result)))
                }
                _ => {
                    RuntimeResult::new().success(Value::String(XenithString::new("?".to_string())))
                }
            }
        } else {
            RuntimeResult::new().success(Value::String(XenithString::new("".to_string())))
        }
    }

    fn input(&self) -> RuntimeResult {
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        RuntimeResult::new().success(Value::String(XenithString::new(input.trim().to_string())))
    }

    fn input_int(&self) -> RuntimeResult {
        loop {
            let mut input = String::new();
            io::stdin().read_line(&mut input).unwrap();
            if let Ok(num) = input.trim().parse::<i64>() {
                return RuntimeResult::new().success(Value::Number(Number::new(num as f64)));
            }
            println!("'{}' must be an integer. Try again!", input.trim());
        }
    }

    fn clear(&self) -> RuntimeResult {
        print!("\x1B[2J\x1B[1;1H");
        io::stdout().flush().unwrap();
        RuntimeResult::new().success(Value::Number(Number::null()))
    }

    fn is_num(&self, args: Vec<Value>) -> RuntimeResult {
        let result = matches!(args.first(), Some(Value::Number(_)));
        RuntimeResult::new().success(Value::Number(Number::new(if result { 1.0 } else { 0.0 })))
    }

    fn is_str(&self, args: Vec<Value>) -> RuntimeResult {
        let result = matches!(args.first(), Some(Value::String(_)));
        RuntimeResult::new().success(Value::Number(Number::new(if result { 1.0 } else { 0.0 })))
    }

    fn is_list(&self, args: Vec<Value>) -> RuntimeResult {
        let result = matches!(args.first(), Some(Value::List(_)));
        RuntimeResult::new().success(Value::Number(Number::new(if result { 1.0 } else { 0.0 })))
    }

    fn is_fun(&self, args: Vec<Value>) -> RuntimeResult {
        let result = matches!(
            args.first(),
            Some(Value::Function(_)) | Some(Value::BuiltInFunction(_))
        );
        RuntimeResult::new().success(Value::Number(Number::new(if result { 1.0 } else { 0.0 })))
    }

    fn append(&self, args: Vec<Value>) -> RuntimeResult {
        if args.len() != 2 {
            return RuntimeResult::new().failure(
                RuntimeError::new(
                    crate::position::Position::new(0, 0, 0, "", ""),
                    crate::position::Position::new(0, 0, 0, "", ""),
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
                    crate::position::Position::new(0, 0, 0, "", ""),
                    crate::position::Position::new(0, 0, 0, "", ""),
                    "First argument to append must be a list",
                    None,
                )
                .base,
            ),
        }
    }

    fn pop(&self, args: Vec<Value>) -> RuntimeResult {
        if args.len() != 2 {
            return RuntimeResult::new().failure(
                RuntimeError::new(
                    crate::position::Position::new(0, 0, 0, "", ""),
                    crate::position::Position::new(0, 0, 0, "", ""),
                    "pop expects 2 arguments",
                    None,
                )
                .base,
            );
        }

        match (&args[0], &args[1]) {
            (Value::List(list), Value::Number(idx)) => {
                let idx_usize = idx.value as usize;
                if idx_usize >= list.elements.len() {
                    return RuntimeResult::new().failure(
                        RuntimeError::new(
                            crate::position::Position::new(0, 0, 0, "", ""),
                            crate::position::Position::new(0, 0, 0, "", ""),
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
                    crate::position::Position::new(0, 0, 0, "", ""),
                    crate::position::Position::new(0, 0, 0, "", ""),
                    "pop expects a list and an index",
                    None,
                )
                .base,
            ),
        }
    }

    fn extend(&self, args: Vec<Value>) -> RuntimeResult {
        if args.len() != 2 {
            return RuntimeResult::new().failure(
                RuntimeError::new(
                    crate::position::Position::new(0, 0, 0, "", ""),
                    crate::position::Position::new(0, 0, 0, "", ""),
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
                RuntimeError::new(
                    crate::position::Position::new(0, 0, 0, "", ""),
                    crate::position::Position::new(0, 0, 0, "", ""),
                    "extend expects two lists",
                    None,
                )
                .base,
            ),
        }
    }

    fn len(&self, args: Vec<Value>) -> RuntimeResult {
        if args.len() != 1 {
            return RuntimeResult::new().failure(
                RuntimeError::new(
                    crate::position::Position::new(0, 0, 0, "", ""),
                    crate::position::Position::new(0, 0, 0, "", ""),
                    "len expects 1 argument",
                    None,
                )
                .base,
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
                    crate::position::Position::new(0, 0, 0, "", ""),
                    crate::position::Position::new(0, 0, 0, "", ""),
                    "len expects a list or string",
                    None,
                )
                .base,
            ),
        }
    }

    fn run(&self, args: Vec<Value>, interpreter: &mut Interpreter) -> RuntimeResult {
        if args.len() != 1 {
            return RuntimeResult::new().failure(
                RuntimeError::new(
                    crate::position::Position::new(0, 0, 0, "", ""),
                    crate::position::Position::new(0, 0, 0, "", ""),
                    "run expects 1 argument",
                    None,
                )
                .base,
            );
        }

        let filename = match &args[0] {
            Value::String(s) => &s.value,
            _ => {
                return RuntimeResult::new().failure(
                    RuntimeError::new(
                        crate::position::Position::new(0, 0, 0, "", ""),
                        crate::position::Position::new(0, 0, 0, "", ""),
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
                    crate::position::Position::new(0, 0, 0, "", ""),
                    crate::position::Position::new(0, 0, 0, "", ""),
                    &format!("Failed to load script \"{}\": {}", filename, e),
                    None,
                )
                .base,
            ),
        }
    }
}
