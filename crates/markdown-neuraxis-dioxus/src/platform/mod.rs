//! Platform-specific functionality
//!
//! This module provides abstractions for platform-specific operations.
//! With Android support moved to native Kotlin (see ADR-0010/0011),
//! this module now only contains desktop functionality.

use std::path::PathBuf;

/// Opens a native folder picker dialog asynchronously.
/// Returns `Some(path)` if user selected a folder, `None` if cancelled.
#[must_use]
pub async fn pick_folder(start_dir: Option<&std::path::Path>) -> Option<PathBuf> {
    use rfd::AsyncFileDialog;
    let mut dialog = AsyncFileDialog::new();
    if let Some(dir) = start_dir {
        dialog = dialog.set_directory(dir);
    }
    dialog.pick_folder().await.map(|h| h.path().to_path_buf())
}
