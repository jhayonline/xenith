//! Every fixture, through both engines, compared byte for byte.
//!
//! This is the reason the tree walker is kept while the VM is built, and it is
//! what makes a rewrite of this size survivable. A fixture the VM declines to
//! compile is skipped and counted, not failed -- phase 3 covers a deliberately
//! small slice of the language, and the count is how that slice is watched as
//! it grows.
//!
//! What is compared is what a user sees: stdout, stderr, and the exit status.
//! Not the value, not the instruction count.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const XENITH: &str = env!("CARGO_BIN_EXE_xenith");

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

struct Output {
    stdout: String,
    stderr: String,
    status: Option<i32>,
}

fn run(program: &Path, on_vm: bool) -> Output {
    let mut command = Command::new(XENITH);
    command.arg(program);
    // The fixtures that read stdin would block; none currently do, and a new
    // one that does will hang this test rather than fail it.
    command.env_remove("XENITH_VM");
    if on_vm {
        command.env("XENITH_VM", "1");
    }

    let output = command.output().expect("xenith should run");
    Output {
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        status: output.status.code(),
    }
}

/// Every `.xen` file under a directory, sorted, non-recursive.
fn fixtures(dir: &Path) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", dir.display()))
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| path.extension().and_then(|e| e.to_str()) == Some("xen"))
        .collect();
    found.sort();
    found
}

/// Did the VM compile this file? Told by asking the disassembler.
///
/// A real disassembly, not merely the absence of a refusal: a fixture with a
/// static error makes `--dump-bytecode` exit before it reaches the compiler,
/// and counting that as compiled would fill the comparison with programs the
/// VM never runs.
fn compiles(program: &Path) -> bool {
    let output = Command::new(XENITH)
        .arg("--dump-bytecode")
        .arg(program)
        .output()
        .expect("xenith should run");
    let text = String::from_utf8_lossy(&output.stdout);
    output.status.success() && text.starts_with("constants:")
}

#[test]
fn both_engines_agree_on_every_fixture() {
    let root = repo_root();
    let dirs = [
        root.join("tests/cases"),
        root.join("tests/errors"),
        root.join("tests/modules"),
        root.join("testies"),
    ];

    let mut failures = Vec::new();
    let mut compared = 0;
    let mut skipped = 0;

    for dir in &dirs {
        for program in fixtures(dir) {
            let name = program
                .strip_prefix(&root)
                .unwrap_or(&program)
                .display()
                .to_string();

            if !compiles(&program) {
                skipped += 1;
                continue;
            }

            let walker = run(&program, false);
            let vm = run(&program, true);
            compared += 1;

            if walker.stdout != vm.stdout {
                failures.push(format!(
                    "{name}: stdout differs\n  tree walker: {:?}\n  vm:          {:?}",
                    walker.stdout, vm.stdout
                ));
            }
            if walker.stderr != vm.stderr {
                failures.push(format!(
                    "{name}: stderr differs\n  tree walker: {:?}\n  vm:          {:?}",
                    walker.stderr, vm.stderr
                ));
            }
            if walker.status != vm.status {
                failures.push(format!(
                    "{name}: exit status differs: tree walker {:?}, vm {:?}",
                    walker.status, vm.status
                ));
            }
        }
    }

    // Printed on success too, via `cargo test -- --nocapture`. The number is
    // the phase's real progress measure: it should climb every phase, and a
    // drop means something stopped compiling that used to.
    println!("differential: {compared} compared, {skipped} skipped as not compiled");

    assert!(
        failures.is_empty(),
        "\n{} of {} compiled fixture(s) diverged:\n\n{}",
        failures.len(),
        compared,
        failures.join("\n\n")
    );
}

#[test]
fn the_vm_compiles_something() {
    // A guard against the harness passing vacuously. If phase 3 compiles
    // nothing at all, `both_engines_agree_on_every_fixture` is green and
    // meaningless.
    let root = repo_root();
    let compiled = fixtures(&root.join("tests/cases"))
        .iter()
        .filter(|program| compiles(program))
        .count();

    assert!(
        compiled > 0,
        "the VM compiled none of the case fixtures, so the differential test proves nothing"
    );
}
