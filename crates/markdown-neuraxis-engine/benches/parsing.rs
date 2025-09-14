use criterion::{Criterion, criterion_group, criterion_main};
use pulldown_cmark::Parser;
mod common;

fn bench_pulldown_cmark_baseline(c: &mut Criterion) {
    let mut group = c.benchmark_group("parsing");
    group.sample_size(10);

    let content = common::generate_markdown_content(100);
    group.bench_function("pulldown_cmark", |b| {
        b.iter(|| {
            let parser = Parser::new(std::hint::black_box(&content));
            let events: Vec<_> = parser.collect();
            std::hint::black_box(events);
        });
    });

    group.finish();
}

criterion_group!(benches, bench_pulldown_cmark_baseline);
criterion_main!(benches);
