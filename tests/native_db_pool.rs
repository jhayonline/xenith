use std::{fs, path::Path, process::Command};
use xenith::native::{check::check, codegen::{emit_object, link_host}, syntax::parse};

#[test]
fn native_db_pool_acquires_and_releases_bounded_connections() {
    let source = r#"
function main() -> i32 {
    let pool: i64 = db_pool_new(":memory:", 2i64, 2i64)
    let first: i64 = db_pool_acquire(pool)
    let second: i64 = db_pool_acquire(pool)
    db_pool_release(pool, first)
    db_pool_release(pool, second)
    db_pool_close(pool)
    if (first != 0i64 && second != 0i64) { return 0 }
    return 1
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
fn std_db_pool_exhaustion_is_recoverable() {
    let source = r#"
grab { pool_new, pool_acquire, pool_release, pool_close } from "std::db"
function main() -> i32 {
    let pool: i64 = pool_new(":memory:", 1i64, 1i64)
    let handle: i64 = pool_acquire(pool)
    try {
        let unavailable: i64 = pool_acquire(pool)
        return 1
    } catch (error) {
        pool_release(pool, handle)
        pool_close(pool)
        if (error.code == "XEN300") { return 0 }
        return 1
    }
}
"#;
    let directory = tempfile::tempdir().unwrap();
    let main = directory.path().join("main.xen");
    fs::write(&main, source).unwrap();
    let program = xenith::native::project::load_program(&main).unwrap();
    let typed = check(&program.0).unwrap();
    let object = directory.path().join("main.o");
    let executable = directory.path().join("main");
    emit_object(&typed, &object).unwrap();
    link_host(&object, &executable).unwrap();
    assert_eq!(Command::new(executable).status().unwrap().code(), Some(0));
}
