#!/bin/bash
set -e
source set-android-envs.sh
dx build --platform android --target aarch64-linux-android --package markdown-neuraxis-dioxus
