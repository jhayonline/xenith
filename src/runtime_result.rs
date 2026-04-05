//! # Runtime Result Module
//!
//! Manages the execution state during interpretation including return values,
//! errors, and loop control (continue/break). Enables proper handling of
//! control flow at runtime.

use crate::error::Error;
use crate::values::Value;

/// Result of runtime execution with control flow tracking
#[derive(Debug, Clone)]
pub struct RuntimeResult {
    pub value: Option<Value>,
    pub error: Option<Error>,
    pub func_return_value: Option<Value>,
    pub loop_should_continue: bool,
    pub loop_should_break: bool,
}

impl RuntimeResult {
    /// Creates a new runtime result
    pub fn new() -> Self {
        Self {
            value: None,
            error: None,
            func_return_value: None,
            loop_should_continue: false,
            loop_should_break: false,
        }
    }

    /// Registers a sub-result
    pub fn register(&mut self, mut res: RuntimeResult) -> Value {
        self.error = res.error.take();
        self.func_return_value = res.func_return_value.take();
        self.loop_should_continue = res.loop_should_continue;
        self.loop_should_break = res.loop_should_break;
        res.value.unwrap()
    }

    /// Creates a successful result
    pub fn success(mut self, value: Value) -> Self {
        self.value = Some(value);
        self
    }

    /// Creates a result that returns from a function
    pub fn success_return(mut self, value: Value) -> Self {
        self.func_return_value = Some(value);
        self
    }

    /// Creates a result that continues a loop
    pub fn success_continue(mut self) -> Self {
        self.loop_should_continue = true;
        self
    }

    /// Creates a result that breaks from a loop
    pub fn success_break(mut self) -> Self {
        self.loop_should_break = true;
        self
    }

    /// Creates a failure result
    pub fn failure(mut self, error: Error) -> Self {
        self.error = Some(error);
        self
    }

    /// Checks if execution should return (error, return, continue, or break)
    pub fn should_return(&self) -> bool {
        self.error.is_some()
            || self.func_return_value.is_some()
            || self.loop_should_continue
            || self.loop_should_break
    }
}

impl Default for RuntimeResult {
    fn default() -> Self {
        Self::new()
    }
}
