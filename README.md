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
npm run test:bindings    # Node binding test
npm run docs:ast        # regenerate docs/ast-nodes.md from src/node-types.json
npm run build:wasm       # build the WebAssembly module (needs Docker or Emscripten)
cargo test               # Rust binding tests, including the differential test against lp_parser_rs
npx prettier@3 --write grammar.js  # format grammar.js
```

CI (`.github/workflows/ci.yml`) runs each of these on the platforms it supports, and fails if the generated files in `src/` or `docs/ast-nodes.md` are out of date.

After editing `grammar.js`, run `npm run build` then `npm test`. The generated files in `src/` are committed, so commit them alongside the grammar change.

## Documentation

- [Syntax reference](docs/syntax-reference.md): the LP syntax the grammar accepts, section by section.
- [Supported dialects](docs/supported-dialects.md): which constructs CPLEX, Gurobi, HiGHS, PuLP, lp_parser_rs and this grammar support.
- [Tree-sitter API](docs/tree-sitter-api.md): the Rust, Node and Python entry points, query files and frozen node kinds.
- [AST node types](docs/ast-nodes.md): every named node kind, generated from the grammar.
- [Editor integration](docs/editor-integration.md): Neovim, Helix and VS Code, with the lp-lsp language server.

## Bindings

The Rust crate exposes `LANGUAGE`, `NODE_TYPES`, `HIGHLIGHTS_QUERY` and `LOCALS_QUERY`:

```rust
let mut parser = tree_sitter::Parser::new();
parser.set_language(&tree_sitter_lp::LANGUAGE.into())?;
let tree = parser.parse("min\n x\nst\n x >= 0\nend\n", None).unwrap();
```

The Node.js (`tree-sitter-lp`) and Python (`tree_sitter_lp`) packages expose the same language object and query strings.

## Not planned

These are deliberately left out of this project:

- Dialect validation. The grammar stays permissive. lp_parser_rs performs the semantic checks.
- Go and C bindings.
- A standalone playground. Use `tree-sitter playground`, or the WebAssembly build (`npm run build:wasm`).
- An LSP, formatter or VS Code extension for this grammar. These live in lp-lsp, in lp_parser_rs.

## Acknowledgements

The grammar follows the LP dialect of [lp_parser_rs](https://github.com/dandxy89/lp_parser_rs) (MIT OR Apache-2.0): its Logos lexer and LALRPOP grammar are the reference for tokens, keyword rules and section structure. `grammar.js` records the upstream version it was last synced with, and `cargo test` compares the two parsers on generated files.

## Licence

MIT OR Apache-2.0
