# Xenith Structs & Methods

## Introduction

Structs are custom data types that group related data together. In Xenith, structs can have methods attached to them via `impl` blocks, providing object-oriented programming capabilities with a unique syntax.

## Defining Structs

Use the `struct` keyword followed by the name and fields inside `{}`:

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

struct Student {
    name: string,
    grade: int,
    scores: list<int>
}
```

## Creating Struct Instances

Instantiate a struct using the struct name followed by `{ field: value, ... }`:

```xenith
spawn alice: Person = Person {
    name: "Alice",
    age: 25
}

spawn point: Point = Point {
    x: 10.5,
    y: 20.3
}

spawn rect: Rectangle = Rectangle {
    width: 100,
    height: 50
}

spawn student: Student = Student {
    name: "Bob",
    grade: 85,
    scores: [90, 85, 88]
}
```

## Field Access

Use dot notation `.` to access struct fields:

```xenith
spawn person: Person = Person {
    name: "Alice",
    age: 25
}

echo(person.name)  # Alice
echo(person.age)   # 25
```

## Field Mutation

Fields are mutable by default - you can modify them after creation:

```xenith
spawn person: Person = Person {
    name: "Alice",
    age: 25
}

person.age = 26
echo(person.age)  # 26

person.name = "Alicia"
echo(person.name)  # Alicia
```

## Implementation Blocks (`impl`)

Attach methods to structs using `impl` blocks:

```xenith
struct Person {
    name: string,
    age: int
}

impl Person {
    method greet(self: Self) -> string {
        release "Hello, my name is " + self.name
    }
    
    method isAdult(self: Self) -> bool {
        release self.age >= 18
    }
    
    method birthday(self: Self) -> null {
        self.age = self.age + 1
        release null
    }
    
    method updateName(self: Self, new_name: string) -> null {
        self.name = new_name
        release null
    }
}
```

## Instance Methods with `self`

Methods that operate on an instance must take `self: Self` as the first parameter:

```xenith
impl Person {
    # Read-only method
    method getName(self: Self) -> string {
        release self.name
    }
    
    # Method that modifies the struct
    method celebrateBirthday(self: Self) -> null {
        self.age = self.age + 1
        echo("Happy birthday {self.name}!")
        release null
    }
    
    # Method with additional parameters
    method haveBirthday(self: Self, years: int) -> null {
        self.age = self.age + years
        echo("{self.name} is now {self.age}")
        release null
    }
}
```

## Calling Methods

Xenith uses a unique syntax: `StructName::method(instance, arguments...)`

```xenith
spawn alice: Person = Person {
    name: "Alice",
    age: 25
}

# Call instance methods
echo(Person::greet(alice))           # Hello, my name is Alice
echo(Person::isAdult(alice))         # true

# Methods that modify the instance
Person::birthday(alice)
echo(alice.age)                      # 26

# Methods with parameters
Person::updateName(alice, "Alicia")
echo(alice.name)                     # Alicia
```

## Complete Example

```xenith
# Define a BankAccount struct
struct BankAccount {
    owner: string,
    balance: float,
    account_number: string
}

# Implement methods
impl BankAccount {
    # Constructor pattern
    method new(owner: string, initial_balance: float) -> BankAccount {
        spawn account: BankAccount = BankAccount {
            owner: owner,
            balance: initial_balance,
            account_number: "ACC" + (1000 as string)
        }
        release account
    }
    
    # Deposit money
    method deposit(self: Self, amount: float) -> null {
        when amount <= 0 {
            echo("Deposit amount must be positive")
            release null
        }
        self.balance = self.balance + amount
        echo("Deposited ${amount}. New balance: ${self.balance}")
        release null
    }
    
    # Withdraw money
    method withdraw(self: Self, amount: float) -> null {
        when amount <= 0 {
            echo("Withdrawal amount must be positive")
            release null
        }
        when amount > self.balance {
            echo("Insufficient funds!")
            release null
        }
        self.balance = self.balance - amount
        echo("Withdrew ${amount}. New balance: ${self.balance}")
        release null
    }
    
    # Check balance
    method getBalance(self: Self) -> float {
        release self.balance
    }
    
    # Transfer money
    method transfer(self: Self, target: BankAccount, amount: float) -> null {
        when amount > self.balance {
            echo("Cannot transfer ${amount}. Insufficient funds!")
            release null
        }
        self.balance = self.balance - amount
        target.balance = target.balance + amount
        echo("Transferred ${amount} from {self.owner} to {target.owner}")
        release null
    }
    
    # Display account info
    method display(self: Self) -> null {
        echo("Account: {self.account_number}")
        echo("Owner: {self.owner}")
        echo("Balance: ${self.balance}")
        release null
    }
}

# Usage
spawn alice_account: BankAccount = BankAccount {
    owner: "Alice",
    balance: 1000.0,
    account_number: "ACC1001"
}

spawn bob_account: BankAccount = BankAccount {
    owner: "Bob",
    balance: 500.0,
    account_number: "ACC1002"
}

# Deposit money
BankAccount::deposit(alice_account, 200.0)

# Withdraw money
BankAccount::withdraw(bob_account, 50.0)

# Transfer between accounts
BankAccount::transfer(alice_account, bob_account, 300.0)

# Check balances
echo("Alice's balance: ${BankAccount::getBalance(alice_account)}")
echo("Bob's balance: ${BankAccount::getBalance(bob_account)}")

# Display full info
BankAccount::display(alice_account)
```

## Multiple Methods Example

```xhenith
struct Rectangle {
    width: int,
    height: int
}

impl Rectangle {
    # Calculate area
    method area(self: Self) -> int {
        release self.width * self.height
    }
    
    # Calculate perimeter
    method perimeter(self: Self) -> int {
        release 2 * (self.width + self.height)
    }
    
    # Check if square
    method isSquare(self: Self) -> bool {
        release self.width == self.height
    }
    
    # Scale the rectangle
    method scale(self: Self, factor: int) -> null {
        self.width = self.width * factor
        self.height = self.height * factor
        release null
    }
    
    # Compare with another rectangle
    method isLargerThan(self: Self, other: Rectangle) -> bool {
        release self.area() > other.area()
    }
}

spawn rect1: Rectangle = Rectangle { width: 10, height: 5 }
spawn rect2: Rectangle = Rectangle { width: 7, height: 7 }

echo("Rectangle 1 area: {Rectangle::area(rect1)}")        # 50
echo("Rectangle 1 perimeter: {Rectangle::perimeter(rect1)}")  # 30
echo("Rectangle 1 is square: {Rectangle::isSquare(rect1)}")    # false

echo("Rectangle 2 area: {Rectangle::area(rect2)}")        # 49
echo("Rectangle 2 is square: {Rectangle::isSquare(rect2)}")    # true

echo("Rect1 larger than Rect2: {Rectangle::isLargerThan(rect1, rect2)}")  # true

Rectangle::scale(rect1, 2)
echo("After scaling: {Rectangle::area(rect1)}")  # 200
```

## Working with Nested Structs

```xenith
struct Address {
    street: string,
    city: string,
    zip: string
}

struct Person {
    name: string,
    age: int,
    address: Address
}

impl Person {
    method getFullAddress(self: Self) -> string {
        release "{self.address.street}, {self.address.city} {self.address.zip}"
    }
    
    method moveTo(self: Self, new_address: Address) -> null {
        self.address = new_address
        release null
    }
}

spawn alice: Person = Person {
    name: "Alice",
    age: 25,
    address: Address {
        street: "123 Main St",
        city: "Springfield",
        zip: "12345"
    }
}

echo(Person::getFullAddress(alice))
# Output: 123 Main St, Springfield 12345

spawn new_address: Address = Address {
    street: "456 Oak Ave",
    city: "Shelbyville",
    zip: "67890"
}

Person::moveTo(alice, new_address)
echo(Person::getFullAddress(alice))
# Output: 456 Oak Ave, Shelbyville 67890
```

## Structs with Lists and Maps

```xenith
struct Classroom {
    name: string,
    students: list<string>,
    scores: map<string, int>
}

impl Classroom {
    method addStudent(self: Self, student_name: string) -> null {
        self.students.append(student_name)
        self.scores[student_name] = 0
        release null
    }
    
    method setScore(self: Self, student: string, score: int) -> null {
        when self.scores.has_key(student) {
            self.scores[student] = score
        } otherwise {
            echo("Student {student} not found")
        }
        release null
    }
    
    method getAverage(self: Self) -> float {
        spawn total: int = 0
        for score in self.scores.values() {
            total = total + score
        }
        when self.students.len() == 0 {
            release 0.0
        }
        release (total as float) / (self.students.len() as float)
    }
    
    method getTopStudent(self: Self) -> string {
        spawn top_name: string = ""
        spawn top_score: int = -1
        
        for name, score in self.scores.items() {
            when score > top_score {
                top_score = score
                top_name = name
            }
        }
        release top_name
    }
}

spawn math_class: Classroom = Classroom {
    name: "Math 101",
    students: [],
    scores: {}
}

Classroom::addStudent(math_class, "Alice")
Classroom::addStudent(math_class, "Bob")
Classroom::addStudent(math_class, "Charlie")

Classroom::setScore(math_class, "Alice", 95)
Classroom::setScore(math_class, "Bob", 87)
Classroom::setScore(math_class, "Charlie", 92)

echo("Average: {Classroom::getAverage(math_class)}")        # 91.33
echo("Top student: {Classroom::getTopStudent(math_class)}") # Alice
```

## Why `Struct::method(instance)` Syntax?

Xenith uses this unique syntax to make it explicit whether you're calling:

1. **Instance methods** - Pass the instance as first argument
2. **Static methods** (coming soon) - No instance argument

This design choice emphasizes clarity and eliminates ambiguity about what `self` refers to.

## Best Practices

1. **Use descriptive field names** - Names should clearly indicate their purpose
2. **Keep structs focused** - Each struct should represent a single concept
3. **Document your structs** - Add comments for complex fields
4. **Group related methods** - Keep method definitions organized in `impl` blocks
5. **Validate in methods** - Check inputs before modifying state

```xenith
# Good - clear, focused struct
struct Product {
    id: string,
    name: string,
    price: float,
    in_stock: bool
}

# Avoid - too many unrelated fields
struct Data {
    user_name: string,
    temperature: float,
    counter: int,
    flag: bool
}
```

## Common Patterns

### Constructor Pattern

```xenith
impl Person {
    method new(name: string, age: int) -> Person {
        release Person {
            name: name,
            age: age
        }
    }
}

spawn alice: Person = Person::new("Alice", 25)
```

### Validation on Creation

```xenith
impl Person {
    method newValidated(name: string, age: int) -> Person {
        when age < 0 {
            echo("Age cannot be negative! Setting to 0")
            spawn age: int = 0
        }
        when name == "" {
            echo("Name cannot be empty! Setting to 'Unknown'")
            spawn name: string = "Unknown"
        }
        release Person {
            name: name,
            age: age
        }
    }
}
```

### Immutable Fields (via convention)

```xenith
# By convention, don't modify certain fields directly
struct User {
    id: string,      # Should never change
    username: string # Should never change
    email: string    # Can be changed via method
}

impl User {
    method updateEmail(self: Self, new_email: string) -> null {
        # Validate email format...
        self.email = new_email
        release null
    }
    
    method getId(self: Self) -> string {
        release self.id  # Read-only access
    }
}
```

## Next Steps

- Learn about [COLLECTIONS.md](COLLECTIONS.md) for lists and maps
- Read [METHODS.md](METHODS.md) for more on method definitions
- Explore [TYPE_SYSTEM.md](TYPE_SYSTEM.md) for advanced type concepts
```
