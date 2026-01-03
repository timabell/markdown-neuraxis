# Android Build Guide for markdown-neuraxis

## Overview
This document outlines the process for building markdown-neuraxis as an Android APK using Dioxus with a Docker-based build system.

## Prerequisites

- Docker installed on your system
- Project source code
- (Optional) `adb` for installing APK on devices

## Building the APK

We use Docker to handle all Android SDK/NDK dependencies, so you don't need to install anything Android-related on your machine.

### Quick Build

From the project root:

```bash
# Build debug APK (default)
./build-android.sh

# Build release APK
./build-android.sh release

# Rebuild Docker image and then build APK
./build-android.sh debug --rebuild
```

### Output Location

APKs are saved to `build/android/`:
- Debug: `build/android/markdown-neuraxis-debug.apk`
- Release: `build/android/markdown-neuraxis-release.apk`

### First Build

The first build will:
1. Build the Docker image (~3GB, includes Android SDK/NDK/Rust)
2. Compile the app for Android
3. Output the APK

Subsequent builds use the cached Docker image and are much faster.

## Testing the APK

### On Physical Device

1. Enable Developer Options and USB Debugging on your Android device
2. Connect via USB
3. Install the APK:
   ```bash
   adb install build/android/markdown-neuraxis-debug.apk
   ```

### On Emulator

If you have Android Studio installed locally:
1. Create an AVD (Android Virtual Device)
2. Launch the emulator
3. Install the APK:
   ```bash
   adb install build/android/markdown-neuraxis-debug.apk
   ```

### Without Local Android Tools

You can also transfer the APK to your device via:
- Email
- Cloud storage (Google Drive, Dropbox)
- USB file transfer
- Web server

Then install it directly on the device (requires "Install from unknown sources" permission).

## Current Limitations

1. **Experimental Status**: Android support in Dioxus is still experimental
2. **WebView Based**: Uses platform WebView, not native UI components
3. **File Size**: Basic APK is ~15MB due to Rust runtime
4. **Animations**: Native Android animations not supported (use CSS animations)
5. **File System Access**: May need special permissions for accessing device storage

## Docker Build System Details

The Docker-based build system keeps your host machine clean:

### What's in the Docker Image
- Ubuntu 22.04 base
- Android SDK (API 34)
- Android NDK 25.2.9519653
- OpenJDK 17
- Rust with all Android targets
- Dioxus CLI
- Cross-compilation toolchain

### Why Docker?

1. **Clean Host**: No Android SDK/NDK cluttering your machine
2. **Works with asdf**: No conflicts with your local Rust setup via asdf
3. **Reproducible**: Same build environment for all developers
4. **CI/CD Ready**: Can use the same image in CI pipelines
5. **Version Controlled**: All dependencies tracked in Dockerfile

## Next Steps for markdown-neuraxis

1. **Handle file system permissions** for accessing markdown files
2. **Optimize for mobile UI** (touch targets, responsive layout)
3. **Implement mobile-specific features** (share intent, document provider)
4. **Test on various Android versions** (API 21+)
5. **Set up APK signing** for release builds

## Troubleshooting

### Docker Build Issues

If the Docker build fails:
1. Ensure Docker is running
2. Check available disk space (need ~4GB)
3. Try rebuilding: `./build-android.sh debug --rebuild`

### APK Not Found

If the APK isn't in the expected location:
1. Check the build script output for errors
2. Look in `target/dx/` subdirectories for `*.apk` files

### Build Silent Failures

Dioxus Android builds can fail silently. Check:
1. Dioxus.toml exists in project root
2. Project structure matches Dioxus expectations
3. All Rust code compiles for Android targets

### File Access on Android

The app will need permissions to access device storage:
- Permissions are configured in `Dioxus.toml`
- May need to use Android's Storage Access Framework
- Consider using app-specific storage initially

## More Information

- Docker build details: `docker/android/README.md`
- Dioxus mobile docs: https://dioxuslabs.com/learn/0.6/guides/mobile/
- Android development: https://developer.android.com/