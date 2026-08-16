# The Type System

Everything in Xenith has a type, and the type is checked. This page collects the
rules that the earlier pages showed one at a time.

## The types

| Type | Values |
| --- | --- |
| `int` | 64 bit signed integers |
| `float` | 64 bit floating point |
| `string` | UTF-8 text |
| `bytes` | raw bytes, no encoding assumed |
| `bool` | `true` and `false` |
| `null` | only `null` |
| `list<T>` | any number of `T` |
| `map<K, V>` | string keys, `V` values |
| `(A, B, ...)` | a tuple of fixed size |
| a struct name | a record you declared |
| an enum name | one of the variants you declared |
| `method(A, B) -> C` | a method value |

## Annotations and inference

An annotation is optional when the value makes the type obvious:

```xenith
let a = 5              # int
let b: int = 5         # the same thing, said out loud
```

Annotate when it helps a reader, or when you want the type enforced at that line
rather than wherever the value ends up. Both forms produce identical code.

Inference is local. It looks at the initialiser and nothing else; it does not
flow across statements or work out a type from later use.

## Where checking happens

Before anything runs. A static pass walks the whole file first, so an error in a
branch that never executes is still reported, and no output appears before it:

```xenith
let n: int = 1

when n > 100 {
    let broken: int = "not an int"
}

echo("finished")
```

```
error XEN001: Type Mismatch
  expected `int`, found `string`
```

Every error in the file is reported at once, rather than one per run:

```xenith
let a: int = "one"
let b: string = 2
method f(n: int) -> int => n
let c = f(1, 2)
```

```
error XEN001: Type Mismatch
```

That program reports three errors and a count, then stops without running.

The checker is deliberately cautious. Where it cannot work out a type it says
nothing rather than guessing, so a reported error is a real one. The interpreter
still checks as it runs, so anything the static pass misses is caught at the
point it happens.

## What is checked

Declarations, against the annotation:

```xenith
let n: int = "five"
```

```
error XEN001: Type Mismatch
  expected `int`, found `string`
```

Reassignment, against the original declaration:

```xenith
let n: int = 1
n = "two"
```

```
error XEN001: Type Mismatch
```

Arithmetic, which refuses mixed operands:

```xenith
echo("{1 + 2.0}")
```

```
error XEN001: Type Mismatch
  cannot add int and float
```

Method arguments, by count and by type:

```xenith
method double(n: int) -> int => n * 2

echo("{double("five")}")
```

```
error XEN001: Type Mismatch
  expected `int`, found `string`
```

Struct literals, for missing fields, unknown fields and field types:

```xenith
struct Point {
    x: int,
    y: int
}

let p: Point = Point { x: 1 }
```

```
error XEN009: Missing Field
  struct `Point` is missing `y`
```

List elements, against the declared element type:

```xenith
let ns: list<int> = [1, 2, "three"]
```

```
error XEN001: Type Mismatch
```

Return values, against the declared result type:

```xenith
method wrong() -> int {
    release "not an int"
}

echo(wrong())
```

```
error XEN001: Type Mismatch
  expected `int`, found `string`
```

The short arrow form is checked the same way:

```xenith
method also_wrong(n: int) -> string => n * 2
```

```
error XEN001: Type Mismatch
  expected `string`, found `int`
```

## What is not checked

Built in functions. Most of them accept more than one type of argument in a way
the type system cannot yet describe, so calls to `len`, `append` and the rest are
not checked. Calls to methods you write are checked, by both count and type.

Interpolated expressions, index access and field access are all checked.

## Conversions

Nothing converts implicitly. `as` is the only way across:

```xenith
let n: int = 7

echo("{n as float}")
echo("{n as string}")
echo("{n as bool}")
```

```
7.0
7
true
```

The conversions that exist are between `int`, `float`, `string` and `bool`, and
between `string` and `bytes`. There is no conversion to or from lists, maps,
tuples or structs.

## bytes

A `string` is guaranteed to be valid UTF-8. A `bytes` is guaranteed nothing,
which is what makes it the right type for anything that came from outside the
program: a file that is not text, an image, a compressed stream.

```xenith
let raw: bytes = "hello" as bytes

echo("{len(raw)} bytes, first is {raw[0]}")
echo("{(raw + (" world" as bytes)) as string}")
```

```
5 bytes, first is 104
hello world
```

Indexing gives an `int` in 0 to 255 rather than a one byte `bytes`, because
almost everything done with a single byte is arithmetic or a comparison against
a code. `+` joins, `==` compares, and `len` counts bytes rather than characters:

```xenith
let accented: bytes = "héllo" as bytes

echo("{"héllo".len()} characters, {len(accented)} bytes")
```

```
5 characters, 6 bytes
```

Text always converts to bytes. Bytes do not always convert to text, and `as`
stops the program when they do not:

```xenith
let (raw, error) = bytes_from_list([255, 254])

let text: string = raw as string
```

```
error XEN011: Invalid Type Conversion
  bytes are not valid UTF-8 (at byte 0)
```

When that is a case to handle rather than a bug, `bytes_to_string` hands the
reason back instead of stopping:

```xenith
let (raw, build_error) = bytes_from_list([255, 254])

let (text, error) = bytes_to_string(raw)

echo("[{error}]")
```

```
[not valid UTF-8 (at byte 0)]
```

`echo` prints how many bytes there are rather than the bytes themselves, which
would spray control characters at the terminal. See
[The standard library](20-standard-library.md) for `std::bytes`.

## Type aliases

`type` gives an existing type another name:

```xenith
type Celsius = float
type Scores = list<int>
type Lookup = map<string, int>
type IntFn = method(int) -> int

let temp: Celsius = 21.5
let marks: Scores = [88, 92]

echo("{temp} {marks}")
```

```
21.5 [88, 92]
```

An alias is a second name for the same type, not a new one. A `Celsius` and a
plain `float` are interchangeable, so this does not stop you adding a temperature
to a length. What it does buy you is a signature that says what it means:

```xenith
type Celsius = float

method is_freezing(t: Celsius) -> bool => t <= 0.0

echo("{is_freezing(-4.0)}")
```

```
true
```

Aliases are most useful for method types, where writing
`method(list<int>, int) -> list<int>` twice in a signature is unpleasant.

## Null

`null` is its own type with one value. It is not a member of every other type, so
an `int` cannot be null and there is nothing to check for before using one.

```xenith
let nothing: null = null
echo("{nothing}")
```

```
null
```

Methods that do work rather than compute a value declare `-> null` and release
`null`.

There is no optional type yet. The pattern for "might not have a value" is a
tuple of the value and a flag, as shown in [Tuples](10-tuples.md).

Next: [Modules](15-modules.md)
