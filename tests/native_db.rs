use std::{path::Path, process::Command};
use xenith::native::{check::check, codegen::{emit_object, link_host}, syntax::parse};

#[test]
fn native_database_abi_opens_executes_and_closes_sqlite() {
    let source = r#"
function main() -> i32 {
    let handle: i64 = db_open(":memory:")
    let error: string = db_exec(handle, "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT)")
    db_close(handle)
    return len(error)
}
"#;
    let program = parse(Path::new("src/main.xen"), source).unwrap();
    let typed = check(&program).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let object = directory.path().join("main.o");
    let executable = directory.path().join("main");
    emit_object(&typed, &object).unwrap();
    link_host(&object, &executable).unwrap();
    assert_eq!(Command::new(executable).status().unwrap().code(), Some(0));
}

#[test]
fn std_db_module_exposes_native_database_functions() {
    let directory = tempfile::tempdir().unwrap();
    let source = r#"
grab { open, execute, close } from "std::db"
function main() -> i32 {
    let handle: i64 = open(":memory:")
    let error: string = execute(handle, "CREATE TABLE users (id INTEGER PRIMARY KEY)")
    close(handle)
    return len(error)
}
"#;
    let main = directory.path().join("main.xen");
    std::fs::write(&main, source).unwrap();
    let (program, _) = xenith::native::project::load_program(&main).unwrap();
    let typed = check(&program).unwrap();
    let object = directory.path().join("main.o");
    let executable = directory.path().join("main");
    emit_object(&typed, &object).unwrap();
    link_host(&object, &executable).unwrap();
    assert_eq!(Command::new(executable).status().unwrap().code(), Some(0));
}

#[test]
fn native_database_store_finds_a_row_as_json() {
    let source = r#"
function main() -> i32 {
    let handle: i64 = db_open(":memory:")
    let setup: string = db_exec(handle, "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT); INSERT INTO users (id, name) VALUES (1, 'Ada')")
    let store: i64 = db_store(handle, "users")
    let found: tuple<string, string> = db_find(store, 1i64)
    db_close(handle)
    return len(found[0])
}
"#;
    let program = parse(Path::new("src/main.xen"), source).unwrap();
    let typed = check(&program).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let object = directory.path().join("main.o");
    let executable = directory.path().join("main");
    emit_object(&typed, &object).unwrap();
    link_host(&object, &executable).unwrap();
    assert_eq!(Command::new(executable).status().unwrap().code(), Some(23));
}

#[test]
fn native_database_store_decodes_columns_as_a_typed_map() {
    let source = r#"
function main() -> i32 {
    let handle: i64 = db_open(":memory:")
    let setup: string = db_exec(handle, "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT); INSERT INTO users (id, name) VALUES (1, 'Ada')")
    let store: i64 = db_store(handle, "users")
    let row: map<string, string> = db_find_map(store, 1i64)
    db_close(handle)
    return len(row["name"])
}
"#;
    let program = parse(Path::new("src/main.xen"), source).unwrap();
    let typed = check(&program).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let object = directory.path().join("main.o");
    let executable = directory.path().join("main");
    emit_object(&typed, &object).unwrap();
    link_host(&object, &executable).unwrap();
    assert_eq!(Command::new(executable).status().unwrap().code(), Some(3));
}

#[test]
fn std_db_helpers_convert_row_fields_for_struct_mapping() {
    let directory = tempfile::tempdir().unwrap();
    let source = r#"
grab { open, execute, store, find_map, as_i64, close } from "std::db"
function main() -> i32 {
    let handle: i64 = open(":memory:")
    let setup: string = execute(handle, "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT); INSERT INTO users (id, name) VALUES (7, 'Ada')")
    let users: i64 = store(handle, "users")
    let row: map<string, string> = find_map(users, 7i64)
    let id: i64 = as_i64(row, "id")
    close(handle)
    if (id == 7i64) { return 0 }
    return 1
}
"#;
    let main = directory.path().join("main.xen");
    std::fs::write(&main, source).unwrap();
    let (program, _) = xenith::native::project::load_program(&main).unwrap();
    let typed = check(&program).unwrap();
    let object = directory.path().join("main.o");
    let executable = directory.path().join("main");
    emit_object(&typed, &object).unwrap();
    link_host(&object, &executable).unwrap();
    assert_eq!(Command::new(executable).status().unwrap().code(), Some(0));
}
