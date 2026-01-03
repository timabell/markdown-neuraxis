# ADR-0010: Android Folder Picker via Helper Activity

## Status
Accepted

## Context

The app needs a native folder picker on Android so users can select an existing notes folder without manually typing paths. Android provides the Storage Access Framework (SAF) with `ACTION_OPEN_DOCUMENT_TREE` for folder selection.

However, the SAF folder picker is Activity-based and returns results through Android's Activity lifecycle:

1. Call `startActivityForResult(intent)` to launch the system folder picker
2. User interacts with the picker (a separate Activity)
3. Android calls `onActivityResult(requestCode, resultCode, data)` **on the calling Activity**

This callback mechanism presents a challenge: Dioxus uses Android's `NativeActivity` which we don't control. Even if we call `startActivityForResult` via JNI from Rust, the result callback goes to NativeActivity's Java code, not back to our Rust code.

There is no blocking or callback-based API for folder selection - it's inherently Activity-based.

### Why not pure JNI?

JNI allows calling any Android API, but cannot:
- Override Activity methods to receive callbacks
- Intercept `onActivityResult` on an Activity we don't control
- Block waiting for user interaction (Android's main thread would freeze)

The folder picker requires *receiving* a callback, not just *making* a call.

## Decision

We will create a minimal Java helper Activity (`FolderPickerActivity`) that:

1. Launches `ACTION_OPEN_DOCUMENT_TREE` in `onCreate()`
2. Receives the result in `onActivityResult()`
3. Converts the content URI to a file path
4. Stores the result in static fields accessible via JNI
5. Calls `finish()` to return to the app

Rust code polls the static fields to detect completion and retrieve the selected path.

### URI to Path Conversion

SAF returns content URIs like:
```
content://com.android.externalstorage.documents/tree/primary:Documents/myfolder
```

Since we have `MANAGE_EXTERNAL_STORAGE` permission (ADR-0009), we can convert this to a real path:
```
/storage/emulated/0/Documents/myfolder
```

This allows continued use of standard file I/O rather than ContentResolver.

### Build Integration

The helper Activity is registered in the custom `AndroidManifest.xml` configured via `Dioxus.toml`.

The Java source file is stored at `android/java/co/rustworkshop/markdown_neuraxis/FolderPickerActivity.java` and must be copied to the generated Android project during build.

## Consequences

### Positive
- Native Android folder selection UX (familiar system UI)
- No manual path typing required on Android
- Follows standard Android pattern for native code needing activity results
- Minimal Java code (~100 lines)
- Works with existing MANAGE_EXTERNAL_STORAGE permission

### Negative
- Requires Java code in an otherwise pure-Rust project
- Java file must be copied to generated project during build
- Polling for results is slightly inefficient (though brief)

### Neutral
- Can be removed if Dioxus adds native folder picker support

## Alternatives Considered

1. **Pure JNI without helper Activity** - Not possible due to Activity lifecycle callback requirements

2. **Modify Dioxus's generated MainActivity** - Would break on Dioxus updates and require maintaining a fork

3. **BroadcastReceiver pattern** - More complex, still requires Java code, no real advantage

4. **Text input only** - Poor UX, users must know exact paths, error-prone

5. **Wait for Dioxus support** - No indication this is planned; folder pickers are app-specific

## References

- [Android Storage Access Framework](https://developer.android.com/guide/topics/providers/document-provider)
- [Getting results from activities](https://developer.android.com/training/basics/intents/result)
- [NativeActivity limitations](https://developer.android.com/ndk/guides/concepts#naa)
- ADR-0009: Android Storage Permissions Workaround
