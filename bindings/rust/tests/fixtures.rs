//! Every file under `test/fixtures/` must parse without `ERROR` or `MISSING`
//! nodes: the fixtures are hand-written in the style of CPLEX, Gurobi, `HiGHS`
//! and `PuLP` output and stand for files users really open.

use std::fs;
use std::path::{Path, PathBuf};

use tree_sitter::{Node, Parser};

/// All `.lp` files below `dir`, sorted for stable reporting.
pub fn lp_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(next) = pending.pop() {
        let entries = fs::read_dir(&next)
            .unwrap_or_else(|e| panic!("cannot read directory {}: {e}", next.display()));
        for entry in entries {
            let path = entry
                .unwrap_or_else(|e| panic!("cannot read entry in {}: {e}", next.display()))
                .path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|ext| ext == "lp") {
                files.push(path);
            }
        }
    }
    files.sort();
    files
}

/// The directory holding the fixtures.
pub fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("test/fixtures")
}

fn first_problem(node: Node<'_>) -> Option<Node<'_>> {
    if node.is_error() || node.is_missing() {
        return Some(node);
    }
    let mut cursor = node.walk();
    let found = node.children(&mut cursor).find_map(first_problem);
    found
}

#[test]
fn fixtures_exist_for_every_dialect() {
    let dir = fixtures_dir();
    for dialect in ["cplex", "gurobi", "highs", "pulp", "generated"] {
        assert!(
            !lp_files(&dir.join(dialect)).is_empty(),
            "no fixtures for {dialect}"
        );
    }
}

#[test]
fn every_fixture_parses_without_errors() {
    let mut parser = Parser::new();
    parser
        .set_language(&crate::LANGUAGE.into())
        .expect("Error loading LP parser");
    let files = lp_files(&fixtures_dir());
    assert!(
        files.len() >= 10,
        "expected fixtures, found {}",
        files.len()
    );
    for path in files {
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
        let tree = parser
            .parse(&source, None)
            .unwrap_or_else(|| panic!("no tree for {}", path.display()));
        let root = tree.root_node();
        if let Some(bad) = first_problem(root) {
            panic!(
                "{} has a syntax problem at {}: {}",
                path.display(),
                bad.start_position(),
                root.to_sexp()
            );
        }
        assert_eq!(
            root.end_byte(),
            source.len(),
            "{}: root does not span the input",
            path.display()
        );
    }
}
