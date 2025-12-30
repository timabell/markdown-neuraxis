#!/bin/sh
# must be 'sourced' to set env vars'
# run as follows: 'source set-android-envs.sh'
set -e

# Path for android studio installed by jetbrains toolbox on linux
# https://www.jetbrains.com/toolbox-app/
android_studio="$HOME/.local/share/JetBrains/Toolbox/apps/android-studio"

export JAVA_HOME="$android_studio/jbr"

export ANDROID_HOME="$HOME/Android/Sdk"

export NDK_HOME="$ANDROID_HOME/ndk/29.0.14206865"

export PATH="$PATH:$ANDROID_HOME/emulator:$ANDROID_HOME/platform-tools"
