# Xenith Methods

## What are Methods?

Methods are reusable blocks of code that perform specific tasks. In Xenith, they are defined using the `method` keyword and can optionally return values.

## Defining a Method

### Basic Syntax

```xenith
method methodName(param: type, param2: type) -> returnType {
    # method body
    release value
}
```

### Simple Example

```xenith
method greet(name: string) -> string {
    release "Hello, " + name
}

spawn message: string = greet("Alice")
echo(message)  # Hello, Alice
```

## Parameters

All parameters require explicit type annotations:

```xenith
method add(a: int, b: int) -> int {
    release a + b
}

method introduce(age: int, name: string) -> string {
    release name + " is " + (age as string) + " years old"
}
```

## Return Values with `release`

Use the `release` keyword to return a value from a method:

```xenith
method double(x: int) -> int {
    release x * 2
}

method isEven(x: int) -> bool {
    release x % 2 == 0
}

method getPi() -> float {
    release 3.14159
}
```

## Void Methods (Returning `null`)

If a method doesn't return a meaningful value, use `null` as the return type:

```xenith
method logMessage(msg: string) -> null {
    echo("LOG: {msg}")
    release null
}

logMessage("Application started")
```

## Arrow Methods `=>`

For simple one-line methods, use the arrow syntax:

```xenith
method double(x: int) -> int => x * 2
method square(x: int) -> int => x ^ 2
method isPositive(x: int) -> bool => x > 0

# These are equivalent to:
method double(x: int) -> int {
    release x * 2
}
```

## Calling Methods

```xenith
method add(a: int, b: int) -> int {
    release a + b
}

method multiply(a: int, b: int) -> int => a * b

spawn sum: int = add(5, 3)           # 8
spawn product: int = multiply(4, 2)  # 8

echo(add(10, 20))  # 30
```

## Multiple Parameters

Methods can have any number of parameters:

```xenith
method calculate(a: int, b: int, c: int, d: int) -> int {
    release (a + b) * (c - d)
}

method formatData(name: string, age: int, city: string) -> string {
    release "{name} is {age} years old from {city}"
}
```

## Recursion

Methods can call themselves (recursion):

```xenith
method factorial(n: int) -> int {
    when n <= 1 {
        release 1
    }
    release n * factorial(n - 1)
}

echo(factorial(5))  # 120

method fibonacci(n: int) -> int {
    when n <= 1 {
        release n
    }
    release fibonacci(n - 1) + fibonacci(n - 2)
}

echo(fibonacci(7))  # 13
```

## Method Types

Methods can be stored in variables using method types:

```xenith
# Define a method type
type BinaryOperation = method(int, int) -> int

# Methods that match the type
method add(a: int, b: int) -> int => a + b
method multiply(a: int, b: int) -> int => a * b

# Store method in variable
spawn operation: BinaryOperation = add
echo(operation(5, 3))  # 8

operation = multiply
echo(operation(5, 3))  # 15
```

### Method Type Syntax

| Pattern | Meaning |
|---------|---------|
| `method() -> int` | Takes no parameters, returns int |
| `method(int) -> string` | Takes one int, returns string |
| `method(int, string) -> bool` | Takes int and string, returns bool |
| `method(list<int>) -> null` | Takes list of ints, returns null |

## Method Scope

Variables inside a method are local and don't affect outer scopes:

```xenith
spawn x: int = 10

method changeX() -> null {
    spawn x: int = 20  # Local variable
    echo("Inside method: {x}")  # 20
    release null
}

changeX()
echo("Outside method: {x}")  # 10 (unchanged)
```

However, you can modify outer variables by reassigning them (without `spawn`):

```xenith
spawn counter: int = 0

method increment() -> null {
    counter = counter + 1  # Modifies outer variable
    release null
}

increment()
increment()
echo(counter)  # 2
```

## Early Returns

Use `release` anywhere in a method to exit early:

```xenith
method findFirstEven(numbers: list<int>) -> int {
    for n in numbers {
        when n % 2 == 0 {
            release n  # Exit immediately when found
        }
    }
    release -1  # Not found
}

spawn nums: list<int> = [1, 3, 5, 6, 7, 8]
echo(findFirstEven(nums))  # 6
```

## Method Overloading

Xenith does **not** support method overloading (multiple methods with the same name but different parameters). Each method name must be unique.

## Complete Examples

### Calculator Methods

```xenith
method add(a: int, b: int) -> int => a + b
method subtract(a: int, b: int) -> int => a - b
method multiply(a: int, b: int) -> int => a * b
method divide(a: int, b: int) -> float => (a as float) / (b as float)

spawn x: int = 10
spawn y: int = 3

echo(add(x, y))       # 13
echo(subtract(x, y))  # 7
echo(multiply(x, y))  # 30
echo(divide(x, y))    # 3.333...
```

### String Utilities

```xenith
method repeat(text: string, times: int) -> string {
    spawn result: string = ""
    for i = 0 to times {
        result = result + text
    }
    release result
}

method isPalindrome(text: string) -> bool {
    spawn reversed: string = ""
    for i = text.len() - 1 to 0 step -1 {
        reversed = reversed + text[i] as string
    }
    release text == reversed
}

echo(repeat("ha", 3))           # hahaha
echo(isPalindrome("radar"))     # true
echo(isPalindrome("hello"))     # false
```

## Best Practices

1. **Use descriptive names** - `calculateAverage`, not `calc`
2. **Keep methods focused** - One method should do one thing
3. **Add type annotations** - Always specify parameter and return types
4. **Use arrow methods for simple logic** - Improves readability
5. **Add comments for complex logic** - Explain why, not what

## Next Steps

- Learn about [STRUCTS.md](STRUCTS.md) to create custom data types
- Read [IMPL.md](IMPL.md) to attach methods to structs
- Explore [COLLECTIONS.md](COLLECTIONS.md) for lists and maps
```
