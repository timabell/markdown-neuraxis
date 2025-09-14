use criterion::{Criterion, criterion_group, criterion_main};
use markdown_neuraxis_engine::editing::{anchors::create_anchors_from_tree, document::Document};
mod common;

fn bench_anchor_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("anchors");
    group.sample_size(10);

    let content = common::generate_markdown_content(100);
    let doc = Document::from_bytes(content.as_bytes()).unwrap();

    group.bench_function("create_anchors", |b| {
        let mut d = doc.clone();
        b.iter(|| {
            create_anchors_from_tree(std::hint::black_box(&mut d));
            std::hint::black_box(&d);
        });
    });

    group.finish();
}

criterion_group!(benches, bench_anchor_operations);
criterion_main!(benches);
