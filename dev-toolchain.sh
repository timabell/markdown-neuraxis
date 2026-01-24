#!/bin/sh -v
# https://dioxuslabs.com/learn/0.7/getting_started/
sudo apt update
sudo apt install libwebkit2gtk-4.1-dev \
  xmlstarlet \
  qemu-kvm \
  libvirt-daemon-system \
  libvirt-clients \
  bridge-utils \
  build-essential \
  cpu-checker \
  curl \
  wget \
  file \
  libxdo-dev \
  libssl-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev \
  lld

# check kvm for android emulator
# https://developer.android.com/studio/run/emulator-acceleration#vm-linux-check-kvm
egrep -c '(vmx|svm)' /proc/cpuinfo
sudo kvm-ok

# cargo-ndk for building Rust libraries for Android (UniFFI bindings)
# https://github.com/nickelc/cargo-ndk
cargo install cargo-ndk

# Gradle for generating gradle wrapper (or just open in Android Studio)
sudo apt install gradle
