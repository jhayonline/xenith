# The Parser

`src/parser.rs`, 5192 lines, the largest file in the project. A hand written
recursive descent parser producing a `Node` tree.

## Interface

```rust
let mut parser = Parser::new(tokens);
let result = parser.parse();

if let Some(error) = result.error { /* syntax error */ }
if let Some(node) = result.node { /* the tree */ }
```

`ParseResult` in `src/parse_result.rs` carries an optional node, an optional
error, and counters used for backtracking:

```rust
pub struct ParseResult {
    pub error: Option<Error>,
    pub node: Option<Node>,
    pub parsed_type: Option<Type>,
    pub last_registered_advance_count: usize,
    pub advance_count: usize,
    pub to_reverse_count: usize,
}
```

`register` folds a sub result into the current one and propagates errors, the
same shape as `RuntimeResult` in the interpreter.

## Backtracking

The parser saves `self.token_index`, tries something, and restores the index if
it did not work out. `expr()` does this several times over: it tries a field
assignment, then an indexed assignment, then a declaration, then a reassignment,
before falling through to an ordinary expression.

```rust
let start_index = self.token_index;
// try something
self.token_index = start_index;   // put it back
```

It works and it is easy to follow, but it means some constructs are parsed more
than once. If parse time ever matters, this is where to look first.

## The expression grammar

Precedence is encoded as a chain of functions, loosest first. Each level calls
the next tighter one and loops on its own operators.

```
expression   ->  ternary_expr
ternary_expr ->  comp_expr ( "?" expression ":" expression )?
comp_expr    ->  or_expr
or_expr      ->  and_expr  ( "||" and_expr )*
and_expr     ->  rel_expr  ( "&&" rel_expr )*
rel_expr     ->  arith_expr ( ("=="|"!="|"<"|">"|"<="|">=") arith_expr )*
arith_expr   ->  term ( ("+"|"-") term )*
term         ->  cast_expr ( ("*"|"/"|"%") cast_expr )*
cast_expr    ->  factor ( "as" TYPE )*
factor       ->  ("+"|"-"|"!") factor | power
power        ->  call ( "^" factor )*
call         ->  atom ( "(" args ")" | "[" index "]" | "." name )*
atom         ->  literal | identifier | "(" expression ")" | list | map | ...
```

`or_expr`, `and_expr`, `rel_expr` and `cast_expr` used to be a single level.
That made `a > 1 && b < 2` parse as `((a > 1) && b) < 2`, and put `as` below
multiplication so `n as float / 2.0` never reached the division. If you add an
operator, add it at the right level rather than to an existing one.

`call` is where postfix forms are handled. Indexing builds a `BinaryOperator`
node with a synthetic `TokenType::Index` operator, so `xs[0]` and `xs + 1` have
the same node shape and the interpreter tells them apart by operator kind.

## Statements

`statements()` parses a newline separated sequence into a `Node::List`. It is
used for the whole program and for every block body.

`block()` expects `{`, parses statements, expects `}`. Because it returns a
`Node::List`, a block and a program are the same kind of node, which is why
`visit` needs no special case for them.

## Line breaks

`is_line_break` treats `Newline` and `Semicolon` the same:

```rust
fn is_line_break(tok: &Token) -> bool {
    matches!(tok.kind, TokenType::Newline | TokenType::Semicolon)
}
```

`skip_line_breaks()` consumes a run of them. Call it inside any bracketed
construct, where a newline is layout rather than a statement end. List and map
literals do this, which is what lets them span several lines.

## Assignment forms

Four shapes are recognised, all producing a `BinaryOperator` with an `Eq`
operator, distinguished by the shape of the left node:

| Source | Left node |
| --- | --- |
| `x = v` | `VarAccess` |
| `p.field = v` | `MethodAccess` |
| `xs[i] = v` | `BinaryOperator` with `Index` |
| `let x = v` | a `VarAssign` node instead |

`VarAssignNode` carries `is_declaration` to tell `let x = v` from `x = v`. The
interpreter needs that distinction: a declaration writes to the current scope, an
assignment finds the scope that declared the name. Without the flag, assigning
inside a block silently created a shadow copy.

## Where the grammar lives

The file is long but flat. Finding a construct is usually a matter of searching
for its keyword:

| Construct | Function |
| --- | --- |
| `when` / `or when` / `otherwise` | `if_expr` |
| `for (;;)` | `for_classic` |
| `for x in xs` | `for_in` |
| `while` | `while_expr` |
| `method` | `func_def` |
| `struct` | `struct_definition`, `struct_instantiation` |
| `type` | `type_alias` |
| `grab` | `grab_statement` |
| `export` | `export_statement`, `exported_func_def` |
| `panic` | `panic_expr` |
| `[...]` | `list_expr` |
| `{...}` | `map_expr` |
| `(a, b)` | `tuple_literal` |
| `let (a, b) = ...` | `parse_destructure_pattern` |
| `let` | `var_declaration` |
| `x = v` | `var_reassignment` |
| `x += v`, `x++` | `compound_assignment`, `increment_decrement` |

## Type annotations

`parse_type` reads a type after a colon or an arrow, handling `list<T>`,
`map<K, V>`, tuple types and `method(A) -> B`. It also resolves any alias
declared so far, which is why the parser keeps a `type_aliases` map and the
interpreter hands its own copy over before parsing.

## Error reporting

Errors are `InvalidSyntaxError` or the `Error::unexpected_token` helper, both
carrying the position of the offending token. The parser stops at the first one;
there is no error recovery, so a file with two syntax errors reports the first.

For the language server, that means an incomplete file often yields one
diagnostic rather than several. Adding recovery, most usefully by skipping to the
next statement boundary, would improve that.

Next: [The AST](04-ast.md)
