# ADR-0009: Android Storage Permissions Workaround

## Status
Accepted

## Context

ADR-0008 documented the Android storage architecture and specified that permissions should be configured in `Dioxus.toml`:

```toml
[bundle.android.permissions]
permissions = [
    "android.permission.READ_EXTERNAL_STORAGE",
    "android.permission.WRITE_EXTERNAL_STORAGE"
]
```

However, as of January 2025, Dioxus does not support configuring Android permissions through `Dioxus.toml`. The relevant feature request ([#3870](https://github.com/DioxusLabs/dioxus/issues/3870)) and pull request ([#3535](https://github.com/DioxusLabs/dioxus/pull/3535)) remain unmerged.

This causes a critical issue: when users select an existing folder during setup, no files are shown because the app lacks `READ_EXTERNAL_STORAGE` permission. New folders work because the app creates them itself.

Additionally, Android 11+ (API 30+) introduced scoped storage, which further restricts file access even with legacy storage permissions. The app targets SDK 33 (Android 13).

## Decision

We will use a build-time manifest patching script as a workaround until Dioxus supports Android permissions natively.

### Permissions Added

1. **READ_EXTERNAL_STORAGE** - Required to read files on Android 10 and below
2. **WRITE_EXTERNAL_STORAGE** - Required to write files on Android 10 and below
3. **MANAGE_EXTERNAL_STORAGE** - Required for full file access on Android 11+

### Application Flags

- **requestLegacyExternalStorage="true"** - Enables legacy storage mode on Android 10

### Implementation

The `patch-android-manifest.sh` script modifies the generated `AndroidManifest.xml` after Dioxus creates it but before the Gradle build runs:

```bash
# After dx generates the project, patch the manifest
./patch-android-manifest.sh

# Then run the Gradle build manually or via dx
```

The build scripts (`build-android-dx.sh`, etc.) should be updated to include this patching step.

## Consequences

### Positive
- Enables reading/writing markdown files in user-selected folders
- Works around Dioxus limitation without forking the framework
- Preserves compatibility with future Dioxus permission support

### Negative
- Fragile workaround that depends on Dioxus's manifest generation format
- MANAGE_EXTERNAL_STORAGE is a sensitive permission requiring manual user approval in Settings
- May break if Dioxus changes its Android project generation

### User Experience on Android 11+
For full file access on Android 11+, users must:
1. Install the app
2. Go to Settings > Apps > markdown-neuraxis > Permissions
3. Enable "All files access"

This is not ideal UX but is the only option without implementing Storage Access Framework (SAF) integration.

## Alternatives Considered

1. **Wait for Dioxus support** - Would delay Android functionality indefinitely
2. **Fork Dioxus** - Too much maintenance burden
3. **Implement SAF** - Requires significant JNI/Kotlin bridging code and architectural changes
4. **Restrict to app-created folders only** - Would prevent users from using existing notes

## Future Work

- Monitor Dioxus for native permission support and migrate when available
- Consider SAF integration for a more Android-native experience
- Add in-app permission request flow with guidance for enabling all-files access

## Dependencies

The `patch-android-manifest.sh` script requires `xmlstarlet` for proper XML manipulation:
```bash
apt install xmlstarlet
```

## References

- [Dioxus Mobile Permissions Support #3870](https://github.com/DioxusLabs/dioxus/issues/3870)
- [Dioxus Android Manifest PR #4842](https://github.com/DioxusLabs/dioxus/pull/4842) (draft - linker-based permissions)
- [Dioxus Android Permissions PR #3535](https://github.com/DioxusLabs/dioxus/pull/3535) (closed)
- [Android Storage Access Framework](https://developer.android.com/guide/topics/providers/document-provider)
- [Android MANAGE_EXTERNAL_STORAGE](https://developer.android.com/training/data-storage/manage-all-files)
