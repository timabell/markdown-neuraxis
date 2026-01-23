# ADR 0010: Shift Android Target to Native Kotlin App

## Motivating issue

[Native folder chooser for new/existing notes folder in setup · Issue #10 · timabell/markdown-neuraxis](https://github.com/timabell/markdown-neuraxis/issues/10)

## Status
Accepted

## Context

This ADR is the result of lengthy research and discussion with the assitance of GPT, and this is a lightly edited GPT-generated summary of the understanding of the current situation of Android development with dioxus.

The `markdown-neuraxis` project uses a modular architecture with a core Rust engine and multiple frontends, including a Dioxus-based desktop GUI and a TUI prototype. Initially, the project attempted to target Android using [Dioxus CLI (`dx`)](https://github.com/DioxusLabs/dioxus), which builds a Rust+WebView Android app using internal tooling.

This approach had the theoretical advantage of sharing UI logic between desktop and mobile platforms. However, the mobile experience rapidly encountered critical limitations, especially when trying to integrate with standard Android APIs and platform capabilities that typically rely on Jetpack (AndroidX) libraries.

### JNI Access: Strengths and Frictions
Dioxus apps do have JNI access. The mobile runtime provides a valid Java `Activity` reference and `JNIEnv`, which allows calling Android SDK methods directly from Rust using the [`jni`](https://docs.rs/jni) crate. This mechanism has been demonstrated successfully in the project already, such as obtaining the internal storage path using `getFilesDir()` via JNI.

This means that in theory, **100% of the Android platform APIs can be accessed**, albeit in a verbose and error-prone way. Developers must manually manage JVM references, method signatures, and threading. Complex interactions (e.g., callback interfaces, services, content providers) often require Java glue code or subclasses, which Rust/JNI alone cannot express.

### The Blocking Issue: No Java Dependency Support
Modern Android features rely heavily on Jetpack libraries (AndroidX). These libraries are distributed via Maven and integrated using Gradle. Key examples include:

- `androidx.documentfile:documentfile` for accessing the Storage Access Framework
- `androidx.webkit:webkit` for modern WebView support
- `androidx.appcompat` and `material` for theming and compatibility
- `androidx.activity` and `activity-ktx` for result APIs and lifecycle management

Dioxus CLI 0.6+ introduced a custom build toolchain and explicitly dropped support for [`cargo-mobile`](https://github.com/BrainiumLLC/cargo-mobile). As a result:

- There is no documented or supported mechanism to declare Java dependencies (e.g., via `Cargo.toml`, `Dioxus.toml`, or CLI flags)
- The previous metadata config `[package.metadata.cargo-android] app-dependencies = [...]` is ignored
- The `dx` CLI generates the entire Android project internally, with no hooks for injecting Gradle dependencies
- The project templates themselves (e.g., `build.gradle.kts`) are not designed for extension without forking the CLI

[GitHub issue #3068](https://github.com/DioxusLabs/dioxus/issues/3068#issuecomment-2696999761) confirms this explicitly. The Dioxus team states they now "own the mobile tooling" and do not use cargo-mobile.

### Roadblock: Folder Selection via SAF Requires AndroidX
A critical integration roadblock occurred when implementing folder selection via the Storage Access Framework (SAF). The goal was to let users seamlessly choose a folder for their notes within emulated SD card storage using the modern Android permission model.

This requires launching a SAF folder picker and obtaining long-term URI permission to that folder — a capability normally handled with `DocumentFile` and related classes from `androidx.documentfile`. Without these helpers, replicating the same functionality involves extensive, brittle JNI code and complex permission handling.

Without AndroidX support, the only workaround is to instruct the user to manually open the system settings and grant the app full access to external storage — an outdated and unacceptable user experience for any modern Android application. This was a definitive usability blocker.

### Attempts to Work Around
Several paths were discussed to try to restore the ability to include Java dependencies:

1. **Manual .java Injection** – Adding one `.java` file into the build works if no dependencies are required. However, as soon as that file imports classes from Jetpack (e.g., `DocumentFile`), the build fails because those classes aren't present in the generated APK.

2. **Hand-wiring .jar/.aar Files** – Attempting to manually fetch and dex Java dependencies (e.g., using `d8`) and merge them into the APK is theoretically possible but brittle. This approach involves:
   - Downloading and resolving all transitive dependencies manually from Maven
   - Compiling Java code using `javac` against `android.jar`
   - Dexing both app and dependency code with `d8`
   - Merging multiple dex files, updating the APK, and resigning
   This workflow is complex, error-prone, and unmaintainable for real-world projects.

3. **Forking Dioxus CLI** – [Solana Mobile](https://github.com/regolith-labs/solana-mobile) forked `dioxus-cli` and modified the internal Gradle templates to support dependencies. While this works, it requires maintaining a fork of upstream tooling, losing benefits of future updates.

### What You *Can* Do Without Java Dependencies
Despite these issues, a Rust-based Dioxus Android app can:

- Use JNI to call platform APIs like file paths, permissions, sensors, etc.
- Trigger Android intents to open browsers or pick files (with manual URI handling)
- Render UIs via Rust/WebView and receive input events
- Execute network and compute-intensive logic in Rust

This works well for constrained or sandboxed use-cases. However, anything requiring broader Android integration hits immediate limitations.

### What You *Cannot* Do Practically
You cannot:

- Embed Google Maps, Firebase, or Play Services (they require Maven dependencies)
- Use CameraX, biometric APIs, or advanced media playback (e.g. ExoPlayer)
- Implement proper document handling without SAF helpers like `DocumentFile`
- Handle background tasks, notifications, or services without writing and registering Java components
- Extend UI functionality via Jetpack Compose, Material Theming, or AppCompat

All of these are core to delivering a high-quality Android app.

## Decision

We will replace the current Dioxus-based Android target with a native **Kotlin Android app**, which links to the Rust core engine as a shared library. This preserves:

- Cross-platform logic and shared Markdown processing in Rust
- Native Android user experience with full SDK and Jetpack integration
- Fast iteration and access to ecosystem tools (e.g. Android Studio, Gradle, Compose)

Meanwhile, Dioxus remains the frontend stack for desktop (macOS, Windows, Linux), where such platform integration constraints do not exist.

## Consequences

- The Rust core (`markdown-neuraxis-core`) will expose a clean C-compatible or JNI interface
- Android integration will follow standard NDK workflows using `cargo-ndk` or `uniffi`
- The Kotlin app can use Jetpack libraries (DocumentFile, WorkManager, Compose, etc.) without restriction
- The Dioxus CLI mobile backend will be excluded from Android release builds, and only used for desktop bundling

## References

- [Jetpack / AndroidX libraries](https://developer.android.com/jetpack)
- [AndroidX DocumentFile](https://developer.android.com/reference/androidx/documentfile/provider/DocumentFile)
- [Dioxus Issue: Cannot add dependencies](https://github.com/DioxusLabs/dioxus/issues/3068#issuecomment-2696999761)
- [Solana fork of Dioxus CLI with Gradle integration](https://github.com/regolith-labs/solana-mobile)
- [Dioxus Desktop & Mobile Native APIs · Issue #3855 · DioxusLabs/dioxus](https://github.com/DioxusLabs/dioxus/issues/3855)
