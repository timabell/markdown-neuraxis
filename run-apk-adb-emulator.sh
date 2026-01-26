#!/bin/bash
# Build, install, and run on emulator via adb
set -e

./build-android.sh

echo "Installing via adb..."
adb install -r android/app/build/outputs/apk/debug/app-debug.apk

echo "Starting app..."
adb shell am start -n co.rustworkshop.markdownneuraxis/.MainActivity
