pub mod cache;
pub mod check;
pub mod codegen;
pub mod error;
pub mod project;
pub mod syntax;
pub use error::NativeError;
use std::{collections::HashMap, fs, path::PathBuf, process::Command};
pub struct BuildArtifact {
    pub executable: PathBuf,
    pub reused: bool,
}
pub fn check_project(p: &project::Project) -> Result<check::TypedProgram, NativeError> {
    let s = fs::read_to_string(&p.main_path)
        .map_err(|_| NativeError::NotImplemented("missing src/main.xen".into()))?;
    let a = syntax::parse(&p.main_path, &s)
        .map_err(|e| NativeError::diagnostics(p.main_path.clone(), s.clone(), e))?;
    check::check(&a).map_err(|mut e| {
        attach_name_spans(&s, &mut e);
        NativeError::diagnostics(p.main_path.clone(), s, e)
    })
}

fn attach_name_spans(source: &str, diagnostics: &mut [syntax::Diagnostic]) {
    let mut occurrences = HashMap::new();
    for diagnostic in diagnostics {
        if diagnostic.span.is_some() {
            continue;
        }
        if let Some((_, tail)) = diagnostic.message.split_once('`') {
            if let Some((name, _)) = tail.split_once('`') {
                let spans = identifier_spans(source, name);
                let occurrence = occurrences.entry(diagnostic.message.clone()).or_insert(0);
                let span = if diagnostic.message.starts_with("duplicate binding `") {
                    spans.get(1).or_else(|| spans.first())
                } else {
                    spans.get(*occurrence).or_else(|| spans.last())
                };
                *occurrence += 1;
                if let Some(span) = span {
                    diagnostic.span = Some(*span);
                    continue;
                }
            }
        }
        let keyword = if diagnostic.message.starts_with("if condition") {
            Some("if")
        } else if diagnostic.message.starts_with("while condition") {
            Some("while")
        } else {
            None
        };
        if let Some(keyword) = keyword {
            if let Some(start) = source.rfind(keyword) {
                diagnostic.span = Some(syntax::Span::new(start, start + keyword.len()));
            }
        }
        if diagnostic.span.is_none()
            && (diagnostic.message.starts_with("numeric operands")
                || diagnostic.message.starts_with("arithmetic operands")
                || diagnostic.message.starts_with("comparison operands")
                || diagnostic.message.starts_with("ordering comparisons"))
        {
            if let Some((start, operator)) = ["==", "!=", "<=", ">=", "+", "-", "*", "/", "<", ">"]
                .iter()
                .find_map(|operator| source.rfind(operator).map(|start| (start, *operator)))
            {
                diagnostic.span = Some(syntax::Span::new(start, start + operator.len()));
            }
        }
    }
}

fn identifier_spans(source: &str, name: &str) -> Vec<syntax::Span> {
    source
        .match_indices(name)
        .filter_map(|(start, _)| {
            let end = start + name.len();
            let before = source[..start].chars().next_back();
            let after = source[end..].chars().next();
            let is_identifier_character =
                |character: char| character.is_ascii_alphanumeric() || character == '_';
            if before.is_none_or(|character| !is_identifier_character(character))
                && after.is_none_or(|character| !is_identifier_character(character))
            {
                Some(syntax::Span::new(start, end))
            } else {
                None
            }
        })
        .collect()
}
pub fn build(p: &project::Project) -> Result<BuildArtifact, NativeError> {
    let t = check_project(p)?;
    let d = p.target_dir.join("host/debug");
    fs::create_dir_all(&d).map_err(NativeError::io)?;
    let n = p.root.file_name().unwrap().to_string_lossy();
    let x = d.join(n.as_ref());
    let stamp = d.join(".xenith-fingerprint");
    let f = cache::fingerprint(&p.manifest_path, &p.main_path)?;
    if x.is_file() && fs::read_to_string(&stamp).ok().as_deref() == Some(&f) {
        return Ok(BuildArtifact {
            executable: x,
            reused: true,
        });
    }
    let o = d.join("main.o.tmp");
    let tmp = d.join(format!("{n}.tmp"));
    codegen::emit_object(&t, &o)?;
    codegen::link_host(&o, &tmp)?;
    fs::rename(tmp, &x).map_err(NativeError::io)?;
    fs::write(stamp, f).map_err(NativeError::io)?;
    Ok(BuildArtifact {
        executable: x,
        reused: false,
    })
}
pub fn run(p: &project::Project) -> Result<i32, NativeError> {
    let a = build(p)?;
    Ok(Command::new(a.executable)
        .status()
        .map_err(NativeError::io)?
        .code()
        .unwrap_or(1))
}
