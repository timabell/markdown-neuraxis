#!/bin/bash
# Build Android APK for ARM64 physical devices
set -e
source set-android-envs.sh

# Generate Android project with Rust native library
dx build --platform android --target aarch64-linux-android --package markdown-neuraxis-dioxus

# Patch manifest and rebuild APK (see ADR-0009)
./patch-android-manifest.sh
