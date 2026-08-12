# Built in Functions

Everything on this page is available in every file with no import. There is no
standard library beyond this yet, which is deliberate: the language is being
settled before the library is built on top of it.

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

## A note on format

There is a `format` builtin, but it cannot be used as an expression: `format` is
a keyword to the lexer rather than an identifier, so
`let s = format("{}", x)` and `echo(format(...))` both fail to parse. It only
works as a statement on its own, where it prints directly.

String interpolation covers everything `format` would do:

```xenith
let name: string = "Ada"
let count: int = 3

echo("{name} has {count} items")
```

```
Ada has 3 items
```

Next: [Errors](16-errors.md)
