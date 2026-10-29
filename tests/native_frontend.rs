use std::path::Path;
use std::process::Command;
use xenith::native::{check::check, syntax::parse};

fn assert_error(source: &str, fragment: &str) {
    let program = parse(Path::new("src/main.xen"), source).expect("source should parse");
    let errors = check(&program).expect_err("source should not type-check");
    assert!(
        errors.iter().any(|error| error.message.contains(fragment)),
        "{errors:?}"
    );
}

fn check_source(
    source: &str,
) -> Result<xenith::native::check::TypedProgram, Vec<xenith::native::syntax::Diagnostic>> {
    let program = parse(Path::new("src/main.xen"), source).expect("source should parse");
    check(&program)
}

fn build_source(source: &str) -> std::path::PathBuf {
    let program = parse(Path::new("src/main.xen"), source).unwrap();
    let typed = check(&program).unwrap();
    let directory = tempfile::tempdir().unwrap().keep();
    let object = directory.join("main.o");
    let executable = directory.join("main");
    xenith::native::codegen::emit_object(&typed, &object).unwrap();
    xenith::native::codegen::link_host(&object, &executable).unwrap();
    executable
}

#[test]
fn accepts_typed_main_with_integer_expression() {
    let source = "fn main() -> i32 { let value: i32 = 40 + 2 return value }";
    let program = parse(Path::new("src/main.xen"), source).unwrap();
    assert!(check(&program).is_ok());
}

#[test]
fn parses_typed_function_and_call() {
    let program = parse(
        Path::new("main.xen"),
        "fn add(a: i32, b: i32) -> i32 { return a + b } fn main() -> i32 { return add(20, 22) }",
    )
    .unwrap();

    assert_eq!(program.functions[0].params.len(), 2);
}

#[test]
fn accepts_forward_calls_and_rejects_bad_calls() {
    assert!(check_source(
        "fn main() -> i32 { return add(1, 2) } fn add(a: i32, b: i32) -> i32 { return a + b }",
    )
    .is_ok());
    assert_error("fn main() -> i32 { return missing(1) }", "unknown function");
    assert_error(
        "fn add(a: i32) -> i32 { return a } fn main() -> i32 { return add(1, 2) }",
        "arguments",
    );
    assert_error(
        "fn add(a: i32) -> i32 { return a } fn main() -> i32 { return add(1i64) }",
        "argument type",
    );
}

#[test]
fn rejects_missing_or_invalid_main() {
    assert_error("fn nope() -> i32 { return 0 }", "main");
    assert_error("fn main() -> i64 { return 0 }", "-> i32");
    assert_error(
        "fn main(value: i32) -> i32 { return value }",
        "takes no parameters",
    );
}

#[test]
fn rejects_reassignment_without_mut_and_mixed_numbers() {
    assert_error("fn main() -> i32 { let x = 1 x = 2 return x }", "immutable");
    assert_error("fn main() -> i32 { return 1 + 2i64 }", "same type");
}

#[test]
fn generated_native_program_returns_main_exit_code() {
    let source = "fn main() -> i32 { return 40 + 2 }";
    let program = parse(Path::new("src/main.xen"), source).unwrap();
    let typed = check(&program).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let object = directory.path().join("main.o");
    let executable = directory.path().join("main");
    xenith::native::codegen::emit_object(&typed, &object).unwrap();
    xenith::native::codegen::link_host(&object, &executable).unwrap();
    let status = Command::new(executable).status().unwrap();
    assert_eq!(status.code(), Some(42));
}

#[test]
fn native_program_calls_typed_function() {
    let artifact = build_source(
        "fn twice(value: i32) -> i32 { return value * 2 } fn main() -> i32 { return twice(21) }",
    );
    assert_eq!(Command::new(artifact).status().unwrap().code(), Some(42));
}
