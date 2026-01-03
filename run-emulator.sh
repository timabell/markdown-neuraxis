#!/bin/bash -v
# https://dioxuslabs.com/learn/0.7/guides/platforms/mobile#android
set -e

source set-android-envs.sh

# run list-emulator-devices.sh to get valid device names
emulator -avd Medium_Phone_API_36.1 -netdelay none -netspeed full
