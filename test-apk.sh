#!/bin/sh -v
set -e
adb devices
adb uninstall co.rustworkshop.MarkdownNeuraxisDioxus
ls -al build/android/markdown-neuraxis-debug.apk
adb install build/android/markdown-neuraxis-debug.apk
adb shell am start -n co.rustworkshop.MarkdownNeuraxisDioxus/dev.dioxus.main.MainActivity
