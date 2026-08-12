# Xenith Internals

How the interpreter is built, for anyone working on it rather than in it.

If you want to learn the language, read [the tutorial](../tutorial/README.md)
instead.

## Contents

| | Page | Covers |
| --- | --- | --- |
| 1 | [Architecture](01-architecture.md) | The pipeline, the modules, the decisions behind them |
| 2 | [The lexer](02-lexer.md) | Characters to tokens, and the interpolation encoding |
| 3 | [The parser](03-parser.md) | Recursive descent, the precedence chain, backtracking |
| 4 | [The AST](04-ast.md) | Node types, positions, adding one |
| 5 | [The interpreter](05-interpreter.md) | Dispatch, contexts, scoping, assignment |
| 6 | [The static checker](06-checker.md) | Type errors reported before anything runs |
| 7 | [Values](07-values.md) | The value enum, number semantics, why everything is cloned |
| 8 | [Errors and diagnostics](08-errors.md) | The error type, codes, rendering, positions |
| 9 | [Modules](09-modules.md) | Resolution, loading, caching, exports |
| 10 | [Performance](10-performance.md) | What was slow, why, and how to measure |
| 11 | [The language server](11-language-server.md) | How the editor integration works |
| 12 | [Contributing](12-contributing.md) | Building, testing, adding things, what to work on |

## The short version

Xenith is a tree walking interpreter in about 13,900 lines of Rust with seven
dependencies. Source becomes tokens, tokens become a tree, the tree is executed
by recursive descent over it. There is no bytecode, no compilation and no
optimiser.

```
src/lexer.rs        characters  ->  tokens
src/parser.rs       tokens      ->  AST
src/checker.rs      AST         ->  type errors, before anything runs
src/interpreter.rs  AST         ->  values and effects
```

`src/lib.rs` runs the three in order. `src/main.rs` is the `xenith` binary,
`src/bin/xenith-lsp.rs` is the language server.

## Five things that will surprise you

**Values are copied, never shared.** There is no reference type. A method that
mutates its receiver has to hand back the new value, and the caller writes it
back to the variable it came from. `assign_into` in the interpreter exists
entirely to unwind this through nested containers.

**Names are resolved lexically.** A `Function` captures the context it was
defined in, and a call runs against a child of that. This is what gives the
language closures and lets a module's exports reach its private helpers. It also
makes a reference cycle for every named method, which `Rc` never frees; see
[The interpreter](05-interpreter.md).

**Symbol table reads clone.** `get` returns an owned `Value`. Anything reachable
from a `Value` therefore has to be cheap to clone, which is why `Function` holds
its body in an `Rc`.

**Every node carries two positions, and each holds the whole source.** They share
it through `Arc`, because owning it made every clone copy the file.

**A static pass runs before execution.** `src/checker.rs` reports type errors
before anything runs, including in branches that never execute, and reports all
of them at once. It is deliberately conservative: what it cannot work out becomes
`Type::Unknown` and is left alone, so a reported error is a real one. The
interpreter still checks as it runs; the pass sits in front of it rather than
replacing it. See [The static checker](06-checker.md).

## Where to start reading

To fix a bug in how a program behaves, start at
[The interpreter](05-interpreter.md).

To fix a bug in what parses, start at [The parser](03-parser.md), and check the
precedence chain first; several bugs have lived there.

To make it faster, read [Performance](10-performance.md) before changing
anything. The four fixes described there were all in places profiling was not
expected to point.

To work on the editor experience, [The language server](11-language-server.md).
