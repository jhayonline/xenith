use std::path::Path;
use std::process::Command;
use xenith::native::{check::check, codegen::{emit_object, link_host}, syntax::parse};

#[test]
fn native_try_catch_is_parsed_and_checked() {
    let source = r#"fn main() -> i32 {
        try {
            panic("boom")
        } catch (error) {
            return 42
        }
        return 0
    }"#;
    let program = parse(Path::new("main.xen"), source).expect("source should parse");
    check(&program).expect("try/catch should type-check");
}

#[test]
fn native_try_catch_runs_the_handler_after_panic() {
    let source = r#"fn main() -> i32 {
        try { panic("boom") } catch (error) { return 42 }
        return 0
    }"#;
    let program = parse(Path::new("main.xen"), source).unwrap();
    let typed = check(&program).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let object = directory.path().join("main.o");
    let executable = directory.path().join("main");
    emit_object(&typed, &object).unwrap();
    link_host(&object, &executable).unwrap();
    assert_eq!(Command::new(executable).status().unwrap().code(), Some(42));
}

#[test]
fn native_catch_exposes_error_code() {
    let source = r#"fn main() -> i32 {
        try { panic("boom") } catch (error) {
            if (error.code == "XEN300") { return 42 } else { return 1 }
        }
        return 0
    }"#;
    let program = parse(Path::new("main.xen"), source).unwrap();
    let typed = check(&program).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let object = directory.path().join("main.o");
    let executable = directory.path().join("main");
    emit_object(&typed, &object).unwrap();
    link_host(&object, &executable).unwrap();
    assert_eq!(Command::new(executable).status().unwrap().code(), Some(42));
}

#[test]
fn native_catch_handles_division_by_zero() {
    let source = r#"fn main() -> i32 {
        let denominator: i32 = 0
        try { return 10 / denominator } catch (error) {
            return len(error.code)
        }
    }"#;
    let program = parse(Path::new("main.xen"), source).unwrap();
    let typed = check(&program).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let object = directory.path().join("main.o");
    let executable = directory.path().join("main");
    emit_object(&typed, &object).unwrap();
    link_host(&object, &executable).unwrap();
    assert_eq!(Command::new(executable).status().unwrap().code(), Some(6));
}

#[test]
fn native_catch_handles_collection_bounds() {
    let source = r#"fn main() -> i32 {
        let items: list<i32> = [1]
        try { return items[2] } catch (error) {
            return len(error.code)
        }
    }"#;
    let program = parse(Path::new("main.xen"), source).unwrap();
    let typed = check(&program).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let object = directory.path().join("main.o");
    let executable = directory.path().join("main");
    emit_object(&typed, &object).unwrap();
    link_host(&object, &executable).unwrap();
    assert_eq!(Command::new(executable).status().unwrap().code(), Some(6));
}

#[test]
fn native_catch_handles_missing_map_keys() {
    let source = r#"fn main() -> i32 {
        let names: map<string, i32> = {"answer": 2}
        try { return names["missing"] } catch (error) {
            return len(error.code)
        }
    }"#;
    let program = parse(Path::new("main.xen"), source).unwrap();
    let typed = check(&program).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let object = directory.path().join("main.o");
    let executable = directory.path().join("main");
    emit_object(&typed, &object).unwrap();
    link_host(&object, &executable).unwrap();
    assert_eq!(Command::new(executable).status().unwrap().code(), Some(6));
}
