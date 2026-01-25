#!/bin/bash
# Script that runs inside the Docker container to build the Android APK
# Called by build-apk.sh - do not run directly
set -e

GRADLE_TASK="$1"
APK_PATH="$2"
OUTPUT_APK="$3"

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

# Create jniLibs directory structure
mkdir -p android/app/src/main/jniLibs/arm64-v8a
mkdir -p android/app/src/main/jniLibs/x86_64

# Copy .so files
cp target/aarch64-linux-android/release/libmarkdown_neuraxis_ffi.so \
   android/app/src/main/jniLibs/arm64-v8a/
cp target/x86_64-linux-android/release/libmarkdown_neuraxis_ffi.so \
   android/app/src/main/jniLibs/x86_64/

# Generate Kotlin bindings with UniFFI
echo 'Generating Kotlin bindings...'
cargo run -p markdown-neuraxis-ffi --bin uniffi-bindgen generate \
  --library target/aarch64-linux-android/release/libmarkdown_neuraxis_ffi.so \
  --language kotlin \
  --out-dir android/app/src/main/java/

# Generate gradle wrapper if needed
if [ ! -f android/gradlew ]; then
    echo 'Generating gradle wrapper...'
    cd android
    gradle wrapper --gradle-version 8.9
    cd ..
fi

# Build APK with Gradle
echo 'Building APK with Gradle...'
cd android
./gradlew "$GRADLE_TASK"
cd ..

# Copy APK to output and fix ownership
if [ -f "$APK_PATH" ]; then
    echo 'APK built successfully!'
    cp "$APK_PATH" "/output/$OUTPUT_APK"
    chown "$HOST_UID:$HOST_GID" "/output/$OUTPUT_APK"
else
    echo 'Error: APK not found at expected location'
    find android -name '*.apk' -type f 2>/dev/null || true
    exit 1
fi
