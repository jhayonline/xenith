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
| 6 | [Values](06-values.md) | The value enum, number semantics, why everything is cloned |
| 7 | [Errors and diagnostics](07-errors.md) | The error type, codes, rendering, positions |
| 8 | [Modules](08-modules.md) | Resolution, loading, caching, exports |
| 9 | [Performance](09-performance.md) | What was slow, why, and how to measure |
| 10 | [The language server](10-language-server.md) | How the editor integration works |
| 11 | [Contributing](11-contributing.md) | Building, testing, adding things, what to work on |

## The short version

Xenith is a tree walking interpreter in about 13,900 lines of Rust with seven
dependencies. Source becomes tokens, tokens become a tree, the tree is executed
by recursive descent over it. There is no bytecode, no compilation and no
optimiser.

```
src/lexer.rs        characters  ->  tokens
src/parser.rs       tokens      ->  AST
src/interpreter.rs  AST         ->  values and effects
```

`src/lib.rs` runs the three in order. `src/main.rs` is the `xenith` binary,
`src/bin/xenith-lsp.rs` is the language server.

## Five things that will surprise you

**Values are copied, never shared.** There is no reference type. A method that
mutates its receiver has to hand back the new value, and the caller writes it
back to the variable it came from. `assign_into` in the interpreter exists
entirely to unwind this through nested containers.

**Names are resolved dynamically.** A method body runs in a child of the
*caller's* context, not the one it was written in. That is why there are no
closures and why a module's exports cannot see its private helpers.

**Symbol table reads clone.** `get` returns an owned `Value`. Anything reachable
from a `Value` therefore has to be cheap to clone, which is why `Function` holds
its body in an `Rc`.

**Every node carries two positions, and each holds the whole source.** They share
it through `Arc`, because owning it made every clone copy the file.

**There is no type checking pass.** Types are checked as operations execute,
which means an error in a branch that never runs is never reported. This is the
largest missing piece; see [Contributing](11-contributing.md).

## Where to start reading

To fix a bug in how a program behaves, start at
[The interpreter](05-interpreter.md).

To fix a bug in what parses, start at [The parser](03-parser.md), and check the
precedence chain first; several bugs have lived there.

To make it faster, read [Performance](09-performance.md) before changing
anything. The four fixes described there were all in places profiling was not
expected to point.

To work on the editor experience, [The language server](10-language-server.md).
