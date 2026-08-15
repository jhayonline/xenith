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
| | | each runs in a scratch directory of its own, so a fixture may write files |
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

`samples_still_run` carries a list of `testies/` samples that are expected to
fail, each with the reason. If one starts passing the test says so, so the list
cannot quietly rot. It is empty today.

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

## Adding a standard library module

Write it as a `.xen` file in `src/stdlib/`, then add one line to `source` in
`src/stdlib/mod.rs` and one to `MODULE_NAMES`. `include_str!` picks it up at
compile time, so there is nothing to install and nothing to find at run time.

Write it in Xenith. Reach for a Rust builtin only when the language genuinely
cannot express the thing, not when Rust would be faster; that question gets asked
later, with measurements, and moving a function into the interpreter does not
change its signature.

When a primitive is needed, its name says what kind of thing it is. An operation
on a built in type is global under its own name, like `substring` or `sin`. A
service is prefixed and wrapped by the module, like the `fs_` family: reading a
file is something a program asks the world to do, and that should be visible in
its imports.

The library is checked by the same static pass as any other module, so a type
error in it fails at the `grab` rather than silently.

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
        .with_code("XEN024")
        .with_note("why this rule exists")
        .with_help("what to do instead")
}
```

Take the next free code. Do not reuse one, even for something that feels related.
Codes are what users search for.

Document it in `docs/tutorial/17-errors.md`.

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

**1. A standard library.** Strings first, since they are the most obviously
missing. Written in Xenith where possible, so the language gets exercised, with
Rust builtins only where it has to be.

## Smaller things worth doing

- Teach the static pass to follow `grab`, so an imported method call and an
  imported struct literal are checked before the program runs rather than as
  they run. Everything needed is already on the `Module`.
- Give the parser error recovery, so a file yields more than one diagnostic.
- Delete `resolve_stdlib` and the duplicated third candidate in `resolve_local`
  in `src/modules.rs`.
- Allow an expression to span lines, so a long condition does not have to be one
  line. The lexer ends a statement at a newline with no continuation rule.
- Add hex and binary integer literals. `0xff` does not parse, which is felt most
  when working with `bytes`.
- Store the two variable `for k, v` names as a proper field instead of the
  literal text `(k,v)` in one token.
- Give the language server real scope information. Its rename is still by name
  and file local. A resolver pass would fix that, and would share machinery with
  [the checker](06-checker.md).

Back to [the internals index](README.md)
