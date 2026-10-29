use std::{fmt, io};
#[derive(Debug)]
pub enum NativeError {
    Usage(String),
    InvalidPackageName(String),
    ProjectNotFound,
    AlreadyExists(String),
    Io(io::Error),
    NotImplemented(String),
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
        }
    }
}
impl std::error::Error for NativeError {}
