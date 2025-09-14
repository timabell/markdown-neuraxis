use criterion::{Criterion, criterion_group, criterion_main};
use markdown_neuraxis_engine::editing::{anchors::create_anchors_from_tree, document::Document};
mod common;

fn bench_integration_scenarios(c: &mut Criterion) {
    let mut group = c.benchmark_group("integration");
    group.sample_size(10);

    group.bench_function("parse_anchor_snapshot", |b| {
        let content = common::generate_complex_markdown(8, 3);
        b.iter(|| {
            let mut doc = Document::from_bytes(std::hint::black_box(content.as_bytes())).unwrap();
            create_anchors_from_tree(&mut doc);
            let snapshot = doc.snapshot();
            std::hint::black_box(snapshot);
        });
    });

    group.finish();
}

criterion_group!(benches, bench_integration_scenarios);
criterion_main!(benches);
