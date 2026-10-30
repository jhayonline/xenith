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
                let declaration_keyword = if diagnostic.message.starts_with("duplicate function `")
                {
                    Some("fn")
                } else if diagnostic.message.starts_with("duplicate struct `") {
                    Some("struct")
                } else {
                    None
                };
                let spans = declaration_keyword.map_or_else(
                    || identifier_spans(source, name),
                    |keyword| declaration_spans(source, keyword, name),
                );
                let occurrence = occurrences.entry(diagnostic.message.clone()).or_insert(0);
                let span = if diagnostic.message.starts_with("duplicate binding `")
                    || diagnostic.message.starts_with("duplicate parameter `")
                    || diagnostic.message.starts_with("duplicate function `")
                    || diagnostic.message.starts_with("duplicate struct `")
                {
                    spans.get(*occurrence + 1).or_else(|| spans.last())
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
            let spans = keyword_spans(source, keyword);
            let occurrence = occurrences.entry(diagnostic.message.clone()).or_insert(0);
            diagnostic.span = spans
                .get(*occurrence)
                .copied()
                .or_else(|| spans.last().copied());
            *occurrence += 1;
        }
        let operators = if diagnostic.message.starts_with("numeric operands")
            || diagnostic.message.starts_with("arithmetic operands")
        {
            Some(arithmetic_operator_spans(source))
        } else if diagnostic.message.starts_with("comparison operands")
            || diagnostic.message.starts_with("ordering comparisons")
        {
            Some(comparison_operator_spans(source))
        } else {
            None
        };
        if diagnostic.span.is_none() {
            if let Some(spans) = operators {
                let occurrence = occurrences.entry(diagnostic.message.clone()).or_insert(0);
                diagnostic.span = spans
                    .get(*occurrence)
                    .copied()
                    .or_else(|| spans.last().copied());
                *occurrence += 1;
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

fn keyword_spans(source: &str, keyword: &str) -> Vec<syntax::Span> {
    let mut line_start = 0;
    source
        .split_inclusive('\n')
        .flat_map(|line| {
            let code = line.split_once('#').map_or(line, |(code, _)| code);
            let spans = identifier_spans(code, keyword)
                .into_iter()
                .map(|span| syntax::Span::new(line_start + span.start, line_start + span.end))
                .collect::<Vec<_>>();
            line_start += line.len();
            spans
        })
        .collect()
}

fn declaration_spans(source: &str, keyword: &str, name: &str) -> Vec<syntax::Span> {
    keyword_spans(source, keyword)
        .into_iter()
        .filter_map(|keyword_span| {
            let rest = &source[keyword_span.end..];
            let start = keyword_span.end + rest.len() - rest.trim_start().len();
            let end = start + name.len();
            (source.get(start..end) == Some(name)
                && source[end..]
                    .chars()
                    .next()
                    .is_none_or(|character| !character.is_ascii_alphanumeric() && character != '_'))
            .then_some(syntax::Span::new(start, end))
        })
        .collect()
}

fn arithmetic_operator_spans(source: &str) -> Vec<syntax::Span> {
    operator_spans(source, |character| {
        matches!(character, '+' | '-' | '*' | '/')
    })
}

fn comparison_operator_spans(source: &str) -> Vec<syntax::Span> {
    operator_spans(source, |character| {
        matches!(character, '<' | '>' | '=' | '!')
    })
}

fn operator_spans(source: &str, is_operator: impl Fn(char) -> bool) -> Vec<syntax::Span> {
    let mut spans = Vec::new();
    let mut characters = source.char_indices().peekable();
    while let Some((start, character)) = characters.next() {
        if character == '#' {
            while let Some((_, character)) = characters.next() {
                if character == '\n' {
                    break;
                }
            }
            continue;
        }
        if !is_operator(character)
            || (character == '-' && characters.peek().is_some_and(|(_, next)| *next == '>'))
        {
            continue;
        }
        let mut end = start + character.len_utf8();
        if matches!(character, '<' | '>' | '=' | '!')
            && characters.peek().is_some_and(|(_, next)| *next == '=')
        {
            let (next_start, next) = characters.next().unwrap();
            end = next_start + next.len_utf8();
        }
        spans.push(syntax::Span::new(start, end));
    }
    spans
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
