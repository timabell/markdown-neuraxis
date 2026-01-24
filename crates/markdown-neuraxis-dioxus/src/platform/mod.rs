//! Platform-specific functionality
//!
//! This module provides abstractions for platform-specific operations.
//! With Android support moved to native Kotlin (see ADR-0010/0011),
//! this module now only contains desktop stubs.

/// Result of a storage permission check
#[derive(Debug, Clone, PartialEq)]
pub enum StoragePermissionStatus {
    /// Permission is granted, can access external storage
    Granted,
    /// Permission denied, need to request it
    Denied,
    /// Need to open settings for user to grant permission manually
    NeedsSettingsIntent,
}

/// Check if the app has storage permission to read external folders.
///
/// On desktop platforms, always returns `Granted`.
pub fn check_storage_permission() -> StoragePermissionStatus {
    StoragePermissionStatus::Granted
}

/// Request storage permission.
///
/// On desktop platforms, this is a no-op and always returns `true`.
pub fn request_storage_permission() -> bool {
    true
}
