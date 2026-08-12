# Xenith Documentation

Two sets, depending on what you are here for.

## [Tutorial](tutorial/README.md)

Learning the language. Eighteen short pages covering everything Xenith can
currently do, in the order it makes sense to meet it. Start at
[Getting started](tutorial/01-getting-started.md).

If you already program in something else, the whole thing is an afternoon.

Worth reading early:

- [Numbers](tutorial/03-numbers.md), because the split between `int` and `float`
  is the decision most of the rest of the language follows from.
- [Known limitations](tutorial/18-limitations.md), because knowing what does not
  work yet saves an hour of finding out.

## [Internals](internals/README.md)

How the interpreter is built. Eleven pages on the lexer, parser, AST,
interpreter, values, errors, modules, performance and the language server, plus
[Contributing](internals/11-contributing.md) for what to work on and how.

Written for someone reading the codebase for the first time. It says why things
are the way they are, including the parts that are wrong and known to be.

## Everything else

- The `testies/` directory holds sample programs.
- `editors/nvim` is the Neovim plugin.
- `packaging/arch` has a PKGBUILD.
