use super::syntax::Diagnostic;
use std::{fmt, io, path::PathBuf};
#[derive(Debug)]
pub enum NativeError {
    Usage(String),
    InvalidPackageName(String),
    ProjectNotFound,
    AlreadyExists(String),
    Io(io::Error),
    NotImplemented(String),
    Diagnostics {
        path: PathBuf,
        source: String,
        diagnostics: Vec<Diagnostic>,
    },
}
impl NativeError {
    pub fn usage(m: impl Into<String>) -> Self {
        Self::Usage(m.into())
    }
    pub fn io(e: io::Error) -> Self {
        Self::Io(e)
    }
    pub fn not_implemented(m: impl Into<String>) -> Self {
        Self::NotImplemented(m.into())
    }
    pub fn diagnostics(path: PathBuf, source: String, diagnostics: Vec<Diagnostic>) -> Self {
        Self::Diagnostics {
            path,
            source,
            diagnostics,
        }
    }
}
impl fmt::Display for NativeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usage(m) => write!(f, "{m}"),
            Self::InvalidPackageName(n) => write!(
                f,
                "invalid package name `{n}`: package names use lowercase letters, digits, and underscores"
            ),
            Self::ProjectNotFound => {
                write!(f, "no xenith.toml found in this directory or its parents")
            }
            Self::AlreadyExists(p) => write!(f, "project path already exists: {p}"),
            Self::Io(e) => write!(f, "{e}"),
            Self::NotImplemented(m) => {
                write!(f, "native compiler support is not implemented yet: {m}")
            }
            Self::Diagnostics {
                path,
                source,
                diagnostics,
            } => {
                for (index, diagnostic) in diagnostics.iter().enumerate() {
                    if index > 0 {
                        writeln!(f)?;
                        writeln!(f)?;
                    }
                    render_diagnostic(f, path, source, diagnostic)?;
                }
                Ok(())
            }
        }
    }
}

fn render_diagnostic(
    f: &mut fmt::Formatter<'_>,
    path: &std::path::Path,
    source: &str,
    diagnostic: &Diagnostic,
) -> fmt::Result {
    let Some(span) = diagnostic.span else {
        return write!(f, "{}", diagnostic.message);
    };
    let prefix = &source[..span.start];
    let line_start = prefix.rfind('\n').map_or(0, |index| index + 1);
    let line_end = source[span.start..]
        .find('\n')
        .map_or(source.len(), |index| span.start + index);
    let line = &source[line_start..line_end];
    let line_number = source[..line_start]
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count()
        + 1;
    let column = source[line_start..span.start].chars().count() + 1;
    let marker = " ".repeat(column - 1);
    let width = source[span.start..span.end].chars().count().max(1);
    write!(
        f,
        "{}:{}:{}: error: {}\n{} | {}\n{} | {}{}",
        path.display(),
        line_number,
        column,
        diagnostic.message,
        line_number,
        line,
        " ".repeat(line_number.to_string().len()),
        marker,
        "^".repeat(width),
    )?;
    if let Some(name) = diagnostic
        .message
        .strip_prefix("duplicate binding `")
        .and_then(|tail| tail.strip_suffix('`'))
    {
        if let Some(previous) = source[..span.start].find(name) {
            let previous_prefix = &source[..previous];
            let previous_line_start = previous_prefix.rfind('\n').map_or(0, |index| index + 1);
            let previous_line_end = source[previous..]
                .find('\n')
                .map_or(source.len(), |index| previous + index);
            let previous_line_number = source[..previous_line_start]
                .bytes()
                .filter(|byte| *byte == b'\n')
                .count()
                + 1;
            let previous_column = source[previous_line_start..previous].chars().count() + 1;
            writeln!(f)?;
            writeln!(f)?;
            writeln!(f, "note: previous binding `{name}` here")?;
            writeln!(
                f,
                "{} | {}",
                previous_line_number,
                &source[previous_line_start..previous_line_end]
            )?;
            write!(
                f,
                "{} | {}{}",
                " ".repeat(previous_line_number.to_string().len()),
                " ".repeat(previous_column - 1),
                "^".repeat(name.chars().count())
            )?;
        }
    }
    Ok(())
}
impl std::error::Error for NativeError {}
