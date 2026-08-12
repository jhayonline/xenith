# Control Flow

Xenith spells if, else if and else as `when`, `or when` and `otherwise`.

## when

```xenith
let age: int = 20

when age >= 18 {
    echo("adult")
}
```

```
adult
```

Braces are required. There is no form that takes a single statement without
them, so a `when` can never accidentally cover only the first line of what you
meant.

## or when, otherwise

```xenith
let score: int = 74

when score >= 90 {
    echo("A")
} or when score >= 80 {
    echo("B")
} or when score >= 70 {
    echo("C")
} otherwise {
    echo("F")
}
```

```
C
```

Branches are tested top to bottom and the first match wins. `otherwise` runs when
nothing matched, and is optional.

`or when` and `otherwise` must sit on the same line as the closing brace of the
branch before them. Starting them on a new line does not parse:

```xenith
let n: int = 5

when n < 0 {
    echo("negative")
}
otherwise {
    echo("positive")
}
```

```
error XEN013: Unexpected Token
```

Keep them attached to the brace:

```xenith
let n: int = 5

when n < 0 {
    echo("negative")
} otherwise {
    echo("positive")
}
```

```
positive
```

## Truthiness

A condition does not have to be a `bool`. Other values count as true or false by
these rules:

| Value | Counts as |
| --- | --- |
| `false`, `0`, `0.0` | false |
| `""` | false |
| `[]`, `{}` | false |
| `null` | false |
| anything else | true |

```xenith
when 3 {
    echo("a non-zero int is true")
}

when "" {
    echo("never runs")
} otherwise {
    echo("an empty string is false")
}
```

```
a non-zero int is true
an empty string is false
```

It works, but writing the comparison out reads better and says what you actually
mean:

```xenith
let items: list<int> = [1, 2, 3]

when items.len() > 0 {
    echo("has items")
}
```

```
has items
```

This is the one place the language is looser than the rest of it would suggest.
Everywhere else, an `int` used where a `bool` belongs is an error.

## Blocks have their own scope

A variable declared inside a branch is gone at the closing brace:

```xenith
when true {
    let temp: int = 42
    echo("inside: {temp}")
}
```

```
inside: 42
```

Referring to `temp` after that block is an XEN002. Assignment still reaches
outward, so this works and is the usual pattern:

```xenith
let label: string = "unset"

when 5 > 3 {
    label = "bigger"
}

echo(label)
```

```
bigger
```

## Combining conditions

```xenith
let hour: int = 14
let is_weekend: bool = false

when hour > 9 && hour < 17 && !is_weekend {
    echo("working hours")
} otherwise {
    echo("off the clock")
}
```

```
working hours
```

## The ternary, again

For choosing a value rather than a path, the ternary is shorter:

```xenith
let n: int = 7

let parity: string = n % 2 == 0 ? "even" : "odd"
echo(parity)
```

```
odd
```

Both are fine. Pick whichever makes the line easier to read.

## What there is not

There is no `match` or `switch`. A chain of `or when` covers the same ground, and
leaving pattern matching out keeps the language small. If you find yourself
writing a long chain over the same value, that is usually a sign the values want
to be a map.

Next: [Loops](07-loops.md)
