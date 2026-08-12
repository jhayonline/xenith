# Booleans and Operators

## bool

A `bool` is `true` or `false`. Nothing else is a boolean, and nothing else is
accepted where one is expected.

```xenith
let ready: bool = true
let done: bool = false

echo("{ready} {done}")
```

```
true false
```

## Comparison

| Operator | Meaning |
| --- | --- |
| `==` | equal |
| `!=` | not equal |
| `<` | less than |
| `>` | greater than |
| `<=` | less than or equal |
| `>=` | greater than or equal |

```xenith
let a: int = 7
let b: int = 2

echo("{a == b}")
echo("{a != b}")
echo("{a > b}")
echo("{a <= 7}")
```

```
false
true
true
true
```

Comparison always produces a `bool`. It never produces 1 or 0.

Both sides have to be the same type. Comparing an `int` to a `float` is the same
type error as adding them, for the same reason.

## Logical operators

| Operator | Meaning |
| --- | --- |
| `&&` | and |
| `\|\|` | or |
| `!` | not |

```xenith
let temperature: int = 18

echo("{temperature > 15 && temperature < 25}")
echo("{temperature < 0 || temperature > 30}")
echo("{!(temperature > 30)}")
```

```
true
false
true
```

`&&` and `||` short circuit: if the left side settles the answer, the right side
is never evaluated at all. That is what makes a guard work.

```xenith
let text: string = "abc   "
let last: int = text.len()

while last > 0 && text[last - 1] == " " {
    last = last - 1
}

echo("{last}")
```

```
3
```

When `last` reaches 0 the left side is false, so `text[last - 1]` is never
reached. Without short circuiting that would index position -1.

## Precedence

From loosest to tightest:

```
||
&&
==  !=  <  >  <=  >=
+   -
*   /   %
as
^
unary -  !
```

Which means all of these read the way they look:

```xenith
let n: int = 18

echo("{n > 15 && n < 25}")
echo("{1 + 2 * 3 == 7}")
echo("{n as float / 2.0}")
```

```
true
true
9.0
```

Parentheses group when you want something else, and they are worth adding
whenever a line takes a second read.

## The ternary

`condition ? a : b` chooses between two values:

```xenith
let temperature: int = 18
let advice: string = temperature > 20 ? "shorts" : "coat"

echo(advice)
```

```
coat
```

Use the ternary when you want a value. Use `when` when you want to run
different code. Nesting ternaries is legal and almost never worth it.

## Truthiness in conditions

`when` and `while` accept values that are not `bool`, treating `0`, `0.0`, `""`,
an empty list, an empty map and `null` as false and everything else as true.
[Control flow](06-control-flow.md) has the table.

Outside a condition there is no coercion: an `int` where a `bool` is declared is
still a type error.

## Built in boolean constants

`TRUE` and `FALSE` exist alongside the `true` and `false` literals and mean
exactly the same thing. The lowercase spellings are the ones to use.

```xenith
echo("{TRUE} {FALSE} {NULL}")
```

```
true false null
```

Next: [Control flow](06-control-flow.md)
