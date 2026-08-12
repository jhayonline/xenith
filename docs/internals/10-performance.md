# Performance

A tree walking interpreter is never going to win a benchmark. The aim here is
that nothing be gratuitously slow, and in particular that nothing in the hot path
be doing work that is invisible from the source.

## Where it stands

Measured on the machine this was written on, release build, no other load:

| Program | Time |
| --- | --- |
| `fib(25)`, naive recursion | about 355 ms |
| Counting loop, 3,000,000 iterations | about 1.8 s |

Take these as a shape rather than a number. Measure on your own machine before
and after any change you make.

Getting there took four fixes, none of which were where profiling was expected to
point. Each is a trap that is easy to reintroduce.

## Do not clone the function body

`Function` held its body as `Box<Node>`:

```rust
pub struct Function {
    pub body_node: Box<Node>,   // this was the problem
    ...
}
```

`SymbolTable::get` returns values by clone. Every reference to a function
therefore deep copied its entire body tree, and a recursive program spent most of
its time in `malloc` and `free`.

```rust
pub struct Function {
    /// Shared, not owned. Symbol table reads clone the Value, so a `Box` here
    /// meant every reference to a function deep copied its entire body.
    pub body_node: Rc<Node>,
    pub arg_names: Rc<Vec<String>>,
    pub param_types: Rc<Vec<Type>>,
    ...
}
```

This was the largest single win by a wide margin. The rule it implies: anything
reachable from a `Value` should be cheap to clone, because values are cloned
constantly.

## Do not copy the source text

`Position` owned its file name and file text as `String`:

```rust
pub struct Position {
    pub file_name: String,   // and this
    pub file_text: String,   // and especially this
    ...
}
```

Every AST node carries two positions, and nodes are cloned all through
evaluation. Each clone was copying the whole source file.

```rust
pub struct Position {
    pub index: usize,
    pub line: usize,
    pub column: usize,
    pub file_name: Arc<str>,
    pub file_text: Arc<str>,
}
```

`Arc` rather than `Rc` because the language server shares parsed trees across
threads. `Position::with_source` shares the existing handles and never
allocates; prefer it in the lexer and parser.

## Keep RuntimeResult small

`RuntimeResult` is returned by value from every visit, so its size is multiplied
by every node evaluated. Boxing the error took it from 392 bytes to 160:

```rust
pub struct RuntimeResult {
    pub value: Option<Value>,
    pub error: Option<Box<Error>>,   // Box matters here
    ...
}
```

`Error` is eight fields including three `String`s and two `Position`s. Inline, it
dominated the struct, and errors are the rare case.

## Do not walk the scope chain more than once

`Context::parent` was `Option<Box<Context>>`, so creating a child scope deep
copied the entire chain, making calls quadratic in depth. It is now
`Option<Rc<Context>>`.

Separately, assignment used to ask four questions in four walks: is it declared,
is it constant, what type was it declared as, and then the write.
`resolve_for_assign` answers the first three in one walk:

```rust
pub struct BindingInfo {
    pub is_constant: bool,
    pub declared_type: Option<Type>,
}

pub fn resolve_for_assign(&self, name: &str) -> Option<BindingInfo>
```

That change alone took a three million iteration loop from 2.5 s to 1.7 s.

## Hashing

Symbol tables use `FxHashMap` from `src/fxhash.rs`, a small non cryptographic
hasher of the kind rustc uses. Variable lookup is hot enough that SipHash, the
standard library default, showed up in profiles. The implementation is
self contained, so it costs no dependency.

Hash lookups are still roughly a tenth of the time in a tight loop. Removing them
means resolving each variable reference to a slot index at parse time, so a
lookup becomes a vector index. That needs a resolver pass, which is the same
thing the static type checker needs.

## Loop scopes are reused

A loop body gets one child context, cleared each iteration rather than
reallocated:

```rust
let mut body_ctx = loop_ctx.create_child("<for body>", position);

loop {
    body_ctx.symbol_table.clear_local();
    result.register(self.visit(&node.body_node, &mut body_ctx));
    ...
}
```

`clear_local` empties this scope's maps and leaves the parents alone. Without it
a three million iteration loop allocates three million symbol tables.

## The release profile

```toml
[profile.release]
opt-level = 3
lto = true
codegen-units = 1
panic = "abort"
```

Full LTO with a single codegen unit makes a noticeable difference for an
interpreter, where the hot path crosses module boundaries constantly. It also
makes release builds slow, so use `cargo check` while iterating and build release
only to measure.

## How to measure

Beware background load. Building with LTO saturates the machine, and timings
taken while a build is running can be three times too slow. Wait for the build to
finish before measuring.

For wall clock:

```sh
cargo build --release
for i in 1 2 3; do
    /usr/bin/env time -f '%e s' ./target/release/xenith bench.xen
done
```

For where the time goes, callgrind reads well on this codebase:

```sh
valgrind --tool=callgrind ./target/release/xenith bench.xen
callgrind_annotate callgrind.out.*
```

Add `debug = true` to the release profile first so the output has symbols, and
take it out again afterwards.

## Parse interpolated expressions once

The text inside every `{}` used to be re-lexed and re-parsed each time the string
was evaluated, so a loop printing one paid for a parse per iteration.
`InterpolationPart` now carries the node, parsed at parse time.

Measured A/B on a 300,000 iteration loop printing an interpolated string:

| | Time |
| --- | --- |
| re-parsing each evaluation | 1.8 to 2.5 s |
| parsed once | 0.37 to 0.49 s |

About five times faster, and it is what lets the static checker see inside a
string at all.

## What is left

In rough order of expected return:

1. **Resolve variables to slots.** Removes hashing from the hot path. Needs a
   resolver pass, which shares most of its machinery with the checker.
2. **Intern identifiers.** Every identifier allocates a `String` in the lexer and
   is compared by content thereafter.
3. **Avoid cloning values out of the symbol table.** The largest remaining
   structural cost, and the hardest to change, since the whole interpreter
   assumes values are owned.

Next: [The language server](11-language-server.md)
