use criterion::{Criterion, criterion_group, criterion_main};
use markdown_neuraxis_engine::editing::{anchors::create_anchors_from_tree, document::Document};
mod common;

fn bench_snapshot_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("snapshots");
    group.sample_size(10);

    let content = common::generate_markdown_content(100);
    let mut doc = Document::from_bytes(content.as_bytes()).unwrap();
    create_anchors_from_tree(&mut doc);

    group.bench_function("snapshot", |b| {
        b.iter(|| {
            let snapshot = doc.snapshot();
            std::hint::black_box(snapshot);
        });
    });

    group.finish();
}

criterion_group!(benches, bench_snapshot_operations);
criterion_main!(benches);
