# Xenith Control Flow

## Introduction

Control flow statements determine the order in which code executes. Xenith provides conditionals (`when`/`or when`/`otherwise`), ternary expressions, and loops (covered in LOOPS.md).

## Conditional Statements: `when`

Xenith uses `when` instead of `if` for conditional execution.

### Basic `when`

```xenith
spawn age: int = 18

when age >= 18 {
    echo("You can vote!")
}
```

### `when` / `or when` Chains

Use `or when` for multiple conditions (like `else if`):

```xenith
spawn score: int = 85

when score >= 90 {
    echo("Grade: A")
} or when score >= 80 {
    echo("Grade: B")
} or when score >= 70 {
    echo("Grade: C")
} or when score >= 60 {
    echo("Grade: D")
} otherwise {
    echo("Grade: F")
}
# Output: Grade: B
```

### `otherwise` Clause

Use `otherwise` for the default case (like `else`):

```xenith
spawn temperature: int = 25

when temperature > 30 {
    echo("Hot outside!")
} or when temperature > 20 {
    echo("Warm outside")
} otherwise {
    echo("Cool outside")
}
# Output: Warm outside
```


### Complex Conditions

Use logical operators (`&&`, `||`, `!`) for complex conditions:

```xenith
spawn age: int = 25
spawn has_license: bool = true
spawn is_weekend: bool = false

when age >= 18 && has_license {
    echo("You can drive")
}

when is_weekend || (age >= 16 && !has_license) {
    echo("You can take the bus")
}
```

## Ternary Operator

The ternary operator provides a concise way to write simple conditionals:

### Syntax

```xenith
condition ? value_if_true : value_if_false
```

### Basic Examples

```xenith
spawn age: int = 18
spawn status: string = age >= 18 ? "Adult" : "Minor"
echo(status)  # Adult

spawn score: int = 75
spawn result: string = score >= 60 ? "Pass" : "Fail"
echo(result)  # Pass
```

### With Different Types

```xenith
# Numbers
spawn x: int = 10
spawn abs_x: int = x >= 0 ? x : -x
echo(abs_x)  # 10

spawn y: int = -5
spawn abs_y: int = y >= 0 ? y : -y
echo(abs_y)  # 5

# Boolean
spawn is_even: bool = x % 2 == 0 ? true : false
echo(is_even)  # true

# Method calls
spawn max: int = x > y ? x : y
echo(max)  # 10
```

### Nested Ternary

You can nest ternary operators for multiple conditions:

```xenith
spawn score: int = 85
spawn grade: string = score >= 90 ? "A" : 
                      (score >= 80 ? "B" : 
                       (score >= 70 ? "C" : 
                        (score >= 60 ? "D" : "F")))
echo(grade)  # B
```

### Ternary vs `when`

Use ternary for simple, single-line assignments:

```xenith
# Good for ternary - simple assignment
spawn can_vote: bool = age >= 18 ? true : false

# Better with when - multiple statements
when age >= 18 {
    spawn status: string = "Adult"
    echo("Welcome!")
} otherwise {
    spawn status: string = "Minor"
    echo("Sorry, too young")
}
```

## Common Patterns

### Validation Pattern

```xenith
method validateAge(age: int) -> string {
    when age < 0 {
        release "Invalid: Negative age"
    } or when age < 13 {
        release "Child"
    } or when age < 20 {
        release "Teenager"
    } or when age < 65 {
        release "Adult"
    } otherwise {
        release "Senior"
    }
}

echo(validateAge(25))   # Adult
echo(validateAge(70))   # Senior
echo(validateAge(-5))   # Invalid: Negative age
```

### Early Return Pattern

```xenith
method processNumber(n: int) -> string {
    when n < 0 {
        release "Negative numbers not allowed"
    }
    when n == 0 {
        release "Zero"
    }
    when n % 2 == 0 {
        release "Even positive"
    }
    release "Odd positive"
}

echo(processNumber(-5))  # Negative numbers not allowed
echo(processNumber(0))   # Zero
echo(processNumber(4))   # Even positive
echo(processNumber(7))   # Odd positive
```

### Range Checking

```xenith
method isBetween(value: int, min: int, max: int) -> bool {
    release value >= min && value <= max
}

spawn temperature: int = 72

when isBetween(temperature, 60, 80) {
    echo("Comfortable temperature")
} or when isBetween(temperature, 40, 59) {
    echo("A bit chilly")
} or when isBetween(temperature, 81, 100) {
    echo("Getting warm")
} otherwise {
    echo("Extreme temperature!")
}
```

### Multiple Conditions with Ternary

```xenith
spawn a: int = 5
spawn b: int = 10
spawn c: int = 7

# Find maximum of three numbers
spawn max: int = a > b ? (a > c ? a : c) : (b > c ? b : c)
echo(max)  # 10

# Determine sign
spawn x: int = -5
spawn sign: string = x > 0 ? "Positive" : (x < 0 ? "Negative" : "Zero")
echo(sign)  # Negative
```

## Truthy and Falsy Values

In conditionals, values are evaluated as truthy or falsy:

| Value | Truthy? |
|-------|---------|
| `true` | ✅ Truthy |
| `false` | ❌ Falsy |
| Non-zero numbers | ✅ Truthy |
| `0` | ❌ Falsy |
| Non-empty string | ✅ Truthy |
| `""` (empty) | ❌ Falsy |
| Non-empty list | ✅ Truthy |
| `[]` (empty) | ❌ Falsy |
| Non-empty map | ✅ Truthy |
| `{}` (empty) | ❌ Falsy |
| Non-null values | ✅ Truthy |
| `null` | ❌ Falsy |

```xenith
spawn name: string = "Alice"
when name {
    echo("Name exists: {name}")  # Executes
}

spawn empty: string = ""
when empty {
    echo("This won't print")
} otherwise {
    echo("String is empty")
}

spawn count: int = 5
when count {
    echo("Count is non-zero: {count}")  # Executes
}

spawn items: list<int> = []
when items {
    echo("List has elements")  # Won't execute
} otherwise {
    echo("List is empty")
}
```

## Complete Examples

### Calculator with Multiple Operations

```xenith
method calculate(a: int, b: int, op: string) -> int {
    when op == "add" {
        release a + b
    } or when op == "subtract" {
        release a - b
    } or when op == "multiply" {
        release a * b
    } or when op == "divide" {
        release a / b
    } otherwise {
        release 0
    }
}

spawn x: int = 10
spawn y: int = 5

echo(calculate(x, y, "add"))       # 15
echo(calculate(x, y, "multiply"))  # 50
```

### Day of Week Message

```xenith
method getDayMessage(day: string) -> string {
    when day == "Monday" {
        release "Start of work week"
    } or when day == "Friday" {
        release "TGIF!"
    } or when day == "Saturday" || day == "Sunday" {
        release "Weekend!"
    } otherwise {
        release "Regular work day"
    }
}

echo(getDayMessage("Monday"))    # Start of work week
echo(getDayMessage("Saturday"))  # Weekend!
echo(getDayMessage("Tuesday"))   # Regular work day
```

### Discount Calculator

```xenith
method calculatePrice(original: float, is_member: bool, quantity: int) -> float {
    spawn discount: float = 0.0
    
    when is_member {
        discount = 0.10  # 10% for members
    }
    
    when quantity >= 10 {
        discount = discount + 0.05  # Extra 5% for bulk
    }
    
    spawn final_price: float = original * (1.0 - discount)
    release final_price
}

spawn price: float = calculatePrice(100.0, true, 5)
echo(price)   # 90.0 (10% off)

spawn bulk_price: float = calculatePrice(100.0, true, 10)
echo(bulk_price)  # 85.0 (15% off total)
```

## Best Practices

1. **Use `otherwise` for default cases** - Always handle unexpected values
2. **Keep conditions simple** - Extract complex logic into methods
3. **Use ternary for simple assignments** - Avoid nesting beyond 2 levels
4. **Early returns for validation** - Check invalid cases first
5. **Be explicit with comparisons** - Don't rely on truthy/falsy for non-booleans when clarity matters

```xenith
# Good - explicit
when name != "" && name != null {
    echo("Hello, {name}")
}

# Acceptable but less clear
when name {
    echo("Hello, {name}")
}
```

## Next Steps

- Read [LOOPS.md](LOOPS.md) for iteration
- Explore [METHODS.md](METHODS.md) for reusable code
- Learn about [PATTERN_MATCHING.md](PATTERN_MATCHING.md) for advanced conditional logic
```
