#!/bin/bash
# Build Android APK for ARM64 physical devices
set -e
source set-android-envs.sh

# Generate Android project with Rust native library
# Uses custom AndroidManifest.xml via Dioxus.toml (see ADR-0009)
dx build --platform android --target aarch64-linux-android --package markdown-neuraxis-dioxus

echo ""
echo "IMPORTANT: For MANAGE_EXTERNAL_STORAGE permission on Android 11+, users must:"
echo "  1. Go to Settings > Apps > markdown-neuraxis > Permissions"
echo "  2. Enable 'All files access'"
