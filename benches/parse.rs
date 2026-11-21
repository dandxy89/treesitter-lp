//! Parse throughput over a generated 50 000-row model, plus an incremental
//! reparse after a one-character edit.
//!
//! Run with `cargo bench`; `cargo bench --no-run` only builds it.

// `criterion_group!` generates an undocumented public function.
#![allow(missing_docs)]

use std::fmt::Write as _;
use std::hint::black_box;

use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use tree_sitter::{InputEdit, Parser, Point};

const ROWS: usize = 50_000;
const COLUMNS: usize = 5_000;

/// A deterministic model with `ROWS` constraints over `COLUMNS` variables.
fn generate() -> String {
    let mut s = String::with_capacity(ROWS * 48);
    s.push_str("Minimize\n obj:");
    for c in 0..COLUMNS {
        write!(s, " + {} x{c}", c % 9 + 1).expect("writing to a String cannot fail");
    }
    s.push_str("\nSubject To\n");
    for r in 0..ROWS {
        let (a, b, c) = (r % COLUMNS, (r * 7 + 1) % COLUMNS, (r * 13 + 2) % COLUMNS);
        writeln!(
            s,
            " row{r}: {} x{a} + {} x{b} - x{c} >= {}",
            r % 5 + 1,
            r % 3 + 1,
            r % 40
        )
        .expect("writing to a String cannot fail");
    }
    s.push_str("Bounds\n");
    for c in 0..COLUMNS {
        writeln!(s, " 0 <= x{c} <= {}", c % 90 + 10).expect("writing to a String cannot fail");
    }
    s.push_str("End\n");
    s
}

fn parser() -> Parser {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_lp::LANGUAGE.into())
        .expect("Error loading LP parser");
    parser
}

fn bench_parse(c: &mut Criterion) {
    let source = generate();
    debug_assert!(source.lines().count() > ROWS);
    let mut parser = parser();
    let check = parser.parse(&source, None).expect("parse returned no tree");
    assert!(
        !check.root_node().has_error(),
        "the generated benchmark model must parse cleanly"
    );

    let mut group = c.benchmark_group("parse");
    group.sample_size(10);
    group.throughput(Throughput::Bytes(source.len() as u64));
    group.bench_function("full_50k_rows", |b| {
        b.iter(|| {
            black_box(
                parser
                    .parse(black_box(&source), None)
                    .expect("parse returned no tree"),
            )
        });
    });

    // Change the digit of the first row's right-hand side and reparse.
    let at = source.find(" >= ").expect("model has a constraint") + 4;
    let mut edited = source.clone();
    edited.replace_range(at..=at, "9");
    let row = source[..at].matches('\n').count();
    let column = at - source[..at].rfind('\n').map_or(0, |nl| nl + 1);
    let edit = InputEdit {
        start_byte: at,
        old_end_byte: at + 1,
        new_end_byte: at + 1,
        start_position: Point::new(row, column),
        old_end_position: Point::new(row, column + 1),
        new_end_position: Point::new(row, column + 1),
    };
    group.bench_function("incremental_one_char_edit", |b| {
        b.iter_batched(
            || {
                let mut old = check.clone();
                old.edit(&edit);
                old
            },
            |old| {
                black_box(
                    parser
                        .parse(black_box(&edited), Some(&old))
                        .expect("parse returned no tree"),
                )
            },
            criterion::BatchSize::LargeInput,
        );
    });
    group.finish();
}

criterion_group!(benches, bench_parse);
criterion_main!(benches);
