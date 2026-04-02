fn main() {
    // Watch CSS file for changes so cargo rebuilds when it's modified
    println!("cargo:rerun-if-changed=src/assets/solarized-light.css");
}
