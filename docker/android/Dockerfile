# Multi-stage Dockerfile for building Android APKs with Dioxus
FROM ubuntu:22.04 AS android-builder

# Prevent interactive prompts during package installation
ENV DEBIAN_FRONTEND=noninteractive

# Install base dependencies
RUN apt-get update && apt-get install -y \
    curl \
    wget \
    unzip \
    git \
    build-essential \
    pkg-config \
    libssl-dev \
    openjdk-17-jdk \
    && rm -rf /var/lib/apt/lists/*

# Set up Android SDK
ENV ANDROID_HOME=/opt/android-sdk
ENV PATH=$PATH:$ANDROID_HOME/cmdline-tools/latest/bin:$ANDROID_HOME/platform-tools:$ANDROID_HOME/emulator

# Download and install Android command line tools
RUN mkdir -p $ANDROID_HOME/cmdline-tools && \
    cd $ANDROID_HOME/cmdline-tools && \
    wget -q https://dl.google.com/android/repository/commandlinetools-linux-11076708_latest.zip && \
    unzip -q commandlinetools-linux-11076708_latest.zip && \
    rm commandlinetools-linux-11076708_latest.zip && \
    mv cmdline-tools latest

# Accept Android SDK licenses and install required components
RUN yes | sdkmanager --licenses && \
    sdkmanager "platform-tools" \
    "platforms;android-34" \
    "build-tools;34.0.0" \
    "ndk;25.2.9519653" \
    "cmake;3.22.1"

# Set NDK environment variable
ENV NDK_HOME=$ANDROID_HOME/ndk/25.2.9519653
ENV JAVA_HOME=/usr/lib/jvm/java-17-openjdk-amd64

# Add NDK toolchain to PATH immediately after setting NDK_HOME
ENV PATH=$NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin:$PATH

# Install Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
ENV PATH="/root/.cargo/bin:${PATH}"

# Add Android targets for Rust
RUN rustup target add \
    aarch64-linux-android \
    armv7-linux-androideabi \
    i686-linux-android \
    x86_64-linux-android

# Install cargo-ndk for easier Android builds
RUN cargo install cargo-ndk

# Install dioxus-cli with locked dependencies
# This uses the exact dependency versions that worked when 0.6.3 was released
RUN cargo install dioxus-cli --locked

# Create workspace directory
WORKDIR /workspace

# Copy and set up cargo config for Android cross-compilation
COPY cargo-config.toml /root/.cargo/config.toml

# Set environment for Android builds
ENV ANDROID_NDK_ROOT=$NDK_HOME

# Set up environment variables for cross-compilation
ENV CC_aarch64_linux_android=aarch64-linux-android30-clang
ENV CC_armv7_linux_androideabi=armv7a-linux-androideabi30-clang
ENV CC_i686_linux_android=i686-linux-android30-clang
ENV CC_x86_64_linux_android=x86_64-linux-android30-clang
ENV AR_aarch64_linux_android=llvm-ar
ENV AR_armv7_linux_androideabi=llvm-ar
ENV AR_i686_linux_android=llvm-ar
ENV AR_x86_64_linux_android=llvm-ar

# Default command
CMD ["/bin/bash"]