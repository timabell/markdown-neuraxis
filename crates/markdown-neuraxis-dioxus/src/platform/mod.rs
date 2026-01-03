//! Platform-specific functionality
//!
//! This module provides abstractions for platform-specific operations,
//! primarily Android permissions and folder picker handling.

#[cfg(target_os = "android")]
mod android;

#[cfg(target_os = "android")]
pub use android::*;

/// Result of a storage permission check
#[derive(Debug, Clone, PartialEq)]
pub enum StoragePermissionStatus {
    /// Permission is granted, can access external storage
    Granted,
    /// Permission denied, need to request it
    Denied,
    /// Need to open settings for user to grant permission manually (Android 11+)
    NeedsSettingsIntent,
}

/// Check if the app has storage permission to read external folders.
///
/// On non-Android platforms, always returns `Granted`.
/// On Android, checks the appropriate permission based on API level.
#[cfg(not(target_os = "android"))]
pub fn check_storage_permission() -> StoragePermissionStatus {
    StoragePermissionStatus::Granted
}

/// Request storage permission.
///
/// On non-Android platforms, this is a no-op.
/// On Android 10 and below, requests READ_EXTERNAL_STORAGE.
/// On Android 11+, opens the "All files access" settings page.
///
/// Returns `true` if the request was initiated successfully.
#[cfg(not(target_os = "android"))]
pub fn request_storage_permission() -> bool {
    true
}

// ============================================================================
// Folder Picker (see ADR-0010)
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
