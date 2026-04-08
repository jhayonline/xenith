//! Random generation built-in functions
//! These are called by the std::random wrapper module

use crate::error::RuntimeError;
use crate::position::Position;
use crate::runtime_result::RuntimeResult;
use crate::values::{List, Number, Value, XenithString};
use rand::Rng;
use std::cell::RefCell;

thread_local! {
    static RNG: RefCell<rand::rngs::ThreadRng> = RefCell::new(rand::rngs::ThreadRng::default());
}

fn dummy_pos() -> Position {
    Position::new(0, 0, 0, "", "")
}

pub fn rand_int(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 0 {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                "__rand_int expects 0 arguments",
                None,
            )
            .base,
        );
    }

    let value = RNG.with(|rng| {
        let mut rng = rng.borrow_mut();
        rng.r#gen::<i64>()
    });

    RuntimeResult::new().success(Value::Number(Number::new(value as f64)))
}

pub fn rand_int_range(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 2 {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                "__rand_int_range expects 2 arguments (min, max)",
                None,
            )
            .base,
        );
    }

    let min = match &args[0] {
        Value::Number(n) => n.value as i64,
        _ => {
            return RuntimeResult::new().failure(
                RuntimeError::new(
                    dummy_pos(),
                    dummy_pos(),
                    "__rand_int_range: min must be a number",
                    None,
                )
                .base,
            );
        }
    };

    let max = match &args[1] {
        Value::Number(n) => n.value as i64,
        _ => {
            return RuntimeResult::new().failure(
                RuntimeError::new(
                    dummy_pos(),
                    dummy_pos(),
                    "__rand_int_range: max must be a number",
                    None,
                )
                .base,
            );
        }
    };

    if min >= max {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                "__rand_int_range: min must be less than max",
                None,
            )
            .base,
        );
    }

    let value = RNG.with(|rng| {
        let mut rng = rng.borrow_mut();
        rng.gen_range(min..max)
    });

    RuntimeResult::new().success(Value::Number(Number::new(value as f64)))
}

pub fn rand_float(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 0 {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                "__rand_float expects 0 arguments",
                None,
            )
            .base,
        );
    }

    let value: f64 = RNG.with(|rng| {
        let mut rng = rng.borrow_mut();
        rng.r#gen()
    });

    RuntimeResult::new().success(Value::Number(Number::new(value)))
}

pub fn rand_float_range(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 2 {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                "__rand_float_range expects 2 arguments (min, max)",
                None,
            )
            .base,
        );
    }

    let min = match &args[0] {
        Value::Number(n) => n.value,
        _ => {
            return RuntimeResult::new().failure(
                RuntimeError::new(
                    dummy_pos(),
                    dummy_pos(),
                    "__rand_float_range: min must be a number",
                    None,
                )
                .base,
            );
        }
    };

    let max = match &args[1] {
        Value::Number(n) => n.value,
        _ => {
            return RuntimeResult::new().failure(
                RuntimeError::new(
                    dummy_pos(),
                    dummy_pos(),
                    "__rand_float_range: max must be a number",
                    None,
                )
                .base,
            );
        }
    };

    if min >= max {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                "__rand_float_range: min must be less than max",
                None,
            )
            .base,
        );
    }

    let value = RNG.with(|rng| {
        let mut rng = rng.borrow_mut();
        rng.gen_range(min..max)
    });

    RuntimeResult::new().success(Value::Number(Number::new(value)))
}

pub fn rand_bool(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 0 {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                "__rand_bool expects 0 arguments",
                None,
            )
            .base,
        );
    }

    let value: bool = RNG.with(|rng| {
        let mut rng = rng.borrow_mut();
        rng.r#gen()
    });

    RuntimeResult::new().success(Value::Bool(value))
}

pub fn choice(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 1 {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                "__rand_choice expects 1 argument (list)",
                None,
            )
            .base,
        );
    }

    let list = match &args[0] {
        Value::List(l) => l,
        _ => {
            return RuntimeResult::new().failure(
                RuntimeError::new(
                    dummy_pos(),
                    dummy_pos(),
                    "__rand_choice: argument must be a list",
                    None,
                )
                .base,
            );
        }
    };

    if list.elements.is_empty() {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                "__rand_choice: list cannot be empty",
                None,
            )
            .base,
        );
    }

    let index = RNG.with(|rng| {
        let mut rng = rng.borrow_mut();
        rng.gen_range(0..list.elements.len())
    });

    RuntimeResult::new().success(list.elements[index].clone())
}

pub fn shuffle(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 1 {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                "__rand_shuffle expects 1 argument (list)",
                None,
            )
            .base,
        );
    }

    let list = match &args[0] {
        Value::List(l) => l.clone(),
        _ => {
            return RuntimeResult::new().failure(
                RuntimeError::new(
                    dummy_pos(),
                    dummy_pos(),
                    "__rand_shuffle: argument must be a list",
                    None,
                )
                .base,
            );
        }
    };

    let mut elements = list.elements;

    // Fisher-Yates shuffle
    for i in (1..elements.len()).rev() {
        let j = RNG.with(|rng| {
            let mut rng = rng.borrow_mut();
            rng.gen_range(0..=i)
        });
        elements.swap(i, j);
    }

    RuntimeResult::new().success(Value::List(List::new(elements)))
}

pub fn uuid(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 0 {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                "__rand_uuid expects 0 arguments",
                None,
            )
            .base,
        );
    }

    let uuid = uuid::Uuid::new_v4();
    RuntimeResult::new().success(Value::String(XenithString::new(uuid.to_string())))
}
