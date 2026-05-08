# Xenith String Features

## Introduction

Xenith provides powerful string manipulation features including concatenation, interpolation, and repetition. Strings are UTF-8 encoded and support various operations.

## String Literals

Strings are defined using double quotes:

```xenith
let greeting: string = "Hello, World!"
let name: string = "Alice"
let empty: string = ""
let with_quotes: string = 'She said "Hello"'  # Single quotes work too
```

## String Concatenation

Use the `+` operator to combine strings:

### Basic Concatenation

```xenith
let first: string = "Hello"
let second: string = "World"
let result: string = first + " " + second
echo(result)  # Hello World

let greeting: string = "Good" + " " + "Morning"
echo(greeting)  # Good Morning
```

### Concatenating Different Types

```xenith
let name: string = "Alice"
let age: int = 25

# Numbers are automatically converted
let message: string = name + " is " + (age as string) + " years old"
echo(message)  # Alice is 25 years old

# Float concatenation
let price: float = 19.99
let text: string = "Price: " + (price as string)
echo(text)  # Price: 19.99
```

### Chained Concatenation

```xenith
let result: string = "The " + "quick " + "brown " + "fox"
echo(result)  # The quick brown fox

# Building longer strings
let html: string = "<div>" +
                     "<h1>Welcome</h1>" +
                     "<p>Hello, World!</p>" +
                     "</div>"
echo(html)
```

## String Interpolation

Use `{expression}` inside strings to embed values directly:

### Basic Interpolation

```xenith
let name: string = "Alice"
let age: int = 25

echo("Hello, {name}!")           # Hello, Alice!
echo("You are {age} years old")  # You are 25 years old
```

### Multiple Interpolations

```xenith
let first: string = "John"
let last: string = "Doe"
let age: int = 30
let city: string = "New York"

echo("{first} {last} is {age} years old and lives in {city}")
# John Doe is 30 years old and lives in New York
```

### Expressions in Interpolation

```xenith
let x: int = 10
let y: int = 20

echo("{x} + {y} = {x + y}")      # 10 + 20 = 30
echo("{x} * {y} = {x * y}")      # 10 * 20 = 200
echo("{x} ^ {y} = {x ^ y}")      # 10 ^ 20 = 100000000000000000000

let price: float = 19.99
let quantity: int = 3
echo("Total: ${price * quantity}")  # Total: 59.97
```

### Method Calls in Interpolation

```xenith
method double(n: int) -> int => n * 2
method greet(name: string) -> string => "Hello, " + name

let value: int = 5
echo("Double of {value} is {double(value)}")  # Double of 5 is 10

let user: string = "Alice"
echo("{greet(user)}")  # Hello, Alice
```

### Complex Interpolations

```xenith
# Conditions in interpolation
let age: int = 18
let status: string = "{age >= 18 ? "Adult" : "Minor"}"
echo("Status: {status}")  # Status: Adult

# Multiple expressions
let a: int = 5
let b: int = 3
echo("{a} + {b} = {a + b}, {a} * {b} = {a * b}")
# 5 + 3 = 8, 5 * 3 = 15

# List access in interpolation
let fruits: list<string> = ["apple", "banana", "orange"]
echo("My favorite fruit is {fruits[1]}")  # My favorite fruit is banana
```

## String Repetition

Use the `*` operator to repeat strings:

### Basic Repetition

```xenith
let dash: string = "-" * 10
echo(dash)  # ----------

let star: string = "*" * 5
echo(star)  # *****

let pattern: string = "AB" * 3
echo(pattern)  # ABABAB
```

### Practical Repetition Examples

```xenith
# Creating separators
method printSeparator() -> null {
    let line: string = "=" * 50
    echo(line)
    release null
}

printSeparator()  # ==================================================
echo("Header")
printSeparator()

# Creating indentation
method printIndented(text: string, level: int) -> null {
    let indent: string = "  " * level
    echo("{indent}{text}")
    release null
}

printIndented("Level 1", 1)    #   Level 1
printIndented("Level 2", 2)    #     Level 2
printIndented("Level 3", 3)    #       Level 3

# Creating progress bars
method showProgress(percent: int) -> null {
    let filled: int = percent / 2
    let empty: int = 50 - filled
    let bar: string = "█" * filled + "░" * empty
    echo("[{bar}] {percent}%")
    release null
}

showProgress(25)  # [████████████░░░░░░░░░░░░░░░░░░░░░░░░░░░░] 25%
showProgress(50)  # [████████████████████████░░░░░░░░░░░░░░░░░░] 50%
showProgress(75)  # [████████████████████████████████████░░░░░░] 75%
showProgress(100) # [████████████████████████████████████████████] 100%
```

## Advanced String Examples

### Formatted Table

```xenith
method printTable() -> null {
    let header: string = "Name" + " " * 10 + "Age" + " " * 5 + "City"
    let separator: string = "-" * len(header)

    echo(header)
    echo(separator)
    echo("Alice" + " " * 12 + "25" + " " * 8 + "NYC")
    echo("Bob" + " " * 14 + "30" + " " * 8 + "LA")
    echo("Charlie" + " " * 10 + "35" + " " * 8 + "CHI")
    release null
}

printTable()
# Output:
# Name          Age     City
# ---------------------------
# Alice         25      NYC
# Bob           30      LA
# Charlie       35      CHI
```

### Dynamic String Building

```xenith
method buildList(items: list<string>) -> string {
    let result: string = "Items:\n"
    let separator: string = "-" * 20 + "\n"

    result = result + separator

    for i = 0 to items.len() {
        result = result + "{i + 1}. {items[i]}\n"
    }

    result = result + separator
    release result
}

let shopping: list<string> = ["Apples", "Bananas", "Oranges", "Grapes"]
echo(buildList(shopping))
# Output:
# Items:
# --------------------
# 1. Apples
# 2. Bananas
# 3. Oranges
# 4. Grapes
# --------------------
```

### Text Formatting Utilities

```xenith
method center(text: string, width: int) -> string {
    let padding: int = (width - len(text)) / 2
    when padding < 0 {
        padding = 0
    }
    release (" " * padding) + text + (" " * padding)
}

method padLeft(text: string, width: int) -> string {
    let padding: int = width - len(text)
    when padding < 0 {
        padding = 0
    }
    release (" " * padding) + text
}

method padRight(text: string, width: int) -> string {
    let padding: int = width - len(text)
    when padding < 0 {
        padding = 0
    }
    release text + (" " * padding)
}

# Usage examples
let title: string = "Welcome"
echo(center(title, 30))
# Output:         Welcome

echo(padLeft("123", 10))   #        123
echo(padRight("123", 10))  # 123
```

### Text Box Generator

```xenith
method textBox(text: string) -> null {
    let width: int = len(text) + 4
    let top_bottom: string = "+" + ("-" * width) + "+"
    let middle: string = "|  " + text + "  |"

    echo(top_bottom)
    echo(middle)
    echo(top_bottom)
    release null
}

textBox("Hello, World!")
# Output:
# +-----------------+
# |  Hello, World!  |
# +-----------------+

textBox("Xenith")
# Output:
# +----------+
# |  Xenith  |
# +----------+
```

### Name Formatter

```xenith
method formatName(first: string, last: string, include_middle: bool, middle: string) -> string {
    let result: string = ""

    when include_middle {
        result = "{first} {middle} {last}"
    } otherwise {
        result = "{first} {last}"
    }

    release result
}

method getInitials(name: string) -> string {
    let parts: list<string> = name.split(" ")
    let initials: string = ""

    for part in parts {
        when len(part) > 0 {
            initials = initials + (part[0] as string) + "."
        }
    }

    release initials
}

let full_name: string = formatName("John", "Doe", true, "Q")
echo(full_name)  # John Q Doe
echo("Initials: {getInitials(full_name)}")  # Initials: J.Q.D.
```

### URL Builder

```xenith
method buildURL(base: string, endpoint: string, params: map<string, string>) -> string {
    let url: string = base + "/" + endpoint

    when params.len() > 0 {
        url = url + "?"
        let first: bool = true

        for key, value in params.items() {
            when !first {
                url = url + "&"
            }
            url = url + "{key}={value}"
            first = false
        }
    }

    release url
}

let api_url: string = buildURL(
    "https://api.example.com",
    "users",
    {"page": "1", "limit": "10", "sort": "name"}
)
echo(api_url)
# Output: https://api.example.com/users?page=1&limit=10&sort=name
```

### Email Template

```xenith
method generateEmail(name: string, order_id: string, total: float) -> string {
    let border: string = "=" * 50
    let greeting: string = "Dear {name},"
    let message: string = "Thank you for your order! Order #{order_id} has been confirmed."
    let amount: string = "Total amount: ${total}"
    let footer: string = "Best regards,\nCustomer Support"

    release "\n{border}\n{greeting}\n\n{message}\n{amount}\n\n{footer}\n{border}"
}

let email: string = generateEmail("Alice", "ORD-12345", 299.99)
echo(email)
# Output:
# ==================================================
# Dear Alice,
#
# Thank you for your order! Order #ORD-12345 has been confirmed.
# Total amount: $299.99
#
# Best regards,
# Customer Support
# ==================================================
```

### JSON String Builder

```xenith
method buildJSON(name: string, age: int, city: string) -> string {
    release '{{"name": "{name}", "age": {age}, "city": "{city}"}}'
}

let json: string = buildJSON("Alice", 25, "New York")
echo(json)  # {"name": "Alice", "age": 25, "city": "New York"}

# More complex JSON
method buildUserJSON(id: int, email: string, active: bool) -> string {
    let active_str: string = active ? "true" : "false"
    release '{{"id": {id}, "email": "{email}", "active": {active_str}}}'
}

echo(buildUserJSON(1001, "alice@example.com", true))
# {"id": 1001, "email": "alice@example.com", "active": true}
```

### SQL Query Builder

```xenith
method buildSelectQuery(table: string, columns: list<string>, where: map<string, string>) -> string {
    # Build columns part
    let cols: string = ""
    for i = 0 to columns.len() {
        when i > 0 {
            cols = cols + ", "
        }
        cols = cols + columns[i]
    }

    # Build query
    let query: string = "SELECT {cols} FROM {table}"

    # Add WHERE clause if needed
    when where.len() > 0 {
        query = query + " WHERE "
        let first: bool = true

        for key, value in where.items() {
            when !first {
                query = query + " AND "
            }
            query = query + "{key} = '{value}'"
            first = false
        }
    }

    query = query + ";"
    release query
}

let query: string = buildSelectQuery(
    "users",
    ["id", "name", "email"],
    {"active": "true", "city": "NYC"}
)
echo(query)
# SELECT id, name, email FROM users WHERE active = 'true' AND city = 'NYC';
```

## Escape Sequences

```xenith
# Newline
echo("Line 1\nLine 2")
# Output:
# Line 1
# Line 2

# Tab
echo("Column 1\tColumn 2")
# Output: Column 1    Column 2

# Quotes inside strings
echo("She said \"Hello\"")
# Output: She said "Hello"

# Backslash
echo("Path: C:\\Users\\Name")
# Output: Path: C:\Users\Name

# Curly braces in strings (escape with double braces)
echo("{{Hello}}")  # Output: {Hello}
echo("{{name}}")   # Output: {name}
```

## String Methods

```xenith
# Get string length
let text: string = "Hello"
let length: int = len(text)
echo(length)  # 5

# String repetition (works with any string)
let repeated: string = "Ha" * 3
echo(repeated)  # HaHaHa

# String indexing (access individual characters)
let text: string = "Hello"
echo(text[0])  # H
echo(text[1])  # e
echo(text[4])  # o
```

## Best Practices

1. **Use interpolation over concatenation** - More readable and efficient
2. **Avoid excessive repetition** - Don't repeat strings thousands of times
3. **Escape properly** - Use `{{` and `}}` for literal braces
4. **Format for readability** - Use spaces and line breaks in complex interpolations

```xenith
# Good - readable interpolation
let message: string = "User {name} is {age} years old"

# Acceptable - simple concatenation
let result: string = "Hello" + " " + "World"

# Avoid - overly complex concatenation
let bad: string = "The " + "quick " + "brown " + "fox " + "jumps " + "over"
```

## Common Patterns

### Greeting Generator

```xenith
method getGreeting(name: string, hour: int) -> string {
    when hour < 12 {
        release "Good morning, {name}!"
    } or when hour < 18 {
        release "Good afternoon, {name}!"
    } otherwise {
        release "Good evening, {name}!"
    }
}

let username: string = "Alice"
let current_hour: int = 14
echo(getGreeting(username, current_hour))  # Good afternoon, Alice!
```

### Progress Indicator

```xenith
method showSpinner(iteration: int) -> null {
    let frames: list<string> = ["|", "/", "-", "\\"]
    let frame: string = frames[iteration % 4]
    echo("\rProcessing {frame} ", false)  # false = no newline
    release null
}

# Simulate processing
for i = 0 to 20 {
    showSpinner(i)
    # Simulate work
    for delay = 0 to 10000 {
        # Waste time
    }
}
echo("\nDone!")
```

## Next Steps

- Learn about [COLLECTIONS.md](COLLECTIONS.md) for list and map operations
- Read [BUILT-IN_METHODS.md](BUILT-IN_METHODS.md) for string-related methods
- Explore [OPERATORS.md](OPERATORS.md) for more string operations

```

```
