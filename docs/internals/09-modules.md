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

`resolve_path` turns a module path into a file. `::` becomes a directory
separator and `.xen` is appended, so `geometry::shapes` becomes
`geometry/shapes.xen`.

`resolve_local` tries three locations in order:

1. Relative to the importing file's directory.
2. Relative to that directory's parent.
3. The same as 1 again.

The third is a duplicate of the first and does nothing. Worth removing along with
`resolve_stdlib`, which still looks for a `stdlib/` directory that no longer
exists.

Resolution stops at the first file that is present, and `None` becomes XEN012 at
the `grab`.

## Loading

`Interpreter::load_module` does the work:

1. If the module is already in the cache, return it.
2. Read the file.
3. Lex and parse it. A syntax error in the module surfaces at the `grab`.
4. Run its top level in a fresh context.
5. Collect its exports.
6. Cache and return.

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

## The scoping problem

A module's exported method cannot call that module's private helpers.

The reason is in [The interpreter](05-interpreter.md): a method body evaluates in
a child of the *caller's* context. When another file calls `double_all`, the
context chain runs back to that file's scope, not to the module's. The module's
unexported names were bound in a context that is no longer reachable.

The fix is the same one that would give the language closures: record the
defining context in `Function` and use it in `execute`. Until then, a module's
exports have to stand alone.

## What is missing

- No circular import detection. Two modules importing each other will recurse
  until the depth limit.
- No versioning or namespacing beyond the file path.
- `resolve_stdlib` still exists and searches for a directory that was deleted.
- The duplicated third candidate in `resolve_local`.

Next: [Performance](10-performance.md)
