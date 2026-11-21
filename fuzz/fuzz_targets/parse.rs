//! Fuzz target `parse`.
//!
//! Invariants, for any byte string and any single edit of it (the last six
//! bytes of the input choose the edit):
//! 1. parsing never panics or hangs;
//! 2. the root node lies inside the input;
//! 3. reparsing the edited text incrementally gives the same S-expression as
//!    parsing it from scratch.
//!
//! Run with `cargo +nightly fuzz run parse` (from the repository root).

#![no_main]

use libfuzzer_sys::fuzz_target;
use tree_sitter::{InputEdit, Parser, Point};

/// Trailing bytes of the input that choose the edit; the rest is the text.
const PARAMS: usize = 6;

/// The `Point` of byte offset `at`.
fn point(text: &[u8], at: usize) -> Point {
    let before = &text[..at];
    let row = before.iter().filter(|&&b| b == b'\n').count();
    let line_start = before
        .iter()
        .rposition(|&b| b == b'\n')
        .map_or(0, |nl| nl + 1);
    Point::new(row, at - line_start)
}

fuzz_target!(|data: &[u8]| {
    // Bound the work per run: the target also reparses.
    if data.len() <= PARAMS || data.len() > 8192 {
        return;
    }
    let (text, params) = data.split_at(data.len() - PARAMS);
    let scale = |byte: u8, modulus: usize| usize::from(byte) * 257 % (modulus + 1);

    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_lp::LANGUAGE.into())
        .expect("Error loading LP parser");

    let mut tree = parser.parse(text, None).expect("parse returned no tree");
    assert!(tree.root_node().end_byte() <= text.len());

    // Replace `text[start..end]` with a slice copied from elsewhere in the text.
    let start = scale(params[0], text.len());
    let end = (start + scale(params[1], 64)).min(text.len());
    let from = scale(params[2], text.len());
    let to = (from + scale(params[3], 64)).min(text.len());
    let inserted = &text[from..to];

    let mut edited = text[..start].to_vec();
    edited.extend_from_slice(inserted);
    edited.extend_from_slice(&text[end..]);

    let new_end = start + inserted.len();
    tree.edit(&InputEdit {
        start_byte: start,
        old_end_byte: end,
        new_end_byte: new_end,
        start_position: point(text, start),
        old_end_position: point(text, end),
        new_end_position: point(&edited, new_end),
    });
    let incremental = parser
        .parse(&edited, Some(&tree))
        .expect("incremental parse returned no tree");
    let fresh = parser
        .parse(&edited, None)
        .expect("fresh parse returned no tree");
    assert!(incremental.root_node().end_byte() <= edited.len());
    assert_eq!(
        incremental.root_node().to_sexp(),
        fresh.root_node().to_sexp(),
        "incremental reparse differs from a fresh parse"
    );
});
