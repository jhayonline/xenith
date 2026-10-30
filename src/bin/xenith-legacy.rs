//! Compatibility runner used by the VM differential test.
//!
//! The public `xenith` binary compiles native projects. This binary preserves
//! the script runner that compares the legacy tree walker with the bytecode VM.

use std::{env, fs, io::Write, path::Path};
use xenith::{run_with_graph, utils::value_to_string, values::Value};

fn run_file(filename: &str) {
    let path = Path::new(filename);
    if path.extension().and_then(|ext| ext.to_str()) != Some("xen") {
        eprintln!("Error: '{filename}' is not a .xen file");
        std::process::exit(1);
    }

    let source = match fs::read_to_string(filename) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("Error: Could not read file '{filename}': {error}");
            std::process::exit(1);
        }
    };
    let graph = match xenith::program::build(filename, &source) {
        Ok(graph) => Some(graph),
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
        Err(_) => None,
    };

    if let Some(graph) = &graph {
        let errors = graph.check_modules_of_a_program();
        if !errors.is_empty() {
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
    }

    let is_program = graph
        .as_ref()
        .and_then(|graph| graph.root())
        .is_some_and(|root| {
            xenith::entry::shape_of(&root.ast) == xenith::entry::ProgramShape::Program
        });
    match run_with_graph(filename, &source, graph.as_ref()) {
        Ok(result) if is_program => {
            let code = match result {
                Value::Int(code) => code as i32,
                _ => 0,
            };
            let _ = std::io::stdout().flush();
            std::process::exit(code);
        }
        Ok(result) => {
            let output = value_to_string(&result);
            if !output.is_empty() && output != "null" && !output.starts_with('[') && output != "0" {
                println!("{output}");
            }
        }
        Err(error) => {
            eprintln!("{}", error.as_string_colored());
            std::process::exit(1);
        }
    }
}

fn dump_bytecode(filename: &str) {
    let source = match fs::read_to_string(filename) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("Error: Could not read file '{filename}': {error}");
            std::process::exit(1);
        }
    };
    let (errors, types, ast) = match xenith::check_source_typed(filename, &source) {
        Ok(checked) => checked,
        Err(error) => {
            eprintln!("{}", error.as_string_colored());
            std::process::exit(1);
        }
    };
    if !errors.is_empty() {
        for error in errors {
            eprintln!("{}", error.as_string_colored());
        }
        std::process::exit(1);
    }
    match xenith::vm::compile::compile(&ast, &types) {
        Ok(chunk) => print!("{}", chunk.disassemble()),
        Err(unsupported) => {
            println!("not compiled: {}", unsupported.what);
            println!("this file runs on the tree walker");
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let worker = std::thread::Builder::new()
        .name("xenith-legacy".to_owned())
        .stack_size(256 * 1024 * 1024)
        .spawn(move || match args.as_slice() {
            [_, flag, filename] if flag == "--dump-bytecode" => dump_bytecode(filename),
            [_, filename] => run_file(filename),
            _ => {
                eprintln!("usage: xenith-legacy [--dump-bytecode] <file.xen>");
                std::process::exit(1);
            }
        })
        .expect("failed to start interpreter thread");
    if worker.join().is_err() {
        std::process::exit(1);
    }
}
