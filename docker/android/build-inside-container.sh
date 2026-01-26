#!/bin/bash
# Script that runs inside the Docker container to build the Android APK
# Called by build-apk.sh - do not run directly
#
# Source is mounted read-only at /src to:
#   1. Keep Docker build isolated - can't modify host files
#   2. Prevent root-owned files leaking into the source tree
#
# We create /workspace with symlinks to /src, but android/ is a real dir
# so Kotlin/Gradle can create .kotlin/, .gradle/ etc in writable space.
# Build outputs go to /build via CARGO_TARGET_DIR and Gradle init script.
set -e

GRADLE_TASK="$1"
APK_PATH="$2"
OUTPUT_APK="$3"

# CARGO_TARGET_DIR is set by docker run to /build/target

# Set up workspace with symlinks to read-only source
# This allows Kotlin/Gradle to create .kotlin/, .gradle/ etc in writable space
echo 'Setting up workspace symlinks...'
mkdir -p /workspace
ln -sf /src/crates /workspace/crates
ln -sf /src/Cargo.toml /workspace/Cargo.toml
ln -sf /src/Cargo.lock /workspace/Cargo.lock
ln -sf /src/docker /workspace/docker

# Android dir is special - needs to be real dir so .kotlin/ can be created
mkdir -p /workspace/android
ln -sf /src/android/app /workspace/android/app
ln -sf /src/android/build.gradle.kts /workspace/android/build.gradle.kts
ln -sf /src/android/settings.gradle.kts /workspace/android/settings.gradle.kts
ln -sf /src/android/gradle.properties /workspace/android/gradle.properties
ln -sf /src/android/gradle /workspace/android/gradle

echo 'Verifying Android toolchain...'
if ! which aarch64-linux-android30-clang > /dev/null 2>&1; then
    echo 'ERROR: aarch64-linux-android30-clang not found in PATH'
    exit 1
fi
echo 'Android toolchain verified'
echo ''

# Build Rust FFI for arm64 (devices)
echo 'Building Rust FFI for aarch64-linux-android (arm64-v8a)...'
cargo ndk -t aarch64-linux-android build --release -p markdown-neuraxis-ffi

# Build Rust FFI for x86_64 (emulators)
echo 'Building Rust FFI for x86_64-linux-android (x86_64)...'
cargo ndk -t x86_64-linux-android build --release -p markdown-neuraxis-ffi

# Create jniLibs in /build (Gradle init script adds this as source)
mkdir -p /build/jniLibs/arm64-v8a
mkdir -p /build/jniLibs/x86_64

# Copy .so files from CARGO_TARGET_DIR
cp "$CARGO_TARGET_DIR/aarch64-linux-android/release/libmarkdown_neuraxis_ffi.so" \
   /build/jniLibs/arm64-v8a/
cp "$CARGO_TARGET_DIR/x86_64-linux-android/release/libmarkdown_neuraxis_ffi.so" \
   /build/jniLibs/x86_64/

# Generate Kotlin bindings to /build (Gradle init script adds this as source)
echo 'Generating Kotlin bindings...'
cargo run -p markdown-neuraxis-ffi --bin uniffi-bindgen generate \
  --library "$CARGO_TARGET_DIR/aarch64-linux-android/release/libmarkdown_neuraxis_ffi.so" \
  --language kotlin \
  --out-dir /build/uniffi/

# Build APK with Gradle using init script for source/output redirection
# Use system gradle directly (installed in Docker image)
echo 'Building APK with Gradle...'
cd /workspace/android
gradle \
    --project-cache-dir /build/gradle-project-cache \
    --init-script /workspace/docker/android/gradle-init.gradle \
    -Dkotlin.compiler.execution.strategy=in-process \
    "$GRADLE_TASK"
cd /workspace

# Copy APK to output and fix ownership
# APK is in /build/android/app/build/outputs/...
# Convert task name to lowercase for path (assembleDebug -> debug)
APK_VARIANT=$(echo "${GRADLE_TASK#assemble}" | tr '[:upper:]' '[:lower:]')
BUILD_APK_PATH="/build/android/app/build/outputs/apk/${APK_VARIANT}/app-${APK_VARIANT}.apk"
if [ -f "$BUILD_APK_PATH" ]; then
    echo 'APK built successfully!'
    cp "$BUILD_APK_PATH" "/output/$OUTPUT_APK"
    chown "$HOST_UID:$HOST_GID" "/output/$OUTPUT_APK"
else
    echo 'Error: APK not found at expected location'
    echo "Looked for: $BUILD_APK_PATH"
    find /build -name '*.apk' -type f 2>/dev/null || true
    exit 1
fi
