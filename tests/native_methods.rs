use std::{path::Path, process::Command};
use xenith::native::{check::check, codegen::{emit_object, link_host}, syntax::parse};

#[test]
fn compiles_receiver_style_native_method_calls() {
    let source = "fn add(self: i32, amount: i32) -> i32 { return self + amount } fn main() -> i32 { let value: i32 = 40 return value.add(2) }";
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
fn specializes_generic_receiver_methods_from_the_receiver_type() {
    let source = "function first<T>(self: list<T>) -> T { return self[0] } function main() -> i32 { let values: list<i32> = [41, 42] return values.first() }";
    let program = parse(Path::new("src/main.xen"), source).unwrap();
    let typed = check(&program).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let object = directory.path().join("main.o");
    let executable = directory.path().join("main");
    emit_object(&typed, &object).unwrap();
    link_host(&object, &executable).unwrap();
    assert_eq!(Command::new(executable).status().unwrap().code(), Some(41));
}
