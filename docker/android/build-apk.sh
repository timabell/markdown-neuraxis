#!/bin/bash
set -e

# Script to build Android APK using Docker
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

echo "Building Android APK for markdown-neuraxis (${BUILD_TYPE} mode)"
echo "Project root: $PROJECT_ROOT"

# Handle Docker image based on cache flag
if [[ "$CACHE_FLAG" == "rebuild" ]]; then
    echo "Rebuilding Docker image..."
    docker build -t $DOCKER_IMAGE -f "$PROJECT_ROOT/docker/android/Dockerfile" "$PROJECT_ROOT/docker/android"
elif [[ "$CACHE_FLAG" == "cached" ]]; then
    if ! docker image inspect $DOCKER_IMAGE > /dev/null 2>&1; then
        echo "No cached Docker image found. Building new image..."
        docker build -t $DOCKER_IMAGE -f "$PROJECT_ROOT/docker/android/Dockerfile" "$PROJECT_ROOT/docker/android"
    else
        echo "Using cached Docker image: $DOCKER_IMAGE"
    fi
fi

# Create output directory
mkdir -p "$OUTPUT_DIR"

# Build command based on type - specify the dioxus package explicitly
if [ "$BUILD_TYPE" == "release" ]; then
    BUILD_CMD="dx build --platform android --package markdown-neuraxis-dioxus --release"
    APK_PATH="target/dx/markdown-neuraxis-dioxus/release/android/app/app/build/outputs/apk/release/app-release.apk"
else
    BUILD_CMD="dx build --platform android --package markdown-neuraxis-dioxus"
    APK_PATH="target/dx/markdown-neuraxis-dioxus/debug/android/app/app/build/outputs/apk/debug/app-debug.apk"
fi

# Run Docker container to build APK
echo "Running build in Docker container..."
docker run --rm \
    -v "$PROJECT_ROOT:/workspace" \
    -v "$HOME/.cargo/registry:/root/.cargo/registry" \
    -v "$HOME/.cargo/git:/root/.cargo/git" \
    -w /workspace \
    $DOCKER_IMAGE \
    bash -c "
        set -e
        echo 'Verifying Android toolchain...'
        if ! which aarch64-linux-android30-clang > /dev/null 2>&1; then
            echo 'ERROR: aarch64-linux-android30-clang not found in PATH'
            echo 'NDK_HOME: '\$NDK_HOME
            echo 'PATH: '\$PATH
            echo 'Expected location: '\$NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android30-clang
            ls -la \$NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android30-clang 2>/dev/null || echo 'File does not exist at expected location'
            exit 1
        fi
        echo 'SUCCESS: Android toolchain verified'
        echo ''
        echo 'Building APK...'
        cd /workspace

        # Build the APK
        $BUILD_CMD

        # Check if APK was created
        if [ -f '$APK_PATH' ]; then
            echo 'APK built successfully!'
            cp '$APK_PATH' '/workspace/build/android/markdown-neuraxis-${BUILD_TYPE}.apk'
            echo 'APK copied to build/android/'
        else
            echo 'Error: APK not found at expected location'
            echo 'Checking for APK files...'
            find target/dx -name '*.apk' -type f 2>/dev/null || true
            exit 1
        fi
    "

if [ -f "$OUTPUT_DIR/markdown-neuraxis-${BUILD_TYPE}.apk" ]; then
    echo ""
    echo "✅ APK built successfully!"
    echo "📦 Output: $OUTPUT_DIR/markdown-neuraxis-${BUILD_TYPE}.apk"
    echo ""
    echo "To install on a connected device:"
    echo "  adb install $OUTPUT_DIR/markdown-neuraxis-${BUILD_TYPE}.apk"
else
    echo "❌ Build failed - APK not found"
    exit 1
fi