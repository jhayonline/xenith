use std::{path::Path, process::Command};
use xenith::native::{check::check, codegen::{emit_object, link_host}, syntax::parse};

#[test]
fn compiles_native_string_interpolation() {
    let source = "fn main() -> i32 { let name: string = \"xenith\" echo(\"hello {name}\") return 42 }";
    let program = parse(Path::new("src/main.xen"), source).unwrap();
    let typed = check(&program).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let object = directory.path().join("main.o");
    let executable = directory.path().join("main");
    emit_object(&typed, &object).unwrap();
    link_host(&object, &executable).unwrap();
    let output = Command::new(executable).output().unwrap();
    assert_eq!(output.status.code(), Some(42));
    assert_eq!(String::from_utf8_lossy(&output.stdout), "hello xenith\n");
}
