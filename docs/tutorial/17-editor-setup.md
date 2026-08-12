# Editor Setup

Xenith ships a language server, `xenith-lsp`, and a Neovim plugin. Both are
installed by `cargo install --path .` at the repository root.

## What the language server gives you

- **Diagnostics as you type.** Syntax and type errors appear immediately, from
  the same lexer, parser and checker the interpreter uses, so what the editor
  tells you is what the command line will tell you.
- **Hover.** Over your own definition it shows the type or signature. Over a
  builtin or a keyword it shows what that does.
- **Go to definition, find references, rename**, within the current file.
- **Document symbols**, nested so a method's parameters and locals sit under it.
- **Completion** for keywords, types, builtins and everything the file defines.

Symbol lookup is by name and file local. Two variables called `i` in different
methods count as one symbol, so renaming a common name will rename all of them.
Imports are not followed across files.

## Neovim

The plugin lives in `editors/nvim` in the repository. It provides filetype
detection for `.xen`, syntax highlighting, indenting, and the server
registration.

With lazy.nvim, pointing at a local checkout:

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

Or straight from the repository:

```lua
{
  "jhayonline/xenith",
  name = "xenith.nvim",
  lazy = false,
  config = function()
    require("xenith").setup()
  end,
}
```

Without a plugin manager, copy or symlink the contents of `editors/nvim` into
`~/.config/nvim` and call `require("xenith").setup()` from your config.

Highlighting and indenting need no setup call. `setup()` only registers the
language server.

### Options

```lua
require("xenith").setup({
  cmd = { "xenith-lsp" },     -- how to start the server
  root_markers = { ".git" },  -- how a project root is found
  enable = true,              -- false to skip the server entirely
  server = {                  -- merged into vim.lsp.config
    capabilities = require("cmp_nvim_lsp").default_capabilities(),
  },
})
```

If `xenith-lsp` is not on your PATH the plugin says so once and carries on with
highlighting only.

## Other editors

Any editor that speaks LSP can use the server; it talks JSON-RPC over stdio and
needs no arguments. Point your client at the `xenith-lsp` binary and associate it
with the `xenith` filetype for `.xen` files.

There is no VS Code extension yet.

## Syntax highlighting without the server

The Neovim plugin's `syntax/xenith.vim` is a plain Vim syntax file with no
dependencies, so it works in Vim as well as Neovim and does not need the server
running. It covers keywords, types, builtins, numbers, operators, comments,
strings with their escapes and interpolation, and backtick raw strings.

There is no tree-sitter grammar yet.

Next: [Known limitations](18-limitations.md)
