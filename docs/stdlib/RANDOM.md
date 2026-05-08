# std::random - Random Generation

## Introduction

The `std::random` module provides functions for generating random numbers, selecting random elements, shuffling lists, and creating UUIDs.

## Importing

```xenith
grab { rand_int, rand_int_range, rand_float, rand_float_range, rand_bool, choice, shuffle, uuid } from "std::random"
```

## Functions

### `rand_int() -> int`

Returns a random integer across the full 64-bit range.

```xenith
let num: int = rand_int()
echo(num)  # Example: -9143264551332010000
```

### `rand_int_range(min: int, max: int) -> int`

Returns a random integer between `min` and `max` (inclusive).

```xenith
# Random number between 1 and 10
let dice: int = rand_int_range(1, 10)

# Random number between 0 and 99
let index: int = rand_int_range(0, 99)
```

### `rand_float() -> float`

Returns a random float between 0.0 and 1.0 (inclusive of 0.0, exclusive of 1.0).

```xenith
let percentage: float = rand_float()
echo(percentage)  # Example: 0.04806421861508503
```

### `rand_float_range(min: float, max: float) -> float`

Returns a random float between `min` and `max` (inclusive).

```xenith
# Random temperature between 18.5 and 26.5
let temp: float = rand_float_range(18.5, 26.5)

# Random price between 9.99 and 19.99
let price: float = rand_float_range(9.99, 19.99)
```

### `rand_bool() -> bool`

Returns a random boolean value (true or false).

```xenith
when rand_bool() {
    echo("Heads")
} otherwise {
    echo("Tails")
}
```

### `choice(items: list<any>) -> any`

Returns a random element from a list.

```xenith
let fruits: list<string> = ["apple", "banana", "orange", "grape", "mango"]
let random_fruit: string = choice(fruits)
echo(random_fruit)  # Example: "apple"

let colors: list<string> = ["red", "blue", "green"]
let random_color: string = choice(colors)
```

### `shuffle(items: list<any>) -> list<any>`

Returns a new list with the elements randomly shuffled (original list unchanged).

```xenith
let numbers: list<int> = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
let shuffled: list<int> = shuffle(numbers)
echo(ret(shuffled))  # Example: [3, 2, 6, 1, 8, 9, 4, 10, 7, 5]

# Original list remains unchanged
echo(ret(numbers))   # [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
```

### `uuid() -> string`

Returns a random UUID (Universally Unique Identifier) version 4.

```xenith
let id: string = uuid()
echo(id)  # Example: "56fbb2d7-10f6-421f-afa5-b0ed3af8d29d"

let session_id: string = uuid()
```

## Complete Examples

### Dice Roller

```xenith
grab { rand_int_range } from "std::random"

method roll_dice(sides: int) -> int {
    release rand_int_range(1, sides)
}

echo("D6: {roll_dice(6)}")
echo("D20: {roll_dice(20)}")
echo("D100: {roll_dice(100)}")
```

### Random Password Generator

```xenith
grab { rand_int_range, choice } from "std::random"

method generate_password(length: int) -> string {
    let chars: list<string> = [
        "A","B","C","D","E","F","G","H","I","J","K","L","M",
        "N","O","P","Q","R","S","T","U","V","W","X","Y","Z",
        "a","b","c","d","e","f","g","h","i","j","k","l","m",
        "n","o","p","q","r","s","t","u","v","w","x","y","z",
        "0","1","2","3","4","5","6","7","8","9",
        "!","@","#","$","%","^","&","*","(",")"
    ]

    let result: string = ""
    for i = 0 to length {
        result = result + choice(chars)
    }
    release result
}

let password: string = generate_password(12)
echo("Password: {password}")
```

### Coin Flip Simulator

```xenith
grab { rand_bool } from "std::random"

method flip_coin() -> string {
    when rand_bool() {
        release "Heads"
    }
    release "Tails"
}

let heads: int = 0
let tails: int = 0

for i = 0 to 100 {
    let result: string = flip_coin()
    when result == "Heads" {
        heads = heads + 1
    } otherwise {
        tails = tails + 1
    }
}

echo("Heads: {heads}, Tails: {tails}")
```

### Random Item Selector

```xenith
grab { choice } from "std::random"

let menu: list<string> = [
    "Pizza", "Burger", "Sushi", "Pasta", "Salad"
]

let lunch: string = choice(menu)
echo("You should eat: {lunch}")
```

### Deck Shuffler

```xenith
grab { shuffle } from "std::random"

method create_deck() -> list<string> {
    let suits: list<string> = ["Hearts", "Diamonds", "Clubs", "Spades"]
    let values: list<string> = ["A","2","3","4","5","6","7","8","9","10","J","Q","K"]
    let deck: list<string> = []

    for suit in suits {
        for value in values {
            deck.append(value + " of " + suit)
        }
    }
    release deck
}

let deck: list<string> = create_deck()
let shuffled_deck: list<string> = shuffle(deck)

echo("First card: {shuffled_deck[0]}")
```

### Random Number Guessing Game

```xenith
grab { rand_int_range } from "std::random"

method guessing_game() -> null {
    let secret: int = rand_int_range(1, 100)
    let attempts: int = 0
    let guessed: bool = false

    echo("I'm thinking of a number between 1 and 100")

    while !guessed {
        echo("Enter your guess: ")
        let guess: int = input_int()
        attempts = attempts + 1

        when guess == secret {
            echo("Correct! You guessed it in {attempts} attempts!")
            guessed = true
        } or when guess < secret {
            echo("Too low!")
        } otherwise {
            echo("Too high!")
        }
    }
    release null
}

guessing_game()
```

### Random Color Generator

```xenith
grab { rand_int_range } from "std::random"

method random_rgb() -> string {
    let r: int = rand_int_range(0, 255)
    let g: int = rand_int_range(0, 255)
    let b: int = rand_int_range(0, 255)
    release "rgb({r}, {g}, {b})"
}

method random_hex() -> string {
    let hex_chars: list<string> = ["0","1","2","3","4","5","6","7","8","9","A","B","C","D","E","F"]
    let result: string = "#"
    for i = 0 to 6 {
        result = result + choice(hex_chars)
    }
    release result
}

echo("Random RGB: {random_rgb()}")
echo("Random Hex: {random_hex()}")
```

### Unique ID Generator

```xenith
grab { uuid, rand_int_range } from "std::random"

method short_id() -> string {
    let chars: list<string> = [
        "A","B","C","D","E","F","G","H","I","J","K","L","M",
        "N","O","P","Q","R","S","T","U","V","W","X","Y","Z",
        "0","1","2","3","4","5","6","7","8","9"
    ]

    let result: string = ""
    for i = 0 to 8 {
        result = result + choice(chars)
    }
    release result
}

echo("Full UUID: {uuid()}")
echo("Short ID: {short_id()}")
```

### Monte Carlo Simulation

```xenith
grab { rand_float } from "std::random"

method estimate_pi(iterations: int) -> float {
    let inside_circle: int = 0

    for i = 0 to iterations {
        let x: float = rand_float()
        let y: float = rand_float()

        when x*x + y*y <= 1.0 {
            inside_circle = inside_circle + 1
        }
    }

    release 4.0 * (inside_circle as float) / (iterations as float)
}

let pi_estimate: float = estimate_pi(1000000)
echo("Estimated PI: {pi_estimate}")
echo("Actual PI: {MATH_PI}")
```

## Performance Notes

- All functions are implemented in Rust and are very fast
- The random number generator is thread-local for safety and performance
- `shuffle()` creates a new list (O(n) time and space)
- `choice()` is O(1) time

## See Also

- `std::math` for mathematical functions
- `std::time` for generating time-based seeds

```

```
