# std::fs - File System Operations

## Introduction

The `std::fs` module provides file and directory operations for Xenith programs. All functions are synchronous and return errors that can be caught with `try-catch`.

## Importing

```xenith
grab { read, write, append, exists, is_file, is_dir, mkdir, mkdir_all, remove, remove_all, list_dir, copy } from "std::fs"
```

## Functions

### `read(path: string) -> string`

Reads the entire contents of a file as a string.

```xenith
spawn content: string = read("config.txt")
echo(content)
```

**Errors:** File not found, permission denied

### `write(path: string, content: string) -> null`

Writes a string to a file (overwrites existing content).

```xenith
write("output.txt", "Hello, World!")
```

**Errors:** Permission denied, invalid path

### `append(path: string, content: string) -> null`

Appends a string to the end of a file. Creates the file if it doesn't exist.

```xenith
append("log.txt", "New log entry\n")
```

**Errors:** Permission denied, invalid path

### `exists(path: string) -> bool`

Returns `true` if a file or directory exists at the given path.

```xenith
when exists("data.txt") {
    spawn content: string = read("data.txt")
} otherwise {
    echo("File not found")
}
```

### `is_file(path: string) -> bool`

Returns `true` if the path exists and is a regular file.

```xenith
when is_file("document.txt") {
    echo("It's a file!")
}
```

### `is_dir(path: string) -> bool`

Returns `true` if the path exists and is a directory.

```xenith
when is_dir("src/") {
    echo("It's a directory!")
}
```

### `mkdir(path: string) -> null`

Creates a new directory. Fails if parent directories don't exist.

```xenith
mkdir("new_folder")
```

**Errors:** Parent directory missing, permission denied, directory already exists

### `mkdir_all(path: string) -> null`

Creates a directory and all parent directories if they don't exist.

```xenith
mkdir_all("a/b/c/d")  # Creates all intermediate directories
```

**Errors:** Permission denied

### `remove(path: string) -> null`

Deletes a file or empty directory.

```xenith
remove("temp.txt")
remove("empty_folder")
```

**Errors:** File not found, directory not empty, permission denied

### `remove_all(path: string) -> null`

Deletes a file or directory recursively (removes all contents).

```xenith
remove_all("project/cache")  # Deletes entire directory tree
```

**Errors:** Permission denied

### `list_dir(path: string) -> list<string>`

Returns a list of all file and directory names in the given directory.

```xenith
spawn files: list<string> = list_dir(".")
for file in files {
    echo(file)
}
```

**Errors:** Directory not found, permission denied

### `copy(source: string, dest: string) -> null`

Copies a file from source to destination.

```xenith
copy("backup.txt", "restore.txt")
```

**Errors:** Source not found, permission denied

## Complete Example

```xenith
grab { 
    read, write, append, exists, is_file, is_dir,
    mkdir, mkdir_all, remove, remove_all, list_dir, copy
} from "std::fs"

# Create a directory for our data
mkdir_all("app/data")

# Write some data
write("app/data/users.txt", "Alice\nBob\nCharlie\n")

# Append more data
append("app/data/users.txt", "Diana\n")

# Read and process
spawn content: string = read("app/data/users.txt")
echo("Users:\n{content}")

# Check file type
echo("Is file? {is_file("app/data/users.txt")}")   # true
echo("Is dir? {is_dir("app/data")}")                # true

# List directory contents
spawn items: list<string> = list_dir("app/data")
for item in items {
    echo("  {item}")
}

# Backup the file
copy("app/data/users.txt", "app/data/users_backup.txt")

# Clean up
remove("app/data/users_backup.txt")
remove_all("app")
```

## Error Handling

All functions can fail. Use `try-catch` to handle errors gracefully:

```xenith
try {
    spawn content: string = read("missing.txt")
} catch err {
    echo("Error reading file: {err}")
}

try {
    mkdir("readonly/dir")
} catch err {
    echo("Failed to create directory: {err}")
}
```

## Performance Notes

- All operations are synchronous and block the current thread
- For large files, consider reading line by line (coming soon)
- `remove_all` can be slow on large directories

## See Also

- `std::path` - Path manipulation utilities
- `std::process` - Running external commands
```
