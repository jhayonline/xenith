-- Buffer-local settings for Xenith source files.

vim.bo.commentstring = "# %s"
vim.bo.comments = "b:#"

-- Four spaces, matching the samples in testies/ and docs/.
vim.bo.expandtab = true
vim.bo.shiftwidth = 4
vim.bo.softtabstop = 4
vim.bo.tabstop = 4

-- `#` starts a comment, so it must not be treated as part of a word.
vim.opt_local.iskeyword:append("_")

vim.opt_local.suffixesadd:prepend(".xen")
