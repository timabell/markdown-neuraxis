#!/bin/bash
# Build Android APK for x86_64 emulator
# For physical ARM64 devices, use build-android-dx.sh
set -e
source set-android-envs.sh

# Generate Android project with Rust native library (default target for emulator)
dx build --android --package markdown-neuraxis-dioxus

# Patch manifest and rebuild APK (see ADR-0009)
./patch-android-manifest.sh
