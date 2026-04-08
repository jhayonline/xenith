# Xenith Built-in Constants

## Introduction

Xenith provides several built-in constants for common values. These constants are globally available and can be used anywhere in your code.

## Boolean Constants

### `TRUE` and `FALSE`

Boolean constants representing true and false values.

```xenith
# Direct usage
spawn is_active: bool = TRUE
spawn is_deleted: bool = FALSE

# In conditions
when is_active == TRUE {
    echo("System is active")
}

# In comparisons
spawn flag: bool = TRUE
when flag {
    echo("Flag is true")
}

# With logical operators
spawn result: bool = TRUE && FALSE  # false
```

### Practical Example with Booleans

```xenith
method processUser(age: int, has_license: bool) -> string {
    when age >= 18 && has_license == TRUE {
        release "User can drive"
    } or when age >= 18 && has_license == FALSE {
        release "User is adult but needs license"
    } otherwise {
        release "User is a minor"
    }
}

spawn alice_age: int = 25
spawn alice_license: bool = TRUE
echo(processUser(alice_age, alice_license))  # User can drive

spawn bob_age: int = 20
spawn bob_license: bool = FALSE
echo(processUser(bob_age, bob_license))  # User is adult but needs license
```

## Null Constant

### `NULL`

Represents the absence of a value. Often used for optional values or method returns.

```xenith
# Initialize variables as null
spawn user_data: string = NULL
spawn temp_value: int = NULL

# Methods can return null
method findUser(name: string) -> string {
    when name == "Alice" {
        release "Found: Alice"
    }
    release NULL  # User not found
}

spawn result: string = findUser("Bob")
when result == NULL {
    echo("User not found")
} otherwise {
    echo(result)
}
```

### Practical Null Examples

```xenith
# Configuration with optional values
spawn config_timeout: int = NULL
spawn config_retry: bool = NULL

method getTimeout() -> int {
    when config_timeout == NULL {
        release 30  # Default value
    }
    release config_timeout
}

method isRetryEnabled() -> bool {
    when config_retry == NULL {
        release TRUE  # Default to enabled
    }
    release config_retry
}

# Setting configuration later
config_timeout = 60
config_retry = FALSE

echo("Timeout: {getTimeout()}")        # 60
echo("Retry enabled: {isRetryEnabled()}")  # false
```

### Null in Data Structures

```xenith
# Optional fields in structs
struct User {
    name: string,
    email: string,
    phone: string  # Optional field
}

spawn alice: User = User {
    name: "Alice",
    email: "alice@example.com",
    phone: NULL  # No phone number
}

method displayUserInfo(user: User) -> null {
    echo("Name: {user.name}")
    echo("Email: {user.email}")
    
    when user.phone == NULL {
        echo("Phone: Not provided")
    } otherwise {
        echo("Phone: {user.phone}")
    }
    release null
}

displayUserInfo(alice)
# Output:
# Name: Alice
# Email: alice@example.com
# Phone: Not provided
```

### Null Checking Best Practices

```xenith
# Always check for null before using
spawn data: string = NULL

# Safe - check first
when data == NULL {
    echo("No data available")
} otherwise {
    echo("Data: {data}")
}

# Dangerous - might cause error if used directly
# echo(data)  # Would panic if data is NULL

# Safe access pattern
method processValue(value: string) -> null {
    when value == NULL {
        echo("Warning: Received null value")
        release null
    }
    echo("Processing: {value}")
    release null
}
```

## Mathematical Constant

### `MATH_PI`

The mathematical constant π (pi), approximately 3.141592653589793.

```xenith
# Basic usage
spawn pi: float = MATH_PI
echo(pi)  # 3.141592653589793

# Circle calculations
method circleArea(radius: float) -> float {
    release MATH_PI * radius * radius
}

method circleCircumference(radius: float) -> float {
    release 2 * MATH_PI * radius
}

spawn r: float = 5.0
echo("Area: {circleArea(r)}")           # 78.5398
echo("Circumference: {circleCircumference(r)}")  # 31.4159
```

### Practical PI Examples

```xenith
# Geometry calculations
method sphereVolume(radius: float) -> float {
    release (4.0 / 3.0) * MATH_PI * (radius ^ 3)
}

method sphereSurfaceArea(radius: float) -> float {
    release 4 * MATH_PI * (radius ^ 2)
}

method cylinderVolume(radius: float, height: float) -> float {
    release MATH_PI * (radius ^ 2) * height
}

method cylinderSurfaceArea(radius: float, height: float) -> float {
    release 2 * MATH_PI * radius * (radius + height)
}

# Test calculations
spawn radius: float = 3.0
spawn height: float = 10.0

echo("Sphere radius: {radius}")
echo("Sphere volume: {sphereVolume(radius)}")
echo("Sphere surface area: {sphereSurfaceArea(radius)}")

echo("\nCylinder radius: {radius}, height: {height}")
echo("Cylinder volume: {cylinderVolume(radius, height)}")
echo("Cylinder surface area: {cylinderSurfaceArea(radius, height)}")
```

### Trigonometry with PI

```xenith
# Convert degrees to radians
method toRadians(degrees: float) -> float {
    release degrees * MATH_PI / 180.0
}

# Convert radians to degrees
method toDegrees(radians: float) -> float {
    release radians * 180.0 / MATH_PI
}

# Trigonometric calculations (approximations)
method sinTaylor(x: float) -> float {
    # Simplified sine approximation using Taylor series
    release x - (x ^ 3) / 6.0 + (x ^ 5) / 120.0
}

# Test conversions
spawn angle_deg: float = 90.0
spawn angle_rad: float = toRadians(angle_deg)
echo("{angle_deg}° = {angle_rad} radians")

spawn back_to_deg: float = toDegrees(angle_rad)
echo("{angle_rad} rad = {back_to_deg}°")

# Calculate sine of 30 degrees
spawn thirty_deg: float = 30.0
spawn thirty_rad: float = toRadians(thirty_deg)
spawn sin_val: float = sinTaylor(thirty_rad)
echo("sin(30°) ≈ {sin_val}")  # Approximately 0.5
```

## Complete Example: Geometry Calculator

```xenith
# geometry_calc.xen
method showCircleMenu() -> null {
    clear()
    echo("=== Circle Calculator ===")
    echo("1. Calculate area")
    echo("2. Calculate circumference")
    echo("3. Calculate diameter")
    echo("4. Back to main menu")
    release null
}

method calculateCircleArea() -> null {
    clear()
    echo("Enter circle radius: ")
    spawn radius: float = input_int() as float
    
    spawn area: float = MATH_PI * radius * radius
    echo("\nCircle with radius {radius}:")
    echo("Area = {area}")
    
    echo("\nPress Enter to continue...")
    input()
    release null
}

method calculateCircleCircumference() -> null {
    clear()
    echo("Enter circle radius: ")
    spawn radius: float = input_int() as float
    
    spawn circumference: float = 2 * MATH_PI * radius
    echo("\nCircle with radius {radius}:")
    echo("Circumference = {circumference}")
    
    echo("\nPress Enter to continue...")
    input()
    release null
}

method calculateCircleDiameter() -> null {
    clear()
    echo("Enter circle radius: ")
    spawn radius: float = input_int() as float
    
    spawn diameter: float = 2 * radius
    echo("\nCircle with radius {radius}:")
    echo("Diameter = {diameter}")
    
    echo("\nPress Enter to continue...")
    input()
    release null
}

method circleCalculator() -> null {
    spawn running: bool = TRUE
    
    while running {
        showCircleMenu()
        spawn choice: int = input_int()
        
        match choice {
            1 => { calculateCircleArea() }
            2 => { calculateCircleCircumference() }
            3 => { calculateCircleDiameter() }
            4 => { running = FALSE }
            _ => {
                echo("Invalid choice!")
                echo("Press Enter to continue...")
                input()
            }
        }
    }
    release null
}

method showMainMenu() -> null {
    clear()
    echo("=== Geometry Calculator ===")
    echo("1. Circle Calculator")
    echo("2. Sphere Calculator")
    echo("3. Cylinder Calculator")
    echo("4. Exit")
    echo("\nChoice: ")
    release null
}

method calculateSphere() -> null {
    clear()
    echo("Enter sphere radius: ")
    spawn radius: float = input_int() as float
    
    spawn volume: float = (4.0 / 3.0) * MATH_PI * (radius ^ 3)
    spawn surface: float = 4 * MATH_PI * (radius ^ 2)
    
    echo("\nSphere with radius {radius}:")
    echo("Volume = {volume}")
    echo("Surface Area = {surface}")
    
    echo("\nPress Enter to continue...")
    input()
    release null
}

method calculateCylinder() -> null {
    clear()
    echo("Enter cylinder radius: ")
    spawn radius: float = input_int() as float
    echo("Enter cylinder height: ")
    spawn height: float = input_int() as float
    
    spawn volume: float = MATH_PI * (radius ^ 2) * height
    spawn surface: float = 2 * MATH_PI * radius * (radius + height)
    
    echo("\nCylinder with radius {radius}, height {height}:")
    echo("Volume = {volume}")
    echo("Surface Area = {surface}")
    
    echo("\nPress Enter to continue...")
    input()
    release null
}

method main() -> null {
    spawn running: bool = TRUE
    
    while running {
        showMainMenu()
        spawn choice: int = input_int()
        
        match choice {
            1 => { circleCalculator() }
            2 => { calculateSphere() }
            3 => { calculateCylinder() }
            4 => {
                echo("Goodbye!")
                running = FALSE
            }
            _ => {
                echo("Invalid choice!")
                echo("Press Enter to continue...")
                input()
            }
        }
    }
    release null
}

# Run the program
main()
```

## Advanced Example: Physics Calculator

```xenith
# physics.xen
method calculatePendulumPeriod(length: float, gravity: float) -> float {
    # T = 2π √(L/g)
    release 2 * MATH_PI * ((length / gravity) ^ 0.5)
}

method calculateWaveSpeed(frequency: float, wavelength: float) -> float {
    # v = fλ
    release frequency * wavelength
}

method calculateCircularMotionPeriod(radius: float, velocity: float) -> float {
    # T = 2πr / v
    release 2 * MATH_PI * radius / velocity
}

method calculateAngularVelocity(period: float) -> float {
    # ω = 2π / T
    release 2 * MATH_PI / period
}

# Main program
clear()
echo("=== Physics Calculator ===")

# Pendulum calculation
spawn pendulum_length: float = 2.0  # meters
spawn gravity: float = 9.81  # m/s²
spawn period: float = calculatePendulumPeriod(pendulum_length, gravity)
echo("Pendulum length: {pendulum_length}m")
echo("Period: {period}s")

# Wave calculation
spawn frequency: float = 440.0  # Hz (A4 note)
spawn wavelength: float = 0.78  # meters
spawn speed: float = calculateWaveSpeed(frequency, wavelength)
echo("\nWave frequency: {frequency}Hz")
echo("Wavelength: {wavelength}m")
echo("Wave speed: {speed}m/s")

# Circular motion
spawn orbit_radius: float = 10.0  # meters
spawn orbit_velocity: float = 5.0  # m/s
spawn orbit_period: float = calculateCircularMotionPeriod(orbit_radius, orbit_velocity)
spawn angular_vel: float = calculateAngularVelocity(orbit_period)
echo("\nCircular motion:")
echo("Radius: {orbit_radius}m")
echo("Velocity: {orbit_velocity}m/s")
echo("Period: {orbit_period}s")
echo("Angular velocity: {angular_vel} rad/s")

# Demonstrate constants
echo("\n=== Constants Used ===")
echo("MATH_PI = {MATH_PI}")
echo("TRUE = {TRUE}")
echo("FALSE = {FALSE}")
echo("NULL = {NULL}")
```

## Best Practices

1. **Use constants for clarity** - `TRUE` is more readable than `1`
2. **Check for NULL explicitly** - Always compare with `== NULL`
3. **Use MATH_PI for precision** - More accurate than hardcoding 3.14
4. **Combine with type checking** - Verify types when using NULL

```xenith
# Good - explicit null check
when value == NULL {
    echo("Value is null")
}

# Good - using TRUE/FALSE for clarity
spawn is_ready: bool = TRUE
when is_ready == TRUE {
    echo("Ready!")
}

# Good - using MATH_PI
spawn circumference: float = 2 * MATH_PI * radius

# Avoid - magic numbers
# spawn circumference: float = 2 * 3.14159 * radius  # Less accurate
```

## Common Patterns

### Optional Value Pattern

```xenith
method findInCache(key: string) -> string {
    # Simulated cache lookup
    when key == "user_123" {
        release "Alice"
    }
    release NULL
}

spawn cached_value: string = findInCache("user_456")
when cached_value == NULL {
    echo("Cache miss - loading from database")
    cached_value = "Bob"
} otherwise {
    echo("Cache hit: {cached_value}")
}
```

### Default Value Pattern

```xenith
method getConfig(key: string) -> string {
    # Simulated config lookup
    spawn config: map<string, string> = {
        "timeout": "30",
        "retries": "3"
    }
    
    when config.has_key(key) {
        release config[key]
    }
    release NULL
}

spawn timeout: string = getConfig("timeout")
when timeout == NULL {
    timeout = "60"  # Default value
}
echo("Timeout: {timeout}")
```

### Initialization Pattern

```xenith
spawn database_connection: string = NULL

method initDatabase() -> null {
    when database_connection == NULL {
        database_connection = "Connected to DB"
        echo("Database initialized")
    } otherwise {
        echo("Database already initialized")
    }
    release null
}

method queryDatabase(sql: string) -> null {
    when database_connection == NULL {
        echo("Error: Database not initialized!")
        release null
    }
    echo("Executing: {sql}")
    release null
}

initDatabase()    # Initializes connection
initDatabase()    # Shows "already initialized"
queryDatabase("SELECT * FROM users")
```

## Next Steps

- Learn about [OPERATORS.md](OPERATORS.md) for working with values
- Read [BUILT-IN_METHODS.md](BUILT-IN_METHODS.md) for available functions
- Explore [TYPE_SYSTEM.md](TYPE_SYSTEM.md) for advanced type concepts
```
