/// Generates one test function per .md file in tests/snapshots/ (recursively).
/// Input files are shared at workspace root; each crate stores its own .snap outputs.
fn main() {
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let dest = std::path::Path::new(&out_dir).join("snapshot_tests.rs");

    // Shared input directory at workspace root
    let input_dir = std::path::Path::new("../../tests/snapshots");
    let mut tests = Vec::new();
    collect_md_files(input_dir, input_dir, &mut tests);
    tests.sort();

    let mut code = String::from(
        r#"mod parse_snapshots {
    use super::snapshot_test;
"#,
    );

    for (test_name, rel_path) in &tests {
        code.push_str(&format!(
            r#"
    #[test]
    fn {test_name}() {{
        snapshot_test("{rel_path}");
    }}
"#
        ));
    }

    code.push_str("}\n");
    std::fs::write(&dest, code).unwrap();

    // Rerun if snapshots change
    println!("cargo::rerun-if-changed=../../tests/snapshots");
}

/// Recursively collect .md files under root, producing (test_name, relative_path) pairs.
/// test_name uses underscores for path separators (e.g., "blocks_heading_h1").
/// rel_path is the path relative to root (e.g., "blocks/heading_h1").
fn collect_md_files(
    dir: &std::path::Path,
    root: &std::path::Path,
    out: &mut Vec<(String, String)>,
) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut entries: Vec<_> = entries.filter_map(|e| e.ok()).collect();
    entries.sort_by_key(|e| e.path());

    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            collect_md_files(&path, root, out);
        } else if path.extension().is_some_and(|e| e == "md") {
            let rel = path.strip_prefix(root).unwrap();
            let rel_str = rel.with_extension("").to_str().unwrap().replace('\\', "/");
            // Test name: replace / and - with _ for valid Rust identifier
            let test_name = rel_str.replace(['/', '-'], "_");
            out.push((test_name, rel_str));
        }
    }
}
