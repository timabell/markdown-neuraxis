#!/bin/bash
# Build Android APK for x86_64 emulator
# For physical ARM64 devices, use build-android-dx.sh
set -e
source set-android-envs.sh

# Generate Android project with Rust native library (default target for emulator)
# Uses custom AndroidManifest.xml via Dioxus.toml (see ADR-0009)
dx build --android --package markdown-neuraxis-dioxus

echo ""
echo "IMPORTANT: For MANAGE_EXTERNAL_STORAGE permission on Android 11+, users must:"
echo "  1. Go to Settings > Apps > markdown-neuraxis > Permissions"
echo "  2. Enable 'All files access'"
