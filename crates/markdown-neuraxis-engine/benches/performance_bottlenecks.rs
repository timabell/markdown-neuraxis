use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use markdown_neuraxis_engine::editing::Cmd;
use markdown_neuraxis_engine::editing::document::Document;
use std::time::Instant;
mod common;

// Benchmark the specific performance bottlenecks identified in investigation

// Critical Bottleneck: Tree-sitter parsing overhead during document cloning
fn bench_tree_sitter_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("tree_sitter_parsing");

    for size_kb in [1, 10, 100, 1000] {
        let content = common::generate_complex_markdown(size_kb / 2, 4);

        group.throughput(Throughput::Bytes(content.len() as u64));
        group.bench_with_input(
            BenchmarkId::new("parse_document", size_kb),
            &content,
            |b, content| {
                b.iter_custom(|iters| {
                    let mut total_duration = std::time::Duration::new(0, 0);

                    for _ in 0..iters {
                        let start = Instant::now();

                        // This is the expensive operation happening in Document::clone()
                        let document =
                            std::hint::black_box(Document::from_bytes(content.as_bytes()).unwrap());

                        total_duration += start.elapsed();
                        std::hint::black_box(document);
                    }

                    total_duration
                });
            },
        );
    }

    group.finish();
}

// Critical Bottleneck: Anchor system overhead on every edit
fn bench_anchor_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("anchor_operations");

    let content = common::generate_complex_markdown(20, 3);
    let document = Document::from_bytes(content.as_bytes()).unwrap();

    // Create snapshot to simulate realistic usage
    let _snapshot = document.snapshot();

    group.bench_function("anchor_rebinding_on_edit", |b| {
        b.iter_custom(|iters| {
            let mut total_duration = std::time::Duration::new(0, 0);

            for i in 0..iters {
                let mut doc_copy = document.clone();

                let start = Instant::now();

                // Apply an edit that triggers anchor rebinding
                let edit_position = (i % 1000) as usize + 100;
                let command = Cmd::InsertText {
                    at: edit_position,
                    text: "X".to_string(),
                };

                std::hint::black_box(doc_copy.apply(command));

                total_duration += start.elapsed();
                std::hint::black_box(doc_copy);
            }

            total_duration
        });
    });

    group.finish();
}

// Critical Bottleneck: String conversion overhead
fn bench_string_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("string_operations");

    for size_kb in [1, 10, 100, 1000] {
        let content = common::generate_complex_markdown(size_kb / 2, 3);
        let document = Document::from_bytes(content.as_bytes()).unwrap();

        group.throughput(Throughput::Bytes(content.len() as u64));
        group.bench_with_input(
            BenchmarkId::new("rope_to_string", size_kb),
            &document,
            |b, doc| {
                b.iter(|| {
                    // This happens frequently throughout the codebase
                    let text = std::hint::black_box(doc.text());
                    std::hint::black_box(text);
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("buffer_to_string", size_kb),
            &content,
            |b, content| {
                b.iter(|| {
                    // Simulate the buffer.to_string() calls in document operations
                    let doc = Document::from_bytes(content.as_bytes()).unwrap();
                    let text = std::hint::black_box(doc.text());
                    std::hint::black_box(text);
                });
            },
        );
    }

    group.finish();
}

// Critical Bottleneck: Full document snapshot creation
fn bench_snapshot_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("snapshot_overhead");

    for size_kb in [1, 10, 100, 1000] {
        let content = common::generate_complex_markdown(size_kb / 2, 4);
        let document = Document::from_bytes(content.as_bytes()).unwrap();

        group.throughput(Throughput::Bytes(content.len() as u64));
        group.bench_with_input(
            BenchmarkId::new("full_snapshot_creation", size_kb),
            &document,
            |b, doc| {
                b.iter(|| {
                    let snapshot = std::hint::black_box(doc.snapshot());
                    std::hint::black_box(snapshot);
                });
            },
        );

        // Benchmark the specific tree traversal that happens in snapshot creation
        group.bench_with_input(
            BenchmarkId::new("tree_traversal_overhead", size_kb),
            &document,
            |b, doc| {
                b.iter(|| {
                    let snapshot = std::hint::black_box(doc.snapshot());
                    let blocks = std::hint::black_box(&snapshot.blocks);
                    // Simulate accessing all blocks (common UI pattern)
                    let count = blocks.len();
                    std::hint::black_box(count);
                });
            },
        );
    }

    group.finish();
}

// Simulate realistic editing scenarios that trigger cascading operations
fn bench_editing_cascades(c: &mut Criterion) {
    let mut group = c.benchmark_group("editing_cascades");

    let content = common::generate_editing_sequence_document();

    group.bench_function("edit_clone_snapshot_cascade", |b| {
        b.iter_custom(|iters| {
            let mut total_duration = std::time::Duration::new(0, 0);

            for i in 0..iters {
                let mut document = Document::from_bytes(content.as_bytes()).unwrap();

                let start = Instant::now();

                // Simulate the current cascade that happens on each edit:

                // 1. Apply edit
                let command = Cmd::InsertText {
                    at: (i % 100) as usize + 50,
                    text: "edit".to_string(),
                };
                std::hint::black_box(document.apply(command));

                // 2. Clone document for UI (major bottleneck)
                let ui_document = std::hint::black_box(document.clone());

                // 3. Create snapshot (another bottleneck)
                let snapshot = std::hint::black_box(document.snapshot());

                // 4. Clone snapshot for UI components
                let ui_snapshot = std::hint::black_box(snapshot.clone());

                total_duration += start.elapsed();
                std::hint::black_box((ui_document, ui_snapshot));
            }

            total_duration
        });
    });

    group.finish();
}

// Benchmark file I/O patterns that contribute to UI delays
fn bench_io_patterns(c: &mut Criterion) {
    let mut group = c.benchmark_group("io_patterns");

    let content = common::generate_complex_markdown(10, 3);

    group.bench_function("auto_save_simulation", |b| {
        use std::fs;
        use tempfile::NamedTempFile;

        b.iter_custom(|iters| {
            let mut total_duration = std::time::Duration::new(0, 0);

            for _ in 0..iters {
                let temp_file = NamedTempFile::new().unwrap();
                let mut document = Document::from_bytes(content.as_bytes()).unwrap();

                let start = Instant::now();

                // Simulate auto-save on keystroke pattern
                let command = Cmd::InsertText {
                    at: 100,
                    text: "x".to_string(),
                };
                document.apply(command);

                // Convert to string (expensive)
                let text = document.text();

                // Write to file (blocking I/O)
                fs::write(temp_file.path(), &text).unwrap();

                total_duration += start.elapsed();
                std::hint::black_box(document);
            }

            total_duration
        });
    });

    group.finish();
}

criterion_group!(
    performance_bottlenecks,
    bench_tree_sitter_parsing,
    bench_anchor_operations,
    bench_string_operations,
    bench_snapshot_overhead,
    bench_editing_cascades,
    bench_io_patterns
);

criterion_main!(performance_bottlenecks);
