use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use markdown_neuraxis_engine::editing::Cmd;
use markdown_neuraxis_engine::editing::document::Document;
use std::time::Instant;

// Generate realistic test data for UI pipeline benchmarks
fn generate_markdown_content(size_kb: usize) -> String {
    let base = r#"# Section Header

## Subsection

This is a paragraph with some content that makes it realistic for testing performance. 
It contains multiple sentences and enough text to be representative of real documents.

### Features

- Bullet point item
  - Nested bullet point
    - Deep nested item
- Another bullet point with more content
- Final bullet point

```rust
fn example_code() {
    let value = 42;
    println!("Value: {}", value);
    
    for i in 0..10 {
        process_item(i);
    }
}
```

More paragraph content follows the code block. This helps create a realistic document
structure that users would actually work with in practice.

"#;

    let target_bytes = size_kb * 1024;
    let mut content = String::new();
    let mut section_num = 0;

    while content.len() < target_bytes {
        content.push_str(&format!("# Document Section {}\n\n", section_num));
        content.push_str(base);
        content.push('\n');
        section_num += 1;
    }

    content
}

fn generate_editing_sequence() -> Vec<Cmd> {
    vec![
        Cmd::InsertText {
            at: 0,
            text: "New text at start\n".to_string(),
        },
        Cmd::InsertText {
            at: 100,
            text: "Inserted in middle\n".to_string(),
        },
        Cmd::InsertText {
            at: 200,
            text: "More text\n".to_string(),
        },
    ]
}

// Critical Benchmark 1: Document Cloning Overhead (Current Major Bottleneck)
fn bench_document_cloning(c: &mut Criterion) {
    let mut group = c.benchmark_group("document_cloning");

    for size_kb in [1, 10, 100, 1000] {
        let content = generate_markdown_content(size_kb);
        let document = Document::from_bytes(content.as_bytes()).unwrap();

        group.throughput(Throughput::Bytes(content.len() as u64));
        group.bench_with_input(
            BenchmarkId::new("clone_document", size_kb),
            &document,
            |b, doc| {
                b.iter(|| {
                    let cloned = black_box(doc.clone());
                    black_box(cloned);
                });
            },
        );
    }

    group.finish();
}

// Critical Benchmark 2: UI Component Render Pipeline
fn bench_component_rendering(c: &mut Criterion) {
    let mut group = c.benchmark_group("component_rendering");

    for size_kb in [1, 10, 100] {
        let content = generate_markdown_content(size_kb);
        let document = Document::from_bytes(content.as_bytes()).unwrap();
        let snapshot = document.snapshot();

        group.throughput(Throughput::Bytes(content.len() as u64));
        group.bench_with_input(
            BenchmarkId::new("render_document_content", size_kb),
            &(document, snapshot),
            |b, (doc, snap)| {
                b.iter(|| {
                    // Simulate component rendering overhead by accessing document data
                    let text_len = doc.text().len();
                    let block_count = snap.blocks.len();
                    let simulated_render = format!(
                        "Document with {} chars and {} blocks",
                        text_len, block_count
                    );
                    black_box(simulated_render);
                });
            },
        );
    }

    group.finish();
}

// Critical Benchmark 3: Edit Sequence Simulation (Typing Performance)
fn bench_edit_sequence(c: &mut Criterion) {
    let mut group = c.benchmark_group("edit_sequences");

    let content = generate_markdown_content(10); // 10KB document
    let commands = generate_editing_sequence();

    group.bench_function("keystroke_simulation", |b| {
        b.iter_custom(|iters| {
            let mut total_duration = std::time::Duration::new(0, 0);

            for _ in 0..iters {
                let mut document = Document::from_bytes(content.as_bytes()).unwrap();

                let start = Instant::now();

                // Simulate a sequence of keystrokes
                for command in &commands {
                    document.apply(black_box(command.clone()));

                    // Simulate UI component cloning (current bottleneck)
                    let _cloned_for_ui = black_box(document.clone());

                    // Simulate snapshot creation (current bottleneck)
                    let _snapshot = black_box(document.snapshot());
                }

                total_duration += start.elapsed();
                black_box(document);
            }

            total_duration
        });
    });

    group.finish();
}

// Critical Benchmark 4: Document Loading + Parsing + UI Pipeline
fn bench_full_pipeline(c: &mut Criterion) {
    let mut group = c.benchmark_group("full_pipeline");

    for size_kb in [1, 10, 100] {
        let content = generate_markdown_content(size_kb);

        group.throughput(Throughput::Bytes(content.len() as u64));
        group.bench_with_input(
            BenchmarkId::new("load_parse_render", size_kb),
            &content,
            |b, content| {
                b.iter_custom(|iters| {
                    let mut total_duration = std::time::Duration::new(0, 0);

                    for _ in 0..iters {
                        let start = Instant::now();

                        // 1. Parse document (tree-sitter)
                        let document = black_box(Document::from_bytes(content.as_bytes()).unwrap());

                        // 2. Create snapshot (current bottleneck)
                        let snapshot = black_box(document.snapshot());

                        // 3. Clone for UI (current major bottleneck)
                        let ui_document = black_box(document.clone());
                        let ui_snapshot = black_box(snapshot.clone());

                        // 4. Simulate component rendering
                        let text_len = ui_document.text().len();
                        let _simulated_render = format!("Document loaded with {} chars", text_len);

                        total_duration += start.elapsed();
                        black_box((ui_document, ui_snapshot));
                    }

                    total_duration
                });
            },
        );
    }

    group.finish();
}

// Memory Allocation Pressure Benchmark
fn bench_memory_pressure(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_allocation");

    let content = generate_markdown_content(50); // 50KB document
    let document = Document::from_bytes(content.as_bytes()).unwrap();

    group.bench_function("repeated_cloning", |b| {
        b.iter(|| {
            // Simulate the current pattern where every UI component clones
            let mut clones = Vec::new();
            for _ in 0..10 {
                clones.push(black_box(document.clone()));
            }
            black_box(clones);
        });
    });

    group.bench_function("string_conversions", |b| {
        b.iter(|| {
            // Simulate current string conversion overhead
            let text = black_box(document.text());
            let bytes = black_box(text.as_bytes());
            let back_to_string = black_box(String::from_utf8_lossy(bytes));
            black_box(back_to_string);
        });
    });

    group.finish();
}

// Snapshot Creation Overhead Benchmark
fn bench_snapshot_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("snapshot_creation");

    for size_kb in [1, 10, 100, 1000] {
        let content = generate_markdown_content(size_kb);
        let document = Document::from_bytes(content.as_bytes()).unwrap();

        group.throughput(Throughput::Bytes(content.len() as u64));
        group.bench_with_input(
            BenchmarkId::new("create_snapshot", size_kb),
            &document,
            |b, doc| {
                b.iter(|| {
                    let snapshot = black_box(doc.snapshot());
                    black_box(snapshot);
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    ui_pipeline_benches,
    bench_document_cloning,
    bench_component_rendering,
    bench_edit_sequence,
    bench_full_pipeline,
    bench_memory_pressure,
    bench_snapshot_creation
);

criterion_main!(ui_pipeline_benches);
