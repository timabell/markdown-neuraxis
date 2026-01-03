#!/bin/bash
# Build, install, and run on x86_64 emulator via adb
set -e

./build-android-dx-emulator.sh

echo "Installing via adb..."
adb install ./target/dx/markdown-neuraxis-dioxus/debug/android/app/app/build/outputs/apk/debug/app-debug.apk

echo "Starting app via adb..."
adb shell am start -n co.rustworkshop.markdown_neuraxis/dev.dioxus.main.MainActivity
