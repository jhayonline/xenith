//! Time built-in functions
//! These are called by the std::time wrapper module

use crate::error::{Error, RuntimeError};
use crate::position::Position;
use crate::runtime_result::RuntimeResult;
use crate::values::{Number, Value};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn dummy_pos() -> Position {
    Position::new(0, 0, 0, "", "")
}

pub fn timestamp(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 0 {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                "__time_timestamp expects 0 arguments",
                None,
            )
            .base,
        );
    }

    let now = SystemTime::now();
    let since_epoch = match now.duration_since(UNIX_EPOCH) {
        Ok(d) => d,
        Err(e) => {
            return RuntimeResult::new().failure(
                RuntimeError::new(
                    dummy_pos(),
                    dummy_pos(),
                    &format!("Time went backwards: {}", e),
                    None,
                )
                .base,
            );
        }
    };

    RuntimeResult::new().success(Value::Number(Number::new(since_epoch.as_secs() as f64)))
}

pub fn timestamp_ms(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 0 {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                "__time_timestamp_ms expects 0 arguments",
                None,
            )
            .base,
        );
    }

    let now = SystemTime::now();
    let since_epoch = match now.duration_since(UNIX_EPOCH) {
        Ok(d) => d,
        Err(e) => {
            return RuntimeResult::new().failure(
                RuntimeError::new(
                    dummy_pos(),
                    dummy_pos(),
                    &format!("Time went backwards: {}", e),
                    None,
                )
                .base,
            );
        }
    };

    RuntimeResult::new().success(Value::Number(Number::new(since_epoch.as_millis() as f64)))
}

pub fn sleep(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 1 {
        return RuntimeResult::new().failure(
            Error::new(
                dummy_pos(),
                dummy_pos(),
                "Argument Error",
                "__time_sleep expects 1 argument (milliseconds)",
            )
            .with_code("XEN100"),
        );
    }

    let ms = match &args[0] {
        Value::Number(n) => n.value as u64,
        _ => {
            return RuntimeResult::new().failure(Error::type_mismatch(
                "number",
                "other",
                dummy_pos(),
                dummy_pos(),
            ));
        }
    };

    thread::sleep(Duration::from_millis(ms));
    RuntimeResult::new().success(Value::Number(Number::null()))
}

pub fn sleep_sec(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 1 {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                "__time_sleep_sec expects 1 argument (seconds)",
                None,
            )
            .base,
        );
    }

    let secs = match &args[0] {
        Value::Number(n) => n.value as u64,
        _ => {
            return RuntimeResult::new().failure(
                RuntimeError::new(
                    dummy_pos(),
                    dummy_pos(),
                    "__time_sleep_sec: argument must be a number",
                    None,
                )
                .base,
            );
        }
    };

    thread::sleep(Duration::from_secs(secs));
    RuntimeResult::new().success(Value::Number(Number::null()))
}

pub fn duration_secs(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 2 {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                "__time_duration_secs expects 2 arguments (start_ms, end_ms)",
                None,
            )
            .base,
        );
    }

    let start = match &args[0] {
        Value::Number(n) => n.value as u128,
        _ => {
            return RuntimeResult::new().failure(
                RuntimeError::new(
                    dummy_pos(),
                    dummy_pos(),
                    "__time_duration_secs: first argument must be a number",
                    None,
                )
                .base,
            );
        }
    };

    let end = match &args[1] {
        Value::Number(n) => n.value as u128,
        _ => {
            return RuntimeResult::new().failure(
                RuntimeError::new(
                    dummy_pos(),
                    dummy_pos(),
                    "__time_duration_secs: second argument must be a number",
                    None,
                )
                .base,
            );
        }
    };

    let duration_secs = (end - start) as f64 / 1000.0;
    RuntimeResult::new().success(Value::Number(Number::new(duration_secs)))
}

pub fn duration_ms(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 2 {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                "__time_duration_ms expects 2 arguments (start_ms, end_ms)",
                None,
            )
            .base,
        );
    }

    let start = match &args[0] {
        Value::Number(n) => n.value as u128,
        _ => {
            return RuntimeResult::new().failure(
                RuntimeError::new(
                    dummy_pos(),
                    dummy_pos(),
                    "__time_duration_ms: first argument must be a number",
                    None,
                )
                .base,
            );
        }
    };

    let end = match &args[1] {
        Value::Number(n) => n.value as u128,
        _ => {
            return RuntimeResult::new().failure(
                RuntimeError::new(
                    dummy_pos(),
                    dummy_pos(),
                    "__time_duration_ms: second argument must be a number",
                    None,
                )
                .base,
            );
        }
    };

    let duration_ms = (end - start) as f64;
    RuntimeResult::new().success(Value::Number(Number::new(duration_ms)))
}
