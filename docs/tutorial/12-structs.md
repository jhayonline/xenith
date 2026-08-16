# Structs

A struct is a record: named fields with types, and nothing else. No methods, no
inheritance, no interfaces, no hidden behaviour. What the definition says is what
the value contains.

```xenith
struct Point {
    x: int,
    y: int
}

let origin: Point = Point { x: 0, y: 0 }

echo("({origin.x}, {origin.y})")
```

```
(0, 0)
```

## Defining

Fields are `name: type`, separated by commas. The trailing comma is optional.

```xenith
struct Person {
    name: string,
    age: int,
    active: bool
}

let ada: Person = Person {
    name: "Ada",
    age: 36,
    active: true
}

echo("{ada.name}, {ada.age}, active={ada.active}")
```

```
Ada, 36, active=true
```

Struct names conventionally start with a capital. Nothing enforces it, but the
editor colours capitalised names as types.

## Creating a value

Write the struct name followed by every field in braces. All fields are required
and there are no defaults:

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

An unknown field is caught too:

```xenith
struct Point {
    x: int,
    y: int
}

let p: Point = Point { x: 1, y: 2, z: 3 }
```

```
error XEN009: Field Not Found
  field `z` not found for struct `Point`
```

And so is a value of the wrong type:

```xenith
struct Point {
    x: int,
    y: int
}

let p: Point = Point { x: "one", y: 2 }
```

```
error XEN001: Type Mismatch
  expected `int`, found `string`
```

Field order in the literal does not have to match the definition.

Field names cannot be keywords. `from`, `type`, `in` and the rest of the
[reserved words](02-variables.md) are unavailable, so a field holding a source
position wants a name like `start` rather than `from`.

## Reading and writing fields

Dot notation both ways:

```xenith
struct Counter {
    label: string,
    value: int
}

let c: Counter = Counter { label: "hits", value: 0 }

c.value = c.value + 1
c.value = c.value + 1

echo("{c.label}: {c.value}")
```

```
hits: 2
```

Fields nest, and so does assignment to them. A field can be reached through
another field, through a list index, or through a map key, to any depth:

```xenith
struct Point {
    x: int,
    y: int
}

struct Line {
    start: Point,
    end: Point
}

let path: list<Line> = [
    Line { start: Point { x: 0, y: 0 }, end: Point { x: 1, y: 1 } }
]

path[0].end.x = 9
echo("{path[0].end.x}")
```

```
9
```

A list held in a field behaves like any other list:

```xenith
struct Basket {
    items: list<string>
}

let b: Basket = Basket { items: ["apple"] }

b.items.append("pear")
b.items[0] = "plum"

echo("{b.items}")
```

```
[plum, pear]
```

Asking for a field that does not exist is an error:

```xenith
struct Point {
    x: int,
    y: int
}

let p: Point = Point { x: 1, y: 2 }
echo("{p.z}")
```

```
error XEN009: Field Not Found
```

## Behaviour lives in methods

Because a struct holds no code, anything that operates on one is an ordinary
method that takes it as an argument:

```xenith
struct Rectangle {
    width: int,
    height: int
}

method area(r: Rectangle) -> int {
    release r.width * r.height
}

method perimeter(r: Rectangle) -> int {
    release 2 * (r.width + r.height)
}

method describe(r: Rectangle) -> string {
    release "{r.width}x{r.height}, area {area(r)}"
}

let box: Rectangle = Rectangle { width: 4, height: 3 }

echo("{area(box)}")
echo("{perimeter(box)}")
echo(describe(box))
```

```
12
14
4x3, area 12
```

This is the Go style rather than the Rust or Java style, and it is deliberate.
Everything that touches a `Rectangle` is a top level name you can grep for. There
is no separate place where methods hide, and no question about what a value can
do beyond what its fields say.

## Nesting

A struct field can be another struct:

```xenith
struct Point {
    x: int,
    y: int
}

struct Line {
    start: Point,
    end: Point
}

let diagonal: Line = Line {
    start: Point { x: 0, y: 0 },
    end: Point { x: 4, y: 3 }
}

echo("start ({diagonal.start.x}, {diagonal.start.y})")
echo("end ({diagonal.end.x}, {diagonal.end.y})")
```

```
start (0, 0)
end (4, 3)
```

## Collections of structs

A list of records is the usual way to hold a table of data:

```xenith
struct Task {
    title: string,
    done: bool
}

let tasks: list<Task> = [
    Task { title: "write docs", done: true },
    Task { title: "add tests", done: false },
    Task { title: "ship", done: false }
]

let remaining: int = 0

for task in tasks {
    when task.done {
        echo("[x] {task.title}")
    } otherwise {
        echo("[ ] {task.title}")
        remaining = remaining + 1
    }
}

echo("{remaining} left")
```

```
[x] write docs
[ ] add tests
[ ] ship
2 left
```

## Structs are copied

Assigning a struct copies it. Changing the copy leaves the original alone:

```xenith
struct Point {
    x: int,
    y: int
}

let a: Point = Point { x: 1, y: 2 }
let b: Point = a

b.x = 99

echo("a.x = {a.x}")
echo("b.x = {b.x}")
```

```
a.x = 1
b.x = 99
```

The same applies when you pass one to a method: the method gets its own copy. If
a method needs to change a struct, have it return the new value:

```xenith
struct Point {
    x: int,
    y: int
}

method moved_right(p: Point, by: int) -> Point {
    release Point { x: p.x + by, y: p.y }
}

let start: Point = Point { x: 0, y: 0 }
let moved: Point = moved_right(start, 5)

echo("start {start.x}, moved {moved.x}")
```

```
start 0, moved 5
```

## Structs and modules

`export` does not currently work on a struct definition. A module can export
methods and `let` bindings only, so a struct that is needed in two files has to
be defined in each. See [Modules](15-modules.md).

Next: [Enums and match](13-enums.md)
