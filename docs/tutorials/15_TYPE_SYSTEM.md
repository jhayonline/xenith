# Xenith Type System

## Introduction

Xenith is a **statically and strongly typed** language. Every variable, parameter, and return value must have an explicit type annotation. The type system includes primitive types, generic collections, user-defined structs, method types, and type aliases.

## Core Philosophy

**Explicit is safe** - Types are never inferred. You write them, you own them. If your code parses, it's type-safe.

## Primitive Data Types

| Type | Description | Example |
|------|-------------|---------|
| `int` | 64-bit signed integer | `-42`, `0`, `1000` |
| `float` | 64-bit floating point | `3.14`, `-0.5`, `2.0` |
| `string` | UTF-8 text | `"Hello"`, `""`, `"123"` |
| `bool` | Boolean true/false | `true`, `false` |
| `null` | Absence of value | `null` |

```xenith
spawn age: int = 25
spawn price: float = 19.99
spawn name: string = "Alice"
spawn active: bool = true
spawn nothing: null = null
```

## Generic Types

### Lists: `list<T>`

Lists are ordered collections where all elements must be of the same type `T`.

```xenith
# List of integers
spawn numbers: list<int> = [1, 2, 3, 4, 5]

# List of strings
spawn fruits: list<string> = ["apple", "banana", "orange"]

# List of floats
spawn prices: list<float> = [19.99, 29.99, 39.99]

# List of booleans
spawn flags: list<bool> = [true, false, true]

# Nested list (list of lists)
spawn matrix: list<list<int>> = [[1, 2], [3, 4], [5, 6]]

# Empty list
spawn empty: list<int> = []
```

### Maps: `map<K, V>`

Maps are key-value pairs where keys are strings and values are of type `V`.

```xenith
# String to int map
spawn ages: map<string, int> = {
    "Alice": 25,
    "Bob": 30,
    "Charlie": 35
}

# String to string map
spawn capitals: map<string, string> = {
    "France": "Paris",
    "Japan": "Tokyo",
    "Brazil": "Brasilia"
}

# String to list map
spawn scores: map<string, list<int>> = {
    "Alice": [95, 87, 92],
    "Bob": [78, 88, 91]
}

# String to map map (nested)
spawn users: map<string, map<string, string>> = {
    "alice@email.com": {
        "name": "Alice",
        "city": "New York"
    }
}

# Empty map
spawn empty: map<string, int> = {}
```

### Using Generic Types

```xenith
# Type safety is enforced
spawn numbers: list<int> = [1, 2, 3]
# numbers.append("four")  # Error! String not allowed

spawn ages: map<string, int> = {"Alice": 25}
# ages["Bob"] = "thirty"  # Error! String not allowed

# Generic methods work with any type
method getFirst<T>(items: list<T>) -> T {
    when items.len() == 0 {
        panic "List is empty"
    }
    release items[0]
}

spawn first_num: int = getFirst([1, 2, 3])        # Returns int
spawn first_str: string = getFirst(["a", "b"])    # Returns string
```

## Type Aliases

Use `type` to create alternative names for existing types.

### Basic Type Aliases

```xenith
# Create aliases for primitive types
type UserId = int
type UserName = string
type Score = float

# Use aliases in declarations
spawn id: UserId = 1001
spawn name: UserName = "Alice"
spawn score: Score = 95.5

# Aliases in method signatures
method findUser(id: UserId) -> UserName {
    when id == 1001 {
        release "Alice"
    }
    release "Unknown"
}

echo(findUser(1001))  # Alice
```

### Complex Type Aliases

```xenith
# Alias for list type
type IntList = list<int>
type StringList = list<string>
type NumberGrid = list<list<int>>

spawn numbers: IntList = [1, 2, 3, 4, 5]
spawn names: StringList = ["Alice", "Bob", "Charlie"]
spawn matrix: NumberGrid = [[1, 2], [3, 4]]

# Alias for map type
type ScoreMap = map<string, int>
type ConfigMap = map<string, string>

spawn scores: ScoreMap = {"Alice": 95, "Bob": 87}
spawn config: ConfigMap = {"host": "localhost", "port": "8080"}

# Alias for nested structures
type UserScores = map<string, list<int>>
spawn user_scores: UserScores = {
    "Alice": [95, 87, 92],
    "Bob": [78, 88, 91]
}
```

### Type Aliases in Methods

```xenith
type Callback = method(int) -> string
type Predicate = method(int) -> bool
type Transformer = method(int) -> int

method processNumbers(numbers: list<int>, callback: Callback) -> list<string> {
    spawn result: list<string> = []
    for n in numbers {
        result.append(callback(n))
    }
    release result
}

method toString(n: int) -> string => "Number: {n}"
method isEven(n: int) -> bool => n % 2 == 0
method double(n: int) -> int => n * 2

spawn nums: list<int> = [1, 2, 3, 4, 5]
spawn strings: list<string> = processNumbers(nums, toString)
echo(ret(strings))  # ["Number: 1", "Number: 2", ...]
```

## Method Types

Method types define the signature of a method (parameters and return type).

### Basic Method Types

```xenith
# Method taking no parameters, returning int
type ZeroArgMethod = method() -> int

# Method taking one int, returning string
type IntToString = method(int) -> string

# Method taking two ints, returning int
type BinaryOp = method(int, int) -> int

# Method taking string and bool, returning null
type Validator = method(string, bool) -> null
```

### Using Method Types

```xenith
# Define methods that match the type
method getRandom() -> int {
    release 42
}

method intToString(n: int) -> string {
    release "Value: {n}"
}

method add(a: int, b: int) -> int => a + b
method multiply(a: int, b: int) -> int => a * b

# Store methods in variables
spawn zero: ZeroArgMethod = getRandom
spawn converter: IntToString = intToString
spawn operation: BinaryOp = add

# Call through the variable
echo(zero())                # 42
echo(converter(100))        # Value: 100
echo(operation(5, 3))       # 8

# Change method at runtime
operation = multiply
echo(operation(5, 3))       # 15
```

### Method Types as Parameters

```xenith
type MapFunction = method(int) -> int
type FilterFunction = method(int) -> bool

method mapList(numbers: list<int>, func: MapFunction) -> list<int> {
    spawn result: list<int> = []
    for n in numbers {
        result.append(func(n))
    }
    release result
}

method filterList(numbers: list<int>, predicate: FilterFunction) -> list<int> {
    spawn result: list<int> = []
    for n in numbers {
        when predicate(n) {
            result.append(n)
        }
    }
    release result
}

method double(x: int) -> int => x * 2
method square(x: int) -> int => x * x
method isEven(x: int) -> bool => x % 2 == 0

spawn numbers: list<int> = [1, 2, 3, 4, 5]

spawn doubled: list<int> = mapList(numbers, double)
spawn squared: list<int> = mapList(numbers, square)
spawn evens: list<int> = filterList(numbers, isEven)

echo(ret(doubled))  # [2, 4, 6, 8, 10]
echo(ret(squared))  # [1, 4, 9, 16, 25]
echo(ret(evens))    # [2, 4]
```

### Method Type Composition

```xenith
type Transformer = method(int) -> int
type Stringifier = method(int) -> string

method compose(transform: Transformer, stringify: Stringifier) -> method(int) -> string {
    method composed(x: int) -> string {
        spawn transformed: int = transform(x)
        release stringify(transformed)
    }
    release composed
}

method double(x: int) -> int => x * 2
method format(x: int) -> string => "Result: {x}"

spawn doubleThenFormat: method(int) -> string = compose(double, format)
echo(doubleThenFormat(5))  # Result: 10
```

## Struct Types

Structs are user-defined composite types that group related data.

### Basic Struct Definition

```xenith
struct Person {
    name: string,
    age: int
}

struct Point {
    x: float,
    y: float
}

struct Rectangle {
    width: int,
    height: int
}

# Using struct types
spawn alice: Person = Person { name: "Alice", age: 25 }
spawn point: Point = Point { x: 10.5, y: 20.3 }
spawn rect: Rectangle = Rectangle { width: 100, height: 50 }
```

### Structs with Generic Fields

```xenith
# Struct with list field
struct Classroom {
    name: string,
    students: list<string>,
    scores: map<string, int>
}

# Struct containing other structs
struct Address {
    street: string,
    city: string,
    zip: string
}

struct Employee {
    name: string,
    address: Address,
    salary: float
}

# Usage
spawn addr: Address = Address {
    street: "123 Main St",
    city: "Springfield",
    zip: "12345"
}

spawn emp: Employee = Employee {
    name: "Alice",
    address: addr,
    salary: 75000.0
}
```

### Struct Type Safety

```xenith
struct User { id: int, name: string }
struct Product { id: int, name: string, price: float }

# Type mismatch - different struct types
spawn user: User = User { id: 1, name: "Alice" }
# spawn product: Product = user  # Error! Types don't match

# Even with same field structure, types are distinct
spawn user2: User = User { id: 2, name: "Bob" }  # OK
# spawn product: Product = User { id: 1, name: "Item" }  # Error!
```

## Method Type Signatures

### Defining Method Types

```xenith
# Various method type signatures
type NoArgs = method() -> int
type OneArg = method(string) -> bool
type TwoArgs = method(int, int) -> int
type VoidMethod = method(string) -> null
type MethodWithList = method(list<int>) -> string
type MethodWithMap = method(map<string, int>) -> bool
```

### Complex Method Types

```xenith
# Method returning a list of methods
type IntProcessor = method(int) -> int
type ProcessorList = method() -> list<IntProcessor>

# Method taking a method parameter
type Callback = method(string) -> null
type EventHandler = method(Callback) -> null

# Method returning a method
type StringTransformer = method(string) -> string
type TransformerFactory = method(int) -> StringTransformer

# Example implementation
method createMultiplier(factor: int) -> StringTransformer {
    method multiplyAndFormat(x: string) -> string {
        spawn num: int = x as int
        release (num * factor) as string
    }
    release multiplyAndFormat
}

spawn double: StringTransformer = createMultiplier(2)
echo(double("5"))  # 10
```

## Type Compatibility

### Numeric Types

```xenith
# int and float are not directly compatible
spawn i: int = 42
# spawn f: float = i  # Error! Use 'as' for conversion
spawn f: float = i as float  # OK

# Operations require same type
spawn a: int = 5
spawn b: float = 3.14
# spawn c: int = a + b  # Error! Cannot add int and float
spawn c: float = (a as float) + b  # OK
```

### Collection Types

```xenith
# List types must match exactly
spawn ints: list<int> = [1, 2, 3]
# spawn strings: list<string> = ints  # Error!

# Generic parameters must match
spawn list1: list<list<int>> = [[1, 2], [3, 4]]
# spawn list2: list<list<float>> = list1  # Error! Different inner types

# Map types must match exactly
spawn ages: map<string, int> = {"Alice": 25}
# spawn scores: map<string, float> = ages  # Error! Different value types
```

## Type Inference (Limited)

Xenith does NOT infer types from values. Types must always be explicit:

```xenith
# This is REQUIRED - explicit type
spawn x: int = 5

# This would NOT work - type inference not supported
# spawn y = 5  # Error! Type annotation required

# Exception: Type aliases still need explicit base type
type UserId = int
spawn id: UserId = 1001  # OK - explicit type annotation
```

## Type Checking Examples

### Compile-Time Type Safety

```xenith
# These errors are caught at parse time - code won't run

# Wrong type assignment
spawn x: int = "hello"  # Error: Expected int, got string

# Wrong method parameter type
method greet(name: string) -> string { release "Hello, {name}" }
spawn result: string = greet(42)  # Error: Expected string, got int

# Wrong return type
method getNumber() -> int {
    release "not a number"  # Error: Expected int, got string
}

# Wrong list element type
spawn numbers: list<int> = [1, "two", 3]  # Error: String in int list

# Wrong map value type
spawn ages: map<string, int> = {"Alice": "twenty-five"}  # Error: String instead of int
```

## Complete Example: Generic Data Processor

```xenith
# Type aliases for clarity
type IntList = list<int>
type StringList = list<string>
type FilterFunction<T> = method(T) -> bool
type TransformFunction<T, U> = method(T) -> U

# Generic filter method
method filter<T>(items: list<T>, predicate: FilterFunction<T>) -> list<T> {
    spawn result: list<T> = []
    for item in items {
        when predicate(item) {
            result.append(item)
        }
    }
    release result
}

# Generic map method
method map<T, U>(items: list<T>, transform: TransformFunction<T, U>) -> list<U> {
    spawn result: list<U> = []
    for item in items {
        result.append(transform(item))
    }
    release result
}

# Specific predicate methods
method isEven(n: int) -> bool => n % 2 == 0
method isLongString(s: string) -> bool => len(s) > 5
method isPositive(n: int) -> bool => n > 0

# Specific transform methods
method toString(n: int) -> string => "Number: {n}"
method toLength(s: string) -> int => len(s)

# Using the generic methods
spawn numbers: IntList = [-5, -2, 0, 3, 7, 10, 15]
spawn words: StringList = ["a", "hello", "world", "xenith", "rust"]

# Filtering
spawn evens: IntList = filter(numbers, isEven)
spawn long_words: StringList = filter(words, isLongString)
spawn positives: IntList = filter(numbers, isPositive)

echo("Even numbers: {ret(evens)}")        # [0, 10]
echo("Long words: {ret(long_words)}")     # ["hello", "world", "xenith"]
echo("Positive numbers: {ret(positives)}") # [3, 7, 10, 15]

# Mapping
spawn number_strings: StringList = map(numbers, toString)
spawn word_lengths: IntList = map(words, toLength)

echo("Numbers as strings: {ret(number_strings)}")
echo("Word lengths: {ret(word_lengths)}")
```

## Type System Summary

| Feature | Syntax | Example |
|---------|--------|---------|
| Primitive types | `int`, `float`, `string`, `bool`, `null` | `spawn x: int = 5` |
| List type | `list<T>` | `spawn nums: list<int> = [1, 2]` |
| Map type | `map<K, V>` | `spawn ages: map<string, int> = {}` |
| Type alias | `type Name = Type` | `type UserId = int` |
| Method type | `method(params) -> return` | `type Op = method(int) -> int` |
| Struct type | `struct Name { fields }` | `struct Point { x: int, y: int }` |

## Best Practices

1. **Always specify types** - No inference, be explicit
2. **Use type aliases for complex types** - Improves readability
3. **Keep structs focused** - One clear purpose per struct
4. **Use generic types appropriately** - `list<T>` not `list<any>`
5. **Document method types** - Clear signatures help understanding

```xenith
# Good - clear, explicit types
type UserData = map<string, string>
type UserProcessor = method(UserData) -> bool

# Good - focused struct
struct Config {
    host: string,
    port: int,
    timeout: int
}

# Avoid - overly complex inline types
# spawn callback: method(map<string, list<method(int) -> string>>) -> null
```

## Next Steps

- Learn about [STRUCTS.md](STRUCTS.md) for custom data types
- Read [METHODS.md](METHODS.md) for method definitions
- Explore [COLLECTIONS.md](COLLECTIONS.md) for working with generic types
```
