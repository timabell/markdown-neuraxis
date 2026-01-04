# ADR-0011: Android Storage Access Framework IO Abstraction

## Status
Accepted

## Context

ADR-0009 implemented a workaround using `MANAGE_EXTERNAL_STORAGE` permission to access user-selected folders on Android 11+. This approach has significant UX problems:

1. Users must navigate to Settings to manually enable "All files access"
2. This is a "special permission" that cannot be requested via the normal runtime dialog
3. Google Play Store has strict policies about this permission (intended for file managers/backup apps)
4. Other apps accessing user-selected folders don't require this

The root cause: ADR-0010's folder picker converts SAF content URIs to filesystem paths, then uses `std::fs` for file operations. Standard filesystem access requires `MANAGE_EXTERNAL_STORAGE` on Android 11+.

However, SAF's `ACTION_OPEN_DOCUMENT_TREE` grants **persistent, recursive access** to the selected folder tree. Apps that stay within SAF (using `ContentResolver` and `DocumentFile`) don't need any special permissions - the URI grant itself is the permission.

The app only accesses one user-selected folder, not arbitrary filesystem locations. This is exactly what SAF is designed for.

## Decision

We will refactor to use SAF properly by:

1. **Introducing an `IoProvider` trait** in the engine crate that abstracts file operations
2. **Implementing `StdFsProvider`** for desktop platforms using `std::fs`
3. **Implementing `SafProvider`** for Android using `ContentResolver` via JNI
4. **Storing the content URI** instead of converting to filesystem path
5. **Taking persistable URI permission** for access across app restarts
6. **Removing `MANAGE_EXTERNAL_STORAGE`** from the manifest

### IoProvider Trait

```rust
// In markdown-neuraxis-engine/src/io/mod.rs

pub trait IoProvider: Send + Sync {
    /// Read a file's content
    fn read_file(&self, relative_path: &RelativePath) -> Result<String, IoError>;

    /// Write content to a file (creates parent directories as needed)
    fn write_file(&self, relative_path: &RelativePath, content: &str) -> Result<(), IoError>;

    /// List all markdown files recursively
    fn list_markdown_files(&self) -> Result<Vec<RelativePathBuf>, IoError>;

    /// Check if a file exists
    fn exists(&self, relative_path: &RelativePath) -> bool;

    /// Validate that the storage location is accessible
    fn validate(&self) -> Result<(), IoError>;
}
```

### Platform Implementations

**Desktop (`StdFsProvider`):**
- Wraps existing `std::fs` code
- Takes `notes_root: PathBuf` in constructor
- Unchanged from current behavior

**Android (`SafProvider`):**
- Takes `tree_uri: String` (content URI) in constructor
- Uses JNI to call Android APIs:
  - `ContentResolver.openInputStream()` / `openOutputStream()` for read/write
  - `DocumentFile.fromTreeUri()` for directory traversal
  - `DocumentFile.listFiles()` for scanning
  - `DocumentFile.createFile()` / `createDirectory()` for creation

### FolderPickerActivity Changes

```java
// Instead of converting URI to path:
Uri uri = data.getData();

// Take persistable permission for access across restarts
getContentResolver().takePersistableUriPermission(uri,
    Intent.FLAG_GRANT_READ_URI_PERMISSION | Intent.FLAG_GRANT_WRITE_URI_PERMISSION);

// Return the content URI directly
result = uri.toString();
```

### Config Changes

The config will store the content URI on Android instead of a filesystem path:

```toml
# Android config
notes_uri = "content://com.android.externalstorage.documents/tree/primary%3ADocuments%2Fmarkdown-neuraxis"

# Desktop config (unchanged)
notes_path = "/home/user/notes"
```

## Consequences

### Positive

- **No special permissions required** - SAF URI grant is sufficient
- **No Settings redirect** - users stay in-app throughout setup
- **Play Store compliant** - no need to justify `MANAGE_EXTERNAL_STORAGE`
- **Proper Android integration** - follows platform conventions
- **Clean abstraction** - `IoProvider` trait enables testing and future platforms

### Negative

- **Significant refactor** - all file I/O goes through the trait
- **More JNI code** - Android implementation requires ContentResolver calls
- **Slightly more complex architecture** - trait + implementations vs free functions
- **Content URIs are opaque** - harder to debug than filesystem paths

### Neutral

- **Relative paths still work** - the abstraction preserves the relative path concept
- **Desktop unchanged** - `StdFsProvider` is essentially the existing code wrapped in a trait

## Implementation Plan

1. Define `IoProvider` trait in engine crate
2. Extract existing `std::fs` code into `StdFsProvider`
3. Update engine functions to accept `&dyn IoProvider`
4. Update dioxus crate to create appropriate provider per platform
5. Implement `SafProvider` with JNI calls for Android
6. Update `FolderPickerActivity` to return URI and take persistable permission
7. Update config to store URI on Android
8. Remove `MANAGE_EXTERNAL_STORAGE` from manifest
9. Remove settings redirect flow from setup screen
10. Update/supersede ADR-0009

## Alternatives Considered

1. **Keep MANAGE_EXTERNAL_STORAGE** - Current approach; poor UX and Play Store risk
2. **App-specific storage only** - No permissions but users can't access files externally
3. **Hybrid approach** - Use SAF for selection, convert to path for new folders only. Still needs MANAGE_EXTERNAL_STORAGE for existing folders.

## Supersedes

This ADR supersedes ADR-0009 (Android Storage Permissions Workaround) which will be marked as superseded once this is implemented.

## References

- [Storage Access Framework](https://developer.android.com/guide/topics/providers/document-provider)
- [Open files using SAF](https://developer.android.com/training/data-storage/shared/documents-files)
- [Access documents and other files](https://developer.android.com/training/data-storage/shared/documents-files#persist-permissions)
- [DocumentFile](https://developer.android.com/reference/androidx/documentfile/provider/DocumentFile)
- ADR-0009: Android Storage Permissions Workaround
- ADR-0010: Android Folder Picker via Helper Activity
