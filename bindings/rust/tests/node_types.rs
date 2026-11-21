//! The frozen grammar surface: lp-lsp and the queries depend on these node
//! kinds and fields, so they may be added to but never renamed or removed.

use serde_json::Value;

/// `(kind, named, field names)` for every top-level entry of `node-types.json`.
fn node_types() -> Vec<(String, bool, Vec<String>)> {
    let json: Value = serde_json::from_str(crate::NODE_TYPES).expect("NODE_TYPES is valid JSON");
    json.as_array()
        .expect("NODE_TYPES is an array")
        .iter()
        .map(|entry| {
            let kind = entry["type"]
                .as_str()
                .expect("entry has a type")
                .to_string();
            let named = entry["named"].as_bool().expect("entry has `named`");
            // Leaf types have no `fields` entry at all.
            let fields = entry
                .get("fields")
                .and_then(Value::as_object)
                .map_or_else(Vec::new, |fields| fields.keys().cloned().collect());
            (kind, named, fields)
        })
        .collect()
}

const FROZEN_KINDS: &[&str] = &[
    "source_file",
    "sense",
    "multi_objectives_keyword",
    "objectives_section",
    "named_objective",
    "objective_name",
    "objective_attribute",
    "attribute_name",
    "constraints_section",
    "subject_to_keyword",
    "lazy_constraints_section",
    "lazy_constraints_keyword",
    "user_cuts_section",
    "user_cuts_keyword",
    "general_constraints_section",
    "general_constraints_keyword",
    "general_constraint",
    "function_name",
    "constraint",
    "constraint_name",
    "indicator",
    "linear_expression",
    "term",
    "quadratic_block",
    "quadratic_term",
    "comparison_operator",
    "bounds_section",
    "bounds_keyword",
    "bound_declaration",
    "free_keyword",
    "generals_section",
    "generals_keyword",
    "integers_section",
    "integers_keyword",
    "binaries_section",
    "binaries_keyword",
    "semi_continuous_section",
    "semi_continuous_keyword",
    "sos_section",
    "sos_keyword",
    "sos_constraint_header",
    "sos_name",
    "sos_type",
    "sos_entry",
    "end_marker",
    "identifier",
    "number",
    "infinity",
    "line_comment",
    "block_comment",
];

const FROZEN_FIELDS: &[&str] = &["function", "name"];

#[test]
fn frozen_node_kinds_are_named_node_types() {
    let types = node_types();
    assert!(!types.is_empty(), "no node types read from NODE_TYPES");
    for kind in FROZEN_KINDS {
        assert!(
            types.iter().any(|(k, named, _)| *named && k == kind),
            "frozen node kind `{kind}` is missing from node-types.json"
        );
    }
}

#[test]
fn frozen_fields_exist() {
    let types = node_types();
    for field in FROZEN_FIELDS {
        assert!(
            types
                .iter()
                .any(|(_, _, fields)| fields.iter().any(|f| f == field)),
            "frozen field `{field}` is missing from node-types.json"
        );
    }
}

#[test]
fn every_frozen_kind_resolves_in_the_language() {
    let language: tree_sitter::Language = crate::LANGUAGE.into();
    for kind in FROZEN_KINDS {
        assert_ne!(
            language.id_for_node_kind(kind, true),
            0,
            "language has no named node kind `{kind}`"
        );
    }
    for field in FROZEN_FIELDS {
        assert!(
            language.field_id_for_name(field).is_some(),
            "language has no field `{field}`"
        );
    }
}
