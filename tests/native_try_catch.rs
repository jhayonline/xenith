use std::{fs, path::Path, process::Command};
use xenith::native::{check::check, codegen::{emit_object, link_host}, syntax::parse};

#[test]
fn native_try_catch_is_parsed_and_checked() {
    let source = r#"fn main() -> i32 {
        try {
            panic("boom")
        } catch (error) {
            return 42
        }
        return 0
    }"#;
    let program = parse(Path::new("main.xen"), source).expect("source should parse");
    check(&program).expect("try/catch should type-check");
}

#[test]
fn native_try_catch_runs_the_handler_after_panic() {
    let source = r#"fn main() -> i32 {
        try { panic("boom") } catch (error) { return 42 }
        return 0
    }"#;
    let program = parse(Path::new("main.xen"), source).unwrap();
    let typed = check(&program).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let object = directory.path().join("main.o");
    let executable = directory.path().join("main");
    emit_object(&typed, &object).unwrap();
    link_host(&object, &executable).unwrap();
    assert_eq!(Command::new(executable).status().unwrap().code(), Some(42));
}

#[test]
fn native_catch_exposes_error_code() {
    let source = r#"fn main() -> i32 {
        try { panic("boom") } catch (error) {
            if (error.code == "XEN300") { return 42 } else { return 1 }
        }
        return 0
    }"#;
    let program = parse(Path::new("main.xen"), source).unwrap();
    let typed = check(&program).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let object = directory.path().join("main.o");
    let executable = directory.path().join("main");
    emit_object(&typed, &object).unwrap();
    link_host(&object, &executable).unwrap();
    assert_eq!(Command::new(executable).status().unwrap().code(), Some(42));
}

#[test]
fn native_catch_handles_division_by_zero() {
    let source = r#"fn main() -> i32 {
        let denominator: i32 = 0
        try { return 10 / denominator } catch (error) {
            return len(error.code)
        }
    }"#;
    let program = parse(Path::new("main.xen"), source).unwrap();
    let typed = check(&program).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let object = directory.path().join("main.o");
    let executable = directory.path().join("main");
    emit_object(&typed, &object).unwrap();
    link_host(&object, &executable).unwrap();
    assert_eq!(Command::new(executable).status().unwrap().code(), Some(6));
}

#[test]
fn native_catch_handles_collection_bounds() {
    let source = r#"fn main() -> i32 {
        let items: list<i32> = [1]
        try { return items[2] } catch (error) {
            return len(error.code)
        }
    }"#;
    let program = parse(Path::new("main.xen"), source).unwrap();
    let typed = check(&program).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let object = directory.path().join("main.o");
    let executable = directory.path().join("main");
    emit_object(&typed, &object).unwrap();
    link_host(&object, &executable).unwrap();
    assert_eq!(Command::new(executable).status().unwrap().code(), Some(6));
}

#[test]
fn native_catch_handles_missing_map_keys() {
    let source = r#"fn main() -> i32 {
        let names: map<string, i32> = {"answer": 2}
        try { return names["missing"] } catch (error) {
            return len(error.code)
        }
    }"#;
    let program = parse(Path::new("main.xen"), source).unwrap();
    let typed = check(&program).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let object = directory.path().join("main.o");
    let executable = directory.path().join("main");
    emit_object(&typed, &object).unwrap();
    link_host(&object, &executable).unwrap();
    assert_eq!(Command::new(executable).status().unwrap().code(), Some(6));
}

#[test]
fn native_runtime_reports_allocation_failures_without_aborting() {
    let directory = tempfile::tempdir().unwrap();
    let runtime = xenith::native::runtime::write_source(directory.path()).unwrap();
    let harness = directory.path().join("harness.c");
    fs::write(
        &harness,
        r#"#include <stdint.h>
        #include <limits.h>
        #include <string.h>
        void *xenith_list_new(int64_t, int64_t);
        void *xenith_list_at(void *, int64_t);
        void *xenith_map_new(int64_t, int64_t);
        void *xenith_closure_new(int64_t);
        void xenith_string_concat(const unsigned char *, int64_t, const unsigned char *, int64_t, unsigned char **, int64_t *);
        int8_t xenith_error_pending(void);
        const unsigned char *xenith_error_code(void);
        int64_t xenith_error_code_len(void);
        void xenith_error_clear(void);
        int main(void) {
            xenith_list_new(INT64_MAX, INT64_MAX);
            if (!xenith_error_pending() || xenith_error_code_len() != 6 || memcmp(xenith_error_code(), "XEN005", 6) != 0) return 1;
            xenith_list_at((void *)0, 0);
            if (memcmp(xenith_error_code(), "XEN005", 6) != 0) return 5;
            xenith_error_clear();
            xenith_map_new(INT64_MAX, 1);
            if (!xenith_error_pending() || xenith_error_code_len() != 6) return 2;
            xenith_error_clear();
            xenith_closure_new(INT64_MAX);
            if (!xenith_error_pending() || xenith_error_code_len() != 6) return 3;
            xenith_error_clear();
            unsigned char *out = 0; int64_t length = 0;
            xenith_string_concat((const unsigned char *)"a", INT64_MAX, (const unsigned char *)"b", 1, &out, &length);
            return xenith_error_pending() && xenith_error_code_len() == 6 && length == 0 ? 0 : 4;
        }
        "#,
    )
    .unwrap();
    let executable = directory.path().join("harness");
    let status = Command::new("cc")
        .args([runtime.as_os_str(), harness.as_os_str(), "-o".as_ref(), executable.as_os_str()])
        .status()
        .unwrap();
    assert!(status.success());
    assert!(Command::new(executable).status().unwrap().success());
}

#[test]
fn native_uncaught_errors_print_a_diagnostic() {
    let source = r#"fn main() -> i32 {
        panic("boom")
        return 0
    }"#;
    let program = parse(Path::new("main.xen"), source).unwrap();
    let typed = check(&program).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let object = directory.path().join("main.o");
    let executable = directory.path().join("main");
    emit_object(&typed, &object).unwrap();
    link_host(&object, &executable).unwrap();
    let output = Command::new(executable).output().unwrap();
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("XEN300"), "missing error code: {stderr}");
    assert!(stderr.contains("Panic"), "missing error name: {stderr}");
    assert!(stderr.contains("boom"), "missing error message: {stderr}");
    assert!(stderr.contains("help:"), "missing help suggestion: {stderr}");
}
