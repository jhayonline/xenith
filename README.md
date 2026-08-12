# Xenith

A small, statically typed language with C style syntax. Roughly thirty keywords,
no runtime surprises, and numbers that behave the way you expect.

```xenith
struct Runner {
    name: string,
    minutes: int
}

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

## What it takes from where

Xenith is an attempt to keep the good parts of four languages and leave out most
of the rest.

**From Go**, structs that are plain data. No methods on types, no inheritance, no
interfaces. Behaviour is free functions that take the data, so everything that
touches a type is a name you can search for.

**From Rust and Zig**, arithmetic that refuses to lie. `int` is a real 64 bit
integer, `float` is a real double, the two never mix without you saying so, and
integer overflow is an error rather than a silent wrap.

**From C**, the shape. Braces, semicolon separated `for` loops, familiar operator
precedence, nothing clever.

**From TypeScript**, type annotations that read easily and can be left off when
the value makes the type obvious.

## Installing

```sh
cargo install --path .
```

That builds two binaries into `~/.cargo/bin`:

- `xenith`, the interpreter. `xenith program.xen` runs a file, no arguments opens
  a REPL.
- `xenith-lsp`, the language server your editor talks to.

On Arch, `packaging/arch/PKGBUILD` installs the same two under pacman:

```sh
cd packaging/arch && makepkg -si
```

## Editor support

`editors/nvim` is a Neovim plugin with filetype detection, syntax highlighting,
indenting and the language server. With lazy.nvim:

```lua
{
  dir = "/path/to/xenith/editors/nvim",
  name = "xenith.nvim",
  lazy = false,
  config = function()
    require("xenith").setup()
  end,
}
```

The server gives diagnostics as you type, including type errors, plus hover, go
to definition, references, rename, document symbols and completion. Any LSP capable editor can use it; see
[Editor setup](docs/tutorial/17-editor-setup.md).

## Documentation

- **[Tutorial](docs/tutorial/README.md)** to learn the language. Eighteen short
  pages covering all of it.
- **[Internals](docs/internals/README.md)** to work on the interpreter. Eleven
  pages on how it is built and what to fix next.

## The language in one screen

```xenith
# Comments start with a hash.

let name: string = "Ada"        # the type is optional when it is obvious
let age = 36
const let LIMIT: int = 100      # cannot be reassigned

# int and float never mix without an explicit conversion.
let count: int = 7
let total: float = (count as float) * 1.5

echo("{name} is {age}")         # strings interpolate

when age >= 18 {                # if / else if / else
    echo("adult")
} or when age >= 13 {
    echo("teenager")
} otherwise {
    echo("child")
}

let label: string = age >= 18 ? "adult" : "minor"

for (let i: int = 0; i < 3; i++) { echo("{i}") }
for item in [1, 2, 3] { echo("{item}") }
for key, value in {"a": 1} { echo("{key}={value}") }
while count > 0 { count = count - 1 }

let numbers: list<int> = [1, 2, 3]
numbers.append(4)
numbers[0] = 99

let ages: map<string, int> = {"ada": 36}
ages["alan"] = 41

let (quotient, remainder) = (17 / 5, 17 % 5)   # tuples

method add(a: int, b: int) -> int {
    release a + b
}

method square(n: int) -> int => n * n          # single expression form

type Celsius = float                           # alias an existing type
```

## Where it stands

Xenith works and is worth writing small programs in. It is not finished.

**Solid:** the number semantics, structs and collections, the four loop forms,
methods including passing them as values, modules, and the diagnostics, which
point at the right code and suggest a fix.

**Missing:** there is no standard library, so no files, no networking, no JSON,
and strings have only `+`, `.len()` and comparison. There are no closures,
because names resolve against the caller's scope rather than the defining one,
which is also the main thing the static checker cannot see through.

The full list, with what to do instead in each case, is
[Known limitations](docs/tutorial/18-limitations.md).

## Performance

A tree walking interpreter, so not fast in absolute terms, but nothing in the hot
path is doing invisible work. On the machine this was written on, naive `fib(25)`
runs in about 355 ms and a three million iteration counting loop in about 1.8 s.

Getting there meant fixing four things, none of them where profiling was expected
to point. [Performance](docs/internals/10-performance.md) describes each, since
they are all easy to reintroduce.

## Contributing

```sh
cargo test
```

The suite is driven by fixtures under `tests/`, so adding a test means adding a
`.xen` file and its expected output rather than writing Rust. See
[Contributing](docs/internals/12-contributing.md), which also lists what is worth
working on. The short version: a static type checking pass is the most important
thing missing.

## License

MIT. See [LICENSE](LICENSE).
