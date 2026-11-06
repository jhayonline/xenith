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
fn compiles_string_native_closure_captures() {
    let source = "fn main() -> i32 { let prefix: string = \"x\" let f: fn(i32) -> string = fn(x: i32) -> string => prefix echo(f(1)) return 0 }";
    let program = parse(Path::new("main.xen"), source).unwrap();
    let checked = check(&program).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let object = dir.path().join("main.o");
    emit_object(&checked, &object).unwrap();
    let executable = dir.path().join("main");
    link_host(&object, &executable).unwrap();
    assert_eq!(Command::new(executable).status().unwrap().code(), Some(0));
}

#[test]
fn compiles_struct_native_closure_captures() {
    let source = "struct Pair { first: i32, second: i32 } fn main() -> i32 { let pair: Pair = Pair { first: 40, second: 1 } let f: fn(i32) -> i32 = fn(x: i32) -> i32 => pair.first + pair.second + x return f(1) }";
    let program = parse(Path::new("main.xen"), source).unwrap();
    let checked = check(&program).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let object = dir.path().join("main.o");
    emit_object(&checked, &object).unwrap();
    let executable = dir.path().join("main");
    link_host(&object, &executable).unwrap();
    assert_eq!(Command::new(executable).status().unwrap().code(), Some(42));
}

#[test]
fn compiles_tuple_native_closure_captures() {
    let source = "fn main() -> i32 { let pair: tuple<i32, i32> = (40, 1) let f: fn(i32) -> i32 = fn(x: i32) -> i32 => pair[0] + pair[1] + x return f(1) }";
    let program = parse(Path::new("main.xen"), source).unwrap();
    let checked = check(&program).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let object = dir.path().join("main.o");
    emit_object(&checked, &object).unwrap();
    let executable = dir.path().join("main");
    link_host(&object, &executable).unwrap();
    assert_eq!(Command::new(executable).status().unwrap().code(), Some(42));
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

#[test]
fn compiles_native_closure_captures_with_arc_cleanup() {
    let source = "fn main() -> i32 { let values: list<i32> = [41] let f: fn() -> i32 = fn() -> i32 => values[0] return f() }";
    let dir = tempfile::tempdir().unwrap();
    let source_path = dir.path().join("main.xen");
    std::fs::write(&source_path, source).unwrap();
    let program = parse(&source_path, source).unwrap();
    let checked = check(&program).unwrap();
    let object = dir.path().join("main.o");
    emit_object(&checked, &object).unwrap();
    let object_bytes = std::fs::read(&object).unwrap();
    assert!(object_bytes
        .windows(b"xenith_closure_set_mask".len())
        .any(|window| window == b"xenith_closure_set_mask"));
    let executable = dir.path().join("main");
    link_host(&object, &executable).unwrap();
    assert_eq!(Command::new(executable).status().unwrap().code(), Some(41));
}

#[test]
fn preserves_closure_environment_across_repeated_function_calls() {
    let source = "function apply(f: fn() -> i32) -> i32 { return f() } function main() -> i32 { let values: list<i32> = [41] let f: fn() -> i32 = fn() -> i32 => values[0] return apply(f) + apply(f) - 82 }";
    let dir = tempfile::tempdir().unwrap();
    let source_path = dir.path().join("main.xen");
    std::fs::write(&source_path, source).unwrap();
    let program = parse(&source_path, source).unwrap();
    let checked = check(&program).unwrap();
    let object = dir.path().join("main.o");
    emit_object(&checked, &object).unwrap();
    let executable = dir.path().join("main");
    link_host(&object, &executable).unwrap();
    assert_eq!(Command::new(executable).status().unwrap().code(), Some(0));
}

#[test]
fn preserves_closure_environment_when_returned_from_a_function() {
    let source = "function make() -> fn() -> i32 { let values: list<i32> = [41] return fn() -> i32 => values[0] } function main() -> i32 { let f: fn() -> i32 = make() return f() }";
    let dir = tempfile::tempdir().unwrap();
    let source_path = dir.path().join("main.xen");
    std::fs::write(&source_path, source).unwrap();
    let program = parse(&source_path, source).unwrap();
    let checked = check(&program).unwrap();
    let object = dir.path().join("main.o");
    emit_object(&checked, &object).unwrap();
    let executable = dir.path().join("main");
    link_host(&object, &executable).unwrap();
    assert_eq!(Command::new(executable).status().unwrap().code(), Some(41));
}

#[test]
fn preserves_nested_closure_environments() {
    let source = "function make() -> fn() -> fn() -> i32 { let base: i32 = 40 return fn() -> fn() -> i32 => fn() -> i32 => base + 2 } function main() -> i32 { let outer: fn() -> fn() -> i32 = make() let inner: fn() -> i32 = outer() return inner() }";
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
fn balances_temporary_collection_arguments() {
    let source = "function first(values: list<i32>) -> i32 { return values[0] } function main() -> i32 { return first([41]) + first([1]) }";
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
