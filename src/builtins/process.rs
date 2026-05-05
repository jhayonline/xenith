//! Process built-in functions
//! These are called by the std::process wrapper module

use crate::error::{Error, RuntimeError};
use crate::position::Position;
use crate::runtime_result::RuntimeResult;
use crate::values::{Map, Number, Struct, Value, XenithString};
use std::env;
use std::process::{Command, Stdio};

/// Result of a command execution
struct ProcessOutput {
    status: i32,
    stdout: String,
    stderr: String,
}

fn create_process_result_struct(output: ProcessOutput) -> Value {
    let mut result = Struct::new("ProcessResult".to_string());
    result.set_field(
        "exit_code".to_string(),
        Value::Number(Number::new(output.status as f64)),
    );
    result.set_field(
        "stdout".to_string(),
        Value::String(XenithString::new(output.stdout)),
    );
    result.set_field(
        "stderr".to_string(),
        Value::String(XenithString::new(output.stderr)),
    );
    Value::Struct(result)
}

fn get_string_arg(args: &[Value], index: usize, call_pos: Position) -> Result<String, Error> {
    match &args[index] {
        Value::String(s) => Ok(s.value.clone()),
        _ => Err(Error::type_mismatch(
            "string",
            "other",
            call_pos.clone(),
            call_pos,
        )),
    }
}

/// Run a command and display output in real-time
pub fn run_command(args: Vec<Value>, call_pos: Position) -> RuntimeResult {
    if args.len() != 1 {
        return RuntimeResult::new().failure(
            Error::new(
                call_pos.clone(),
                call_pos,
                "Argument Error",
                "__process_run expects 1 argument (command)",
            )
            .with_code("XEN100"),
        );
    }

    let cmd = match get_string_arg(&args, 0, call_pos.clone()) {
        Ok(s) => s,
        Err(e) => return RuntimeResult::new().failure(e),
    };

    // Parse command - support both "ls -la" and ["ls", "-la"] style
    let parts: Vec<&str> = cmd.split_whitespace().collect();
    if parts.is_empty() {
        return RuntimeResult::new()
            .failure(RuntimeError::new(call_pos.clone(), call_pos, "Empty command", None).base);
    }

    let program = parts[0];
    let args = &parts[1..];

    match Command::new(program)
        .args(args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
    {
        Ok(status) => {
            if status.success() {
                RuntimeResult::new().success(Value::Number(Number::null()))
            } else {
                RuntimeResult::new().failure(
                    RuntimeError::new(
                        call_pos.clone(),
                        call_pos,
                        &format!(
                            "Command failed with exit code: {}",
                            status.code().unwrap_or(-1)
                        ),
                        None,
                    )
                    .base,
                )
            }
        }
        Err(e) => RuntimeResult::new().failure(
            RuntimeError::new(
                call_pos.clone(),
                call_pos,
                &format!("Failed to run command: {}", e),
                None,
            )
            .base,
        ),
    }
}

/// Execute a command and return stdout as string
pub fn exec_command(args: Vec<Value>, call_pos: Position) -> RuntimeResult {
    if args.len() != 1 {
        return RuntimeResult::new().failure(
            Error::new(
                call_pos.clone(),
                call_pos,
                "Argument Error",
                "__process_exec expects 1 argument (command)",
            )
            .with_code("XEN100"),
        );
    }

    let cmd = match get_string_arg(&args, 0, call_pos.clone()) {
        Ok(s) => s,
        Err(e) => return RuntimeResult::new().failure(e),
    };

    let parts: Vec<&str> = cmd.split_whitespace().collect();
    if parts.is_empty() {
        return RuntimeResult::new()
            .failure(RuntimeError::new(call_pos.clone(), call_pos, "Empty command", None).base);
    }

    let program = parts[0];
    let args = &parts[1..];

    match Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
    {
        Ok(output) => {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                RuntimeResult::new().success(Value::String(XenithString::new(stdout)))
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                RuntimeResult::new().failure(
                    RuntimeError::new(
                        call_pos.clone(),
                        call_pos,
                        &format!("Command failed: {}", stderr),
                        None,
                    )
                    .base,
                )
            }
        }
        Err(e) => RuntimeResult::new().failure(
            RuntimeError::new(
                call_pos.clone(),
                call_pos,
                &format!("Failed to execute command: {}", e),
                None,
            )
            .base,
        ),
    }
}

/// Execute a command and return full output (exit code, stdout, stderr)
pub fn output_command(args: Vec<Value>, call_pos: Position) -> RuntimeResult {
    if args.len() != 1 {
        return RuntimeResult::new().failure(
            Error::new(
                call_pos.clone(),
                call_pos,
                "Argument Error",
                "__process_output expects 1 argument (command)",
            )
            .with_code("XEN100"),
        );
    }

    let cmd = match get_string_arg(&args, 0, call_pos.clone()) {
        Ok(s) => s,
        Err(e) => return RuntimeResult::new().failure(e),
    };

    let parts: Vec<&str> = cmd.split_whitespace().collect();
    if parts.is_empty() {
        return RuntimeResult::new()
            .failure(RuntimeError::new(call_pos.clone(), call_pos, "Empty command", None).base);
    }

    let program = parts[0];
    let args = &parts[1..];

    match Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
    {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            let exit_code = output.status.code().unwrap_or(-1);

            let result_output = ProcessOutput {
                status: exit_code,
                stdout,
                stderr,
            };

            RuntimeResult::new().success(create_process_result_struct(result_output))
        }
        Err(e) => RuntimeResult::new().failure(
            RuntimeError::new(
                call_pos.clone(),
                call_pos,
                &format!("Failed to execute command: {}", e),
                None,
            )
            .base,
        ),
    }
}

/// Get current working directory
pub fn current_dir(args: Vec<Value>, call_pos: Position) -> RuntimeResult {
    if !args.is_empty() {
        return RuntimeResult::new().failure(
            Error::new(
                call_pos.clone(),
                call_pos,
                "Argument Error",
                "__process_current_dir expects 0 arguments",
            )
            .with_code("XEN100"),
        );
    }

    match env::current_dir() {
        Ok(path) => RuntimeResult::new().success(Value::String(XenithString::new(
            path.to_string_lossy().to_string(),
        ))),
        Err(e) => RuntimeResult::new().failure(
            RuntimeError::new(
                call_pos.clone(),
                call_pos,
                &format!("Failed to get current directory: {}", e),
                None,
            )
            .base,
        ),
    }
}

/// Set current working directory
pub fn set_current_dir(args: Vec<Value>, call_pos: Position) -> RuntimeResult {
    if args.len() != 1 {
        return RuntimeResult::new().failure(
            Error::new(
                call_pos.clone(),
                call_pos,
                "Argument Error",
                "__process_set_current_dir expects 1 argument (path)",
            )
            .with_code("XEN100"),
        );
    }

    let path = match get_string_arg(&args, 0, call_pos.clone()) {
        Ok(s) => s,
        Err(e) => return RuntimeResult::new().failure(e),
    };

    match env::set_current_dir(&path) {
        Ok(_) => RuntimeResult::new().success(Value::Number(Number::null())),
        Err(e) => RuntimeResult::new().failure(
            RuntimeError::new(
                call_pos.clone(),
                call_pos,
                &format!("Failed to change directory to '{}': {}", path, e),
                None,
            )
            .base,
        ),
    }
}

/// Get environment variable
pub fn env_var(args: Vec<Value>, call_pos: Position) -> RuntimeResult {
    if args.len() != 1 {
        return RuntimeResult::new().failure(
            Error::new(
                call_pos.clone(),
                call_pos,
                "Argument Error",
                "__process_env_var expects 1 argument (key)",
            )
            .with_code("XEN100"),
        );
    }

    let key = match get_string_arg(&args, 0, call_pos.clone()) {
        Ok(s) => s,
        Err(e) => return RuntimeResult::new().failure(e),
    };

    match env::var(&key) {
        Ok(value) => RuntimeResult::new().success(Value::String(XenithString::new(value))),
        Err(e) => RuntimeResult::new().failure(
            RuntimeError::new(
                call_pos.clone(),
                call_pos,
                &format!("Environment variable '{}' not found: {}", key, e),
                None,
            )
            .base,
        ),
    }
}

/// Get all environment variables as a map
pub fn env_vars(args: Vec<Value>, call_pos: Position) -> RuntimeResult {
    if !args.is_empty() {
        return RuntimeResult::new().failure(
            Error::new(
                call_pos.clone(),
                call_pos,
                "Argument Error",
                "__process_env_vars expects 0 arguments",
            )
            .with_code("XEN100"),
        );
    }

    let mut map = Map::new();
    for (key, value) in env::vars() {
        map.set(key, Value::String(XenithString::new(value)));
    }
    RuntimeResult::new().success(Value::Map(map))
}

/// Set environment variable
pub fn set_env_var(args: Vec<Value>, call_pos: Position) -> RuntimeResult {
    if args.len() != 2 {
        return RuntimeResult::new().failure(
            Error::new(
                call_pos.clone(),
                call_pos,
                "Argument Error",
                "__process_set_env_var expects 2 arguments (key, value)",
            )
            .with_code("XEN100"),
        );
    }

    let key = match get_string_arg(&args, 0, call_pos.clone()) {
        Ok(s) => s,
        Err(e) => return RuntimeResult::new().failure(e),
    };

    let value = match get_string_arg(&args, 1, call_pos.clone()) {
        Ok(s) => s,
        Err(e) => return RuntimeResult::new().failure(e),
    };

    unsafe {
        env::set_var(key, value);
    }
    RuntimeResult::new().success(Value::Number(Number::null()))
}

/// Remove environment variable
pub fn remove_env_var(args: Vec<Value>, call_pos: Position) -> RuntimeResult {
    if args.len() != 1 {
        return RuntimeResult::new().failure(
            Error::new(
                call_pos.clone(),
                call_pos,
                "Argument Error",
                "__process_remove_env_var expects 1 argument (key)",
            )
            .with_code("XEN100"),
        );
    }

    let key = match get_string_arg(&args, 0, call_pos.clone()) {
        Ok(s) => s,
        Err(e) => return RuntimeResult::new().failure(e),
    };

    unsafe {
        env::remove_var(key);
    }
    RuntimeResult::new().success(Value::Number(Number::null()))
}

/// Exit the program with a code
pub fn exit_program(args: Vec<Value>, call_pos: Position) -> RuntimeResult {
    let code = if args.len() >= 1 {
        match &args[0] {
            Value::Number(n) => n.value as i32,
            _ => {
                return RuntimeResult::new().failure(Error::type_mismatch(
                    "number",
                    "other",
                    call_pos.clone(),
                    call_pos,
                ));
            }
        }
    } else {
        0
    };

    std::process::exit(code);
}
