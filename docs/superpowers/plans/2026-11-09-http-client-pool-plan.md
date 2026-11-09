# HTTP Client Pool Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace the single-socket HTTP client pool with a bounded, endpoint-keyed keep-alive pool with explicit cleanup.

**Architecture:** The native runtime will own a mutex-protected dynamic array of connection records. Each record contains an endpoint key, socket, in-use state, and per-record mutex; requests reuse idle matching records or create new records up to the configured bound. Failures remove records, and `pool_close` closes and frees all records.

**Tech Stack:** Rust native checker/codegen/project loader, generated C runtime, POSIX sockets and pthreads, Cargo integration tests.

**Spec:** `docs/superpowers/specs/2026-11-09-http-client-pool-design.md`

## Global Constraints

- Preserve the tuple API `(status, response, error)`.
- Keep failures recoverable through the existing status/error ABI.
- Do not add dependencies.
- Keep commits focused and free of co-author or tool-attribution trailers.

## Review Focus

- Two requests to one endpoint reuse a socket; test in `native_http_client.rs`.
- Requests to different endpoints do not share a socket; test with two listeners.
- The configured maximum rejects excess concurrent requests; test with a blocked server.
- `Connection: close` causes eviction and the next request reconnects; test with two accepts.
- Closing a pool releases sockets and rejects later requests; test the native `pool_close` path.

### Task 1: Specify pool behavior with failing integration tests

**Files:**
- Modify: `tests/native_http_client.rs`

**Interfaces:**
- Consumes: existing `client_pool`, `pool_request`, and project test helpers.
- Produces: executable regression tests for reuse, endpoint isolation, reconnect, capacity, and close behavior.

- [ ] **Step 1: Add failing tests** for same-endpoint reuse, reconnect after `Connection: close`, endpoint isolation, bounded concurrency, and `pool_close`.
- [ ] **Step 2: Run `cargo test -q --test native_http_client`** and verify the new tests fail because the runtime has no multi-record pool or close API.
- [ ] **Step 3: Commit** the red tests with `test: define HTTP client pool lifecycle behavior`.

### Task 2: Implement the native pool data model and request lifecycle

**Files:**
- Modify: `src/native/runtime.rs`

**Interfaces:**
- Consumes: existing `xenith_tcp_connect`, `xenith_http_send_all`, timeout helpers, and recoverable error helpers.
- Produces: `xenith_http_client_pool_request_reuse(...)` and `xenith_http_client_pool_close(int64_t)` with the existing request tuple ABI.

- [ ] **Step 1: Add the dynamic connection-record structs** and replace the single `fd/address` fields with record storage, preserving pool bounds and mutex ownership.
- [ ] **Step 2: Implement record acquisition** keyed by endpoint, including idle reuse, creation up to `maximum`, and recoverable exhaustion errors.
- [ ] **Step 3: Implement persistent HTTP/1.1 request/response exchange** with content-length framing and keep-alive headers.
- [ ] **Step 4: Implement eviction** for failed I/O, malformed/incomplete responses, and `Connection: close`, then reconnect on the next request.
- [ ] **Step 5: Implement `xenith_http_client_pool_close`** to mark closed, close sockets, destroy record mutexes, free endpoint keys/records, and free the pool.
- [ ] **Step 6: Run the focused tests** and verify all Task 1 tests pass.
- [ ] **Step 7: Commit** with `feat: add bounded reusable HTTP pool connections`.

### Task 3: Wire checker, project wrappers, and code generation

**Files:**
- Modify: `src/native/check.rs`
- Modify: `src/native/codegen.rs`
- Modify: `src/native/project.rs`

**Interfaces:**
- Consumes: runtime functions from Task 2.
- Produces: typed `pool_close(pool: i64) -> null` and the updated pool request lowering.

- [ ] **Step 1: Add the `http_client_pool_close` checker signature and `std::http.pool_close` wrapper.**
- [ ] **Step 2: Point pool requests at the reusable runtime symbol and lower `pool_close` to the close ABI.**
- [ ] **Step 3: Run focused native HTTP tests and the checker tests.**
- [ ] **Step 4: Commit** with `feat: expose HTTP client pool cleanup`.

### Task 4: Document and verify the shipped behavior

**Files:**
- Modify: `README.md`
- Modify: `docs/superpowers/specs/2026-11-09-http-client-pool-design.md` only if implementation decisions need clarification.

- [ ] **Step 1: Update the HTTP client documentation** with multi-endpoint reuse, bounded capacity, reconnect behavior, and `pool_close`.
- [ ] **Step 2: Run `cargo test --no-fail-fast` and `git diff --check`.**
- [ ] **Step 3: Commit** with `docs: document reusable HTTP client pools`.
- [ ] **Step 4: Push all focused commits to `origin main`.**
