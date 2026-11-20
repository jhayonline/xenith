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
| `src/native/http.rs`, `orm.rs` | Rust-side HTTP parser/server and SQLite ORM. Only tests use them; compiled programs use the C runtime |
| `tests/common/mod.rs` | Helpers shared by test binaries via `mod common;`, e.g. `connect_when_ready` |
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
`std::fs` sat like this until 2026-11-20, and its `append` and `remove` could
not be called at all.

**A test binary that dies takes its results with it.** A stack overflow or
signal aborts the whole binary, and cargo prints no `test result` line for it,
so the totals simply come out lower. Check that every `Running` line has a
`test result` before trusting a count.

Tests that spawn a compiled server must not sleep a fixed time before
connecting. macOS assesses a newly linked executable on its first launch, which
takes 300-550 ms. Use `common::connect_when_ready(port)`.

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
definition. The same goes for arrays codegen builds on the stack for the
runtime: model field kinds were stored eight bytes apart for a C parameter
declared `const int8_t *`, so every field after the first read stray stack
bytes. Match the element width to the C declaration.

**Values lower to a fixed number of lanes.** A `string` is two lanes (pointer,
length); `bytes` is *one* (a pointer to a length-carrying object). Reading a
lane that is not there panics the compiler rather than failing to build.
`tcp_write` shipped reading `body[1]` of a one-lane `bytes` and panicked on
every call, because no test ever called it. Copy the lane handling from a
working builtin of the same signature.

**Raise messages derive their own length.** Use the `xenith_raise(message, code,
name)` helper. Hand-written byte counts beside a string literal drifted on 31 of
51 sites, over-reading past the literal or truncating the text.

**aarch64 passes a struct-return pointer in x8, not as an argument.** Generated
functions returning a string, struct or more than two lanes take a Cranelift
`StructReturn` pointer. x86-64 passes it as the first argument; aarch64 puts it
in x8. Any C code that calls back into generated code must use the
`XenithIndirectResult` pattern in `runtime.rs`, which declares the callback as
returning a large struct so C loads x8. Route handlers and map callbacks got
this wrong and crashed on every arm64 host, unnoticed because the suite only
ran on x86-64.

**glibc forgives memory bugs that macOS does not.** Reading a struct after
`free`, keeping `getenv`'s pointer past an `unsetenv`, and reading
`/proc/self/cmdline` all worked on Linux and failed on macOS. When something
passes on Linux and crashes elsewhere, suspect a lifetime first. Handles with a
`closed` flag must keep their struct alive after close, as the router, config
and schema handles do; `xenith_http_response_close` still frees its struct.

**Platform differences live in `runtime::host_link_args()` and `#if` blocks in
the C.** Don't add `-l` flags anywhere else; the test harnesses that compile the
runtime directly take the same function's output. Two Linux toolchain traps it
already handles: weak references do not count towards `--as-needed`, which
Debian and Ubuntu enable by default and which silently dropped SQLite and
OpenSSL, so it passes `--no-as-needed`; and the runtime must not call a GNU
extension such as `memmem` undeclared, because gcc 12 then truncates the
returned pointer to `int` with only a warning. Run the C through
`gcc -fsyntax-only -Wall` on Linux when adding libc calls.

**Codegen recursion is deep and its frames are huge.** `lower()` is one function
of more than 6,000 lines, about 100 KB of stack per expression nesting level in
a debug build. `emit_object` therefore runs on its own 256 MB thread. Parsing
and checking still run on the caller's stack, at about 10 KB a level.

**Intrinsic names can shadow imports.** The checker special-cases `append`,
`remove`, `map`, `filter`, `find`, `contains`, `len`, `min`, `max` and others
by name. A `std::*` export with one of those names must be checked for, as
`std::fs` `append` and `remove` now are. Codegen already prefers the program's
function.

**Errors are a pending flag, not an unwind.** A raise sets the flag and
returns normally. Codegen checks the flag after every statement and right after
every call into generated code (`emit_error_check` in `codegen.rs`). Inside a
`try` body the check jumps to the catch (`TRY_TARGETS`); otherwise the function
releases its bindings and returns at once with the flag still set, and `main`
ends the program. Two consequences to keep in mind:

- A function that returns early leaves zeros in its result. Anything in the C
  runtime that calls back into generated code must check
  `xenith_error_pending()` before trusting what came back, as the collection
  helpers do.
- HTTP handlers are the exception: `xenith_http_after_handler` ends the process
  on any error that is not a database error, which is what handlers did before
  errors could propagate. Turning those into 500 responses is an open decision.
- A catch takes ownership of the error's message (`xenith_error_catch`, then
  `xenith_error_forget`). Never call `xenith_error_clear` while a catch binding
  may still read it: clearing frees the message, which is how `error.message`
  printed garbage until 2026-11-20.

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

Test results by host:

| Host | Passing | Notes |
|---|---|---|
| macOS, Apple silicon | 552 | Needs Homebrew OpenSSL (`brew install openssl`) or `OPENSSL_DIR` |
| Linux arm64, Debian 12 | 553 | Includes the Linux-only legacy `$6$` password test |
| Linux x86-64, Debian 12 | 536 of 540 | Before the `try` fix, under qemu emulation. The 4 failures were `cc` crashing or failing to link under the emulator, not assertions; rerun on real hardware to confirm |

To run the Linux suite from a Mac, use a `rust:1-bookworm` container with
`libsqlite3-dev libssl-dev libcrypt-dev openssl` installed, clone the
repository inside it, and run `cargo test --no-fail-fast`.

Shipped: the language core, native `std::env`, `std::config`, `std::fs`,
`std::db`, `std::http`, `std::net` and `std::auth`; HTTP server with routing,
middleware and pooled workers; HTTP and HTTPS clients with keep-alive pools;
TLS via the system OpenSSL, weakly linked; streaming responses with trailers;
TCP over IPv4, IPv6 and resolved host names. Passwords hash with
PBKDF2-SHA256 and session cookies sign with HMAC-SHA256, both from libcrypto.

Known open issues, none of them fixed yet:

- `xenith_http_response_close` frees its struct, so a double close is a
  use-after-free.
- The pooled and streaming HTTP clients run `strstr` over receive buffers;
  check each is NUL-terminated. Header names are matched case-sensitively.
- Parsing and checking recurse on the caller's stack at about 10 KB a level.
- Codegen's call lowering matches about 160 builtin and `std` export names
  (`load_config`, `required_config`, ...) before it looks for a program
  function of that name, and only 10 of those checks defer to the program. A
  user function named like one of the rest type-checks as the user's function
  but compiles as the builtin, failing in the Cranelift verifier or worse.

The README's "major work still ahead" section lists what remains beyond that.
