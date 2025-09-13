use criterion::{Criterion, black_box, criterion_group, criterion_main};
use markdown_neuraxis_engine::editing::document::Document;
mod common;

fn bench_document_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("document_creation");
    group.sample_size(20);

    let content = common::generate_markdown_content(100);
    group.bench_function("from_bytes", |b| {
        let bytes = content.as_bytes();
        b.iter(|| {
            let doc = Document::from_bytes(black_box(bytes)).unwrap();
            black_box(doc);
        });
    });

    group.finish();
}

fn bench_document_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("document_operations");
    group.sample_size(20);

    let content = common::generate_markdown_content(100);
    let doc = Document::from_bytes(content.as_bytes()).unwrap();

    group.bench_function("text", |b| {
        b.iter(|| {
            let text = doc.text();
            black_box(text);
        });
    });

    group.finish();
}

criterion_group!(benches, bench_document_creation, bench_document_operations);
criterion_main!(benches);
