use std::{fs, path::{Path, PathBuf}};

/// The native runtime is deliberately C ABI only: generated objects can link
/// with a normal host C compiler without requiring a separately installed VM.
const SOURCE: &str = r#"#include <stdio.h>
#include <stdint.h>
void xenith_echo(const unsigned char *data, int64_t length) {
    fwrite(data, 1, (size_t)length, stdout);
    fputc('\n', stdout);
}
"#;

pub fn write_source(directory: &Path) -> std::io::Result<PathBuf> {
    let path = directory.join("xenith-runtime.c");
    fs::write(&path, SOURCE)?;
    Ok(path)
}
