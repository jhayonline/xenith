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

    match fs::read_to_string(filename) {
        Ok(source) => match run(filename, &source) {
            Ok(result) => {
                // Show result only if not null
                let output = value_to_string(&result);
                if !output.is_empty() && output != "null" {
                    println!("{}", output);
                }
            }
            Err(e) => {
                // Use colored error output
                eprintln!("{}", e.as_string_colored());
                std::process::exit(1);
            }
        },
        Err(e) => eprintln!("Error: Could not read file '{}': {}", filename, e),
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() > 1 {
        run_file(&args[1]);
    } else {
        if let Err(e) = run_repl() {
            eprintln!("REPL error: {}", e);
            std::process::exit(1);
        }
    }
}
