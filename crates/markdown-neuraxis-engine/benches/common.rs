// Benchmark helper functions - Rust's dead code analysis doesn't understand
// that these are used by benchmark files in the same directory
// See: https://users.rust-lang.org/t/cargo-rustc-benches-awarnings/110111/2
#[allow(dead_code)]
pub fn generate_markdown_content(_size: usize) -> String {
    "# Title\n\n## Section\n\nParagraph.\n\n- Bullet\n  - Nested\n- Another\n\n```rust\nfn test() {}\n```\n".to_string()
}

#[allow(dead_code)]
pub fn generate_complex_markdown(_sections: usize, _depth: usize) -> String {
    "# Root\n\n## Sub1\n\n- Item 1\n  - Nested\n    - Deep\n\n### Sub2\n\nContent.\n\n- Item 2\n"
        .to_string()
}
