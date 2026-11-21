# Supported dialects

This page says which LP dialect constructs `tree-sitter-lp` parses, which lp_parser_rs reads, and what each solver's format is documented to support. A solver's own documentation is not in this repository, so its column records only what the repository's sources say.

## How to read the matrix

| Mark | Meaning |
| --- | --- |
| yes | The source (grammar, lp_parser_rs code, tests or its README) supports the construct. |
| check | Supported syntactically; lp_parser_rs performs a semantic check on top (see [Semantic checks](#semantic-checks)). |
| no | The source says the construct is not supported. |
| — | The sources in this repository do not state a position for this dialect. This is not a claim that the dialect lacks the construct. |
| unverified | No evidence in this repository. |

Sources:

- **tree-sitter-lp**: `grammar.js` and the corpus in `test/corpus/`.
- **lp_parser_rs**: its README table and `rust/src/lexer.rs`, `rust/src/assemble.rs`. The reader is documented as accepting the CPLEX, Gurobi, FICO Xpress and Mosek dialects.
- **CPLEX** and **Gurobi**: the dialect named in lp_parser_rs's README row for the construct.
- **HiGHS** and **PuLP**: no source in this repository. Their columns stay *unverified* until fixtures for them exist and are tested (see [Evidence](#evidence)).

## Matrix

| Construct | LP syntax | tree-sitter-lp | lp_parser_rs | CPLEX | Gurobi | HiGHS | PuLP |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Sense (`min`, `max` and variants) | `Minimize`, `Maximise`, `maximum`, ... | yes | yes | — | — | unverified | unverified |
| Linear objective and constraints | `obj: x + 2 y`, `c1: x + y <= 4` | yes | yes | yes | yes | unverified | unverified |
| `Subject To` section | `Subject To`, `such that`, `st`, `s.t.` | yes | yes | — | — | unverified | unverified |
| Bounds section | `Bounds`, `x <= 4`, `x free` | yes | yes | — | — | unverified | unverified |
| Generals / Integers / Binaries | `Generals`, `Integers`, `Binaries` | yes | yes | — | — | unverified | unverified |
| Semi-continuous variables | `Semi-Continuous` | yes | yes | yes | — | unverified | unverified |
| Semi-integer variables | variable in `Generals` and `Semi-Continuous` | yes | yes | yes | — | unverified | unverified |
| SOS sets | `s1: S1 ::` with `x:1` entries | yes | yes | — | — | unverified | unverified |
| Lazy constraints | `Lazy Constraints` section | yes | yes | yes | — | unverified | unverified |
| User cuts | `User Cuts` section | yes | yes | yes | — | unverified | unverified |
| Indicator constraints | `c: b = 1 -> x + y <= 3` | yes | yes | yes | yes | unverified | unverified |
| Indicator `<->` and `<-` | `b = 1 <-> ...` | no | no | — | — | unverified | unverified |
| Quadratic objective | `[ x ^ 2 + x * y ] / 2` | yes | check (`/ 2` required) | yes | yes | unverified | unverified |
| Quadratic constraint | `c: x + [ x ^ 2 ] <= 4` | yes | yes | — | — | unverified | unverified |
| General constraints | `General Constraints`, `g: r = MAX ( x , y )` | yes | check (function set) | — | yes | unverified | unverified |
| General function `PWL` | `r = PWL (...)` | no | no | — | no | unverified | unverified |
| Multi-objective | `Minimize multi-objectives`, `OBJ: Priority=2` | yes | yes | — | yes | unverified | unverified |
| Ranged constraints | `lo <= expr <= hi` | yes | check (operators agree) | — | — | unverified | unverified |
| Flipped constraints | `3 <= x + y` | yes | yes | — | — | unverified | unverified |
| Operators `=<`, `=>` | `c: x =< 4` | yes | yes | yes | — | unverified | unverified |
| Infinity spellings | `inf`, `infinity`, `-inf` (any case) | yes | yes | — | — | unverified | unverified |
| Line comments | `\ text` | yes | yes | yes | — | unverified | unverified |
| Block comments | `\* text *\` | yes | yes | — | — | unverified | unverified |
| Names with `[`, `]`, `%`, `>` | `x[1]`, `ArcFlow%>%[0]` | yes | yes | — | yes | unverified | unverified |

Notes on the rows:

- Quadratic objectives: lp_parser_rs's README requires the `/ 2` ("as in CPLEX and Gurobi"). The grammar accepts the block with or without it.
- General constraints: lp_parser_rs supports `MAX`, `MIN`, `ABS`, `AND` and `OR`. Its README says `PWL` is not supported.
- Multi-objective attributes: `Priority`, `Weight`, `AbsTol`, `RelTol`, each optional.
- Operators: lp_parser_rs's lexer documents that CPLEX accepts `=<` and `=>`.

## Constructs not supported

- Indicator `<->` and `<-` forms (lp_parser_rs README).
- General-constraint function `PWL` (lp_parser_rs README).
- Writing general constraints: lp_parser_rs's writer returns an error. Writing multi-objective models also returns an error, unless `allow_multiple_objectives` is set, which keeps only the first objective. The grammar parses both forms.
- Other dialect-specific sections, such as those of Xpress or Mosek, are not in the grammar and produce `ERROR` nodes.

## Semantic checks

The grammar accepts these forms and lp_parser_rs rejects them. A file can parse cleanly here and still be rejected by lp_parser_rs.

- A quadratic term's exponent other than `2`.
- A quadratic objective without `/ 2`. A quadratic constraint with `/ 2`.
- A ranged constraint whose two operators point different ways.
- An indicator value other than `0` or `1`.
- An SOS type other than `S1` or `S2`, including the unnamed form.
- A general-constraint function outside `MAX`, `MIN`, `ABS`, `AND`, `OR`.
- An unknown multi-objective attribute name.

## Deliberately out of scope

These are not planned for `tree-sitter-lp`:

- Dialect validation. Deferred: the grammar stays permissive, and the semantic checks above belong to lp_parser_rs.
- Go and C bindings. `tree-sitter.json` enables only Rust, Node and Python.
- A standalone playground. Use `tree-sitter playground` with the WebAssembly build.
- An LSP, formatter or VS Code extension. These live in lp-lsp, from lp_parser_rs. See [editor-integration.md](editor-integration.md).

## Evidence

- `npm test` runs the corpus in `test/corpus/` (bounds, comments, constraints, numbers, objectives, ranged, sense, SOS and variable types, among others).
- `cargo test` runs a differential test against lp_parser_rs over generated files. Whenever lp_parser_rs accepts a file, the tree must have no errors and describe the same model.
- The generated corpus and the differential test do not use solver-written files. Fixtures written in the style of CPLEX, Gurobi, HiGHS and PuLP output are needed before those columns can be marked *yes*. Until they exist, those columns stay *unverified*.
