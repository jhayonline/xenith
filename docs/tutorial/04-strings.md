# Strings

A string is a sequence of UTF-8 text written between double quotes.

```xenith
let name: string = "Ada Lovelace"
echo(name)
```

```
Ada Lovelace
```

## Interpolation

Braces inside a string hold an expression. It is evaluated and its value dropped
into place:

```xenith
let name: string = "Ada"
let age: int = 36

echo("{name} is {age}")
```

```
Ada is 36
```

Any expression works, not just a variable name:

```xenith
let items: list<int> = [1, 2, 3]

echo("{2 + 3} items, list has {items.len()}")
echo("{items.len() > 2 ? "several" : "few"}")
```

```
5 items, list has 3
several
```

Interpolation is the normal way to build text in Xenith. There is no separate
formatting function you need for the common case.

## Escapes

A backslash starts an escape sequence.

| Escape | Produces |
| --- | --- |
| `\n` | newline |
| `\t` | tab |
| `\"` | a double quote |
| `\\` | a backslash |
| `\{` | a literal brace |

```xenith
echo("first\nsecond")
echo("name:\tAda")
echo("she said \"hello\"")
echo("path: C:\\Users\\ada")
```

```
first
second
name:	Ada
she said "hello"
path: C:\Users\ada
```

## Literal braces

Doubling a brace prints one, which is how you write text that contains braces
without turning it into an expression:

```xenith
echo("a JSON object looks like {{\"key\": 1}}")
```

```
a JSON object looks like {"key": 1}
```

## Raw strings

A string in backticks is raw. No escape processing, no interpolation, everything
between the backticks arrives exactly as written:

```xenith
let path: string = `C:\Users\ada\Documents`
let pattern: string = `^\d{3}-\d{4}$`

echo(path)
echo(pattern)
```

```
C:\Users\ada\Documents
^\d{3}-\d{4}$
```

Reach for backticks whenever a string is full of backslashes or braces: Windows
paths, regular expressions, JSON fragments, SQL.

Raw strings can span several lines:

```xenith
let query: string = `
    SELECT name, age
    FROM people
    WHERE active = 1
`
echo(query)
```

```

    SELECT name, age
    FROM people
    WHERE active = 1

```

## Joining

`+` concatenates:

```xenith
let first: string = "Ada"
let last: string = "Lovelace"

echo(first + " " + last)
```

```
Ada Lovelace
```

Interpolation reads better for anything with more than two pieces:

```xenith
let first: string = "Ada"
let last: string = "Lovelace"
let age: int = 36

echo("{first} {last}, age {age}")
```

```
Ada Lovelace, age 36
```

## Length

`.len()` gives the number of characters, and the free function `len()` does the
same:

```xenith
let word: string = "xenith"

echo("{word.len()}")
echo("{len(word)}")
```

```
6
6
```

## Comparison

Strings compare with `==` and `!=`:

```xenith
let a: string = "ada"
let b: string = "ada"

echo("{a == b}")
echo("{a != "alan"}")
```

```
true
true
```

Comparison is case sensitive and exact. There is no built in case folding or
trimming yet.

## What strings cannot do yet

There is no indexing (`text[0]`), no slicing, no `split`, `trim`, `upper` or
`replace`. The string type is deliberately small right now. When a standard
library arrives those will live in it rather than being wired into the
interpreter.

Next: [Booleans and operators](05-booleans-and-operators.md)
