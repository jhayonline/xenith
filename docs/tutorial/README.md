# The Xenith Tutorial

Xenith is a small statically typed language with C style syntax. It takes the
data oriented structs of Go, the strictness about numbers of Rust and Zig, the
familiar shape of C, and the readable type annotations of TypeScript, and leaves
out most of the rest.

The whole language is about thirty keywords. These pages cover all of it.

## Read in order

| | Page | Covers |
| --- | --- | --- |
| 1 | [Getting started](01-getting-started.md) | Installing, running a file, the REPL, comments |
| 2 | [Variables](02-variables.md) | `let`, `const let`, inference, scope, shadowing |
| 3 | [Numbers](03-numbers.md) | `int` and `float`, arithmetic, overflow, `as` |
| 4 | [Strings](04-strings.md) | Interpolation, escapes, raw strings |
| 5 | [Booleans and operators](05-booleans-and-operators.md) | Comparison, logic, precedence, the ternary |
| 6 | [Control flow](06-control-flow.md) | `when`, `or when`, `otherwise` |
| 7 | [Loops](07-loops.md) | The counting loop, `for in`, `while`, `skip`, `stop` |
| 8 | [Lists](08-lists.md) | Literals, indexing, the list methods |
| 9 | [Maps](09-maps.md) | Literals, keys, the map methods, ordering |
| 10 | [Tuples](10-tuples.md) | Destructuring, returning several values |
| 11 | [Methods](11-methods.md) | Parameters, `release`, methods as values, recursion |
| 12 | [Structs](12-structs.md) | Records, fields, behaviour in free methods |
| 13 | [Enums and match](13-enums.md) | Variants that carry values, patterns, completeness |
| 14 | [The type system](14-types.md) | What is checked, aliases, conversions |
| 15 | [Modules](15-modules.md) | `export`, `grab`, module resolution |
| 16 | [Built in functions](16-builtins.md) | Everything available without an import |
| 17 | [Errors](17-errors.md) | Reading diagnostics, error codes, `panic` |
| 18 | [Editor setup](18-editor-setup.md) | The language server and the Neovim plugin |
| 19 | [Known limitations](19-limitations.md) | What does not work yet, and why |
| 20 | [The standard library](20-standard-library.md) | `std::string`, `std::bytes`, `std::env`, and how they are built |

## The whole language on one page

```xenith
# Comments start with a hash.

# Variables. The type is optional when the value makes it obvious.
let name: string = "Ada"
let age = 36
const let LIMIT: int = 100

# Two number types that never mix without you saying so.
let count: int = 7
let ratio: float = 1.5
let total: float = (count as float) * ratio

# Strings interpolate.
echo("{name} is {age}")

# Branching.
when age >= 18 {
    echo("adult")
} or when age >= 13 {
    echo("teenager")
} otherwise {
    echo("child")
}

# A value rather than a branch.
let label: string = age >= 18 ? "adult" : "minor"

# Four loop shapes.
for (let i: int = 0; i < 3; i++) { echo("{i}") }
for item in [1, 2, 3] { echo("{item}") }
for key, value in {"a": 1} { echo("{key}={value}") }
while count > 0 { count = count - 1 }

# Lists and maps.
let numbers: list<int> = [1, 2, 3]
numbers.append(4)
numbers[0] = 99

let ages: map<string, int> = {"ada": 36}
ages["alan"] = 41

# Tuples, for returning more than one thing.
let (quotient, remainder) = (17 / 5, 17 % 5)

# Methods.
method add(a: int, b: int) -> int {
    release a + b
}

method square(n: int) -> int => n * n

# Structs are plain data. Behaviour is a method that takes one.
struct Point {
    x: int,
    y: int
}

method distance_from_origin_squared(p: Point) -> int {
    release p.x * p.x + p.y * p.y
}

let origin: Point = Point { x: 3, y: 4 }
echo("{distance_from_origin_squared(origin)}")

# Type aliases name an existing type.
type Celsius = float

# Errors stop the program.
when LIMIT < 0 {
    panic("limit cannot be negative")
}
```

```
Ada is 36
adult
0
1
2
1
2
3
a=1
25
```

## Also worth reading

- The `testies/` directory in the repository holds sample programs you can run
  and edit.
- [The internals documentation](../internals/README.md) if you want to work on
  the interpreter rather than in the language.
