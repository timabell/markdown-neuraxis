use criterion::{Criterion, criterion_group, criterion_main};
use markdown_neuraxis_engine::editing::{commands::Cmd, document::Document};
mod common;

fn bench_command_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("commands");
    group.sample_size(10);

    let content = common::generate_markdown_content(100);
    let doc = Document::from_bytes(content.as_bytes()).unwrap();

    group.bench_function("insert_command", |b| {
        let mut d = doc.clone();
        b.iter(|| {
            let cmd = Cmd::InsertText {
                at: std::hint::black_box(50),
                text: std::hint::black_box("test".to_string()),
            };
            let patch = d.apply(cmd);
            std::hint::black_box(patch);
        });
    });

    group.finish();
}

criterion_group!(benches, bench_command_operations);
criterion_main!(benches);
