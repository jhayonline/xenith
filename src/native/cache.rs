use super::NativeError;
use std::{
    collections::hash_map::DefaultHasher,
    fs,
    hash::{Hash, Hasher},
    path::Path,
};
pub fn fingerprint(a: &Path, b: &Path) -> Result<String, NativeError> {
    let mut h = DefaultHasher::new();
    fs::read(a).map_err(NativeError::io)?.hash(&mut h);
    fs::read(b).map_err(NativeError::io)?.hash(&mut h);
    Ok(format!("{:x}", h.finish()))
}
