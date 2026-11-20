# Xenith

A programming language with its own native compiler. Source is parsed and type
checked in Rust, lowered through Cranelift to an object file, and linked against
a C runtime that the compiler emits itself. There is also a tree-walking
interpreter and a bytecode VM, kept for differential testing against the native
backend.

## Layout

| Path | What it is |
|---|---|
| `src/native/` | The native backend: `syntax.rs`, `check.rs`, `codegen.rs`, `project.rs`, and `runtime.rs` |
| `src/native/runtime.rs` | **The entire C runtime, as one Rust string literal.** Read the warning below before editing |
| `src/interpreter.rs`, `src/vm/` | Interpreter and bytecode VM |
| `src/stdlib/*.xen` | Interpreter-only standard library, written in Xenith |
| `src/native/project.rs` | `standard_native_source()` holds the native `std::*` modules, also written in Xenith |
| `docs/internals/` | Architecture notes |
| `docs/superpowers/plans/`, `docs/superpowers/specs/` | Design plans and specs. See the caveat below |

Native `std::*` modules are plain Xenith source strings inside
`standard_native_source()`, not files. Adding a module means adding an arm
there; adding a function to one means editing its string.

## Building and testing

```sh
cargo build
cargo test --no-fail-fast        # ~2 minutes
```

**Always pass `--no-fail-fast`.** Cargo stops at the first failing test binary
and says nothing about the rest. A plain `cargo test` once reported a green run
while 29 tests in three later binaries were failing, because the suite never
reached them.

Most native tests compile a Xenith program with `cc` and run it, so they are
real end-to-end builds. Two test harness shapes exist, and the difference
matters:

- `parse()` + `check()` — single file, builtins only. No `grab` imports.
- `project::load_program()` — needed for anything that imports `std::*`.

A test written the first way cannot use `grab`, which is how a whole module can
sit unexercised. Prefer the project form when touching a `std::*` module.

## Adding a runtime builtin

Four places, all required, in this order:

1. `src/native/runtime.rs` — the C implementation
2. `src/native/check.rs` — the signature, in `signatures()`
3. `src/native/codegen.rs` — the lowering in `lower()`
4. `src/native/project.rs` — the `std::*` wrapper, if it should be reachable without naming the builtin

Miss step 4 and the feature works but nothing can reach it idiomatically. Miss a
test through `load_program()` and you will not notice.

## Hazards that have already cost real time

**Struct layouts in `runtime.rs` are invisible to the Rust compiler.** Codegen
reads these C structs at hardcoded offsets. Adding a field to one silently
breaks any other struct meant to mirror it: `XenithMap` gained an `owner` field,
which moved `entries[]` from offset 16 to 24, while a duplicate `XenithDbMap`
kept the old layout. Rows were written through one definition and read through
the other. It compiled clean and corrupted memory — 29 tests failed with no type
error. Never keep a parallel copy of one of these structs; share the single
definition.

**Values lower to a fixed number of lanes.** A `string` is two lanes (pointer,
length); `bytes` is *one* (a pointer to a length-carrying object). Reading a
lane that is not there panics the compiler rather than failing to build.
`tcp_write` shipped reading `body[1]` of a one-lane `bytes` and panicked on
every call, because no test ever called it. Copy the lane handling from a
working builtin of the same signature.

**Raise messages derive their own length.** Use the `xenith_raise(message, code,
name)` helper. Hand-written byte counts beside a string literal drifted on 31 of
51 sites, over-reading past the literal or truncating the text.

**Verify documentation examples by compiling them.** Two README examples did not
build — one used tuple destructuring, which the native backend rejects outright.
String interpolation only accepts a bare identifier of type `string`, so
`"{len(x)}"` and `"{f(x)}"` are both errors. Compile and run an example before
pasting it in.

## Plan documents record intent, never progress

Every file in `docs/superpowers/plans/` has its checkboxes unticked, including
plans that shipped in full — `2026-11-04-http-server-stack.md` has twenty open
boxes and is entirely done. **Do not read an unchecked box as outstanding
work.** Establish what exists from `git log`, the builtin table in
`check.rs`, and the test suite. Treat the plans as design rationale.

## Commits

- **No tool attribution.** No `Co-Authored-By`, no "Generated with", no 🤖, and
  no mention of Claude, Anthropic, Codex or any other tool in commit messages or
  PR descriptions.
- **Small, focused commits.** One logical change each. Split behaviour changes,
  tests and formatting rather than bundling them.
- Write messages that explain why the change is correct, not just what moved.

## State as of 2026-11-20

`main` is green: **531 tests passing, 0 failing.** Working tree clean and pushed.

Shipped: the language core, native `std::env`, `std::config`, `std::fs`,
`std::db`, `std::http`, `std::net` and `std::auth`; HTTP server with routing,
middleware and pooled workers; HTTP and HTTPS clients with keep-alive pools;
TLS via the system OpenSSL, weakly linked; streaming responses with trailers;
TCP over IPv4, IPv6 and resolved host names.

### Known blocker before working on a Mac

`link_host()` in `src/native/codegen.rs` hardcodes
`-lsqlite3 -lcrypt -lssl -lcrypto`, and the C runtime includes `<crypt.h>` and
calls `crypt_r`. None of `crypt.h`, `crypt_r` or `-lcrypt` exists on macOS, and
OpenSSL is not present by default. There is no `cfg(target_os)` handling
anywhere in the tree.

So on macOS the Rust crate compiles, but **every test that links a Xenith
program fails**, which is most of the suite. Expect that before assuming a
regression. Making this portable is its own task: pick the password hash per
platform, drop `-lcrypt` off Linux, and point at Homebrew's OpenSSL.

The README's "major work still ahead" section lists what remains beyond that.
