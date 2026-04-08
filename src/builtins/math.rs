//! Math built-in functions
//! These are called by the std::math wrapper module

use crate::error::RuntimeError;
use crate::position::Position;
use crate::runtime_result::RuntimeResult;
use crate::values::{Number, Value};

fn dummy_pos() -> Position {
    Position::new(0, 0, 0, "", "")
}

fn get_float_arg(args: &[Value], index: usize) -> Result<f64, RuntimeError> {
    match &args[index] {
        Value::Number(n) => Ok(n.value),
        _ => Err(RuntimeError::new(
            dummy_pos(),
            dummy_pos(),
            &format!("Argument {} must be a number", index),
            None,
        )),
    }
}

fn get_int_arg(args: &[Value], index: usize) -> Result<i64, RuntimeError> {
    match &args[index] {
        Value::Number(n) => Ok(n.value as i64),
        _ => Err(RuntimeError::new(
            dummy_pos(),
            dummy_pos(),
            &format!("Argument {} must be a number", index),
            None,
        )),
    }
}

pub fn sqrt(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 1 {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                "__math_sqrt expects 1 argument",
                None,
            )
            .base,
        );
    }

    let x = match get_float_arg(&args, 0) {
        Ok(v) => v,
        Err(e) => return RuntimeResult::new().failure(e.base),
    };

    if x < 0.0 {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                "Cannot take square root of negative number",
                None,
            )
            .base,
        );
    }

    RuntimeResult::new().success(Value::Number(Number::new(x.sqrt())))
}

pub fn pow(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 2 {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                "__math_pow expects 2 arguments (base, exponent)",
                None,
            )
            .base,
        );
    }

    let base = match get_float_arg(&args, 0) {
        Ok(v) => v,
        Err(e) => return RuntimeResult::new().failure(e.base),
    };

    let exp = match get_float_arg(&args, 1) {
        Ok(v) => v,
        Err(e) => return RuntimeResult::new().failure(e.base),
    };

    RuntimeResult::new().success(Value::Number(Number::new(base.powf(exp))))
}

pub fn sin(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 1 {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                "__math_sin expects 1 argument (radians)",
                None,
            )
            .base,
        );
    }

    let x = match get_float_arg(&args, 0) {
        Ok(v) => v,
        Err(e) => return RuntimeResult::new().failure(e.base),
    };

    RuntimeResult::new().success(Value::Number(Number::new(x.sin())))
}

pub fn cos(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 1 {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                "__math_cos expects 1 argument (radians)",
                None,
            )
            .base,
        );
    }

    let x = match get_float_arg(&args, 0) {
        Ok(v) => v,
        Err(e) => return RuntimeResult::new().failure(e.base),
    };

    RuntimeResult::new().success(Value::Number(Number::new(x.cos())))
}

pub fn tan(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 1 {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                "__math_tan expects 1 argument (radians)",
                None,
            )
            .base,
        );
    }

    let x = match get_float_arg(&args, 0) {
        Ok(v) => v,
        Err(e) => return RuntimeResult::new().failure(e.base),
    };

    RuntimeResult::new().success(Value::Number(Number::new(x.tan())))
}

pub fn asin(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 1 {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                "__math_asin expects 1 argument",
                None,
            )
            .base,
        );
    }

    let x = match get_float_arg(&args, 0) {
        Ok(v) => v,
        Err(e) => return RuntimeResult::new().failure(e.base),
    };

    if x < -1.0 || x > 1.0 {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                "asin argument must be between -1 and 1",
                None,
            )
            .base,
        );
    }

    RuntimeResult::new().success(Value::Number(Number::new(x.asin())))
}

pub fn acos(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 1 {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                "__math_acos expects 1 argument",
                None,
            )
            .base,
        );
    }

    let x = match get_float_arg(&args, 0) {
        Ok(v) => v,
        Err(e) => return RuntimeResult::new().failure(e.base),
    };

    if x < -1.0 || x > 1.0 {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                "acos argument must be between -1 and 1",
                None,
            )
            .base,
        );
    }

    RuntimeResult::new().success(Value::Number(Number::new(x.acos())))
}

pub fn atan(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 1 {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                "__math_atan expects 1 argument",
                None,
            )
            .base,
        );
    }

    let x = match get_float_arg(&args, 0) {
        Ok(v) => v,
        Err(e) => return RuntimeResult::new().failure(e.base),
    };

    RuntimeResult::new().success(Value::Number(Number::new(x.atan())))
}

pub fn atan2(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 2 {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                "__math_atan2 expects 2 arguments (y, x)",
                None,
            )
            .base,
        );
    }

    let y = match get_float_arg(&args, 0) {
        Ok(v) => v,
        Err(e) => return RuntimeResult::new().failure(e.base),
    };

    let x = match get_float_arg(&args, 1) {
        Ok(v) => v,
        Err(e) => return RuntimeResult::new().failure(e.base),
    };

    RuntimeResult::new().success(Value::Number(Number::new(y.atan2(x))))
}

pub fn log(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 1 {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                "__math_log expects 1 argument",
                None,
            )
            .base,
        );
    }

    let x = match get_float_arg(&args, 0) {
        Ok(v) => v,
        Err(e) => return RuntimeResult::new().failure(e.base),
    };

    if x <= 0.0 {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                "Cannot take logarithm of non-positive number",
                None,
            )
            .base,
        );
    }

    RuntimeResult::new().success(Value::Number(Number::new(x.ln())))
}

pub fn log10(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 1 {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                "__math_log10 expects 1 argument",
                None,
            )
            .base,
        );
    }

    let x = match get_float_arg(&args, 0) {
        Ok(v) => v,
        Err(e) => return RuntimeResult::new().failure(e.base),
    };

    if x <= 0.0 {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                "Cannot take logarithm of non-positive number",
                None,
            )
            .base,
        );
    }

    RuntimeResult::new().success(Value::Number(Number::new(x.log10())))
}

pub fn abs(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 1 {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                "__math_abs expects 1 argument",
                None,
            )
            .base,
        );
    }

    let x = match get_float_arg(&args, 0) {
        Ok(v) => v,
        Err(e) => return RuntimeResult::new().failure(e.base),
    };

    RuntimeResult::new().success(Value::Number(Number::new(x.abs())))
}

pub fn min(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 2 {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                "__math_min expects 2 arguments",
                None,
            )
            .base,
        );
    }

    let a = match get_float_arg(&args, 0) {
        Ok(v) => v,
        Err(e) => return RuntimeResult::new().failure(e.base),
    };

    let b = match get_float_arg(&args, 1) {
        Ok(v) => v,
        Err(e) => return RuntimeResult::new().failure(e.base),
    };

    RuntimeResult::new().success(Value::Number(Number::new(a.min(b))))
}

pub fn max(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 2 {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                "__math_max expects 2 arguments",
                None,
            )
            .base,
        );
    }

    let a = match get_float_arg(&args, 0) {
        Ok(v) => v,
        Err(e) => return RuntimeResult::new().failure(e.base),
    };

    let b = match get_float_arg(&args, 1) {
        Ok(v) => v,
        Err(e) => return RuntimeResult::new().failure(e.base),
    };

    RuntimeResult::new().success(Value::Number(Number::new(a.max(b))))
}

pub fn clamp(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 3 {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                "__math_clamp expects 3 arguments (value, min, max)",
                None,
            )
            .base,
        );
    }

    let value = match get_float_arg(&args, 0) {
        Ok(v) => v,
        Err(e) => return RuntimeResult::new().failure(e.base),
    };

    let min_val = match get_float_arg(&args, 1) {
        Ok(v) => v,
        Err(e) => return RuntimeResult::new().failure(e.base),
    };

    let max_val = match get_float_arg(&args, 2) {
        Ok(v) => v,
        Err(e) => return RuntimeResult::new().failure(e.base),
    };

    let clamped = value.max(min_val).min(max_val);
    RuntimeResult::new().success(Value::Number(Number::new(clamped)))
}

pub fn round(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 1 {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                "__math_round expects 1 argument",
                None,
            )
            .base,
        );
    }

    let x = match get_float_arg(&args, 0) {
        Ok(v) => v,
        Err(e) => return RuntimeResult::new().failure(e.base),
    };

    RuntimeResult::new().success(Value::Number(Number::new(x.round())))
}

pub fn floor(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 1 {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                "__math_floor expects 1 argument",
                None,
            )
            .base,
        );
    }

    let x = match get_float_arg(&args, 0) {
        Ok(v) => v,
        Err(e) => return RuntimeResult::new().failure(e.base),
    };

    RuntimeResult::new().success(Value::Number(Number::new(x.floor())))
}

pub fn ceil(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 1 {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                "__math_ceil expects 1 argument",
                None,
            )
            .base,
        );
    }

    let x = match get_float_arg(&args, 0) {
        Ok(v) => v,
        Err(e) => return RuntimeResult::new().failure(e.base),
    };

    RuntimeResult::new().success(Value::Number(Number::new(x.ceil())))
}

pub fn trunc(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 1 {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                "__math_trunc expects 1 argument",
                None,
            )
            .base,
        );
    }

    let x = match get_float_arg(&args, 0) {
        Ok(v) => v,
        Err(e) => return RuntimeResult::new().failure(e.base),
    };

    RuntimeResult::new().success(Value::Number(Number::new(x.trunc())))
}

pub fn fract(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 1 {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                "__math_fract expects 1 argument",
                None,
            )
            .base,
        );
    }

    let x = match get_float_arg(&args, 0) {
        Ok(v) => v,
        Err(e) => return RuntimeResult::new().failure(e.base),
    };

    RuntimeResult::new().success(Value::Number(Number::new(x.fract())))
}

pub fn radians(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 1 {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                "__math_radians expects 1 argument (degrees)",
                None,
            )
            .base,
        );
    }

    let degrees = match get_float_arg(&args, 0) {
        Ok(v) => v,
        Err(e) => return RuntimeResult::new().failure(e.base),
    };

    RuntimeResult::new().success(Value::Number(Number::new(
        degrees * std::f64::consts::PI / 180.0,
    )))
}

pub fn degrees(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 1 {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                "__math_degrees expects 1 argument (radians)",
                None,
            )
            .base,
        );
    }

    let radians = match get_float_arg(&args, 0) {
        Ok(v) => v,
        Err(e) => return RuntimeResult::new().failure(e.base),
    };

    RuntimeResult::new().success(Value::Number(Number::new(
        radians * 180.0 / std::f64::consts::PI,
    )))
}

pub fn sum(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 1 {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                "__math_sum expects 1 argument (list of numbers)",
                None,
            )
            .base,
        );
    }

    let numbers = match &args[0] {
        Value::List(list) => list,
        _ => {
            return RuntimeResult::new().failure(
                RuntimeError::new(
                    dummy_pos(),
                    dummy_pos(),
                    "__math_sum: argument must be a list",
                    None,
                )
                .base,
            );
        }
    };

    let mut total = 0.0;
    for elem in &numbers.elements {
        match elem {
            Value::Number(n) => total += n.value,
            _ => {
                return RuntimeResult::new().failure(
                    RuntimeError::new(
                        dummy_pos(),
                        dummy_pos(),
                        "__math_sum: list must contain only numbers",
                        None,
                    )
                    .base,
                );
            }
        }
    }

    RuntimeResult::new().success(Value::Number(Number::new(total)))
}

pub fn average(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 1 {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                "__math_average expects 1 argument (list of numbers)",
                None,
            )
            .base,
        );
    }

    let numbers = match &args[0] {
        Value::List(list) => list,
        _ => {
            return RuntimeResult::new().failure(
                RuntimeError::new(
                    dummy_pos(),
                    dummy_pos(),
                    "__math_average: argument must be a list",
                    None,
                )
                .base,
            );
        }
    };

    if numbers.elements.is_empty() {
        return RuntimeResult::new().failure(
            RuntimeError::new(
                dummy_pos(),
                dummy_pos(),
                "__math_average: cannot average empty list",
                None,
            )
            .base,
        );
    }

    let mut total = 0.0;
    for elem in &numbers.elements {
        match elem {
            Value::Number(n) => total += n.value,
            _ => {
                return RuntimeResult::new().failure(
                    RuntimeError::new(
                        dummy_pos(),
                        dummy_pos(),
                        "__math_average: list must contain only numbers",
                        None,
                    )
                    .base,
                );
            }
        }
    }

    RuntimeResult::new().success(Value::Number(Number::new(
        total / numbers.elements.len() as f64,
    )))
}
