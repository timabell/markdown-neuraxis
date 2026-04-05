use criterion::{Criterion, criterion_group, criterion_main};
use markdown_neuraxis_engine::editing::Document;
mod common;

fn bench_anchor_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("anchors");
    group.sample_size(10);

    let content = common::generate_markdown_content(100);

    // Benchmark anchor creation by measuring Document::from_bytes which creates anchors automatically
    group.bench_function("create_anchors", |b| {
        b.iter(|| {
            let doc = Document::from_bytes(std::hint::black_box(content.as_bytes())).unwrap();
            std::hint::black_box(&doc);
        });
    });

    group.finish();
}

criterion_group!(benches, bench_anchor_operations);
criterion_main!(benches);
