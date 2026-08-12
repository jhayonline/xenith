//! # Xenith Programming Language Interpreter
//!
//! This is the main entry point for the Xenith interpreter.
//! It provides both REPL (interactive shell) and file execution modes.

use std::env;
use std::fs;
use std::path::Path;

use xenith::run;
use xenith::run_repl;
use xenith::utils::value_to_string;

/// Runs a Xenith file
fn run_file(filename: &str) {
    // Check file extension
    let path = Path::new(filename);
    if path.extension().and_then(|ext| ext.to_str()) != Some("xen") {
        eprintln!("Error: '{}' is not a .xen file", filename);
        std::process::exit(1);
    }

    let source = match fs::read_to_string(filename) {
        Ok(source) => source,
        Err(e) => {
            eprintln!("Error: Could not read file '{}': {}", filename, e);
            std::process::exit(1);
        }
    };

    // Check the whole file before running any of it, so every static error is
    // reported at once rather than one per attempt.
    match xenith::check_source(filename, &source) {
        Err(fatal) => {
            // Lexing or parsing failed, so there is nothing to check.
            eprintln!("{}", fatal.as_string_colored());
            std::process::exit(1);
        }
        Ok(errors) if !errors.is_empty() => {
            for error in &errors {
                eprintln!("{}", error.as_string_colored());
            }
            eprintln!(
                "{} error{} found, nothing was run",
                errors.len(),
                if errors.len() == 1 { "" } else { "s" }
            );
            std::process::exit(1);
        }
        Ok(_) => {}
    }

    match run(filename, &source) {
        Ok(result) => {
            let output = value_to_string(&result);

            if !output.is_empty() && output != "null" && !output.starts_with('[') && output != "0" {
                println!("{}", output);
            }
        }
        Err(e) => {
            eprintln!("{}", e.as_string_colored());
            std::process::exit(1);
        }
    }
}

/// The interpreter recurses on the Rust stack, so Xenith recursion depth is
/// bounded by it. The default 8 MB main-thread stack runs out at roughly 1,200
/// frames; running on a dedicated large stack leaves plenty of headroom under
/// `MAX_CALL_DEPTH`, which reports a clean error rather than aborting.
const INTERPRETER_STACK_SIZE: usize = 256 * 1024 * 1024;

fn main() {
    let args: Vec<String> = env::args().collect();

    let worker = std::thread::Builder::new()
        .name("xenith".to_string())
        .stack_size(INTERPRETER_STACK_SIZE)
        .spawn(move || {
            if args.len() > 1 {
                run_file(&args[1]);
            } else if let Err(e) = run_repl() {
                eprintln!("REPL error: {}", e);
                std::process::exit(1);
            }
        })
        .expect("failed to start interpreter thread");

    if worker.join().is_err() {
        std::process::exit(1);
    }
}
