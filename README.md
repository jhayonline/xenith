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
[Editor setup](docs/tutorial/18-editor-setup.md).

## Documentation

- **[Tutorial](docs/tutorial/README.md)** to learn the language. Twenty short
  pages covering all of it.
- **[Internals](docs/internals/README.md)** to work on the interpreter. Twelve
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
ages.remove("ada")

let raw: bytes = "hello" as bytes             # raw bytes, no encoding assumed

let (quotient, remainder) = (17 / 5, 17 % 5)   # tuples

method add(a: int, b: int) -> int {
    release a + b
}

method square(n: int) -> int => n * n          # single expression form

enum Shape {                                   # one of several, each able to
    Circle(float),                             # carry values of its own
    Rect(float, float),
    Empty
}

let label: string = match Shape::Circle(2.0) { # an expression, and checked for
    Shape::Circle(r) when r > 10.0 => "big"    # completeness
    Shape::Circle(r) => "circle of {r}"
    Shape::Rect(w, h) => "{w} by {h}"
    Shape::Empty => "nothing"
}

type Celsius = float                           # alias an existing type
```

## Where it stands

Xenith works and is worth writing small programs in. It is not finished.

**Solid:** the number semantics, structs and collections, enums with pattern
matching and completeness checking, the four loop forms, methods including
closures and passing them as values, modules, a static checker that reports every
type error before anything runs, and diagnostics that point at the right code and
suggest a fix.

**Missing:** the standard library covers strings, maths, files, bytes and the
environment, all written in Xenith. Time and randomness are not there, there are
no generics so no `Result<T>` or `Option<T>`, and the static pass does not follow
imports, so an imported name is checked as it runs rather than before.

The full list, with what to do instead in each case, is
[Known limitations](docs/tutorial/19-limitations.md).

## Performance

A tree walking interpreter, so not fast in absolute terms, but nothing in the hot
path is doing invisible work. On an idle machine, naive `fib(25)` runs in roughly
a third of a second and a three million iteration counting loop in under two.

Getting there meant fixing nine things, almost none of them where profiling was
expected to point: a function body deep copied on every reference, the whole
source file copied with every position, a 392 byte struct returned from every
step, an interpolated string re-parsed on every evaluation, an allocation on
every assignment, a `Value` twice the size it needed to be, and every loop
quietly collecting a list of its own iterations.
[Performance](docs/internals/10-performance.md) describes each, since they are
all easy to reintroduce.

## The bytecode VM

The tree walker is finished, in the sense that there is nothing left to fix in
it. The profile that remains is 36% dispatch and 21% binary operators, variable
lookup does not appear at all, and allocation does not appear at all. That is
what a tree walker costs when nothing is being wasted.

So it is being replaced by a **register based bytecode VM with type specialised
opcodes**, in place, one phase at a time. The target is 5 to 15 times the
current throughput, measured by instruction count rather than wall clock -- a
loaded machine has produced two false regressions in this project's history.

The lever is that Xenith is statically typed. Python, Ruby, Lua and JavaScript
spend most of a VM's budget rediscovering types at run time; every `+`
dispatches on a pair of tags. Xenith does not have to. Where the checker knows
both operands are `int`, the compiler emits `ADD_I` instead. Where it does not
know, the compiler emits the generic opcode, which does exactly what the tree
walker does today -- so checker completeness turns into speed continuously,
rather than gating it.

### How it is being built without breaking anything

Three rules, and they are the reason a rewrite of this size is survivable:

- **Anything the compiler cannot handle runs on the tree walker.** Returning
  "unsupported" is never a failure. Compiling to something that behaves
  differently is.
- **Every fixture runs through both engines and is compared byte for byte** --
  stdout, stderr and exit status. That harness has run on every commit since
  phase 3, not at the end.
- **A disassembler was written before the VM loop.** Register allocation bugs
  produce wrong answers rather than crashes, so `xenith --dump-bytecode
  file.xen` exists to make them visible.

The VM is opt in while it is built: `XENITH_VM=1 xenith program.xen`. Without
it, nothing has changed.

### The phases

| | | |
| --- | --- | --- |
| 0 | Shrink `Value` to 16 bytes, node ids, the checker emits a type table | done |
| 1 | `main` as an entry point; program mode and script mode | done |
| 2 | Whole program front end, cross module type checking | done |
| 3 | Chunks, opcodes, register allocation, the disassembler, the VM loop, the differential harness | done |
| 4 | Frames, `CALL` and `RET`, upvalues, `release` | done |
| 5 | Typed opcodes, wired to the checker's table | **in progress** |
| 6 | Structs with indexed fields, tagged enums, `match` jump tables, lists, maps, indexing | not started |
| 7 | Modules, builtins, string interpolation, the standard library precompiled into the binary | not started |
| 8 | The REPL onto the VM, and `src/interpreter.rs` deleted | not started |

### Where it has got to

Phases 0 to 4 are merged. Measured by callgrind instruction count on one
machine and one build:

| | tree walker | VM | |
| --- | --- | --- | --- |
| `fib(25)`, 242,785 calls | 1,391,087,959 | 248,783,608 | 5.59x |
| 400,000 iteration counting loop | 1,181,300,349 | 289,255,472 | 4.08x |

`fib(25)` also runs in 36.9 ms against 273.0 ms, which meets the one success
criterion stated in milliseconds rather than instructions.

The counting loop is the honest number: phase 3 reached 4.45x and **phase 4
gave 8.9% of it back**, because frames made every register access take a base
offset and every instruction fetch walk an `Rc`. That was measured, attributed
to two functions, written down and left, rather than papered over.

Phase 5 is under way and has not been measured yet. What can be said without
measuring is what the counting loop now compiles to. Before, nine instructions
a pass:

```
LOAD_CONST    r2, k1        the bound, reloaded every pass
LT            r2, r0, r2
JUMP_IF_FALSE r2, @0011
ADD           r2, r1, r0
MOVE          r1, r2        total = ...
LOAD_CONST    r2, k2        the step, reloaded every pass
ADD           r2, r0, r2
MOVE          r0, r2        i = ...
JUMP          @0002
```

After, five:

```
LT_IK         r2, r0, k1    the bound read where it already lives
JUMP_IF_FALSE r2, @0007
ADD_I         r1, r1, r0    written straight into total
ADD_IK        r0, r0, k2    and into i
JUMP          @0002
```

Four separate changes, and only the first is what the phase is named after: the
narrowed opcodes, a constant right operand read where it already lives instead
of being loaded into a register, an assignment written straight into its own
variable, and an error that now travels behind a pointer -- every operation in
`src/values.rs` used to return 240 bytes to carry a 16 byte answer.

### What is left

In phase 5: the interpreter loop still reaches its code through an `Rc` on
every dispatch, which is where phase 4's regression went and which phase 4's
notes deferred to here. Then a differential fixture that exercises the narrow
paths and the wide ones together, and the callgrind run that says whether any
of this worked.

After phase 5, the three phases that have not started. Phase 6 is the one that
matters most for real programs: a struct field is a hashed string lookup today,
and `match` compares variant names as strings.

The whole of it is written up in
[Performance](docs/internals/10-performance.md), phase by phase, including the
things that did not work.

## Contributing

```sh
cargo test
```

The suite is driven by fixtures under `tests/`, so adding a test means adding a
`.xen` file and its expected output rather than writing Rust. See
[Contributing](docs/internals/12-contributing.md), which also lists what is worth
working on. The short version: more of the standard library, and a decision about
generics before collections can be written.

## License

MIT. See [LICENSE](LICENSE).
