#!/bin/bash
# Build, install, and run on ARM64 physical device via adb
set -e

./build-android-dx.sh

echo "Installing via adb..."
adb install ./target/dx/markdown-neuraxis-dioxus/debug/android/app/app/build/outputs/apk/debug/app-debug.apk

echo "Starting app via adb..."
adb shell am start -n co.rustworkshop.markdown_neuraxis/dev.dioxus.main.MainActivity
