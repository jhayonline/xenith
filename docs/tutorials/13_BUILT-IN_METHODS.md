# Xenith Built-in Methods

## Introduction

Xenith provides a set of built-in methods for common operations like input/output, type checking, collection manipulation, and executing external files.

## Input/Output Methods

### `echo()` - Print Output

Prints values to the console. Accepts any type and automatically converts to string representation.

```xenith
# Print strings
echo("Hello, World!")

# Print numbers
echo(42)
echo(3.14159)

# Print variables
spawn name: string = "Alice"
echo(name)

# Print expressions
echo(5 + 3)
echo("Value: {name}")

# Print multiple items (concatenated)
echo("The answer is " + (42 as string))
```

### `ret()` - String Representation

Returns the string representation of a value (useful for debugging).

```xenith
spawn number: int = 42
spawn text: string = ret(number)
echo(text)  # "42"

spawn list: list<int> = [1, 2, 3]
spawn list_str: string = ret(list)
echo(list_str)  # "[1, 2, 3]"

spawn map: map<string, int> = {"a": 1, "b": 2}
echo(ret(map))  # "{\"a\": 1, \"b\": 2}"
```

### `input()` - Read String Input

Reads a line of text from the user and returns it as a string.

```xenith
spawn name: string = input()
echo("Hello, {name}!")

# With prompt (print first)
echo("Enter your age: ")
spawn age_str: string = input()
```

### `input_int()` - Read Integer Input

Reads a line of text and converts it to an integer. Re-prompts if input is not a valid integer.

```xenith
spawn age: int = input_int()
echo("You are {age} years old")

# Will keep asking until valid integer is entered
spawn quantity: int = input_int()
```

### `clear()` - Clear Console

Clears the terminal screen.

```xenith
echo("This text will be cleared...")
clear()
echo("Screen is now clear!")
```

## Type Checking Methods

### `is_num()` - Check if Number

Returns `true` if the value is a number (int or float).

```xenith
spawn x: int = 42
spawn y: string = "hello"
spawn z: float = 3.14

echo(is_num(x))  # true
echo(is_num(y))  # false
echo(is_num(z))  # true
```

### `is_str()` - Check if String

Returns `true` if the value is a string.

```xenith
spawn name: string = "Alice"
spawn age: int = 25

echo(is_str(name))  # true
echo(is_str(age))   # false
```

### `is_list()` - Check if List

Returns `true` if the value is a list.

```xenith
spawn numbers: list<int> = [1, 2, 3]
spawn text: string = "hello"

echo(is_list(numbers))  # true
echo(is_list(text))     # false
```

### `is_fun()` - Check if Method

Returns `true` if the value is a method (user-defined or built-in).

```xenith
method greet(name: string) -> string {
    release "Hello, {name}"
}

spawn func_var: method(string) -> string = greet

echo(is_fun(greet))      # true
echo(is_fun(echo))       # true (built-in)
echo(is_fun(42))         # false
```

## List Operations

### `append()` - Add Element to List

Adds an element to the end of a list. Returns the modified list.

```xenith
spawn fruits: list<string> = ["apple", "banana"]
fruits = append(fruits, "orange")
echo(ret(fruits))  # ["apple", "banana", "orange"]

# Method call style (alternative)
fruits.append("grape")
echo(ret(fruits))  # ["apple", "banana", "orange", "grape"]
```

### `pop()` - Remove and Return Element

Removes and returns an element from a list. With index parameter, removes at that position. Without index, removes the last element.

```xenith
spawn numbers: list<int> = [10, 20, 30, 40, 50]

# Pop last element
spawn last: int = pop(numbers)
echo(last)            # 50
echo(ret(numbers))    # [10, 20, 30, 40]

# Pop at specific index
spawn second: int = pop(numbers, 1)
echo(second)          # 20
echo(ret(numbers))    # [10, 30, 40]

# Method call style
spawn first: int = numbers.pop(0)
echo(first)           # 10
```

### `extend()` - Extend List with Another List

Appends all elements from one list to another.

```xenith
spawn list1: list<int> = [1, 2, 3]
spawn list2: list<int> = [4, 5, 6]

list1 = extend(list1, list2)
echo(ret(list1))  # [1, 2, 3, 4, 5, 6]

# Method call style
spawn combined: list<int> = [1, 2, 3]
combined.extend([4, 5, 6])
echo(ret(combined))  # [1, 2, 3, 4, 5, 6]
```

## Length Method

### `len()` - Get Length

Returns the length of a list, string, or map.

```xenith
# List length
spawn fruits: list<string> = ["apple", "banana", "orange"]
echo(len(fruits))  # 3

# String length
spawn text: string = "Hello"
echo(len(text))    # 5

# Map length
spawn scores: map<string, int> = {"Alice": 95, "Bob": 87}
echo(len(scores))  # 2

# Method call style
echo(fruits.len())  # 3
echo(text.len())    # 5
```

## File Execution

### `run()` - Execute External Xenith File

Executes another Xenith file. Returns `null` on success.

```xenith
# Execute another script
run("other_script.xen")

# Execute with path
run("scripts/helpers.xen")

# Error handling
try {
    run("nonexistent.xen")
} catch err {
    echo("Failed to run script: {err}")
}
```

## Complete Examples

### Interactive Number Guessing Game

```xenith
# number_game.xen
method guessGame() -> null {
    clear()
    echo("=== Number Guessing Game ===")
    echo("I'm thinking of a number between 1 and 100")
    
    spawn secret: int = (MATH_PI * 1000) as int % 100 + 1
    spawn attempts: int = 0
    spawn guessed: bool = false
    
    while !guessed {
        echo("\nEnter your guess: ")
        spawn guess: int = input_int()
        attempts = attempts + 1
        
        when guess == secret {
            echo("Correct! You guessed it in {attempts} attempts!")
            guessed = true
        } or when guess < secret {
            echo("Too low! Try again.")
        } otherwise {
            echo("Too high! Try again.")
        }
    }
    
    echo("\nThanks for playing!")
    release null
}

# Run the game
guessGame()
```

### Data Processing Pipeline

```xenith
# data_processor.xen
method processNumbers() -> null {
    clear()
    echo("=== Number Processor ===")
    
    spawn numbers: list<int> = []
    
    # Input numbers
    echo("Enter numbers (type 'done' to finish):")
    while true {
        echo("Number: ")
        spawn input_str: string = input()
        
        when input_str == "done" {
            stop
        }
        
        try {
            spawn num: int = input_str as int
            numbers = append(numbers, num)
        } catch err {
            echo("Invalid number! Try again.")
        }
    }
    
    # Process the list
    when len(numbers) == 0 {
        echo("No numbers entered!")
        release null
    }
    
    # Calculate statistics
    spawn sum: int = 0
    spawn max_val: int = numbers[0]
    spawn min_val: int = numbers[0]
    
    for n in numbers {
        sum = sum + n
        when n > max_val {
            max_val = n
        }
        when n < min_val {
            min_val = n
        }
    }
    
    spawn average: float = (sum as float) / (len(numbers) as float)
    
    # Display results
    echo("\n=== Results ===")
    echo("Numbers: {ret(numbers)}")
    echo("Count: {len(numbers)}")
    echo("Sum: {sum}")
    echo("Average: {average}")
    echo("Maximum: {max_val}")
    echo("Minimum: {min_val}")
    
    # Type checking demonstration
    echo("\n=== Type Checks ===")
    echo("Is numbers a list? {is_list(numbers)}")
    echo("Is sum a number? {is_num(sum)}")
    echo("Is average a number? {is_num(average)}")
    
    release null
}

processNumbers()
```

### To-Do List Application

```xenith
# todo.xen
spawn tasks: list<string> = []

method showMenu() -> null {
    clear()
    echo("=== To-Do List ===")
    echo("1. View tasks")
    echo("2. Add task")
    echo("3. Remove task")
    echo("4. Clear all tasks")
    echo("5. Exit")
    echo("\nChoice: ")
    release null
}

method viewTasks() -> null {
    clear()
    echo("=== Your Tasks ===")
    
    when len(tasks) == 0 {
        echo("No tasks yet!")
    } otherwise {
        for i = 0 to len(tasks) {
            echo("{i + 1}. {tasks[i]}")
        }
    }
    
    echo("\nPress Enter to continue...")
    input()
    release null
}

method addTask() -> null {
    clear()
    echo("Enter task description: ")
    spawn new_task: string = input()
    
    tasks = append(tasks, new_task)
    echo("Task added successfully!")
    
    echo("\nPress Enter to continue...")
    input()
    release null
}

method removeTask() -> null {
    clear()
    
    when len(tasks) == 0 {
        echo("No tasks to remove!")
        echo("\nPress Enter to continue...")
        input()
        release null
    }
    
    viewTasks()
    echo("\nEnter task number to remove: ")
    spawn task_num: int = input_int()
    
    when task_num >= 1 && task_num <= len(tasks) {
        spawn removed: string = pop(tasks, task_num - 1)
        echo("Removed: {removed}")
    } otherwise {
        echo("Invalid task number!")
    }
    
    echo("\nPress Enter to continue...")
    input()
    release null
}

method clearTasks() -> null {
    tasks = []
    echo("All tasks cleared!")
    echo("\nPress Enter to continue...")
    input()
    release null
}

# Main program loop
method runTodoApp() -> null {
    spawn running: bool = true
    
    while running {
        showMenu()
        spawn choice: int = input_int()
        
        match choice {
            1 => { viewTasks() }
            2 => { addTask() }
            3 => { removeTask() }
            4 => { clearTasks() }
            5 => {
                echo("Goodbye!")
                running = false
            }
            _ => {
                echo("Invalid choice! Press Enter to continue...")
                input()
            }
        }
    }
    release null
}

# Start the application
runTodoApp()
```

### Multi-File Project Example

```xenith
# math_utils.xen
export method isEven(n: int) -> bool {
    release n % 2 == 0
}

export method isOdd(n: int) -> bool {
    release n % 2 != 0
}

export method square(n: int) -> int {
    release n * n
}

export method cube(n: int) -> int {
    release n * n * n
}
```

```xenith
# string_utils.xen
export method capitalize(text: string) -> string {
    when len(text) == 0 {
        release ""
    }
    spawn first: string = text[0] as string
    # Would convert to uppercase in real implementation
    release first
}

export method reverse(text: string) -> string {
    spawn result: string = ""
    for i = len(text) - 1 to 0 step -1 {
        result = result + (text[i] as string)
    }
    release result
}

export method isPalindrome(text: string) -> bool {
    release reverse(text) == text
}
```

```xenith
# main.xen
grab { isEven, isOdd, square, cube } from "math_utils"
grab { capitalize, reverse, isPalindrome } from "string_utils"

clear()
echo("=== Utility Demo ===\n")

# Test math utilities
spawn num: int = 7
echo("Number: {num}")
echo("Is even? {isEven(num)}")
echo("Is odd? {isOdd(num)}")
echo("Square: {square(num)}")
echo("Cube: {cube(num)}")

# Test string utilities
spawn text: string = "racecar"
echo("\nText: {text}")
echo("Capitalized: {capitalize(text)}")
echo("Reversed: {reverse(text)}")
echo("Is palindrome? {isPalindrome(text)}")

# Type checking
echo("\n=== Type Checks ===")
echo("Is num a number? {is_num(num)}")
echo("Is text a string? {is_str(text)}")
echo("Is square a function? {is_fun(square)}")
```

## Best Practices

1. **Validate input** - Always check return values from `input_int()` for validity
2. **Handle errors** - Use try-catch with `run()` for missing files
3. **Clear screen appropriately** - Don't overuse `clear()` in production
4. **Use type checking defensively** - Verify types before operations when uncertain
5. **Prefer method call syntax** - `list.append(item)` is more idiomatic than `append(list, item)`

## Common Pitfalls

```xenith
# Don't forget to assign the result of append/pop/extend
spawn numbers: list<int> = [1, 2, 3]
append(numbers, 4)  # WRONG - numbers unchanged!
numbers = append(numbers, 4)  # CORRECT

# Check bounds before pop
spawn empty: list<int> = []
pop(empty)  # ERROR - index out of bounds!

# Use try-catch for safe pop
try {
    spawn value: int = pop(empty)
} catch err {
    echo("List is empty!")
}
```

## Next Steps

- Learn about [BUILT-IN_CONSTANTS.md](BUILT-IN_CONSTANTS.md) for predefined values
- Read [COLLECTIONS.md](COLLECTIONS.md) for more list/map operations
- Explore [MODULES.md](MODULES.md) for organizing code
```
