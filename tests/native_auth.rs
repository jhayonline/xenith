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
