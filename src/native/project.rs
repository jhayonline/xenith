use super::NativeError;
use std::{
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
