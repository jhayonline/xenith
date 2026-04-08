# Xenith Pattern Matching

## Introduction

Pattern matching in Xenith allows you to compare a value against multiple patterns and execute the first matching block. It's a powerful alternative to long chains of `when` statements, especially when comparing against literal values.

## Basic Match Syntax

```xenith
match value {
    pattern1 => {
        # Code for pattern1
    }
    pattern2 => {
        # Code for pattern2
    }
    _ => {
        # Default case (wildcard)
    }
}
```

## Literal Patterns

Match against specific literal values:

```xenith
spawn grade: string = "B"

match grade {
    "A" => {
        echo("Excellent!")
    }
    "B" => {
        echo("Good job!")
    }
    "C" => {
        echo("Satisfactory")
    }
    "D" => {
        echo("Needs improvement")
    }
    "F" => {
        echo("Failing")
    }
}
# Output: Good job!
```

## Wildcard Pattern (`_`)

Use underscore `_` as a catch-all pattern for any value:

```xenith
spawn status_code: int = 404

match status_code {
    200 => {
        echo("OK - Request successful")
    }
    201 => {
        echo("Created - Resource created")
    }
    400 => {
        echo("Bad Request - Invalid syntax")
    }
    401 => {
        echo("Unauthorized - Authentication required")
    }
    403 => {
        echo("Forbidden - Insufficient permissions")
    }
    404 => {
        echo("Not Found - Resource doesn't exist")
    }
    _ => {
        echo("Unknown status code: {status_code}")
    }
}
# Output: Not Found - Resource doesn't exist
```

## Matching Numbers

```xenith
spawn score: int = 85

match score {
    100 => {
        echo("Perfect score!")
    }
    90 => {
        echo("Excellent!")
    }
    80 => {
        echo("Very good!")
    }
    70 => {
        echo("Good")
    }
    _ => {
        echo("Score: {score}")
    }
}
# Output: Very good!
```

## Matching Strings

```xenith
spawn command: string = "quit"

match command {
    "help" => {
        echo("Available commands: help, status, quit")
    }
    "status" => {
        echo("System is running normally")
    }
    "quit" => {
        echo("Goodbye!")
        # Exit program
    }
    "restart" => {
        echo("Restarting...")
    }
    _ => {
        echo("Unknown command: {command}")
    }
}
# Output: Goodbye!
```

## Matching Booleans

```xenith
spawn is_ready: bool = false

match is_ready {
    true => {
        echo("System is ready")
    }
    false => {
        echo("System is not ready")
    }
}
# Output: System is not ready
```

## Matching Lists (Simple)

```xenith
spawn response: list<int> = [200, "OK"]

match response {
    [200, "OK"] => {
        echo("Success response")
    }
    [404, "Not Found"] => {
        echo("Resource not found")
    }
    [500, "Error"] => {
        echo("Server error")
    }
    _ => {
        echo("Unknown response: {ret(response)}")
    }
}
```

## Practical Examples

### Day of Week Handler

```xenith
method getDayType(day: string) -> string {
    match day {
        "Monday" => {
            release "Start of work week"
        }
        "Tuesday" => {
            release "Work day"
        }
        "Wednesday" => {
            release "Midweek"
        }
        "Thursday" => {
            release "Almost Friday"
        }
        "Friday" => {
            release "TGIF!"
        }
        "Saturday" => {
            release "Weekend!"
        }
        "Sunday" => {
            release "Weekend!"
        }
        _ => {
            release "Invalid day"
        }
    }
}

echo(getDayType("Monday"))    # Start of work week
echo(getDayType("Friday"))    # TGIF!
echo(getDayType("Saturday"))  # Weekend!
echo(getDayType("Invalid"))   # Invalid day
```

### HTTP Status Handler

```xenith
method handleHTTPStatus(code: int) -> string {
    match code {
        200 => {
            release "OK"
        }
        201 => {
            release "Created"
        }
        204 => {
            release "No Content"
        }
        301 => {
            release "Moved Permanently"
        }
        302 => {
            release "Found"
        }
        400 => {
            release "Bad Request"
        }
        401 => {
            release "Unauthorized"
        }
        403 => {
            release "Forbidden"
        }
        404 => {
            release "Not Found"
        }
        500 => {
            release "Internal Server Error"
        }
        502 => {
            release "Bad Gateway"
        }
        503 => {
            release "Service Unavailable"
        }
        _ => {
            release "Unknown Status Code: {code}"
        }
    }
}

spawn codes: list<int> = [200, 404, 500, 418]
for code in codes {
    echo("{code}: {handleHTTPStatus(code)}")
}
# Output:
# 200: OK
# 404: Not Found
# 500: Internal Server Error
# 418: Unknown Status Code: 418
```

### Calculator Operation

```xenith
method calculate(a: int, b: int, op: string) -> int {
    match op {
        "add" => {
            release a + b
        }
        "subtract" => {
            release a - b
        }
        "multiply" => {
            release a * b
        }
        "divide" => {
            when b == 0 {
                panic "Division by zero"
            }
            release a / b
        }
        "power" => {
            release a ^ b
        }
        _ => {
            panic "Unknown operation: {op}"
        }
    }
}

try {
    echo(calculate(10, 5, "add"))       # 15
    echo(calculate(10, 5, "multiply")) # 50
    echo(calculate(10, 5, "divide"))   # 2
    echo(calculate(10, 2, "power"))    # 100
    echo(calculate(10, 5, "modulo"))   # Panics
} catch err {
    echo("Error: {err}")
}
```

### Command Parser

```xenith
method parseCommand(input: string) -> null {
    spawn parts: list<string> = input.split(" ")
    
    when parts.len() == 0 {
        release null
    }
    
    spawn command: string = parts[0]
    
    match command {
        "echo" => {
            when parts.len() > 1 {
                echo(parts[1])
            } otherwise {
                echo("")
            }
        }
        "add" => {
            when parts.len() == 3 {
                spawn a: int = parts[1] as int
                spawn b: int = parts[2] as int
                echo(a + b)
            } otherwise {
                echo("Usage: add <num1> <num2>")
            }
        }
        "quit" => {
            echo("Goodbye!")
            stop  # Exit loop if in one
        }
        "help" => {
            echo("Commands: echo, add, quit, help")
        }
        _ => {
            echo("Unknown command: {command}. Type 'help' for available commands")
        }
    }
    release null
}

# Simulated REPL
spawn should_continue: bool = true
while should_continue {
    spawn input_line: string = input()
    parseCommand(input_line)
}
```

### Menu System

```xenith
method showMenu(choice: int) -> null {
    echo("\n=== Menu ===")
    echo("1. Start Game")
    echo("2. Load Game")
    echo("3. Settings")
    echo("4. Credits")
    echo("5. Exit")
    
    match choice {
        1 => {
            echo("Starting new game...")
            # Initialize game
        }
        2 => {
            echo("Load game...")
            # Load saved game
        }
        3 => {
            echo("Settings...")
            # Show settings
        }
        4 => {
            echo("Credits: Xenith Game v1.0")
            echo("Created with Xenith Language")
        }
        5 => {
            echo("Exiting... Goodbye!")
            panic "Exit requested"  # Will be caught by outer handler
        }
        _ => {
            echo("Invalid choice: {choice}. Please select 1-5")
        }
    }
    release null
}

try {
    spawn running: bool = true
    while running {
        spawn selection: int = input_int()
        showMenu(selection)
    }
} catch err {
    when err == "Exit requested" {
        echo("Game exited normally")
    } otherwise {
        echo("Unexpected error: {err}")
    }
}
```

### Configuration Parser

```xenith
method parseConfigValue(key: string, value: string) -> null {
    match key {
        "debug" => {
            match value {
                "true" => {
                    echo("Debug mode enabled")
                }
                "false" => {
                    echo("Debug mode disabled")
                }
                _ => {
                    echo("Invalid debug value: {value}. Use true/false")
                }
            }
        }
        "port" => {
            try {
                spawn port_num: int = value as int
                when port_num < 1024 {
                    echo("Warning: Using privileged port {port_num}")
                } or when port_num > 65535 {
                    echo("Error: Port {port_num} out of range")
                } otherwise {
                    echo("Port set to {port_num}")
                }
            } catch err {
                echo("Invalid port number: {value}")
            }
        }
        "host" => {
            when value == "" {
                echo("Warning: Empty hostname")
            } otherwise {
                echo("Host set to {value}")
            }
        }
        _ => {
            echo("Unknown configuration key: {key}")
        }
    }
    release null
}

spawn config: map<string, string> = {
    "debug": "true",
    "port": "8080",
    "host": "localhost",
    "unknown": "value"
}

for key, value in config.items() {
    parseConfigValue(key, value)
}
```

## Match vs When

When to use `match` vs `when`:

| Use `match` when... | Use `when` when... |
|-------------------|-------------------|
| Comparing against many literal values | Complex boolean conditions |
| Values are discrete (enums, status codes) | Range checks (`x > 5 && x < 10`) |
| You have a default/wildcard case | You need `or when` chains |
| Pattern is the primary decision factor | Conditions involve multiple variables |

```xenith
# Good for match - discrete values
match status {
    "active" => { ... }
    "inactive" => { ... }
    "pending" => { ... }
    _ => { ... }
}

# Good for when - ranges and complex conditions
when age < 0 {
    echo("Invalid")
} or when age < 13 {
    echo("Child")
} or when age < 20 {
    echo("Teen")
} or when age < 65 {
    echo("Adult")
} otherwise {
    echo("Senior")
}
```

## Best Practices

1. **Always include a wildcard `_` case** - Handle unexpected values
2. **Order patterns from most specific to least specific** - First match wins
3. **Keep match blocks focused** - Each arm should be relatively simple
4. **Use match for 3+ literal comparisons** - More readable than multiple `when` statements
5. **Combine with error handling** - Use wildcard to catch unexpected cases

```xenith
# Good - specific to general order
match value {
    "exact" => { ... }      # Most specific
    "prefix_" => { ... }    # Less specific
    _ => { ... }            # Wildcard
}

# Avoid - unreachable patterns
match value {
    _ => { ... }            # This matches everything!
    "specific" => { ... }   # Never executes
}
```

## Common Pitfalls

### Missing Wildcard

```xenith
# This might fail if an unexpected value appears
match color {
    "red" => { echo("Red") }
    "blue" => { echo("Blue") }
    # No _ case - what happens for "green"?
}

# Better - always include wildcard
match color {
    "red" => { echo("Red") }
    "blue" => { echo("Blue") }
    _ => { echo("Unknown color: {color}") }
}
```

### Order Matters

```xenith
# Wrong order - "any" will catch everything
match value {
    _ => { echo("Any value") }      # Always matches first
    "specific" => { echo("Specific") }  # Never reached
}

# Correct order - specific first
match value {
    "specific" => { echo("Specific") }
    _ => { echo("Any value") }
}
```

## Next Steps

- Learn about [ERROR_HANDLING.md](ERROR_HANDLING.md) for robust error management
- Read [CONTROL_FLOW.md](CONTROL_FLOW.md) for conditionals
- Explore [STRUCTS.md](STRUCTS.md) for custom data types
```
