# The Type System

Everything in Xenith has a type, and the type is checked. This page collects the
rules that the earlier pages showed one at a time.

## The types

| Type | Values |
| --- | --- |
| `int` | 64 bit signed integers |
| `float` | 64 bit floating point |
| `string` | UTF-8 text |
| `bool` | `true` and `false` |
| `null` | only `null` |
| `list<T>` | any number of `T` |
| `map<K, V>` | string keys, `V` values |
| `(A, B, ...)` | a tuple of fixed size |
| a struct name | a record you declared |
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

Type checks run while the program executes, at the moment each operation is
reached. So an error in a branch that never runs is never reported:

```xenith
let n: int = 1

when n > 100 {
    let broken: int = "not an int"
}

echo("finished")
```

```
finished
```

That is the significant limitation of the current implementation. A static
checking pass that walks the whole program before it starts is the planned fix.
Until then, run your code to type check it, and lean on the editor for the parts
it can see.

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

## What is not checked yet

Return values are not checked against the declared result type. A method that
says `-> int` and releases a string will do so:

```xenith
method wrong() -> int {
    release "not an int"
}

echo(wrong())
```

```
not an int
```

Treat the return type as documentation for now. It is a real gap and it is on the
list.

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

The conversions that exist are between `int`, `float`, `string` and `bool`. There
is no conversion to or from lists, maps, tuples or structs.

## Type aliases

`type` gives an existing type another name:

```xenith
type Celsius = float
type Scores = list<int>
type Lookup = map<string, int>
type IntFn = method(int) -> int

let temp: Celsius = 21.5
let marks: Scores = [88, 92]

echo("{temp} {ret(marks)}")
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

Next: [Modules](14-modules.md)
