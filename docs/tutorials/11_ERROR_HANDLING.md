# Xenith Error Handling

## Introduction

Xenith provides robust error handling through `try-catch` blocks and `panic` statements. Errors can be caught and handled gracefully without crashing your program.

## The `panic` Statement

Use `panic` to trigger an error condition:

```xenith
panic "Something went wrong!"
```

When a panic occurs, execution stops and the error propagates up to the nearest `catch` block.

### Basic Panic Example

```xenith
method divide(a: int, b: int) -> int {
    when b == 0 {
        panic "Division by zero!"
    }
    release a / b
}

spawn result: int = divide(10, 2)
echo(result)  # 5

# This will panic
spawn bad: int = divide(10, 0)  # Triggers panic
```

## Try-Catch Blocks

Use `try-catch` to handle errors gracefully:

### Syntax

```xenith
try {
    # Code that might panic
} catch error_var {
    # Code that runs if an error occurs
}
```

### Basic Example

```xenith
method divide(a: int, b: int) -> int {
    when b == 0 {
        panic "Cannot divide by zero!"
    }
    release a / b
}

try {
    spawn result: int = divide(10, 0)
    echo("Result: {result}")  # This won't execute
} catch err {
    echo("Error caught: {err}")
}
# Output: Error caught: Cannot divide by zero!
```

### Successful Try Block

```xenith
try {
    spawn result: int = divide(10, 2)
    echo("Result: {result}")  # This executes
} catch err {
    echo("This won't execute")
}
# Output: Result: 5
```

## The Error Variable

The catch block receives an error variable containing the error message:

```xenith
try {
    panic "Database connection failed"
} catch error_msg {
    echo("Error: {error_msg}")
}
# Output: Error: Database connection failed
```

### Using Error Variable for Logic

```xenith
method processFile(filename: string) -> null {
    when filename == "" {
        panic "Empty filename provided"
    }
    when filename == "invalid.txt" {
        panic "File not found: {filename}"
    }
    echo("Processing {filename}")
    release null
}

try {
    processFile("")
} catch err {
    when err == "Empty filename provided" {
        echo("Please provide a filename")
    } otherwise {
        echo("Unexpected error: {err}")
    }
}

try {
    processFile("invalid.txt")
} catch err {
    echo("File error: {err}")
}
```

## Catching Runtime Errors

Try-catch also catches runtime errors like division by zero:

```xenith
try {
    spawn x: int = 10 / 0
    echo("This won't print")
} catch err {
    echo("Caught runtime error: {err}")
}
# Output: Caught runtime error: Division by zero
```

## Nested Try-Catch

You can nest try-catch blocks for fine-grained error handling:

```xenith
try {
    echo("Outer try")
    
    try {
        echo("  Inner try")
        panic "Inner error"
        echo("  This won't print")
    } catch inner_err {
        echo("  Inner catch: {inner_err}")
    }
    
    echo("Outer continues after inner catch")
} catch outer_err {
    echo("Outer catch (won't execute): {outer_err}")
}
# Output:
# Outer try
#   Inner try
#   Inner catch: Inner error
# Outer continues after inner catch
```

## Practical Examples

### User Input Validation

```xenith
method getPositiveNumber() -> int {
    spawn input_str: string = input()
    spawn num: int = input_str as int
    
    when num <= 0 {
        panic "Number must be positive!"
    }
    release num
}

try {
    echo("Enter a positive number:")
    spawn value: int = getPositiveNumber()
    echo("You entered: {value}")
} catch err {
    echo("Invalid input: {err}")
}
```

### File Operations (Simulated)

```xenith
method readConfig(filename: string) -> string {
    when filename == "" {
        panic "No filename provided"
    }
    when filename != "config.xen" {
        panic "File '{filename}' not found"
    }
    release "name=Xenith\nversion=1.0"
}

method parseConfig(content: string) -> map<string, string> {
    spawn config: map<string, string> = {}
    spawn lines: list<string> = content.split("\n")
    
    for line in lines {
        when line == "" {
            skip
        }
        spawn parts: list<string> = line.split("=")
        when parts.len() != 2 {
            panic "Invalid config line: {line}"
        }
        config[parts[0]] = parts[1]
    }
    release config
}

# Use with error handling
try {
    spawn content: string = readConfig("config.xen")
    spawn settings: map<string, string> = parseConfig(content)
    echo("Config loaded successfully")
    for key, value in settings.items() {
        echo("{key} = {value}")
    }
} catch err {
    echo("Failed to load config: {err}")
}
```

### Network Request Simulation

```xenith
method fetchData(url: string) -> string {
    when url == "" {
        panic "URL cannot be empty"
    }
    when url == "bad://example.com" {
        panic "Invalid protocol"
    }
    when url == "http://error.com" {
        panic "Server returned 500 error"
    }
    release '{"status": "ok", "data": "sample"}'
}

method processResponse(data: string) -> null {
    when data == "" {
        panic "Empty response"
    }
    echo("Processing: {data}")
    release null
}

spawn urls: list<string> = [
    "http://api.example.com",
    "bad://example.com",
    "http://error.com",
    ""
]

for url in urls {
    try {
        echo("\nFetching {url}")
        spawn response: string = fetchData(url)
        processResponse(response)
    } catch err {
        echo("Error for {url}: {err}")
    }
}
```

### Banking Transaction Example

```xenith
struct BankAccount {
    owner: string,
    balance: float
}

impl BankAccount {
    method withdraw(self: Self, amount: float) -> null {
        when amount <= 0 {
            panic "Withdrawal amount must be positive"
        }
        when amount > self.balance {
            panic "Insufficient funds: balance ${self.balance}, tried to withdraw ${amount}"
        }
        self.balance = self.balance - amount
        echo("Withdrew ${amount}. New balance: ${self.balance}")
        release null
    }
    
    method deposit(self: Self, amount: float) -> null {
        when amount <= 0 {
            panic "Deposit amount must be positive"
        }
        self.balance = self.balance + amount
        echo("Deposited ${amount}. New balance: ${self.balance}")
        release null
    }
    
    method transfer(self: Self, target: BankAccount, amount: float) -> null {
        try {
            self.withdraw(amount)
            target.deposit(amount)
            echo("Transfer of ${amount} from {self.owner} to {target.owner} successful")
        } catch err {
            panic "Transfer failed: {err}"
        }
        release null
    }
}

spawn alice: BankAccount = BankAccount { owner: "Alice", balance: 100.0 }
spawn bob: BankAccount = BankAccount { owner: "Bob", balance: 50.0 }

# Successful transfer
try {
    BankAccount::transfer(alice, bob, 30.0)
} catch err {
    echo("Transfer error: {err}")
}

# Failed transfer (insufficient funds)
try {
    BankAccount::transfer(alice, bob, 100.0)
} catch err {
    echo("Transfer error: {err}")
}

# Failed transfer (invalid amount)
try {
    BankAccount::transfer(alice, bob, -10.0)
} catch err {
    echo("Transfer error: {err}")
}
```

## Error Propagation

Panics automatically propagate up through nested method calls:

```xenith
method level3() -> null {
    panic "Error at level 3"
    release null
}

method level2() -> null {
    echo("Level 2 start")
    level3()
    echo("Level 2 end")  # Won't execute
    release null
}

method level1() -> null {
    echo("Level 1 start")
    level2()
    echo("Level 1 end")  # Won't execute
    release null
}

try {
    level1()
} catch err {
    echo("Caught at top level: {err}")
}
# Output:
# Level 1 start
# Level 2 start
# Caught at top level: Error at level 3
```

## Multiple Error Types

Handle different error conditions with conditional logic:

```xenith
method processValue(value: int) -> null {
    when value < 0 {
        panic "Negative value not allowed"
    }
    when value == 0 {
        panic "Zero value is invalid"
    }
    when value > 100 {
        panic "Value exceeds maximum (100)"
    }
    echo("Processing {value}")
    release null
}

spawn test_values: list<int> = [-5, 0, 50, 150]

for val in test_values {
    try {
        processValue(val)
    } catch err {
        when err == "Negative value not allowed" {
            echo("Error: Please provide a positive number")
        } or when err == "Zero value is invalid" {
            echo("Error: Zero is not allowed")
        } or when err == "Value exceeds maximum (100)" {
            echo("Error: Value too large")
        } otherwise {
            echo("Unknown error: {err}")
        }
    }
}
```

## Uncaught Panics

If a panic is not caught, it will crash the program:

```xenith
# This will crash the program
panic "Uncaught error!"
echo("This never executes")
```

## Best Practices

1. **Use specific error messages** - Include context to help debugging
2. **Catch errors at appropriate level** - Not too high, not too low
3. **Don't ignore errors** - Always handle or propagate them
4. **Validate early** - Check inputs at the start of methods
5. **Use descriptive error messages** - Help users understand what went wrong

```xenith
# Good - descriptive error
method setAge(age: int) -> null {
    when age < 0 {
        panic "Age cannot be negative. Got: {age}"
    }
    when age > 150 {
        panic "Age exceeds maximum reasonable value. Got: {age}"
    }
    release null
}

# Bad - vague error
method setAge(age: int) -> null {
    when age < 0 {
        panic "Error"  # Not helpful!
    }
    release null
}
```

## Common Patterns

### Retry Pattern

```xenith
method retryOperation(max_attempts: int) -> null {
    spawn attempts: int = 0
    
    while attempts < max_attempts {
        try {
            # Operation that might fail
            spawn random: int = (MATH_PI * 1000) as int % 10
            when random < 7 {
                panic "Random failure"
            }
            echo("Operation succeeded on attempt {attempts + 1}")
            release null
        } catch err {
            attempts = attempts + 1
            echo("Attempt {attempts} failed: {err}")
            when attempts == max_attempts {
                panic "All {max_attempts} attempts failed"
            }
        }
    }
    release null
}

try {
    retryOperation(3)
} catch err {
    echo("Final error: {err}")
}
```

### Cleanup Pattern

```xenith
method withResource(resource_name: string) -> null {
    echo("Opening {resource_name}")
    
    try {
        # Use resource
        when resource_name == "bad" {
            panic "Resource error"
        }
        echo("Using {resource_name}")
    } catch err {
        echo("Error while using resource: {err}")
        # Re-throw after cleanup
        panic "Operation failed"
    } finally {  # Note: finally not yet implemented, this is conceptual
        echo("Closing {resource_name}")
    }
    release null
}
```

## Next Steps

- Learn about [PATTERN_MATCHING.md](PATTERN_MATCHING.md) for advanced conditional logic
- Read [METHODS.md](METHODS.md) for more on method design
- Explore [COLLECTIONS.md](COLLECTIONS.md) for data structures
```
