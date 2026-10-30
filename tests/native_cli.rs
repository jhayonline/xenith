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
fn native_project_builds_and_runs_an_imported_function() {
    let dir =
        fixture_project("grab { answer } from \"helper\"\nfn main() -> i32 { return answer() }");
    let project = dir.path().join("app");
    std::fs::write(
        project.join("src/helper.xen"),
        "export fn answer() -> i32 { return 42 }",
    )
    .unwrap();
    assert_eq!(
        xenith(&project).arg("run").status().unwrap().code(),
        Some(42)
    );
}

#[test]
fn native_project_rejects_an_imported_private_function() {
    let dir = fixture_project("grab { hidden } from \"helper\"\nfn main() -> i32 { return 0 }");
    let project = dir.path().join("app");
    std::fs::write(
        project.join("src/helper.xen"),
        "fn hidden() -> i32 { return 42 }",
    )
    .unwrap();
    let output = xenith(&project).arg("check").output().unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("does not export `hidden`"));
}

#[test]
fn native_project_builds_an_imported_struct() {
    let dir = fixture_project(
        "grab { Pair } from \"types\"\nfn main() -> i32 { let pair = Pair { value: 9 } return pair.value }",
    );
    let project = dir.path().join("app");
    std::fs::write(
        project.join("src/types.xen"),
        "export struct Pair { value: i32 }",
    )
    .unwrap();
    assert_eq!(
        xenith(&project).arg("run").status().unwrap().code(),
        Some(9)
    );
}

#[test]
fn native_build_rebuilds_after_an_imported_module_changes() {
    let dir =
        fixture_project("grab { answer } from \"helper\"\nfn main() -> i32 { return answer() }");
    let project = dir.path().join("app");
    let helper = project.join("src/helper.xen");
    std::fs::write(&helper, "export fn answer() -> i32 { return 1 }").unwrap();
    assert!(
        String::from_utf8_lossy(&xenith(&project).arg("build").output().unwrap().stdout)
            .contains("Built")
    );
    assert!(
        String::from_utf8_lossy(&xenith(&project).arg("build").output().unwrap().stdout)
            .contains("Fresh")
    );
    std::fs::write(&helper, "export fn answer() -> i32 { return 2 }").unwrap();
    assert!(
        String::from_utf8_lossy(&xenith(&project).arg("build").output().unwrap().stdout)
            .contains("Built")
    );
}

#[test]
fn native_project_reports_a_module_cycle() {
    let dir = fixture_project("grab { first } from \"first\"\nfn main() -> i32 { return first() }");
    let project = dir.path().join("app");
    std::fs::write(
        project.join("src/first.xen"),
        "grab { second } from \"second\"\nexport fn first() -> i32 { return second() }",
    )
    .unwrap();
    std::fs::write(
        project.join("src/second.xen"),
        "grab { first } from \"first\"\nexport fn second() -> i32 { return first() }",
    )
    .unwrap();
    let output = xenith(&project).arg("check").output().unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("circular native module import"));
}

#[test]
fn build_and_run_create_a_native_artifact_and_preserve_exit_code() {
    let dir = fixture_project("fn main() -> i32 { return 7 }");
    let project = dir.path().join("app");
    assert!(xenith(&project).arg("build").status().unwrap().success());
    assert!(project.join("target/host/debug/app").is_file());
    assert_eq!(
        xenith(&project).arg("run").status().unwrap().code(),
        Some(7)
    );
}

#[test]
fn unchanged_build_is_fresh_and_missing_main_is_a_diagnostic() {
    let dir = fixture_project("fn main() -> i32 { return 0 }");
    let project = dir.path().join("app");
    assert!(
        String::from_utf8_lossy(&xenith(&project).arg("build").output().unwrap().stdout)
            .contains("Built")
    );
    assert!(
        String::from_utf8_lossy(&xenith(&project).arg("build").output().unwrap().stdout)
            .contains("Fresh")
    );
    std::fs::remove_file(project.join("src/main.xen")).unwrap();
    assert!(
        String::from_utf8_lossy(&xenith(&project).arg("build").output().unwrap().stderr)
            .contains("src/main.xen")
    );
}

#[test]
fn check_renders_a_located_parser_diagnostic() {
    let dir = fixture_project("fn main() -> i32 { return ) }");
    let project = dir.path().join("app");

    let output = xenith(&project).arg("check").output().unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stderr.contains("src/main.xen:1:27: error: expected expression"));
    assert!(stderr.contains("fn main() -> i32 { return ) }"));
    assert!(stderr.contains("                          ^"));
}

#[test]
fn check_renders_a_located_unknown_name_diagnostic() {
    let dir = fixture_project("fn main() -> i32 { return missing }");
    let project = dir.path().join("app");
    let output = xenith(&project).arg("check").output().unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stderr.contains("src/main.xen:1:27: error: unknown name `missing`"));
    assert!(stderr.contains("1 | fn main() -> i32 { return missing }"));
    assert!(stderr.contains("                          ^"));
    assert!(stderr.contains("^^^^^^^"));
}

#[test]
fn check_locates_repeated_semantic_diagnostics_in_source_order() {
    let dir = fixture_project("fn main() -> i32 {\n    return missing\n    return missing\n}");
    let output = xenith(&dir.path().join("app"))
        .arg("check")
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stderr.contains("src/main.xen:2:12: error: unknown name `missing`"));
    assert!(stderr.contains("src/main.xen:3:12: error: unknown name `missing`"));
}

#[test]
fn check_locates_repeated_arithmetic_diagnostics_in_source_order() {
    let dir = fixture_project(
        "fn main() -> i32 {\n    let first = true + false\n    let second = true + false\n    return 0\n}",
    );
    let output = xenith(&dir.path().join("app"))
        .arg("check")
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stderr.contains("src/main.xen:2:22: error: arithmetic operands require integers"));
    assert!(stderr.contains("src/main.xen:3:23: error: arithmetic operands require integers"));
}

#[test]
fn check_locates_repeated_condition_diagnostics_in_source_order() {
    let dir = fixture_project(
        "fn main() -> i32 {\n    if 1 { return 1 } else { return 0 }\n    if 1 { return 1 } else { return 0 }\n}",
    );
    let output = xenith(&dir.path().join("app"))
        .arg("check")
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stderr.contains("src/main.xen:2:5: error: if condition must be bool"));
    assert!(stderr.contains("src/main.xen:3:5: error: if condition must be bool"));
}

#[test]
fn check_locates_each_duplicate_binding_after_the_original() {
    let dir = fixture_project(
        "fn main() -> i32 {\n    let value = 1\n    let value = 2\n    let value = 3\n    return value\n}",
    );
    let output = xenith(&dir.path().join("app"))
        .arg("check")
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stderr.contains("src/main.xen:3:9: error: duplicate binding `value`"));
    assert!(stderr.contains("src/main.xen:4:9: error: duplicate binding `value`"));
}

#[test]
fn check_locates_each_duplicate_parameter_after_the_original() {
    let dir = fixture_project(
        "fn helper(\n    value: i32,\n    value: i32,\n    value: i32\n) -> i32 { return 0 }\nfn main() -> i32 { return helper(1, 2, 3) }",
    );
    let output = xenith(&dir.path().join("app"))
        .arg("check")
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stderr.contains("src/main.xen:3:5: error: duplicate parameter `value`"));
    assert!(stderr.contains("src/main.xen:4:5: error: duplicate parameter `value`"));
}

#[test]
fn check_locates_unknown_struct_literal() {
    let dir = fixture_project("fn main() -> i32 { return Missing { value: 1 }.value }");
    let output = xenith(&dir.path().join("app"))
        .arg("check")
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stderr.contains("src/main.xen:1:27: error: unknown struct `Missing`"));
    assert!(stderr.contains("^^^^^^^"));
}

#[test]
fn check_locates_each_duplicate_function_declaration() {
    let dir = fixture_project(
        "fn helper() -> i32 { return 0 }\nfn helper() -> i32 { return 0 }\nfn helper() -> i32 { return 0 }\nfn main() -> i32 { return helper() }",
    );
    let output = xenith(&dir.path().join("app"))
        .arg("check")
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stderr.contains("src/main.xen:2:4: error: duplicate function `helper`"));
    assert!(stderr.contains("src/main.xen:3:4: error: duplicate function `helper`"));
}

#[test]
fn check_locates_each_duplicate_struct_declaration() {
    let dir = fixture_project(
        "struct Pair { value: i32 }\nstruct Pair { value: i32 }\nstruct Pair { value: i32 }\nfn main() -> i32 { let pair: Pair = Pair { value: 0 } return pair.value }",
    );
    let output = xenith(&dir.path().join("app"))
        .arg("check")
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stderr.contains("src/main.xen:2:8: error: duplicate struct `Pair`"));
    assert!(stderr.contains("src/main.xen:3:8: error: duplicate struct `Pair`"));
}

#[test]
fn check_renders_located_binding_and_condition_diagnostics() {
    let duplicate =
        fixture_project("fn main() -> i32 { let value = 1 let value = 2 return value }");
    let duplicate_output = xenith(&duplicate.path().join("app"))
        .arg("check")
        .output()
        .unwrap();
    assert!(String::from_utf8_lossy(&duplicate_output.stderr)
        .contains("error: duplicate binding `value`"));
    assert!(String::from_utf8_lossy(&duplicate_output.stderr)
        .contains("note: previous binding `value` here"));
    assert!(String::from_utf8_lossy(&duplicate_output.stderr).contains("^"));

    let condition = fixture_project("fn main() -> i32 { if 1 { return 1 } else { return 0 } }");
    let condition_output = xenith(&condition.path().join("app"))
        .arg("check")
        .output()
        .unwrap();
    assert!(String::from_utf8_lossy(&condition_output.stderr)
        .contains("error: if condition must be bool"));
    assert!(String::from_utf8_lossy(&condition_output.stderr).contains("^"));
}

#[test]
fn check_renders_a_located_type_mismatch() {
    let dir = fixture_project("fn main() -> i32 { let mut value = 1 value = false return value }");
    let output = xenith(&dir.path().join("app"))
        .arg("check")
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stderr.contains("error: type mismatch for `value`"));
    assert!(stderr.contains("^"));
}

#[test]
fn check_renders_a_located_return_type_diagnostic() {
    let dir = fixture_project("fn main() -> i32 { return false }");
    let output = xenith(&dir.path().join("app"))
        .arg("check")
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stderr.contains("error: return type mismatch in `main`"));
    assert!(stderr.contains("^"));
}

#[test]
fn check_renders_a_located_arithmetic_diagnostic() {
    let dir = fixture_project("fn main() -> i32 { return true + false }");
    let output = xenith(&dir.path().join("app"))
        .arg("check")
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stderr.contains("error: arithmetic operands require integers"));
    assert!(stderr.contains("^"));
}

#[test]
fn check_renders_every_collected_diagnostic() {
    let dir = fixture_project(
        "fn main() -> i32 { let value = 1 let value = 2 missing = 3 return value }",
    );
    let output = xenith(&dir.path().join("app"))
        .arg("check")
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stderr.contains("duplicate binding `value`"));
    assert!(stderr.contains("unknown name `missing`"));
}

#[test]
fn documented_native_workflow_is_present() {
    let readme = std::fs::read_to_string("README.md").unwrap();
    for command in [
        "xenith new hello",
        "xenith check",
        "xenith build",
        "xenith run",
    ] {
        assert!(readme.contains(command), "README is missing `{command}`");
    }
}
