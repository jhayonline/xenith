# RESUME

Working notes from a design-and-build session on making Xenith a language you
can write a backend in. Written to be picked up cold.

The code is in the repo and speaks for itself. What is *not* in the repo is the
reasoning behind the ordering, the options that were rejected and why, and the
decisions still open. That is most of what is below.

---

## 1. Pick up here

**Uncommitted:** `std::json` — `src/stdlib/json.xen`, its line in
`src/stdlib/mod.rs`, three fixtures (`tests/cases/json_read`, `json_write`,
`json_malformed`), the `std::json` section of
`docs/tutorial/20-standard-library.md`, and the module lists in
`19-limitations.md` and `tutorial/README.md`. `cargo test` is green.

```
feat: std::json
```

The string-lexer fix landed as `6a4c676`, on top of `bbce4e6 feat: sum types and
match`.

**Next piece of work:** copy-on-write collections — roadmap item 3, which
writing `std::json` promoted from "do before benchmarking" to "do next". See the
measurements in §7.

---

## 2. What was built

Three commits' worth, in order.

### `bytes`, `export struct`, map key removal, `std::env`

- **`bytes`** — a real type. Keyword, annotation, `as` both ways, `len`,
  indexing (gives an `int` 0–255), `+`, `==`, ordering. Four primitives
  (`bytes_slice`, `bytes_to_string`, `bytes_to_list`, `bytes_from_list`), three
  filesystem ones (`fs_read_bytes`, `fs_write_bytes`, `fs_append_bytes`), and
  `std::bytes` written in Xenith over them.
  - `text as bytes` never fails; `raw as string` **stops the program** on invalid
    UTF-8, and `bytes_to_string` hands the reason back instead. That split is
    deliberate: `as` when invalid bytes are a bug, `bytes_to_string` when they
    are a case to handle.
- **`export struct`** — structs cross module boundaries. Definitions travel on a
  `struct_exports` map beside `exports`, because a struct is a type and there is
  no value to put in the existing map. Importing a struct that was *not*
  exported now fails; it used to silently succeed.
- **`map.remove(key)`** — errors on a missing key, matching `pop` and `map[key]`.
  Pair it with `has_key`.
- **`std::env`** — `get`/`get_or`/`has`/`get_int`/`get_flag`/`set`/`unset`/`all`,
  `args`/`params`/`program`, `cwd`/`exit`. `get` returns `(value, found)` because
  unset and set-to-empty are different and one string cannot say which.

### Sum types and `match`

```xenith
enum Shape { Circle(float), Rect(float, float), Empty }

method size_of(s: Shape) -> string {
    release match s {
        Shape::Circle(r) when r > 10.0 => "a big circle"
        Shape::Circle(r) => "a circle of {r}"
        Shape::Rect(w, h) => "{w} by {h}"
        Shape::Empty => "nothing"
    }
}
```

Patterns: `_`, bindings, literals (incl. negatives and `null`),
`Enum::Variant(...)`, tuples, `A | B`, `when` guards. They nest.

Checked statically: completeness (XEN022), unknown variants, payload arity and
types, a variant of the wrong enum in a pattern, a literal of a type that can
never occur there, and that all arms produce the same type.

`export enum` works across modules like `export struct`.

### `std::json`

`enum Json` plus a recursive descent parser, a serialiser and the accessors, all
in Xenith. `parse` returns `(Json, string)` and never stops the program.

**It has seven variants, not six.** The sketch in §3 had `Number(float)`; the
shipped enum splits it into `Int(int)` and `Float(float)`:

```xenith
export enum Json {
    Null, Bool(bool), Int(int), Float(float), Text(string),
    Array(list<Json>), Object(map<string, Json>)
}
```

Xenith has a real i64 and collapsing it into a double throws away things nobody
gets back: `{"id": 9007199254740993}` round-trips exactly, and `{"age": 36}`
writes back out as `36` rather than `36.0`. Identifiers from other systems live
in exactly the range a double loses. The rule is that a number with no `.` and
no `e` is an `Int` when it fits and a `Float` when it does not; anything written
with a `.` or an `e` stays a `Float` whatever its value. `as_float` accepts
either, so code that does not care never looks. The cost is a seventh arm in
every exhaustive match, which is the right trade at this size.

Decisions worth keeping:

- **`parse` is strict.** Trailing commas, unquoted keys, leading zeros, `NaN`,
  `inf`, hex, raw control characters in strings, and any text after the value
  are all errors. Trailing text especially: it means a truncated document or two
  concatenated, and ignoring it hides that.
- **Its own depth limit of 200.** The interpreter stops runaway recursion at
  10000 frames by *ending the program*, which is the wrong answer for something
  that arrived over a socket. 200 is reached first and comes back as an ordinary
  error. `tests/cases/json_malformed.xen` posts it 5000 open brackets.
- **Numbers are validated before `as float` touches them**, because `as` on text
  that is not a number stops the program, and Rust's parser accepts `inf` and
  `0x10`, which are not JSON. `fits_int` compares the digit text against
  `9223372036854775807` rather than converting and catching a failure — there is
  nothing to catch.
- **`stringify` never emits invalid JSON.** An infinity or NaN becomes `null`,
  since JSON cannot write them. `check` reports one before it happens. (Note
  that `1.0 / 0.0` is an error in Xenith, so a non-finite float only arrives via
  overflow like `exp(1000.0)` or `"inf" as float`.)
- **Escaping on output is the minimum JSON requires.** Text outside ASCII goes
  through as UTF-8, not `\u` escapes.
- **`\uXXXX` surrogate pairs are combined**, and an unpaired half is an error —
  UTF-8 has nowhere to put one, and `from_code` rejects it.

**Write JSON literals in Xenith with backtick raw strings.** In double quotes
every `"` needs escaping and every `{` needs doubling, because `{` opens an
interpolation. `` `{"a": 1}` `` is just the document. All three fixtures and
every doc example are written that way; it is the first thing anyone hits.

### The string-lexer bug

`make_string`'s interpolation scanner had no terminator — not the closing quote,
not a newline, not end of input. `let out: string = "{"` ate every following line
until some unrelated `}` turned up, then reported an undefined variable in code
that was never the problem. Now bounded; see §7.

---

## 3. The conversation that produced the ordering

This is the part worth keeping.

### The original assessment

Xenith could not be used for backend work because of, in rough order of severity:

1. No `bytes` — HTTP is bytes; `fs_read` failed on anything non-UTF-8.
2. No sum types — **a JSON value has no type in Xenith.** A map is homogeneous;
   a JSON object's values are not. Not awkward, *inexpressible*.
3. No `export struct` — you cannot define an API across files.
4. No concurrency story.
5. Values deep-copy on every call (`args[i].clone()` in `values.rs`), so a 200KB
   request body is copied through every middleware layer.
6. No project tooling — `resolve_local` guesses among three directories.

1, 3 are done. 2 is done, and `std::json` is now written on top of it. 4, 5, 6
remain — and 5 turned out to be worse than "a body copied through every layer":
it makes building any list or map quadratic, which is measured in §7.

### Why not `async`/`await` — the important one

The user wants to write a controller that queries a database and waits for it.
The conclusion after working through it:

**`await` is a compilation feature, not a runtime feature.** Both languages that
have it well transform the function body at compile time:

- **V8** desugars `async function` into a generator: frame on the heap, body
  compiled into a `switch` on a resume point. You can see it directly — ask
  TypeScript to target ES5 and it emits `__awaiter`/`__generator` wrapping your
  body in exactly that switch.
- **rustc** generates an anonymous state-machine struct implementing `Future`.
  Each `await` is a variant; every local living across an await becomes a field.
  (That is also where `Pin` comes from: the struct can hold a reference to one of
  its own fields, and Rust has no move constructors to fix up the pointer.)

Neither suspends a native call stack. Xenith's `Interpreter::visit` recurses on
the Rust stack — thirty or forty frames deep at any `await` — and there is no
pass that could rewrite it.

Four ways out, and their honest costs:

| Option | Cost |
| --- | --- |
| Make `visit` an `async fn` | `Box::pin` per AST node — undoes every fix in `10-performance.md`. And futures must be `Send` to run multithreaded; `Rc<Context>` is not. **Full async tax, concurrency without parallelism, one core forever.** |
| Stackful coroutines (`corosensei`-style) | Real suspension without touching `visit`. Unsafe stack switching, platform-specific, still one thread. |
| Bytecode VM | The correct long-term answer. It *is* the rewrite. |
| Isolated tasks on OS threads | No suspension needed. Real parallelism. Zero evaluator changes. |

**The code already points at the fourth.** `Interpreter` holds `struct_defs`,
`enum_defs`, `type_aliases`, `module_registry`, `loading_modules`, and every
`visit` takes `&mut self`. Two concurrent tasks in one interpreter contend on
that immediately. Give each task its own `Interpreter` and it has its own heap;
once it has its own heap, the thread model falls out.

### The sequencing argument

**The standard library is written in Xenith, so it survives an evaluator
rewrite.** `std::string`, `std::bytes`, `std::fs` do not care how `visit` works.
Neither will `std::json` or `std::http`. The Rust builtins are just functions
over `Vec<Value>`; they survive too.

So the ordering is asymmetric:

- Server stack first, VM later → **nothing is lost.**
- VM first → months on an evaluator with no workload to profile, designing an
  async model with nothing to validate it.

And: shipping blocking-looking tasks now keeps every option open, because the
*semantics* ("a task is isolated, tasks communicate") are the same ones a
green-thread scheduler would have. Shipping `async`/`await` now fixes the syntax
permanently and colours every function in a stdlib not yet written.

### The middle path on `await`, if wanted sooner

```xenith
let users = spawn(fetch_users)
let posts = spawn(fetch_posts)
let (u, uerr) = await users
let (p, perr) = await posts
```

`await` meaning "join this task", not "yield to the scheduler". Real parallelism,
no interpreter surgery, and the keyword means what people intuitively think.
Ceiling is thread count; long-lived connections (websockets, SSE) cost a thread
each. Strict subset of true async, so a later VM tightens it without breaking
programs.

### Why sum types came before everything else

Every controller returns JSON, JSON has no type without them, and the hand-rolled
alternative — a recursive struct with a `kind` field and every payload field
present — requires spelling out all five fields at every leaf, because there are
no defaults. You would write it once and then rewrite it.

Proven: `tests/cases/enum_json.xen` builds and serialises a document with
`enum Json { Null, Bool(bool), Number(float), Text(string), Array(list<Json>),
Object(map<string, Json>) }`. Recursive enums work, including
`map<string, Json>` payloads.

---

## 4. Decisions made, and why

Keep these; they are not obvious from the code.

**`match` is an expression, not a statement.** Otherwise `let x = match ...` is
impossible, which is half the reason to have one. Accepts an asymmetry with
`when`, which stays a statement.

**Variant payloads are positional** (`Circle(float)`, not `Circle { radius: float }`).
Lighter for the one- and two-field cases that dominate.

**`null` stays.** Revisit if `Option<T>` ever arrives — two ways to say "absent"
is the mess Rust dodged.

**Enums are concrete, no generics.** So no `Result<T>`/`Option<T>`. But
`enum Json` and `enum HttpMethod` need no type parameters, which is what made
this shippable ahead of the generics decision.

**Enums reuse `Type::Struct(name, fields)` as the named-user-type node** rather
than getting a `Type::Enum`. Annotations are written identically for both, and
the parser cannot tell an imported name apart anyway. One place disambiguates
instead of one per phase. Documented at the variant in `types.rs`.

**`Identifier::name` is now exclusively an enum variant.** It used to parse into
a `VarAccess` for a static-method call that never existed; no `.xen` file used it.

**A guarded arm never counts toward completeness.** Whether it matches depends on
a value. Same rule as Rust, and easy to get wrong.

**`Circle(r)` covers `Circle`; `Circle(0.0)` does not.** Only a variant pattern
whose sub-patterns are all irrefutable counts — `Pattern::is_irrefutable`.
Without this a match with a real hole passes.

**An enum the checker cannot see is skipped entirely, not guessed at.** The
checker's stated rule is that a reported error must be a real one. Demanding arms
for an imported enum's unknown variants would be a false positive. The
interpreter catches those as XEN023 instead.

**A struct or enum cannot be renamed on import.** Identity is the name, so a
renamed one would be rejected by the exporting module's own methods. Refused with
a clear error rather than half-supported.

**`bytes` echoes as `<bytes N>`**, not its contents — printing raw bytes sprays
control characters at the terminal. `raw as string` is how you ask.

---

## 5. Open questions

Not yet decided. Listed with my leaning.

1. **Concurrency model** — commit to isolated tasks + channels + blocking-looking
   I/O? This decides every I/O signature in the stdlib, so it should be settled
   *before* `std::http`. Leaning: yes.
2. **Generics** — needed for `Result<T>`, `Option<T>`, and `map`/`filter` over
   collections. Suggested shape: type parameters on methods and structs,
   inference at call sites, **erased at runtime** (a tree-walker never needs
   monomorphisation), no constraints in v1. Much smaller than the Rust-shaped
   version.
3. **Error model** — keep `(value, string)` tuples, or move to a `Result`-shaped
   enum once generics land, plus a propagation operator? A controller doing five
   fallible things is the test case.
4. **Does `null` survive** if `Option<T>` arrives?
5. **Multi-line double-quoted strings** are currently allowed (accidentally).
   Most languages disallow them. Disallowing would make unterminated-string
   errors point at the opening quote instead of somewhere later.

---

## 6. Roadmap

Items 1 and 4 done. Remaining, in the order argued for above — except that 3 has
moved to the front, for the reason in §7.

**3. Copy-on-write collections. Do this next.** `append` and map insertion are
each O(n), so building an n element collection is O(n²). Measured, not guessed:
§7. This is no longer "before benchmarking anything server-shaped"; it is the
thing standing between `std::json` and being able to use it.

**2. Concurrency semantics.** The decision that blocks the rest of the stdlib.
Also requires dropping `panic = "abort"` from `Cargo.toml`.

**4. `std::json`.** Done. Written entirely in Xenith, as argued; if profiling
later says the character loop in `parse_string` or `parse_number` is the cost,
those move to Rust builtins without the signatures changing. Do not do that
before item 3, or you will move the wrong thing — the collection copying is a
much larger term than the scanning.

**5. `std::time`, `std::crypto`, `std::rand`.**

**6. `std::http`.** Client and server. Needs bytes (have it), the concurrency
decision (item 2), and JSON (item 4).

**7. Project tooling.** `xenith.toml`, lockfile, dependency fetching,
`xenith test`, `xenith fmt`. Also replaces the three-candidate path guessing in
`resolve_local`, which will behave strangely in a real project. *Go's advantage
over PHP was never mostly the language.*

**8. Bytecode VM,** informed by real programs and real profiles. Then `await` on
top of it.

Cheap and worth folding in early, since they hurt daily:

- Expressions cannot span lines (a long `||` chain must be one line).
- No hex literals (`0xff`), felt most when working with `bytes`.
- The static pass does not follow `grab`.

---

## 7. Known issues found along the way

**Building a collection is quadratic.** The largest thing found this session, and
the reason roadmap item 3 moved to the front.

`xs.append(x)` and `m[k] = v` each cost O(n) in the size of the collection, so
filling one is O(n²). Timings, on the release build:

| Program | Time |
| --- | --- |
| 2000 × `xs.append(i)` | 0.27s |
| 8000 × `xs.append(i)` | 3.99s |
| 2000 × `m["k{i}"] = i` | 0.56s |
| 8000 × `m["k{i}"] = i` | 8.70s |

Four times the work, fifteen times the time, both of them. `append`ing a 200
character string instead of an int costs 7.5× more than the int at the same
count, so the copy is deep, not a pointer shuffle.

The effect on `std::json`, parsing a JSON array of objects:

| Document | Parse |
| --- | --- |
| 6.4KB | 0.55s |
| 12.8KB | 1.06s |
| 25.8KB | 2.78s |
| 51.9KB | 9.06s |

Doubling the document more than triples the time. Two things it is *not*: string
arguments are not cloned per call (20000 calls passing a 25KB string cost the
same as passing an int), and character indexing is only mildly position
dependent. It is the collections.

This is `args[i].clone()` in `Function::execute` (`values.rs`) — the item already
on the roadmap — and it caps every list- or map-building program in the language,
not just this one. `std::string`'s `split` has the same shape.

**The static pass does not follow `grab`.** An imported method call, struct
literal or enum `match` is checked as it *runs*, not before. Same errors, same
messages; what is lost is finding out before output appears, and finding out
about a branch that never executes. This makes cross-module checking a
**prerequisite** for enum completeness checking to be worth much — the enum you
most want checked is the one from `std::json`. Top of the contributing list.

**`from` cannot be a struct field name.** Any keyword is reserved inside a
literal, so `Line { from: a, to: b }` does not parse. Hit this writing test
fixtures.

**Broken-pipe panic.** `xenith foo.xen | head` panics instead of exiting. One-line
fix: `flush().unwrap()` → `.ok()` in `echo` and `clear` in `values.rs`. Not done.

**`bytes` is now a reserved word.** It broke one fixture that used it as a
variable name.

**No exponent literals.** `1.0e20` lexes as `1.0` followed by an undefined
variable `e20`. `"1e20" as float` is the workaround, and is how `std::json`
writes its float constants. Belongs with the hex-literal gap in §6.

**An empty `[]` or `{}` has no type in a `match` arm.** `_ => []` where the
method returns `list<string>` is XEN001, "expected `list<string>`, found
`null`". A typed `let none: list<string> = []` outside the match and `_ => none`
works. Three methods in `std::json` are written that way.

**Fixed this session:** unterminated strings, raw strings, and interpolations no
longer swallow the rest of the file. The scanner cannot simply stop at the next
quote, because an interpolated expression may contain a string of its own —
`"{ages["ada"]}"` is ordinary and appears in the tutorial. It tracks nested
strings and stops at a newline or end of input. Do not "simplify" this later;
`tests/cases/string_braces.xen` pins it.

---

## 8. Practical notes

**Testing** is fixture-driven; adding a test means adding a file, not writing
Rust.

| Directory | Holds | Checked |
| --- | --- | --- |
| `tests/cases/` | `name.xen` + `name.out` | exit 0, byte-identical stdout |
| `tests/errors/` | `name.xen` + `name.err` | non-zero exit, that code in stderr |
| `tests/modules/` | `main.xen` + its imports | exit 0 and its `.out` |

A `.xen` with no `.err` beside it is a support file and is not run alone.

**Docs** are kept current with the code, and the tutorial was renumbered this
session (a new `13-enums.md` pushed 13–19 to 14–20). All cross-links were
verified to resolve.

**A convention worth keeping:** every code example in the docs was executed and
its output pasted from the run, rather than written from memory.

**Where things live:**

| What | Where |
| --- | --- |
| Enum/match runtime | `visit_enum_def`, `visit_enum_variant`, `visit_match`, `pattern_matches` in `interpreter.rs` |
| Completeness checking | `check_exhaustive` in `checker.rs` |
| Pattern grammar | `parse_pattern`, `match_expression` in `parser.rs` |
| The string scanner | `make_string` in `lexer.rs` |
| JSON | `src/stdlib/json.xen`, all of it Xenith |
| New builtins | `builtins/registry.rs` + an arm in `BuiltInFunction::execute` |
| Stdlib modules | `src/stdlib/*.xen`, registered in `mod.rs` |

**New error codes this session:** XEN022 (Match Not Exhaustive, static), XEN023
(No Matching Case, runtime), XEN009 reused for Variant Not Found. Next free code
is **XEN024**.
