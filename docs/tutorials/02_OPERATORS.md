# Xenith Operators

## Arithmetic Operators

Xenith supports standard arithmetic operations on numbers.

| Operator | Operation        | Example  | Result |
| -------- | ---------------- | -------- | ------ |
| `+`      | Addition         | `5 + 3`  | `8`    |
| `-`      | Subtraction      | `10 - 4` | `6`    |
| `*`      | Multiplication   | `6 * 7`  | `42`   |
| `/`      | Division         | `15 / 2` | `7.5`  |
| `^`      | Power (exponent) | `2 ^ 4`  | `16`   |

```xenith
let a: int = 10
let b: int = 3

echo(a + b)   # 13
echo(a - b)   # 7
echo(a * b)   # 30
echo(a / b)   # 3.333...
echo(a ^ b)   # 1000
```

## String Operators

### Concatenation with `+`

```xenith
let first: string = "Hello"
let last: string = "World"
let message: string = first + " " + last
echo(message)  # Hello World
```

### Repetition with `*`

```xenith
let dash: string = "-" * 10
echo(dash)  # ----------
```

## List Operators

### Concatenation with `+`

```xenith
let list1: list<int> = [1, 2, 3]
let list2: list<int> = [4, 5, 6]
let combined: list<int> = list1 + list2
echo(ret(combined))  # [1, 2, 3, 4, 5, 6]
```

## Comparison Operators

All comparison operators return a boolean (`true` or `false`).

| Operator | Meaning               | Example           |
| -------- | --------------------- | ----------------- |
| `==`     | Equal to              | `5 == 5` → `true` |
| `!=`     | Not equal to          | `5 != 3` → `true` |
| `<`      | Less than             | `3 < 5` → `true`  |
| `>`      | Greater than          | `5 > 3` → `true`  |
| `<=`     | Less than or equal    | `5 <= 5` → `true` |
| `>=`     | Greater than or equal | `5 >= 3` → `true` |

```xenith
let x: int = 10
let y: int = 20

echo(x == y)  # false
echo(x != y)  # true
echo(x < y)   # true
echo(x > y)   # false
echo(x <= 10) # true
echo(y >= 20) # true
```

## Logical Operators

| Operator | Meaning | Description                        |
| -------- | ------- | ---------------------------------- | --- | ----------------------------------- |
| `&&`     | AND     | `true` only if both sides are true |
| `        |         | `                                  | OR  | `true` if at least one side is true |
| `!`      | NOT     | Inverts a boolean value            |

```xenith
let t: bool = true
let f: bool = false

echo(t && t)   # true
echo(t && f)   # false
echo(t || f)   # true
echo(f || f)   # false
echo(!t)       # false
echo(!f)       # true
```

## Assignment Operators

| Operator | Meaning             | Equivalent             |
| -------- | ------------------- | ---------------------- |
| `=`      | Assign              | `x = 5`                |
| `+=`     | Add and assign      | `x += 3` → `x = x + 3` |
| `-=`     | Subtract and assign | `x -= 2` → `x = x - 2` |
| `++`     | Increment by 1      | `x++` → `x = x + 1`    |
| `--`     | Decrement by 1      | `x--` → `x = x - 1`    |

```xenith
let counter: int = 0

counter += 5   # counter = 5
counter -= 2   # counter = 3
counter++      # counter = 4
counter--      # counter = 3

echo(counter)  # 3
```

## Type Conversion Operator `as`

Convert between compatible types:

```xenith
let num: int = 42
let float_val: float = num as float      # 42.0
let string_val: string = num as string   # "42"
let bool_val: bool = num as bool         # true (non-zero)

let pi: float = 3.14159
let int_val: int = pi as int             # 3 (truncates)

let text: string = "123"
let parsed: int = text as int            # 123

let flag: bool = true
let flag_int: int = flag as int          # 1
```

### Type Conversion Rules

| From     | To       | Behavior                                           |
| -------- | -------- | -------------------------------------------------- |
| `int`    | `float`  | Adds `.0`                                          |
| `int`    | `string` | String representation                              |
| `int`    | `bool`   | `0` → `false`, non-zero → `true`                   |
| `float`  | `int`    | Truncates decimal part                             |
| `float`  | `string` | String representation                              |
| `string` | `int`    | Parses integer, errors if invalid                  |
| `string` | `float`  | Parses float, errors if invalid                    |
| `string` | `bool`   | `"true"`/`"1"` → `true`, `"false"`/`"0"` → `false` |
| `bool`   | `int`    | `true` → `1`, `false` → `0`                        |
| `bool`   | `string` | `"true"` or `"false"`                              |
| `bool`   | `float`  | `true` → `1.0`, `false` → `0.0`                    |

## Operator Precedence

From highest to lowest precedence:

1. `^` (power)
2. `*`, `/` (multiplication, division)
3. `+`, `-` (addition, subtraction)
4. `==`, `!=`, `<`, `>`, `<=`, `>=` (comparisons)
5. `&&` (AND)
6. `||` (OR)

Use parentheses `()` to control evaluation order:

```xenith
let result: int = (2 + 3) * 4   # 20, not 14
```

## Next Steps

- Learn about [CONTROL_FLOW.md](CONTROL_FLOW.md) for conditionals and loops
- Explore [COLLECTIONS.md](COLLECTIONS.md) for lists and maps
- Read [METHODS.md](METHODS.md) to create reusable code

```

```
