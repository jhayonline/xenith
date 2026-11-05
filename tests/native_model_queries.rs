use std::fs;
use std::process::Command;

#[test]
fn std_db_typed_model_queries_order_page_and_query() {
    let directory = tempfile::tempdir().unwrap();
    let source = r#"
grab { open, store, execute, close } from "std::db"
struct User { id: i64, name: string, age: i64 }
function main() -> i32 {
    let database: i64 = open(":memory:")
    execute(database, "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT, age INTEGER); INSERT INTO users VALUES (1, 'Ada', 36); INSERT INTO users VALUES (2, 'Grace', 40); INSERT INTO users VALUES (3, 'Linus', 30)")
    let users: i64 = store(database, "users")
    let ordered: list<User> = User.order(users, "age", true)
    let page: list<User> = User.page(users, 2i64, 1i64)
    let queried: list<User> = User.query(users, "age", "40", "name", false, 10i64, 0i64)
    close(database)
    if (len(ordered) != 3 || len(page) != 2 || len(queried) != 1) { return 1 }
    if (ordered[0].id != 2i64 || queried[0].id != 2i64) { return 2 }
    return 0
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
