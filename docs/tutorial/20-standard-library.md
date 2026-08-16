# The Standard Library

Import it with the `std::` prefix. There is nothing to install and no path to
configure; the library is built into the interpreter.

```xenith
grab { trim, upper, split } from "std::string"

echo("[{trim("  hello  ")}]")
echo(upper("shout"))
echo("{ret(split("a,b,c", ","))}")
```

```
[hello]
SHOUT
[a, b, c]
```

## It is written in Xenith

Every function in it is ordinary Xenith, on top of a small set of primitives the
language provides because they cannot be expressed in it: string indexing,
`substring`, `code_at` and `from_code` for text, the float functions for maths,
and the `fs_`, `bytes_` and `env_` families for reaching outside the program.

That is a deliberate choice rather than a stopgap. It means you can read the
source of `trim` and it will look like code you would write:

```xenith
export method trim_start(text: string) -> string {
    let start: int = 0
    while start < text.len() && is_space(text[start]) {
        start = start + 1
    }
    release substring(text, start, text.len())
}
```

It also means the library is the language's most demanding user. Writing it
found a bug in `&&` within twenty lines.

The cost is speed. A Xenith `split` walks the string a character at a time
through the interpreter, so it is far slower than the same thing written in
Rust. Where that matters for a particular function, that function can be moved
into the interpreter later without its signature changing.

## std::string

### Trimming

| Function | |
| --- | --- |
| `trim(text)` | whitespace off both ends |
| `trim_start(text)` | off the front |
| `trim_end(text)` | off the end |

```xenith
grab { trim, trim_start, trim_end } from "std::string"

echo("[{trim("  hello  ")}]")
echo("[{trim_start("  left")}]")
echo("[{trim_end("right  ")}]")
```

```
[hello]
[left]
[right]
```

### Case and shape

| Function | |
| --- | --- |
| `upper(text)` | uppercase, ASCII only |
| `lower(text)` | lowercase, ASCII only |
| `reverse(text)` | characters in the opposite order |
| `repeat(text, times)` | the text `times` over |
| `is_empty(text)` | no characters |

```xenith
grab { upper, lower, reverse, repeat } from "std::string"

echo(upper("hello World 123"))
echo(lower("HELLO World"))
echo(reverse("abcdef"))
echo("[{repeat("ab", 3)}]")
```

```
HELLO WORLD 123
hello world
fedcba
[ababab]
```

Case handling is ASCII only. Text outside ASCII comes back unchanged rather than
guessed at, because correct Unicode case folding needs tables the library does
not carry.

### Characters

Each takes a one character string and returns a `bool`. Anything longer is
false.

| Function | True for |
| --- | --- |
| `is_space(c)` | space, tab, newline, carriage return |
| `is_digit(c)` | `0` to `9` |
| `is_alpha(c)` | `a` to `z`, `A` to `Z` |
| `is_upper(c)` | `A` to `Z` |
| `is_lower(c)` | `a` to `z` |

```xenith
grab { is_digit, is_alpha, is_space } from "std::string"

echo("{is_digit("7")} {is_digit("x")} {is_digit("77")}")
echo("{is_alpha("Q")} {is_space(" ")}")
```

```
true false false
true true
```

### Searching

| Function | |
| --- | --- |
| `contains(text, needle)` | is it in there |
| `starts_with(text, prefix)` | |
| `ends_with(text, suffix)` | |
| `count(text, needle)` | how many non overlapping times |
| `index_of(text, needle)` | `(position, found)` |
| `last_index_of(text, needle)` | `(position, found)` |

The two `index_of` functions return a tuple, because a position alone cannot say
"not here". Destructuring makes you name the flag, so it cannot be ignored:

```xenith
grab { index_of, contains, count } from "std::string"

let (at, found) = index_of("hello world", "world")

when found {
    echo("found at {at}")
} otherwise {
    echo("not there")
}

echo("{contains("haystack", "stack")}")
echo("{count("banana", "an")}")
```

```
found at 6
true
2
```

That `(value, bool)` shape is the convention across the whole library for
anything that can come up empty. See [Errors](17-errors.md).

### Splitting and joining

| Function | |
| --- | --- |
| `split(text, separator)` | a `list<string>` |
| `join(parts, separator)` | back into one string |
| `replace(text, needle, replacement)` | every occurrence |

```xenith
grab { split, join, replace } from "std::string"

echo("{ret(split("a,b,c", ","))}")
echo("{ret(split("a::b::c", "::"))}")
echo("[{join(["x", "y"], " and ")}]")
echo("[{replace("a-b-c", "-", "+")}]")
```

```
[a, b, c]
[a, b, c]
[x and y]
[a+b+c]
```

An empty separator splits into single characters:

```xenith
grab { split } from "std::string"

echo("{ret(split("abc", ""))}")
```

```
[a, b, c]
```

Splitting keeps empty pieces, so a leading or trailing separator gives an empty
string at that end. That is what makes `split` and `join` reverse each other.

### Padding

| Function | |
| --- | --- |
| `pad_start(text, width, fill)` | fill on the left until `width` |
| `pad_end(text, width, fill)` | fill on the right |

```xenith
grab { pad_start, pad_end } from "std::string"

echo("[{pad_start("7", 3, "0")}]")
echo("[{pad_end("x", 4, ".")}]")
echo("[{pad_start("toolong", 3, "0")}]")
```

```
[007]
[x...]
[toolong]
```

Text already at or past the width is returned unchanged.

## Putting it together

The functions compose, which is the point of having them in the language rather
than as opaque builtins:

```xenith
grab { split, join, trim, lower, upper, is_empty } from "std::string"

method title_case(text: string) -> string {
    let words: list<string> = split(lower(trim(text)), " ")
    let out: list<string> = []
    for word in words {
        when is_empty(word) {
            skip
        }
        out.append(upper(word[0]) + substring(word, 1, word.len()))
    }
    release join(out, " ")
}

echo(title_case("  the QUICK brown   fox  "))
```

```
The Quick Brown Fox
```

## std::math

### Whole numbers

| Function | |
| --- | --- |
| `abs(n)` | |
| `min(a, b)`, `max(a, b)` | |
| `clamp(n, low, high)` | |
| `sign(n)` | -1, 0 or 1 |
| `is_even(n)`, `is_odd(n)` | |
| `gcd(a, b)`, `lcm(a, b)` | always positive |
| `factorial(n)` | |
| `pow(base, exponent)` | whole number result |

```xenith
grab { abs, clamp, gcd, lcm, factorial } from "std::math"

echo("{abs(-5)} {clamp(10, 0, 5)}")
echo("{gcd(12, 18)} {lcm(4, 6)}")
echo("{factorial(20)}")
```

```
5 5
6 12
2432902008176640000
```

### Floats

The same shapes with a `_float` suffix: `abs_float`, `min_float`, `max_float`,
`clamp_float`, `sign_float`. Plus `sqrt(x)` and `pow_float(base, exponent)`.

The suffix is not a style choice. Without generics there is no way to write one
`abs` that takes an `int` and a `float` both, so the int version has the plain
name and the float version is spelled out. This module is the clearest evidence
of what that costs.

### Rounding

All four take a float and give an int.

| Function | Towards |
| --- | --- |
| `floor(x)` | negative infinity |
| `ceil(x)` | positive infinity |
| `trunc(x)` | zero |
| `round(x)` | nearest, halves away from zero |

```xenith
grab { floor, ceil, trunc, round } from "std::math"

echo("{floor(2.7)} {floor(-2.7)}")
echo("{ceil(2.1)} {ceil(-2.1)}")
echo("{trunc(2.7)} {trunc(-2.7)}")
echo("{round(2.5)} {round(-2.5)}")
```

```
2 -3
3 -2
2 -2
3 -3
```

The negative cases are where these differ, and where a naive implementation goes
wrong: `as int` truncates towards zero, so truncating -2.7 gives -2, which is the
ceiling rather than the floor.

### Over a list

`sum(values)` and `product(values)` take a `list<int>`. `min_of` and `max_of`
return `(value, found)`, because an empty list has no smallest element and
saying so beats inventing one:

```xenith
grab { sum, product, max_of } from "std::math"

echo("{sum([1, 2, 3, 4])} {product([2, 3, 4])}")

let (largest, found) = max_of([5, 2, 8])
when found {
    echo("largest {largest}")
}
```

```
10 24
largest 8
```

### Constants

`MATH_PI` is built into the language. `std::math` adds `E`, `TAU`, `INT_MAX` and
`INT_MIN`.

### The transcendental functions

`sin`, `cos`, `tan`, `atan2`, `log`, `log10` and `exp` are language primitives,
so they need no import:

```xenith
echo("{cos(0.0)} {exp(0.0)} {log10(1000.0)}")
```

```
1.0 1.0 3.0
```

They are primitives because a series expansion written in Xenith would give wrong
answers away from zero, not because Rust is faster. `sqrt` is not among them:
`x ^ 0.5` already is one. `log10` is, because `log(x) / log(10.0)` gives
2.9999999999999996 for a thousand.

They take floats, not ints. The language does not convert between the two
anywhere else and this is not where it should start, so `sqrt(n as float)` says
what it does.

## std::fs

Files and paths. This is the one module where failure is ordinary rather than
exceptional, so it reports differently from the rest of the library.

### How failure is reported

Anything that touches the filesystem hands back an error as a string, empty when
nothing went wrong:

```xenith
grab { read } from "std::fs"

let (text, error) = read("nowhere.txt")

when error != "" {
    echo("could not read it")
} otherwise {
    echo(text)
}
```

```
could not read it
```

That is not the `(value, bool)` used everywhere else, and the difference is
deliberate. `index_of` failing has exactly one meaning, so a flag says
everything. A file operation failing has a dozen, and the reason is most of what
a caller needs.

Operations with nothing to return give back only the error:

```xenith
grab { write } from "std::fs"

let error: string = write("out.txt", "hello")
when error == "" {
    echo("written")
}
```

```
written
```

### Files

| Function | |
| --- | --- |
| `read(path)` | `(contents, error)` |
| `write(path, contents)` | replaces what was there, returns an error |
| `append(path, contents)` | adds to the end |
| `remove(path)` | |
| `exists(path)`, `is_file(path)`, `is_dir(path)` | plain `bool` |
| `size(path)` | `(byte_count, error)` |
| `copy(source, destination)` | any file, held in memory |

`read` and `write` are for text and fail on a file that is not valid UTF-8.
Three more take the file as it is:

| Function | |
| --- | --- |
| `read_bytes(path)` | `(bytes, error)` |
| `write_bytes(path, raw)` | |
| `append_bytes(path, raw)` | |

```xenith
grab { read_bytes, write_bytes } from "std::fs"
grab { to_hex, from_list } from "std::bytes"

let (raw, build_error) = from_list([0, 159, 146, 150])
echo("[{write_bytes("icon.dat", raw)}]")

let (back, read_error) = read_bytes("icon.dat")
echo("{to_hex(back)} [{read_error}]")
```

```
[]
009f9296 []
```

### Lines

`read_lines(path)` gives `(list<string>, error)`, and `write_lines(path, lines)`
writes each with a newline after it. A trailing newline does not become a final
empty line, because a text file conventionally ends with one and almost nobody
means it as an extra line:

```xenith
grab { write_lines, read_lines } from "std::fs"

write_lines("notes.txt", ["first", "second"])

let (lines, error) = read_lines("notes.txt")
echo("{lines.len()} {ret(lines)}")
```

```
2 [first, second]
```

### Directories

`list_dir(path)` gives `(list<string>, error)`, sorted and without the leading
path, so a program that walks a directory behaves the same on every machine.
`create_dir(path)` makes any missing parents and is happy if the directory is
already there.

`remove_dir(path)` deletes an empty directory. There is no recursive form on
purpose: removing a tree by accident from a script is not a mistake worth making
convenient.

### Paths

These touch no files at all. A path is taken apart the same way whether or not
anything exists at it. Separators are forward slashes.

| Function | `"/a/b/c.txt"` gives |
| --- | --- |
| `basename(path)` | `c.txt` |
| `dirname(path)` | `/a/b` |
| `extension(path)` | `txt` |
| `stem(path)` | `c` |
| `join_path(left, right)` | one separator between, however many you supplied |
| `with_extension(path, ext)` | the same path with a different ending |

```xenith
grab { basename, dirname, extension, stem, join_path, with_extension } from "std::fs"

echo("{basename("/a/b/c.txt")} {dirname("/a/b/c.txt")}")
echo("{extension("archive.tar.gz")} {stem("archive.tar.gz")}")
echo("{join_path("a/", "/b")}")
echo("{with_extension("notes/draft.txt", "md")}")
echo("[{extension(".gitignore")}]")
```

```
c.txt /a/b
gz archive.tar
a/b
notes/draft.md
[]
```

A leading dot means a hidden file rather than an extension, so `.gitignore` has
none.

All of the path handling is written in Xenith on top of `std::string`. Only the
filesystem itself needs primitives, and those carry an `fs_` prefix: an operation
on a built in type like `substring` is global under its own name, but reading a
file is a service, and it should be visible in the imports that a program does
it.

## std::bytes

Raw bytes. The language already gives the type `len`, indexing, `+`, `==` and
`as` in both directions; this module is what sits above those.

```xenith
grab { from_string, to_hex, index_of, slice } from "std::bytes"

let raw: bytes = from_string("hello world")

let (at, found) = index_of(raw, from_string("world"))
echo("{at} {found}")
echo(to_hex(slice(raw, 0, 2)))
```

```
6 true
6865
```

### Making and converting

| Function | |
| --- | --- |
| `empty()` | no bytes at all |
| `from_string(text)` | never fails; text is already UTF-8 |
| `to_string(raw)` | `(text, error)` |
| `to_list(raw)` | each byte as an int in 0 to 255 |
| `from_list(codes)` | `(bytes, error)`; a value outside 0 to 255 is an error |

`raw as string` is the same conversion as `to_string`, except that it stops the
program instead of handing back the reason. Use `as` when invalid bytes would be
a bug, and `to_string` when they are a case to handle.

### Looking at them

| Function | |
| --- | --- |
| `size(raw)` | the same as `len(raw)` |
| `is_blank(raw)` | whether there are none |
| `at(raw, index)` | `(byte, in_range)`, where `raw[index]` would stop the program |
| `slice(raw, start, end)` | clamped at both ends, so it never fails |
| `concat(left, right)` | the same as `left + right` |
| `equal(left, right)` | the same as `left == right` |

### Searching

| Function | |
| --- | --- |
| `starts_with(raw, prefix)`, `ends_with(raw, suffix)` | |
| `index_of(raw, needle)` | `(position, found)` |
| `contains(raw, needle)` | |
| `count(raw, needle)` | without overlaps |

### Hex

| Function | |
| --- | --- |
| `to_hex(raw)` | lowercase, two characters per byte, no separators |
| `from_hex(text)` | `(bytes, error)`; upper case is accepted |

```xenith
grab { from_hex, to_hex, from_string } from "std::bytes"

echo(to_hex(from_string("hi")))

let (raw, error) = from_hex("68656c6c6f")
echo("{raw as string} [{error}]")

let (odd, odd_error) = from_hex("abc")
echo("[{odd_error}]")
```

```
6869
hello []
[hex text must have an even number of characters]
```

`from_hex` is also the practical way to write a run of byte values, since the
language has no `0xff` literal.

## std::env

The process environment: variables, the command line, the working directory and
the exit status.

Reading a variable hands back a pair, because a variable that is unset and one
set to the empty string are different things and a single string cannot tell
them apart:

```xenith
grab { get, get_or } from "std::env"

let (value, found) = get("PORT")
when !found {
    echo("PORT is not set")
}

echo(get_or("PORT", "8080"))
```

```
PORT is not set
8080
```

### Variables

| Function | |
| --- | --- |
| `get(name)` | `(value, found)` |
| `get_or(name, fallback)` | the fallback only when unset, not when empty |
| `has(name)` | |
| `get_int(name)` | `(number, ok)` |
| `get_flag(name)` | `1`, `true`, `yes`, `on` in any case; anything else is false |
| `set(name, value)`, `unset(name)` | this process and anything it starts |
| `all()` | `map<string, string>` |

`set` does not reach the shell that started the program. No process can change
its parent's environment, in any language.

### The command line

| Function | |
| --- | --- |
| `args()` | the program, then everything after it |
| `params()` | just the arguments, without the program name |
| `program()` | the path of the program, as it was typed |

```xenith
grab { params } from "std::env"

for argument in params() {
    echo(argument)
}
```

```sh
xenith greet.xen Ada Alan
```

```
Ada
Alan
```

### The process

| Function | |
| --- | --- |
| `cwd()` | `(path, error)` |
| `exit(code)` | stops immediately; nothing after it runs |

`exit` is for the end of a program. It is not a way to report a failure up a
call chain, because nothing gets to see it happen. See [Errors](17-errors.md).

## std::json

JSON as a Xenith value, and back again.

```xenith
grab { parse, get, as_text, as_int } from "std::json"

let (doc, error) = parse(`{"name": "Ada", "age": 36}`)
when error != "" {
    echo("not JSON: {error}")
}

let (name_value, found) = get(doc, "name")
let (name, is_text) = as_text(name_value)

echo("{name} is here: {found}")
```

```
Ada is here: true
```

Note the backticks. A document written in double quotes needs every `"` escaped
and every `{` doubled; a [raw string](04-strings.md#raw-strings) takes it exactly
as written, which is how the examples here and the tests are all written.

### The value type

Everything in the module is built on one exported enum:

```xenith
export enum Json {
    Null,
    Bool(bool),
    Int(int),
    Float(float),
    Text(string),
    Array(list<Json>),
    Object(map<string, Json>)
}
```

That is the shape a JSON document actually has: a value is exactly one of those
things. A `map` cannot stand in for an object, because a map is homogeneous and
an object's values are not — which is the reason [enums](13-enums.md) exist in
the language at all.

It is an ordinary exported enum, so you can `match` on it and build values with
it directly. `grab { Json }` and the variants are yours.

### Two number variants

JSON has one number type and most libraries make it a double. This one does not,
because Xenith has a real 64 bit `int` and losing it costs more than it saves:

```xenith
grab { parse, stringify, type_name, get } from "std::json"

let (doc, _) = parse(`{"id": 9007199254740993, "ratio": 36.0}`)

let (id, _) = get(doc, "id")
let (ratio, _) = get(doc, "ratio")

echo("{type_name(id)} {type_name(ratio)}")
echo(stringify(doc))
```

```
int float
{"id":9007199254740993,"ratio":36.0}
```

Through a double that identifier would come back as `...92`, and `36` would come
back as `36.0`. The rule is: a number with no `.` and no `e` is a `Json::Int` if
it fits in an `int`, and a `Json::Float` otherwise. Anything written with a `.`
or an `e` stays a `Json::Float` whatever its value, because that is what the
document said.

`as_float` accepts either, so code that does not care never has to look.

### Getting values out

Each of these answers "is it this, and if so what is it" in one call, so the
check and the read cannot drift apart:

| Function | |
| --- | --- |
| `as_bool(v)` | `(bool, ok)` |
| `as_int(v)` | `(int, ok)`; a whole `Json::Float` like `36.0` reads as `36` |
| `as_float(v)` | `(float, ok)` from either number variant |
| `as_text(v)` | `(string, ok)` |
| `as_array(v)` | `(list<Json>, ok)` |
| `as_object(v)` | `(map<string, Json>, ok)` |

And for finding them:

| Function | |
| --- | --- |
| `get(v, key)` | `(value, found)`, false for anything that is not an object |
| `at(v, index)` | `(value, in_range)`, false for anything that is not an array |
| `keys(v)` | the field names, empty for anything else |
| `size(v)` | elements or fields, `0` for anything else |
| `type_name(v)` | `"null"`, `"bool"`, `"int"`, `"float"`, `"string"`, `"array"`, `"object"` |
| `is_null`, `is_bool`, `is_number`, `is_text`, `is_array`, `is_object` | |

`get` and `at` return false rather than stopping when the value is the wrong
shape, so walking a document you did not write does not need a type check at
every step:

```xenith
grab { parse, get, as_array, as_text, is_null, size } from "std::json"

let (doc, _) = parse(`{"rows": [{"user": "ada"}, {"user": "alan"}], "next": null}`)

let (rows, _) = get(doc, "rows")
echo("{size(rows)} rows")

let (items, _) = as_array(rows)
for row in items {
    let (user, _) = get(row, "user")
    let (word, _) = as_text(user)
    echo("  {word}")
}

let (next, _) = get(doc, "next")
echo("more pages: {!is_null(next)}")
```

```
2 rows
  ada
  alan
more pages: false
```

### Writing

```xenith
grab { Json, stringify, pretty } from "std::json"

let doc: Json = Json::Object({
    "name": Json::Text("Ada"),
    "age": Json::Int(36),
    "tags": Json::Array([Json::Text("founder")])
})

echo(stringify(doc))
echo(pretty(doc, 2))
```

```
{"age":36,"name":"Ada","tags":["founder"]}
{
  "age": 36,
  "name": "Ada",
  "tags": [
    "founder"
  ]
}
```

Fields come out in order of name, because that is the order a Xenith map keeps
them in. It costs nothing and makes output you can diff.

`stringify` escapes only what JSON requires — the quote, the backslash and the
control characters. Text outside ASCII is written as itself rather than as `\u`
escapes, which is shorter and what every reader expects.

### The one thing `stringify` cannot write

JSON has no way to write an infinity or a NaN. `stringify` never produces
invalid JSON, so those come out as `null`, and `check` is how you find out
before that happens:

```xenith
grab { Json, stringify, check } from "std::json"

# A computation that overflows. Division by zero is an error in Xenith, so this
# is the way a float actually becomes infinite.
let scaled: float = exp(1000.0)
let doc: Json = Json::Object({"scaled": Json::Float(scaled)})

echo("{scaled}")
echo(stringify(doc))
echo(check(doc))
```

```
inf
{"scaled":null}
inf cannot be written as JSON
```

`check` gives an empty string for a document that is safe.

### When a document is wrong

`parse` never stops the program. A document arrives from outside, so being
malformed is a case to handle rather than a bug in the code that called it:

```xenith
grab { parse } from "std::json"

let (doc, error) = parse(`{"a": [1, 2,]}`)
echo("[{error}]")

let (other, other_error) = parse(`{"a": 1} trailing`)
echo("[{other_error}]")
```

```
[`]` does not start a value at position 12]
[unexpected text after the value at position 9]
```

Positions are character positions, counted the way `len()` counts.

The parser is strict, which for a format this widely mis-implemented is the
useful setting. It rejects trailing commas, unquoted field names, leading zeros,
single quotes, `NaN` and `inf`, hex, control characters sitting raw inside a
string, and any text after the value. Text after the value is worth calling out:
it almost always means the document was truncated or two of them were run
together, and a parser that ignores it hides that.

Values may nest 200 deep. Past that `parse` returns an error rather than
recursing until the interpreter ends the program, which matters the first time
somebody posts you five thousand open brackets.

### What it costs

The parser is Xenith walking the text a character at a time, so it is slow:
roughly 25KB in a couple of seconds, and worse than linear as documents grow,
because `append` and map insertion currently copy the collection. That is fine
for configuration files and small request bodies, and not yet fine for a busy
server. Both are interpreter problems rather than library ones, and the
signatures here will not change when they are fixed.

## What is not here yet

Time and randomness. Collections are waiting on a decision about generics:
without them there is no way to write one `map` or `filter` that works for a
`list<int>` and a `list<string>` both, and `std::math` already shows the cost in
its `_float` suffixes.

## The cost of importing

A `grab` from `std::` parses, checks and runs that module, which takes roughly
ten milliseconds on top of a program's own startup. That is per module and per
run. It is worth knowing about for something invoked in a tight shell loop, and
irrelevant otherwise.

Back to [the tutorial index](README.md)
