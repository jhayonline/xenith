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

#[test]
fn native_rows_map_into_an_ordinary_xenith_struct() {
    let directory = tempfile::tempdir().unwrap();
    let source = r#"
grab { open, execute, store, find_map, as_i64, as_bool, close } from "std::db"
struct User { id: i64, name: string, active: bool }
function user_from_row(row: map<string, string>) -> User {
    return User { id: as_i64(row, "id"), name: row["name"], active: as_bool(row, "active") }
}
function main() -> i32 {
    let handle: i64 = open(":memory:")
    let setup: string = execute(handle, "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT, active INTEGER); INSERT INTO users (id, name, active) VALUES (7, 'Ada', 1)")
    let users: i64 = store(handle, "users")
    let row: map<string, string> = find_map(users, 7i64)
    let user: User = user_from_row(row)
    close(handle)
    if (user.active && user.id == 7i64) { return len(user.name) }
    return 0
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
    assert_eq!(Command::new(executable).status().unwrap().code(), Some(3));
}

#[test]
fn std_db_insert_returns_generated_id() {
    let directory = tempfile::tempdir().unwrap();
    let source = r#"
grab { open, execute, store, insert, close } from "std::db"
function main() -> i32 {
    let handle: i64 = open(":memory:")
    let setup: string = execute(handle, "CREATE TABLE users (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT)")
    let users: i64 = store(handle, "users")
    let row: map<string, string> = {"name": "Ada"}
    let id: i64 = insert(users, row)
    close(handle)
    if (id == 1i64) { return 0 }
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

#[test]
fn std_db_delete_removes_a_row() {
    let directory = tempfile::tempdir().unwrap();
    let source = r#"
grab { open, execute, store, delete, close } from "std::db"
function main() -> i32 {
    let handle: i64 = open(":memory:")
    let setup: string = execute(handle, "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT); INSERT INTO users (id, name) VALUES (1, 'Ada')")
    let users: i64 = store(handle, "users")
    let error: string = delete(users, 1i64)
    close(handle)
    if (len(error) == 0) { return 0 }
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

#[test]
fn std_db_update_changes_a_row() {
    let directory = tempfile::tempdir().unwrap();
    let source = r#"
grab { open, execute, store, update, find_map, as_i64, close } from "std::db"
function main() -> i32 {
    let handle: i64 = open(":memory:")
    let setup: string = execute(handle, "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT, age INTEGER); INSERT INTO users (id, name, age) VALUES (1, 'Ada', 36)")
    let users: i64 = store(handle, "users")
    let row: map<string, string> = {"name": "Grace", "age": "37"}
    let error: string = update(users, 1i64, row)
    let updated: map<string, string> = find_map(users, 1i64)
    let age: i64 = as_i64(updated, "age")
    close(handle)
    if (len(error) == 0 && age == 37i64 && len(updated["name"]) == 5) { return 0 }
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

#[test]
fn std_db_all_returns_rows_as_maps() {
    let directory = tempfile::tempdir().unwrap();
    let source = r#"
grab { open, execute, store, all, as_i64, close } from "std::db"
function main() -> i32 {
    let handle: i64 = open(":memory:")
    let setup: string = execute(handle, "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT); INSERT INTO users (id, name) VALUES (1, 'Ada'); INSERT INTO users (id, name) VALUES (2, 'Grace')")
    let users: i64 = store(handle, "users")
    let rows: list<map<string, string>> = all(users)
    let first: map<string, string> = rows[0]
    let second: map<string, string> = rows[1]
    let first_id: i64 = as_i64(first, "id")
    let second_id: i64 = as_i64(second, "id")
    close(handle)
    if (len(rows) == 2 && first_id == 1i64 && second_id == 2i64) { return 0 }
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

#[test]
fn std_db_filter_returns_matching_rows() {
    let directory = tempfile::tempdir().unwrap();
    let source = r#"
grab { open, execute, store, filter, as_i64, close } from "std::db"
function main() -> i32 {
    let handle: i64 = open(":memory:")
    let setup: string = execute(handle, "CREATE TABLE users (id INTEGER PRIMARY KEY, role TEXT); INSERT INTO users (id, role) VALUES (1, 'admin'); INSERT INTO users (id, role) VALUES (2, 'user')")
    let users: i64 = store(handle, "users")
    let rows: list<map<string, string>> = filter(users, "role", "admin")
    let row: map<string, string> = rows[0]
    let id: i64 = as_i64(row, "id")
    close(handle)
    if (len(rows) == 1 && id == 1i64) { return 0 }
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

#[test]
fn std_db_order_sorts_rows() {
    let directory = tempfile::tempdir().unwrap();
    let source = r#"
grab { open, execute, store, order, as_i64, close } from "std::db"
function main() -> i32 {
    let handle: i64 = open(":memory:")
    let setup: string = execute(handle, "CREATE TABLE users (id INTEGER PRIMARY KEY, score INTEGER); INSERT INTO users (id, score) VALUES (1, 20); INSERT INTO users (id, score) VALUES (2, 10)")
    let users: i64 = store(handle, "users")
    let rows: list<map<string, string>> = order(users, "score", false)
    let first: map<string, string> = rows[0]
    let score: i64 = as_i64(first, "score")
    close(handle)
    if (score == 10i64) { return 0 }
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

#[test]
fn std_db_page_limits_and_offsets_rows() {
    let directory = tempfile::tempdir().unwrap();
    let source = r#"
grab { open, execute, store, page, as_i64, close } from "std::db"
function main() -> i32 {
    let handle: i64 = open(":memory:")
    let setup: string = execute(handle, "CREATE TABLE users (id INTEGER PRIMARY KEY); INSERT INTO users (id) VALUES (1); INSERT INTO users (id) VALUES (2); INSERT INTO users (id) VALUES (3)")
    let users: i64 = store(handle, "users")
    let rows: list<map<string, string>> = page(users, 1i64, 1i64)
    let row: map<string, string> = rows[0]
    let id: i64 = as_i64(row, "id")
    close(handle)
    if (len(rows) == 1 && id == 2i64) { return 0 }
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

#[test]
fn std_db_query_combines_filter_order_and_page() {
    let directory = tempfile::tempdir().unwrap();
    let source = r#"
grab { open, execute, store, query, as_i64, close } from "std::db"
function main() -> i32 {
    let handle: i64 = open(":memory:")
    let setup: string = execute(handle, "CREATE TABLE users (id INTEGER PRIMARY KEY, role TEXT, score INTEGER); INSERT INTO users (id, role, score) VALUES (1, 'admin', 20); INSERT INTO users (id, role, score) VALUES (2, 'admin', 10); INSERT INTO users (id, role, score) VALUES (3, 'user', 30)")
    let users: i64 = store(handle, "users")
    let rows: list<map<string, string>> = query(users, "role", "admin", "score", false, 1i64, 1i64)
    let row: map<string, string> = rows[0]
    let id: i64 = as_i64(row, "id")
    close(handle)
    if (len(rows) == 1 && id == 1i64) { return 0 }
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
