# The Language Server

`src/bin/xenith-lsp.rs`, 917 lines. A separate binary built from the same crate,
speaking LSP over stdio.

Built on `tower-lsp` with `tokio` for the runtime and `dashmap` for the document
store. Those three dependencies exist only for this binary.

## What it does not do

It never runs the interpreter. Analysing a file must not execute it, so the
server calls the lexer and parser and stops there. That is also why it reports no
type errors: type checking happens during execution, and there is no separate
checking pass to call.

## The document model

```rust
struct Document {
    text: String,
    lines: Vec<String>,
    symbols: Vec<Symbol>,
    references: Vec<Reference>,
}
```

`lines` exists for position conversion. Every open document is re-analysed from
scratch on each keystroke, which is fine at the file sizes involved.

```rust
struct Symbol {
    name: String,
    kind: SymbolKind,
    range: Range,             // the whole construct, for the outline
    selection_range: Range,   // just the identifier, for goto and rename
    detail: Option<String>,
    children: Vec<Symbol>,
}

struct Reference {
    name: String,
    range: Range,
}
```

Symbols are definitions and nest, so a method's parameters and locals sit under
it. References are mentions and stay flat.

## Position conversion

Xenith positions count characters. LSP positions count UTF-16 code units. They
agree only while a line is ASCII.

```rust
fn to_lsp(&self, pos: &XenithPosition) -> Position {
    let character = match self.lines.get(pos.line) {
        Some(line) => line.chars().take(pos.column)
            .map(|c| c.len_utf16() as u32).sum(),
        None => pos.column as u32,
    };
    Position { line: pos.line as u32, character }
}
```

Both are zero based, so lines pass through unchanged. Do not shortcut the column;
one non-ASCII character earlier in a line shifts every diagnostic on it.

## Analysis

```rust
fn analyze(uri: &str, doc: &mut Document) -> Vec<Diagnostic>
```

Lex, then parse, converting whatever error comes back into a `Diagnostic`. Both
stages stop at the first error, so a file usually yields one diagnostic rather
than a list. Parser error recovery would change that.

The tree is indexed even when parsing failed, since a partial tree still has
useful symbols in it and the file is nearly always mid-edit.

A zero width error range is widened by one character, otherwise the underline is
invisible.

## Walking the tree

`Collector::walk` recurses over every node, pushing definitions into a tree and
mentions into a flat list.

The version before this one matched on the node it was handed and never recursed.
Since the root of a program is a `Node::List`, it fell through to `_ => {}` and
the symbol list was always empty, which silently disabled hover, go to
definition, references, rename and completion of anything the user had written.
If you add a node type, add it to `walk`.

## Name resolution

Flat and by name:

```rust
fn find_definition(&self, name: &str) -> Option<&Symbol>
fn occurrences(&self, name: &str) -> Vec<Range>
```

Two variables called `i` in different methods are one symbol. Rename rewrites
every occurrence in the file. Imports are not followed.

Fixing that needs real scope information, which is the resolver pass the
interpreter also wants. Building it once and sharing it between the two is the
right shape.

## The word fallback

`word_at(position)` reads the identifier under the cursor straight out of the
buffer. Hover, go to definition and references fall back to it when the AST index
has nothing at that position, which covers keywords, builtins the parser handles
specially, and text inside a region that failed to parse.

Rename deliberately does not use the fallback: renaming from something the server
does not track would produce an incomplete edit.

## Sharing the builtin list

Completion and hover read `src/builtins/registry.rs`, the same list the
interpreter installs into the global scope. Before that existed the two drifted,
and the server was offering functions from a standard library that had been
deleted.

Adding a builtin in one place now updates the editor as well.

## Capabilities

Advertised in `initialize`: full document sync, completion, hover, definition,
references, document symbols, rename.

Not implemented: signature help, code actions, formatting, semantic tokens,
workspace symbols, folding, inlay hints.

## Testing it

The server is awkward to test by hand because the client has to complete the
initialize handshake before any request is answered. A short script that speaks
the protocol over stdio is the quickest way in: send `initialize`, wait for the
response, send `initialized`, then `didOpen`, then whatever you are testing.

To check it end to end inside Neovim:

```sh
nvim --headless -c 'lua vim.defer_fn(function()
  print(vim.inspect(vim.lsp.get_clients({bufnr=0})[1] ~= nil))
  print(#vim.diagnostic.get(0))
  vim.cmd("qa!")
end, 4000)' file.xen
```

Next: [Contributing](11-contributing.md)
