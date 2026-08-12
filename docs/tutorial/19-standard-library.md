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

Every function in it is ordinary Xenith, on top of four primitives the language
provides because they cannot be expressed in it: string indexing, `substring`,
`code_at` and `from_code`.

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
anything that can come up empty. See [Errors](16-errors.md).

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

## What is not here yet

Files, time and randomness. Collections are waiting on a decision about generics:
without them there is no way to write one `map` or `filter` that works for a
`list<int>` and a `list<string>` both, and `std::math` already shows the cost in
its `_float` suffixes.

## The cost of importing

A `grab` from `std::` parses, checks and runs that module, which takes roughly
ten milliseconds on top of a program's own startup. That is per module and per
run. It is worth knowing about for something invoked in a tight shell loop, and
irrelevant otherwise.

Back to [the tutorial index](README.md)
