# Errors and Diagnostics

`src/error.rs`, 643 lines. One error type, a set of constructors for the common
cases, and the renderer.

## The type

```rust
pub struct Error {
    pub code: String,
    pub position_start: Position,
    pub position_end: Position,
    pub error_name: String,
    pub details: String,
    pub note: Option<String>,
    pub help: Option<String>,
    pub cause: Option<Box<Error>>,
}
```

Built with `new` and refined with a builder chain:

```rust
Error::new(pos_start, pos_end, "Integer Overflow", "integer overflow in addition")
    .with_code("XEN017")
    .with_note("int arithmetic is checked")
    .with_help("use a float if the value needs to be this large")
```

`details` is the one line under the heading. `note` explains the rule, `help`
suggests a fix. Both are optional and both are worth writing; a diagnostic that
only says what is wrong makes the reader work out what to do.

## The wrapper types

Four thin wrappers exist to set the code and the standard note and help for a
category:

| Type | Code | For |
| --- | --- | --- |
| `IllegalCharError` | XEN100 | a character the lexer cannot use |
| `ExpectedCharError` | XEN101 | something left unclosed |
| `InvalidSyntaxError` | XEN102 | a construct in the wrong shape |
| `RuntimeError` | XEN200 | anything at runtime with no better code |

Each holds a `.base: Error`, so a call site ends in `.base` to get the error out.
`RuntimeError` also carries an optional `Context` for a traceback.

## The constructors

`Error` has helpers for the errors that come up often, each setting its own code,
note and help:

```rust
Error::type_mismatch(expected, found, pos_start, pos_end)     // XEN001
Error::undefined_variable(name, pos_start, pos_end)           // XEN002
Error::division_by_zero(pos_start, pos_end)                   // XEN003
Error::index_out_of_bounds(index, len, pos_start, pos_end)    // XEN004
Error::field_not_found(struct_name, field, pos_start, pos_end)// XEN009
Error::module_not_found(name, pos_start, pos_end)             // XEN012
Error::unexpected_token(found, expected, pos_start, pos_end)  // XEN013
Error::too_many_arguments(expected, got, pos_start, pos_end)  // XEN015
Error::too_few_arguments(expected, got, pos_start, pos_end)   // XEN016
```

Prefer one of these over a bare `RuntimeError`. They give the user a code they
can look up and a help line, and they keep the wording consistent.

Watch the argument order on `field_not_found`: it is struct name first, then
field name. Getting it backwards produces "field `Point` not found for struct
`z`", which reads as nonsense.

## Codes

The full table is in the [tutorial](../tutorial/16-errors.md). The ranges:

| Range | Meaning |
| --- | --- |
| XEN001 to XEN020 | type, value and semantic errors |
| XEN100 to XEN102 | lexing and parsing |
| XEN200 | uncategorised runtime |
| XEN300 | panic |

Codes are a promise to users, so do not reuse one for a different meaning.
XEN100 was briefly both "Illegal Character" and "Destructuring Mismatch", which
is why the latter now has XEN020.

Some constructors are unreachable today because the code that raised them was
removed with the standard library: `file_not_found` (XEN005), `invalid_json`
(XEN006), `env_not_found` (XEN007), `method_not_found` (XEN008),
`permission_denied` (XEN010) and `missing_return` (XEN014). They are kept for
when the features return.

## Rendering

`as_string_colored` produces what the user sees:

```
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
error XEN001: Type Mismatch
  expected `int`, found `string`
  → program.xen:3:14

     3 │ let count: int = "many"
         ^^^^^^^^^^^^^^^^^^^^^^^

  = note: the declared type and the value disagree
  = help: use type conversion: `value as int`
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

The source line is pulled out of `position_start.file_text`, which is why every
position carries the whole source. `as_string` is the same without colour.

Two things here were once broken and are easy to break again:

- `details` was rendered nowhere, so every error showed only its category.
- The caret row double counted its indentation. `get_arrow` returns only the
  carets, and the caller supplies the leading spaces. Keep that split.

## Positions

`src/position.rs`:

```rust
pub struct Position {
    pub index: usize,
    pub line: usize,
    pub column: usize,
    pub file_name: Arc<str>,
    pub file_text: Arc<str>,
}
```

Line and column are zero based, which happens to match LSP. `index` and `column`
count characters, not bytes, because the lexer walks with `chars()`. The language
server converts to UTF-16 offsets before sending anything.

`file_name` and `file_text` are `Arc<str>`, not `String`. Every AST node holds
two positions and they are cloned constantly; owning the text meant each clone
copied the entire source file. `Arc` rather than `Rc` because the language server
shares parsed trees across threads.

Use `Position::with_source` in hot paths, which shares the existing handles and
never allocates. `Position::dummy()` is for internally generated nodes with no
source.

## Attaching a position after the fact

Arithmetic in `values.rs` does not know where it is being evaluated, so it raises
errors with dummy positions. `visit_binary_op` fills them in:

```rust
Err(mut e) => {
    if e.position_start.index == 0 && e.position_end.index == 0 {
        e.position_start = node.position_start.clone();
        e.position_end = node.position_end.clone();
    }
    RuntimeResult::new().failure(e)
}
```

If you raise an error somewhere without position information, check that it
passes through a visitor that does this, or it will point at line 1.

Next: [Modules](08-modules.md)
