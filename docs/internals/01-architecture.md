# Architecture

Xenith is a tree walking interpreter written in Rust. Source text becomes tokens,
tokens become a tree, and the tree is executed by walking it. There is no
bytecode and no compilation step.

```
source text
    |
    v
  Lexer            src/lexer.rs        characters  ->  tokens
    |
    v
  Parser           src/parser.rs       tokens      ->  AST
    |
    v
  Checker          src/checker.rs      AST         ->  type errors
    |
    v
  Interpreter      src/interpreter.rs  AST         ->  values and effects
```

The checker never changes the tree and never runs anything. When it finds errors
the program does not start.

The whole pipeline is about 13,900 lines across 25 files, with seven
dependencies.

## Entry points

`src/lib.rs` exposes `run(filename, source)`, which runs the three stages in
order and returns either the program's final value or an `Error`. Everything else
in the project goes through it.

There are two binaries:

- `src/main.rs` builds `xenith`. It reads a file and calls `run`, or starts the
  REPL when given no arguments.
- `src/bin/xenith-lsp.rs` builds `xenith-lsp`, the language server. It calls the
  lexer, the parser and the checker, but never the interpreter, since analysing a
  file must not run it.

`main.rs` does one unusual thing: it runs everything on a dedicated thread with a
256 MB stack.

```rust
const INTERPRETER_STACK_SIZE: usize = 256 * 1024 * 1024;
```

The interpreter recurses on the Rust stack as it walks the tree, so a Xenith
program's recursion depth is bounded by the host stack. The default 8 MB main
thread runs out at roughly 1,200 Xenith calls, which is well under the 10,000
call limit the interpreter enforces. Without the big stack the process would
abort with a stack overflow before that limit could produce a diagnostic.

## The modules

| File | Lines | Responsibility |
| --- | --- | --- |
| `parser.rs` | 5192 | Recursive descent parser, all of the grammar |
| `interpreter.rs` | 2211 | Tree walking evaluator |
| `values.rs` | 1218 | Runtime values, arithmetic, built in functions |
| `bin/xenith-lsp.rs` | 917 | Language server |
| `lexer.rs` | 773 | Characters to tokens |
| `error.rs` | 643 | Error types and diagnostic rendering |
| `nodes.rs` | 541 | AST node definitions |
| `repl.rs` | 533 | Interactive shell |
| `utils.rs` | 241 | Character classification, value formatting |
| `symbol_table.rs` | 206 | Scoped name to value mapping |
| `builtins/registry.rs` | 180 | The list of builtins, shared with the LSP |
| `modules.rs` | 179 | Module loading and caching |
| `tokens.rs` | 149 | Token kinds and the keyword list |
| `types.rs` | 147 | The `Type` enum |
| `position.rs` | 110 | Source positions |
| `parse_result.rs` | 103 | Parser result and error propagation |
| `runtime_result.rs` | 95 | Interpreter result and control flow signals |
| `builtins/format.rs` | 86 | The `format` builtin |
| `context.rs` | 78 | Execution context, the scope chain |
| `main.rs` | 70 | The `xenith` binary |
| `fxhash.rs` | 64 | Fast non cryptographic hasher |

## Data flow in one example

Given `let x: int = 1 + 2`:

**Lexer** produces
`Keyword(let)`, `Identifier(x)`, `Colon`, `TypeInt`, `Eq`, `Int(1)`, `Plus`,
`Int(2)`, `Newline`, `Eof`.

**Parser** produces

```
VarAssign {
    variable_name_token: x,
    var_type: Some(Int),
    is_declaration: true,
    value_node: BinaryOperator {
        left:  Number(1),
        op:    Plus,
        right: Number(2),
    },
}
```

**Interpreter** visits the `VarAssign`, which visits the `BinaryOperator`, which
visits both `Number` nodes, adds them with a checked `i64` add, verifies the
result matches the declared `int`, and stores it in the current scope's symbol
table.

## Design decisions worth knowing

**Values are copied, not shared.** Assigning a struct or a list copies it.
Mutating methods such as `.append()` therefore have to write the changed value
back to the variable it came from. See [Values](07-values.md).

**Names are resolved lexically.** A method captures the context it was defined
in and runs against a child of that, which is what gives the language closures.
Call depth is threaded separately from the caller, because the recursion guard
counts calls rather than lexical nesting. See
[The interpreter](05-interpreter.md).

**Everything carries a source position.** Every token and every AST node has a
start and an end position, so any error can point at the code that caused it.
Positions share their file name and text through `Arc`, because copying them was
once the single largest cost in the interpreter. See
[Performance](10-performance.md).

**Checking happens twice, on purpose.** `src/checker.rs` reports what it can
prove before the program starts, and the interpreter re-checks as it runs. The
static pass is conservative and gives up on anything it cannot type, so the
runtime checks are what make the guarantee complete. See
[The static checker](06-checker.md).

Next: [The lexer](02-lexer.md)
