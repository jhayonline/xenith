use std::{path::Path, process::Command};
use xenith::native::{
    check::check,
    codegen::{emit_object, link_host},
    syntax::parse,
};

#[test]
fn rejects_mutable_closure_captures() {
    let source = "fn main() -> i32 { let mut n: i32 = 1 let f: fn(i32) -> i32 = fn(x: i32) -> i32 => x + n return f(1) }";
    let program = parse(Path::new("main.xen"), source).unwrap();
    let errors = check(&program).expect_err("mutable captures should be rejected");
    assert!(
        errors
            .iter()
            .any(|error| error.message.contains("capture mutable binding `n`")),
        "{errors:?}"
    );
}

#[test]
fn rejects_non_scalar_native_closure_captures() {
    let source = "fn main() -> i32 { let prefix: string = \"x\" let f: fn(i32) -> string = fn(x: i32) -> string => prefix return 0 }";
    let program = parse(Path::new("main.xen"), source).unwrap();
    let errors = check(&program).expect_err("non-scalar captures should be rejected");
    assert!(
        errors
            .iter()
            .any(|error| error.message.contains("can only capture i32")),
        "{errors:?}"
    );
}

#[test]
fn compiles_native_named_function_values_and_indirect_calls() {
    let source = "fn add(value: i32) -> i32 { return value + 2 } fn main() -> i32 { let f: fn(i32) -> i32 = add return f(40) }";
    let program = parse(Path::new("src/main.xen"), source).unwrap();
    let typed = check(&program).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let object = directory.path().join("main.o");
    let executable = directory.path().join("main");
    emit_object(&typed, &object).unwrap();
    link_host(&object, &executable).unwrap();
    assert_eq!(Command::new(executable).status().unwrap().code(), Some(42));
}

#[test]
fn compiles_native_anonymous_non_capturing_closures() {
    let source =
        "fn main() -> i32 { let f: fn(i32) -> i32 = fn(x: i32) -> i32 => x + 1 return f(41) }";
    let dir = tempfile::tempdir().unwrap();
    let source_path = dir.path().join("main.xen");
    std::fs::write(&source_path, source).unwrap();
    let program = parse(&source_path, source).unwrap();
    let checked = check(&program).unwrap();
    let object = dir.path().join("main.o");
    emit_object(&checked, &object).unwrap();
    let executable = dir.path().join("main");
    link_host(&object, &executable).unwrap();
    assert_eq!(Command::new(executable).status().unwrap().code(), Some(42));
}

#[test]
fn compiles_native_capturing_closures() {
    let source = "fn main() -> i32 { let n: i32 = 41 let f: fn(i32) -> i32 = fn(x: i32) -> i32 => x + n return f(1) }";
    let dir = tempfile::tempdir().unwrap();
    let source_path = dir.path().join("main.xen");
    std::fs::write(&source_path, source).unwrap();
    let program = parse(&source_path, source).unwrap();
    let checked = check(&program).unwrap();
    let object = dir.path().join("main.o");
    emit_object(&checked, &object).unwrap();
    let executable = dir.path().join("main");
    link_host(&object, &executable).unwrap();
    assert_eq!(Command::new(executable).status().unwrap().code(), Some(42));
}
