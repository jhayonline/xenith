use std::fs;
use std::process::Command;

#[test]
fn std_db_model_find_hydrates_an_ordinary_struct() {
    let directory = tempfile::tempdir().unwrap();
    let source = r#"
grab { open, store, execute, close } from "std::db"
struct User { id: i64, name: string, age: i64 }
function main() -> i32 {
    let database: i64 = open(":memory:")
    execute(database, "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT, age INTEGER); INSERT INTO users VALUES (1, 'Ada', 36)")
    let users: i64 = store(database, "users")
    let user: User = User.find(users, 1i64)
    close(database)
    if (user.id == 1i64 && user.name == "Ada" && user.age == 36i64) { return 0 }
    return 1
}
"#;
    let main = directory.path().join("main.xen");
    fs::write(&main, source).unwrap();
    let (program, _) = xenith::native::project::load_program(&main).unwrap();
    let typed = xenith::native::check::check(&program).unwrap();
    let object = directory.path().join("main.o");
    let executable = directory.path().join("main");
    xenith::native::codegen::emit_object(&typed, &object).unwrap();
    xenith::native::codegen::link_host(&object, &executable).unwrap();
    assert_eq!(Command::new(executable).status().unwrap().code(), Some(0));
}

#[test]
fn std_db_typed_model_helpers_validate_and_map_rows() {
    let directory = tempfile::tempdir().unwrap();
    let source = r#"
grab { open, store, schema, field, migrate, insert_model, find_model, update_model, close, schema_close } from "std::db"
function main() -> i32 {
    let database: i64 = open(":memory:")
    let definition: i64 = schema("users")
    field(definition, "name", "string")
    field(definition, "age", "i64")
    migrate(database, "users", definition)
    let users: i64 = store(database, "users")
    let model: map<string, string> = {"id": "i64", "name": "string", "age": "i64"}
    let id: i64 = insert_model(users, model, {"id": "1", "name": "Ada", "age": "36"})
    let row: map<string, string> = find_model(users, id, model)
    let update_error: string = update_model(users, id, model, {"id": "1", "name": "Ada", "age": "37"})
    schema_close(definition)
    close(database)
    if (len(update_error) == 0 && row["name"] == "Ada" && row["age"] == "36") { return 0 }
    return 1
}
"#;
    let main = directory.path().join("main.xen");
    fs::write(&main, source).unwrap();
    let (program, _) = xenith::native::project::load_program(&main).unwrap();
    let typed = xenith::native::check::check(&program).unwrap();
    let object = directory.path().join("main.o");
    let executable = directory.path().join("main");
    xenith::native::codegen::emit_object(&typed, &object).unwrap();
    xenith::native::codegen::link_host(&object, &executable).unwrap();
    assert_eq!(Command::new(executable).status().unwrap().code(), Some(0));
}
