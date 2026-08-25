//! # Xenith Programming Language Interpreter
//!
//! This is the main entry point for the Xenith interpreter.
//! It provides both REPL (interactive shell) and file execution modes.

use std::env;
use std::io::Write;
use std::fs;
use std::path::Path;

use xenith::run_with_graph;
use xenith::run_repl;
use xenith::utils::value_to_string;
use xenith::values::Value;

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

    // Check the whole file, and everything it imports, before running any of
    // it -- so every static error is reported at once rather than one per
    // attempt. The graph is what does the checking, and it is handed to `run`
    // afterwards rather than built a second time: building it parses and checks
    // every module the program reaches.
    let graph = match xenith::program::build(filename, &source) {
        Ok(graph) => Some(graph),

        // This file's own errors, found with the imported signatures in hand.
        Err(xenith::modules::ModuleError::Failed { module, errors })
            if module == filename && !errors.is_empty() =>
        {
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

        // A missing module, a cycle, or a module with its own errors: the
        // interpreter reports those from the `grab` that caused them, which is
        // where they are worth pointing at.
        Err(_) => None,
    };

    if let Some(graph) = &graph {
        let module_errors = graph.check_modules_of_a_program();
        if !module_errors.is_empty() {
            for error in &module_errors {
                eprintln!("{}", error.as_string_colored());
            }
            eprintln!(
                "{} error{} found, nothing was run",
                module_errors.len(),
                if module_errors.len() == 1 { "" } else { "s" }
            );
            std::process::exit(1);
        }
    }

    let is_program = graph
        .as_ref()
        .and_then(|graph| graph.root())
        .map(|root| xenith::entry::shape_of(&root.ast) == xenith::entry::ProgramShape::Program)
        .unwrap_or(false);

    match run_with_graph(filename, &source, graph.as_ref()) {
        Ok(result) => {
            // A program's result is `main`'s, which is the exit code. A
            // script's result is its last statement's, printed under the same
            // conditions it always was.
            if is_program {
                let code = match result {
                    Value::Int(code) => code as i32,
                    // `check_main_signature` has already rejected anything but
                    // `-> int`, so this is unreachable outside a bug.
                    _ => 0,
                };
                // `exit` does not unwind, and stdout is block buffered when it
                // is a pipe rather than a terminal.
                let _ = std::io::stdout().flush();
                std::process::exit(code);
            }

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

/// Prints the bytecode a file compiles to, or why it does not compile.
///
/// A register machine cannot be debugged without this. It is not gated on
/// `XENITH_VM`: seeing the code is useful whether or not it would be run.
fn dump_bytecode(filename: &str) {
    let source = match fs::read_to_string(filename) {
        Ok(source) => source,
        Err(e) => {
            eprintln!("Error: Could not read file '{}': {}", filename, e);
            std::process::exit(1);
        }
    };

    let ast = match xenith::check_source_typed(filename, &source) {
        Ok((errors, _, ast)) if errors.is_empty() => ast,
        Ok((errors, _, _)) => {
            for error in &errors {
                eprintln!("{}", error.as_string_colored());
            }
            std::process::exit(1);
        }
        Err(fatal) => {
            eprintln!("{}", fatal.as_string_colored());
            std::process::exit(1);
        }
    };

    match xenith::vm::compile::compile(&ast) {
        Ok(chunk) => print!("{}", chunk.disassemble()),
        Err(unsupported) => {
            println!("not compiled: {}", unsupported.what);
            println!("this file runs on the tree walker");
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
            if args.len() == 3 && args[1] == "--dump-bytecode" {
                dump_bytecode(&args[2]);
            } else if args.len() > 1 {
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
