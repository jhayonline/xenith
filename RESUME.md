# RESUME

Working notes from a design-and-build session on making Xenith a language you
can write a backend in. Written to be picked up cold.

The code is in the repo and speaks for itself. What is *not* in the repo is the
reasoning behind the ordering, the options that were rejected and why, and the
decisions still open. That is most of what is below.

---

## 1. Pick up here

**Uncommitted:** the string-lexer bug fix, plus 8 new test fixtures. Everything
else is committed (`bbce4e6 feat: sum types and match`).

```sh
git status              # the lexer fix + tests/cases/string_braces.* + tests/errors/unclosed_*
cargo test              # green as of the end of the session
```

Suggested message for it:

```
fix: an unterminated string or interpolation no longer eats the rest of the file

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>
```

**Next piece of work:** `std::json`, written in Xenith on top of the `Json` enum
that is now expressible. See §6.

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

1, 3 are done. 2 is done. 4, 5, 6 remain.

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

Item 1 done. Remaining, in the order argued for above.

**2. Concurrency semantics.** The decision that blocks the stdlib. Also requires
dropping `panic = "abort"` from `Cargo.toml`.

**3. Copy-on-write collections.** `args[i].clone()` (`values.rs`, in
`Function::execute`) deep-copies lists, maps and structs into every call. Move to
`Rc` + COW before benchmarking anything server-shaped, or you will draw wrong
conclusions about where time goes.

**4. `std::json`.** Now writable. **This is the next concrete task.** Write the
value type as an enum, the parser and serialiser in Xenith first; move the hot
parts to Rust builtins later if measurement says so.

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
| New builtins | `builtins/registry.rs` + an arm in `BuiltInFunction::execute` |
| Stdlib modules | `src/stdlib/*.xen`, registered in `mod.rs` |

**New error codes this session:** XEN022 (Match Not Exhaustive, static), XEN023
(No Matching Case, runtime), XEN009 reused for Variant Not Found. Next free code
is **XEN024**.
