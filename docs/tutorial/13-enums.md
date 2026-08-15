# Enums and match

A struct is every one of its fields at once. An enum is exactly one of its
variants, and each variant can carry values of its own.

```xenith
enum Shape {
    Circle(float),
    Rect(float, float),
    Empty
}

let circle: Shape = Shape::Circle(2.0)
let rect: Shape = Shape::Rect(3.0, 4.0)
let nothing: Shape = Shape::Empty

echo("{circle}")
```

```
Shape::Circle(2.0)
```

A variant is always written with its enum in front, so nothing is added to the
surrounding namespace and a reader never has to guess where `Circle` came from.
An enum prints as the text that would have built it, which is what makes one
useful to `echo` while you are working.

## Why they exist

Because a tuple cannot say that only one thing is there.

```xenith
# Both halves always exist, and nothing stops a caller reading the number
# after the flag came back false.
method find(key: string) -> (int, bool) { ... }
```

An enum makes the absent case genuinely absent:

```xenith
enum Lookup {
    Found(int),
    Missing
}
```

There is no number to read on the `Missing` side. The only way to get at one is
to say which case you are in.

## match

`match` picks a branch by shape rather than by condition. It is an expression,
so it can stand on the right of a `let` or be released directly:

```xenith
method area(s: Shape) -> float {
    release match s {
        Shape::Circle(r) => MATH_PI * r * r
        Shape::Rect(w, h) => w * h
        Shape::Empty => 0.0
    }
}

echo("{area(Shape::Rect(3.0, 4.0))}")
```

```
12.0
```

The names in a pattern -- `r`, `w`, `h` -- bind whatever that variant is
carrying, and are visible only inside their own arm.

Arms are separated by a newline. A comma between them is allowed and does
nothing.

## Completeness is checked

This is the reason enums are worth having. A match that leaves a case out does
not run:

```xenith
let label: string = match circle {
    Shape::Circle(r) => "circle"
    Shape::Empty => "nothing"
}
```

```
error XEN022: Match Not Exhaustive
  this match does not cover `Rect`
  💡 add the missing arms, or a `_` arm for everything else
```

So adding a variant to an enum turns every match with a hole in it into an
error that names the file and line. That is the property worth designing for:
the compiler walks you round the codebase instead of you remembering to.

A `bool` is checked the same way, because it also has a complete set of cases:

```xenith
method yes_no(b: bool) -> string {
    release match b {
        true => "yes"
        false => "no"
    }
}
```

An `int` or a `string` has no finite set of cases, so a match on one always
needs a catch-all.

## Patterns

| Pattern | Matches |
| --- | --- |
| `_` | anything, binding nothing |
| `name` | anything, binding it to `name` |
| `0`, `"GET"`, `true`, `null` | that exact value |
| `-1` | a negative number |
| `Shape::Empty` | that variant |
| `Shape::Circle(r)` | that variant, binding what it carries |
| `Shape::Circle(2.0)` | that variant carrying exactly that |
| `(a, b)` | a two element tuple |
| `A \| B` | either of them |

They nest, so a pattern can reach into a variant and into a tuple inside it:

```xenith
enum Point {
    At((int, int)),
    Nowhere
}

method where_is(p: Point) -> string {
    release match p {
        Point::At((0, 0)) => "the origin"
        Point::At((x, 0)) => "on the x axis at {x}"
        Point::At((x, y)) => "({x}, {y})"
        Point::Nowhere => "nowhere"
    }
}

echo(where_is(Point::At((5, 0))))
```

```
on the x axis at 5
```

Order matters. Arms are tried top to bottom, so put the specific ones first; a
`_` at the top would swallow everything.

## Guards

`when` on an arm adds a condition, which is tried only once the pattern already
matched. The pattern's bindings are visible to it:

```xenith
method size_of(s: Shape) -> string {
    release match s {
        Shape::Circle(r) when r > 10.0 => "a big circle"
        Shape::Circle(r) => "a circle of {r}"
        Shape::Rect(w, h) when w == h => "a square"
        Shape::Rect(w, h) => "{w} by {h}"
        Shape::Empty => "nothing"
    }
}
```

A guarded arm never counts towards completeness. Whether it matches depends on a
value, and the checker will not pretend to know:

```xenith
enum Answer { Yes, No }

let text: string = match answer {
    Answer::Yes when ready => "yes"
    Answer::No => "no"
}
```

```
error XEN022: Match Not Exhaustive
  this match does not cover `Yes`
```

The fix is an unguarded `Answer::Yes` arm after the guarded one, which is also
the code you wanted: what happens when it is `Yes` and you are not ready.

## Arms agree about their type

A match is an expression, so every arm has to produce the same kind of thing:

```xenith
let value = match answer {
    Answer::Yes => 1
    Answer::No => "two"
}
```

```
error XEN001: Type Mismatch
  expected `int`, found `string`
```

An arm can be a block when one expression is not enough. Its value is its last
statement:

```xenith
method describe(n: int) -> string {
    release match n {
        0 => "zero"
        1 | 2 | 3 => "small"
        other => {
            let doubled: int = other * 2
            "{other}, which doubles to {doubled}"
        }
    }
}

echo(describe(7))
```

```
7, which doubles to 14
```

A `{` after `=>` always opens a block, never a map literal. Wrap a map in
parentheses if you need one there.

## Comparing

`==` compares the variant and everything it carries:

```xenith
echo("{Shape::Circle(2.0) == Shape::Circle(2.0)}")
echo("{Shape::Circle(2.0) == Shape::Circle(9.0)}")
echo("{Shape::Circle(2.0) == Shape::Empty}")
```

```
true
false
false
```

## They can refer to themselves

A variant may carry the enum it belongs to, which is how a tree is described:

```xenith
enum Json {
    Null,
    Bool(bool),
    Number(float),
    Text(string),
    Array(list<Json>),
    Object(map<string, Json>)
}
```

That shape has no other spelling in Xenith. A `map<string, ...>` holds one type
of value, and a JSON object's values are not all the same type -- an enum is
what lets the one type be "any of these".

## Exporting

`export` in front of an enum makes it visible to other files, exactly as for a
struct:

```xenith
# status.xen
export enum Status {
    Ok(int),
    Failed(int, string)
}
```

```xenith
grab { Status } from "status"

let s: Status = Status::Ok(200)
```

An enum cannot be renamed on the way in, for the same reason a struct cannot:
it is identified by its name. See [Modules](15-modules.md).

## What is not here

- **No generics**, so there is no `Result<T>` or `Option<T>`. A concrete enum
  per case works today; a container that carries any type does not.
- **No struct patterns**. `Point { x, y }` does not parse; read the fields with
  `p.x` after matching.
- **No list patterns.** There is no `[first, ..rest]`.
- **No ranges** in patterns, and no `@` bindings.

See [Known limitations](19-limitations.md).

Next: [The type system](14-types.md)
