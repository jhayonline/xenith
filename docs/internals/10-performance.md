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

Four more passes took it the rest of the way to **16 bytes**, and
`RuntimeResult` to **48**:

- `Bytes` held a `Vec<u8>` inline, 24 bytes. Behind an `Rc`.
- `Tuple` held a `Vec<Value>` inline, 24 bytes, copied in full on every clone.
  Behind an `Rc`, like `List`. Nothing mutates a tuple in place, so there is no
  copy-on-write to do.
- `BuiltInFunction` held a `String` to name one of about thirty fixed builtins,
  which made it 24 bytes and allocated on every clone. It holds a `u16` index
  into `BUILTIN_FUNCTIONS`, so dispatch compares a `&'static str` rather than an
  owned one.
- `Number` wrapped an `i64` or an `f64` behind a second tag, 16 bytes to carry
  8. A `Value` now stores `Int(i64)` and `Float(f64)` directly.

```rust
pub enum Value {
    Int(i64),
    Float(f64),
    String(Rc<XenithString>),
    Bytes(Rc<Bytes>),             // Vec<u8>, 24 bytes inline
    List(List),
    Function(Box<Function>),      // 56 bytes inline
    BuiltInFunction(BuiltInFunction),
    Map(Box<Map>),                // 48 bytes inline
    Struct(Box<Struct>),          // 72 bytes inline
    Bool(bool),
    Tuple(Rc<Vec<Value>>),        // Vec<Value>, 24 bytes inline
    Enum(Box<EnumValue>),         // 72 bytes inline
    Null,
}
```

Nothing wider than a word is left inline, so the discriminant fits in a niche
and the enum costs no more than its largest member. Measured on a three million
iteration counting loop, the four passes together were worth **15%** (0.97 s to
0.82 s) before any other change.

The rule: before adding a variant, check whether it is larger than the current
biggest. If it is, box it. `tests/layout.rs` enforces it -- a size change there
is a decision, not an accident.

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

That check is not paranoia. The same node can still run against different chains:
a closure produced twice from the same method has a different captured scope each
time, and a method defined inside a block sees a scope that is rebuilt on every
pass.

```xenith
method make_adder(n: int) -> IntFn {
    release method(x: int) -> int => x + n
}
let add_ten: IntFn = make_adder(10)
let add_one: IntFn = make_adder(1)
```

Both closures share one `x + n` node, against two different scopes. The name
check is what keeps that honest.

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

## Collections were quadratic to build

Everything above is about constant factors. This one was a complexity class, and
it went unnoticed because the programs that had been measured — counting loops,
`fib` — never build a collection.

`xs.append(x)` copied the whole list. Four times, in fact: reading `xs` out of
the symbol table cloned it, `visit_call` cloned it again for `call_method`,
`call_method` cloned the result to return it, and the write back stored it. So
filling a list was O(n²), and the same for `m[key] = value`. Writing `std::json`
made it obvious: parsing 6KB took half a second, and 52KB took nine.

The fix has two halves, and neither works without the other.

**Share the storage.** `List`, `Map` and `String` hold their contents behind an
`Rc`, so a clone is a refcount bump. Writes go through `Rc::make_mut`, which
copies only when the data is genuinely shared — value semantics preserved
exactly. See [Values](07-values.md#copied-but-not-deep-copied).

That alone took 8000 appends from 3.99s to 1.59s and left it quadratic, which is
the trap worth remembering: **copy-on-write does nothing if the mutation always
finds the data shared.** While `append` was changing the list, the symbol table
still held it, so `make_mut` copied every time.

**Take the value out of the binding.** `SymbolTable::take` lifts the value out
and leaves `Null`, so the mutation has the only reference and `make_mut` has
nothing to do. `visit_call` does this for `append`, `pop` and `remove` on a plain
variable; `assign_into` does it for `m[key] = value`.

| | before | Rc only | Rc + take |
| --- | --- | --- | --- |
| 8000 × `xs.append(i)` | 3.99s | 1.59s | 0.02s |
| 8000 × `m[key] = i` | 8.70s | 9.14s | 0.03s |

The ordering trap: both must evaluate the arguments and the index *before*
lifting the receiver, or `xs.append(xs.len())` reads the hole. That is a test,
not a comment.

## Strings were quadratic to scan

Found in the same measurement, and worth separating because it is a different
mistake. Xenith counts and indexes strings by character; a `String` is UTF-8.
So `text.len()` was `chars().count()`, an O(n) walk — and every scanner in the
standard library is written `while i < text.len()`. `text[i]` was
`chars().nth(i)`, O(i). `substring` collected the entire string into a
`Vec<char>` before taking a slice of it.

`XenithString` now carries its character count and an all-ASCII flag, both
settled once at construction. On ASCII, indexing and `substring` are byte
ranges.

Together with the collection fix, parsing 52KB of JSON went from 9.06s to 1.39s,
and — the point — from quadratic to linear:

| Document | before | after |
| --- | --- | --- |
| 6.4KB | 0.55s | 0.42s |
| 12.8KB | 1.06s | 0.55s |
| 25.8KB | 2.78s | 0.83s |
| 51.9KB | 9.06s | 1.39s |

A fixed 0.27s of each of those is parsing and checking `std::json` itself on
import. Subtract it and the four are 0.16, 0.29, 0.56, 1.12 — doubling with the
input, which is what was wanted.

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
   The large cases — strings, lists and maps — are now shared rather than
   copied, so the clone is a refcount bump; what is left is the bump itself and
   the same problem for `Struct`, `Enum` payloads and `Tuple`, which are still
   copied in full.
2. **Intern identifiers.** Every identifier allocates a `String` in the lexer and
   is compared by content thereafter. Names in a scope are already `Rc<str>`;
   sharing those with the tokens would let comparison be a pointer check, which
   would also speed up the slot cache's name verification.
3. **Flatten the dispatch.** `visit` is a match over 31 variants called for every
   node. Splitting the hot arms out, or ordering them by frequency, is worth
   measuring.

## The bytecode VM, phase 3

The counting loop in `benches/counting_loop.xen`, 400,000 iterations, measured
by callgrind instruction count on one machine and one build:

| | I refs | |
| --- | --- | --- |
| tree walker | 1,181,300,349 | |
| bytecode VM | 265,650,817 | 4.45x |

Where the 4.45 comes from is worth writing down, because the first VM to run
this benchmark managed only 2.30x and the gap was two things, neither of them
the design:

| | loop body | I refs | |
| --- | --- | --- | --- |
| three-address code, naively emitted | 13 instructions | 513,253,740 | 2.30x |
| operands borrowed rather than cloned | 13 instructions | | |
| a local named directly as an operand | 9 instructions | 265,650,817 | 4.45x |

The first: `binary` in `src/vm/run.rs` cloned both operands out of the
register file before applying the operation, so every `ADD` of two ints ran
`Value::clone`'s match over thirteen variants twice. Two immutable borrows out
of one slice are fine as long as the write happens after both have ended,
which it does.

The second: reading a local used to copy it into a temporary, so
`total = total + i` spent three of its five instructions moving values that
were already where they needed to be. An instruction reads its operand
registers before it writes its destination, and a destination is always
strictly above every live local -- `Compiler::operand` explains why -- so the
copy is not needed when the operand is only read.

Two `MOVE`s per iteration are left, both writing an assignment's result back
into its local. Removing them needs the destination threaded down into the
expression, which is only safe where the value emits no jumps: a `when` used
as a value emits one `MOVE` per branch, and pointing the last of them at the
local would leave the other branches writing somewhere else.

Phase 3 compiles a deliberately small slice of the language -- literals,
operators, locals, `when`, `while`, the classic `for`, and `echo`. Anything
else returns `Unsupported` and runs on the tree walker, which is why
`tests/differential.rs` reports a skipped count alongside a compared one. That
count is currently 2 compared and 117 skipped, and it is the number to watch:
it should climb every phase, and a drop means something stopped compiling that
used to.

The measurement to distrust here is wall clock. Two of the false regressions
in the history above were found that way, on a loaded machine.

Next: [The language server](11-language-server.md)
