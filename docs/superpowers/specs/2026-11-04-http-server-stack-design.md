# HTTP Server Stack Design

## Intent

Turn the existing native HTTP socket and router primitives into a usable Xenith
backend foundation while keeping ordinary Xenith syntax, static types, and
explicit application control.

## API

The first server API is `serve(router, address) -> string`. It owns the blocking
accept/read/dispatch/write loop and returns an empty string on clean completion
or a recoverable error message. Handlers continue to receive method, path, and
body and return `(status, body)`.

The request API adds `headers(request)`, `query(request, key)`, and
`param(request, name)` as typed helpers once request handles are introduced.
JSON helpers are `json(status, body)` and `json_body(request)`; the initial JSON
boundary is UTF-8 text so the compiler does not need reflection or generics.

Shutdown is explicit through a server handle and `shutdown(server)`. Settings
remain data passed to `serve` rather than new syntax.

## Constraints

- No new language keywords, MVC requirement, garbage collector, or Rust-style
  ownership model.
- Native runtime owns sockets and allocations.
- Failures use the existing status/error ABI where the current wrapper ABI can
  preserve them; wrapper propagation is upgraded before exposing new fallible
  constructors.
- Blocking serving is the first implementation; async scheduling is later.
