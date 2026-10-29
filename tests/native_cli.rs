use std::path::Path;
use std::process::Command;

fn xenith(directory: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_xenith"));
    command.current_dir(directory);
    command
}

#[test]
fn new_creates_a_native_project_with_main_entry() {
    let dir = tempfile::tempdir().unwrap();
    let status = xenith(dir.path()).args(["new", "hello"]).status().unwrap();
    assert!(status.success());
    assert!(dir.path().join("hello/xenith.toml").is_file());
    assert!(dir.path().join("hello/src/main.xen").is_file());
}

#[test]
fn new_rejects_an_invalid_package_name() {
    let dir = tempfile::tempdir().unwrap();
    let output = xenith(dir.path())
        .args(["new", "not-valid!"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("package name"));
}
