/**
 * @file Tree-sitter grammar for LP (Linear Programming) files.
 *
 * Follows the dialect accepted by the `lp_parser_rs` crate: CPLEX LP plus
 * Gurobi extensions (multi-objectives, general constraints, indicators).
 *
 * Derived from lp_parser_rs (https://github.com/dandxy89/lp_parser_rs),
 * MIT OR Apache-2.0: its Logos lexer (`rust/src/lexer.rs`) and LALRPOP
 * grammar (`rust/src/lp.lalrpop`) are the reference. Last synced with
 * commit 43b756b (branch `fix/improvements`).
 */

/// <reference types="tree-sitter-cli/dsl" />
// @ts-check

// Identifier: matches the Logos lexer regex in lp_parser_rs exactly.
// `>` is only accepted mid-name before a non-numeric name character, so
// `y>=3` still lexes as a comparison.
const ID_SPECIAL = "!#$%&(),.;?@{}~'\\[\\]";
const ID_FIRST = `[a-zA-Z_${ID_SPECIAL}]`;
const ID_CONT = `[a-zA-Z0-9_${ID_SPECIAL}|]`;
const ID_AFTER_GT = "[a-zA-Z_!#$%&(),;?@{}~'|\\[\\]]";
const IDENTIFIER_RE = new RegExp(
  `${ID_FIRST}(${ID_CONT}|-${ID_CONT}|>${ID_AFTER_GT})*`,
);

// Additive sign between or before terms.
const sign = () => choice("+", "-");

// `item (sign item)*` with an optional leading sign.
const signed = (item) => seq(optional(sign()), item, repeat(seq(sign(), item)));

// An unsigned numeric literal or `inf`/`infinity`.
const constant = ($) => choice($.number, $.infinity);

export default grammar({
  name: "lp",

  extras: ($) => [/[ \t\r\n]/, $.line_comment, $.block_comment],

  // Single-word section keywords are line-start sensitive, and block comments
  // carry line starts across them (src/scanner.c).
  externals: ($) => [
    $.bounds_keyword,
    $.generals_keyword,
    $.integers_keyword,
    $.binaries_keyword,
    $.semi_continuous_keyword,
    $.sos_keyword,
    $.end_marker,
    $._genconstrs_keyword,
    $.block_comment,
    $._constraint_name,
    // In no rule, so valid only during error recovery.
    $._error_sentinel,
  ],

  // Constraints are parsed deterministically, as upstream groups them (see
  // `_constraint_body`); only `b = 1 ->` (indicator or equality) and a body
  // opening with `3 + ...` (constant lower bound or standard left-hand side)
  // need a token or two of extra lookahead.
  conflicts: ($) => [
    [$.term],
    [$.named_objective],
    [$.indicator, $._variable_term],
    [$._lhs_constant, $._constant_bound],
  ],

  rules: {
    // Sense, objectives and `Subject To` are fixed; the remaining sections
    // may appear in any order and any number of times.
    source_file: ($) =>
      seq(
        $.sense,
        optional($.multi_objectives_keyword),
        optional($.objectives_section),
        $.constraints_section,
        repeat($._any_section),
        optional($.end_marker),
      ),

    sense: (_) => token(/minimi[sz]e|minimum|min|maximi[sz]e|maximum|max/i),

    multi_objectives_keyword: (_) => token(/multi-objectives?/i),

    // The upstream lexer consumes an optional same-line `:` after the header.
    subject_to_keyword: (_) =>
      token(/(subject[ \t]+to|such[ \t]+that|s\.t\.|st)([ \t]*:)?/i),

    lazy_constraints_keyword: (_) => token(/lazy[ \t]+constraints/i),

    user_cuts_keyword: (_) => token(/user[ \t]+cuts/i),

    // Multi-word forms only; single-word `genconstrs` comes from the scanner.
    general_constraints_keyword: (_) =>
      token(/general[ \t]+constraints?|general[ \t]+constrs?|gen[ \t]+cons/i),

    free_keyword: (_) => token(/free/i),

    sos_type: (_) => token(/s[12]/i),

    number: (_) => token(/([0-9]+\.?[0-9]*|[0-9]*\.[0-9]+)([eE][+-]?[0-9]+)?/),

    // Signs are separate tokens so `-inflow` is `-` and the name `inflow`.
    infinity: (_) => token(/inf(inity)?/i),

    identifier: (_) => token(IDENTIFIER_RE),

    line_comment: (_) => token(/\\([^\n*][^\n]*)?/),

    // `=<` and `=>` are accepted as aliases for `<=` and `>=`.
    comparison_operator: (_) => choice("<=", "=<", ">=", "=>", "<", ">", "="),

    _numeric_value: ($) => seq(optional(sign()), constant($)),

    // Objective term: a variable with an optional coefficient, or a bare
    // constant. Constraints build the same `term` nodes from their own rules.
    term: ($) => choice(seq(optional(constant($)), $.identifier), constant($)),

    linear_expression: ($) => signed($._expression_item),

    _expression_item: ($) => choice($.term, $.quadratic_block),

    // `/ 2` is optional here; upstream requires it in objectives and
    // rejects it in constraints (a semantic check, like the exponent being 2).
    quadratic_block: ($) =>
      seq("[", signed($.quadratic_term), "]", optional(seq("/", $.number))),

    quadratic_term: ($) =>
      seq(
        optional($.number),
        $.identifier,
        choice(seq("^", $.number), seq("*", $.identifier)),
      ),

    // A single unnamed objective, optionally followed by named ones, or
    // named objectives only (multi-objective files).
    objectives_section: ($) =>
      choice(
        seq($.linear_expression, repeat($.named_objective)),
        repeat1($.named_objective),
      ),

    named_objective: ($) =>
      seq(
        field("name", alias($.identifier, $.objective_name)),
        ":",
        repeat($.objective_attribute),
        optional($.linear_expression),
      ),

    // Gurobi multi-objective attribute: `Priority=2`, `Weight=1`, ...
    objective_attribute: ($) =>
      seq(
        field("name", alias($.identifier, $.attribute_name)),
        "=",
        $._numeric_value,
      ),

    constraints_section: ($) => seq($.subject_to_keyword, repeat($.constraint)),

    // The name is an external token: an identifier followed by `:` or `::`
    // (src/scanner.c), so a constant ending one constraint's expression is
    // told apart from the coefficient of a variable by one token. During
    // error recovery the scanner lexes no names, and a name is an identifier
    // followed by `:`, so recovery can resume at a named constraint.
    constraint: ($) =>
      seq(
        optional(
          seq(
            field(
              "name",
              alias(
                choice($._constraint_name, $.identifier),
                $.constraint_name,
              ),
            ),
            choice(":", "::"),
          ),
        ),
        optional($.indicator),
        $._constraint_body,
      ),

    // Indicator head: `b = 1 ->`
    indicator: ($) => seq($.identifier, "=", $.number, "->"),

    // Mirrors upstream `assemble_constraints` (lp_parser_rs
    // `rust/src/assemble.rs`), which reads an entry greedily:
    // - a left-hand side that mentions a variable is standard, `expr op rhs`,
    //   with a single signed number on the right;
    // - a left-hand side of constants only is flipped, `lo op expr`, or,
    //   when another operator follows the (longest) expression, ranged,
    //   `lo op expr op hi`. Upstream rejects a range whose operators point
    //   different ways; that is a semantic check, not a grouping one.
    _constraint_body: ($) =>
      choice(
        seq(
          alias($._standard_lhs, $.linear_expression),
          $.comparison_operator,
          $._numeric_value,
        ),
        seq(
          $._constant_bound,
          $.comparison_operator,
          alias($._constraint_expression, $.linear_expression),
          optional(seq($.comparison_operator, $._numeric_value)),
        ),
      ),

    // A left-hand side with at least one variable term or quadratic block.
    _standard_lhs: ($) =>
      seq(
        optional(sign()),
        repeat(seq(alias($._lhs_constant, $.term), sign())),
        choice(alias($._variable_term, $.term), $.quadratic_block),
        repeat(seq(sign(), $._constraint_item)),
      ),

    _lhs_constant: ($) => constant($),

    // Constants only: `-4`, or (folded upstream) `2 + 3`.
    _constant_bound: ($) =>
      seq(optional(sign()), constant($), repeat(seq(sign(), constant($)))),

    // The expression of a flipped or ranged constraint extends as far as it
    // can: a sign continues it (prec.right) and a number followed by a
    // variable is that variable's coefficient (the negative precedence of
    // `_constant_term`), so `3 <= 2 x + y <= 9` is one ranged constraint.
    _constraint_expression: ($) => prec.right(signed($._constraint_item)),

    _constraint_item: ($) =>
      choice(
        alias($._variable_term, $.term),
        alias($._constant_term, $.term),
        $.quadratic_block,
      ),

    _variable_term: ($) => seq(optional(constant($)), $.identifier),

    _constant_term: ($) => prec(-1, constant($)),

    _any_section: ($) =>
      choice(
        $.lazy_constraints_section,
        $.user_cuts_section,
        $.general_constraints_section,
        $.bounds_section,
        $.generals_section,
        $.integers_section,
        $.binaries_section,
        $.semi_continuous_section,
        $.sos_section,
      ),

    lazy_constraints_section: ($) =>
      seq($.lazy_constraints_keyword, repeat($.constraint)),

    user_cuts_section: ($) => seq($.user_cuts_keyword, repeat($.constraint)),

    general_constraints_section: ($) =>
      seq(
        choice(
          $.general_constraints_keyword,
          alias($._genconstrs_keyword, $.general_constraints_keyword),
        ),
        repeat($.general_constraint),
      ),

    // Gurobi: `name: r = MAX ( x , y , 3 )`
    general_constraint: ($) =>
      seq(
        optional(
          seq(field("name", alias($.identifier, $.constraint_name)), ":"),
        ),
        field("resultant", $.identifier),
        "=",
        field("function", alias($.identifier, $.function_name)),
        "(",
        $._general_argument,
        repeat(seq(",", $._general_argument)),
        ")",
      ),

    _general_argument: ($) => choice($.identifier, $._numeric_value),

    bounds_section: ($) => seq($.bounds_keyword, repeat($.bound_declaration)),

    bound_declaration: ($) =>
      choice(
        // `x free` or `x <= hi`
        seq(
          $.identifier,
          choice($.free_keyword, seq($.comparison_operator, $._numeric_value)),
        ),
        // `lo <= x` or ranged `lo <= x <= hi`
        seq(
          $._numeric_value,
          $.comparison_operator,
          $.identifier,
          optional(seq($.comparison_operator, $._numeric_value)),
        ),
      ),

    generals_section: ($) => seq($.generals_keyword, repeat($.identifier)),

    integers_section: ($) => seq($.integers_keyword, repeat($.identifier)),

    binaries_section: ($) => seq($.binaries_keyword, repeat($.identifier)),

    semi_continuous_section: ($) =>
      seq($.semi_continuous_keyword, repeat($.identifier)),

    sos_section: ($) => seq($.sos_keyword, repeat($._sos_item)),

    _sos_item: ($) => choice($.sos_constraint_header, $.sos_entry),

    // `s1: S1 ::` opens an SOS set; the `name: weight` entries that follow
    // belong to it.
    sos_constraint_header: ($) =>
      seq(
        field("name", alias($.identifier, $.sos_name)),
        ":",
        $.sos_type,
        "::",
      ),

    sos_entry: ($) => seq($.identifier, ":", $._numeric_value),
  },
});
