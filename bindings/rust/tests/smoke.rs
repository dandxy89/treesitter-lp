//! Stable-toolchain smoke test with the same invariants as the cargo-fuzz
//! target (`fuzz/fuzz_targets/parse.rs`), driven by a fixed-seed generator
//! over the fixtures: parsing never panics, the root lies inside the input,
//! and an incremental reparse after a random splice equals a fresh parse.

use std::fmt::Write as _;
use std::fs;

use tree_sitter::{InputEdit, Parser, Point};

use super::fixtures::{fixtures_dir, lp_files};

/// xorshift64*: deterministic, no dependency.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn below(&mut self, n: usize) -> usize {
        debug_assert!(n > 0, "empty range");
        let n = u64::try_from(n).expect("range fits in u64");
        usize::try_from(self.next() % n).expect("below a usize bound")
    }
}

fn bytecount(bytes: &[u8]) -> usize {
    bytes.split(|&b| b == b'\n').count() - 1
}

fn point(text: &[u8], at: usize) -> Point {
    let before = &text[..at];
    let row = bytecount(before);
    let line_start = before
        .iter()
        .rposition(|&b| b == b'\n')
        .map_or(0, |nl| nl + 1);
    Point::new(row, at - line_start)
}

const NOISE: &[&[u8]] = &[
    b"\n",
    b" ",
    b"+",
    b"-",
    b">=",
    b"<=",
    b":",
    b"::",
    b"[",
    b"]",
    b"\\* ",
    b"*\\",
    b"\\",
    b"st\n",
    b"bounds\n",
    b"end\n",
    b"\xc3\xa9",
    b"\xff",
    b"0",
    b"1e",
    b"inf",
    b"->",
];

fn check_one(parser: &mut Parser, rng: &mut Rng, text: &[u8]) {
    let mut tree = parser.parse(text, None).expect("parse returned no tree");
    assert!(tree.root_node().end_byte() <= text.len());

    let start = rng.below(text.len() + 1);
    let end = (start + rng.below(48)).min(text.len());
    let inserted: Vec<u8> = if rng.below(2) == 0 {
        NOISE[rng.below(NOISE.len())].to_vec()
    } else {
        let from = rng.below(text.len() + 1);
        text[from..(from + rng.below(48)).min(text.len())].to_vec()
    };

    let mut edited = text[..start].to_vec();
    edited.extend_from_slice(&inserted);
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
        "incremental reparse differs from a fresh parse after splicing {inserted:?} over {start}..{end} of {:?}",
        String::from_utf8_lossy(text)
    );
}

#[test]
fn incremental_reparse_equals_fresh_parse_on_fixtures() {
    let mut parser = Parser::new();
    parser
        .set_language(&crate::LANGUAGE.into())
        .expect("Error loading LP parser");
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    for path in lp_files(&fixtures_dir()) {
        let text =
            fs::read(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
        // The generated medium fixture is large: fewer rounds keep this quick.
        let rounds = if text.len() > 8_000 { 20 } else { 300 };
        for _ in 0..rounds {
            check_one(&mut parser, &mut rng, &text);
        }
    }
}

#[test]
fn random_bytes_never_panic_and_stay_inside_the_input() {
    let mut parser = Parser::new();
    parser
        .set_language(&crate::LANGUAGE.into())
        .expect("Error loading LP parser");
    let mut rng = Rng(42);
    for _ in 0..2_000 {
        let len = rng.below(200);
        let text: Vec<u8> = (0..len)
            .map(|_| match rng.below(4) {
                0 => NOISE[rng.below(NOISE.len())][0],
                1 => b"abxyz019 \n"[rng.below(10)],
                _ => (rng.next() & 0xff) as u8,
            })
            .collect();
        let tree = parser.parse(&text, None).expect("parse returned no tree");
        assert!(tree.root_node().end_byte() <= text.len());
    }
}

#[test]
fn deeply_repeated_input_parses_without_overflow() {
    let mut parser = Parser::new();
    parser
        .set_language(&crate::LANGUAGE.into())
        .expect("Error loading LP parser");
    let mut text = String::from("min\n obj: x\nst\n");
    for i in 0..5_000 {
        writeln!(text, " c{i}: x + [ x ^ 2 ] <= {i}").expect("writing to a String cannot fail");
    }
    let tree = parser.parse(&text, None).expect("parse returned no tree");
    assert!(!tree.root_node().has_error());
    let brackets = "[".repeat(10_000);
    let tree = parser
        .parse(format!("min\n obj: x {brackets}\nst\n c: x >= 1\n"), None)
        .expect("parse returned no tree");
    assert!(tree.root_node().end_byte() > 0);
}
