# Android APK Build with Docker

This directory contains a Docker-based build system for creating Android APKs without installing Android SDK/NDK on your host machine.

## Prerequisites

- Docker installed on your system
- Project source code

## Usage

### Quick Build

From the project root, run:

```bash
# Build debug APK (default)
./build-android.sh

# Build release APK
./build-android.sh release

# Rebuild Docker image and build APK
./build-android.sh debug --rebuild
```

### Output

The APK will be created in `build/android/`:
- Debug: `build/android/markdown-neuraxis-debug.apk`
- Release: `build/android/markdown-neuraxis-release.apk`

### What's Included

The Docker image contains:
- Ubuntu 22.04 base
- OpenJDK 17
- Android SDK (API 34)
- Android NDK 25.2.9519653
- Rust with Android targets
- Dioxus CLI

## Troubleshooting

### Build Fails Silently

If the build fails without clear error:
1. Check if the Dioxus.toml exists in project root
2. Ensure the crates structure is correct
3. Check the build script output for errors

### APK Not Found

The APK location might vary. Check:
- `target/dx/*/android/app/app/build/outputs/apk/`
- Look for `app-debug.apk` or `app-release.apk`

### Docker Image Size

The Docker image is large (~3GB) due to Android SDK/NDK. This is normal.
The image is cached after first build.

### Permission Issues

If you get permission errors, ensure Docker has access to:
- Project directory
- Cargo cache directories

## Advantages of Docker Build

1. **Clean Host**: No Android SDK/NDK installed on your machine
2. **Reproducible**: Same environment for all developers
3. **Version Control**: Dockerfile tracks exact versions
4. **CI/CD Ready**: Can use same image in CI pipelines
5. **asdf Compatible**: Doesn't interfere with asdf or rustup on host