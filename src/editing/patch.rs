/// Result of applying a command
pub struct Patch {
    pub changed: Vec<std::ops::Range<usize>>,
    pub new_selection: std::ops::Range<usize>,
    pub version: u64,
}
