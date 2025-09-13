// Benchmark helper functions - Rust's dead code analysis doesn't understand
// that these are used by benchmark files in the same directory
// See: https://users.rust-lang.org/t/cargo-rustc-benches-awarnings/110111/2
#[allow(dead_code)]
pub fn generate_markdown_content(size: usize) -> String {
    let base = "# Title\n\n## Section\n\nParagraph with some content.\n\n- Bullet point\n  - Nested item\n- Another item\n\n```rust\nfn example() {\n    println!(\"Hello\");\n}\n```\n\n";
    base.repeat(size)
}

#[allow(dead_code)]
pub fn generate_complex_markdown(sections: usize, depth: usize) -> String {
    let mut content = String::new();

    for section in 0..sections {
        content.push_str(&format!("# Section {}\n\n", section));
        content.push_str(&generate_nested_content(depth, 2));
        content.push('\n');
    }

    content
}

#[allow(dead_code)]
fn generate_nested_content(remaining_depth: usize, current_level: usize) -> String {
    if remaining_depth == 0 {
        return String::new();
    }

    let mut content = String::new();
    let header_prefix = "#".repeat(current_level);

    content.push_str(&format!(
        "{} Subsection Level {}\n\n",
        header_prefix, current_level
    ));
    content.push_str("Some paragraph content with multiple sentences. This helps create realistic document structure for benchmarking.\n\n");

    // Add bullet points
    for i in 0..3 {
        let indent = "  ".repeat((current_level - 2).min(3));
        content.push_str(&format!(
            "{}* Item {} at level {}\n",
            indent, i, current_level
        ));
    }
    content.push('\n');

    // Add code block occasionally
    if current_level % 3 == 0 {
        content.push_str("```rust\nfn benchmark_function() {\n    // Example code\n    let value = 42;\n    println!(\"{}\", value);\n}\n```\n\n");
    }

    // Recurse to deeper levels
    if remaining_depth > 1 && current_level < 6 {
        content.push_str(&generate_nested_content(
            remaining_depth - 1,
            current_level + 1,
        ));
    }

    content
}

#[allow(dead_code)]
pub fn generate_large_document() -> String {
    generate_complex_markdown(50, 4)
}

#[allow(dead_code)]
pub fn generate_editing_sequence_document() -> String {
    generate_complex_markdown(20, 3)
}
