use super::{
    syntax::{self, Diagnostic, Program},
    NativeError,
};
use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};
#[derive(Debug, Clone)]
pub struct Project {
    pub root: PathBuf,
    pub manifest_path: PathBuf,
    pub main_path: PathBuf,
    pub target_dir: PathBuf,
}

pub fn load_program(main_path: &Path) -> Result<(Program, Vec<PathBuf>), NativeError> {
    if !main_path.is_file() {
        return Err(NativeError::NotImplemented("missing src/main.xen".into()));
    }
    let mut visiting = HashSet::new();
    let mut loaded = HashSet::new();
    let mut paths = Vec::new();
    let program = load_module(main_path, true, &mut visiting, &mut loaded, &mut paths)?;
    Ok((program, paths))
}

fn load_module(
    path: &Path,
    root: bool,
    visiting: &mut HashSet<PathBuf>,
    loaded: &mut HashSet<PathBuf>,
    paths: &mut Vec<PathBuf>,
) -> Result<Program, NativeError> {
    let path = path.canonicalize().map_err(NativeError::io)?;
    if !visiting.insert(path.clone()) {
        return Err(NativeError::diagnostics(
            path,
            String::new(),
            vec![Diagnostic {
                message: "circular native module import".into(),
                span: None,
            }],
        ));
    }
    let source = fs::read_to_string(&path).map_err(NativeError::io)?;
    let parsed = syntax::parse(&path, &source)
        .map_err(|e| NativeError::diagnostics(path.clone(), source.clone(), e))?;
    let mut structs = Vec::new();
    let mut functions = Vec::new();
    let mut imported_names = HashSet::new();
    for import in &parsed.imports {
        let imported_path = path
            .parent()
            .unwrap()
            .join(&import.path)
            .with_extension("xen");
        let dependency = load_module(&imported_path, false, visiting, loaded, paths)?;
        if let Some(namespace) = &import.namespace {
            for function in dependency.functions.iter().filter(|item| item.exported) {
                let mut function = function.clone();
                function.name = format!("{namespace}__{}", function.name);
                functions.push(function);
            }
            continue;
        }
        for name in &import.names {
            if !imported_names.insert(name.local.clone()) {
                return Err(NativeError::diagnostics(
                    path.clone(),
                    source.clone(),
                    vec![Diagnostic {
                        message: format!("ambiguous imported name `{}`", name.local),
                        span: None,
                    }],
                ));
            }
            let function = dependency
                .functions
                .iter()
                .find(|f| f.name == name.original && f.exported)
                .cloned();
            let structure = dependency
                .structs
                .iter()
                .find(|s| s.name == name.original && s.exported)
                .cloned();
            match (function, structure) {
                (Some(mut f), None) => {
                    f.name = name.local.clone();
                    functions.push(f)
                }
                (None, Some(mut s)) => {
                    s.name = name.local.clone();
                    structs.push(s)
                }
                _ => {
                    return Err(NativeError::diagnostics(
                        path.clone(),
                        source.clone(),
                        vec![Diagnostic {
                            message: format!(
                                "module `{}` does not export `{}`",
                                import.path, name.original
                            ),
                            span: None,
                        }],
                    ))
                }
            }
        }
    }
    if root {
        structs.extend(parsed.structs);
        functions.extend(parsed.functions);
    } else {
        structs.extend(parsed.structs.into_iter().filter(|item| item.exported));
        functions.extend(parsed.functions.into_iter().filter(|item| item.exported));
    }
    visiting.remove(&path);
    if loaded.insert(path.clone()) {
        paths.push(path);
    }
    Ok(Program {
        imports: vec![],
        structs,
        functions,
    })
}
impl Project {
    pub fn discover(start: &Path) -> Result<Self, NativeError> {
        let mut cursor = if start.is_file() {
            start.parent().unwrap_or(start)
        } else {
            start
        };
        loop {
            if cursor.join("xenith.toml").is_file() {
                return Ok(Self::at(cursor));
            }
            cursor = cursor.parent().ok_or(NativeError::ProjectNotFound)?;
        }
    }
    fn at(root: &Path) -> Self {
        Self {
            root: root.into(),
            manifest_path: root.join("xenith.toml"),
            main_path: root.join("src/main.xen"),
            target_dir: root.join("target"),
        }
    }
}
pub fn create_project(path: &Path, name: &str) -> Result<Project, NativeError> {
    if !valid_name(name) {
        return Err(NativeError::InvalidPackageName(name.into()));
    }
    let root = path.join(name);
    if root.exists() {
        return Err(NativeError::AlreadyExists(root.display().to_string()));
    }
    fs::create_dir_all(root.join("src")).map_err(NativeError::io)?;
    fs::write(
        root.join("xenith.toml"),
        format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2027\"\n"),
    )
    .map_err(NativeError::io)?;
    fs::write(
        root.join("src/main.xen"),
        "fn main() -> i32 {\n    return 0\n}\n",
    )
    .map_err(NativeError::io)?;
    Ok(Project::at(&root))
}
fn valid_name(name: &str) -> bool {
    let mut chars = name.chars();
    matches!(chars.next(), Some(first) if first.is_ascii_lowercase())
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}
