# std::http - HTTP Client

## Introduction

The `std::http` module provides a synchronous HTTP client for making requests to web servers. It supports GET, POST, PUT, DELETE, and PATCH methods with custom headers and timeouts.

## Importing

```xenith
grab { get, post, put, delete, patch, get_with_headers, post_with_headers, put_with_headers, delete_with_headers, patch_with_headers, set_timeout, set_user_agent } from "std::http"
```

## HttpResponse Struct

All HTTP methods return an `HttpResponse` struct with the following fields and methods:

### Fields

| Field | Type | Description |
|-------|------|-------------|
| `status` | `int` | HTTP status code (200, 404, 500, etc.) |
| `body` | `string` | Response body as string |
| `headers` | `map<string, string>` | Response headers |

### Methods

| Method | Return Type | Description |
|--------|-------------|-------------|
| `ok()` | `bool` | Returns `true` if status code is 2xx |
| `json()` | `json` | Parses response body as JSON |
| `text()` | `string` | Returns response body as string |
| `header(name)` | `string` | Gets a specific header value |

## Functions

### GET Requests

#### `get(url: string) -> HttpResponse`

Sends a GET request to the specified URL.

```xenith
spawn response: HttpResponse = get("https://api.example.com/users")
echo("Status: {response.status}")
echo("Body: {response.text()}")
```

#### `get_with_headers(url: string, headers: map<string, string>) -> HttpResponse`

Sends a GET request with custom headers.

```xenith
spawn headers: map<string, string> = {
    "Authorization": "Bearer token123",
    "Accept": "application/json"
}
spawn response: HttpResponse = get_with_headers("https://api.example.com/protected", headers)
```

### POST Requests

#### `post(url: string, body: string) -> HttpResponse`

Sends a POST request with a string body.

```xenith
spawn response: HttpResponse = post("https://api.example.com/users", '{"name":"Alice"}')
```

#### `post_with_headers(url: string, body: string, headers: map<string, string>) -> HttpResponse`

Sends a POST request with custom headers.

```xenith
spawn headers: map<string, string> = {
    "Content-Type": "application/json"
}
spawn response: HttpResponse = post_with_headers("https://api.example.com/users", '{"name":"Alice"}', headers)
```

### PUT Requests

#### `put(url: string, body: string) -> HttpResponse`

Sends a PUT request to update a resource.

```xenith
spawn response: HttpResponse = put("https://api.example.com/users/1", '{"name":"Alice Updated"}')
```

#### `put_with_headers(url: string, body: string, headers: map<string, string>) -> HttpResponse`

Sends a PUT request with custom headers.

### DELETE Requests

#### `delete(url: string) -> HttpResponse`

Sends a DELETE request to remove a resource.

```xenith
spawn response: HttpResponse = delete("https://api.example.com/users/1")
```

#### `delete_with_headers(url: string, headers: map<string, string>) -> HttpResponse`

Sends a DELETE request with custom headers.

### PATCH Requests

#### `patch(url: string, body: string) -> HttpResponse`

Sends a PATCH request to partially update a resource.

```xenith
spawn response: HttpResponse = patch("https://api.example.com/users/1", '{"name":"Alice"}')
```

#### `patch_with_headers(url: string, body: string, headers: map<string, string>) -> HttpResponse`

Sends a PATCH request with custom headers.

### Configuration

#### `set_timeout(seconds: int) -> null`

Sets the timeout for all subsequent HTTP requests (default is 30 seconds).

```xenith
set_timeout(10)  # 10 second timeout
```

#### `set_user_agent(agent: string) -> null`

Sets the User-Agent header for all subsequent requests.

```xenith
set_user_agent("MyApp/1.0")
```

## Complete Examples

### Fetching JSON Data

```xenith
grab { get } from "std::http"
grab { parse } from "std::json"

try {
    spawn response: HttpResponse = get("https://jsonplaceholder.typicode.com/posts/1")
    
    when response.ok() {
        spawn data: json = response.json()
        echo("Title: {data["title"]}")
        echo("Body: {data["body"]}")
    } otherwise {
        echo("HTTP Error: {response.status}")
    }
} catch err {
    echo("Network Error: {err}")
}
```

### Creating a Resource with POST

```xenith
grab { post_with_headers } from "std::http"
grab { parse, stringify } from "std::json"

spawn new_post: json = parse({
    "title": "My Post",
    "body": "This is my post content",
    "userId": 1
})

spawn headers: map<string, string> = {
    "Content-Type": "application/json"
}

try {
    spawn response: HttpResponse = post_with_headers(
        "https://jsonplaceholder.typicode.com/posts",
        stringify(new_post),
        headers
    )
    
    when response.ok() {
        spawn created: json = response.json()
        echo("Created post ID: {created["id"]}")
    } otherwise {
        echo("Failed to create post: {response.status}")
    }
} catch err {
    echo("Error: {err}")
}
```

### API Client with Authentication

```xenith
grab { get_with_headers, set_timeout, set_user_agent } from "std::http"
grab { parse } from "std::json"

# Configure client
set_timeout(15)
set_user_agent("MyApiClient/1.0")

method fetchUser(userId: int, apiToken: string) -> json {
    spawn url: string = "https://api.example.com/users/" + (userId as string)
    spawn headers: map<string, string> = {
        "Authorization": "Bearer " + apiToken,
        "Accept": "application/json"
    }
    
    try {
        spawn response: HttpResponse = get_with_headers(url, headers)
        
        when response.ok() {
            release response.json()
        } otherwise {
            panic "API returned status {response.status}"
        }
    } catch err {
        panic "Request failed: {err}"
    }
}

spawn user_data: json = fetchUser(123, "my-secret-token")
echo("User: {user_data["name"]}")
```

### Error Handling with Retry

```xenith
grab { get } from "std::http"

method fetchWithRetry(url: string, max_retries: int) -> HttpResponse {
    spawn attempts: int = 0
    
    while attempts < max_retries {
        try {
            release get(url)
        } catch err {
            attempts = attempts + 1
            echo("Attempt {attempts} failed: {err}")
            when attempts == max_retries {
                panic "All {max_retries} attempts failed"
            }
        }
    }
    
    panic "Unexpected error"
}

spawn response: HttpResponse = fetchWithRetry("https://api.example.com/data", 3)
echo("Success after retries!")
```

### Working with Query Parameters

```xenith
grab { get } from "std::http"

method buildURL(base: string, params: map<string, string>) -> string {
    spawn url: string = base + "?"
    spawn first: bool = true
    
    for key, value in params.items() {
        when !first {
            url = url + "&"
        }
        url = url + key + "=" + value
        first = false
    }
    
    release url
}

spawn params: map<string, string> = {
    "page": "1",
    "limit": "10",
    "sort": "desc"
}

spawn url: string = buildURL("https://api.example.com/users", params)
spawn response: HttpResponse = get(url)

when response.ok() {
    spawn data: json = response.json()
    echo("Got {data.len()} users")
}
```

### File Download

```xenith
grab { get } from "std::http"
grab { write } from "std::fs"

method downloadFile(url: string, outputPath: string) -> null {
    try {
        spawn response: HttpResponse = get(url)
        
        when response.ok() {
            write(outputPath, response.text())
            echo("Downloaded to {outputPath}")
        } otherwise {
            panic "HTTP {response.status}: Failed to download"
        }
    } catch err {
        panic "Download failed: {err}"
    }
    release null
}

downloadFile("https://example.com/file.txt", "downloaded.txt")
```

### Batch Requests

```xenith
grab { get } from "std::http"
grab { parse } from "std::json"

spawn urls: list<string> = [
    "https://jsonplaceholder.typicode.com/posts/1",
    "https://jsonplaceholder.typicode.com/posts/2",
    "https://jsonplaceholder.typicode.com/posts/3"
]

spawn results: list<json> = []

for url in urls {
    try {
        spawn response: HttpResponse = get(url)
        when response.ok() {
            results.append(response.json())
        }
    } catch err {
        echo("Failed to fetch {url}: {err}")
    }
}

echo("Fetched {results.len()} posts")
```

### Status Code Handling

```xenith
grab { get } from "std::http"

method handleResponse(response: HttpResponse) -> null {
    match response.status {
        200 => {
            echo("Success: {response.text()}")
        }
        201 => {
            echo("Created: {response.header("location")}")
        }
        400 => {
            echo("Bad Request: {response.text()}")
        }
        401 => {
            echo("Unauthorized - Check your API key")
        }
        403 => {
            echo("Forbidden - Insufficient permissions")
        }
        404 => {
            echo("Not Found - Resource doesn't exist")
        }
        429 => {
            echo("Too Many Requests - Rate limited")
        }
        500 => {
            echo("Server Error - Try again later")
        }
        _ => {
            echo("Unknown status: {response.status}")
        }
    }
    release null
}

spawn response: HttpResponse = get("https://api.example.com/data")
handleResponse(response)
```

## Error Handling

All HTTP methods can fail. Always use `try-catch`:

```xenith
try {
    spawn response: HttpResponse = get("https://api.example.com/data")
    # Process response...
} catch err {
    echo("Request failed: {err}")
}
```

Common errors:
- Network connection issues
- DNS resolution failures
- Timeout exceeded
- Invalid URL
- SSL/TLS errors

## Performance Notes

- All requests are synchronous (blocking)
- Default timeout is 30 seconds
- Connection pooling is enabled automatically
- Responses are buffered in memory

## See Also

- `std::json` - Parse JSON responses
- `std::fs` - Save downloaded files
- `std::time` - Measure request durations
```
