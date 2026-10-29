pub mod cache;
pub mod check;
pub mod codegen;
pub mod error;
pub mod project;
pub mod syntax;
pub use error::NativeError;
use std::{fs, path::PathBuf, process::Command};
pub struct BuildArtifact {
    pub executable: PathBuf,
    pub reused: bool,
}
pub fn check_project(p: &project::Project) -> Result<check::TypedProgram, NativeError> {
    let s = fs::read_to_string(&p.main_path)
        .map_err(|_| NativeError::NotImplemented("missing src/main.xen".into()))?;
    let a = syntax::parse(&p.main_path, &s)
        .map_err(|e| NativeError::NotImplemented(e[0].message.clone()))?;
    check::check(&a).map_err(|e| NativeError::NotImplemented(e[0].message.clone()))
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
