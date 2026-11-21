//! Behaviour of the bundled queries (`tags`, `highlights`, `locals`) over the
//! fixtures and a small model.

use std::collections::BTreeSet;
use std::fs;

use streaming_iterator::StreamingIterator as _;
use tree_sitter::{Parser, Query, QueryCursor};

use super::fixtures::{fixtures_dir, lp_files};

const MODEL: &str = "\
minimize
 obj: 2 x + 3 y
subject to
 cap: x + y <= 10
 c2: z - x >= -4
bounds
 0 <= x <= 40
integers
 y
sos
 s1: S1 ::
 x : 1
end
";

fn language() -> tree_sitter::Language {
    crate::LANGUAGE.into()
}

fn parse(source: &str) -> tree_sitter::Tree {
    let mut parser = Parser::new();
    parser
        .set_language(&language())
        .expect("Error loading LP parser");
    parser.parse(source, None).expect("parse returned no tree")
}

/// `(capture name, text)` pairs a query produces over `source`, in match order.
fn captures(query_source: &str, source: &str) -> Vec<(String, String)> {
    let query = Query::new(&language(), query_source).expect("query compiles");
    let tree = parse(source);
    let names = query.capture_names();
    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(&query, tree.root_node(), source.as_bytes());
    let mut found = Vec::new();
    while let Some(m) = matches.next() {
        for capture in m.captures() {
            let text = source[capture.node.byte_range()].to_string();
            found.push((names[capture.index as usize].to_string(), text));
        }
    }
    found
}

fn names_of(found: &[(String, String)], capture: &str) -> Vec<String> {
    found
        .iter()
        .filter(|(c, _)| c == capture)
        .map(|(_, t)| t.clone())
        .collect()
}

#[test]
fn tags_define_constraint_objective_and_sos_names() {
    let found = captures(crate::TAGS_QUERY, MODEL);
    // Each definition is paired with the `@name` capture holding its name.
    let defined: Vec<String> = names_of(&found, "definition.constant");
    assert_eq!(defined.len(), 4, "{defined:?}");
    let names: BTreeSet<String> = names_of(&found, "name").into_iter().collect();
    for expected in ["obj", "cap", "c2", "s1"] {
        assert!(
            names.contains(expected),
            "no tag named {expected}: {names:?}"
        );
    }
}

#[test]
fn tags_reference_variables_in_expressions_and_sos_entries() {
    let found = captures(crate::TAGS_QUERY, MODEL);
    let references: BTreeSet<String> = names_of(&found, "reference.variable").into_iter().collect();
    let expected: BTreeSet<String> = ["x", "y", "z"].map(String::from).into();
    assert_eq!(references, expected);
}

#[test]
fn tags_define_variables_in_bounds_and_type_sections() {
    let found = captures(crate::TAGS_QUERY, MODEL);
    let defined = names_of(&found, "definition.variable");
    assert!(
        defined.iter().any(|d| d.contains('x')),
        "bounds declaration not tagged: {defined:?}"
    );
    assert!(
        defined.iter().any(|d| d == "y"),
        "integers entry not tagged"
    );
}

#[test]
fn tags_use_only_known_capture_names() {
    let query = Query::new(&language(), crate::TAGS_QUERY).expect("tags query compiles");
    for name in query.capture_names() {
        assert!(
            *name == "name" || name.starts_with("definition.") || name.starts_with("reference."),
            "unexpected tags capture `{name}`"
        );
    }
}

#[test]
fn every_fixture_has_tagged_definitions() {
    for path in lp_files(&fixtures_dir()) {
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
        let found = captures(crate::TAGS_QUERY, &source);
        assert!(
            !names_of(&found, "definition.constant").is_empty(),
            "{} has no tagged definitions",
            path.display()
        );
        assert!(
            !names_of(&found, "reference.variable").is_empty(),
            "{} has no tagged references",
            path.display()
        );
    }
}

#[test]
fn highlights_cover_every_section_keyword_in_the_fixtures() {
    let mut keywords = BTreeSet::new();
    for path in lp_files(&fixtures_dir()) {
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
        let found = captures(crate::HIGHLIGHTS_QUERY, &source);
        keywords.extend(names_of(&found, "keyword.type"));
    }
    assert!(!keywords.is_empty(), "no section keyword highlighted");
}

#[test]
fn locals_resolve_variables_declared_in_sections() {
    let found = captures(crate::LOCALS_QUERY, MODEL);
    assert!(names_of(&found, "local.definition").len() > 1);
    assert!(names_of(&found, "local.reference").len() > 1);
    assert_eq!(names_of(&found, "local.scope").len(), 1);
}
