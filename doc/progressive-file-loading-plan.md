# Progressive File Loading for Android

## Problem

The Android app blocks the UI while scanning large folders because:
1. `listMarkdownFiles()` is synchronous, called in `remember()` block
2. No coroutines or async patterns exist in the app
3. UI doesn't render until full file list is collected

## Solution Overview

Two-part fix:
1. **Kotlin side**: Add coroutines + StateFlow for progressive UI updates
2. **Rust engine**: Add `FileModel` data structure (for future workspace features)

## Implementation Plan

### Phase 1: Kotlin Progressive File Loading

**File**: `android/app/src/main/java/co/rustworkshop/markdownneuraxis/MainActivity.kt`

**1.1 Add FileDiscoveryState data class** (~line 50)
```kotlin
data class FileDiscoveryState(
    val files: List<DocumentFile> = emptyList(),
    val isScanning: Boolean = false,
    val error: String? = null
)
```

**1.2 Create progressive scanning function** (replace `listMarkdownFiles`)
```kotlin
private suspend fun scanMarkdownFilesProgressively(
    context: Context,
    folderUri: Uri,
    onBatch: (List<DocumentFile>) -> Unit
) {
    withContext(Dispatchers.IO) {
        val folder = DocumentFile.fromTreeUri(context, folderUri) ?: return@withContext
        scanFolderRecursively(folder, onBatch)
    }
}

private suspend fun scanFolderRecursively(
    folder: DocumentFile,
    onBatch: (List<DocumentFile>) -> Unit,
    batch: MutableList<DocumentFile> = mutableListOf()
) {
    for (file in folder.listFiles()) {
        if (file.isDirectory) {
            scanFolderRecursively(file, onBatch, batch)
        } else if (file.name?.endsWith(".md") == true) {
            batch.add(file)
            if (batch.size >= 20) {
                onBatch(batch.toList())
                batch.clear()
                yield() // Allow UI updates
            }
        }
    }
    if (batch.isNotEmpty()) {
        onBatch(batch.toList())
        batch.clear()
    }
}
```

**1.3 Update FileListScreen composable** (~line 130)
```kotlin
@Composable
fun FileListScreen(notesUri: Uri, ...) {
    var discoveryState by remember { mutableStateOf(FileDiscoveryState()) }
    val coroutineScope = rememberCoroutineScope()

    LaunchedEffect(notesUri) {
        discoveryState = FileDiscoveryState(isScanning = true)
        scanMarkdownFilesProgressively(context, notesUri) { batch ->
            discoveryState = discoveryState.copy(
                files = (discoveryState.files + batch).sortedBy { it.name }
            )
        }
        discoveryState = discoveryState.copy(isScanning = false)
    }

    // Show loading indicator if scanning and no files yet
    if (discoveryState.isScanning && discoveryState.files.isEmpty()) {
        CircularProgressIndicator()
    }

    LazyColumn {
        items(discoveryState.files) { file ->
            // existing file item rendering
        }

        // Show scanning indicator at bottom while loading more
        if (discoveryState.isScanning) {
            item {
                Text("Scanning...", style = MaterialTheme.typography.bodySmall)
            }
        }
    }
}
```

### Phase 2: Rust Engine FileModel (Foundation for Future)

**New file**: `crates/markdown-neuraxis-engine/src/models/file_model.rs`

A simple in-memory model of discovered files that can be updated incrementally:

```rust
use std::path::PathBuf;
use std::collections::BTreeMap;

/// Represents a discovered markdown file
#[derive(Debug, Clone)]
pub struct FileEntry {
    pub relative_path: PathBuf,
    pub display_name: String,
}

/// In-memory model of discovered files
#[derive(Debug, Default)]
pub struct FileModel {
    files: BTreeMap<PathBuf, FileEntry>,
}

impl FileModel {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a single file
    pub fn add_file(&mut self, relative_path: PathBuf, display_name: String) {
        self.files.insert(relative_path.clone(), FileEntry {
            relative_path,
            display_name,
        });
    }

    /// Add multiple files (batch)
    pub fn add_files(&mut self, entries: impl IntoIterator<Item = (PathBuf, String)>) {
        for (path, name) in entries {
            self.add_file(path, name);
        }
    }

    /// Get all files (sorted by path)
    pub fn files(&self) -> impl Iterator<Item = &FileEntry> {
        self.files.values()
    }

    /// Get file count
    pub fn len(&self) -> usize {
        self.files.len()
    }

    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    /// Clear all files
    pub fn clear(&mut self) {
        self.files.clear();
    }
}
```

**Update**: `crates/markdown-neuraxis-engine/src/models/mod.rs`
- Add `pub mod file_model;`

**Update**: `crates/markdown-neuraxis-engine/src/lib.rs`
- Re-export `FileModel` and `FileEntry`

### Phase 3: Unit Tests

**File**: `crates/markdown-neuraxis-engine/src/models/file_model.rs` (tests module)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_single_file() { ... }

    #[test]
    fn test_add_batch() { ... }

    #[test]
    fn test_files_sorted() { ... }
}
```

## Files to Modify

1. `android/app/src/main/java/co/rustworkshop/markdownneuraxis/MainActivity.kt`
   - Add `FileDiscoveryState` data class
   - Add `scanMarkdownFilesProgressively()` suspend function
   - Add `scanFolderRecursively()` suspend function
   - Update `FileListScreen` to use `LaunchedEffect` + state
   - Add loading indicators

2. `crates/markdown-neuraxis-engine/src/models/file_model.rs` (NEW)
   - `FileEntry` struct
   - `FileModel` struct with incremental add methods

3. `crates/markdown-neuraxis-engine/src/models/mod.rs`
   - Add module export

4. `crates/markdown-neuraxis-engine/src/lib.rs`
   - Re-export types

## Verification

1. **Android testing**:
   - Build APK: `cd android && ./gradlew assembleDebug`
   - Install on device/emulator with large notes folder (100+ files)
   - Verify UI appears immediately with loading indicator
   - Verify files appear progressively in the list
   - Verify scanning completes without blocking

2. **Rust testing**:
   - `cargo test -p markdown-neuraxis-engine`
   - Verify FileModel unit tests pass

3. **Format/lint**:
   - `cargo fmt && cargo clippy`
   - Android: `./gradlew ktlintCheck`

## Future Enhancements (Not in Scope)

- FFI bridge to expose FileModel to Kotlin (when needed for workspace features)
- File watching for live updates
- Caching scanned file list to disk
