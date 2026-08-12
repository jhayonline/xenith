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

## Reaching into a string

Index with brackets, the same as a list. You get back a one character string,
counted in characters rather than bytes:

```xenith
let word: string = "hello"

echo(word[0])
echo(word[4])
echo("{"héllo"[1]}")
```

```
h
o
é
```

`substring(text, start, end)` takes the characters from `start` up to but not
including `end`. Both ends are clamped, so it never fails and callers need no
bounds check:

```xenith
let word: string = "hello"

echo(substring(word, 1, 3))
echo(substring(word, 0, 99))
echo("[{substring(word, 3, 1)}]")
```

```
el
hello
[]
```

`code_at(text, index)` gives the Unicode code point of a character, and
`from_code(n)` turns one back into a string. Together they are how case
conversion and classification get written:

```xenith
echo("{code_at("A", 0)}")
echo(from_code(97))
echo(from_code(code_at("A", 0) + 32))
```

```
65
a
a
```

## What strings cannot do yet

There is no `split`, `trim`, `upper`, `replace` or `contains` built in. Those
belong in a standard library, and the four operations above are deliberately the
whole primitive set they need: everything else about strings is meant to be
written in Xenith rather than wired into the interpreter.

`trim` is about twenty lines and needs nothing further:

```xenith
method is_space(c: string) -> bool {
    release c == " " || c == "\t" || c == "\n" || c == "\r"
}

method trim(text: string) -> string {
    let start: int = 0
    while start < text.len() && is_space(text[start]) {
        start = start + 1
    }
    let last: int = text.len()
    while last > start && is_space(text[last - 1]) {
        last = last - 1
    }
    release substring(text, start, last)
}

echo("[{trim("   padded   ")}]")
```

```
[padded]
```

Next: [Booleans and operators](05-booleans-and-operators.md)
