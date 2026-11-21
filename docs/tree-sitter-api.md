# Tree-sitter API

This page covers what a consumer of `tree-sitter-lp` gets: the language object in each binding, the query files, the node types, and the command-line tools. The node kinds themselves are in [ast-nodes.md](ast-nodes.md), which is generated from the grammar.

## Packages

| Package | Registry | Entry point | Language object |
| --- | --- | --- | --- |
| `tree-sitter-lp` | crates.io | `tree_sitter_lp::LANGUAGE`, `NODE_TYPES`, `*_QUERY` | `LANGUAGE.into()` gives a `tree_sitter::Language` |
| `tree-sitter-lp` | npm | `import Lp from "tree-sitter-lp"` | the default export; pass it to `parser.setLanguage` |
| `tree_sitter_lp` | PyPI | `tree_sitter_lp.language()` | a capsule for `tree_sitter.Language(...)` |

The Node package needs `tree-sitter` as a peer dependency (`^0.25`). The Python package's `core` extra installs `tree-sitter~=0.24`. The Rust crate depends on `tree-sitter-language` 0.1.

### Rust

```rust
let mut parser = tree_sitter::Parser::new();
parser.set_language(&tree_sitter_lp::LANGUAGE.into())?;
let tree = parser.parse("min\n x\nst\n x >= 0\nend\n", None).unwrap();
```

| Item | Type | Contents |
| --- | --- | --- |
| `LANGUAGE` | `LanguageFn` | the generated parser |
| `NODE_TYPES` | `&str` | `src/node-types.json` |
| `HIGHLIGHTS_QUERY` | `&str` | `queries/highlights.scm` |
| `LOCALS_QUERY` | `&str` | `queries/locals.scm` |
| `FOLDS_QUERY` | `&str` | `queries/folds.scm` |
| `INDENTS_QUERY` | `&str` | `queries/indents.scm` |

The queries are compiled into the crate with `include_str!`, so they always match the parser.

### Node

```js
import Parser from "tree-sitter";
import Lp from "tree-sitter-lp";

const parser = new Parser();
parser.setLanguage(Lp);
const tree = parser.parse("min\n x\nst\n x >= 0\nend\n");
```

The package exposes four query properties, read lazily from `queries/`: `HIGHLIGHTS_QUERY`, `LOCALS_QUERY`, `INJECTIONS_QUERY` and `TAGS_QUERY`. Only the first two files exist on `main` today. Until `queries/tags.scm` and `queries/injections.scm` are added, the other two properties are `undefined`.

### Python

```python
from tree_sitter import Language, Parser
import tree_sitter_lp

parser = Parser(Language(tree_sitter_lp.language()))
```

The module exposes `HIGHLIGHTS_QUERY` and `LOCALS_QUERY`, read from the wheel's bundled `queries/` directory on first access.

## Query files

| File | Purpose |
| --- | --- |
| `queries/highlights.scm` | syntax highlighting captures (keywords, operators, numbers, variables, comments, labels) |
| `queries/locals.scm` | local scopes, definitions and references, for variable navigation |
| `queries/folds.scm` | foldable regions: the body of each section |
| `queries/indents.scm` | indentation rules, based on the syntax tree |

Queries use standard tree-sitter capture names, so any editor that understands them can use them. The Rust crate's `test_queries_compile` test checks that all four compile against the grammar.

## Node types

The named node kinds, their fields and their children are listed in [ast-nodes.md](ast-nodes.md). It is generated from `src/node-types.json` by `scripts/gen-ast-docs.mjs` (`npm run docs:ast`). CI regenerates it and fails if the committed file is stale.

Fields that consumers may depend on:

| Node | Field | Meaning |
| --- | --- | --- |
| `constraint` | `name` | the constraint's label |
| `named_objective` | `name` | the objective's label (aliased to `objective_name`) |
| `general_constraint` | `name`, `resultant`, `function` | the label, the variable assigned to, and the function name (`function_name`) |
| `sos_constraint_header` | `name` | the SOS set's label (aliased to `sos_name`) |

Some kinds are aliases. For example, a constraint label is a `constraint_name` node, and a Gurobi function name is a `function_name` node. Check `ast-nodes.md` for the kind to match.

### Frozen surface

Downstream consumers, such as the lp-lsp language server, depend on specific node kinds and fields. These are marked **frozen** in [ast-nodes.md](ast-nodes.md). The rules are:

- Renaming or removing a frozen kind or field is a breaking change to the parse tree.
- Adding a new kind or field is not a breaking change.
- `scripts/gen-ast-docs.mjs` fails if a frozen kind or field is missing from `src/node-types.json`.

## Command line

With the tree-sitter CLI (installed as a dev dependency, `npx tree-sitter`):

| Task | Command |
| --- | --- |
| regenerate `src/` after editing `grammar.js` | `npm run build` (`tree-sitter generate`) |
| print the tree for a file | `npm run parse -- FILE.lp` |
| run the corpus tests | `npm test` |
| check highlight captures | `npx tree-sitter highlight --check test/highlight/*.lp` |
| build a shared library for local editors | `npx tree-sitter build -o lp.so` |
| build the WebAssembly module | `npm run build:wasm` (`tree-sitter build --wasm`; needs Docker or Emscripten) |
| regenerate the AST reference | `npm run docs:ast` |

The WebAssembly module can be loaded in a browser with `web-tree-sitter`, for example to build a web editor or a tree viewer without a native build.

## Versions

- The crate, npm package and Python package share one version, set in `Cargo.toml`, `package.json`, `tree-sitter.json` and `pyproject.toml`.
- A change that alters the parse tree is a breaking change for consumers. While the version is `0.x`, it ships as a minor version.
