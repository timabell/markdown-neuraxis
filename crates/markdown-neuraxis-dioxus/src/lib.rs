use std::path::PathBuf;

pub mod platform;
pub mod ui;

/// Storage location for notes - either a filesystem path or SAF URI
#[derive(Clone, Debug)]
pub enum StorageLocation {
    /// Filesystem path (desktop platforms)
    Path(PathBuf),
    /// Content URI (Android SAF)
    Uri(String),
}
