# Xenith for Neovim

Filetype detection, syntax highlighting, indenting, and the `xenith-lsp`
language server.

## Install

Build and install the binaries first — the plugin runs the server by name:

```sh
cargo install --path /path/to/xenith
```

That puts `xenith` and `xenith-lsp` in `~/.cargo/bin`.

### lazy.nvim

Point at a local checkout:

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

Or install straight from the repository, which carries the plugin in a
subdirectory:

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

### Without a plugin manager

Copy or symlink the contents of this directory into `~/.config/nvim`, then call
`require("xenith").setup()` from your config.

## Options

```lua
require("xenith").setup({
  cmd = { "xenith-lsp" },   -- server command
  root_markers = { ".git" },-- project root detection
  enable = true,            -- false to skip the server entirely
  server = {                -- merged into vim.lsp.config
    capabilities = require("cmp_nvim_lsp").default_capabilities(),
  },
})
```

Highlighting and indenting work with no setup call at all; `setup()` only
registers the language server.

## What the server does

- **Diagnostics** on every keystroke, from the same lexer and parser the
  interpreter uses, so the message in the editor is the message you get on the
  command line.
- **Hover** over a definition for its type or signature, and over a built-in or
  keyword for its documentation.
- **Go to definition**, **find references**, and **rename** across the file.
- **Document symbols** for the outline, nested so a method's parameters and
  locals sit under it.
- **Completion** for keywords, types, built-ins, and everything the file
  defines.

### Known limits

Symbol resolution is by name and file-local. Two variables with the same name in
different scopes are treated as one symbol, so rename over a common name like
`i` will rewrite all of them. Cross-file `grab` imports are not followed. Both
are waiting on the resolver pass in the interpreter.

Diagnostics are lexer and parser errors only. Type errors still surface at
runtime until the static checker lands.
