use criterion::{Criterion, criterion_group, criterion_main};
use markdown_neuraxis_engine::editing::{anchors::create_anchors_from_tree, document::Document};
mod common;

fn bench_point_description(c: &mut Criterion) {
    let mut group = c.benchmark_group("point_description");
    group.sample_size(10);

    let content = common::generate_markdown_content(100);
    let mut doc = Document::from_bytes(content.as_bytes()).unwrap();
    create_anchors_from_tree(&mut doc);

    group.bench_function("describe_point", |b| {
        b.iter(|| {
            let point = doc.describe_point(std::hint::black_box(50));
            std::hint::black_box(point);
        });
    });

    group.finish();
}

criterion_group!(benches, bench_point_description);
criterion_main!(benches);
