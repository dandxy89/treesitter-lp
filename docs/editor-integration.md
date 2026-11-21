# Editor integration

There are two ways to get LP support in an editor, and they can be used together:

- **Tree-sitter** (this repository) gives syntax highlighting, folding, indentation and local-variable navigation from the grammar and queries alone. It needs no server.
- **lp-lsp** (in [lp_parser_rs](https://github.com/dandxy89/lp_parser_rs)) is a language server that adds diagnostics, navigation, rename, hover, completion, formatting, code actions and semantic tokens. It uses this grammar for its syntax index. It also ships the VS Code extension.

The tree-sitter setup is enough for highlighting. Add lp-lsp for the language features.

## Neovim

### Tree-sitter highlighting and folding

These steps target the `main` branch of [nvim-treesitter](https://github.com/nvim-treesitter/nvim-treesitter/tree/main). Add to your config (e.g. `init.lua`):

```lua
vim.filetype.add({ extension = { lp = 'lp' } })

vim.api.nvim_create_autocmd('User', {
  pattern = 'TSUpdate',
  callback = function()
    require('nvim-treesitter.parsers').lp = {
      install_info = {
        url = 'https://github.com/dandxy89/treesitter-lp',
        queries = 'queries',
      },
    }
  end,
})

-- Enable highlighting and folding for LP buffers
vim.api.nvim_create_autocmd('FileType', {
  pattern = 'lp',
  callback = function()
    vim.treesitter.start()
    vim.wo.foldexpr = 'v:lua.vim.treesitter.foldexpr()'
    vim.wo.foldmethod = 'expr'
  end,
})
```

Then run `:TSInstall lp`.

### lp-lsp

Install the server with `cargo install --locked lp-lsp`, then add a server config. This needs Neovim 0.11 or later.

```lua
-- ~/.config/nvim/lsp/lp_lsp.lua
return {
  cmd = { 'lp-lsp' },
  filetypes = { 'lp' },
  root_markers = { '.git' },
  settings = { lp = { format = { indent = 2 } } },
}
```

```lua
-- init.lua
vim.filetype.add({ extension = { lp = 'lp' } })
vim.lsp.enable('lp_lsp')
vim.lsp.inlay_hint.enable(true)
```

The "used in N constraints" code lens runs the client command `lp.showReferences`. To send it to the quickfix list, add this to the server config:

```lua
commands = {
  ['lp.showReferences'] = function(command, ctx)
    local client = assert(vim.lsp.get_client_by_id(ctx.client_id))
    local items = vim.lsp.util.locations_to_items(command.arguments[3] or {}, client.offset_encoding)
    vim.fn.setqflist({}, ' ', { title = 'LP references', items = items })
    vim.cmd.copen()
  end,
},
```

## Helix

Helix uses tree-sitter for highlighting and an LSP for language features. Both go in `~/.config/helix/languages.toml`.

Tree-sitter grammar and queries:

```toml
[[language]]
name = "lp"
scope = "source.lp"
file-types = ["lp"]
comment-token = "\\"
roots = []
language-servers = ["lp-lsp"]

[[grammar]]
name = "lp"
source = { git = "https://github.com/dandxy89/treesitter-lp", rev = "main" }
```

Replace `rev` with a tag once one is published. Then fetch and build the grammar, and copy the queries next to Helix's runtime:

```sh
hx --grammar fetch
hx --grammar build
mkdir -p ~/.config/helix/runtime/queries/lp
cp path/to/treesitter-lp/queries/*.scm ~/.config/helix/runtime/queries/lp/
```

The language server, which runs `lp-lsp`:

```toml
[language-server.lp-lsp]
command = "lp-lsp"
```

## VS Code

Install the lp-lsp extension from [`lsp/editors/vscode`](https://github.com/dandxy89/lp_parser_rs/tree/main/lsp/editors/vscode). It needs `lp-lsp` on the `PATH`:

```bash
cargo install --locked lp-lsp
cd lp_parser_rs/lsp/editors/vscode
npm ci && npm run package
code --install-extension lp-lsp-*.vsix
```

The extension starts `lp-lsp` for `*.lp` files. It looks for the server in this order: the `lp.server.path` setting, a bundled `lp-lsp` binary, then `lp-lsp` on the `PATH`. This repository does not ship a VS Code extension.

## Other editors

Any editor with tree-sitter support can use the grammar and queries from this repository. Any editor with LSP support can run `lp-lsp` for `*.lp` files. Zed, for example, needs a language extension to register `*.lp` files, then can point at the `lp-lsp` binary in `settings.json`. The lp-lsp README has the full steps.
