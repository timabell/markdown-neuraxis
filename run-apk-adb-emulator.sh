#!/bin/bash
# Install and run on emulator via adb
set -e

./build-android.sh # make sure build is up to date

echo "Installing via adb..."
# build with ./build-android.sh to create this file
adb install -r android/app/build/outputs/apk/debug/app-debug.apk

echo "Starting app..."
adb shell am start -n co.rustworkshop.markdownneuraxis/.MainActivity
