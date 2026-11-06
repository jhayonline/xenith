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
    DiagnosticsMany {
        entries: Vec<(PathBuf, String, Vec<Diagnostic>)>,
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
    pub fn diagnostics_many(entries: Vec<(PathBuf, String, Vec<Diagnostic>)>) -> Self {
        Self::DiagnosticsMany { entries }
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
            Self::DiagnosticsMany { entries } => {
                let mut first = true;
                for (path, source, diagnostics) in entries {
                    for diagnostic in diagnostics {
                        if !first { writeln!(f)?; writeln!(f)?; }
                        first = false;
                        render_diagnostic(f, path, source, diagnostic)?;
                    }
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
        write!(f, "error: {}", diagnostic.message)?;
        if let Some((explanation, suggestion)) = diagnostic_help(&diagnostic.message) {
            writeln!(f)?;
            writeln!(f, "note: {explanation}")?;
            if let Some(suggestion) = suggestion {
                write!(f, "help: {suggestion}")?;
            }
        }
        return Ok(());
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
    if let Some((explanation, suggestion)) = diagnostic_help(&diagnostic.message) {
        writeln!(f)?;
        writeln!(f, "note: {explanation}")?;
        if let Some(suggestion) = suggestion {
            write!(f, "help: {suggestion}")?;
        }
    }
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

fn diagnostic_help(message: &str) -> Option<(&'static str, Option<&'static str>)> {
    if message.starts_with("unknown name `") {
        Some((
            "this name is not declared in the current scope",
            Some("check the spelling, or declare it with `let` before using it"),
        ))
    } else if message.starts_with("unknown function `") {
        Some(("the compiler cannot find a function with this name", Some("check the function name and import it with `grab { name } from \"module\"` if it lives in another file")))
    } else if message.starts_with("immutable binding `") {
        Some((
            "this binding cannot be assigned to after it is created",
            Some("use `let mut` when declaring a value that must change"),
        ))
    } else if message == "if condition must be bool" || message == "while condition must be bool" {
        Some((
            "conditions must evaluate to true or false",
            Some("compare values explicitly, for example `if (count > 0)`"),
        ))
    } else if message == "arithmetic operands require integers" {
        Some((
            "arithmetic operators need numeric operands of compatible types",
            Some("check that both sides are `i32`, `i64`, or `float` values"),
        ))
    } else if message == "comparison operands must have the same type" {
        Some((
            "a comparison can only compare values with the same type",
            Some("convert or adjust one side so both expressions have matching types"),
        ))
    } else if message.starts_with("return type mismatch") {
        Some((
            "the value returned by this function does not match its declared result type",
            Some("change the returned expression or update the function's `-> type` declaration"),
        ))
    } else if message.starts_with("type mismatch") {
        Some((
            "the expression's type does not match the type required here",
            Some("check the annotation and the value assigned to it"),
        ))
    } else if message.starts_with("closures cannot capture mutable binding") {
        Some(("native closures copy captured values when they are created, so mutable captures would become stale", Some("capture an immutable value, or pass the changing value as a function argument")))
    } else if message == "division by zero" {
        Some((
            "the divisor is known to be zero",
            Some("guard the divisor before dividing"),
        ))
    } else if message.starts_with("closure reference cycle") {
        Some((
            "reference counting cannot reclaim a cycle of closures that own one another",
            Some("capture an immutable value or pass the function as an argument instead of storing a closure back into its own environment"),
        ))
    } else if message.starts_with("unknown struct") {
        Some((
            "the compiler cannot find this struct declaration",
            Some("check the spelling or import the struct from its module"),
        ))
    } else if message.starts_with("unknown field") {
        Some((
            "this field is not declared on the value's struct type",
            Some("check the field name against the struct declaration"),
        ))
    } else if message.starts_with("map keys must be string") {
        Some((
            "native maps use strings as keys",
            Some("change the key expression to a string value"),
        ))
    } else {
        None
    }
}
impl std::error::Error for NativeError {}
