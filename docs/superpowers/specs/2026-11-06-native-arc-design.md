# Native ARC/ORC design

## Intent

Make heap-backed Xenith values deterministic and automatic for ordinary code.
Programs must not call `malloc` or `free`, and the native compiler must release
temporary and local collection values without changing primitive-value behavior.

## Scope of this slice

- Add a runtime reference-count header for lists, maps, and bytes.
- Expose retain/release operations through the native runtime ABI.
- Add compiler lifetime hooks at the first safe ownership boundaries: local
  bindings, returned values, closure captures, and function exits.
- Keep existing pointer layouts and public Xenith signatures unchanged.
- Add tests that exercise collection aliases, early returns, and closure captures.

## Non-goals

This slice does not add user-visible ownership annotations, weak-reference syntax,
cycle collection, or a complete escape/dataflow optimizer. Those follow after the
basic retain/release contract is measured and stable.

## Approaches

1. Rewrite the compiler around an ownership/dataflow IR. This gives the strongest
   eventual guarantees but is too disruptive while the native frontend is still
   expanding.
2. Add intrusive runtime reference counts and insert operations at known codegen
   boundaries. This preserves the current ABI and allows each value category to
   be migrated independently. This is the selected approach.
3. Use request/function arenas. This is easy to implement but does not provide
   deterministic per-value cleanup and is a poor fit for long-lived servers.

## Design

Every managed allocation starts with a hidden header containing a strong count,
object kind, and destructor metadata. The pointer exposed to Xenith remains the
payload pointer. Retain is a no-op for null and immortal/static values. Release
decrements the count and invokes the existing kind-specific destructor at zero.

Codegen calls retain when a managed pointer is copied into a second live slot and
release when a slot is overwritten, leaves a function, or is consumed by a return.
The first implementation is conservative: extra retains are acceptable, while a
release without a matching retain is not. Closure environments retain captured
managed values until the environment is destroyed.

## Builder work remaining

Implement runtime headers and tests first, then add codegen hooks one value class at
a time. Run the native collection, closure, and HTTP suites after each class.
