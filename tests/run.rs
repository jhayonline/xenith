//! Integration tests for the Xenith interpreter.
//!
//! Three suites, all driven by files rather than by Rust code, so adding a test
//! means adding a fixture and not touching this file:
//!
//! - `tests/cases/*.xen` with a matching `.out`. The program must exit 0 and
//!   produce exactly that output.
//! - `tests/errors/*.xen` with a matching `.err` holding an error code. The
//!   program must exit non-zero and report that code. A `.xen` with no `.err`
//!   is a support file for another case and is not run on its own.
//! - `tests/modules/main.xen`, which exercises imports across files.
//!
//! Plus a smoke pass over `testies/`, which only checks that the samples still
//! run at all.
//!
//! `echo` writes straight to stdout, so the interpreter is driven as a
//! subprocess rather than called in process.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Path to the interpreter Cargo just built.
const XENITH: &str = env!("CARGO_BIN_EXE_xenith");

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

struct Run {
    stdout: String,
    stderr: String,
    success: bool,
}

fn run(program: &Path) -> Run {
    let output = Command::new(XENITH)
        .arg(program)
        .output()
        .unwrap_or_else(|e| panic!("could not run {}: {e}", program.display()));

    Run {
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        success: output.status.success(),
    }
}

/// Every `.xen` in a directory, sorted so failures are reported in a stable
/// order.
fn fixtures(dir: &Path) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("could not read {}: {e}", dir.display()))
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "xen"))
        .collect();
    found.sort();
    found
}

/// Renders a difference between expected and actual output line by line, which
/// is far easier to read than two blobs when only one line moved.
fn describe_diff(expected: &str, actual: &str) -> String {
    let expected: Vec<&str> = expected.lines().collect();
    let actual: Vec<&str> = actual.lines().collect();
    let mut report = String::new();

    for i in 0..expected.len().max(actual.len()) {
        match (expected.get(i), actual.get(i)) {
            (Some(e), Some(a)) if e == a => {}
            (e, a) => {
                report.push_str(&format!(
                    "  line {}:\n    expected: {:?}\n    actual:   {:?}\n",
                    i + 1,
                    e.unwrap_or(&"<missing>"),
                    a.unwrap_or(&"<missing>")
                ));
            }
        }
    }
    report
}

#[test]
fn cases_produce_expected_output() {
    let dir = repo_root().join("tests/cases");
    let mut failures = Vec::new();
    let mut checked = 0;

    for program in fixtures(&dir) {
        let expected_path = program.with_extension("out");
        let expected = fs::read_to_string(&expected_path).unwrap_or_else(|_| {
            panic!(
                "{} has no matching .out file",
                program.file_name().unwrap().to_string_lossy()
            )
        });

        let result = run(&program);
        checked += 1;
        let name = program.file_name().unwrap().to_string_lossy().to_string();

        if !result.success {
            failures.push(format!("{name}: exited non-zero\n{}", result.stderr));
            continue;
        }
        if result.stdout != expected {
            failures.push(format!(
                "{name}: output differs\n{}",
                describe_diff(&expected, &result.stdout)
            ));
        }
    }

    assert!(checked > 0, "no case fixtures found in {}", dir.display());
    assert!(
        failures.is_empty(),
        "\n{} of {checked} case(s) failed:\n\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn errors_report_the_expected_code() {
    let dir = repo_root().join("tests/errors");
    let mut failures = Vec::new();
    let mut checked = 0;

    for program in fixtures(&dir) {
        let expected_path = program.with_extension("err");
        // A .xen with no .err is a support file imported by another case.
        let Ok(expected) = fs::read_to_string(&expected_path) else {
            continue;
        };
        // One code per line. Every one has to appear, which is how a program
        // that should report several errors at once is tested.
        let codes: Vec<&str> = expected
            .lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            .collect();

        let result = run(&program);
        checked += 1;
        let name = program.file_name().unwrap().to_string_lossy().to_string();

        if result.success {
            failures.push(format!(
                "{name}: expected {} but the program succeeded",
                codes.join(", ")
            ));
            continue;
        }

        let missing: Vec<&&str> = codes
            .iter()
            .filter(|code| !result.stderr.contains(**code))
            .collect();

        if !missing.is_empty() {
            failures.push(format!(
                "{name}: did not report {:?}, got:\n{}",
                missing,
                result.stderr.lines().take(6).collect::<Vec<_>>().join("\n")
            ));
        }
    }

    assert!(checked > 0, "no error fixtures found in {}", dir.display());
    assert!(
        failures.is_empty(),
        "\n{} of {checked} error case(s) failed:\n\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn modules_resolve_across_files() {
    let program = repo_root().join("tests/modules/main.xen");
    let expected = fs::read_to_string(program.with_extension("out")).unwrap();

    let result = run(&program);

    assert!(result.success, "module test exited non-zero:\n{}", result.stderr);
    assert_eq!(
        result.stdout,
        expected,
        "module output differs\n{}",
        describe_diff(&expected, &result.stdout)
    );
}

/// The samples under `testies/` have no expected output, so this only checks
/// that they still run. It is a canary for changes that break the language
/// wholesale, not a correctness test.
#[test]
fn samples_still_run() {
    // Known broken, with the reason. Remove an entry when its cause is fixed;
    // the test says so if one starts passing, which is how this list stays
    // honest rather than accumulating.
    const KNOWN_FAILURES: &[(&str, &str)] = &[];

    let dir = repo_root().join("testies");
    let mut unexpected_failures = Vec::new();
    let mut unexpected_passes = Vec::new();

    for program in fixtures(&dir) {
        let name = program.file_name().unwrap().to_string_lossy().to_string();
        let known = KNOWN_FAILURES.iter().find(|(f, _)| *f == name);
        let result = run(&program);

        match (known, result.success) {
            (None, false) => unexpected_failures.push(format!(
                "{name}:\n{}",
                result.stderr.lines().take(4).collect::<Vec<_>>().join("\n")
            )),
            // A known failure that now passes means the fixture should come off
            // the list, so say so rather than letting the list rot.
            (Some((_, reason)), true) => {
                unexpected_passes.push(format!("{name} now passes; it was listed as: {reason}"))
            }
            _ => {}
        }
    }

    assert!(
        unexpected_failures.is_empty(),
        "\nsamples that stopped running:\n\n{}",
        unexpected_failures.join("\n")
    );
    assert!(
        unexpected_passes.is_empty(),
        "\n{}",
        unexpected_passes.join("\n")
    );
}
