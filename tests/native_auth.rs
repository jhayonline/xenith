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
fn auth_roles_grant_check_and_revoke() {
    let directory = tempfile::tempdir().unwrap();
    let executable = compile(r#"
grab { open, close } from "std::db"
grab { role_grant, role_revoke, user_has_role } from "std::auth"
function main() -> i32 {
    let database: i64 = open(":memory:")
    let granted: string = role_grant(database, 7i64, "admin")
    if (len(granted) != 0) { return 10 }
    if (!user_has_role(database, 7i64, "admin")) { return 20 }
    let revoked: string = role_revoke(database, 7i64, "admin")
    if (len(revoked) != 0) { return 30 }
    if (user_has_role(database, 7i64, "admin")) { return 40 }
    close(database)
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

#[test]
fn auth_session_cookies_are_hmac_signed_and_never_carry_the_secret() {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("sessions.db");
    let issue = compile(&format!(r#"
grab {{ open, close }} from "std::db"
grab {{ session_open, session_cookie_create }} from "std::auth"
function main() -> i32 {{
    let database: i64 = open("{}")
    session_open(database)
    let cookie: string = session_cookie_create(database, 7i64, 3600i64, "application-secret")
    echo(cookie)
    close(database)
    return 0
}}
"#, database.display()), &directory);
    let output = Command::new(issue).output().unwrap();
    assert!(output.status.success());
    let cookie = String::from_utf8(output.stdout).unwrap().trim_end().to_string();

    let (token, signature) = cookie.split_once('.').unwrap();
    assert_eq!(token.len(), 48);
    assert_eq!(signature.len(), 64, "an HMAC-SHA256 signature is 64 hex digits");
    assert!(signature.bytes().all(|byte| byte.is_ascii_hexdigit()));
    assert!(!cookie.contains("application"), "the signing secret leaked into the cookie: {cookie}");

    let last = signature.as_bytes()[63];
    let tampered = format!("{}{}", &cookie[..cookie.len() - 1], if last == b'0' { '1' } else { '0' });
    let accepts = |candidate: &str, secret: &str| {
        let executable = compile(&format!(r#"
grab {{ open, close }} from "std::db"
grab {{ session_cookie_user }} from "std::auth"
function main() -> i32 {{
    let database: i64 = open("{}")
    let user: i64 = session_cookie_user(database, "{candidate}", "{secret}")
    close(database)
    if (user == 7i64) {{ return 0 }}
    return 1
}}
"#, database.display()), &directory);
        Command::new(executable).status().unwrap().code() == Some(0)
    };
    assert!(accepts(&cookie, "application-secret"));
    assert!(!accepts(&tampered, "application-secret"));
    assert!(!accepts(&cookie, "application-secreT"), "the whole secret, not a prefix, must key the signature");
}

#[test]
fn auth_hashes_passwords_with_pbkdf2_sha256() {
    let directory = tempfile::tempdir().unwrap();
    let executable = compile(r#"
grab { hash_password } from "std::auth"
function main() -> i32 {
    let encoded: string = hash_password("correct horse battery staple")
    echo(encoded)
    return 0
}
"#, &directory);
    let output = Command::new(executable).output().unwrap();
    assert!(output.status.success());
    let encoded = String::from_utf8(output.stdout).unwrap().trim_end().to_string();
    let fields: Vec<&str> = encoded.split('$').collect();
    assert_eq!(fields[..3], ["", "pbkdf2-sha256", "600000"], "unexpected hash format: {encoded}");
    assert_eq!(fields[3].len(), 32, "16-byte salt in hex");
    assert_eq!(fields[4].len(), 64, "32-byte derived key in hex");
}

#[test]
fn auth_verifies_an_independently_computed_pbkdf2_hash() {
    // Computed with Python's hashlib.pbkdf2_hmac, so this checks the encoding
    // and the derivation against another implementation, not against itself.
    let directory = tempfile::tempdir().unwrap();
    let executable = compile(r#"
grab { verify_password } from "std::auth"
function main() -> i32 {
    let encoded: string = "$pbkdf2-sha256$1000$000102030405060708090a0b0c0d0e0f$a69b179e3add3c1e0aaf227a0eb3aa2aa8645ab86fecf6ca00c17512697c719e"
    if (!verify_password("correct horse battery staple", encoded)) { return 10 }
    if (verify_password("correct horse battery stapler", encoded)) { return 20 }
    let weakened: string = "$pbkdf2-sha256$999$000102030405060708090a0b0c0d0e0f$a69b179e3add3c1e0aaf227a0eb3aa2aa8645ab86fecf6ca00c17512697c719e"
    if (verify_password("correct horse battery staple", weakened)) { return 30 }
    let truncated: string = "$pbkdf2-sha256$1000$000102030405060708090a0b0c0d0e0f$a69b179e"
    if (verify_password("correct horse battery staple", truncated)) { return 40 }
    return 0
}
"#, &directory);
    assert_eq!(Command::new(executable).status().unwrap().code(), Some(0));
}

#[cfg(target_os = "linux")]
#[test]
fn auth_still_verifies_legacy_sha512_crypt_hashes_on_linux() {
    // Produced by `openssl passwd -6`, the format hash_password wrote before
    // it moved to PBKDF2. Stored hashes must keep working after the upgrade.
    let directory = tempfile::tempdir().unwrap();
    let executable = compile(r#"
grab { verify_password } from "std::auth"
function main() -> i32 {
    let encoded: string = "$6$xenithsaltvalue$1VHQeatIa43SI.lniulIHy3Jf3px0rZYH9fZcepOgq8gISSfNAHIkrYF2WMtbahncAHFUCXXZNZgTGhVxxw5L1"
    if (!verify_password("correct horse battery staple", encoded)) { return 10 }
    if (verify_password("wrong", encoded)) { return 20 }
    return 0
}
"#, &directory);
    assert_eq!(Command::new(executable).status().unwrap().code(), Some(0));
}
