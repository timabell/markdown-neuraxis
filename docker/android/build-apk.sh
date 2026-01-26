#!/bin/bash
set -e

# Script to build Android APK using Docker
# Builds native Kotlin app with UniFFI bindings to Rust core
# Usage: ./build-apk.sh [--debug|--release] [--cached|--rebuild]

show_usage() {
    echo "Usage: $0 [--debug|--release] [--cached|--rebuild]"
    echo ""
    echo "Build type:"
    echo "  --debug    Build debug APK (default)"
    echo "  --release  Build release APK"
    echo ""
    echo "Docker image:"
    echo "  --cached   Use existing Docker image, build if missing"
    echo "  --rebuild  Force rebuild of Docker image"
    echo ""
    echo "Examples:"
    echo "  $0 --debug --cached     # Debug build with cached image"
    echo "  $0 --release --rebuild  # Release build, rebuild image"
}

# Parse arguments
BUILD_TYPE=""
CACHE_FLAG=""

while [[ $# -gt 0 ]]; do
    case $1 in
        --debug)
            BUILD_TYPE="debug"
            shift
            ;;
        --release)
            BUILD_TYPE="release"
            shift
            ;;
        --cached)
            CACHE_FLAG="cached"
            shift
            ;;
        --rebuild)
            CACHE_FLAG="rebuild"
            shift
            ;;
        *)
            echo "Error: Unknown argument '$1'"
            echo ""
            show_usage
            exit 1
            ;;
    esac
done

# Validate required arguments
if [[ -z "$BUILD_TYPE" ]]; then
    echo "Error: You must specify either --debug or --release"
    echo ""
    show_usage
    exit 1
fi

if [[ -z "$CACHE_FLAG" ]]; then
    echo "Error: You must specify either --cached or --rebuild"
    echo ""
    show_usage
    exit 1
fi

PROJECT_ROOT=$(cd "$(dirname "$0")/../.." && pwd)
DOCKER_IMAGE="markdown-neuraxis-android-builder"
OUTPUT_DIR="$PROJECT_ROOT/build/android"

# Named volumes for caching
CARGO_CACHE_VOLUME="markdown-neuraxis-cargo-cache"
GRADLE_CACHE_VOLUME="markdown-neuraxis-gradle-cache"
BUILD_VOLUME="markdown-neuraxis-build"

echo "Building Android APK for markdown-neuraxis (${BUILD_TYPE} mode)"
echo "Project root: $PROJECT_ROOT"

# Handle Docker image based on cache flag
if [[ "$CACHE_FLAG" == "rebuild" ]]; then
    echo "Rebuilding Docker image and clearing caches..."
    docker volume rm "$CARGO_CACHE_VOLUME" "$GRADLE_CACHE_VOLUME" "$BUILD_VOLUME" 2>/dev/null || true
    docker build -t "$DOCKER_IMAGE" -f "$PROJECT_ROOT/docker/android/Dockerfile" "$PROJECT_ROOT/docker/android"
elif [[ "$CACHE_FLAG" == "cached" ]]; then
    if ! docker image inspect "$DOCKER_IMAGE" > /dev/null 2>&1; then
        echo "No cached Docker image found. Building..."
        docker build -t "$DOCKER_IMAGE" -f "$PROJECT_ROOT/docker/android/Dockerfile" "$PROJECT_ROOT/docker/android"
    else
        echo "Using cached Docker image: $DOCKER_IMAGE"
    fi
fi

# Create output directory (owned by current user)
mkdir -p "$OUTPUT_DIR"

# Gradle task based on build type
if [ "$BUILD_TYPE" == "release" ]; then
    GRADLE_TASK="assembleRelease"
    APK_PATH="android/app/build/outputs/apk/release/app-release-unsigned.apk"
    OUTPUT_APK="markdown-neuraxis-release.apk"
else
    GRADLE_TASK="assembleDebug"
    APK_PATH="android/app/build/outputs/apk/debug/app-debug.apk"
    OUTPUT_APK="markdown-neuraxis-debug.apk"
fi

# Run Docker container to build APK
# Source mounted read-only at /src to:
#   1. Keep Docker build isolated - can't modify host files
#   2. Prevent root-owned files leaking into the source tree
# Build script creates /workspace with symlinks to source, allowing
# Kotlin/Gradle to create .kotlin/, .gradle/ in writable space
echo "Running build in Docker container..."
docker run --rm \
    -e HOST_UID="$(id -u)" \
    -e HOST_GID="$(id -g)" \
    -e CARGO_TARGET_DIR=/build/target \
    -v "$CARGO_CACHE_VOLUME:/root/.cargo" \
    -v "$GRADLE_CACHE_VOLUME:/root/.gradle" \
    -v "$BUILD_VOLUME:/build" \
    -v "$OUTPUT_DIR:/output" \
    -v "$PROJECT_ROOT:/src:ro" \
    -w /workspace \
    "$DOCKER_IMAGE" \
    /src/docker/android/build-inside-container.sh "$GRADLE_TASK" "$APK_PATH" "$OUTPUT_APK"

if [ -f "$OUTPUT_DIR/$OUTPUT_APK" ]; then
    echo ""
    echo "APK built successfully!"
    echo "Output: $OUTPUT_DIR/$OUTPUT_APK"
    echo ""
    echo "To install on a connected device:"
    echo "  adb install $OUTPUT_DIR/$OUTPUT_APK"
else
    echo "Build failed - APK not found"
    exit 1
fi
