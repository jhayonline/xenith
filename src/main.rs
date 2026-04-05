//! # Xenith Programming Language Interpreter
//!
//! This is the main entry point for the Xenith interpreter.
//! It provides both REPL (interactive shell) and file execution modes.

use std::env;
use std::fs;
use std::io::{self, Write};

use xenith::run;

/// Runs a Xenith file
fn run_file(filename: &str) {
    match fs::read_to_string(filename) {
        Ok(source) => match run(filename, &source) {
            Ok(_) => {}
            Err(e) => eprintln!("{}", e.as_string()),
        },
        Err(e) => eprintln!("Error: Could not read file '{}': {}", filename, e),
    }
}

/// Runs the interactive REPL shell
fn run_repl() {
    println!("Xenith Interactive Shell");
    println!("Type 'exit()' to quit");
    println!("{}", "=".repeat(40));

    loop {
        print!("xenith > ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() {
            break;
        }

        let input = input.trim();
        if input.is_empty() {
            continue;
        }

        if input == "exit()" {
            println!("Goodbye!");
            break;
        }

        match run("<stdin>", input) {
            Ok(_) => {}
            Err(e) => eprintln!("{}", e.as_string()),
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() > 1 {
        run_file(&args[1]);
    } else {
        run_repl();
    }
}
