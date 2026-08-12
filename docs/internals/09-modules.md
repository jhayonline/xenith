# Modules

`src/modules.rs`, 179 lines. Finds module files, runs them, and caches the
result.

## The registry

```rust
pub struct ModuleRegistry {
    modules: HashMap<String, Module>,
    current_file: PathBuf,
}

pub struct Module {
    pub name: String,
    pub exports: HashMap<String, Value>,
    pub ast: Node,
}
```

The registry lives on the `Interpreter` as `module_registry: Option<...>`, built
lazily the first time a `grab` runs. `current_file` is what makes relative paths
work.

## Resolution

`ModuleRegistry::locate` answers where a module's source comes from.

A `std::` path is served from `src/stdlib/`, which is compiled into the binary
with `include_str!`:

```rust
pub fn source(name: &str) -> Option<&'static str> {
    match name {
        "string" => Some(include_str!("string.xen")),
        _ => None,
    }
}
```

The sources are ordinary `.xen` files sitting next to that `mod.rs`, so they are
readable and editable like any other Xenith code, and the binary carries them.
There is no install path to get wrong and no way for the library to fall out of
step with the interpreter shipping it. Adding a module is one line in that match
plus the file.

Everything else is a file on disk. `resolve_local` turns the path into one: `::`
becomes a directory separator and `.xen` is appended, so `geometry::shapes`
becomes `geometry/shapes.xen`. It tries the importing file's directory, then
that directory's parent.

Resolution failing becomes XEN012 at the `grab`.

## Loading

`Interpreter::load_module` does the work:

1. If the module is already in the cache, return it.
2. Read the file.
3. Lex and parse it.
4. Run [the static checker](06-checker.md) over it. A module gets the same
   checking as a file run directly.
5. Run its top level in a fresh context.
6. Collect its exports.
7. Cache and return.

A failure at any of steps 2 to 5 comes back as a `ModuleError`:

```rust
pub enum ModuleError {
    NotFound(String),
    Unreadable(String, String),
    Failed { module: String, errors: Vec<Error> },
    Circular(Vec<String>),
}
```

`Interpreter::module_failure` turns each into the error the user sees, with its
own code. `Failed` carries the module's own errors, so a type error inside an
imported file is reported as a type error at its own line in its own file, not
flattened into "module not found". The note says which module it came from, and
how many errors that module has if there is more than one.

This used to be a `Result<Module, String>` holding an already-rendered
diagnostic, which meant the caller had to search the text for the words
"circular import" to work out what had happened.

Because the module's top level runs, a module with statements outside a
definition performs them on import. Caching means that happens once no matter how
many files import it.

## Exports

`export` produces a `Node::Export` wrapping the definition. `visit_export`
evaluates the inner node and records the resulting value in `context.exports`.

Only methods and `let` bindings work. `export struct` does not parse, because
`export_statement` expects a definition it can evaluate to a value and a struct
definition evaluates to `Null`. Fixing it means teaching the module system to
carry struct definitions, not just values, and having the importing side register
them in `struct_names` and `struct_defs`.

## Importing

`visit_grab` handles both forms.

Named imports look each name up in `module.exports` and bind it in the current
scope, under its alias if `as` was used. A name that is not exported is XEN012.

`grab * as name` builds a `Value::Map` of every export and binds that, which is
why the namespace form is used with brackets: `stats["total"](xs)`.

## Scoping

A module's exported method can call that module's private helpers, because a
method captures the context it was written in and a module's top level runs in
its own context. Nothing has to be exported just to be reachable from inside the
module.

That was not true while name resolution was dynamic: the body ran against the
importing file's scope, and the module's own unexported names were unreachable.

## What is missing

- No versioning or namespacing beyond the file path.
- A `grab` from `std::` parses, checks and runs that module on every program
  start, roughly ten milliseconds for `std::string`. It is per module and per
  run. Splitting the library into smaller modules would mean paying only for
  what a program actually uses.

Next: [Performance](10-performance.md)
