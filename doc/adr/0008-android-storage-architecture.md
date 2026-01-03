# ADR-0008: Android Storage Architecture

## Status
Accepted

## Context

During Android APK development, we encountered several storage-related challenges:

1. **Initial crash**: The app crashed with "capacity overflow" when calling `env::args()` on Android
2. **Tilde expansion failure**: Paths using `~` (like `~/.config/`) don't work on Android as there's no shell expansion
3. **Permission denied errors**: Attempting to write to `/storage/emulated/0/.config/` failed with permission errors
4. **User accessibility**: Need to balance Linux conventions with Android's storage model and user expectations

The app needs to:
- Store configuration files somewhere accessible to the app
- Store markdown notes where users can access them via file managers
- Work within Android's permission model
- Maintain some compatibility with Linux filesystem conventions where possible

## Decision

We will use a hybrid approach optimized for Android users:

1. **Configuration storage**: Use Android's internal app storage
   - Path: `/data/data/co.rustworkshop.MarkdownNeuraxisDioxus/files/.config/markdown-neuraxis/config.toml`
   - No special permissions required
   - Follows Linux convention within app's sandbox
   - Gets cleaned up on app uninstall

2. **Notes storage**: Use external Documents folder
   - Path: `/storage/emulated/0/Documents/markdown-neuraxis/`
   - Requires WRITE_EXTERNAL_STORAGE permission
   - User-accessible via any file manager
   - Persists after app uninstall
   - Follows Android convention for document storage

3. **Future enhancement**: Add folder picker UI
   - Allow users to choose custom locations
   - Store choice in config file
   - Default to Documents folder on first run

## Implementation Details

### Permission Requirements
Permissions are configured in `Dioxus.toml`:
```toml
[bundle.android.permissions]
permissions = [
    "android.permission.READ_EXTERNAL_STORAGE",
    "android.permission.WRITE_EXTERNAL_STORAGE"
]
```

These get injected into the Android manifest during the build process.

### Path Resolution
```rust
#[cfg(target_os = "android")]
pub fn config_path() -> PathBuf {
    // Use app's internal storage for config
    PathBuf::from("/data/data/co.rustworkshop.MarkdownNeuraxisDioxus/files/.config/markdown-neuraxis/config.toml")
}

#[cfg(target_os = "android")]
fn default_notes_path() -> PathBuf {
    // Use external Documents for notes
    PathBuf::from("/storage/emulated/0/Documents/markdown-neuraxis")
}
```

### Error Handling
- If config directory doesn't exist, create it
- If Documents folder isn't writable, fail with clear error message
- Log all storage operations for debugging via Android logcat

## Consequences

### Positive
- Users can easily access their markdown files via any Android file manager
- Config follows Linux conventions within app sandbox
- Notes persist even if app is uninstalled
- Clear separation between app config and user data

### Negative
- Requires storage permissions (may concern privacy-conscious users)
- Config is not user-accessible without root
- Different paths than desktop Linux version
- Must handle permission denial gracefully

### Neutral
- Follows Android platform conventions over Linux conventions
- Similar to how Termux and other Linux-on-Android apps handle storage

## Alternatives Considered

1. **Everything in internal storage**: Would work without permissions but users couldn't access their files
2. **Everything in external storage**: Would expose config files and require permissions for basic operation
3. **Termux-style home directory**: Would be more Linux-like but less Android-native
4. **Scoped Storage API**: More modern but complex and doesn't fit our use case well

## References
- Android Storage Overview: https://developer.android.com/training/data-storage
- Android External Storage: https://developer.android.com/training/data-storage/shared
- Issue: Initial Android crashes due to env::args() and tilde expansion