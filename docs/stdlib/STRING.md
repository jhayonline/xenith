# std::string - String Utilities

## Introduction

The `std::string` module provides string manipulation functions including splitting, joining, trimming, replacing, case conversion, and more.

## Importing

```xenith
grab { split, join, trim, trim_start, trim_end, replace, contains, starts_with, ends_with, to_upper, to_lower, reverse } from "std::string"
```

## Functions

### Splitting & Joining

#### `split(text: string, delimiter: string) -> list<string>`

Splits a string into a list of substrings using the specified delimiter.

```xenith
spawn parts: list<string> = split("a,b,c", ",")
# parts = ["a", "b", "c"]

# Empty delimiter splits into characters
spawn chars: list<string> = split("hello", "")
# chars = ["h", "e", "l", "l", "o"]
```

#### `join(strings: list<string>, separator: string) -> string`

Joins a list of strings into a single string with the specified separator.

```xenith
spawn fruits: list<string> = ["apple", "banana", "orange"]
spawn result: string = join(fruits, ", ")
echo(result)  # "apple, banana, orange"

spawn path: list<string> = ["home", "user", "docs"]
spawn full_path: string = join(path, "/")
echo(full_path)  # "home/user/docs"
```

### Trimming

#### `trim(text: string) -> string`

Removes leading and trailing whitespace.

```xenith
spawn cleaned: string = trim("  hello  ")
echo(cleaned)  # "hello"
```

#### `trim_start(text: string) -> string`

Removes leading whitespace only.

```xenith
spawn cleaned: string = trim_start("  hello  ")
echo(cleaned)  # "hello  "
```

#### `trim_end(text: string) -> string`

Removes trailing whitespace only.

```xenith
spawn cleaned: string = trim_end("  hello  ")
echo(cleaned)  # "  hello"
```

### Replacement

#### `replace(text: string, old: string, new: string) -> string`

Replaces all occurrences of a substring with another string.

```xenith
spawn result: string = replace("hello world", "world", "xenith")
echo(result)  # "hello xenith"

spawn result: string = replace("foo bar foo", "foo", "baz")
echo(result)  # "baz bar baz"
```

### Searching

#### `contains(text: string, substring: string) -> bool`

Returns `true` if the substring is found within the text.

```xenith
when contains("hello world", "world") {
    echo("Found!")
}
```

#### `starts_with(text: string, prefix: string) -> bool`

Returns `true` if the text starts with the specified prefix.

```xenith
when starts_with("filename.txt", "file") {
    echo("It's a file!")
}
```

#### `ends_with(text: string, suffix: string) -> bool`

Returns `true` if the text ends with the specified suffix.

```xenith
when ends_with("document.pdf", ".pdf") {
    echo("PDF document")
}
```

### Case Conversion

#### `to_upper(text: string) -> string`

Converts all characters to uppercase.

```xenith
spawn upper: string = to_upper("Hello World")
echo(upper)  # "HELLO WORLD"
```

#### `to_lower(text: string) -> string`

Converts all characters to lowercase.

```xenith
spawn lower: string = to_lower("Hello World")
echo(lower)  # "hello world"
```

### Reversal

#### `reverse(text: string) -> string`

Reverses the string.

```xenith
spawn reversed: string = reverse("hello")
echo(reversed)  # "olleh"

spawn palindrome: bool = reverse("racecar") == "racecar"
echo(palindrome)  # true
```

## Complete Examples

### CSV Parser

```xenith
grab { split, join, trim } from "std::string"

method parse_csv_line(line: string) -> list<string> {
    release split(line, ",")
}

method parse_csv(file_path: string) -> list<list<string>> {
    spawn content: string = read(file_path)
    spawn lines: list<string> = split(content, "\n")
    spawn result: list<list<string>> = []
    
    for line in lines {
        when trim(line) != "" {
            result.append(parse_csv_line(line))
        }
    }
    release result
}

# Example usage
spawn csv_data: list<list<string>> = parse_csv("data.csv")
for row in csv_data {
    echo(join(row, " | "))
}
```

### String Validator

```xenith
grab { starts_with, ends_with, contains, to_lower } from "std::string"

method is_valid_email(email: string) -> bool {
    release contains(email, "@") && contains(email, ".")
}

method is_valid_url(url: string) -> bool {
    release starts_with(url, "http://") || starts_with(url, "https://")
}

method is_image_file(filename: string) -> bool {
    spawn lower: string = to_lower(filename)
    release ends_with(lower, ".jpg") || ends_with(lower, ".png") || ends_with(lower, ".gif")
}

# Test
echo(is_valid_email("user@example.com"))   # true
echo(is_valid_email("invalid"))             # false
echo(is_valid_url("https://xenith.dev"))    # true
echo(is_image_file("photo.jpg"))            # true
```

### Text Processor

```xenith
grab { split, join, replace, to_upper, to_lower, trim } from "std::string"

method slugify(text: string) -> string {
    spawn lower: string = to_lower(text)
    spawn trimmed: string = trim(lower)
    spawn words: list<string> = split(trimmed, " ")
    release join(words, "-")
}

method word_count(text: string) -> int {
    spawn trimmed: string = trim(text)
    when trimmed == "" {
        release 0
    }
    spawn words: list<string> = split(trimmed, " ")
    release words.len()
}

method censored(text: string, bad_words: list<string>) -> string {
    spawn result: string = text
    for word in bad_words {
        result = replace(result, word, "****")
    }
    release result
}

# Example
spawn title: string = "  Hello World from Xenith  "
echo(slugify(title))  # "hello-world-from-xenith"

spawn message: string = "This is a bad word example"
spawn bad: list<string> = ["bad"]
echo(censored(message, bad))  # "This is a **** word example"
```

### Name Formatter

```xenith
grab { trim, to_upper, to_lower, split, join } from "std::string"

method capitalize(name: string) -> string {
    when name.len() == 0 {
        release ""
    }
    spawn first: string = to_upper(name[0] as string)
    spawn rest: string = to_lower(name[1..name.len()] as string)
    release first + rest
}

method format_name(first: string, last: string) -> string {
    release capitalize(trim(first)) + " " + capitalize(trim(last))
}

method initials(name: string) -> string {
    spawn parts: list<string> = split(trim(name), " ")
    spawn result: list<string> = []
    for part in parts {
        result.append(to_upper(part[0] as string))
    }
    release join(result, ".")
}

# Example
spawn full: string = format_name("  john  ", "  DOE  ")
echo(full)  # "John Doe"

spawn init: string = initials("John Michael Doe")
echo(init)  # "J.M.D"
```

### Log File Analyzer

```xenith
grab { contains, split, trim, starts_with } from "std::string"

method analyze_log(log_content: string) -> null {
    spawn lines: list<string> = split(log_content, "\n")
    spawn error_count: int = 0
    spawn warning_count: int = 0
    
    for line in lines {
        spawn trimmed: string = trim(line)
        when trimmed == "" {
            skip
        }
        
        when contains(trimmed, "ERROR") {
            error_count = error_count + 1
            echo("ERROR: {trimmed}")
        } or when contains(trimmed, "WARNING") {
            warning_count = warning_count + 1
        }
        
        when starts_with(trimmed, "FATAL") {
            echo("FATAL: {trimmed}")
        }
    }
    
    echo("\nSummary:")
    echo("  Errors: {error_count}")
    echo("  Warnings: {warning_count}")
    release null
}

# Example log content
spawn log_data: string = "
INFO: Application started
WARNING: Low memory
ERROR: Connection failed
FATAL: Database corrupted
WARNING: Retry attempt 1
"

analyze_log(log_data)
```

### Template Engine

```xenith
grab { replace, contains } from "std::string"

method render_template(template: string, variables: map<string, string>) -> string {
    spawn result: string = template
    
    for key, value in variables.items() {
        result = replace(result, "{{" + key + "}}", value)
    }
    
    release result
}

# Example
spawn template: string = "Hello {{name}}, you are {{age}} years old!"
spawn data: map<string, string> = {
    "name": "Alice",
    "age": "25"
}

spawn output: string = render_template(template, data)
echo(output)  # "Hello Alice, you are 25 years old!"
```

### Password Validator

```xenith
grab { contains, to_lower, to_upper } from "std::string"

method is_strong_password(password: string) -> bool {
    when password.len() < 8 {
        release false
    }
    
    spawn has_upper: bool = false
    spawn has_lower: bool = false
    spawn has_digit: bool = false
    
    for i = 0 to password.len() {
        spawn ch: string = password[i] as string
        when contains("ABCDEFGHIJKLMNOPQRSTUVWXYZ", ch) {
            has_upper = true
        } or when contains("abcdefghijklmnopqrstuvwxyz", ch) {
            has_lower = true
        } or when contains("0123456789", ch) {
            has_digit = true
        }
    }
    
    release has_upper && has_lower && has_digit
}

echo(is_strong_password("Password123"))   # true
echo(is_strong_password("weak"))          # false
echo(is_strong_password("NoDigits"))      # false
```

## Performance Notes

- `split()` and `replace()` create new strings (O(n) time complexity)
- `join()` is efficient for building strings from lists
- `reverse()` creates a new string (O(n) time and space)
- Case conversion handles Unicode characters properly

## See Also

- Built-in string interpolation: `"Hello {name}"`
- Built-in string concatenation: `"Hello " + name`
- Built-in string repetition: `"-" * 10`
- `std::fs` for reading/writing text files
```
