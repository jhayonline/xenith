# Contributing

## Building

```sh
cargo check --all-targets    # while iterating, a second or two
cargo build --release        # to measure or install, a couple of minutes
cargo install --path .       # into ~/.cargo/bin
```

The release profile uses full LTO with one codegen unit, which is why it is slow.
Use `cargo check` for everything except benchmarking.

## Testing

```sh
cargo test
```

`tests/run.rs` drives everything from files, so adding a test means adding a
fixture rather than writing Rust.

| Directory | Holds | Checked against |
| --- | --- | --- |
| `tests/cases/` | `name.xen` with `name.out` | exit 0 and byte identical stdout |
| `tests/errors/` | `name.xen` with `name.err` | non zero exit and that error code in stderr |
| `tests/modules/` | `main.xen` and the module it imports | exit 0 and its `.out` |
| `testies/` | the older samples, with no expected output | that they still run at all |

### Adding a case

Write the program, run it, read the output carefully, and save it:

```sh
xenith tests/cases/my_case.xen          # read this and check it is right
xenith tests/cases/my_case.xen > tests/cases/my_case.out
```

Recording output you have not read defeats the point. The golden file is the
assertion.

### Adding an error case

The `.err` file holds one error code:

```sh
echo 'let n: int = "five"' > tests/errors/my_error.xen
echo 'XEN001' > tests/errors/my_error.err
```

Only the code is matched, not the wording, so improving a message does not break
the test. A `.xen` with no `.err` beside it is treated as a support file for
another case and is not run on its own, which is how the circular import case
gets its two extra modules.

### Known failures

`samples_still_run` carries a short list of `testies/` samples that are expected
to fail, each with the reason. If one starts passing the test says so, so the
list cannot rot. There is one entry today: `backtick_strings.xen`, which needs
`format` to work as an expression.

## Adding a builtin function

Two edits.

`src/builtins/registry.rs`:

```rust
BuiltinFn {
    name: "reverse",
    signature: "reverse(list) -> list",
    doc: "Returns a new list with the elements in the opposite order.",
},
```

`src/values.rs`, in `BuiltInFunction::execute`:

```rust
"reverse" => self.reverse(args, call_pos),
```

The registry entry alone registers the name and then fails at the call, so both
are needed. The language server picks up the new entry with no further work.

## Adding a keyword

1. Add it to `KEYWORDS` in `src/tokens.rs`.
2. Parse it in `src/parser.rs`.
3. Add the node and the visitor, as below.
4. Add it to the right group in `src/builtins/registry.rs`, so completion offers
   it, and write a line for it in `keyword_doc` so hover explains it.
5. Add it to `editors/nvim/syntax/xenith.vim`.

Step 4 and 5 are the ones the compiler will not remind you about.

Think twice before adding one. The keyword list is short on purpose, and a
language with thirty keywords can be learned in an afternoon in a way that one
with sixty cannot.

## Adding a node type

1. Add the variant to `Node` in `src/nodes.rs` and the struct beside it, ending
   with `position_start` and `position_end`.
2. Add arms to `Node::position_start()` and `Node::position_end()`.
3. Parse it.
4. Add `visit_*` in `src/interpreter.rs` and an arm to `visit`.
5. If it introduces a name, handle it in `Collector::walk` in
   `src/bin/xenith-lsp.rs`.

The compiler forces 2 and 4. Nothing forces 5, and skipping it means the editor
goes quietly blind to the new construct.

## Adding an error

Prefer a constructor in `src/error.rs` over a bare `RuntimeError`, so the error
gets a code, a note and a help line, and so its wording matches the others.

```rust
pub fn my_error(detail: &str, pos_start: Position, pos_end: Position) -> Self {
    Self::new(pos_start, pos_end, "My Error", detail)
        .with_code("XEN022")
        .with_note("why this rule exists")
        .with_help("what to do instead")
}
```

Take the next free code. Do not reuse one, even for something that feels related.
Codes are what users search for.

Document it in `docs/tutorial/16-errors.md`.

## Style

Follow what is there. Some things worth knowing:

**Comments say why, not what.** `// increment i` is noise. A comment earning its
place explains a decision or warns about a trap:

```rust
// Shared, not owned. Symbol table reads clone the Value, so a `Box` here
// meant every reference to a function deep copied its entire body.
pub body_node: Rc<Node>,
```

**Leave the reason behind a fix in the code.** Most of the subtle bugs in this
project were reintroductions of an earlier bug. A comment at the site is what
stops that.

**Diagnostics are the user interface.** An error with no `help` is half finished.

## The big outstanding work

In the order it should probably be done:

**1. A static checking pass.** A new module between parse and interpret that
walks the tree, infers types for un-annotated declarations, validates annotated
ones, checks calls, struct literals, index and field access, and reports every
error before anything runs. This makes the language's central claim actually
true, and it lets the language server report type errors. Once it exists, the
runtime checks scattered through the visitors can come out of the hot path.

**2. A resolver pass.** Give every variable reference a scope depth and slot
index, so lookup is a vector index rather than a hashed walk. It shares most of
its machinery with the checker, so build them together. This also gives the
language server real scope information, which is what its rename needs.

**3. Lexical scoping.** Record the defining context in `Function` and use it in
`execute` instead of the caller's. That gives the language closures and lets a
module's exports call its private helpers. It changes what existing programs do,
so it wants a decision rather than a patch.

**4. A standard library.** Strings first, since they are the most obviously
missing. Written in Xenith where possible, so the language gets exercised, with
Rust builtins only where it has to be.

## Smaller things worth doing

- Move `format` out of `KEYWORDS` so it can be used as an expression.
- Parse interpolated expressions at parse time rather than re-parsing them on
  every evaluation.
- Make `export struct` work.
- Give the parser error recovery, so a file yields more than one diagnostic.
- Delete `resolve_stdlib` and the duplicated third candidate in `resolve_local`
  in `src/modules.rs`.
- Replace the `Result<Module, String>` module errors with a real error type, so a
  nested failure does not arrive as a pre-rendered string.
- Store the two variable `for k, v` names as a proper field instead of the
  literal text `(k,v)` in one token.

Back to [the internals index](README.md)
