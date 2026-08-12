# Numbers

Xenith has two number types and keeps them apart. This is the single most
important thing to understand about the language, because most of its
predictability comes from here.

| Type | What it is | Range |
| --- | --- | --- |
| `int` | 64 bit signed integer | -9223372036854775808 to 9223372036854775807 |
| `float` | 64 bit IEEE 754 double | the usual |

A literal with no decimal point is an `int`. A literal with one is a `float`.

```xenith
let a = 42        # int
let b = 42.0      # float
let c = 3.14      # float
```

## int is a real integer

An `int` is not a double pretending. It holds every 64 bit value exactly:

```xenith
let big: int = 9007199254740993
echo("{big}")
```

```
9007199254740993
```

A language that stores every number as a double rounds that to
`9007199254740992` and says nothing about it.

## Arithmetic

| Operator | Meaning |
| --- | --- |
| `+` | add |
| `-` | subtract, and negate as a prefix |
| `*` | multiply |
| `/` | divide |
| `%` | remainder |
| `^` | raise to a power |

```xenith
echo("{7 + 2}")
echo("{7 - 2}")
echo("{7 * 2}")
echo("{7 / 2}")
echo("{7 % 2}")
echo("{2 ^ 10}")
echo("{-7}")
```

```
9
5
14
3
1
1024
-7
```

Note `7 / 2` is `3`. Integer division truncates towards zero, the same as C, Go
and Rust. If you want `3.5` you are asking about floats:

```xenith
echo("{7.0 / 2.0}")
```

```
3.5
```

## The two types never mix

Adding an `int` to a `float` is an error, not a promotion:

```xenith
let count: int = 3
let rate: float = 1.5

echo("{count + rate}")
```

```
error XEN001: Type Mismatch
  cannot add int and float
```

This feels strict for about a day, and then it starts catching things. The
mistake it prevents is the one where a value silently becomes a float halfway
through a calculation and your integer arithmetic starts producing 0.30000000004.

To combine them, convert explicitly with `as`:

```xenith
let count: int = 3
let rate: float = 1.5

let total: float = (count as float) * rate
echo("{total}")
```

```
4.5
```

The conversion is written at the exact point where precision could be lost,
which is where a reader wants to see it.

## Converting with `as`

`as` converts between the four primitive types:

```xenith
echo("{42 as string}")
echo("{"41" as int}")
echo("{"2.5" as float}")
echo("{3.99 as int}")
echo("{true as int}")
echo("{0 as bool}")
```

```
42
41
2.5
3
1
false
```

Converting a `float` to an `int` truncates, it does not round. `3.99 as int` is
`3`.

`as` binds more tightly than arithmetic, so this does what it looks like:

```xenith
let n: int = 7
echo("{n as float / 2.0}")
```

```
3.5
```

A string that is not a number is a conversion error rather than a zero:

```xenith
echo("{"hello" as int}")
```

```
error XEN011: Invalid Type Conversion
```

## Overflow is an error

Integer arithmetic is checked. When a result will not fit, the program stops:

```xenith
let max: int = 9223372036854775807
let oops: int = max + 1
```

```
error XEN017: Integer Overflow
  integer overflow in addition
```

Wrapping silently is the alternative, and it produces numbers that are wrong in a
way nothing points at. An error at the operation is easier to fix than a negative
total three functions later.

The same applies to a literal that does not fit:

```xenith
let too_big: int = 99999999999999999999
```

```
error XEN017: Integer Overflow
```

## Division by zero is an error

```xenith
echo("{10 / 0}")
```

```
error XEN003: Division by Zero
  cannot divide by zero
```

Guard it if the divisor could be zero:

```xenith
let numerator: int = 10
let divisor: int = 0

when divisor == 0 {
    echo("cannot divide")
} otherwise {
    echo("{numerator / divisor}")
}
```

```
cannot divide
```

## Compound assignment

```xenith
let n: int = 10

n += 5
echo("{n}")

n -= 3
echo("{n}")

n++
echo("{n}")

n--
echo("{n}")
```

```
15
12
13
12
```

`++` and `--` are statements, not expressions. You write `i++` on its own line;
you cannot use its value inside a larger expression.

## Built in constant

`MATH_PI` is available everywhere:

```xenith
echo("{MATH_PI}")
```

```
3.141592653589793
```

Next: [Strings](04-strings.md)
