# Xenith Modules & Imports

## Introduction

Xenith has a module system that allows you to organize code across multiple files. Use `grab` to import items from other modules and `export` to make items available to other modules.

## Module Basics

A module is a `.xen` file that can export functions, constants, and variables for use in other files.

### File Structure Example

```
project/
├── main.xen
├── math.xen
├── strings.xen
└── utils/
    ├── helpers.xen
    └── validators.xen
```

## Exporting Items

Use the `export` keyword to make items available to other modules:

### Exporting Methods

```xenith
# math.xen
export method add(a: int, b: int) -> int {
    release a + b
}

export method subtract(a: int, b: int) -> int {
    release a - b
}

export method multiply(a: int, b: int) -> int {
    release a * b
}

export method divide(a: int, b: int) -> float {
    when b == 0 {
        panic "Division by zero"
    }
    release (a as float) / (b as float)
}
```

### Exporting Constants

```xenith
# constants.xen
export let PI: float = 3.14159
export let E: float = 2.71828
export let APP_NAME: string = "XenithApp"
export let MAX_USERS: int = 1000
```

### Exporting Variables

```xenith
# config.xen
export let debug_mode: bool = false
export let log_level: string = "INFO"
export let api_url: string = "https://api.example.com"
```

## Importing with `grab`

### Import Specific Items

```xenith
# main.xen
grab { add, multiply, PI } from "math"

let sum: int = add(5, 3)           # 8
let product: int = multiply(4, 2)  # 8
echo(PI)                              # 3.14159
```

### Import with Aliases

Use `as` to rename imported items:

```xenith
# main.xen
grab { add as sum, multiply as product, PI as pi_value } from "math"

let result1: int = sum(10, 20)        # 30
let result2: int = product(6, 7)      # 42
echo(pi_value)                           # 3.14159
```

### Import Multiple Items

```xenith
# main.xen
grab { add, subtract, multiply, divide, PI, E } from "math"

let a: int = add(10, 5)        # 15
let b: int = subtract(10, 5)   # 5
let c: int = multiply(10, 5)   # 50
let d: float = divide(10, 3)   # 3.333...
```

## Namespace Imports

Import all exports from a module under a namespace:

```xenith
# main.xen
grab * as math from "math"

let sum: int = math.add(5, 3)        # 8
let difference: int = math.subtract(10, 4)  # 6
let product: int = math.multiply(3, 7)      # 21
echo(math.PI)                         # 3.14159
echo(math.E)                          # 2.71828
```

## Complete Module Examples

### String Utilities Module

```xenith
# strings.xen
export method toUpper(text: string) -> string {
    # Implementation would convert to uppercase
    release text  # Placeholder
}

export method toLower(text: string) -> string {
    # Implementation would convert to lowercase
    release text  # Placeholder
}

export method reverse(text: string) -> string {
    let result: string = ""
    for i = text.len() - 1 to 0 step -1 {
        result = result + (text[i] as string)
    }
    release result
}

export method isPalindrome(text: string) -> bool {
    release reverse(text) == text
}

export method countChars(text: string) -> map<string, int> {
    let counts: map<string, int> = {}
    for i = 0 to text.len() {
        let ch: string = text[i] as string
        when counts.has_key(ch) {
            counts[ch] = counts[ch] + 1
        } otherwise {
            counts[ch] = 1
        }
    }
    release counts
}

export let VERSION: string = "1.0.0"
```

### Math Utilities Module

```xenith
# advanced_math.xen
export method factorial(n: int) -> int {
    when n <= 1 {
        release 1
    }
    release n * factorial(n - 1)
}

export method fibonacci(n: int) -> int {
    when n <= 1 {
        release n
    }
    release fibonacci(n - 1) + fibonacci(n - 2)
}

export method isPrime(n: int) -> bool {
    when n <= 1 {
        release false
    }
    when n == 2 {
        release true
    }
    when n % 2 == 0 {
        release false
    }

    let i: int = 3
    while i * i <= n {
        when n % i == 0 {
            release false
        }
        i = i + 2
    }
    release true
}

export method gcd(a: int, b: int) -> int {
    when b == 0 {
        release a
    }
    release gcd(b, a % b)
}

export method lcm(a: int, b: int) -> int {
    release (a * b) / gcd(a, b)
}

export let PHI: float = 1.618033988749895
```

### Using Multiple Modules

```xenith
# main.xen
grab { toUpper, reverse, isPalindrome, VERSION } from "strings"
grab { factorial, fibonacci, isPrime, gcd, PHI } from "advanced_math"
grab { add, multiply, PI } from "math"

# Test string utilities
let text: string = "racecar"
echo("Original: {text}")
echo("Reversed: {reverse(text)}")
echo("Is palindrome? {isPalindrome(text)}")
echo("String Utils Version: {VERSION}")

# Test math utilities
echo("\nMath Utilities:")
echo("Factorial of 5: {factorial(5)}")      # 120
echo("Fibonacci of 10: {fibonacci(10)}")    # 55
echo("Is 17 prime? {isPrime(17)}")          # true
echo("GCD of 48 and 18: {gcd(48, 18)}")     # 6
echo("Phi: {PHI}")

# Test basic math
echo("\nBasic Math:")
echo("5 + 3 = {add(5, 3)}")                 # 8
echo("4 * 7 = {multiply(4, 7)}")            # 28
echo("PI = {PI}")                           # 3.14159
```

## Namespace Import Example

```xenith
# geometry.xen
export method rectangleArea(width: int, height: int) -> int {
    release width * height
}

export method rectanglePerimeter(width: int, height: int) -> int {
    release 2 * (width + height)
}

export method circleArea(radius: float) -> float {
    release PI * radius * radius
}

export method circleCircumference(radius: float) -> float {
    release 2 * PI * radius
}

export let PI: float = 3.14159
```

```xenith
# main.xen
grab * as geo from "geometry"

let rect_area: int = geo.rectangleArea(10, 5)
let rect_perim: int = geo.rectanglePerimeter(10, 5)
let circ_area: float = geo.circleArea(7.0)
let circ_circum: float = geo.circleCircumference(7.0)

echo("Rectangle area: {rect_area}")           # 50
echo("Rectangle perimeter: {rect_perim}")     # 30
echo("Circle area: {circ_area}")              # 153.938
echo("Circle circumference: {circ_circum}")   # 43.982
echo("PI: {geo.PI}")                          # 3.14159
```

## Module Organization Patterns

### Separate Concerns

```xenith
# database.xen
export method connect(url: string) -> null {
    echo("Connecting to {url}")
    # Connection logic
    release null
}

export method query(sql: string) -> list<map<string, string>> {
    echo("Executing: {sql}")
    # Query logic
    release []
}

export method disconnect() -> null {
    echo("Disconnecting from database")
    release null
}
```

```xenith
# validation.xen
export method isEmail(email: string) -> bool {
    release email.contains("@") && email.contains(".")
}

export method isPhone(phone: string) -> bool {
    let digits: int = 0
    for i = 0 to phone.len() {
        let ch: string = phone[i] as string
        when ch >= "0" && ch <= "9" {
            digits = digits + 1
        }
    }
    release digits == 10
}

export method isPostalCode(code: string) -> bool {
    release code.len() == 5 || code.len() == 9
}
```

```xenith
# main.xen
grab { connect, query, disconnect } from "database"
grab { isEmail, isPhone, isPostalCode } from "validation"

# Validate input
let email: string = "user@example.com"
when isEmail(email) {
    echo("Valid email: {email}")
} otherwise {
    echo("Invalid email: {email}")
}

# Use database
connect("postgres://localhost:5432/mydb")
let results: list<map<string, string>> = query("SELECT * FROM users")
disconnect()
```

### Configuration Module

```xenith
# config.xen
export let APP_NAME: string = "XenithApp"
export let VERSION: string = "1.0.0"
export let DEBUG: bool = false

export method setDebug(enabled: bool) -> null {
    DEBUG = enabled
    echo("Debug mode set to {DEBUG}")
    release null
}

export method getConfig() -> map<string, string> {
    release {
        "app_name": APP_NAME,
        "version": VERSION,
        "debug": (DEBUG as string)
    }
}
```

```xenith
# main.xen
grab * as cfg from "config"

echo("Starting {cfg.APP_NAME} v{cfg.VERSION}")

when cfg.DEBUG {
    echo("Running in debug mode")
}

cfg.setDebug(true)
let config_map: map<string, string> = cfg.getConfig()

for key, value in config_map.items() {
    echo("{key} = {value}")
}
```

## Module Search Path

Xenith looks for modules in:

1. **Current directory** - Relative paths like `"math"`
2. **Subdirectories** - Paths like `"utils/helpers"`
3. **Standard library** - Paths starting with `std::` like `"std::math"`

### Subdirectory Example

```xenith
# utils/helpers.xen
export method formatDate(year: int, month: int, day: int) -> string {
    release "{year}-{month}-{day}"
}

export method sleep(ms: int) -> null {
    # Sleep implementation
    release null
}
```

```xenith
# main.xen
grab { formatDate, sleep } from "utils/helpers"

let today: string = formatDate(2024, 4, 8)
echo("Today is {today}")
```

## Best Practices

1. **Export only what's needed** - Keep module interfaces minimal
2. **Use descriptive module names** - Match filenames to their purpose
3. **Group related functionality** - Each module should have a single responsibility
4. **Use namespaces for large modules** - Prevents naming conflicts
5. **Document exported items** - Add comments for public API

```xenith
# Good - focused module
# user.xen
export method create(name: string, email: string) -> User { ... }
export method find(id: int) -> User { ... }
export method delete(id: int) -> null { ... }

# Avoid - too many unrelated exports
# everything.xen
export method add(a: int, b: int) -> int { ... }
export method formatDate(date: string) -> string { ... }
export method sendEmail(to: string, msg: string) -> null { ... }
```

## Common Patterns

### Re-exporting

```xenith
# math/index.xen
grab { add, subtract } from "math/basic"
grab { factorial, fibonacci } from "math/advanced"

export { add, subtract, factorial, fibonacci }
```

```xenith
# main.xen
# Now you can import everything from one place
grab { add, factorial } from "math/index"
```

### Conditional Imports

```xenith
# config.xen
export let USE_ADVANCED_MATH: bool = true

# main.xen
grab { USE_ADVANCED_MATH } from "config"

when USE_ADVANCED_MATH {
    grab { factorial, fibonacci } from "advanced_math"
} otherwise {
    grab { add, multiply } from "basic_math"
}
```

## Error Handling with Imports

```xenith
# Try to import, handle if module not found
try {
    grab { fastSort } from "optimized_algorithms"
    echo("Using optimized algorithms")
} catch err {
    echo("Falling back to basic algorithms")
    grab { bubbleSort } from "basic_algorithms"
}
```

## Next Steps

- Learn about [STRUCTS.md](STRUCTS.md) for creating custom types
- Read [ERROR_HANDLING.md](ERROR_HANDLING.md) for robust error management
- Explore [BUILT-IN_METHODS.md](BUILT-IN_METHODS.md) for available utilities

```

```
