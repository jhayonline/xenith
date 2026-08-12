# Built in Functions

Everything on this page is available in every file with no import.

These are the primitives: the operations the language provides because they
cannot be written in it. Anything that can be written in Xenith belongs in
[the standard library](19-standard-library.md) instead.

## Output

### echo(value)

Writes a value and a newline to standard output.

```xenith
echo("hello")
echo(42)
echo([1, 2, 3])
```

```
hello
42
[1, 2, 3]
```

Parentheses are optional for a single argument:

```xenith
echo "no parentheses needed"
```

```
no parentheses needed
```

### ret(value)

Turns any value into a string without printing it. This is what you want when
putting a list or a map inside interpolation.

```xenith
let numbers: list<int> = [1, 2, 3]

echo("the numbers are {ret(numbers)}")
```

```
the numbers are [1, 2, 3]
```

`ret` on a one element list drops the brackets: `ret([5])` is `5`.

## Size

### len(value)

The number of elements in a list or map, or characters in a string.

```xenith
echo("{len("xenith")}")
echo("{len([1, 2, 3])}")
```

```
6
3
```

Lists, maps and strings also have a `.len()` method, which reads better in a
chain:

```xenith
let words: list<string> = ["a", "b"]

echo("{words.len()}")
```

```
2
```

## List functions

These take a list and return a new one. They do not change what you pass in,
unlike the `.append()` and `.pop()` methods described in [Lists](08-lists.md).

### append(list, value)

```xenith
let base: list<int> = [1, 2]
let bigger: list<int> = append(base, 3)

echo("{ret(bigger)}")
echo("{ret(base)}")
```

```
[1, 2, 3]
[1, 2]
```

### extend(list, other)

```xenith
let a: list<int> = [1, 2]
let joined: list<int> = extend(a, [3, 4])

echo("{ret(joined)}")
```

```
[1, 2, 3, 4]
```

### pop(list, index)

Returns the element at the index. The list you pass in is unchanged.

```xenith
let values: list<int> = [10, 20, 30]
let first: int = pop(values, 0)

echo("{first}")
echo("{ret(values)}")
```

```
10
[10, 20, 30]
```

## Type predicates

Each returns a `bool`.

| Function | True when |
| --- | --- |
| `is_num(v)` | `v` is an `int` or a `float` |
| `is_str(v)` | `v` is a string |
| `is_list(v)` | `v` is a list |
| `is_fun(v)` | `v` is a method |

```xenith
method square(n: int) -> int => n * n

echo("{is_num(1)}")
echo("{is_num(1.5)}")
echo("{is_str("text")}")
echo("{is_list([1])}")
echo("{is_fun(square)}")
echo("{is_num("7")}")
```

```
true
true
true
true
true
false
```

## Input

### input()

Reads one line from standard input and returns it as a string, without the
newline.

```xenith
echo("What is your name?")
let name: string = input()
echo("Hello, {name}")
```

### input_int()

Reads one line and parses it as an `int`, asking again until it gets one.

```xenith
echo("How old are you?")
let age: int = input_int()
echo("In ten years you will be {age + 10}")
```

Neither of these has example output here, because both wait for you to type
something.

## Terminal

### clear()

Clears the screen.

```xenith
clear()
echo("fresh start")
```

## Running another file

### run(path)

Executes another `.xen` file. The path is a string including the extension.

```xenith
run("setup.xen")
echo("setup finished")
```

Use [modules](14-modules.md) instead when you want definitions from another file.
`run` is for the case where you want the file's effects.

## Built in constants

| Name | Value |
| --- | --- |
| `TRUE` | `true` |
| `FALSE` | `false` |
| `NULL` | `null` |
| `MATH_PI` | 3.141592653589793 |

```xenith
echo("{MATH_PI}")
echo("{TRUE} {FALSE} {NULL}")
```

```
3.141592653589793
true false null
```

The lowercase literals `true`, `false` and `null` mean the same as the constants
and are the ones to write.

## String primitives

`substring(text, start, end)`, `code_at(text, index)` and `from_code(code)`,
along with indexing a string, are what a string library is built from. See
[Strings](04-strings.md).

## Float primitives

`sin(x)`, `cos(x)`, `tan(x)`, `atan2(y, x)`, `log(x)`, `log10(x)` and `exp(x)`.

Each takes a float and returns one. They are here because a series expansion
written in Xenith would give wrong answers away from zero, not because Rust is
faster. `sqrt` is not among them, because `x ^ 0.5` already is one.

```xenith
echo("{cos(0.0)} {exp(0.0)} {log10(1000.0)}")
```

```
1.0 1.0 3.0
```

They will not take an int. The language does not convert between int and float
anywhere else, so `sqrt(n as float)` says what it does.

[std::math](19-standard-library.md) has `abs`, `floor`, `round`, `sqrt` and the
rest, written in Xenith on top of these.

## format

### String primitives

`substring(text, start, end)`, `code_at(text, index)` and `from_code(code)`,
along with indexing a string, are what a string library is built from. See
[Strings](04-strings.md).

## Float primitives

`sin(x)`, `cos(x)`, `tan(x)`, `atan2(y, x)`, `log(x)`, `log10(x)` and `exp(x)`.

Each takes a float and returns one. They are here because a series expansion
written in Xenith would give wrong answers away from zero, not because Rust is
faster. `sqrt` is not among them, because `x ^ 0.5` already is one.

```xenith
echo("{cos(0.0)} {exp(0.0)} {log10(1000.0)}")
```

```
1.0 1.0 3.0
```

They will not take an int. The language does not convert between int and float
anywhere else, so `sqrt(n as float)` says what it does.

[std::math](19-standard-library.md) has `abs`, `floor`, `round`, `sqrt` and the
rest, written in Xenith on top of these.

## format(text)

Applies `{}` interpolation to a string and returns the result.

An ordinary double quoted string is interpolated as it is built, so `format` has
nothing to do to one. Its use is a [backtick raw string](04-strings.md), which is
not interpolated, so its braces survive until you ask for them to be filled in:

```xenith
let name: string = "Ada"
let raw: string = `Hello {name}!`

echo(raw)
echo(format(raw))
```

```
Hello {name}!
Hello Ada!
```

That is the whole point of it: keep the text exactly as written, and decide later
when to fill it in. It reads well for anything full of braces and backslashes:

```xenith
let table: string = "users"

echo(format(`SELECT * FROM {table} WHERE active = 1`))
```

```
SELECT * FROM users WHERE active = 1
```

It is an ordinary expression, so it can be assigned, nested and passed around,
and it evaluates in the scope it was called from:

```xenith
method describe(who: string, age: int) -> string {
    let suffix: string = " years old"
    release format(`{who} is {age}{suffix}`)
}

echo(describe("Ada", 36))
```

```
Ada is 36 years old
```

It takes exactly one string. For everything else, plain interpolation is
shorter:

```xenith
let name: string = "Ada"
let count: int = 3

echo("{name} has {count} items")
```

```
Ada has 3 items
```

Next: [Errors](16-errors.md)
