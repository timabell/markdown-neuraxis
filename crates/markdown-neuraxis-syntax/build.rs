/// Generates one test function per .md file in src/snapshots/.
/// This gives us both DRY code and individual test names in the runner.
fn main() {
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let dest = std::path::Path::new(&out_dir).join("snapshot_tests.rs");

    let mut code = String::from(
        r#"mod parse_snapshots {
    use super::snapshot_test;
"#,
    );

    let mut entries: Vec<_> = std::fs::read_dir("src/snapshots")
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    entries.sort_by_key(|e| e.path());

    for entry in entries {
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "md") {
            let name = path.file_stem().unwrap().to_str().unwrap();
            code.push_str(&format!(
                r#"
    #[test]
    fn {name}() {{
        snapshot_test("{name}");
    }}
"#
            ));
        }
    }

    code.push_str("}\n");
    std::fs::write(&dest, code).unwrap();

    // Rerun if snapshots change
    println!("cargo::rerun-if-changed=src/snapshots");
}
