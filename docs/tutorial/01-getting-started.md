# Getting Started

Xenith is a small, statically typed language with C style syntax. It reads like
Go, checks like Rust, and has about thirty keywords in total. You can learn the
whole thing in an afternoon.

This tutorial assumes you have written code before, in any language. It does not
assume you know Rust.

## Installing

Xenith is built from source with Cargo:

```sh
git clone https://github.com/jhayonline/xenith
cd xenith
cargo install --path .
```

That produces two binaries in `~/.cargo/bin`:

- `xenith`, the interpreter and REPL
- `xenith-lsp`, the language server your editor talks to

On Arch there is a `PKGBUILD` under `packaging/arch` if you would rather have
pacman own the install:

```sh
cd packaging/arch
makepkg -si
```

## Your first program

Create a file called `hello.xen`:

```xenith
echo("Hello, world!")
```

Run it:

```sh
xenith hello.xen
```

```
Hello, world!
```

Source files must end in `.xen`. The interpreter refuses anything else.

## The REPL

Running `xenith` with no arguments opens an interactive shell:

```
xenith
```

Type expressions and they are evaluated immediately. A few commands are built
in:

| Command | What it does |
| --- | --- |
| `:help` | List the available commands |
| `:vars` | Show everything currently defined |
| `:clear` | Clear the screen |
| `:load <file>` | Read a file into the session |
| `:exit` or `:quit` | Leave |

Ctrl+D and Ctrl+C also exit. Your history is kept in `~/.xenith_history` between
sessions.

## Comments

A `#` starts a comment that runs to the end of the line. There is no block
comment form.

```xenith
# This whole line is a comment.
let x: int = 5   # So is this, from the hash onwards.
```

## Statements and line endings

A newline ends a statement. You can write a semicolon instead if you want two
statements on one line, but you never have to:

```xenith
let a: int = 1
let b: int = 2

let c: int = 3; let d: int = 4
```

## What a program looks like

Here is a complete program that uses most of the ideas you will meet in the next
few pages:

```xenith
# A record type. Plain fields, nothing else.
struct Runner {
    name: string,
    minutes: int
}

# A method with typed parameters and a typed result.
method faster_of(a: Runner, b: Runner) -> Runner {
    when a.minutes < b.minutes {
        release a
    }
    release b
}

let ada: Runner = Runner { name: "Ada", minutes: 31 }
let alan: Runner = Runner { name: "Alan", minutes: 28 }

let winner: Runner = faster_of(ada, alan)
echo("{winner.name} won in {winner.minutes} minutes")
```

```
Alan won in 28 minutes
```

Three things to notice, because they run through everything else:

1. **Types are written down and they are checked.** `minutes: int` means an
   `int`, and putting a `float` there is an error at the point you do it.
2. **`release` returns a value.** Not `return`. The keyword set is deliberately
   its own.
3. **Braces are always required.** There is no single statement `when` without
   braces, so there is no dangling else to worry about.

## Where to go next

Read the pages in order. Each one is short and builds on the last.

1. [Variables](02-variables.md)
2. [Numbers](03-numbers.md)
3. [Strings](04-strings.md)
4. [Booleans and operators](05-booleans-and-operators.md)
5. [Control flow](06-control-flow.md)
6. [Loops](07-loops.md)
7. [Lists](08-lists.md)
8. [Maps](09-maps.md)
9. [Tuples](10-tuples.md)
10. [Methods](11-methods.md)
11. [Structs](12-structs.md)
12. [The type system](14-types.md)
13. [Modules](15-modules.md)
14. [Built in functions](16-builtins.md)
15. [Errors](17-errors.md)
16. [Editor setup](18-editor-setup.md)
17. [Known limitations](19-limitations.md)
