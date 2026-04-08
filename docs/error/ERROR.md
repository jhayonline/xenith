# Xenith Error System

## Introduction

Xenith provides a rich, helpful error system that guides developers to fix issues quickly. Every error includes:
- A unique error code
- The exact file, line, and column location
- A visual arrow pointing to the problem
- A clear explanation (note)
- A helpful suggestion (help)

## Error Code Categories

| Range | Category | Description |
|-------|----------|-------------|
| XEN001-XEN099 | Type System | Type mismatches, conversions |
| XEN100-XEN199 | Lexical/Syntax | Illegal characters, unexpected tokens |
| XEN200-XEN299 | Runtime | Division by zero, index out of bounds |
| XEN300-XEN399 | I/O | File not found, permission denied |
| XEN400-XEN499 | Modules | Module not found, import errors |
| XEN500-XEN599 | Environment | Environment variables, process errors |
| XEN600-XEN699 | Methods/Structs | Method not found, field not found |

## Error Code Reference

### Type System Errors

#### XEN001: Type Mismatch
Occurs when a value doesn't match the expected type.

```xenith
spawn x: string = 42
```

**Error:**
```
Error XEN001: Type Mismatch
  → tests/example.xen:2:19
  │
  │ spawn x: string = 42
  │                   ^^
  = note: cannot assign `int` to variable of type `string`
  = help: use type conversion: `value as string`
```

#### XEN002: Undefined Variable
Occurs when accessing a variable that hasn't been declared.

```xenith
echo(undefined_var)
```

**Error:**
```
Error XEN002: Undefined Variable
  → tests/example.xen:2:6
  │
  │ echo(undefined_var)
  │      ^^^^^^^^^^^^^
  = note: variables must be declared with `spawn` before use
  = help: check spelling or declare the variable first
```

### Runtime Errors

#### XEN003: Division by Zero
Occurs when attempting to divide by zero.

```xenith
spawn result: int = 10 / 0
```

**Error:**
```
Error XEN003: Division by Zero
  → tests/example.xen:2:21
  │
  │ spawn result: int = 10 / 0
  │                     ^^^^^^
  = note: division by zero is not allowed
  = help: check if denominator is zero before dividing
```

#### XEN004: Index Out of Bounds
Occurs when accessing a list index that doesn't exist.

```xenith
spawn numbers: list<int> = [1, 2, 3]
spawn value: int = numbers[10]
```

**Error:**
```
Error XEN004: Index Out of Bounds
  → tests/example.xen:3:20
  │
  │ spawn value: int = numbers[10]
  │                    ^^^^^^^^^^^
  = note: list length is `3`, but index `10` was requested
  = help: valid indices are `0` to `2`
```

### I/O Errors

#### XEN005: File Not Found
Occurs when trying to read a file that doesn't exist.

```xenith
grab { read } from "std::fs"
spawn content: string = read("missing.txt")
```

**Error:**
```
Error XEN005: File Not Found
  → tests/example.xen:3:22
  │
  │ spawn content: string = read("missing.txt")
  │                      ^
  = note: attempted to open: `missing.txt`
  = help: check if the file exists and the path is correct
```

#### XEN006: Invalid JSON
Occurs when parsing malformed JSON.

```xenith
grab { parse } from "std::json"
spawn data: json = parse('{"name": "Alice"')
```

**Error:**
```
Error XEN006: Invalid JSON
  → tests/example.xen:3:19
  │
  │ spawn data: json = parse('{"name": "Alice"')
  │                   ^
  = note: the provided string is not valid JSON
  = help: check the JSON syntax
```

#### XEN010: Permission Denied
Occurs when trying to access a file or directory without permission.

```xenith
grab { write } from "std::fs"
write("/root/protected.txt", "content")
```

**Error:**
```
Error XEN010: Permission Denied
  → tests/example.xen:3:1
  │
  │ write("/root/protected.txt", "content")
  │ ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  = note: insufficient permissions to access the file or directory
  = help: check file permissions or run with appropriate privileges
```

### Environment Errors

#### XEN007: Environment Variable Not Found
Occurs when trying to access an environment variable that doesn't exist.

```xenith
grab { get } from "std::dotenv"
spawn value: string = get("UNDEFINED_VAR")
```

**Error:**
```
Error XEN007: Environment Variable Not Found
  → tests/example.xen:3:22
  │
  │ spawn value: string = get("UNDEFINED_VAR")
  │                      ^
  = note: environment variable `UNDEFINED_VAR` is not set
  = help: check if the variable exists or provide a default value
```

### Method/Struct Errors

#### XEN008: Method Not Found
Occurs when calling a method that doesn't exist on a struct.

```xenith
struct Person { name: string }
impl Person {
    method greet(self: Self) -> string { release "Hello" }
}

spawn alice: Person = Person { name: "Alice" }
Person::goodbye(alice)
```

**Error:**
```
Error XEN008: Method Not Found
  → tests/example.xen:8:15
  │
  │ Person::goodbye(alice)
  │        ^^^^^^^
  = note: the struct `Person` has no method named `goodbye`
  = help: check the method name spelling or define the method in an `impl` block
```

#### XEN009: Field Not Found
Occurs when accessing a struct field that doesn't exist.

```xenith
struct Person { name: string }
spawn alice: Person = Person { name: "Alice" }
echo(alice.age)
```

**Error:**
```
Error XEN009: Field Not Found
  → tests/example.xen:4:11
  │
  │ echo(alice.age)
  │           ^^^
  = note: the struct `Person` has no field named `age`
  = help: check the field name spelling
```

### Lexical/Syntax Errors

#### XEN013: Unexpected Token
Occurs when the parser encounters an unexpected token.

```xenith
spawn x: int = 5 6
```

**Error:**
```
Error XEN013: Unexpected Token
  → tests/example.xen:2:17
  │
  │ spawn x: int = 5 6
  │                 ^
  = note: the parser encountered an unexpected token
  = help: check the syntax near this location
```

#### XEN100: Illegal Character
Occurs when the lexer encounters an invalid character.

```xenith
spawn x: int = 5 @ 3
```

**Error:**
```
Error XEN100: Illegal Character
  → tests/example.xen:2:17
  │
  │ spawn x: int = 5 @ 3
  │                 ^
  = note: character `'@'` is not allowed
  = help: remove the illegal character or use a valid one
```

### Function Errors

#### XEN015: Too Many Arguments
Occurs when passing more arguments than a method expects.

```xenith
method add(a: int, b: int) -> int { release a + b }
spawn result: int = add(1, 2, 3)
```

**Error:**
```
Error XEN015: Too Many Arguments
  → tests/example.xen:3:22
  │
  │ spawn result: int = add(1, 2, 3)
  │                      ^^^^^^^^^^^
  = note: the method takes `2` arguments but `3` were provided
  = help: check the method signature and remove extra arguments
```

#### XEN016: Too Few Arguments
Occurs when passing fewer arguments than a method expects.

```xenith
method add(a: int, b: int) -> int { release a + b }
spawn result: int = add(1)
```

**Error:**
```
Error XEN016: Too Few Arguments
  → tests/example.xen:3:22
  │
  │ spawn result: int = add(1)
  │                      ^^^^^
  = note: the method takes `2` arguments but only `1` were provided
  = help: check the method signature and add missing arguments
```

## Error Chaining

When errors are nested (e.g., HTTP request fails, then JSON parse fails), Xenith shows the full chain:

```xenith
try {
    spawn response: string = http::get("https://api.example.com")
    spawn data: json = parse(response)
} catch err {
    echo("Error: {err}")
}
```

**Error:**
```
Error XEN006: Invalid JSON
  → tests/api.xen, line 4, column 10
  |
4 | spawn data: json = parse(response)
  |                     ^^^^^^^^^^^^^^ invalid JSON format
  |
  = note: expected double quote at line 1, column 10
  |
Caused by:
  → Error XEN200: HTTP request failed
    → tests/api.xen, line 3, column 15
    |
  3 | spawn response: string = http::get("https://api.example.com")
    |                          ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ connection timeout
```

## Handling Errors in Code

Use `try`/`catch` to handle errors gracefully:

```xenith
try {
    spawn content: string = read("config.json")
    spawn config: json = parse(content)
} catch err {
    echo("Failed to load config: {err}")
    # Use default configuration
    spawn config: json = parse({ "debug": false })
}
```

## Best Practices

1. **Read the error code** - Look up XEN### in documentation
2. **Check the location** - File, line, and column pinpoint the issue
3. **Read the note** - Explains why the error occurred
4. **Follow the help** - Provides specific action to fix
5. **Use try-catch** - Handle expected errors gracefully

## See Also

- `std::fs` - File I/O errors
- `std::json` - JSON parsing errors
- `std::dotenv` - Environment variable errors
```
