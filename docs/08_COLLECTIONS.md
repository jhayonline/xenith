# Xenith Collections

## Introduction

Xenith provides two powerful collection types: **Lists** (ordered sequences) and **Maps** (key-value dictionaries). Both are strongly typed and support various operations.

## Lists

Lists are ordered collections of elements, all of the same type.

### Creating Lists

```xenith
# Empty list
spawn empty: list<int> = []

# List of integers
spawn numbers: list<int> = [1, 2, 3, 4, 5]

# List of strings
spawn fruits: list<string> = ["apple", "banana", "orange"]

# List of floats
spawn prices: list<float> = [19.99, 29.99, 39.99]

# List of booleans
spawn flags: list<bool> = [true, false, true]

# List of lists (nested)
spawn matrix: list<list<int>> = [[1, 2], [3, 4], [5, 6]]
```

### Accessing Elements

Use square brackets `[]` with the index (zero-based):

```xenith
spawn fruits: list<string> = ["apple", "banana", "orange"]

echo(fruits[0])  # apple
echo(fruits[1])  # banana
echo(fruits[2])  # orange

# Modify elements
fruits[1] = "blueberry"
echo(fruits[1])  # blueberry
```

### List Methods

#### `append()` - Add element to end

```xenith
spawn numbers: list<int> = [1, 2, 3]
numbers.append(4)
numbers.append(5)

echo(ret(numbers))  # [1, 2, 3, 4, 5]
```

#### `pop()` - Remove and return element

```xenith
spawn fruits: list<string> = ["apple", "banana", "orange", "grape"]

# Pop last element
spawn last: string = fruits.pop()
echo(last)              # grape
echo(ret(fruits))       # [apple, banana, orange]

# Pop at specific index
spawn second: string = fruits.pop(1)
echo(second)            # banana
echo(ret(fruits))       # [apple, orange]
```

#### `len()` - Get list length

```xenith
spawn items: list<int> = [10, 20, 30, 40, 50]
spawn count: int = items.len()
echo(count)  # 5

# Common pattern for loops
for i = 0 to items.len() {
    echo(items[i])
}
```

### List Operations

#### Concatenation with `+`

```xenith
spawn list1: list<int> = [1, 2, 3]
spawn list2: list<int> = [4, 5, 6]
spawn combined: list<int> = list1 + list2

echo(ret(combined))  # [1, 2, 3, 4, 5, 6]

# Original lists unchanged
echo(ret(list1))  # [1, 2, 3]
```

#### Repetition with `*`

```xenith
spawn base: list<int> = [1, 2]
spawn repeated: list<int> = base * 3
echo(ret(repeated))  # [1, 2, 1, 2, 1, 2]
```

### Iterating Over Lists

```xenith
spawn fruits: list<string> = ["apple", "banana", "orange"]

# Value iteration
for fruit in fruits {
    echo(fruit)
}

# Index iteration
for i = 0 to fruits.len() {
    echo("{i}: {fruits[i]}")
}

# While loop iteration
spawn i: int = 0
while i < fruits.len() {
    echo(fruits[i])
    i = i + 1
}
```

## Maps

Maps are key-value pairs where keys are strings and values are of a specified type.

### Creating Maps

```xenith
# Empty map
spawn empty: map<string, int> = {}

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

# Nested map
spawn users: map<string, map<string, string>> = {
    "alice@email.com": {
        "name": "Alice",
        "city": "New York"
    }
}
```

### Accessing Values

Use square brackets `[]` with the key:

```xenith
spawn ages: map<string, int> = {
    "Alice": 25,
    "Bob": 30
}

echo(ages["Alice"])  # 25
echo(ages["Bob"])    # 30

# Modify values
ages["Alice"] = 26
echo(ages["Alice"])  # 26

# Add new key-value pair
ages["Charlie"] = 35
echo(ages["Charlie"])  # 35
```

### Map Methods

#### `keys()` - Get all keys

```xenith
spawn user: map<string, int> = {
    "Alice": 25,
    "Bob": 30,
    "Charlie": 35
}

spawn names: list<string> = user.keys()
echo(ret(names))  # ["Alice", "Bob", "Charlie"]

# Iterate over keys
for name in user.keys() {
    echo("Key: {name}")
}
```

#### `values()` - Get all values

```xenith
spawn ages: map<string, int> = {
    "Alice": 25,
    "Bob": 30,
    "Charlie": 35
}

spawn age_list: list<int> = ages.values()
echo(ret(age_list))  # [25, 30, 35]

# Iterate over values
for age in ages.values() {
    echo("Value: {age}")
}
```

#### `items()` - Get key-value pairs

Returns a list of lists, each containing `[key, value]`:

```xenith
spawn scores: map<string, int> = {
    "Alice": 95,
    "Bob": 87,
    "Charlie": 92
}

spawn pairs: list<list> = scores.items()
# Returns: [["Alice", 95], ["Bob", 87], ["Charlie", 92]]

# Iterate over key-value pairs
for pair in scores.items() {
    spawn name: string = pair[0] as string
    spawn score: int = pair[1] as int
    echo("{name}: {score}")
}

# Or with tuple unpacking
for name, score in scores.items() {
    echo("{name}: {score}")
}
```

#### `has_key()` - Check if key exists

```xenith
spawn user: map<string, int> = {
    "Alice": 25,
    "Bob": 30
}

echo(user.has_key("Alice"))   # true
echo(user.has_key("Charlie")) # false

# Common pattern for safe access
when user.has_key("Charlie") {
    echo(user["Charlie"])
} otherwise {
    echo("Charlie not found")
}
```

#### `len()` - Get number of key-value pairs

```xenith
spawn user: map<string, int> = {
    "Alice": 25,
    "Bob": 30,
    "Charlie": 35
}

spawn count: int = user.len()
echo(count)  # 3
```

### Map Operations

#### Removing Entries

```xenith
spawn ages: map<string, int> = {
    "Alice": 25,
    "Bob": 30,
    "Charlie": 35
}

# Set to null to remove (or use remove method if available)
ages["Bob"] = null
```

## Practical Examples

### List Processing

```xenith
# Filter even numbers
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

# Map (transform) list
method doubleAll(numbers: list<int>) -> list<int> {
    spawn result: list<int> = []
    for n in numbers {
        result.append(n * 2)
    }
    release result
}

spawn doubled: list<int> = doubleAll([1, 2, 3, 4, 5])
echo(ret(doubled))  # [2, 4, 6, 8, 10]

# Sum of list
method sumList(numbers: list<int>) -> int {
    spawn total: int = 0
    for n in numbers {
        total = total + n
    }
    release total
}

echo(sumList([10, 20, 30, 40, 50]))  # 150
```

### Map Processing

```xenith
# Student grade management
spawn grades: map<string, list<int>> = {
    "Alice": [85, 90, 88],
    "Bob": [78, 82, 79],
    "Charlie": [92, 95, 91]
}

# Calculate average for each student
method calculateAverage(scores: list<int>) -> float {
    spawn sum: int = 0
    for score in scores {
        sum = sum + score
    }
    release (sum as float) / (scores.len() as float)
}

echo("Student Averages:")
for name, scores in grades.items() {
    spawn avg: float = calculateAverage(scores)
    echo("{name}: {avg}")
}

# Find highest scoring student
method findTopStudent(grades: map<string, list<int>>) -> string {
    spawn top_name: string = ""
    spawn highest_avg: float = 0.0
    
    for name, scores in grades.items() {
        spawn avg: float = calculateAverage(scores)
        when avg > highest_avg {
            highest_avg = avg
            top_name = name
        }
    }
    release top_name
}

echo("Top student: {findTopStudent(grades)}")
```

### Shopping Cart Example

```xenith
# Shopping cart using list of maps
spawn cart: list<map<string, string>> = []

method addItem(name: string, price: string, quantity: string) -> null {
    spawn item: map<string, string> = {
        "name": name,
        "price": price,
        "quantity": quantity
    }
    cart.append(item)
    release null
}

method calculateTotal() -> float {
    spawn total: float = 0.0
    for item in cart {
        spawn price: float = item["price"] as float
        spawn qty: int = item["quantity"] as int
        total = total + (price * (qty as float))
    }
    release total
}

addItem("Apple", "0.50", "3")
addItem("Banana", "0.30", "5")
addItem("Orange", "0.75", "2")

echo("Cart contents:")
for item in cart {
    echo("{item["name"]}: {item["quantity"]} @ ${item["price"]}")
}
echo("Total: ${calculateTotal()}")
```

### Word Frequency Counter

```xenith
method countWords(text: string) -> map<string, int> {
    spawn frequencies: map<string, int> = {}
    
    # Simple split by space (in real implementation, would parse properly)
    spawn words: list<string> = text.split(" ")  # Hypothetical split method
    
    for word in words {
        when frequencies.has_key(word) {
            frequencies[word] = frequencies[word] + 1
        } otherwise {
            frequencies[word] = 1
        }
    }
    release frequencies
}

spawn sentence: string = "the cat and the dog and the bird"
spawn word_counts: map<string, int> = countWords(sentence)

for word, count in word_counts.items() {
    echo("{word}: {count}")
}
# Output:
# the: 3
# cat: 1
# and: 2
# dog: 1
# bird: 1
```

## Type Safety

Lists and maps enforce type safety at compile time:

```xenith
# Valid - all elements same type
spawn numbers: list<int> = [1, 2, 3, 4, 5]

# Invalid - mixed types (won't compile)
spawn mixed: list = [1, "two", 3]  # Error!

# Map value types must match
spawn ages: map<string, int> = {
    "Alice": 25,
    "Bob": "thirty"  # Error! String instead of int
}
```

## Common Patterns

### Check if List is Empty

```xenith
spawn items: list<int> = []

when items.len() == 0 {
    echo("List is empty")
}

# Or using truthiness
when items {
    echo("List has elements")
} otherwise {
    echo("List is empty")
}
```

### Safe Map Access

```xenith
method getValue(map: map<string, int>, key: string, default: int) -> int {
    when map.has_key(key) {
        release map[key]
    }
    release default
}

spawn scores: map<string, int> = {"Alice": 95}
echo(getValue(scores, "Alice", 0))    # 95
echo(getValue(scores, "Bob", 0))      # 0
```

### Copy List

```xenith
spawn original: list<int> = [1, 2, 3]
spawn copy: list<int> = original  # Creates a copy
copy.append(4)

echo(ret(original))  # [1, 2, 3] - unchanged
echo(ret(copy))      # [1, 2, 3, 4]
```

## Performance Tips

1. **Pre-allocate when possible** - Building lists incrementally is fine for most cases
2. **Use `len()` caching** - For loops, `len()` is called each iteration
3. **Avoid deep nesting** - Nested lists/maps can become hard to manage
4. **Use appropriate collection** - Lists for ordered data, maps for lookups by key

## Next Steps

- Learn about [STRUCTS.md](STRUCTS.md) for custom data types
- Read [LOOPS.md](LOOPS.md) for iteration patterns
- Explore [METHODS.md](METHODS.md) for reusable operations
```
