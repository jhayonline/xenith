use std::{fs, path::Path, process::Command};
use xenith::native::{
    check::check,
    codegen::{emit_object, link_host},
    syntax::parse,
};

#[test]
fn native_try_catch_is_parsed_and_checked() {
    let source = r#"
    fn main() -> i32 {
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
fn native_function_keyword_declares_a_function() {
    let source = "function main() -> i32 { return 42 }";
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
fn native_catch_exposes_error_hint() {
    let source = r#"fn main() -> i32 {
        try { panic("boom") } catch (error) {
            if (len(error.hint) > 0) { return 42 } else { return 1 }
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
fn native_catch_exposes_error_function() {
    let source = r#"fn main() -> i32 {
        try { panic("boom") } catch (error) {
            if (error.function == "main" && error.file == "main.xen") { return 42 } else { return 1 }
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
fn native_catch_exposes_error_source_offset() {
    let source = r#"
    fn main() -> i32 {
        try { panic("boom") } catch (error) {
            if (error.offset > 0i64) { return 42 } else { return 1 }
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
fn native_division_error_uses_operation_offset() {
    let source = r#"fn main() -> i32 {
        let denominator: i32 = 0
        try { return 10 / denominator } catch (error) {
            if (error.line == 3i64 && error.column > 20i64) { return 42 } else { return 1 }
        }
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
        .args([
            runtime.as_os_str(),
            harness.as_os_str(),
            "-lcrypt".as_ref(),
            "-o".as_ref(),
            executable.as_os_str(),
        ])
        .status()
        .unwrap();
    assert!(status.success());
    assert!(Command::new(executable).status().unwrap().success());
}

#[test]
fn native_runtime_reference_counting_is_deterministic() {
    let directory = tempfile::tempdir().unwrap();
    let runtime = xenith::native::runtime::write_source(directory.path()).unwrap();
    let harness = directory.path().join("harness.c");
    fs::write(
        &harness,
        r#"#include <stdint.h>
        void *xenith_rc_alloc(int64_t);
        void xenith_retain(void *);
        void xenith_release(void *);
        int64_t xenith_rc_live(void);
        int main(void) {
            void *value = xenith_rc_alloc(32);
            if (!value || xenith_rc_live() != 1) return 1;
            xenith_retain(value);
            xenith_release(value);
            if (xenith_rc_live() != 1) return 2;
            xenith_release(value);
            return xenith_rc_live() == 0 ? 0 : 3;
        }
        "#,
    )
    .unwrap();
    let executable = directory.path().join("harness");
    let status = Command::new("cc")
        .args([
            runtime.as_os_str(),
            harness.as_os_str(),
            "-lcrypt".as_ref(),
            "-o".as_ref(),
            executable.as_os_str(),
        ])
        .status()
        .unwrap();
    assert!(status.success());
    assert!(Command::new(executable).status().unwrap().success());
}

#[test]
fn native_runtime_releases_closure_captures_with_the_environment() {
    let directory = tempfile::tempdir().unwrap();
    let runtime = xenith::native::runtime::write_source(directory.path()).unwrap();
    let harness = directory.path().join("harness.c");
    fs::write(
        &harness,
        r#"#include <stdint.h>
        #include <stddef.h>
        void *xenith_rc_alloc(int64_t);
        void xenith_retain(void *);
        void xenith_release(void *);
        int64_t xenith_rc_live(void);
        void *xenith_closure_new(int64_t);
        void xenith_closure_set(void *, int64_t, int64_t);
        void xenith_closure_set_mask(void *, int64_t);
        int main(void) {
            void *captured = xenith_rc_alloc(8);
            void *closure = xenith_closure_new(1);
            if (!captured || !closure || xenith_rc_live() != 2) return 1;
            xenith_closure_set(closure, 0, (int64_t)(intptr_t)captured);
            xenith_retain(captured);
            xenith_closure_set_mask(closure, 1);
            xenith_release(closure);
            if (xenith_rc_live() != 1) return 2;
            xenith_release(captured);
            xenith_release(captured);
            return xenith_rc_live() == 0 ? 0 : 3;
        }
        "#,
    )
    .unwrap();
    let executable = directory.path().join("harness");
    let status = Command::new("cc")
        .args([
            runtime.as_os_str(),
            harness.as_os_str(),
            "-lcrypt".as_ref(),
            "-o".as_ref(),
            executable.as_os_str(),
        ])
        .status()
        .unwrap();
    assert!(status.success());
    assert!(Command::new(executable).status().unwrap().success());
}

#[test]
fn native_runtime_releases_managed_map_values() {
    let directory = tempfile::tempdir().unwrap();
    let runtime = xenith::native::runtime::write_source(directory.path()).unwrap();
    let harness = directory.path().join("harness.c");
    fs::write(&harness, r#"#include <stdint.h>
        void *xenith_rc_alloc(int64_t); void xenith_retain(void *); void xenith_release(void *);
        void *xenith_map_new(int64_t, int64_t);
        void xenith_map_put(void *, int64_t, const unsigned char *, int64_t, const void *);
        void xenith_map_set_managed_descriptor(void *, int64_t, const int64_t *, const int64_t *);
        int64_t xenith_rc_live(void);
        int main(void) { void *value = xenith_rc_alloc(8); void *map = xenith_map_new(1, 8);
            int64_t offsets[] = {0}; int64_t kinds[] = {2}; const unsigned char key[] = "value";
            xenith_retain(value); xenith_map_put(map, 0, key, 5, &value);
            xenith_map_set_managed_descriptor(map, 1, offsets, kinds);
            if (xenith_rc_live() != 2) return 1; xenith_release(map);
            return xenith_rc_live() == 1 ? 0 : 2; }
        "#).unwrap();
    let executable = directory.path().join("harness");
    let status = Command::new("cc").args([runtime.as_os_str(), harness.as_os_str(), "-lcrypt".as_ref(), "-o".as_ref(), executable.as_os_str()]).status().unwrap();
    assert!(status.success());
    assert!(Command::new(executable).status().unwrap().success());
}

#[test]
fn native_runtime_map_assignment_releases_old_and_retains_new_value() {
    let directory = tempfile::tempdir().unwrap();
    let runtime = xenith::native::runtime::write_source(directory.path()).unwrap();
    let harness = directory.path().join("harness.c");
    fs::write(&harness, r#"#include <stdint.h>
        void *xenith_rc_alloc(int64_t); void xenith_retain(void *); void xenith_release(void *);
        void *xenith_map_new(int64_t, int64_t);
        void xenith_map_put(void *, int64_t, const unsigned char *, int64_t, const void *);
        void xenith_map_set_managed_descriptor(void *, int64_t, const int64_t *, const int64_t *);
        void xenith_map_assign(void *, const unsigned char *, int64_t, const void *);
        int64_t xenith_rc_live(void);
        int main(void) { void *old_value = xenith_rc_alloc(8); void *new_value = xenith_rc_alloc(8);
            void *map = xenith_map_new(1, 8); int64_t offsets[] = {0}; int64_t kinds[] = {2};
            const unsigned char key[] = "value";
            xenith_map_put(map, 0, key, 5, &old_value); xenith_map_set_managed_descriptor(map, 1, offsets, kinds);
            xenith_map_assign(map, key, 5, &new_value);
            if (xenith_rc_live() != 2) return 1; xenith_release(new_value); xenith_release(map);
            return xenith_rc_live() == 0 ? 0 : 2; }
        "#).unwrap();
    let executable = directory.path().join("harness");
    let status = Command::new("cc").args([runtime.as_os_str(), harness.as_os_str(), "-lcrypt".as_ref(), "-o".as_ref(), executable.as_os_str()]).status().unwrap();
    assert!(status.success());
    assert!(Command::new(executable).status().unwrap().success());
}

#[test]
fn native_runtime_tcp_bind_reports_invalid_configuration_recoverably() {
    let directory = tempfile::tempdir().unwrap();
    let runtime = xenith::native::runtime::write_source(directory.path()).unwrap();
    let harness = directory.path().join("harness.c");
    fs::write(&harness, r#"#include <stdint.h>
        int64_t xenith_tcp_bind(const unsigned char *, int64_t, int64_t);
        int8_t xenith_error_pending(void); int64_t xenith_error_code_len(void);
        const unsigned char *xenith_error_code(void); void xenith_error_clear(void);
        int main(void) { const unsigned char address[] = "not-an-address";
            xenith_tcp_bind(address, 14, 128);
            if (!xenith_error_pending() || xenith_error_code_len() != 6) return 1;
            xenith_error_clear(); return 0; }
        "#).unwrap();
    let executable = directory.path().join("harness");
    let status = Command::new("cc").args([runtime.as_os_str(), harness.as_os_str(), "-lcrypt".as_ref(), "-o".as_ref(), executable.as_os_str()]).status().unwrap();
    assert!(status.success());
    assert!(Command::new(executable).status().unwrap().success());
}

#[test]
fn native_runtime_append_retains_managed_elements() {
    let directory = tempfile::tempdir().unwrap();
    let runtime = xenith::native::runtime::write_source(directory.path()).unwrap();
    let harness = directory.path().join("harness.c");
    fs::write(&harness, r#"#include <stdint.h>
        void *xenith_rc_alloc(int64_t); void xenith_retain(void *); void xenith_release(void *);
        void *xenith_list_new(int64_t, int64_t); void *xenith_list_at(void *, int64_t);
        void xenith_list_set_managed_descriptor(void *, int64_t, const int64_t *, const int64_t *);
        void *xenith_list_append(void *, int64_t, const void *); int64_t xenith_rc_live(void);
        int main(void) { void *old_value = xenith_rc_alloc(8); void *new_value = xenith_rc_alloc(8);
            void *list = xenith_list_new(1, 8); int64_t offsets[] = {0}; int64_t kinds[] = {2};
            *(void **)xenith_list_at(list, 0) = old_value;
            xenith_list_set_managed_descriptor(list, 1, offsets, kinds);
            void *grown = xenith_list_append(list, 8, &new_value);
            if (xenith_rc_live() != 4) return 1; xenith_release(list); xenith_release(new_value);
            if (xenith_rc_live() != 3) return 2; xenith_release(grown);
            return xenith_rc_live() == 0 ? 0 : 3; }
        "#).unwrap();
    let executable = directory.path().join("harness");
    let status = Command::new("cc").args([runtime.as_os_str(), harness.as_os_str(), "-lcrypt".as_ref(), "-o".as_ref(), executable.as_os_str()]).status().unwrap();
    assert!(status.success());
    assert!(Command::new(executable).status().unwrap().success());
}

#[test]
fn native_runtime_rejects_closure_reference_cycles() {
    let directory = tempfile::tempdir().unwrap();
    let runtime = xenith::native::runtime::write_source(directory.path()).unwrap();
    let harness = directory.path().join("harness.c");
    fs::write(
        &harness,
        r#"#include <stdint.h>
        void *xenith_closure_new(int64_t);
        void xenith_closure_set(void *, int64_t, int64_t);
        void xenith_closure_set_mask(void *, int64_t);
        int8_t xenith_error_pending(void);
        const unsigned char *xenith_error_code(void);
        int64_t xenith_error_code_len(void);
        void xenith_error_clear(void);
        void xenith_release(void *);
        int main(void) {
            void *closure = xenith_closure_new(1);
            xenith_closure_set(closure, 0, (int64_t)(intptr_t)closure);
            xenith_closure_set_mask(closure, 1);
            if (!xenith_error_pending() || xenith_error_code_len() != 6) return 1;
            if (__builtin_memcmp(xenith_error_code(), "XEN006", 6) != 0) return 2;
            xenith_error_clear();
            xenith_release(closure);
            return 0;
        }
        "#,
    )
    .unwrap();
    let executable = directory.path().join("harness");
    let status = Command::new("cc")
        .args([
            runtime.as_os_str(),
            harness.as_os_str(),
            "-lcrypt".as_ref(),
            "-o".as_ref(),
            executable.as_os_str(),
        ])
        .status()
        .unwrap();
    assert!(status.success());
    assert!(Command::new(executable).status().unwrap().success());
}

#[test]
fn native_codegen_emits_arc_release_for_collection_locals() {
    let source = r#"
function main() -> i32 {
    let values: list<i32> = [1, 2, 3]
    return 0
}
"#;
    let program = parse(Path::new("main.xen"), source).unwrap();
    let typed = check(&program).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let object = directory.path().join("main.o");
    let executable = directory.path().join("main");
    emit_object(&typed, &object).unwrap();
    link_host(&object, &executable).unwrap();
    let bytes = fs::read(object).unwrap();
    assert!(bytes
        .windows(b"xenith_release".len())
        .any(|window| window == b"xenith_release"));
    assert_eq!(Command::new(executable).status().unwrap().code(), Some(0));
}

#[test]
fn native_codegen_retains_collection_aliases_before_releasing_locals() {
    let source = r#"
function main() -> i32 {
    let first: list<i32> = [1, 2, 3]
    let second: list<i32> = first
    return 0
}
"#;
    let program = parse(Path::new("main.xen"), source).unwrap();
    let typed = check(&program).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let object = directory.path().join("main.o");
    let executable = directory.path().join("main");
    emit_object(&typed, &object).unwrap();
    link_host(&object, &executable).unwrap();
    assert_eq!(Command::new(executable).status().unwrap().code(), Some(0));
}

#[test]
fn native_codegen_preserves_managed_values_returned_from_functions() {
    let source = r#"
function make() -> list<i32> {
    let values: list<i32> = [1, 2, 3]
    return values
}
function main() -> i32 {
    let values: list<i32> = make()
    return values[0]
}
"#;
    let program = parse(Path::new("main.xen"), source).unwrap();
    let typed = check(&program).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let object = directory.path().join("main.o");
    let executable = directory.path().join("main");
    emit_object(&typed, &object).unwrap();
    link_host(&object, &executable).unwrap();
    assert_eq!(Command::new(executable).status().unwrap().code(), Some(1));
}

#[test]
fn native_codegen_releases_collection_locals_on_nested_control_flow_returns() {
    let source = r#"
function main() -> i32 {
    let values: list<i32> = [1, 2, 3]
    if (true) {
        return 0
    } else {
        return 1
    }
}
"#;
    let program = parse(Path::new("main.xen"), source).unwrap();
    let typed = check(&program).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let object = directory.path().join("main.o");
    emit_object(&typed, &object).unwrap();
    let bytes = fs::read(object).unwrap();
    assert!(bytes
        .windows(b"xenith_release".len())
        .any(|window| window == b"xenith_release"));
}

#[test]
fn native_codegen_releases_collection_locals_on_try_catch_returns() {
    let source = r#"
function main() -> i32 {
    let values: list<i32> = [1, 2, 3]
    try {
        return 0
    } catch (error) {
        return 1
    }
}
"#;
    let program = parse(Path::new("main.xen"), source).unwrap();
    let typed = check(&program).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let object = directory.path().join("main.o");
    emit_object(&typed, &object).unwrap();
    let bytes = fs::read(object).unwrap();
    assert!(bytes
        .windows(b"xenith_release".len())
        .any(|window| window == b"xenith_release"));
}

#[test]
fn native_codegen_releases_collection_locals_in_classic_for_loops() {
    let source = r#"
function main() -> i32 {
    for (let mut i: i32 = 0; i < 1; i = i + 1) {
        let values: list<i32> = [1, 2, 3]
        if (i == 0) {
            stop
        }
    }
    return 0
}
"#;
    let program = parse(Path::new("main.xen"), source).unwrap();
    let typed = check(&program).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let object = directory.path().join("main.o");
    emit_object(&typed, &object).unwrap();
    let bytes = fs::read(object).unwrap();
    assert!(bytes
        .windows(b"xenith_release".len())
        .any(|window| window == b"xenith_release"));
}

#[test]
fn native_codegen_releases_temporaries_on_stop_and_skip() {
    let source = r#"
function main() -> i32 {
    let mut total: i32 = 0
    while (total < 2) {
        let values: list<i32> = [1, 2, 3]
        total = total + 1
        if (total == 1) {
            skip
        }
        stop
    }
    return 42
}

"#;
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
fn native_codegen_releases_temporary_collection_builtin_arguments() {
    let source = r#"
function main() -> i32 {
    return map([1, 2], fn(value: i32) -> i32 => value + 1)[0]
}
"#;
    let program = parse(Path::new("main.xen"), source).unwrap();
    let typed = check(&program).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let object = directory.path().join("main.o");
    emit_object(&typed, &object).unwrap();
    let dump = Command::new("objdump")
        .args(["-dr", object.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(dump.status.success());
    let text = String::from_utf8_lossy(&dump.stdout);
    assert!(text.matches("xenith_release").count() >= 3, "{text}");
}

#[test]
fn native_codegen_releases_nested_managed_aggregate_lanes() {
    let source = r#"
struct Bucket { values: list<i32> }
function main() -> i32 {
    let bucket: Bucket = Bucket { values: [1, 2, 3] }
    let copy: Bucket = bucket
    return len(copy.values)
}
"#;
    let program = parse(Path::new("main.xen"), source).unwrap();
    let typed = check(&program).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let object = directory.path().join("main.o");
    emit_object(&typed, &object).unwrap();
    let dump = Command::new("objdump")
        .args(["-dr", object.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(dump.status.success());
    let text = String::from_utf8_lossy(&dump.stdout);
    assert!(text.matches("xenith_release").count() >= 2, "{text}");
}

#[test]
fn native_filesystem_builtins_read_and_write_text() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("message.txt");
    let source = format!(
        "fn main() -> i32 {{ let write_error = fs_write(\"{}\", \"hello\") let result = fs_read(\"{}\") if (len(write_error) == 0 && len(result[1]) == 0 && result[0] == \"hello\") {{ return 42 }} else {{ return 1 }} }}",
        path.display(),
        path.display()
    );
    let program = parse(Path::new("main.xen"), &source).unwrap();
    let typed = check(&program).unwrap();
    let object = directory.path().join("main.o");
    let executable = directory.path().join("main");
    emit_object(&typed, &object).unwrap();
    link_host(&object, &executable).unwrap();
    assert_eq!(Command::new(executable).status().unwrap().code(), Some(42));
}

#[test]
fn native_filesystem_read_returns_an_error_string() {
    let source = "fn main() -> i32 { let result = fs_read(\"/definitely/missing/xenith-file\") if (len(result[1]) > 0) { return 42 } else { return 1 } }";
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
fn native_filesystem_append_and_byte_io_round_trip() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("payload.bin");
    let source = format!(
        "fn main() -> i32 {{ let first = fs_write_bytes(\"{}\", bytes(\"ab\")) let second = fs_append_bytes(\"{}\", bytes(\"cd\")) let result = fs_read_bytes(\"{}\") if (len(first) == 0 && len(second) == 0 && len(result[1]) == 0 && len(result[0]) == 4 && result[0][0] == 97 && result[0][3] == 100) {{ return 42 }} else {{ return 1 }} }}",
        path.display(), path.display(), path.display()
    );
    let program = parse(Path::new("main.xen"), &source).unwrap();
    let typed = check(&program).unwrap();
    let object = directory.path().join("main.o");
    let executable = directory.path().join("main");
    emit_object(&typed, &object).unwrap();
    link_host(&object, &executable).unwrap();
    assert_eq!(Command::new(executable).status().unwrap().code(), Some(42));
}

#[test]
fn native_filesystem_append_text_round_trip() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("message.txt");
    let source = format!(
        "fn main() -> i32 {{ let first = fs_write(\"{}\", \"hello\") let second = fs_append(\"{}\", \" world\") let result = fs_read(\"{}\") if (len(first) == 0 && len(second) == 0 && len(result[1]) == 0 && result[0] == \"hello world\") {{ return 42 }} else {{ return 1 }} }}",
        path.display(), path.display(), path.display()
    );
    let program = parse(Path::new("main.xen"), &source).unwrap();
    let typed = check(&program).unwrap();
    let object = directory.path().join("main.o");
    let executable = directory.path().join("main");
    emit_object(&typed, &object).unwrap();
    link_host(&object, &executable).unwrap();
    assert_eq!(Command::new(executable).status().unwrap().code(), Some(42));
}

#[test]
fn native_filesystem_metadata_and_remove_work() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("metadata.txt");
    let source = format!(
        "fn main() -> i32 {{ let write_error = fs_write(\"{}\", \"hello\") let size = fs_size(\"{}\") let file = fs_is_file(\"{}\") let exists = fs_exists(\"{}\") let remove_error = fs_remove(\"{}\") let gone = fs_exists(\"{}\") if (len(write_error) == 0 && size[0] == 5i64 && len(size[1]) == 0 && file && exists && len(remove_error) == 0 && !gone) {{ return 42 }} else {{ return 1 }} }}",
        path.display(), path.display(), path.display(), path.display(), path.display(), path.display()
    );
    let program = parse(Path::new("main.xen"), &source).unwrap();
    let typed = check(&program).unwrap();
    let object = directory.path().join("main.o");
    let executable = directory.path().join("main");
    emit_object(&typed, &object).unwrap();
    link_host(&object, &executable).unwrap();
    assert_eq!(Command::new(executable).status().unwrap().code(), Some(42));
}

#[test]
fn native_environment_builtins_round_trip() {
    let source = r#"fn main() -> i32 {
        env_set("XENITH_NATIVE_TEST", "present")
        let found = env_get("XENITH_NATIVE_TEST")
        let cwd = env_cwd()
        let args = env_args()
        let vars = env_vars()
        env_unset("XENITH_NATIVE_TEST")
        if (found[0] == "present" && found[1] && contains(vars, "XENITH_NATIVE_TEST") && len(cwd[0]) > 0 && len(cwd[1]) == 0 && len(args) > 0) { return 42 } else { return 1 }
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
fn native_filesystem_directory_operations_work() {
    let directory = tempfile::tempdir().unwrap();
    let child = directory.path().join("nested");
    let file = child.join("entry.txt");
    let source = format!(
        "fn main() -> i32 {{ let made = fs_create_dir(\"{}\") let wrote = fs_write(\"{}\", \"x\") let listed = fs_list(\"{}\") let removed = fs_remove(\"{}\") let removed_dir = fs_remove_dir(\"{}\") if (len(made) == 0 && len(wrote) == 0 && len(listed[1]) == 0 && len(listed[0]) == 1 && listed[0][0] == \"entry.txt\" && len(removed) == 0 && len(removed_dir) == 0) {{ return 42 }} else {{ return 1 }} }}",
        child.display(), file.display(), child.display(), file.display(), child.display()
    );
    let program = parse(Path::new("main.xen"), &source).unwrap();
    let typed = check(&program).unwrap();
    let object = directory.path().join("main.o");
    let executable = directory.path().join("main");
    emit_object(&typed, &object).unwrap();
    link_host(&object, &executable).unwrap();
    assert_eq!(Command::new(executable).status().unwrap().code(), Some(42));
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
    assert!(
        stderr.contains("help:"),
        "missing help suggestion: {stderr}"
    );
    assert!(stderr.contains("main.xen"), "missing source file: {stderr}");
}

#[test]
fn native_runtime_raised_messages_are_exactly_their_literal_length() {
    let directory = tempfile::tempdir().unwrap();
    let runtime = xenith::native::runtime::write_source(directory.path()).unwrap();
    let harness = directory.path().join("harness.c");
    fs::write(
        &harness,
        r#"#include <stdint.h>
        #include <string.h>
        int64_t xenith_tcp_bind(const unsigned char *, int64_t, int64_t);
        int8_t xenith_error_pending(void);
        const unsigned char *xenith_error_message(void); int64_t xenith_error_message_len(void);
        const unsigned char *xenith_error_name(void); int64_t xenith_error_name_len(void);
        void xenith_error_clear(void);
        int main(void) {
            const char expected[] = "TCP address or backlog is invalid";
            const char expected_name[] = "TCP Error";
            const unsigned char address[] = "not-an-address";
            xenith_tcp_bind(address, 14, 128);
            if (!xenith_error_pending()) return 1;
            if (xenith_error_message_len() != (int64_t)strlen(expected)) return 2;
            if (memcmp(xenith_error_message(), expected, strlen(expected))) return 3;
            if (xenith_error_name_len() != (int64_t)strlen(expected_name)) return 4;
            if (memcmp(xenith_error_name(), expected_name, strlen(expected_name))) return 5;
            xenith_error_clear();
            return 0;
        }
        "#,
    )
    .unwrap();
    let executable = directory.path().join("harness");
    let status = Command::new("cc")
        .args([
            runtime.as_os_str(),
            harness.as_os_str(),
            "-lcrypt".as_ref(),
            "-o".as_ref(),
            executable.as_os_str(),
        ])
        .status()
        .unwrap();
    assert!(status.success());
    assert_eq!(
        Command::new(executable).status().unwrap().code(),
        Some(0),
        "raised message or name did not match its literal exactly"
    );
}
