# Performance

A tree walking interpreter is never going to win a benchmark. The aim here is
that nothing be gratuitously slow, and in particular that nothing in the hot path
be doing work that is invisible from the source.

## Where it stands

The figure to compare against is instruction count, because it does not care what
else the machine is doing. On a 400,000 iteration counting loop:

| | Instructions |
| --- | --- |
| before any of the work below | 2,345M |
| after | 1,192M |

For a sense of wall clock on an idle machine, naive `fib(25)` runs in roughly a
third of a second and a three million iteration counting loop in under two. Treat
those as a shape rather than a number, and see [How to
measure](#how-to-measure) before quoting one.

Getting there took nine fixes, almost none of which were where profiling was
expected to point. Each is a trap that is easy to reintroduce, which is why they
are all written down.

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

Hashing is now off the hot path entirely, because each reference remembers where
its name lives; see below. The map is still what a first lookup and every
declaration go through.

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

## Assignment: one walk, no allocation

`x = x + 1` is the hottest statement in most programs, and it used to do a
surprising amount of work:

- `resolve_for_assign` walked the scope chain and **cloned the declared type**,
  purely to compare it and throw it away.
- `assign_existing` then walked the chain a **second time**.
- Storing did `contains_key` then `insert`, hashing the name twice and
  allocating a fresh `String` key **on every assignment**.

`SymbolTable::assign_checked` does the whole thing in one walk: one hash lookup
held as a mutable slot, the type compared behind the borrow without cloning, and
the value moved in rather than copied. The declared type is cloned only to build
an error message.

Separately, `value_matches_type` called `resolve_type_alias`, which rebuilds the
type and allocates for anything compound. Most annotations contain no alias at
all, so it now checks first with `contains_alias`.

## Value was too big to move cheaply

`Value` was **72 bytes**, because `Struct { name: String, fields: HashMap }` is
exactly that inline, and every `Value` was sized for the largest variant whether
or not it was a struct. `Map` was 48 and `Function` 56.

Boxing those three took `Value` to **32 bytes** and `RuntimeResult`, which
contains one, from 160 to **80**. Both are moved constantly: out of the symbol
table on every read, and back out of every single `visit`.

```rust
pub enum Value {
    Number(Number),
    String(XenithString),
    List(List),
    Function(Box<Function>),      // 56 bytes inline
    BuiltInFunction(BuiltInFunction),
    Map(Box<Map>),                // 48 bytes inline
    Struct(Box<Struct>),          // 72 bytes inline
    Bool(bool),
    Tuple(Vec<Value>),
    Null,
}
```

The rule: before adding a variant, check whether it is larger than the current
biggest. If it is, box it.

## Bindings live in a Vec, and each reference remembers where

Reading a variable meant walking out through the scope chain, hashing the name in
each scope until one had it. `SymbolTable::get` was around 12% of a counting
loop, and the `Value` it cloned another 13%.

Two changes together removed it from the profile entirely.

**Storage.** A scope now holds `Vec<Entry>` with a name-to-position map beside
it, where an `Entry` carries the value, its declared type and its constness
together. Reading by position is a vector index. An assignment touches one
structure instead of the three separate maps it used to.

Nothing is ever removed from a table, only overwritten or cleared wholesale,
which is what makes a position stable for the life of a scope.

**Remembering.** Every `VarAccessNode` and `VarAssignNode` carries a `SlotCache`:
how many scopes out the name was found last time, and at which position. The
next execution of that line goes straight there.

```rust
pub struct SlotCache {
    pub hops: u16,
    pub slot: u32,
    pub valid: bool,
}
```

**The cache is never trusted blindly.** `get_slot` compares the name at that
position before returning the value, and `assign_slot` hands the value back
untouched if it does not match. A stale entry costs a lookup, never a wrong
answer.

That check is not paranoia. Xenith resolves names against the caller's scope, so
one method body can run against completely different chains:

```xenith
method show() -> null {
    echo("{n}")
    release null
}
method first() -> null  { let n: int = 111        show() release null }
method second() -> null { let n: string = "text"  show() release null }
```

The same `echo` node reads an `int`, then a `string`, then an `int` again. The
name check makes it miss and re-resolve each time.

This is why it is a cache filled at run time rather than a resolver pass filling
in positions ahead of time. A static pass would have to predict the exact shape
and insertion order of every scope, including the builtins seeded into the global
one, and any disagreement with the interpreter would shift every position after
it. `tests/cases/scope_resolution.xen` covers the cases that would break a naive
version.

## Loops were building a list nobody could read

`WhileNode` and `ForNode` carry `should_return_null`, and the parser set it to
`false` for both, so every loop collected the value of every iteration into a
`Vec` and returned it as the loop's value.

Nothing in the language can read that value. There is no loop-as-expression form.
A two million iteration `while` loop was therefore building a two million element
list and discarding it:

| | Peak RSS |
| --- | --- |
| collecting | 370 MB |
| not collecting | 3.7 MB |

Memory now stays flat however long a loop runs. If a loop-expression form is ever
added, this is the flag to turn back on, and only for that form.

## Measuring under load

Wall clock on a busy machine is worthless. Timings taken while a build or
anything else was running came out three to five times too slow, and twice in
this project's history that noise was mistaken for a real regression.

Instruction counts do not care what else is running:

```sh
valgrind --tool=callgrind --callgrind-out-file=/tmp/cg.out ./target/release/xenith bench.xen
```

The `I refs` total it prints is the number to compare. The changes above took a
400,000 iteration counting loop from **2,345M to 1,192M instructions, 49%
fewer**, measured that way.

For attribution rather than a total, add `debug = true` to the release profile,
rerun, and `callgrind_annotate /tmp/cg.out`. Take the `debug` line back out
afterwards.

## What is left

The profile after all of the above, on a 400,000 iteration counting loop:

| Share | |
| --- | --- |
| 36% | `visit`, the dispatch itself |
| 21% | `visit_binary_op` |
| 10% | `visit_var_assign` |
| 8% | dropping `Value`s |
| 4% | cloning `Value`s |

Variable lookup no longer appears at all. What is left is the shape of a tree
walking interpreter: the cost is in walking the tree and in moving values
around.

In rough order of expected return:

1. **Avoid cloning values out of the symbol table.** Reading a variable still
   copies its value, because the whole interpreter assumes values are owned.
   Returning a reference, or making large values shared rather than copied,
   is the biggest structural change left and the one with the most in it.
2. **Intern identifiers.** Every identifier allocates a `String` in the lexer and
   is compared by content thereafter. Names in a scope are already `Rc<str>`;
   sharing those with the tokens would let comparison be a pointer check, which
   would also speed up the slot cache's name verification.
3. **Flatten the dispatch.** `visit` is a match over 31 variants called for every
   node. Splitting the hot arms out, or ordering them by frequency, is worth
   measuring.

Next: [The language server](11-language-server.md)
