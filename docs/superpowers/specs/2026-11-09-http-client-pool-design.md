# HTTP Client Pool Design

## Intent

Make `std::http.client_pool` a real bounded keep-alive pool for backend
clients. Requests should reuse idle TCP connections for the same endpoint,
support multiple endpoints, and recover transparently when a peer closes or a
connection fails.

## Goals

- Keep up to `max_connections` reusable connections in the pool.
- Key connections by the exact endpoint address supplied to `pool_request`.
- Serialize use of each socket while allowing different sockets to serve
  concurrent requests.
- Evict broken, expired, or `Connection: close` sockets.
- Reconnect on the next request after eviction.
- Expose an explicit `pool_close` operation that closes sockets and releases
  all pool memory.
- Preserve the existing recoverable tuple API:
  `(status, response, error)`.

## Non-goals

- HTTP/2 multiplexing.
- TLS pooling; secure requests remain on the existing HTTPS path.
- Request cancellation or asynchronous Xenith syntax.

## Approaches

1. A single socket per pool: simplest, but `max_connections` cannot provide
   useful concurrency and multiple hosts continually replace one another.
2. Per-endpoint serialized slots: supports several hosts, but requires a
   fixed endpoint table and still cannot make full use of the configured
   connection bound.
3. A bounded idle/active connection list keyed by endpoint: supports reuse,
   multiple hosts, and bounded concurrency at the cost of a small amount of
   synchronization and bookkeeping. This is the selected approach.

## Design

The native pool owns a mutex and a dynamically allocated array of connection
records. Each record stores the endpoint bytes, socket, in-use flag, and a
per-connection mutex. A request finds an idle matching record or creates a
new record if the bound allows it. It locks the record for the complete
HTTP/1.1 exchange, then marks it idle on success. Failures remove and close
the record before returning a recoverable error.

`pool_close` marks the pool closed, closes every socket, frees endpoint keys
and records, destroys mutexes, and releases the pool itself. Requests racing
with close return a recoverable closed-pool error.

## Verification

Native tests will cover same-endpoint reuse, two endpoints using separate
sockets, capacity enforcement, reconnect after `Connection: close`, and
explicit cleanup. The complete Cargo test suite must remain green.
