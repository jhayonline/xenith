# std::math - Advanced Mathematics

## Introduction

The `std::math` module provides advanced mathematical functions including trigonometry, logarithms, rounding, and statistics.

## Importing

```xenith
grab { sqrt, pow, sin, cos, tan, asin, acos, atan, atan2, log, log10, abs, min, max, clamp, round, floor, ceil, trunc, fract, radians, degrees, sum, average } from "std::math"
```

## Constants

The following constants are available globally (not in std::math):

| Constant | Value |
|----------|-------|
| `MATH_PI` | 3.141592653589793 |
| `TRUE` | true |
| `FALSE` | false |
| `NULL` | null |

## Functions

### Basic Operations

#### `sqrt(x: float) -> float`

Returns the square root of x.

```xenith
echo(sqrt(16))   # 4
echo(sqrt(2))    # 1.4142135623730951
```

**Errors:** Negative numbers cause a runtime error.

#### `pow(base: float, exponent: float) -> float`

Returns base raised to the power of exponent.

```xenith
echo(pow(2, 8))    # 256
echo(pow(5, 2))    # 25
echo(pow(4, 0.5))  # 2
```

#### `abs(x: float) -> float`

Returns the absolute value of x.

```xenith
echo(abs(-42))   # 42
echo(abs(3.14))  # 3.14
```

#### `min(a: float, b: float) -> float`

Returns the smaller of two numbers.

```xenith
echo(min(10, 20))   # 10
echo(min(-5, 3))    # -5
```

#### `max(a: float, b: float) -> float`

Returns the larger of two numbers.

```xenith
echo(max(10, 20))   # 20
echo(max(-5, 3))    # 3
```

#### `clamp(value: float, min_val: float, max_val: float) -> float`

Clamps a value between a minimum and maximum.

```xenith
echo(clamp(15, 10, 20))   # 15
echo(clamp(5, 10, 20))    # 10
echo(clamp(25, 10, 20))   # 20
```

### Rounding

#### `round(x: float) -> float`

Rounds to the nearest integer (halfway away from zero).

```xenith
echo(round(3.14))   # 3
echo(round(3.5))    # 4
echo(round(-3.5))   # -4
```

#### `floor(x: float) -> float`

Returns the largest integer less than or equal to x.

```xenith
echo(floor(3.9))    # 3
echo(floor(3.1))    # 3
echo(floor(-3.1))   # -4
```

#### `ceil(x: float) -> float`

Returns the smallest integer greater than or equal to x.

```xenith
echo(ceil(3.1))     # 4
echo(ceil(3.9))     # 4
echo(ceil(-3.1))    # -3
```

#### `trunc(x: float) -> float`

Returns the integer part of x (removes fractional part).

```xenith
echo(trunc(3.14))   # 3
echo(trunc(-3.14))  # -3
```

#### `fract(x: float) -> float`

Returns the fractional part of x.

```xenith
echo(fract(3.14))   # 0.14
echo(fract(-3.14))  # -0.14
```

### Trigonometry

All trigonometric functions work with **radians**, not degrees.

#### `sin(radians: float) -> float`

Returns the sine of an angle in radians.

```xenith
echo(sin(0))              # 0
echo(sin(MATH_PI / 2))    # 1
```

#### `cos(radians: float) -> float`

Returns the cosine of an angle in radians.

```xenith
echo(cos(0))              # 1
echo(cos(MATH_PI))        # -1
```

#### `tan(radians: float) -> float`

Returns the tangent of an angle in radians.

```xenith
echo(tan(0))              # 0
echo(tan(MATH_PI / 4))    # 1
```

#### `asin(x: float) -> float`

Returns the arcsine of x (inverse sine) in radians.

```xenith
echo(asin(1))    # 1.5707963267948966 (π/2)
```

**Errors:** x must be between -1 and 1.

#### `acos(x: float) -> float`

Returns the arccosine of x (inverse cosine) in radians.

```xenith
echo(acos(1))    # 0
```

**Errors:** x must be between -1 and 1.

#### `atan(x: float) -> float`

Returns the arctangent of x (inverse tangent) in radians.

```xenith
echo(atan(1))    # 0.7853981633974483 (π/4)
```

#### `atan2(y: float, x: float) -> float`

Returns the angle whose tangent is y/x, using both coordinates.

```xenith
echo(atan2(1, 1))    # 0.7853981633974483 (π/4)
echo(atan2(1, 0))    # 1.5707963267948966 (π/2)
```

### Angle Conversion

#### `radians(degrees: float) -> float`

Converts degrees to radians.

```xenith
echo(radians(180))    # 3.141592653589793
echo(radians(90))     # 1.5707963267948966
```

#### `degrees(radians: float) -> float`

Converts radians to degrees.

```xenith
echo(degrees(MATH_PI))     # 180
echo(degrees(MATH_PI / 2)) # 90
```

### Logarithms

#### `log(x: float) -> float`

Returns the natural logarithm (base e) of x.

```xenith
echo(log(2.71828))    # ~1
```

**Errors:** x must be positive.

#### `log10(x: float) -> float`

Returns the base-10 logarithm of x.

```xenith
echo(log10(100))    # 2
echo(log10(1000))   # 3
```

**Errors:** x must be positive.

### Statistics

#### `sum(numbers: list<float>) -> float`

Returns the sum of all numbers in a list.

```xenith
spawn nums: list<float> = [10, 20, 30, 40, 50]
echo(sum(nums))    # 150
```

#### `average(numbers: list<float>) -> float`

Returns the arithmetic mean of all numbers in a list.

```xenith
spawn nums: list<float> = [10, 20, 30, 40, 50]
echo(average(nums))    # 30
```

## Complete Examples

### Right Triangle Calculator

```xenith
grab { sqrt, pow, sin, cos, tan, asin, acos, atan, radians, degrees } from "std::math"

method solve_triangle(a: float, b: float) -> null {
    spawn c: float = sqrt(pow(a, 2) + pow(b, 2))
    spawn angle_A: float = degrees(atan(a / b))
    spawn angle_B: float = degrees(atan(b / a))
    
    echo("Side a: {a}")
    echo("Side b: {b}")
    echo("Hypotenuse c: {c}")
    echo("Angle A: {angle_A}°")
    echo("Angle B: {angle_B}°")
    echo("Angle C: 90°")
    release null
}

solve_triangle(3, 4)
# Output:
# Side a: 3
# Side b: 4
# Hypotenuse c: 5
# Angle A: 36.87°
# Angle B: 53.13°
# Angle C: 90°
```

### Projectile Motion Calculator

```xenith
grab { sin, cos, radians, pow } from "std::math"

const spawn GRAVITY: float = 9.81

method calculate_range(velocity: float, angle_deg: float) -> float {
    spawn angle_rad: float = radians(angle_deg)
    release pow(velocity, 2) * sin(2 * angle_rad) / GRAVITY
}

method calculate_max_height(velocity: float, angle_deg: float) -> float {
    spawn angle_rad: float = radians(angle_deg)
    release pow(velocity * sin(angle_rad), 2) / (2 * GRAVITY)
}

method calculate_time_of_flight(velocity: float, angle_deg: float) -> float {
    spawn angle_rad: float = radians(angle_deg)
    release 2 * velocity * sin(angle_rad) / GRAVITY
}

spawn v: float = 50.0
spawn angle: float = 45.0

echo("Projectile launched at {v} m/s, {angle}°")
echo("Range: {calculate_range(v, angle)} m")
echo("Max height: {calculate_max_height(v, angle)} m")
echo("Time of flight: {calculate_time_of_flight(v, angle)} s")
```

### Statistical Calculator

```xenith
grab { sum, average, sqrt, pow, min, max } from "std::math"

method variance(numbers: list<float>) -> float {
    spawn avg: float = average(numbers)
    spawn sum_sq: float = 0.0
    
    for n in numbers {
        sum_sq = sum_sq + pow(n - avg, 2)
    }
    
    release sum_sq / (numbers.len() as float)
}

method stddev(numbers: list<float>) -> float {
    release sqrt(variance(numbers))
}

method median(numbers: list<float>) -> float {
    spawn sorted: list<float> = numbers
    # Note: sort would go here
    spawn mid: int = sorted.len() / 2
    
    when sorted.len() % 2 == 0 {
        release (sorted[mid - 1] + sorted[mid]) / 2.0
    }
    release sorted[mid]
}

spawn data: list<float> = [12, 15, 18, 22, 25, 30, 35, 40]

echo("Data: {ret(data)}")
echo("Count: {data.len()}")
echo("Sum: {sum(data)}")
echo("Average: {average(data)}")
echo("Min: {min(data[0], data[data.len()-1])}")
echo("Max: {max(data[0], data[data.len()-1])}")
echo("Variance: {variance(data)}")
echo("Std Dev: {stddev(data)}")
echo("Median: {median(data)}")
```

### Circle Geometry Calculator

```xenith
grab { sqrt, pow } from "std::math"

method circle_area(radius: float) -> float {
    release MATH_PI * pow(radius, 2)
}

method circle_circumference(radius: float) -> float {
    release 2 * MATH_PI * radius
}

method circle_diameter(radius: float) -> float {
    release 2 * radius
}

method chord_length(radius: float, angle_deg: float) -> float {
    spawn angle_rad: float = radians(angle_deg)
    release 2 * radius * sin(angle_rad / 2)
}

spawn r: float = 5.0
echo("Radius: {r}")
echo("Diameter: {circle_diameter(r)}")
echo("Circumference: {circle_circumference(r)}")
echo("Area: {circle_area(r)}")
echo("Chord length at 60°: {chord_length(r, 60)}")
```

## Error Handling

Some math functions have domain restrictions:

```xenith
try {
    spawn result: float = sqrt(-1)
} catch err {
    echo("Error: {err}")  # Cannot take square root of negative number
}

try {
    spawn result: float = asin(2)
} catch err {
    echo("Error: {err}")  # asin argument must be between -1 and 1
}

try {
    spawn result: float = log(0)
} catch err {
    echo("Error: {err}")  # Cannot take logarithm of non-positive number
}
```

## Performance Notes

- All functions are implemented in Rust and are very fast
- `sum()` and `average()` iterate through the entire list (O(n))
- Trigonometric functions use Rust's standard library (high precision)

## See Also

- Built-in `MATH_PI` constant
- `std::random` for random number generation
- `std::collections` for data structures
```
