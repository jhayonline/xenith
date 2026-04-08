# Xenith Core Language

## Introduction

Xenith is a statically-typed, interpreted programming language that combines Python-like readability with Rust-like safety. Every variable, parameter, and return value must have an explicit type annotation. If your code parses, it's type-safe.

## Hello, World

```xenith
echo("Hello, World!")
```

A line can be terminated using ';' or not. 

## Variables

### Declaration with `spawn`

All variables are created using the `spawn` keyword followed by a type annotation:

```xenith
spawn age: int = 25
spawn name: string = "Alice"
spawn price: float = 19.99
spawn is_active: bool = true
spawn nothing: null = null
```

### Constants with `const spawn`

Constants cannot be reassigned after declaration:

```xenith
const spawn MAX_SIZE: int = 100
const spawn APP_NAME: string = "Xenith"
```

### Reassignment

Variables are reassigned without the `spawn` keyword:

```xenith
spawn counter: int = 0
counter = 10          # Valid
counter = counter + 5 # Valid
```

Constants **cannot** be reassigned:
```xenith
const spawn MAX: int = 100
MAX = 200  # Error! Cannot reassign constant
```

## Primitive Types

| Type | Description | Example |
|------|-------------|---------|
| `int` | 64-bit integer | `42`, `-7`, `0` |
| `float` | 64-bit floating point | `3.14`, `-0.5`, `2.0` |
| `string` | UTF-8 text | `"Hello"`, `""`, `"123"` |
| `bool` | Boolean value | `true`, `false` |
| `null` | Absence of value | `null` |

## Type Inference (Limited)

Xenith requires explicit types, but the compiler will validate that your assigned value matches:

```xenith
spawn x: int = 42      # Valid
spawn y: int = "hello" # Error! Type mismatch
```

## Comments

Single-line comments start with `#`:

```xenith
# This is a comment
spawn x: int = 5  # Comments can be at the end of a line
```

## Basic Output

Use `echo()` to print to the console:

```xenith
echo("Hello")           # Prints: Hello
echo(42)                # Prints: 42
echo("Value: {x}")      # String interpolation (see STRINGS.md)
```

## Basic Input

Read user input:

```xenith
spawn name: string = input()
echo("Hello, {name}")

spawn age: int = input_int()
echo("You are {age} years old")
```

## Example Program

```xenith
# A simple greeting program
spawn user_name: string = input()
spawn user_age: int = input_int()

echo("Name: {user_name}")
echo("Age: {user_age}")

const spawn GREETING: string = "Welcome!"
echo(GREETING)
```

## Key Principles

1. **Explicit is safe** - Every type is written, every intention is clear
2. **Parse-time safety** - Type errors are caught before any code runs
3. **No surprises** - What you see is what you get

## Next Steps

- Learn about [OPERATORS.md](OPERATORS.md) for arithmetic and logic
- Read [CONTROL_FLOW.md](CONTROL_FLOW.md) for conditionals and loops
- Explore [METHODS.md](METHODS.md) to define reusable code
```

