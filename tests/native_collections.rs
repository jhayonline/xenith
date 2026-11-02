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
    assert_error(
        "fn main() -> i32 { let values: map<i32, i32> = {1: 2} return 0 }",
        "map keys must be string",
    );
}

#[test]
fn executes_list_map_and_bytes_access_and_mutation() {
    let (_directory, executable) = build_source(
        "fn main() -> i32 { let mut values: list<i32> = [1, 2] values[1] = 40 let mut names: map<string, i32> = {\"answer\": 1} names[\"answer\"] = 2 let raw: bytes = bytes(\"*\") return values[1] + names[\"answer\"] + raw[0] - 42 }",
    );
    assert_eq!(Command::new(executable).status().unwrap().code(), Some(42));
}

#[test]
fn accepts_explicitly_typed_empty_collections() {
    let (_directory, executable) = build_source(
        "fn main() -> i32 { let xs: list<i32> = [] let names: map<string, i32> = {} return 42 }",
    );
    assert_eq!(Command::new(executable).status().unwrap().code(), Some(42));
}

#[test]
fn rejects_invalid_collection_operations_statically() {
    assert!(parse(
        Path::new("main.xen"),
        "fn main() -> i32 { let raw: bytes = bytes(123) return 0 }"
    )
    .is_err());
    assert_error(
        "fn main() -> i32 { let outer = [[1]] outer[0][0] = 2 return 0 }",
        "immutable binding `outer`",
    );
    assert_error("fn main() -> i32 { let mut raw: bytes = bytes(\"x\") if true { raw[\"bad\"] = 1 } return 0 }", "bytes index must be i32");
}

#[test]
fn compiles_parenthesized_if_and_for_conditions() {
    let (_directory, executable) = build_source(
        "fn main() -> i32 { let mut total: i32 = 0 if (1 < 2) { total = 40 } for (let mut i: i32 = 0; i < 1; i = i + 1) { total = total + 2 } return total }",
    );
    assert_eq!(Command::new(executable).status().unwrap().code(), Some(42));
}

#[test]
fn compiles_native_floating_point_arithmetic() {
    let (_directory, executable) = build_source(
        "fn main() -> i32 { let value: float = 1.5 * 4.0 if (value > 5.0) { return 42 } else { return 0 } }",
    );
    assert_eq!(Command::new(executable).status().unwrap().code(), Some(42));
}

#[test]
fn compiles_native_null_values_and_equality() {
    let (_directory, executable) = build_source(
        "fn main() -> i32 { let value: null = null if (value == null) { return 42 } else { return 0 } }",
    );
    assert_eq!(Command::new(executable).status().unwrap().code(), Some(42));
}
