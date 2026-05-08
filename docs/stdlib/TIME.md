# std::time - Time Operations

## Introduction

The `std::time` module provides functions for working with system time, sleeping, and measuring durations.

## Importing

```xenith
grab {
    timestamp, timestamp_ms, sleep, sleep_sec,
    duration_secs, duration_ms
} from "std::time"
```

## Functions

### `timestamp() -> int`

Returns the current Unix timestamp (seconds since January 1, 1970).

```xenith
let now: int = timestamp()
echo("Current time: {now}")
```

### `timestamp_ms() -> int`

Returns the current Unix timestamp in milliseconds.

```xenith
let now_ms: int = timestamp_ms()
echo("Current time in milliseconds: {now_ms}")
```

### `sleep(ms: int) -> null`

Pauses execution for the specified number of milliseconds.

```xenith
echo("Starting...")
sleep(500)  # Wait 500 milliseconds
echo("500ms have passed!")
```

### `sleep_sec(seconds: int) -> null`

Pauses execution for the specified number of seconds.

```xenith
echo("Starting...")
sleep_sec(2)  # Wait 2 seconds
echo("2 seconds have passed!")
```

### `duration_secs(start_ms: int, end_ms: int) -> float`

Calculates the duration between two timestamps in seconds (as a float).

```xenith
let start: int = timestamp_ms()
sleep(1500)  # Sleep for 1.5 seconds
let end: int = timestamp_ms()
let elapsed: float = duration_secs(start, end)
echo("Elapsed: {elapsed} seconds")  # ~1.5
```

### `duration_ms(start_ms: int, end_ms: int) -> int`

Calculates the duration between two timestamps in milliseconds.

```xenith
let start: int = timestamp_ms()
sleep(500)
let end: int = timestamp_ms()
let elapsed: int = duration_ms(start, end)
echo("Elapsed: {elapsed} ms")  # ~500
```

## Complete Examples

### Simple Timer

```xenith
grab { timestamp_ms, sleep, duration_secs } from "std::time"

echo("Press Enter to start timer...")
input()

let start: int = timestamp_ms()
echo("Timer started!")

echo("Press Enter to stop timer...")
input()

let end: int = timestamp_ms()
let elapsed: float = duration_secs(start, end)

echo("Elapsed time: {elapsed} seconds")
```

### Countdown Timer

```xenith
grab { sleep_sec } from "std::time"

method countdown(seconds: int) -> null {
    let remaining: int = seconds
    while remaining > 0 {
        echo("{remaining}...")
        sleep_sec(1)
        remaining = remaining - 1
    }
    echo("Time's up!")
    release null
}

countdown(5)
```

### Performance Benchmarking

```xenith
grab { timestamp_ms, duration_ms } from "std::time"

method benchmark(method_to_test: method() -> any, name: string) -> null {
    echo("Benchmarking: {name}")

    let start: int = timestamp_ms()

    # Run the method
    method_to_test()

    let end: int = timestamp_ms()
    let elapsed: int = duration_ms(start, end)

    echo("  Time: {elapsed} ms")
    release null
}

# Example usage
method expensiveOperation() -> null {
    let sum: int = 0
    for i = 0 to 1000000 {
        sum = sum + i
    }
    release null
}

benchmark(expensiveOperation, "Sum of 1 million numbers")
```

### Rate Limiter

```xenith
grab { timestamp_ms, sleep, duration_ms } from "std::time"

class RateLimiter {
    last_call: int,
    min_interval_ms: int
}

method RateLimiter::new(interval_ms: int) -> RateLimiter {
    release RateLimiter {
        last_call: 0,
        min_interval_ms: interval_ms
    }
}

method RateLimiter::wait_if_needed(self: Self) -> null {
    let now: int = timestamp_ms()
    let elapsed: int = duration_ms(self.last_call, now)

    when elapsed < self.min_interval_ms {
        let wait_time: int = self.min_interval_ms - elapsed
        sleep(wait_time)
    }

    self.last_call = timestamp_ms()
    release null
}

# Usage
let limiter: RateLimiter = RateLimiter::new(1000)  # 1 call per second

for i = 0 to 5 {
    limiter.wait_if_needed()
    echo("Call {i + 1} at {timestamp_ms()}")
}
```

### Simple Stopwatch

```xenith
grab { timestamp_ms, duration_secs, sleep_sec } from "std::time"

class Stopwatch {
    start_time: int,
    running: bool
}

method Stopwatch::new() -> Stopwatch {
    release Stopwatch {
        start_time: 0,
        running: false
    }
}

method Stopwatch::start(self: Self) -> null {
    self.start_time = timestamp_ms()
    self.running = true
    echo("Stopwatch started")
    release null
}

method Stopwatch::stop(self: Self) -> null {
    when !self.running {
        echo("Stopwatch not running")
        release null
    }

    let end_time: int = timestamp_ms()
    let elapsed: float = duration_secs(self.start_time, end_time)
    self.running = false

    echo("Elapsed time: {elapsed} seconds")
    release null
}

method Stopwatch::reset(self: Self) -> null {
    self.start_time = 0
    self.running = false
    echo("Stopwatch reset")
    release null
}

# Usage
let watch: Stopwatch = Stopwatch::new()

watch.start()
sleep_sec(2)
watch.stop()

sleep_sec(1)
watch.start()
sleep_sec(3)
watch.stop()
```

## Error Handling

These functions generally don't fail, but it's good practice to handle potential issues:

```xenith
try {
    let ts: int = timestamp()
    echo("Timestamp: {ts}")
} catch err {
    echo("Failed to get timestamp: {err}")
}
```

## Performance Notes

- `timestamp()` and `timestamp_ms()` are very fast (system call overhead)
- `sleep()` and `sleep_sec()` block the current thread
- For high-precision timing, use `timestamp_ms()` for measurements
- Sleeping for very short periods (<10ms) may be less accurate due to OS scheduling

## Use Cases

- **Logging** - Add timestamps to log entries
- **Rate limiting** - Control API request frequency
- **Timeouts** - Implement operation deadlines
- **Benchmarking** - Measure code performance
- **Delays** - Add pauses in games or animations
- **Scheduling** - Run tasks at intervals

## See Also

- `std::random` - For random delays
- `std::process` - For process execution timing

```

```
