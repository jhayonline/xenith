# std::dotenv - Environment Variables from .env Files

## Introduction

The `std::dotenv` module loads environment variables from `.env` files into your Xenith application. This is useful for configuration management, keeping secrets out of your code.

## Importing

```xenith
grab { load, load_file, get, get_or_default, has, set, unset, vars } from "std::dotenv"
```

## .env File Format

Create a `.env` file in your project root:

```env
# Comments start with #
DB_HOST=localhost
DB_PORT=5432
DB_USER=admin
DB_PASSWORD=secret123
API_KEY=abc123xyz
DEBUG=true

# Quotes are optional
APP_NAME="My Application"
```

## Functions

### `load() -> null`

Loads the `.env` file from the current working directory.

```xenith
try {
    load()
    echo(".env loaded successfully")
} catch err {
    echo("No .env file found")
}
```

### `load_file(path: string) -> null`

Loads a `.env` file from a specific path.

```xenith
# Load from custom location
load_file("config/.env.production")

# Load from parent directory
load_file("../.env")
```

### `get(key: string) -> string`

Returns the value of an environment variable. Throws an error if not found.

```xenith
let db_host: string = get("DB_HOST")
let db_port: int = get("DB_PORT") as int
```

### `get_or_default(key: string, default: string) -> string`

Returns the value of an environment variable, or a default value if not found.

```xenith
let api_key: string = get_or_default("API_KEY", "default_key")
let debug: bool = get_or_default("DEBUG", "false") == "true"
```

### `has(key: string) -> bool`

Checks if an environment variable exists.

```xenith
when has("API_KEY") {
    let key: string = get("API_KEY")
    echo("API key configured")
} otherwise {
    echo("Warning: API_KEY not set")
}
```

### `set(key: string, value: string) -> null`

Sets an environment variable at runtime (affects current process only, not the .env file).

```xenith
set("TEMP_DIR", "/tmp/myapp")
set("DEBUG", "true")
```

### `unset(key: string) -> null`

Removes an environment variable at runtime.

```xenith
unset("TEMP_VAR")
```

### `vars() -> map<string, string>`

Returns all loaded environment variables as a map.

```xenith
let all_vars: map<string, string> = vars()
for key, value in all_vars.items() {
    echo("{key} = {value}")
}
```

## Complete Examples

### Database Configuration

```xenith
grab { load, get, get_or_default } from "std::dotenv"

# Load configuration
load()

# Get database settings
let db_host: string = get("DB_HOST")
let db_port: int = get("DB_PORT") as int
let db_user: string = get("DB_USER")
let db_password: string = get("DB_PASSWORD")

# Connect to database (example)
echo("Connecting to {db_host}:{db_port} as {db_user}")
```

### Application Settings

```xenith
grab { load, get_or_default } from "std::dotenv"

load()

# Application configuration with defaults
let app_name: string = get_or_default("APP_NAME", "XenithApp")
let app_port: int = get_or_default("PORT", "3000") as int
let debug_mode: bool = get_or_default("DEBUG", "false") == "true"
let log_level: string = get_or_default("LOG_LEVEL", "info")

when debug_mode {
    echo("Running in debug mode")
}

echo("{app_name} starting on port {app_port}")
```

### API Client Configuration

```xenith
grab { load, get, has } from "std::dotenv"

load()

# Check required variables
let required_vars: list<string> = ["API_KEY", "API_URL", "API_TIMEOUT"]
let missing: list<string> = []

for var in required_vars {
    when !has(var) {
        missing.append(var)
    }
}

when missing.len() > 0 {
    echo("Missing required environment variables:")
    for var in missing {
        echo("  {var}")
    }
    panic "Configuration error"
}

# All required variables present
let api_key: string = get("API_KEY")
let api_url: string = get("API_URL")
let api_timeout: int = get("API_TIMEOUT") as int

echo("API client configured for {api_url}")
```

### Environment-specific Configuration

```xenith
grab { load_file, get_or_default } from "std::dotenv"

method load_config(env: string) -> null {
    let config_file: string = ".env." + env

    try {
        load_file(config_file)
        echo("Loaded {config_file}")
    } catch err {
        echo("Warning: {config_file} not found, using defaults")
    }
    release null
}

# Load environment-specific config
let environment: string = get_or_default("ENV", "development")
load_config(environment)

let debug: bool = get_or_default("DEBUG", "false") == "true"
let log_level: string = get_or_default("LOG_LEVEL", "info")

echo("Environment: {environment}")
echo("Debug: {debug}")
echo("Log level: {log_level}")
```

### Runtime Configuration Override

```xenith
grab { load, get, set, unset } from "std::dotenv"

load()

# Original value
let original: string = get("LOG_LEVEL")
echo("Original LOG_LEVEL: {original}")

# Override for this session
set("LOG_LEVEL", "debug")
let overridden: string = get("LOG_LEVEL")
echo("Overridden LOG_LEVEL: {overridden}")

# Restore (unset reverts to .env value? No, unset removes it)
unset("LOG_LEVEL")
when !has("LOG_LEVEL") {
    echo("LOG_LEVEL has been removed")
}
```

### Feature Flags

```xenith
grab { load, has } from "std::dotenv"

load()

# Feature flags from .env
let enable_feature_x: bool = has("FEATURE_X_ENABLED")
let enable_feature_y: bool = has("FEATURE_Y_ENABLED")
let beta_features: bool = has("BETA_FEATURES")

when beta_features {
    echo("Beta features enabled")
    when enable_feature_x {
        echo("Feature X active")
    }
    when enable_feature_y {
        echo("Feature Y active")
    }
}
```

## Error Handling

Always use try-catch when loading .env files:

```xenith
try {
    load()
    echo("Configuration loaded")
} catch err {
    echo("Warning: No .env file found, using defaults")
}

try {
    let value: string = get("MISSING_KEY")
} catch err {
    echo("Key not found, using default")
    let value: string = "default"
}
```

## Security Notes

- Never commit `.env` files with secrets to version control
- Add `.env` to your `.gitignore`
- Use `.env.example` to document required variables
- Environment variables are accessible to child processes

## See Also

- `std::fs` - For reading configuration files
- `std::process` - For accessing process environment

```

```
