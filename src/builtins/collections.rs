//! Collections built-in functions
//! These are called by the std::collections wrapper module

use crate::error::{Error, RuntimeError};
use crate::position::Position;
use crate::runtime_result::RuntimeResult;
use crate::values::{List, Number, Struct, Value};
use once_cell::sync::Lazy;
use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;

// ==================== Set Implementation ====================

static SETS: Lazy<Mutex<HashMap<usize, Vec<Value>>>> = Lazy::new(|| Mutex::new(HashMap::new()));
static NEXT_SET_ID: Lazy<Mutex<usize>> = Lazy::new(|| Mutex::new(1));

fn value_eq(v1: &Value, v2: &Value) -> bool {
    match (v1, v2) {
        (Value::Number(n1), Value::Number(n2)) => (n1.value - n2.value).abs() < 1e-10,
        (Value::String(s1), Value::String(s2)) => s1.value == s2.value,
        (Value::Bool(b1), Value::Bool(b2)) => b1 == b2,
        (Value::List(l1), Value::List(l2)) => {
            if l1.elements.len() != l2.elements.len() {
                return false;
            }
            l1.elements
                .iter()
                .zip(l2.elements.iter())
                .all(|(a, b)| value_eq(a, b))
        }
        (Value::Json(j1), Value::Json(j2)) => j1 == j2,
        // Remove Value::Null case - null is represented as Number(0.0)
        _ => false,
    }
}

fn value_contains(set: &[Value], value: &Value) -> bool {
    set.iter().any(|existing| value_eq(existing, value))
}

pub fn set_new(args: Vec<Value>, call_pos: Position) -> RuntimeResult {
    if !args.is_empty() {
        return RuntimeResult::new().failure(
            Error::new(
                call_pos.clone(),
                call_pos,
                "Argument Error",
                "__set_new expects 0 arguments",
            )
            .with_code("XEN100"),
        );
    }

    let id = {
        let mut next_id = NEXT_SET_ID.lock().unwrap();
        let id = *next_id;
        *next_id += 1;
        id
    };

    let mut sets = SETS.lock().unwrap();
    sets.insert(id, Vec::new());

    let mut set_struct = Struct::new("Set".to_string());
    set_struct.set_field("_id".to_string(), Value::Number(Number::new(id as f64)));

    RuntimeResult::new().success(Value::Struct(set_struct))
}

pub fn set_add(args: Vec<Value>, call_pos: Position) -> RuntimeResult {
    if args.len() != 2 {
        return RuntimeResult::new().failure(
            Error::new(
                call_pos.clone(),
                call_pos,
                "Argument Error",
                "__set_add expects 2 arguments (set, value)",
            )
            .with_code("XEN100"),
        );
    }

    let set_id = match &args[0] {
        Value::Struct(s) => {
            if let Some(Value::Number(id)) = s.get_field("_id") {
                id.value as usize
            } else {
                return RuntimeResult::new().failure(
                    RuntimeError::new(call_pos.clone(), call_pos, "Invalid set object", None).base,
                );
            }
        }
        _ => {
            return RuntimeResult::new().failure(Error::type_mismatch(
                "set",
                "other",
                call_pos.clone(),
                call_pos,
            ));
        }
    };

    let value = args[1].clone();

    let mut sets = SETS.lock().unwrap();
    if let Some(set) = sets.get_mut(&set_id) {
        if !value_contains(set, &value) {
            set.push(value);
        }
        RuntimeResult::new().success(Value::Number(Number::null()))
    } else {
        RuntimeResult::new()
            .failure(RuntimeError::new(call_pos.clone(), call_pos, "Set not found", None).base)
    }
}

pub fn set_contains(args: Vec<Value>, call_pos: Position) -> RuntimeResult {
    if args.len() != 2 {
        return RuntimeResult::new().failure(
            Error::new(
                call_pos.clone(),
                call_pos,
                "Argument Error",
                "__set_contains expects 2 arguments (set, value)",
            )
            .with_code("XEN100"),
        );
    }

    let set_id = match &args[0] {
        Value::Struct(s) => {
            if let Some(Value::Number(id)) = s.get_field("_id") {
                id.value as usize
            } else {
                return RuntimeResult::new().failure(
                    RuntimeError::new(call_pos.clone(), call_pos, "Invalid set object", None).base,
                );
            }
        }
        _ => {
            return RuntimeResult::new().failure(Error::type_mismatch(
                "set",
                "other",
                call_pos.clone(),
                call_pos,
            ));
        }
    };

    let value = &args[1];

    let sets = SETS.lock().unwrap();
    if let Some(set) = sets.get(&set_id) {
        let contains = value_contains(set, value);
        RuntimeResult::new().success(Value::Bool(contains))
    } else {
        RuntimeResult::new()
            .failure(RuntimeError::new(call_pos.clone(), call_pos, "Set not found", None).base)
    }
}

pub fn set_remove(args: Vec<Value>, call_pos: Position) -> RuntimeResult {
    if args.len() != 2 {
        return RuntimeResult::new().failure(
            Error::new(
                call_pos.clone(),
                call_pos,
                "Argument Error",
                "__set_remove expects 2 arguments (set, value)",
            )
            .with_code("XEN100"),
        );
    }

    let set_id = match &args[0] {
        Value::Struct(s) => {
            if let Some(Value::Number(id)) = s.get_field("_id") {
                id.value as usize
            } else {
                return RuntimeResult::new().failure(
                    RuntimeError::new(call_pos.clone(), call_pos, "Invalid set object", None).base,
                );
            }
        }
        _ => {
            return RuntimeResult::new().failure(Error::type_mismatch(
                "set",
                "other",
                call_pos.clone(),
                call_pos,
            ));
        }
    };

    let value = &args[1];

    let mut sets = SETS.lock().unwrap();
    if let Some(set) = sets.get_mut(&set_id) {
        if let Some(pos) = set.iter().position(|existing| value_eq(existing, value)) {
            set.remove(pos);
        }
        RuntimeResult::new().success(Value::Number(Number::null()))
    } else {
        RuntimeResult::new()
            .failure(RuntimeError::new(call_pos.clone(), call_pos, "Set not found", None).base)
    }
}

pub fn set_len(args: Vec<Value>, call_pos: Position) -> RuntimeResult {
    if args.len() != 1 {
        return RuntimeResult::new().failure(
            Error::new(
                call_pos.clone(),
                call_pos,
                "Argument Error",
                "__set_len expects 1 argument (set)",
            )
            .with_code("XEN100"),
        );
    }

    let set_id = match &args[0] {
        Value::Struct(s) => {
            if let Some(Value::Number(id)) = s.get_field("_id") {
                id.value as usize
            } else {
                return RuntimeResult::new().failure(
                    RuntimeError::new(call_pos.clone(), call_pos, "Invalid set object", None).base,
                );
            }
        }
        _ => {
            return RuntimeResult::new().failure(Error::type_mismatch(
                "set",
                "other",
                call_pos.clone(),
                call_pos,
            ));
        }
    };

    let sets = SETS.lock().unwrap();
    if let Some(set) = sets.get(&set_id) {
        RuntimeResult::new().success(Value::Number(Number::new(set.len() as f64)))
    } else {
        RuntimeResult::new()
            .failure(RuntimeError::new(call_pos.clone(), call_pos, "Set not found", None).base)
    }
}

pub fn set_to_list(args: Vec<Value>, call_pos: Position) -> RuntimeResult {
    if args.len() != 1 {
        return RuntimeResult::new().failure(
            Error::new(
                call_pos.clone(),
                call_pos,
                "Argument Error",
                "__set_to_list expects 1 argument (set)",
            )
            .with_code("XEN100"),
        );
    }

    let set_id = match &args[0] {
        Value::Struct(s) => {
            if let Some(Value::Number(id)) = s.get_field("_id") {
                id.value as usize
            } else {
                return RuntimeResult::new().failure(
                    RuntimeError::new(call_pos.clone(), call_pos, "Invalid set object", None).base,
                );
            }
        }
        _ => {
            return RuntimeResult::new().failure(Error::type_mismatch(
                "set",
                "other",
                call_pos.clone(),
                call_pos,
            ));
        }
    };

    let sets = SETS.lock().unwrap();
    if let Some(set) = sets.get(&set_id) {
        let elements: Vec<Value> = set.iter().cloned().collect();
        RuntimeResult::new().success(Value::List(List::new(elements)))
    } else {
        RuntimeResult::new()
            .failure(RuntimeError::new(call_pos.clone(), call_pos, "Set not found", None).base)
    }
}

// ==================== Stack Implementation ====================

static STACKS: Lazy<Mutex<HashMap<usize, Vec<Value>>>> = Lazy::new(|| Mutex::new(HashMap::new()));
static NEXT_STACK_ID: Lazy<Mutex<usize>> = Lazy::new(|| Mutex::new(1));

pub fn stack_new(args: Vec<Value>, call_pos: Position) -> RuntimeResult {
    if !args.is_empty() {
        return RuntimeResult::new().failure(
            Error::new(
                call_pos.clone(),
                call_pos,
                "Argument Error",
                "__stack_new expects 0 arguments",
            )
            .with_code("XEN100"),
        );
    }

    let id = {
        let mut next_id = NEXT_STACK_ID.lock().unwrap();
        let id = *next_id;
        *next_id += 1;
        id
    };

    let mut stacks = STACKS.lock().unwrap();
    stacks.insert(id, Vec::new());

    let mut stack_struct = Struct::new("Stack".to_string());
    stack_struct.set_field("_id".to_string(), Value::Number(Number::new(id as f64)));

    RuntimeResult::new().success(Value::Struct(stack_struct))
}

pub fn stack_push(args: Vec<Value>, call_pos: Position) -> RuntimeResult {
    if args.len() != 2 {
        return RuntimeResult::new().failure(
            Error::new(
                call_pos.clone(),
                call_pos,
                "Argument Error",
                "__stack_push expects 2 arguments (stack, value)",
            )
            .with_code("XEN100"),
        );
    }

    let stack_id = match &args[0] {
        Value::Struct(s) => {
            if let Some(Value::Number(id)) = s.get_field("_id") {
                id.value as usize
            } else {
                return RuntimeResult::new().failure(
                    RuntimeError::new(call_pos.clone(), call_pos, "Invalid stack object", None)
                        .base,
                );
            }
        }
        _ => {
            return RuntimeResult::new().failure(Error::type_mismatch(
                "stack",
                "other",
                call_pos.clone(),
                call_pos,
            ));
        }
    };

    let value = args[1].clone();

    let mut stacks = STACKS.lock().unwrap();
    if let Some(stack) = stacks.get_mut(&stack_id) {
        stack.push(value);
        RuntimeResult::new().success(Value::Number(Number::null()))
    } else {
        RuntimeResult::new()
            .failure(RuntimeError::new(call_pos.clone(), call_pos, "Stack not found", None).base)
    }
}

pub fn stack_pop(args: Vec<Value>, call_pos: Position) -> RuntimeResult {
    if args.len() != 1 {
        return RuntimeResult::new().failure(
            Error::new(
                call_pos.clone(),
                call_pos,
                "Argument Error",
                "__stack_pop expects 1 argument (stack)",
            )
            .with_code("XEN100"),
        );
    }

    let stack_id = match &args[0] {
        Value::Struct(s) => {
            if let Some(Value::Number(id)) = s.get_field("_id") {
                id.value as usize
            } else {
                return RuntimeResult::new().failure(
                    RuntimeError::new(call_pos.clone(), call_pos, "Invalid stack object", None)
                        .base,
                );
            }
        }
        _ => {
            return RuntimeResult::new().failure(Error::type_mismatch(
                "stack",
                "other",
                call_pos.clone(),
                call_pos,
            ));
        }
    };

    let mut stacks = STACKS.lock().unwrap();
    if let Some(stack) = stacks.get_mut(&stack_id) {
        if let Some(value) = stack.pop() {
            RuntimeResult::new().success(value)
        } else {
            RuntimeResult::new().failure(
                RuntimeError::new(
                    call_pos.clone(),
                    call_pos,
                    "Cannot pop from empty stack",
                    None,
                )
                .base,
            )
        }
    } else {
        RuntimeResult::new()
            .failure(RuntimeError::new(call_pos.clone(), call_pos, "Stack not found", None).base)
    }
}

pub fn stack_peek(args: Vec<Value>, call_pos: Position) -> RuntimeResult {
    if args.len() != 1 {
        return RuntimeResult::new().failure(
            Error::new(
                call_pos.clone(),
                call_pos,
                "Argument Error",
                "__stack_peek expects 1 argument (stack)",
            )
            .with_code("XEN100"),
        );
    }

    let stack_id = match &args[0] {
        Value::Struct(s) => {
            if let Some(Value::Number(id)) = s.get_field("_id") {
                id.value as usize
            } else {
                return RuntimeResult::new().failure(
                    RuntimeError::new(call_pos.clone(), call_pos, "Invalid stack object", None)
                        .base,
                );
            }
        }
        _ => {
            return RuntimeResult::new().failure(Error::type_mismatch(
                "stack",
                "other",
                call_pos.clone(),
                call_pos,
            ));
        }
    };

    let stacks = STACKS.lock().unwrap();
    if let Some(stack) = stacks.get(&stack_id) {
        if let Some(value) = stack.last() {
            RuntimeResult::new().success(value.clone())
        } else {
            RuntimeResult::new().failure(
                RuntimeError::new(
                    call_pos.clone(),
                    call_pos,
                    "Cannot peek at empty stack",
                    None,
                )
                .base,
            )
        }
    } else {
        RuntimeResult::new()
            .failure(RuntimeError::new(call_pos.clone(), call_pos, "Stack not found", None).base)
    }
}

pub fn stack_len(args: Vec<Value>, call_pos: Position) -> RuntimeResult {
    if args.len() != 1 {
        return RuntimeResult::new().failure(
            Error::new(
                call_pos.clone(),
                call_pos,
                "Argument Error",
                "__stack_len expects 1 argument (stack)",
            )
            .with_code("XEN100"),
        );
    }

    let stack_id = match &args[0] {
        Value::Struct(s) => {
            if let Some(Value::Number(id)) = s.get_field("_id") {
                id.value as usize
            } else {
                return RuntimeResult::new().failure(
                    RuntimeError::new(call_pos.clone(), call_pos, "Invalid stack object", None)
                        .base,
                );
            }
        }
        _ => {
            return RuntimeResult::new().failure(Error::type_mismatch(
                "stack",
                "other",
                call_pos.clone(),
                call_pos,
            ));
        }
    };

    let stacks = STACKS.lock().unwrap();
    if let Some(stack) = stacks.get(&stack_id) {
        RuntimeResult::new().success(Value::Number(Number::new(stack.len() as f64)))
    } else {
        RuntimeResult::new()
            .failure(RuntimeError::new(call_pos.clone(), call_pos, "Stack not found", None).base)
    }
}

// ==================== Queue Implementation ====================

static QUEUES: Lazy<Mutex<HashMap<usize, VecDeque<Value>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));
static NEXT_QUEUE_ID: Lazy<Mutex<usize>> = Lazy::new(|| Mutex::new(1));

pub fn queue_new(args: Vec<Value>, call_pos: Position) -> RuntimeResult {
    if !args.is_empty() {
        return RuntimeResult::new().failure(
            Error::new(
                call_pos.clone(),
                call_pos,
                "Argument Error",
                "__queue_new expects 0 arguments",
            )
            .with_code("XEN100"),
        );
    }

    let id = {
        let mut next_id = NEXT_QUEUE_ID.lock().unwrap();
        let id = *next_id;
        *next_id += 1;
        id
    };

    let mut queues = QUEUES.lock().unwrap();
    queues.insert(id, VecDeque::new());

    let mut queue_struct = Struct::new("Queue".to_string());
    queue_struct.set_field("_id".to_string(), Value::Number(Number::new(id as f64)));

    RuntimeResult::new().success(Value::Struct(queue_struct))
}

pub fn queue_enqueue(args: Vec<Value>, call_pos: Position) -> RuntimeResult {
    if args.len() != 2 {
        return RuntimeResult::new().failure(
            Error::new(
                call_pos.clone(),
                call_pos,
                "Argument Error",
                "__queue_enqueue expects 2 arguments (queue, value)",
            )
            .with_code("XEN100"),
        );
    }

    let queue_id = match &args[0] {
        Value::Struct(s) => {
            if let Some(Value::Number(id)) = s.get_field("_id") {
                id.value as usize
            } else {
                return RuntimeResult::new().failure(
                    RuntimeError::new(call_pos.clone(), call_pos, "Invalid queue object", None)
                        .base,
                );
            }
        }
        _ => {
            return RuntimeResult::new().failure(Error::type_mismatch(
                "queue",
                "other",
                call_pos.clone(),
                call_pos,
            ));
        }
    };

    let value = args[1].clone();

    let mut queues = QUEUES.lock().unwrap();
    if let Some(queue) = queues.get_mut(&queue_id) {
        queue.push_back(value);
        RuntimeResult::new().success(Value::Number(Number::null()))
    } else {
        RuntimeResult::new()
            .failure(RuntimeError::new(call_pos.clone(), call_pos, "Queue not found", None).base)
    }
}

pub fn queue_dequeue(args: Vec<Value>, call_pos: Position) -> RuntimeResult {
    if args.len() != 1 {
        return RuntimeResult::new().failure(
            Error::new(
                call_pos.clone(),
                call_pos,
                "Argument Error",
                "__queue_dequeue expects 1 argument (queue)",
            )
            .with_code("XEN100"),
        );
    }

    let queue_id = match &args[0] {
        Value::Struct(s) => {
            if let Some(Value::Number(id)) = s.get_field("_id") {
                id.value as usize
            } else {
                return RuntimeResult::new().failure(
                    RuntimeError::new(call_pos.clone(), call_pos, "Invalid queue object", None)
                        .base,
                );
            }
        }
        _ => {
            return RuntimeResult::new().failure(Error::type_mismatch(
                "queue",
                "other",
                call_pos.clone(),
                call_pos,
            ));
        }
    };

    let mut queues = QUEUES.lock().unwrap();
    if let Some(queue) = queues.get_mut(&queue_id) {
        if let Some(value) = queue.pop_front() {
            RuntimeResult::new().success(value)
        } else {
            RuntimeResult::new().failure(
                RuntimeError::new(
                    call_pos.clone(),
                    call_pos,
                    "Cannot dequeue from empty queue",
                    None,
                )
                .base,
            )
        }
    } else {
        RuntimeResult::new()
            .failure(RuntimeError::new(call_pos.clone(), call_pos, "Queue not found", None).base)
    }
}

pub fn queue_peek(args: Vec<Value>, call_pos: Position) -> RuntimeResult {
    if args.len() != 1 {
        return RuntimeResult::new().failure(
            Error::new(
                call_pos.clone(),
                call_pos,
                "Argument Error",
                "__queue_peek expects 1 argument (queue)",
            )
            .with_code("XEN100"),
        );
    }

    let queue_id = match &args[0] {
        Value::Struct(s) => {
            if let Some(Value::Number(id)) = s.get_field("_id") {
                id.value as usize
            } else {
                return RuntimeResult::new().failure(
                    RuntimeError::new(call_pos.clone(), call_pos, "Invalid queue object", None)
                        .base,
                );
            }
        }
        _ => {
            return RuntimeResult::new().failure(Error::type_mismatch(
                "queue",
                "other",
                call_pos.clone(),
                call_pos,
            ));
        }
    };

    let queues = QUEUES.lock().unwrap();
    if let Some(queue) = queues.get(&queue_id) {
        if let Some(value) = queue.front() {
            RuntimeResult::new().success(value.clone())
        } else {
            RuntimeResult::new().failure(
                RuntimeError::new(
                    call_pos.clone(),
                    call_pos,
                    "Cannot peek at empty queue",
                    None,
                )
                .base,
            )
        }
    } else {
        RuntimeResult::new()
            .failure(RuntimeError::new(call_pos.clone(), call_pos, "Queue not found", None).base)
    }
}

pub fn queue_len(args: Vec<Value>, call_pos: Position) -> RuntimeResult {
    if args.len() != 1 {
        return RuntimeResult::new().failure(
            Error::new(
                call_pos.clone(),
                call_pos,
                "Argument Error",
                "__queue_len expects 1 argument (queue)",
            )
            .with_code("XEN100"),
        );
    }

    let queue_id = match &args[0] {
        Value::Struct(s) => {
            if let Some(Value::Number(id)) = s.get_field("_id") {
                id.value as usize
            } else {
                return RuntimeResult::new().failure(
                    RuntimeError::new(call_pos.clone(), call_pos, "Invalid queue object", None)
                        .base,
                );
            }
        }
        _ => {
            return RuntimeResult::new().failure(Error::type_mismatch(
                "queue",
                "other",
                call_pos.clone(),
                call_pos,
            ));
        }
    };

    let queues = QUEUES.lock().unwrap();
    if let Some(queue) = queues.get(&queue_id) {
        RuntimeResult::new().success(Value::Number(Number::new(queue.len() as f64)))
    } else {
        RuntimeResult::new()
            .failure(RuntimeError::new(call_pos.clone(), call_pos, "Queue not found", None).base)
    }
}
