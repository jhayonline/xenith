# HTTP Server Stack Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make Xenith’s native HTTP router serve real requests and expose typed request, query, header, JSON, shutdown, and error APIs.

**Architecture:** Keep parsing, sockets, routing, and allocation in the native runtime. Add thin checker signatures, standard-module wrappers, and codegen lowerings. Build a blocking server first; keep JSON as UTF-8 text helpers instead of adding reflection.

**Tech Stack:** Rust compiler frontend/codegen, embedded C runtime, Xenith integration tests, Cargo.

**Spec:** `docs/superpowers/specs/2026-11-04-http-server-stack-design.md`

## Global Constraints

- No new Xenith keywords or forced MVC/OOP.
- No garbage collector or manual allocation in application code.
- Use the shared status/error ABI for fallible native operations.
- Preserve unrelated `tests/native_frontend.rs` changes.
- Commit and push each completed stage without co-authored trailers.

## Tasks

### Task 1: Blocking `serve` loop

- [ ] Add a failing real TCP Xenith test that registers `/health`, calls `serve`, and verifies an HTTP response.
- [ ] Add `http_serve` signature, wrapper, codegen lowering, and runtime accept/read/dispatch/write loop.
- [ ] Add deterministic test shutdown using a one-request server mode or explicit shutdown handle.
- [ ] Run HTTP tests and commit `feat: add native HTTP serve loop`.

### Task 2: Request headers/query/parameters

- [ ] Add failing tests for query extraction, headers, and `:id` parameters.
- [ ] Extend the request representation and ABI with typed string accessors.
- [ ] Lower the accessors and document their semantics.
- [ ] Run focused tests and commit `feat: expose typed HTTP request data`.

### Task 3: JSON text helpers

- [ ] Add failing tests for JSON response content type and request body extraction.
- [ ] Implement `json(status, body)` and `json_body(request)` with validation/error strings.
- [ ] Run focused tests and commit `feat: add native HTTP JSON helpers`.

### Task 4: Shutdown and configuration

- [ ] Add failing test for explicit server shutdown/configuration.
- [ ] Add server handles, shutdown state, and bounded connection behavior.
- [ ] Run focused tests and commit `feat: add HTTP shutdown configuration`.

### Task 5: Wrapper error propagation and final verification

- [ ] Add a failing allocation/error propagation test through `try/catch` and standard HTTP functions.
- [ ] Update wrapper lowering so pending native errors remain catchable across standard-module calls.
- [ ] Update README with the final API.
- [ ] Run `git diff --check`, `cargo test --test native_http`, and `cargo test`.
- [ ] Commit `feat: make HTTP failures recoverable` and push all changes to `main`.
