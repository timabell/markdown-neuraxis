# Android APK Build with Docker

This directory contains a Docker-based build system for creating Android APKs without installing Android SDK/NDK on your host machine.

## Architecture

The Android app is a native Kotlin app using Jetpack Compose, with UniFFI bindings to the Rust core engine. The build process:

1. Cross-compiles `markdown-neuraxis-ffi` crate for Android (arm64 + x86_64)
2. Generates Kotlin bindings using UniFFI
3. Builds the Kotlin app with Gradle

## Prerequisites

- Docker installed on your system
- Project source code

## Usage

### Quick Build

From the project root, run:

```bash
# Build debug APK with cached Docker image
./docker/android/build-apk.sh --debug --cached

# Build release APK
./docker/android/build-apk.sh --release --cached

# Rebuild Docker image and build APK
./docker/android/build-apk.sh --debug --rebuild
```

### Output

The APK will be created in `build/android/`:
- Debug: `build/android/markdown-neuraxis-debug.apk`
- Release: `build/android/markdown-neuraxis-release.apk`

### What's Included

The Docker image contains:
- Ubuntu 22.04 base
- OpenJDK 17
- Android SDK (API 35)
- Android NDK 25.2.9519653
- Rust with Android targets (arm64, x86_64)
- cargo-ndk for cross-compilation

## Local Build (without Docker)

If you have the Android SDK/NDK set up locally:

```bash
# Set environment variables (adjust paths as needed)
source set-android-envs.sh

# Build and install
./build-android.sh
adb install android/app/build/outputs/apk/debug/app-debug.apk
```

## Troubleshooting

### Build Fails with NDK Error

Ensure the NDK version matches what's expected (25.2.9519653). Check the Dockerfile.

### APK Not Found

The APK location is:
- Debug: `android/app/build/outputs/apk/debug/app-debug.apk`
- Release: `android/app/build/outputs/apk/release/app-release-unsigned.apk`

### Docker Image Size

The Docker image is large (~3GB) due to Android SDK/NDK. This is normal.
The image is cached after first build.

### Permission Issues

If you get permission errors, ensure Docker has access to:
- Project directory
- Cargo cache directories (`~/.cargo/registry`, `~/.cargo/git`)
- Gradle cache directory (`~/.gradle`)
