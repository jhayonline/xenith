# Native HTTP Router Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a typed Xenith `std::http` router with route registration and method/path dispatch on top of the native socket ABI.

**Architecture:** Keep routing in the C native runtime behind opaque `i64` handles. Add checker signatures, codegen lowerings, and standard-module wrappers without adding language keywords. Function values use the existing native closure ABI; dispatch returns `tuple<i32, string>` and route registration returns an error string.

**Tech Stack:** Rust compiler frontend/codegen, embedded C runtime, Xenith integration fixtures, Cargo test.

**Spec:** `docs/superpowers/specs/2026-11-04-native-http-router-design.md`

## Global Constraints

- No new Xenith keywords or MVC-only syntax.
- No garbage collector or manual allocation in application code.
- Recoverable failures use the shared status/error ABI.
- Do not stage unrelated changes in `tests/native_frontend.rs`.
- Commits must not contain co-authored trailers.

## Review Focus

- Duplicate or malformed route patterns return a clear registration error.
- Method mismatch returns a typed 404 response instead of calling a handler.
- Paths with trailing slashes and `:parameter` segments match predictably.
- Handler function values receive the expected method, path, and body lanes.
- Router close and allocation failures do not crash the process.

### Task 1: Router API signatures and failing integration fixture

**Files:**
- Modify: `tests/native_http.rs`
- Modify: `src/native/check.rs`
- Modify: `src/native/project.rs`

- [ ] **Step 1: Write the failing test** `xenith_http_router_dispatches_a_registered_route` compiling a Xenith program that imports `std::http`, registers `get(router, "/hello", handler)`, dispatches `GET /hello`, and asserts status `200` and body `hello`.
- [ ] **Step 2: Run the focused test and verify it fails** with an unknown standard function/module or unsupported builtin diagnostic.
- [ ] **Step 3: Add checker signatures and standard wrappers** for `http_router_new`, `http_router_get`, `http_router_post`, `http_router_dispatch`, and `http_router_close` with the exact types specified by the design.
- [ ] **Step 4: Run the focused test and verify it reaches an unimplemented native symbol failure.
- [ ] **Step 5: Commit** `feat: define native HTTP router API`.

### Task 2: Runtime router storage and matching

**Files:**
- Modify: `src/native/runtime.rs`
- Test: `tests/native_http.rs`

- [ ] **Step 1: Add focused failing cases** for a `:id` route, method mismatch, and missing route.
- [ ] **Step 2: Run the focused cases to confirm failure.
- [ ] **Step 3: Implement opaque router allocation, route registration, method/path matching, dispatch response tuple lanes, explicit close, and recoverable allocation/validation errors in the embedded C runtime.
- [ ] **Step 4: Run the focused HTTP tests and verify all pass.
- [ ] **Step 5: Commit** `feat: implement native HTTP route matching`.

### Task 3: Compiler lowering and handler invocation

**Files:**
- Modify: `src/native/codegen.rs`
- Test: `tests/native_http.rs`

- [ ] **Step 1: Add a failing fixture using a Xenith named handler value and a POST body.
- [ ] **Step 2: Implement codegen lowerings for router creation, registration, dispatch tuple lanes, and close using the existing closure handle ABI.
- [ ] **Step 3: Run focused tests and verify handler arguments and response values.
- [ ] **Step 4: Commit** `feat: lower Xenith HTTP router calls`.

### Task 4: Full verification and documentation

**Files:**
- Modify: `README.md`
- Test: `tests/native_http.rs`

- [ ] **Step 1: Document the minimal router API and its current non-goals.
- [ ] **Step 2: Run `git diff --check`, `cargo test --test native_http`, and `cargo test`.
- [ ] **Step 3: Commit** `docs: document native HTTP router`.
- [ ] **Step 4: Push all commits to `origin main` and verify the worktree contains only the pre-existing unrelated modification.
