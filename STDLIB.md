# Xenith Standard Library Architecture

## Overview

Xenith's standard library is written in **Rust** for performance and stability, with **thin Xenith wrappers** providing the user-facing API. This gives us the best of both worlds: Rust's speed and ecosystem + Xenith's syntax and safety.

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                     User Xenith Code                         │
│  grab { read, write } from "std::fs"                        │
│  spawn content: string = read("file.txt")                   │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                  stdlib/fs.xen (Wrapper)                     │
│  export method read(path: string) -> string {               │
│      release __fs_read(path)  ← Calls Rust built-in         │
│  }                                                          │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│           BuiltInFunction("__fs_read")                       │
│         (Registered in interpreter.rs)                       │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                 src/builtins/fs.rs                           │
│  pub fn read(args: Vec<Value>) -> RuntimeResult {           │
│      let path = get_string_arg(args[0]);                    │
│      let content = std::fs::read_to_string(path)?;          │
│      Ok(Value::String(content))                             │
│  }                                                          │
└─────────────────────────────────────────────────────────────┘
```

## Why This Approach?

| Reason | Explanation |
|--------|-------------|
| **Performance** | Rust's file I/O, JSON parsing, and HTTP clients are blazing fast |
| **Stability** | Battle-tested crates vs. reinventing the wheel |
| **Ecosystem** | Access to `serde_json`, `reqwest`, `rand`, etc. |
| **No Bootstrap Problem** | Works immediately without Xenith being self-hosting |
| **Consistent** | Same pattern as built-ins like `echo()`, `input()` |

## How It Works

### 1. Rust Built-in (src/builtins/fs.rs)
Each operation is a Rust function that:
- Takes `Vec<Value>` arguments
- Returns `RuntimeResult`
- Uses standard Rust crates for actual work
- Converts errors to Xenith `RuntimeError`

### 2. Registration (src/interpreter.rs)
Built-ins are added to the global symbol table with `__` prefix:
```rust
global.set("__fs_read".to_string(), 
    Value::BuiltInFunction(BuiltInFunction::new("__fs_read")));
```

### 3. Dispatch (src/values.rs)
The `BuiltInFunction::execute` method routes calls:
```rust
match self.name.as_str() {
    "__fs_read" => crate::builtins::fs::read(args),
    // ...
}
```

### 4. Xenith Wrapper (stdlib/fs.xen)
Clean user-facing API that calls the prefixed built-in:
```xenith
export method read(path: string) -> string {
    release __fs_read(path)
}
```

## Adding a New Built-in Function

1. **Add Rust implementation** in `src/builtins/module_name.rs`
2. **Register** in `src/interpreter.rs` (add to global symbol table)
3. **Add dispatch** in `src/values.rs` (match arm)
4. **Export wrapper** in `stdlib/module_name.xen`

## Module Resolution

- `std::fs` → looks for `stdlib/fs.xen`
- `std::fs::path` → looks for `stdlib/fs/path.xen`
- Falls back to multiple locations (project root, executable directory)

## Files Structure

```
src/
├── builtins/
│   ├── mod.rs
│   └── fs.rs          # Rust implementation
├── interpreter.rs      # Registration
├── values.rs          # Dispatch
└── modules.rs         # Module resolution

stdlib/
├── fs.xen             # Xenith wrapper
├── path.xen
└── ...
```

## Key Principle

> **Rust does the heavy lifting. Xenith provides the beautiful syntax.**
```
