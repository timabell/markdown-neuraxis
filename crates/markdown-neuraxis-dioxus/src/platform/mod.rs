//! Platform-specific functionality
//!
//! This module provides abstractions for platform-specific operations,
//! primarily Android permissions handling.

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
