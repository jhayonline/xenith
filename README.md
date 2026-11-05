# Xenith

Xenith is a type-safe, native-compiled programming language for building real
applications. Its main goals are practical backend and server programming,
systems programming, game development, desktop applications, and general
purpose software.

Xenith is designed to feel approachable to beginners while still giving
experienced developers direct access to native execution and platform APIs.

## What Xenith is for

- Backend and server applications, in the spirit of PHP and Go, with static
  types and compiler-checked errors.
- Systems software that needs native executables, predictable behavior, and FFI
  access.
- Games with fast update loops, native graphics/input/audio integrations, and
  control over performance-sensitive code.
- Desktop applications with XML-based UI structure and native platform
  backends.
- General-purpose command-line tools, automation, data processing, and scripts
  that can grow into compiled applications.

## Quick start

Create and build a native project:

```sh
xenith new hello
cd hello
xenith check
xenith build
xenith run
```

Every executable project has a `xenith.toml` manifest and a `src/main.xen`
entry point. The compiler emits a host-native executable under
`target/host/debug/`.

## Native HTTP routing

The native HTTP module provides a small router without requiring MVC, classes,
or framework-specific keywords. Handlers are ordinary Xenith functions with a
typed `(method, path, body)` signature and a `(status, body)` response:

```xenith
grab { router, get, post, dispatch, router_close } from "std::http"

function health(method: string, path: string, body: string) -> tuple<i32, string> {
    return (200, "ok")
}

function main() -> i32 {
    let routes: i64 = router()
    get(routes, "/health", health)

    let response: tuple<i32, string> = dispatch(routes, "GET", "/health", "")
    router_close(routes)
    return response[0]
}
```

Routes may contain `:parameter` segments, and `GET`/`POST` method mismatches
return a typed `404` response. Router middleware uses the same ordinary
function type: return status `0` to continue, or a real HTTP status to
short-circuit the request.

For handlers that need structured request data, use the request-context routes.
They receive an opaque, read-only request handle and can extract the method,
full path, body, headers, query values, and named route parameters without
manually parsing HTTP:

```xenith
grab { router, get_request, dispatch_request, request_query, request_param,
       router_close, json } from "std::http"

function show_user(request: i64) -> tuple<i32, string> {
    let id: string = request_param(request, "/users/:id", "id")
    let page: string = request_query(request, "page")
    return json(200, "user {id}, page {page}")
}

function main() -> i32 {
    let routes: i64 = router()
    get_request(routes, "/users/:id", show_user)
    let response: tuple<i32, string> = dispatch_request(routes, "GET /users/42?page=1 HTTP/1.1")
    router_close(routes)
    return response[0]
}
```

The same request-context handlers are used by the blocking server APIs, so a
route can move from tests to production without changing its signature.

For responses that need headers, use a response handle. `response_new` creates
one, `response_header` adds a header, and the accessors read it back. Response
routes return the handle directly, while `use_request` middleware may return
`0` to continue or a response handle to short-circuit:

```xenith
grab { router, get_response, use_request, dispatch_response,
       response_new, response_header, response_status, response_close,
       router_close } from "std::http"

function auth(request: i64) -> i64 { return 0i64 }

function health(request: i64) -> i64 {
    let result: i64 = response_new(200, "ok")
    response_header(result, "Content-Type", "text/plain")
    return result
}
```

`dispatch_response` returns the owned handle; call `response_close` after
reading it. The existing tuple-based `get`, `post`, `use`, and `dispatch`
functions remain available for simple handlers. Response routes and
request-aware middleware also work through `serve`, `run`, `run_for`, and
`run_workers`; the server serializes their status and headers and closes the
owned response after writing it.

```xenith
function require_token(method: string, path: string, body: string) -> tuple<i32, string> {
    if (path == "/private") {
        return (401, "unauthorized")
    }
    return (0, "")
}
```

Register it with `use(routes, require_token)` before adding routes. A real
blocking server can own the socket loop:

```xenith
function server_main() -> i32 {
    let routes: i64 = router()
    get(routes, "/health", health)
    let error: string = serve(routes, "127.0.0.1:8080")
    return len(error)
}
```

`query(path, key)` extracts a query value, `header(request, name)` reads a
request header, and `param(path, pattern, name)` extracts a named route
parameter. `json(status, body)` constructs a typed response tuple. For request
handlers, `json_body(request)` returns `(body, error)` after checking the HTTP
header boundary and validating the complete JSON value; `json_validate(body)`
performs the same validation on an already extracted body, and
`json_escape(value)` produces a JSON string literal for safe responses.
`json_object(body)` decodes a JSON object into a typed `map<string, string>`;
each value is retained as its JSON text so applications can validate or
convert fields explicitly. JSON failures are ordinary strings so a handler can
choose whether to return a client error or catch the failure. The server uses
blocking I/O; the
controllable `server(address)`, `run(server, router)`, and `shutdown(server)`
APIs provide an explicit lifecycle for applications that need one. For bounded
production settings, `server_with_config(address, max_request_size,
read_timeout_ms, write_timeout_ms, backlog)` applies request-size limits,
socket timeouts, and the listener backlog while retaining the same handle and
`run`/`shutdown` lifecycle. `run_for(server, router, max_requests)` processes
a bounded number of requests and returns; passing `0` keeps processing until a
server error or shutdown. `run_workers(server, router, workers)` runs the same
blocking accept/dispatch loop across a fixed native worker pool for concurrent
request handling.

## The language in one screen

```xenith
struct Runner {
    name: string
    minutes: i32
}

function faster_of(a: Runner, b: Runner) -> Runner {
    if (a.minutes < b.minutes) {
        return a
    } else {
        return b
    }
}

function main() -> i32 {
    let ada: Runner = Runner { name: "Ada", minutes: 31 }
    let alan: Runner = Runner { name: "Alan", minutes: 28 }
    let winner: Runner = faster_of(ada, alan)

    echo("The faster runner is selected")
    return winner.minutes
}
```

Declarations use `function`. `fn` is reserved for function types and anonymous
callbacks:

```xenith
function apply(value: i32, operation: fn(i32) -> i32) -> i32 {
    return operation(value)
}

function main() -> i32 {
    let doubled = apply(21, fn(value: i32) -> i32 { return value * 2 })
    return doubled
}
```

Methods attach behavior to a type and use receiver syntax:

```xenith
struct User {
    name: string
}

method User.greeting() -> string {
    return "Hello, " + self.name
}
```

Conditions and loop conditions use parentheses:

```xenith
if (user.name == "Ada") {
    echo("matched")
}

for (let i: i32 = 0; i < 3; i = i + 1) {
    echo("iteration")
}
```

## Types and collections

Xenith has explicit integer widths, booleans, strings, bytes, structs, tuples,
enums, functions, lists, and maps. Collection literals are statically checked
for homogeneity and are heap-backed in native programs.

```xenith
function checksum() -> i32 {
    let mut values: list<i32> = [1, 2, 3]
    values[0] = 99

    let mut names: map<string, i32> = {"ada": 36}
    names["alan"] = 41

    let raw: bytes = bytes("hello")
    return values[0] + names["alan"] + raw[0]
}
```

## Error handling

Expected failures use `try`/`catch`. Xenith does not require Rust-style
`Option<T>` or `Result<T>` values for ordinary error handling.

```xenith
function load_config(path: string) -> string {
    try {
        let (contents, error) = fs_read(path)
        if (error != "") {
            panic(error)
        }
        return contents
    } catch (error) {
        echo(error.message)
        return ""
    }
}
```

Native diagnostics include an error code, name, message, source location, and
beginner-friendly help text. Allocation and filesystem failures use the shared
recoverable native status ABI.

## SQL-free database schemas

`std::db` keeps migration definitions in ordinary Xenith code. The schema
builder validates field names and types, generates the SQLite DDL natively, and
records migration names so applying the same migration again is a no-op:

```xenith
grab { open, schema, field, migrate, store, insert, close, schema_close } from "std::db"

let database: i64 = open("app.db")
let users: i64 = schema("users")
field(users, "name", "string")
field(users, "age", "i32")
migrate(database, "create_users", users)

let table: i64 = store(database, "users")
insert(table, {"name": "Ada", "age": "36"})
schema_close(users)
close(database)
```

Supported field kinds are `string`, `i32`, `i64`, `bool`, and `bytes`. Schema,
field, and migration failures return recoverable error strings; application code
never needs to embed SQL for schema creation.

## Standard library and imports

Native projects can import the filesystem and environment modules:

```xenith
grab { read, write } from "std::fs"
grab { cwd, args } from "std::env"

function main() -> i32 {
    let (directory, error) = cwd()
    if (error != "") {
        echo(error)
        return 1
    }
    echo(directory)
    return 0
}
```

The standard library is being expanded around the language’s target domains:
HTTP and networking, JSON and serialization, processes, databases, time,
randomness, graphics, audio, input, and desktop UI.

## Memory and safety model

Xenith is intended to be safe and easy to reason about without exposing manual
allocation to ordinary programs and without requiring Rust-style ownership
annotations.

The planned model is compiler-managed deterministic reference counting (ARC/ORC):

- No tracing garbage collector or GC pauses.
- No `malloc`/`free` in normal Xenith code.
- Automatic cleanup of heap values when they are no longer referenced.
- Value semantics for ordinary structs and primitive values.
- Weak references for cycles.
- Explicit `unsafe` boundaries for raw FFI and platform-level operations.

The native runtime currently contains the reference-counted allocation
foundation. Compiler-generated retain/release insertion is still being built
and must be completed before the memory model is considered finished.

## Current status

Implemented native compiler capabilities include:

- Native project creation, checking, building, and running.
- Static type checking and homogeneous typed collections.
- Lists, maps, bytes, literals, indexing, mutation, and iteration.
- Enums, pattern matching, closures, function values, methods, and string
  interpolation.
- Parenthesized `if` and `for` conditions.
- Filesystem and environment runtime APIs with recoverable failures.
- Native `std::fs` and `std::env` imports.
- Source-aware diagnostics with codes, hints, function names, files, lines,
  columns, and byte offsets.

The major work still ahead includes:

- Complete compiler-managed ARC/ORC insertion and cycle-safe weak references.
- TCP networking, HTTP servers, routing, middleware, and concurrency.
- JSON, database, TLS, process, and serialization libraries.
- Graphics, audio, input, and game-oriented runtime modules.
- XML parsing and desktop UI tooling.
- FFI safety, portability across operating systems, optimization modes, and
  release-build hardening.

## Installing

```sh
cargo install --path .
```

This installs the `xenith` compiler and the `xenith-lsp` language server. On
Arch Linux, the package definition is in `packaging/arch/PKGBUILD`.

## Editor support

`editors/nvim` provides Neovim filetype detection, syntax highlighting,
indenting, and language-server integration. With lazy.nvim:

```lua
{
  dir = "/path/to/xenith/editors/nvim",
  name = "xenith.nvim",
  lazy = false,
  config = function()
    require("xenith").setup()
  end,
}
```

Any LSP-capable editor can use `xenith-lsp`. See
[Editor setup](docs/tutorial/18-editor-setup.md).

## Documentation

- [Tutorial](docs/tutorial/README.md)
- [Known limitations](docs/tutorial/19-limitations.md)
- [Internals](docs/internals/README.md)

## Development

Run the complete test suite with:

```sh
cargo test --all
```

The compiler and runtime are developed test-first. Native integration tests
cover parsing, static checking, code generation, linking, runtime behavior,
recoverable errors, collections, filesystem access, environment access, and
standard-library imports.

## License

MIT. See [LICENSE](LICENSE).
