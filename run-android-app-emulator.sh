#!/bin/bash
# https://dioxuslabs.com/learn/0.7/guides/platforms/mobile#android
set -e

source set-android-envs.sh

# known issue: this doesn't patch the manifest for permissions
dx serve --android --package markdown-neuraxis-dioxus
