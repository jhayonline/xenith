-- Neovim support for Xenith.
--
-- Syntax highlighting, filetype detection and indenting come from the
-- ftdetect/syntax/indent directories and need no configuration. This module
-- only registers the language server.

local M = {}

local defaults = {
  -- Command used to start the server. `xenith-lsp` is installed alongside the
  -- `xenith` binary by `cargo install --path .`.
  cmd = { "xenith-lsp" },
  -- Files that mark the root of a project. The server analyses one file at a
  -- time and does not read the workspace, so this only affects how many client
  -- processes Neovim starts.
  root_markers = { ".git" },
  -- Extra options merged into the vim.lsp.config table (capabilities,
  -- on_attach, and so on).
  server = {},
  -- Set false to handle the server registration yourself.
  enable = true,
}

---@param opts table|nil
function M.setup(opts)
  opts = vim.tbl_deep_extend("force", defaults, opts or {})

  if vim.fn.executable(opts.cmd[1]) == 0 then
    -- Not an error: the highlighting half of this plugin still works, and the
    -- user may simply not have built the server yet.
    vim.notify_once(
      ("xenith: %s not found on PATH; language features are off. Run `cargo install --path .` in the Xenith repo.")
        :format(opts.cmd[1]),
      vim.log.levels.WARN
    )
    return
  end

  if not opts.enable then
    return
  end

  local config = vim.tbl_deep_extend("force", {
    cmd = opts.cmd,
    filetypes = { "xenith" },
    root_markers = opts.root_markers,
  }, opts.server)

  -- vim.lsp.config landed in 0.11; older versions get a plain autocmd.
  if vim.lsp.config ~= nil and vim.lsp.enable ~= nil then
    vim.lsp.config("xenith_lsp", config)
    vim.lsp.enable("xenith_lsp")
    return
  end

  vim.api.nvim_create_autocmd("FileType", {
    pattern = "xenith",
    group = vim.api.nvim_create_augroup("XenithLsp", { clear = true }),
    callback = function(event)
      local file = vim.api.nvim_buf_get_name(event.buf)
      local root = vim.fs.dirname(vim.fs.find(opts.root_markers, {
        path = file,
        upward = true,
      })[1]) or vim.fs.dirname(file)

      vim.lsp.start(vim.tbl_extend("force", config, {
        name = "xenith_lsp",
        root_dir = root,
      }), { bufnr = event.buf })
    end,
  })
end

return M
