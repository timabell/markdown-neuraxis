//! Platform-specific functionality
//!
//! This module provides abstractions for platform-specific operations,
//! including folder picker handling and SAF-based I/O for Android.
//!
//! On Android, uses Storage Access Framework (SAF) for folder access,
//! which doesn't require any special permissions - see ADR-0011.

#[cfg(target_os = "android")]
mod android;

#[cfg(target_os = "android")]
pub use android::*;

// ============================================================================
// Folder Picker (see ADR-0010, ADR-0011)
// These stubs are only used on Android; on desktop they exist for API symmetry.
// ============================================================================

/// Launch the native folder picker.
///
/// On non-Android platforms, this is a no-op that returns `false`.
/// On Android, launches the system folder picker activity.
#[cfg(not(target_os = "android"))]
#[allow(dead_code)]
pub fn launch_folder_picker() -> bool {
    false
}

/// Check if the folder picker has completed.
///
/// On non-Android platforms, always returns `false`.
#[cfg(not(target_os = "android"))]
#[allow(dead_code)]
pub fn is_folder_picker_complete() -> bool {
    false
}

/// Get the result from the folder picker.
///
/// On non-Android platforms, always returns `None`.
#[cfg(not(target_os = "android"))]
#[allow(dead_code)]
pub fn get_folder_picker_result() -> Option<String> {
    None
}

/// Reset the folder picker state.
///
/// On non-Android platforms, this is a no-op.
#[cfg(not(target_os = "android"))]
#[allow(dead_code)]
pub fn reset_folder_picker() {}
