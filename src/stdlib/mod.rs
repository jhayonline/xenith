//! # The Standard Library
//!
//! Modules written in Xenith and built into the binary.
//!
//! The sources sit next to this file as ordinary `.xen` files, so they can be
//! read and edited like any other Xenith code, and `include_str!` folds them
//! into the binary at compile time. That means there is no install path to get
//! wrong, no way for the library to fall out of step with the interpreter that
//! ships it, and nothing to find at run time.
//!
//! The previous arrangement searched four filesystem locations for a `stdlib`
//! directory, which is the kind of thing that works on the author's machine.
//!
//! Everything here is written in the language on purpose. A standard library is
//! the largest Xenith program there is, and what it finds awkward is what needs
//! fixing in the language.

/// The source of a `std::` module, or `None` if there is no such module.
pub fn source(name: &str) -> Option<&'static str> {
    match name {
        "string" => Some(include_str!("string.xen")),
        "math" => Some(include_str!("math.xen")),
        "fs" => Some(include_str!("fs.xen")),
        "bytes" => Some(include_str!("bytes.xen")),
        "env" => Some(include_str!("env.xen")),
        "json" => Some(include_str!("json.xen")),
        _ => None,
    }
}

/// Every module name the standard library provides, for error messages.
pub const MODULE_NAMES: &[&str] = &["string", "math", "fs", "bytes", "env", "json"];
