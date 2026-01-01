#!/bin/bash
set -e
source set-android-envs.sh
dx build --android --package markdown-neuraxis-dioxus
