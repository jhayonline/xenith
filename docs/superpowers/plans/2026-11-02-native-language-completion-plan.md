# Native Language Completion Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Extend the native compiler with the approved enum/match, methods, iteration, interpolation, function-value, ownership, import, and diagnostic subsets.

**Architecture:** Add syntax nodes and spans in `src/native/syntax.rs`, type rules in `src/native/check.rs`, Cranelift lowering in `src/native/codegen.rs`, and small C ABI helpers in `src/native/runtime.rs`. Keep unsupported edge cases explicit diagnostics. Preserve the existing flattened module loader while carrying exported metadata needed by the checker.

**Tech Stack:** Rust, Cranelift, C ABI runtime, Cargo integration tests.

**Spec:** `docs/superpowers/specs/2026-11-02-native-language-completion-design.md`

## Global Constraints

- Native syntax uses `fn`, `return`, `i32/i64`, parenthesized conditions, and existing `for` syntax.
- Enum tags are `i32`; recursive enum payloads are rejected in the first milestone.
- Map iteration is lexical-key sorted.
- Interpolation accepts string, integer, float, bool, null, and bytes only.
- First closure pass supports non-capturing function values; captured scalars are a later pass.
- Heap values live to program exit; mutation cannot invalidate aliases.
- Every task is test-first, fully verified, committed, and pushed to `main`.

## Review Focus

- Wrong enum variant payload arity/type and non-exhaustive matches fail statically.
- Empty list/map iteration performs zero body executions.
- Map iteration order is deterministic and lexical.
- Interpolation brace escaping and unsupported operands produce located diagnostics.
- Indirect calls preserve multi-lane argument/result layouts.

### Task 1: Native enum declarations and variant expressions

**Files:**
- Modify: `src/native/syntax.rs`, `src/native/check.rs`, `src/native/codegen.rs`, `src/native/runtime.rs`
- Test: `tests/native_enums.rs`

- [ ] Add failing tests for enum declarations, unit variants, payload variants, and static payload mismatch rejection.
- [ ] Run the focused tests and confirm parser/checker failures.
- [ ] Add `Enum`, `Variant`, and enum type metadata to the native AST/checker.
- [ ] Lower enum values to tag-plus-payload storage with `i32` tags and reject recursive payload layouts.
- [ ] Run focused and full Cargo tests.
- [ ] Commit `feat: add native enum values` and push.

### Task 2: Exhaustive native `match`

**Files:**
- Modify: `src/native/syntax.rs`, `src/native/check.rs`, `src/native/codegen.rs`
- Test: `tests/native_enums.rs`

- [ ] Add failing tests for variant arms, wildcard arms, payload bindings, common arm result types, and non-exhaustive matches.
- [ ] Implement match parsing/checking and deterministic tag-branch lowering.
- [ ] Verify empty payload and multi-lane payload handling.
- [ ] Commit `feat: add native enum matching` and push.

### Task 3: Methods and collection iteration

**Files:**
- Modify: `src/native/syntax.rs`, `src/native/check.rs`, `src/native/codegen.rs`, `src/native/runtime.rs`
- Test: `tests/native_methods.rs`, `tests/native_collections.rs`

- [ ] Add failing tests for receiver methods, list iteration, map key/value iteration, empty collections, and lexical map order.
- [ ] Add method declarations/resolution with receiver-first lowering.
- [ ] Add list/map iteration lowering and a sorted-key runtime iterator path.
- [ ] Verify mutable receiver checks and loop scope behavior.
- [ ] Commit `feat: add native methods and iteration` and push.

### Task 4: Native string interpolation

**Files:**
- Modify: `src/native/syntax.rs`, `src/native/check.rs`, `src/native/codegen.rs`, `src/native/runtime.rs`
- Test: `tests/native_strings.rs`

- [ ] Add failing tests for interpolation, escaped braces, numeric/bool/null/bytes formatting, and unsupported operand diagnostics.
- [ ] Parse interpolation parts without losing source spans.
- [ ] Add checked conversion helpers and concatenation lowering.
- [ ] Verify formatting matches existing interpreter conventions.
- [ ] Commit `feat: add native string interpolation` and push.

### Task 5: Function values and closures

**Files:**
- Modify: `src/native/syntax.rs`, `src/native/check.rs`, `src/native/codegen.rs`, `src/native/runtime.rs`
- Test: `tests/native_functions.rs`

- [ ] Add failing tests for assigning named functions to typed function values, indirect calls, arity/type errors, and scalar captures.
- [ ] Add function types and indirect-call AST/checker rules.
- [ ] Lower non-capturing function pointers, then copy-captured scalar environments.
- [ ] Reject unsupported mutable/recursive captures with located diagnostics.
- [ ] Commit `feat: add native function values and closures` and push.

### Task 6: Ownership, imports, and diagnostics

**Files:**
- Modify: `src/native/project.rs`, `src/native/mod.rs`, `src/native/check.rs`, `src/native/runtime.rs`
- Test: `tests/native_cli.rs`, `tests/native_frontend.rs`, new native feature tests

- [ ] Add failing tests for alias-safe append, imported enums/methods, imported diagnostics, and deterministic multi-error output.
- [ ] Make heap mutation alias-safe under arena-to-exit lifetime.
- [ ] Preserve exported enum/function/method identity through module loading and checking.
- [ ] Add source-file origin, primary/secondary spans, and stable diagnostic ordering.
- [ ] Run the full suite and commit `feat: complete native language diagnostics and imports`.

