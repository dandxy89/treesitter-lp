# tree-sitter-lp

A [tree-sitter](https://tree-sitter.github.io/) grammar for LP (Linear Programming) files, the plain-text model format read and written by CPLEX, Gurobi, HiGHS, PuLP and other optimisation solvers.

It ships queries for syntax highlighting, code folding, indentation and local variables, plus bindings for Node.js, Python and Rust.

## Supported syntax

- Objective sense (`Minimize`, `Maximize` and their abbreviations), including Gurobi multi-objective blocks with `Priority=`/`Weight=` attributes
- Linear and quadratic (`[ ... ] / 2`) expressions
- `Subject To`, `Lazy Constraints` and `User Cuts` sections, with named, ranged, flipped and indicator (`b = 1 -> ...`) constraints
- Gurobi `General Constraints` (`r = MAX ( x , y , 3 )`)
- `Bounds` (including `free` and `inf`), `Generals`, `Integers`, `Binaries`, `Semi-Continuous` and `SOS` sections
- Line (`\ ...`) and block (`\* ... *\`) comments

Keywords are case-insensitive and accept the common aliases (`s.t.`, `st`, `gen`, `bin`, …). As in lp_parser_rs, a single-word section keyword (`bounds`, `bin`, `end`, …) is only a keyword at the start of a line and when not followed by `:`, so variables and constraints may use those names.

A minimal example:

```
Minimize
  obj: x1 + 2 x2 + 3 x3

Subject To
  c1: x1 + x2 <= 10
  c2: x2 + x3 >= 5

Bounds
  0 <= x1 <= 40
  x2 >= 0

Generals
  x3

End
```

## Development

Requires Node.js and the [tree-sitter CLI](https://github.com/tree-sitter/tree-sitter/tree/master/crates/cli) (installed as a dev dependency).

```sh
npm install              # install dependencies
npm run build            # regenerate src/parser.c from grammar.js
npm test                 # run the corpus tests in test/corpus/
npm run parse -- FILE    # print the syntax tree for an .lp file
npx prettier@3 --write grammar.js  # format grammar.js
```

After editing `grammar.js`, run `npm run build` then `npm test`. The generated files in `src/` are committed, so commit them alongside the grammar change.

## Neovim

These steps target the `main` branch of [nvim-treesitter](https://github.com/nvim-treesitter/nvim-treesitter/tree/main). Add to your config (e.g. `init.lua`):

```lua
vim.filetype.add({ extension = { lp = 'lp' } })

vim.api.nvim_create_autocmd('User', {
  pattern = 'TSUpdate',
  callback = function()
    require('nvim-treesitter.parsers').lp = {
      install_info = {
        url = 'https://github.com/dandxy89/tree-sitter-lp',
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

## Bindings

The Rust crate exposes `LANGUAGE`, `NODE_TYPES`, `HIGHLIGHTS_QUERY` and `LOCALS_QUERY`:

```rust
let mut parser = tree_sitter::Parser::new();
parser.set_language(&tree_sitter_lp::LANGUAGE.into())?;
let tree = parser.parse("min\n x\nst\n x >= 0\nend\n", None).unwrap();
```

The Node.js (`tree-sitter-lp`) and Python (`tree_sitter_lp`) packages expose the same language object and query strings.

## Acknowledgements

The grammar follows the LP dialect of [lp_parser_rs](https://github.com/dandxy89/lp_parser_rs) (MIT OR Apache-2.0): its Logos lexer and LALRPOP grammar are the reference for tokens, keyword rules and section structure. `grammar.js` records the upstream commit it was last synced with.

## Licence

MIT
