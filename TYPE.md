# Xenith Type System - Complete Type List

## Core Principles

- **Explicit types everywhere** - No inference, full transparency
- **No implicit conversions** - Use `as` for explicit type conversion
- **Mutable by default** - `spawn` creates mutable variables
- **`const spawn` for immutability** - Read-only variables
- **No union types** - Keep it simple and strict
- **Type aliases** - For better code organization

## Primitive Types

| Type | Default Value | Description | Example |
|------|---------------|-------------|---------|
| `int` | `0` | Integer numbers (64-bit signed) | `spawn age: int = 25` |
| `float` | `0.0` | Floating point numbers (64-bit) | `spawn pi: float = 3.14159` |
| `string` | `""` | UTF-8 encoded text | `spawn name: string = "Alice"` |
| `bool` | `false` | Boolean (true/false) | `spawn active: bool = true` |
| `null` | `null` | Null/empty value | `spawn data: null = null` |

## Collection Types

| Type | Default Value | Description | Example |
|------|---------------|-------------|---------|
| `list<T>` | `[]` | Ordered list of type T | `spawn numbers: list<int> = [1, 2, 3]` |
| `map<K, V>` | `{}` | Key-value map with key type K and value type V | `spawn scores: map<string, int> = {"Alice": 100}` |

## Function Type

| Type | Description | Example |
|------|-------------|---------|
| `method(params) -> return` | Function type | `type Callback = method(string) -> void` |

## Function Syntax

### Block Functions (multiple statements)
```xen
method add(a: int, b: int) -> int {
    release a + b
}
```

### Arrow Functions (single expression)
```xen
method double(x: int) -> int => x * 2
```

### Void Functions (no return value)
```xen
method log(message: string) -> null {
    echo(message)
    release null
}
```

**Note:** 
- `->` always denotes the **return type**
- `=>` denotes an **arrow function body** (single expression)
- Arrow functions still specify return type before `=>`
- Block functions use `{ }` with explicit `release`

## Struct Types

| Type | Description | Example |
|------|-------------|---------|
| User-defined structs | Named fields with types | `struct Person { name: string, age: int }` |

## Type Aliases

| Feature | Description | Example |
|---------|-------------|---------|
| `type` alias | Create alternative name for existing type | `type UserId = int` |

## Nested Types - Unlimited Depth

Xenith supports arbitrarily deep nesting of collection types:

```xen
# 2 levels deep
spawn matrix: list<list<int>> = [[1, 2], [3, 4]]
spawn user_data: map<string, list<int>> = {"scores": [100, 90]}

# 3 levels deep
spawn cube: list<list<list<int>>> = [[[1, 2], [3, 4]], [[5, 6], [7, 8]]]
spawn complex: map<string, map<string, list<int>>> = {
    "user1": {"scores": [100, 90]},
    "user2": {"scores": [80, 85]}
}

# 4+ levels - unlimited nesting supported
spawn deep: list<list<list<list<int>>>> = ...
```

## Variable Declaration

### With Initialization
```xen
spawn count: int = 42
spawn name: string = "Alice"
spawn active: bool = true
```

### Without Initialization (uses default values)
```xen
spawn age: int           # age = 0
spawn name: string       # name = ""
spawn active: bool       # active = false
spawn scores: list<int>  # scores = []
```

### Immutable Variables
```xen
const spawn MAX_SIZE: int = 100
const spawn APP_NAME: string = "Xenith"
```

## Type Conversion (Explicit Casting)

Xenith requires explicit type conversion using the `as` keyword:

```xen
# int → float
spawn x: int = 5
spawn y: float = x as float  # 5.0

# float → int (truncates)
spawn a: float = 3.14
spawn b: int = a as int  # 3

# int/float → string
spawn num: int = 42
spawn text: string = num as string  # "42"

# string → int/float (parsing)
spawn input: string = "123"
spawn number: int = input as int  # 123

# bool → int
spawn flag: bool = true
spawn value: int = flag as int  # 1

# int → bool (0 = false, non-zero = true)
spawn count: int = 5
spawn active: bool = count as bool  # true
```

## Type Compatibility Rules

| Operation | Allowed Types | Notes |
|-----------|---------------|-------|
| Arithmetic (`+`, `-`, `*`, `/`, `^`) | `int`, `float` | Both operands must be same type |
| String concatenation (`+`) | `string + string` | - |
| List concatenation (`+`) | `list<T> + list<T>` | Same type T required |
| Comparisons (`==`, `!=`, `<`, `>`, `<=`, `>=`) | Same type for both operands | - |
| Logical (`&&`, `||`, `!`) | `bool` only | - |
| Indexing (`[]`) | `list<T>[int]`, `map<K, V>[K]` | - |

## Type Checking with Nested Collections

When adding elements, the type must match at all levels:

```xen
spawn matrix: list<list<int>> = [[1, 2], [3, 4]]

# ✅ OK - adding list<int>
matrix.append([5, 6])

# ❌ ERROR - wrong inner type
matrix.append([7, "eight"])  # string not allowed in list<int>

# ❌ ERROR - wrong nesting level
matrix.append(10)  # int not allowed, expected list<int>
```

## Complete Type Examples

```xen
# Primitive types
spawn count: int = 42
spawn temperature: float = 98.6
spawn message: string = "Hello"
spawn is_ready: bool = true
spawn empty: null = null

# Collections
spawn numbers: list<int> = [1, 2, 3]
spawn names: list<string> = ["Alice", "Bob"]
spawn matrix: list<list<int>> = [[1, 2], [3, 4]]
spawn user_map: map<string, int> = {"age": 25}
spawn nested_map: map<string, list<int>> = {"scores": [100, 90, 80]}

# Function types
type MathOp = method(int, int) -> int
type Logger = method(string) -> void
type Factory = method() -> Person

# Struct types
struct Person {
    name: string,
    age: int,
    scores: list<int>
}

# Type aliases
type ID = int
type UserMap = map<string, Person>
type Callback = method(Person) -> bool

# Complete example with defaults and nesting
struct GameState {
    score: int,                    # defaults to 0
    player_name: string,           # defaults to ""
    is_active: bool,               # defaults to false
    inventory: list<string>,       # defaults to []
    achievements: map<string, bool> # defaults to {}
}

spawn state: GameState  # All fields have default values
echo("Score: {state.score}")  # 0

# Deep nesting example
spawn world: list<list<list<int>>> = []
spawn layer1: list<list<int>> = []
spawn row: list<int> = [1, 2, 3]

layer1.append(row)
world.append(layer1)
echo("Value: {world[0][0][0]}")  # 1
```

## Future Enhancements (Not Yet Implemented)

### Generic Methods

```xen
# Future feature - not yet implemented
method identity<T>(value: T) -> T {
    release value
}

method first<T>(items: list<T>) -> T {
    release items[0]
}

method map<T, U>(items: list<T>, fn: method(T) -> U) -> list<U> {
    spawn result: list<U> = []
    for item in items {
        result.append(fn(item))
    }
    release result
}
```

## Summary

Xenith's type system is:
- **Strict** - No implicit conversions, all types explicit
- **Safe** - Prevents subtle bugs from automatic type coercion
- **Fast** - Static typing enables optimizations
- **Predictable** - Know your types so you know what to receive and send
- **Complete** - Primitives, collections, structs, and type aliases

This is the complete type system for Xenith - strict, explicit, safe, fast, and predictable!
```
