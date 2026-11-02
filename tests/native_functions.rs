use std::{path::Path, process::Command};
use xenith::native::{
    check::check,
    codegen::{emit_object, link_host},
    syntax::parse,
};

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
