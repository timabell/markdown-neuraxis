#!/bin/bash
# Build Android APK for x86_64 emulator
# For physical ARM64 devices, use build-android-dx.sh
set -e
source set-android-envs.sh

ANDROID_PROJECT="target/dx/markdown-neuraxis-dioxus/debug/android/app"
JAVA_SRC_DIR="$ANDROID_PROJECT/app/src/main/java/co/rustworkshop/markdown_neuraxis"

# Generate Android project with Rust native library (default target for emulator)
# Uses custom AndroidManifest.xml via Dioxus.toml (see ADR-0011 for SAF integration)
dx build --android --package markdown-neuraxis-dioxus

# Copy FolderPickerActivity.java to generated project (see ADR-0010)
echo ""
echo "Adding FolderPickerActivity for native folder selection..."
mkdir -p "$JAVA_SRC_DIR"
cp android/java/co/rustworkshop/markdown_neuraxis/FolderPickerActivity.java "$JAVA_SRC_DIR/"
echo "Copied FolderPickerActivity.java"

# Rebuild APK with Gradle to include Java file
echo ""
echo "Rebuilding APK with Gradle..."
cd "$ANDROID_PROJECT"
./gradlew assembleDebug
cd - > /dev/null

echo ""
echo "APK built at: $ANDROID_PROJECT/app/build/outputs/apk/debug/app-debug.apk"
echo ""
echo "Note: Uses Storage Access Framework (SAF) for folder access - no manual permissions required."
