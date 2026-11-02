# Native Language Completion Design

## Intent

Complete the native compiler's typed language surface so native programs can use the language features already represented by Xenith's interpreter: enums and exhaustive matching, methods, collection iteration, interpolation, function values/closures, explicit error values, imports, and precise diagnostics.

## Goals

- Keep native programs statically typed and executable through the existing C ABI runtime.
- Reuse existing Xenith syntax and semantics where the interpreter already defines them.
- Add focused parser, checker, runtime, and end-to-end tests for every milestone.
- Preserve current collection, tuple, numeric, control-flow, and import behavior.

## Non-goals

- Replacing the interpreter or VM.
- Introducing a garbage collector or changing the existing value model wholesale.
- Adding implicit exceptions or silently accepting unsupported constructs.

## Constraints

- Native lowering currently uses Cranelift and a small C runtime in `src/native/runtime.rs`.
- Native values must have stable layouts for generated code and imported functions.
- Existing user edits and public syntax must remain compatible.
- Each milestone is test-first, independently verifiable, committed, and pushed to `main`.

## Approaches

1. **Native-first lowering (chosen):** extend `src/native/syntax.rs`, `check.rs`, and `codegen.rs`, adding explicit runtime layouts and helper calls. This provides one coherent native implementation and catches unsupported cases statically.
2. **Interpreter fallback:** route unsupported native constructs through the interpreter. This would reduce native work but would produce different runtime behavior and prevent native binaries from being self-contained.
3. **Shared IR rewrite:** replace both frontends with a new common IR. This is too broad for the current compiler and would delay useful milestones.

The native-first approach is selected because the repository already has stable native lowering patterns and a C runtime, while the interpreter supplies syntax and semantic reference behavior.

## Semantic decisions

- Native syntax remains the current native syntax: `fn`, `return`, `i32/i64`, parenthesized conditions, and `for (init; condition; step)`. New syntax is added only where specified below; interpreter-only spellings such as `method`, `release`, and `when` are not added to native parsing.
- Enums use a tagged native value: a discriminant plus fixed-width payload storage. `match` lowers to discriminant tests and payload bindings; exhaustiveness is checked statically.
- The first enum subset supports `Enum::Variant` and `match` with variant patterns, wildcard patterns, and expression arms. Literal/or/guard/block patterns are deferred until this representation is stable. The tag is `i32`; payload storage is an aligned fixed-size byte buffer for the largest non-recursive payload. Recursive enum payloads are rejected with a located diagnostic in the first milestone.
- Methods use `fn` declarations with an explicit first receiver parameter and `receiver.method(...)` call syntax. Resolution is static by receiver named type; builtin collection helpers remain ordinary functions. Method mutation follows existing assignment rules and requires a mutable receiver binding.
- Collection iteration uses `for (value in list)` and `for (key, value in map)` in native syntax. A single map binding is the key. String and bytes iteration are deferred. Map iteration is sorted lexically by key to match interpreter semantics; loop bindings are immutable unless explicitly rebound by an assignment-supported form.
- Interpolation uses `{expression}` inside strings, with `{{` and `}}` for literal braces. The first native subset accepts string, integer, float, bool, null, and bytes operands and formats them with the existing Xenith textual conventions; lists/maps/structs/enums/functions are rejected statically.
- Function values start with non-capturing named functions and typed indirect calls. Capturing closures are then supported for copy-captured scalar/immutable values only; mutable heap collections, recursive closures, and function equality are rejected until an ownership representation exists.
- Error semantics initially means explicit `Result`-like tagged values are out of scope for native syntax. Existing runtime failures remain traps. The ownership milestone establishes arena-to-program-exit lifetime and removes unsafe append/reallocation aliasing before any future result propagation design.
- Imports retain current module flattening, but exported enums, methods, and signatures are carried through the import graph and checked before code generation.
- Diagnostics gain source spans at the parser/checker boundary and preserve module paths for imported failures.

## Milestones

1. Enums, variants, typed payloads, exhaustive variant/wildcard `match`.
2. Methods and sorted collection iteration.
3. String interpolation and formatting runtime helpers.
4. Function values and non-capturing closures, then captured closures.
5. Explicit error/result values and ownership checks for heap-backed values.
6. Import-aware enum/function/method checking and richer multi-span diagnostics.

Each milestone must add a failing test, implement the minimum behavior, run the full test suite, and push a focused commit.

## Review focus

- Matching a variant with the wrong payload arity or type must fail statically.
- Non-exhaustive matches must fail, including imported enums.
- Iterating empty collections must execute zero body iterations without invalid access.
- Captured scalar variables must retain copy behavior after the outer scope exits.
- Interpolation must reject unsupported types with a located diagnostic; runtime traps remain explicit until a result type is designed.
