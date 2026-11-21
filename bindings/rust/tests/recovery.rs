//! Error recovery: broken and truncated files must still yield a tree that
//! spans the input, flags the damage, and keeps the sound parts parseable.
//!
//! `tree-sitter test` only checks that an `:error` case has an error, so the
//! recovered shape is pinned here.

use tree_sitter::{Node, Parser};

fn parse(source: &str) -> tree_sitter::Tree {
    let mut parser = Parser::new();
    parser
        .set_language(&crate::LANGUAGE.into())
        .expect("Error loading LP parser");
    parser.parse(source, None).expect("parse returned no tree")
}

fn contains_kind(node: Node<'_>, kind: &str) -> bool {
    if node.kind() == kind {
        return true;
    }
    let mut cursor = node.walk();
    let found = node.children(&mut cursor).any(|c| contains_kind(c, kind));
    found
}

fn count_kind(node: Node<'_>, kind: &str) -> usize {
    let mut cursor = node.walk();
    let below: usize = node
        .children(&mut cursor)
        .map(|c| count_kind(c, kind))
        .sum();
    below + usize::from(node.kind() == kind)
}

/// A file with `kind` nodes surviving despite damage elsewhere.
struct Survivor {
    kind: &'static str,
    at_least: usize,
}

struct Case {
    name: &'static str,
    source: &'static str,
    /// The recovered root is `source_file` rather than a bare `ERROR`.
    rooted: bool,
    survivors: &'static [Survivor],
}

const fn keep(kind: &'static str, at_least: usize) -> Survivor {
    Survivor { kind, at_least }
}

const CASES: &[Case] = &[
    Case {
        name: "missing right-hand side keeps the next constraint",
        source: "min\n obj: x\nst\n c1: x >=\n c2: x <= 4\nend\n",
        rooted: true,
        survivors: &[keep("constraint", 2), keep("end_marker", 1)],
    },
    Case {
        name: "missing colon keeps the next constraint",
        source: "min\n obj: x\nst\n c1 x >= 1\n c2: x <= 4\nend\n",
        rooted: true,
        survivors: &[keep("constraint", 2), keep("end_marker", 1)],
    },
    Case {
        name: "stray operator keeps the next constraint",
        source: "min\n obj: x + y\nst\n c1: x + >= 1\n c2: x <= 4\nend\n",
        rooted: true,
        survivors: &[keep("constraint", 2), keep("end_marker", 1)],
    },
    Case {
        name: "garbage keeps the next constraint",
        source: "min\n obj: x\nst\n c1: x >= 1 \u{a7}\u{a7}\n c2: x <= 9\nend\n",
        rooted: true,
        survivors: &[keep("constraint", 2), keep("end_marker", 1)],
    },
    Case {
        name: "non-ASCII letter is reported but later sections survive",
        source: "min\n obj: caf\u{e9}\nst\n c1: caf\u{e9} >= 1\nbounds\n x <= 4\nend\n",
        rooted: true,
        survivors: &[keep("constraints_section", 1), keep("end_marker", 1)],
    },
    Case {
        name: "unterminated block comment hides nothing before it, keeps sections after it",
        source: "min\n obj: x\nst\n c1: x >= 1\n\\* never closed\nbounds\n x <= 4\nend\n",
        rooted: true,
        survivors: &[keep("constraint", 1), keep("bounds_section", 1)],
    },
    Case {
        name: "broken constraint keeps later sections",
        source: "min\n obj: x + y\nst\n c1: x + y >=\nbounds\n 0 <= x <= 4\ngenerals\n x\nend\n",
        rooted: true,
        survivors: &[
            keep("bounds_section", 1),
            keep("generals_section", 1),
            keep("end_marker", 1),
        ],
    },
    Case {
        name: "SOS entry without a weight keeps its siblings",
        source: "min\n obj: x + y\nst\n c1: x + y >= 1\nsos\n s1: S1 ::\n x :\n y : 2\nend\n",
        rooted: true,
        survivors: &[keep("sos_entry", 2), keep("end_marker", 1)],
    },
    Case {
        name: "general constraint missing a parenthesis keeps the next one",
        source: "min\n obj: x\nst\n c1: x >= 1\ngeneral constraints\n g1: r = MAX ( x , y\n g2: s = ABS ( x )\nend\n",
        rooted: true,
        survivors: &[keep("general_constraint", 2), keep("end_marker", 1)],
    },
    Case {
        name: "unterminated quadratic block keeps the constraints",
        source: "min\n obj: x + [ x ^ 2\nst\n c1: x >= 1\nend\n",
        rooted: true,
        survivors: &[keep("constraint", 1), keep("end_marker", 1)],
    },
    Case {
        name: "cut after the operator keeps the finished constraints",
        source: "min\n obj: x\nst\n c1: x >= 1\n c2: x <=",
        rooted: true,
        survivors: &[keep("constraint", 2)],
    },
    Case {
        name: "cut mid bound keeps earlier bounds",
        source: "min\n obj: x\nst\n c1: x >= 1\nbounds\n 0 <= x <= 4\n 0 <= y <=",
        rooted: true,
        survivors: &[keep("bound_declaration", 2)],
    },
    Case {
        name: "cut mid objective still reaches the constraints section",
        source: "min\n obj: 3 x + 2",
        rooted: true,
        survivors: &[keep("named_objective", 1), keep("constraints_section", 1)],
    },
    Case {
        name: "cut inside a block comment keeps what precedes it",
        source: "min\n obj: x\nst\n c1: x >= 1\n\\* an open",
        rooted: true,
        survivors: &[keep("constraint", 1)],
    },
    Case {
        name: "cut mid SOS header keeps the constraints",
        source: "min\n obj: x\nst\n c1: x >= 1\nsos\n s1: S1 :",
        rooted: true,
        survivors: &[keep("constraint", 1), keep("sos_section", 1)],
    },
    // The whole file fails to be a `source_file`: the root is a bare ERROR.
    Case {
        name: "misspelt sense leaves a bare ERROR root",
        source: "maximze\n obj: x\nst\n c1: x <= 1\nend\n",
        rooted: false,
        survivors: &[],
    },
    Case {
        name: "empty input leaves a bare ERROR root",
        source: "",
        rooted: false,
        survivors: &[],
    },
    Case {
        name: "a lone sense leaves a bare ERROR root",
        source: "min",
        rooted: false,
        survivors: &[],
    },
];

#[test]
fn damaged_files_recover_around_the_damage() {
    for case in CASES {
        let tree = parse(case.source);
        let root = tree.root_node();
        assert!(root.has_error(), "{}: expected a syntax error", case.name);
        assert_eq!(
            root.kind() == "source_file",
            case.rooted,
            "{}: unexpected root {}",
            case.name,
            root.to_sexp()
        );
        for survivor in case.survivors {
            let found = count_kind(root, survivor.kind);
            assert!(
                found >= survivor.at_least,
                "{}: expected at least {} `{}`, found {found} in {}",
                case.name,
                survivor.at_least,
                survivor.kind,
                root.to_sexp()
            );
        }
    }
}

#[test]
fn recovered_tree_spans_the_input() {
    for case in CASES {
        let tree = parse(case.source);
        let root = tree.root_node();
        assert!(
            root.end_byte() <= case.source.len(),
            "{}: root ends past the input",
            case.name
        );
        // Everything after the first byte of content is inside the root.
        assert!(
            root.end_byte() >= case.source.trim_end().len(),
            "{}: root ({}) stops before the input ({})",
            case.name,
            root.end_byte(),
            case.source.trim_end().len()
        );
    }
}

#[test]
fn pinned_recovery_for_a_missing_right_hand_side() {
    let tree = parse("min\n obj: x\nst\n c1: x >=\n c2: x <= 4\nend\n");
    assert_eq!(
        tree.root_node().to_sexp(),
        "(source_file (sense) (objectives_section (named_objective name: (objective_name) \
         (linear_expression (term (identifier))))) (constraints_section (subject_to_keyword) \
         (constraint name: (constraint_name) (linear_expression (term (identifier))) \
         (comparison_operator) (MISSING number)) (constraint name: (constraint_name) \
         (linear_expression (term (identifier))) (comparison_operator) (number))) (end_marker))"
    );
}

#[test]
fn truncated_prefixes_of_a_valid_file_never_lose_the_root_span() {
    let full = "min\n obj: 3 x + 2 y\nst\n c1: x + y >= 1\n c2: -5 <= x - y <= 5\nbounds\n 0 <= x <= 4\n y free\ngenerals\n x\nend\n";
    for cut in 0..=full.len() {
        if !full.is_char_boundary(cut) {
            continue;
        }
        let prefix = &full[..cut];
        let tree = parse(prefix);
        let root = tree.root_node();
        assert!(
            root.end_byte() <= prefix.len(),
            "cut {cut}: root ends past the input"
        );
        if !root.has_error() {
            assert!(contains_kind(root, "sense") || prefix.trim().is_empty());
        }
    }
}
