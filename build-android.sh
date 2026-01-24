#!/bin/bash
# Build Rust FFI library for Android and generate Kotlin bindings
# See ADR-0011 for implementation plan
set -e

# Source Android environment variables
source "$(dirname "$0")/set-android-envs.sh"

# Build for arm64 (primary target - modern phones)
echo "Building for aarch64-linux-android (arm64-v8a)..."
cargo ndk -t aarch64-linux-android build --release -p markdown-neuraxis-ffi

# Create jniLibs directory structure
mkdir -p android/app/src/main/jniLibs/arm64-v8a

# Copy .so to Android jniLibs
cp target/aarch64-linux-android/release/libmarkdown_neuraxis_ffi.so \
   android/app/src/main/jniLibs/arm64-v8a/

# Generate Kotlin bindings
# UniFFI creates package structure: uniffi/markdown_neuraxis_ffi/
echo "Generating Kotlin bindings..."
cargo run -p markdown-neuraxis-ffi --bin uniffi-bindgen generate \
  --library target/aarch64-linux-android/release/libmarkdown_neuraxis_ffi.so \
  --language kotlin \
  --out-dir android/app/src/main/java/

# Ensure gradle wrapper exists
if [ ! -f android/gradlew ]; then
    echo "Generating gradle wrapper..."
    (cd android && gradle wrapper --gradle-version 8.9)
fi

# Build the APK
echo "Building APK..."
(cd android && ./gradlew assembleDebug)

echo "Done! APK at android/app/build/outputs/apk/debug/app-debug.apk"
