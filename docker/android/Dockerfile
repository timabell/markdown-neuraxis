# Dockerfile for building Android APKs with Kotlin + UniFFI
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

# Install Gradle (newer than Ubuntu's package, needed to generate wrapper)
ENV GRADLE_VERSION=8.9
RUN wget -q "https://services.gradle.org/distributions/gradle-${GRADLE_VERSION}-bin.zip" -O /tmp/gradle.zip && \
    unzip -q /tmp/gradle.zip -d /opt && \
    rm /tmp/gradle.zip && \
    ln -s "/opt/gradle-${GRADLE_VERSION}/bin/gradle" /usr/local/bin/gradle

# Install ktlint for Kotlin formatting (used by UniFFI bindgen)
ENV KTLINT_VERSION=1.5.0
RUN wget -q "https://github.com/pinterest/ktlint/releases/download/${KTLINT_VERSION}/ktlint" -O /usr/local/bin/ktlint && \
    chmod +x /usr/local/bin/ktlint

# Set up Android SDK
ENV ANDROID_HOME=/opt/android-sdk
ENV PATH=$PATH:$ANDROID_HOME/cmdline-tools/latest/bin:$ANDROID_HOME/platform-tools

# Download and install Android command line tools
RUN mkdir -p $ANDROID_HOME/cmdline-tools && \
    cd $ANDROID_HOME/cmdline-tools && \
    wget -q https://dl.google.com/android/repository/commandlinetools-linux-11076708_latest.zip && \
    unzip -q commandlinetools-linux-11076708_latest.zip && \
    rm commandlinetools-linux-11076708_latest.zip && \
    mv cmdline-tools latest

# Accept Android SDK licenses and install required components
# SDK 35 for compileSdk/targetSdk, NDK for Rust cross-compilation
RUN yes | sdkmanager --licenses && \
    sdkmanager "platform-tools" \
    "platforms;android-35" \
    "build-tools;35.0.0" \
    "ndk;25.2.9519653"

# Set NDK environment variable
ENV NDK_HOME=$ANDROID_HOME/ndk/25.2.9519653
ENV ANDROID_NDK_ROOT=$NDK_HOME
ENV JAVA_HOME=/usr/lib/jvm/java-17-openjdk-amd64

# Add NDK toolchain to PATH
ENV PATH=$NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin:$PATH

# Install Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
ENV PATH="/root/.cargo/bin:${PATH}"

# Add Android targets for Rust (arm64 for devices, x86_64 for emulators)
RUN rustup target add \
    aarch64-linux-android \
    x86_64-linux-android

# Install cargo-ndk for Android cross-compilation
RUN cargo install cargo-ndk

# Create workspace directory
WORKDIR /workspace

# Copy cargo config for Android cross-compilation
COPY cargo-config.toml /root/.cargo/config.toml

# Set up environment variables for cross-compilation
ENV CC_aarch64_linux_android=aarch64-linux-android30-clang
ENV CC_x86_64_linux_android=x86_64-linux-android30-clang
ENV AR_aarch64_linux_android=llvm-ar
ENV AR_x86_64_linux_android=llvm-ar

# Default command
CMD ["/bin/bash"]
