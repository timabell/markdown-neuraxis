#!/bin/bash
# https://dioxuslabs.com/learn/0.7/guides/platforms/mobile#android
set -e

source set-android-envs.sh

dx serve --android
