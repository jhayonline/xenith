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

fn fixture_project(source: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    xenith(dir.path()).args(["new", "app"]).status().unwrap();
    std::fs::write(dir.path().join("app/src/main.xen"), source).unwrap();
    dir
}

#[test]
fn build_and_run_create_a_native_artifact_and_preserve_exit_code() {
    let dir = fixture_project("fn main() -> i32 { return 7 }");
    let project = dir.path().join("app");
    assert!(xenith(&project).arg("build").status().unwrap().success());
    assert!(project.join("target/host/debug/app").is_file());
    assert_eq!(xenith(&project).arg("run").status().unwrap().code(), Some(7));
}

#[test]
fn unchanged_build_is_fresh_and_missing_main_is_a_diagnostic() {
    let dir = fixture_project("fn main() -> i32 { return 0 }");
    let project = dir.path().join("app");
    assert!(String::from_utf8_lossy(&xenith(&project).arg("build").output().unwrap().stdout).contains("Built"));
    assert!(String::from_utf8_lossy(&xenith(&project).arg("build").output().unwrap().stdout).contains("Fresh"));
    std::fs::remove_file(project.join("src/main.xen")).unwrap();
    assert!(String::from_utf8_lossy(&xenith(&project).arg("build").output().unwrap().stderr).contains("src/main.xen"));
}

#[test]
fn documented_native_workflow_is_present() {
    let readme = std::fs::read_to_string("README.md").unwrap();
    for command in ["xenith new hello", "xenith check", "xenith build", "xenith run"] {
        assert!(readme.contains(command), "README is missing `{command}`");
    }
}
