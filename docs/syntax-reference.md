# LP syntax reference

This is the syntax that `tree-sitter-lp` parses. It follows the LP dialect of [lp_parser_rs](https://github.com/dandxy89/lp_parser_rs), which is the reference for tokens, keyword rules and section order. For which solver dialect supports which construct, see [supported-dialects.md](supported-dialects.md). For the tree shape, see [tree-sitter-api.md](tree-sitter-api.md) and [ast-nodes.md](ast-nodes.md).

The grammar is permissive about semantics. It accepts a construct whenever the syntax is well formed, and leaves checks such as "the exponent must be 2" or "a range's operators must point the same way" to lp_parser_rs. A file that parses cleanly here may still be rejected by lp_parser_rs.

## File layout

```
<sense> [multi-objectives]
<objectives>
Subject To
<constraints>
[Lazy Constraints | User Cuts | General Constraints | Bounds | Generals | Integers |
 Binaries | Semi-Continuous | SOS]   (any order, any number of times)
End
```

- The sense line and `Subject To` are required. `End` is optional.
- The sections after `Subject To` may appear in any order and repeat.
- Every section keyword is case-insensitive.

A minimal file:

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

## Sense

| Keyword | Meaning |
| --- | --- |
| `minimize`, `minimise`, `minimum`, `min` | minimise the objective |
| `maximize`, `maximise`, `maximum`, `max` | maximise the objective |

## Objectives

An objective is a linear expression, optionally named (`obj: ...`). The grammar also accepts an unnamed objective followed by named ones.

```
Maximize
  profit: 3 x + 2 y + [ x ^ 2 + x * y ] / 2
```

- A quadratic block is `[ ... ]`. Its terms are `x ^ 2` or `x * y`, with an optional coefficient. The grammar accepts the block with or without a trailing `/ 2`. lp_parser_rs requires `/ 2` in objectives.
- `multi-objectives` directly after the sense keyword starts a Gurobi multi-objective model. Each objective may carry attributes before its expression: `name: Priority=2 Weight=1 AbsTol=0 RelTol=0`. Every attribute is optional.

## Constraints

A constraint is an optional name, an optional indicator, and a body. The `Subject To` section and the `Lazy Constraints` and `User Cuts` sections all hold constraints.

```
Subject To
  c1: x + y <= 10
  c2: 2 x - 3 y >= -4
  c3: x + 2 y = 7
  -x1 + x2 <= 3
  2 x1 >= 2
```

The name is an identifier followed by `:` or `::`. An unnamed constraint has no label.

**Standard form.** `expr op number`, where the left side mentions at least one variable and the right side is one signed number. `op` is `<=`, `<`, `>=`, `>`, `=`, or the aliases `=<` and `=>`.

**Flipped form.** A left side made of constants only: `3 <= x + y` is read as `x + y >= 3`. The operator is reversed when the constant moves to the right.

**Ranged form.** `lo <= expr <= hi`. Operators must point the same way (both `<=`-like or both `>=`-like). lp_parser_rs rejects mixed operators, and the grammar does not check this. The grammar reads a range as `lo op expr op hi` even when the expression itself starts with a number, so `3 <= 2 x + y <= 9` is one constraint.

**Indicator.** A binary variable controls the constraint: `name: b = 1 -> x + y <= 3`. The value after `=` is `0` or `1`. The forms `<->` and `<-` are not supported.

**Quadratic constraint.** A quadratic block on the left side: `c: x + [ x ^ 2 + y ^ 2 ] <= 4`. Constraints take no `/ 2`.

## General constraints (Gurobi)

Under a `General Constraints` section, each entry is `name: resultant = FUNCTION ( arg, arg, ... )`:

```
General Constraints
  g1: r = MAX ( x , y , 3 )
  g2: z = ABS ( x )
  g3: w = AND ( b1 , b2 )
```

Functions: `MAX`, `MIN`, `ABS`, `AND`, `OR`. Arguments are variable names or signed numbers. The grammar accepts any identifier as the function name and leaves the set to lp_parser_rs. Parentheses and commas must be separated from names by spaces, because `(`, `)` and `,` are valid name characters.

The section keyword may be written `General Constraints`, `General Constraint`, `General Constrs`, `Gen Cons`, `GenConstrs`, or `GenConstr`.

## Bounds

```
Bounds
  x1 free
  x2 <= 40
  x3 >= -5
  0 <= x4 <= 10
  -inf <= x5
  x6 = 2
```

- `name free` makes the variable free (unbounded both ways).
- `name op number` sets a one-sided bound, with the operator read as written: `x <= 40` is an upper bound and `x >= 3` is a lower bound. `=` fixes the variable.
- `number op name` is the same with the variable on the right. `number op name op number` is a double bound.
- `inf`, `infinity`, `-inf` and `+inf` are the infinity spellings, in any case.

## Variable types

| Section | Keywords | Contents |
| --- | --- | --- |
| Generals | `Generals`, `General`, `Gen` | integer variables with general bounds |
| Integers | `Integers`, `Integer` | integer variables |
| Binaries | `Binaries`, `Binary`, `Bin` | binary variables |
| Semi-Continuous | `Semi-Continuous`, `Semis`, `Semi` | semi-continuous variables |

Each section is followed by one or more variable names. Semi-integer variables are written by listing a variable in both `Generals` (or `Integers`) and `Semi-Continuous`. The grammar records the two sections, and lp_parser_rs derives the semi-integer type.

## Special ordered sets

```
SOS
  s1: S1 ::
    x1:1 x2:2 x3:3
  s2: S2 ::
    y1:1 y2:2
```

- An SOS set opens with `name: S1 ::` or `name: S2 ::`. The type is `S1` or `S2`.
- CPLEX also allows the unnamed form `S1 ::`. There the type is an ordinary identifier, and lp_parser_rs checks that it is `S1` or `S2`.
- Each entry is `variable: weight`. The weight is a signed number.

## Lazy constraints and user cuts (CPLEX)

```
Lazy Constraints
  lc1: x + y <= 5

User Cuts
  uc1: x - y >= -1
```

Each section holds constraints in the same forms as `Subject To`.

## Names

Names are made of:

- letters, digits and `_`
- the characters `! # $ % & ( ) , . ; ? @ { } ~ ' [ ] |`
- `-` and `>` when they sit inside a name: `a-b`, `x>y`

A name cannot start with a digit, `-` or `>`. A `>` continues a name only when a name character follows it, so `y>=3` lexes as `y >= 3`. Keywords are not reserved as names when they appear mid-line, so `bin`, `end` or `min` may be variable names. They are keywords only where the line rules below apply.

Two brackets have a special role. A lone `[` or `]` opens or closes a quadratic block, so it must be separated from neighbouring names by whitespace. Names such as `x[1]` are unaffected.

## Numbers and infinity

- Numbers: `3`, `3.`, `.5`, `1e5`, `2E-3`, `1.5e+2`. Signs are separate tokens, so `-x` is a sign followed by the name `x`.
- Infinity: `inf` or `infinity`, in any case. These are the only spellings the grammar accepts.

## Comments

- `\` starts a line comment that runs to the end of the line.
- `\* ... *\` is a block comment. It may span lines and may appear between tokens.

## Keywords and the line rule

Section keywords are recognised only in the places where they play their grammatical role. The rules follow lp_parser_rs:

- The sense keyword is a keyword only as the first token.
- `multi-objectives` is a keyword only directly after the sense keyword.
- `Subject To` (and its aliases `such that`, `st`, `s.t.`) is a keyword only at the start of a line that does not continue an expression.
- Single-word section keywords (`bounds`, `generals`, `bin`, `sos`, `end`, and so on) are keywords only as the first token of a line that does not continue an expression. They are not keywords when followed by `:` or `::`, which makes them a label.
- `S1` and `S2` are types only in `name: S1 ::`.
- `free` is a keyword only directly after an identifier on the same line.

Consequently a variable called `bin` may appear in an expression, or on its own line inside a list if the line-start rule does not apply. Avoid putting a bare keyword alone on a line in a list, because it reads as a section header.

## Errors and partial files

A syntax error produces `ERROR` or `MISSING` nodes in the tree, and an invalid file is not a valid LP model. The test corpus does not yet assert how much of the tree is recovered after an error, so do not rely on the shape of the tree after one.
