use std::fs;
use std::process::Command;

fn compile(source: &str, directory: &tempfile::TempDir) -> std::path::PathBuf {
    let main = directory.path().join("main.xen");
    fs::write(&main, source).unwrap();
    let (program, _) = xenith::native::project::load_program(&main).unwrap();
    let typed = xenith::native::check::check(&program).unwrap();
    let object = directory.path().join("main.o");
    let executable = directory.path().join("main");
    xenith::native::codegen::emit_object(&typed, &object).unwrap();
    xenith::native::codegen::link_host(&object, &executable).unwrap();
    executable
}

#[test]
fn auth_hashes_and_verifies_passwords_without_exposing_plaintext() {
    let directory = tempfile::tempdir().unwrap();
    let executable = compile(r#"
grab { hash_password, verify_password } from "std::auth"
function main() -> i32 {
    let encoded: string = hash_password("correct horse battery staple")
    if (len(encoded) == 0) { return 10 }
    if (!verify_password("correct horse battery staple", encoded)) { return 20 }
    if (verify_password("wrong", encoded)) { return 30 }
    if (encoded == "correct horse battery staple") { return 40 }
    return 0
}
"#, &directory);
    assert_eq!(Command::new(executable).status().unwrap().code(), Some(0));
}

#[test]
fn auth_signed_session_cookies_verify_and_reject_tampering() {
    let directory = tempfile::tempdir().unwrap();
    let executable = compile(r#"
grab { open, close } from "std::db"
grab { session_open, session_cookie_create, session_cookie_user } from "std::auth"
function main() -> i32 {
    let database: i64 = open(":memory:")
    session_open(database)
    let cookie: string = session_cookie_create(database, 7i64, 3600i64, "application-secret")
    if (len(cookie) < 80) { return 10 }
    if (session_cookie_user(database, cookie, "application-secret") != 7i64) { return 20 }
    if (session_cookie_user(database, cookie, "wrong-secret") != 0i64) { return 30 }
    close(database)
    return 0
}
"#, &directory);
    assert_eq!(Command::new(executable).status().unwrap().code(), Some(0));
}

#[test]
fn auth_sessions_create_lookup_expire_and_revoke() {
    let directory = tempfile::tempdir().unwrap();
    let executable = compile(r#"
grab { open, close } from "std::db"
grab { session_open, session_create, session_user, session_revoke } from "std::auth"
function main() -> i32 {
    let database: i64 = open(":memory:")
    let setup: string = session_open(database)
    if (len(setup) != 0) { return 10 }
    let token: string = session_create(database, 42i64, 3600i64)
    if (len(token) < 32) { return 20 }
    if (session_user(database, token) != 42i64) { return 30 }
    let revoked: string = session_revoke(database, token)
    if (len(revoked) != 0) { return 40 }
    if (session_user(database, token) != 0i64) { return 50 }
    close(database)
    return 0
}
"#, &directory);
    assert_eq!(Command::new(executable).status().unwrap().code(), Some(0));
}
