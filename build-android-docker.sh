#!/bin/bash
# Convenience script to build Android APK from project root
# Usage: ./build-android.sh [--debug|--release] [--cached|--rebuild]

exec ./docker/android/build-apk.sh "$@"