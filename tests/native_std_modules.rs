//! `std::fs` and `std::env` reached the way programs reach them, through
//! `grab`. Their builtins have tests of their own, but nothing compiled the
//! wrapper modules, so a broken wrapper would have gone unnoticed.

use std::{fs, process::Command};

fn run(source: &str, directory: &tempfile::TempDir, arguments: &[&str]) -> Option<i32> {
    let main = directory.path().join("main.xen");
    fs::write(&main, source).unwrap();
    let (program, _) = xenith::native::project::load_program(&main).unwrap();
    let typed = xenith::native::check::check(&program).unwrap();
    let object = directory.path().join("main.o");
    let executable = directory.path().join("main");
    xenith::native::codegen::emit_object(&typed, &object).unwrap();
    xenith::native::codegen::link_host(&object, &executable).unwrap();
    Command::new(executable).args(arguments).status().unwrap().code()
}

#[test]
fn std_fs_round_trips_text_bytes_and_directories() {
    let directory = tempfile::tempdir().unwrap();
    let folder = directory.path().join("data");
    let source = format!(
        r#"
grab {{ read, write, append, read_bytes, write_bytes, append_bytes, remove, exists, is_file, is_dir, size, list_dir, create_dir, remove_dir }} from "std::fs"
function main() -> i32 {{
    let made: string = create_dir("{folder}")
    if (len(made) != 0) {{ return 10 }}
    if (!is_dir("{folder}")) {{ return 11 }}

    let wrote: string = write("{folder}/note.txt", "hello")
    if (len(wrote) != 0) {{ return 20 }}
    let appended: string = append("{folder}/note.txt", " world")
    if (len(appended) != 0) {{ return 21 }}
    let text: tuple<string, string> = read("{folder}/note.txt")
    if (len(text[1]) != 0) {{ return 22 }}
    if (text[0] == "hello world") {{ }} else {{ return 23 }}
    if (!exists("{folder}/note.txt")) {{ return 24 }}
    if (!is_file("{folder}/note.txt")) {{ return 25 }}
    if (is_dir("{folder}/note.txt")) {{ return 26 }}
    let measured: tuple<i64, string> = size("{folder}/note.txt")
    if (measured[0] != 11i64) {{ return 27 }}

    let first: string = write_bytes("{folder}/blob.bin", bytes("ab"))
    let second: string = append_bytes("{folder}/blob.bin", bytes("cd"))
    if (len(first) + len(second) != 0) {{ return 30 }}
    let blob: tuple<bytes, string> = read_bytes("{folder}/blob.bin")
    if (len(blob[1]) != 0) {{ return 31 }}
    if (len(blob[0]) != 4) {{ return 32 }}
    if (blob[0][0] != 97) {{ return 33 }}
    if (blob[0][3] != 100) {{ return 34 }}

    let listed: tuple<list<string>, string> = list_dir("{folder}")
    if (len(listed[1]) != 0) {{ return 40 }}
    if (len(listed[0]) != 2) {{ return 41 }}

    let removed_note: string = remove("{folder}/note.txt")
    let removed_blob: string = remove("{folder}/blob.bin")
    if (len(removed_note) + len(removed_blob) != 0) {{ return 50 }}
    if (exists("{folder}/note.txt")) {{ return 51 }}
    let removed_folder: string = remove_dir("{folder}")
    if (len(removed_folder) != 0) {{ return 52 }}
    if (exists("{folder}")) {{ return 53 }}

    let missing: tuple<string, string> = read("{folder}/absent.txt")
    if (len(missing[1]) == 0) {{ return 60 }}
    return 0
}}
"#,
        folder = folder.display()
    );
    assert_eq!(run(&source, &directory, &[]), Some(0));
}

#[test]
fn std_env_reads_and_changes_the_process_environment() {
    let directory = tempfile::tempdir().unwrap();
    let source = r#"
grab { get, set, unset, all, args, cwd } from "std::env"
function main() -> i32 {
    set("XENITH_STD_ENV_TEST", "present")
    let found: tuple<string, bool> = get("XENITH_STD_ENV_TEST")
    if (!found[1]) { return 10 }
    if (found[0] == "present") { } else { return 11 }
    let everything: map<string, string> = all()
    if (!contains(everything, "XENITH_STD_ENV_TEST")) { return 12 }

    unset("XENITH_STD_ENV_TEST")
    if (found[0] == "present") { } else { return 20 }
    let gone: tuple<string, bool> = get("XENITH_STD_ENV_TEST")
    if (gone[1]) { return 21 }

    let directory: tuple<string, string> = cwd()
    if (len(directory[1]) != 0) { return 30 }
    if (len(directory[0]) == 0) { return 31 }

    let arguments: list<string> = args()
    if (len(arguments) != 3) { return 40 }
    if (arguments[1] == "first") { } else { return 41 }
    if (arguments[2] == "second") { } else { return 42 }
    return 0
}
"#;
    assert_eq!(run(source, &directory, &["first", "second"]), Some(0));
}

#[test]
fn std_env_exit_ends_the_process_with_its_code() {
    let directory = tempfile::tempdir().unwrap();
    let source = r#"
grab { exit } from "std::env"
function main() -> i32 {
    exit(7)
    return 0
}
"#;
    assert_eq!(run(source, &directory, &[]), Some(7));
}
