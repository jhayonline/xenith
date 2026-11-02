use std::{path::Path, process::Command};
use xenith::native::{check::check, syntax::parse};

fn assert_error(source: &str, fragment: &str) {
    let program = parse(Path::new("src/main.xen"), source).expect("source should parse");
    let errors = check(&program).expect_err("source should not type-check");
    assert!(
        errors.iter().any(|error| error.message.contains(fragment)),
        "{errors:?}"
    );
}

fn build_source(source: &str) -> (tempfile::TempDir, std::path::PathBuf) {
    let program = parse(Path::new("src/main.xen"), source).unwrap();
    let typed = check(&program).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let object = directory.path().join("main.o");
    let executable = directory.path().join("main");
    xenith::native::codegen::emit_object(&typed, &object).unwrap();
    xenith::native::codegen::link_host(&object, &executable).unwrap();
    (directory, executable)
}

#[test]
fn rejects_inhomogeneous_literals_and_writes() {
    assert_error(
        "fn main() -> i32 { let values = [1, true] return 0 }",
        "list elements must have the same type",
    );
    assert_error(
        "fn main() -> i32 { let mut values: list<i32> = [1, 2] values[0] = true return 0 }",
        "indexed assignment type mismatch",
    );
}

#[test]
fn executes_list_map_and_bytes_access_and_mutation() {
    let (_directory, executable) = build_source(
        "fn main() -> i32 { let mut values: list<i32> = [1, 2] values[1] = 40 let mut names: map<string, i32> = {\"answer\": 1} names[\"answer\"] = 2 let raw: bytes = bytes(\"*\") return values[1] + names[\"answer\"] + raw[0] - 42 }",
    );
    assert_eq!(Command::new(executable).status().unwrap().code(), Some(42));
}
