# Native HTTP Router Design

## Intent

Provide a small, type-safe routing layer for Xenith backend programs on top of
the existing native socket ABI. The first slice should be usable without new
language keywords, MVC requirements, OOP, or a garbage collector.

## Goals

- Construct a router through ordinary native functions.
- Register `GET` and `POST` routes with literal paths and `:parameter` segments.
- Dispatch a parsed method/path/body request to a named Xenith handler.
- Return a simple response status/body pair.
- Keep socket ownership and allocation inside the native runtime.
- Report malformed routes and missing matches as recoverable HTTP errors.

## Non-goals

- Middleware, extractors, JSON serialization, async scheduling, or framework-
  specific controller syntax in this slice.
- Adding parser keywords or forcing MVC.
- Exposing raw file descriptors to ordinary application code beyond the existing
  low-level `std::http` API.

## API proposal

The standard module adds typed functions:

```xenith
let router: i64 = http.router()
http.get(router, "/users/:id", show_user)
http.post(router, "/users", create_user)
let response: tuple<i32, string> = http.dispatch(router, "GET", "/users/42", "")
```

The initial handler ABI is a function value accepting `(string, string, string)`
for method, path, and body and returning `tuple<i32, string>`. Dispatch returns
the handler response or `(404, "route not found")`; route registration returns a
string error, empty on success.

## Architecture

Native runtime owns a heap-backed router table containing route method/pattern
strings and function-value handles. The compiler lowers the standard functions
to runtime calls. Matching splits both patterns and request paths by `/`, binds
`:name` segments positionally for future extractor support, and selects the
first registered method/pattern match. Xenith code remains responsible for
writing a response through the existing socket API during this first slice.

## Safety and errors

Invalid route patterns and allocation failures use the existing recoverable
status/error ABI. Dispatch never dereferences a missing handler; it returns a
typed 404 response tuple. Router handles are opaque `i64` values and are closed
explicitly with `http.router_close`.
