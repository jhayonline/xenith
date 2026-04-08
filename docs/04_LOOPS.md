# Xenith Loops

## Introduction

Xenith provides two main types of loops: `for` loops for iteration and `while` loops for conditional repetition. Both support `skip` (continue) and `stop` (break) for flow control.

## For Loops

### Range-based For Loop

Iterate over a range of numbers:

```xenith
# Basic range: from start to end (exclusive)
for i = 0 to 5 {
    echo(i)  # Prints 0, 1, 2, 3, 4
}

# With step
for i = 0 to 10 step 2 {
    echo(i)  # Prints 0, 2, 4, 6, 8
}

# Count down with negative step
for i = 5 to 0 step -1 {
    echo(i)  # Prints 5, 4, 3, 2, 1
}
```

### Collection-based For Loop

Iterate over list elements:

```xenith
spawn fruits: list<string> = ["apple", "banana", "orange"]

for fruit in fruits {
    echo("I like {fruit}")
}
# Output:
# I like apple
# I like banana
# I like orange
```

### Map Iteration

Iterate over map keys or key-value pairs:

```xenith
spawn scores: map<string, int> = {
    "Alice": 95,
    "Bob": 87,
    "Charlie": 92
}

# Iterate over keys
for name in scores.keys() {
    echo(name)
}
# Output: Alice, Bob, Charlie

# Iterate over values
for score in scores.values() {
    echo(score)
}
# Output: 95, 87, 92

# Iterate over key-value pairs
for name, score in scores.items() {
    echo("{name}: {score}")
}
# Output:
# Alice: 95
# Bob: 87
# Charlie: 92
```

### Tuple Unpacking in Loops

For lists of pairs, you can unpack directly:

```xenith
spawn pairs: list<list<int>> = [[1, 2], [3, 4], [5, 6]]

for a, b in pairs {
    echo("{a} + {b} = {a + b}")
}
# Output:
# 1 + 2 = 3
# 3 + 4 = 7
# 5 + 6 = 11
```

## While Loops

Execute a block while a condition is true:

```xenith
spawn counter: int = 0

while counter < 5 {
    echo(counter)
    counter = counter + 1
}
# Prints: 0, 1, 2, 3, 4
```

### While Loop with Complex Conditions

```xenith
spawn x: int = 10
spawn y: int = 0

while x > 0 && y < 5 {
    echo("x: {x}, y: {y}")
    x = x - 2
    y = y + 1
}
```

## Loop Control

### Skip - Continue to Next Iteration

Use `skip` to jump to the next iteration:

```xenith
# Print only even numbers
for i = 0 to 10 {
    when i % 2 == 1 {
        skip  # Skip odd numbers
    }
    echo(i)  # Prints: 0, 2, 4, 6, 8
}

# Skip specific values
spawn numbers: list<int> = [1, 2, 3, 4, 5, 6]

for n in numbers {
    when n == 3 {
        skip  # Skip 3
    }
    echo(n)  # Prints: 1, 2, 4, 5, 6
}
```

### Stop - Break Out of Loop

Use `stop` to exit the loop entirely:

```xenith
# Find first number greater than 5
spawn numbers: list<int> = [1, 3, 5, 7, 9, 11]

for n in numbers {
    when n > 5 {
        echo("First number > 5 is: {n}")
        stop  # Exit loop after finding
    }
}
# Output: First number > 5 is: 7

# Stop on condition
spawn i: int = 0
while i < 100 {
    when i * i > 50 {
        echo("Stopping at {i} because {i}^2 = {i * i} > 50")
        stop
    }
    i = i + 1
}
```

## Loop Return Values

For loops return a list of all expression values from each iteration:

```xenith
# Range loop returns list of values
spawn squares: list<int> = for i = 0 to 5 {
    release i * i
}
echo(ret(squares))  # [0, 1, 4, 9, 16]

# While loop also returns a list
spawn countdown: list<int> = while counter > 0 {
    release counter
    counter = counter - 1
}
```

To ignore the return value (return null instead):

```xenith
for i = 0 to 5 {
    echo(i)
}  # Returns list of nulls

# To explicitly return null (not yet implemented in syntax)
```

## Nested Loops

Loops can be nested inside other loops:

```xenith
# Multiplication table
for i = 1 to 4 {
    for j = 1 to 4 {
        echo("{i} * {j} = {i * j}")
    }
    echo("---")
}

# Find pairs that sum to 10
spawn numbers: list<int> = [1, 2, 3, 4, 5, 6, 7, 8, 9]

for a in numbers {
    for b in numbers {
        when a + b == 10 {
            echo("{a} + {b} = 10")
        }
    }
}
```

## Practical Examples

### Sum of Numbers

```xenith
method sumList(numbers: list<int>) -> int {
    spawn total: int = 0
    for n in numbers {
        total = total + n
    }
    release total
}

spawn values: list<int> = [10, 20, 30, 40, 50]
echo(sumList(values))  # 150
```

### Find Maximum Value

```xenith
method findMax(numbers: list<int>) -> int {
    when numbers.len() == 0 {
        release 0
    }
    
    spawn max_val: int = numbers[0]
    for n in numbers {
        when n > max_val {
            max_val = n
        }
    }
    release max_val
}

spawn scores: list<int> = [45, 78, 92, 63, 88]
echo(findMax(scores))  # 92
```

### Filter List

```xenith
method filterEven(numbers: list<int>) -> list<int> {
    spawn result: list<int> = []
    for n in numbers {
        when n % 2 == 0 {
            result.append(n)
        }
    }
    release result
}

spawn nums: list<int> = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
spawn evens: list<int> = filterEven(nums)
echo(ret(evens))  # [2, 4, 6, 8, 10]
```

### Prime Number Checker

```xenith
method isPrime(n: int) -> bool {
    when n <= 1 {
        release false
    }
    when n == 2 {
        release true
    }
    when n % 2 == 0 {
        release false
    }
    
    spawn i: int = 3
    while i * i <= n {
        when n % i == 0 {
            release false
        }
        i = i + 2
    }
    release true
}

# Find all primes up to 30
for i = 2 to 30 {
    when isPrime(i) {
        echo(i)
    }
}
# Output: 2, 3, 5, 7, 11, 13, 17, 19, 23, 29
```

## Performance Considerations

1. **Minimize work inside loops** - Move invariant calculations outside
2. **Use `skip` wisely** - Avoid deep nesting when possible
3. **List pre-allocation** - For large lists, consider building result gradually
4. **Break early** - Use `stop` when you've found what you need

```xenith
# Good - calculation outside loop
spawn limit: int = 1000
spawn threshold: int = limit * limit  # Calculate once

for i = 0 to limit {
    when i * i > threshold {
        stop
    }
}

# Bad - recalculating each iteration
for i = 0 to limit {
    when i * i > limit * limit {  # Recalculates limit * limit every time
        stop
    }
}
```

## Common Pitfalls

### Infinite Loops

```xenith
# DANGER: Infinite loop!
spawn x: int = 0
while x < 10 {
    echo(x)
    # Forgot to increment x!
}

# Fixed:
while x < 10 {
    echo(x)
    x = x + 1
}
```

### Off-by-One Errors

```xenith
# Prints 0 to 4 (5 numbers)
for i = 0 to 5 {
    echo(i)  # 0, 1, 2, 3, 4
}

# For inclusive range, adjust the end
for i = 0 to 6 {
    when i == 5 {
        echo(i)  # Includes 5
    }
}
```

## Summary

| Loop Type | Syntax | Use Case |
|-----------|--------|----------|
| Range for | `for i = start to end step step` | Counting, numeric ranges |
| Collection for | `for item in collection` | Iterating over lists/maps |
| While | `while condition` | Unknown number of iterations |
| Skip | `skip` | Skip current iteration |
| Stop | `stop` | Exit loop completely |

## Next Steps

- Learn about [CONTROL_FLOW.md](CONTROL_FLOW.md) for conditionals
- Read [COLLECTIONS.md](COLLECTIONS.md) for more on lists and maps
- Explore [METHODS.md](METHODS.md) for reusable code
```
