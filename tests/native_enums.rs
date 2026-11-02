use std::path::Path;
use xenith::native::{check::check, codegen::{emit_object, link_host}, syntax::parse};
use std::process::Command;

#[test]
fn parses_native_enum_declarations_and_unit_variants() {
    let program = parse(
        Path::new("src/main.xen"),
        "enum Color { Red, Green } fn main() -> i32 { return 0 }",
    )
    .expect("enum source should parse");
    assert_eq!(program.enums.len(), 1);
    assert_eq!(program.enums[0].name, "Color");
    assert_eq!(program.enums[0].variants.len(), 2);
}

#[test]
fn compiles_native_unit_enum_variant() {
    let source = "enum Color { Red, Green } fn main() -> i32 { let color: Color = Color::Green return 42 }";
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
fn compiles_native_payload_enum_variant() {
    let source = "enum Shape { Circle(float), Empty } fn main() -> i32 { let shape: Shape = Shape::Circle(2.0) return 42 }";
    let program = parse(Path::new("src/main.xen"), source).unwrap();
    let typed = check(&program).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let object = directory.path().join("main.o");
    let executable = directory.path().join("main");
    emit_object(&typed, &object).unwrap();
    link_host(&object, &executable).unwrap();
    assert_eq!(Command::new(executable).status().unwrap().code(), Some(42));
}
