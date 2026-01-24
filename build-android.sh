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
echo "Generating Kotlin bindings..."
mkdir -p android/app/src/main/java/co/rustworkshop/markdownneuraxis/ffi/
cargo run -p markdown-neuraxis-ffi --bin uniffi-bindgen generate \
  --library target/aarch64-linux-android/release/libmarkdown_neuraxis_ffi.so \
  --language kotlin \
  --out-dir android/app/src/main/java/co/rustworkshop/markdownneuraxis/ffi/

echo "Done! Built arm64-v8a and generated Kotlin bindings."
