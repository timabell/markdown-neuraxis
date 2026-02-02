#!/bin/bash -v
# https://dioxuslabs.com/learn/0.7/guides/platforms/mobile#android
set -e

source set-android-envs.sh

# run list-emulator-devices.sh to get valid device names
emulator -avd Medium_Phone_API_36.1 -netdelay none -netspeed full -no-window

# Connect to gui with scrcpy - https://github.com/Genymobile/scrcpy
# ssh -N -L 5554:localhost:5554 -L 5555:localhost:5555 <host> # Forward ports
# adb connect 127.0.0.1:5555
# adb devices -l
# scrcpy -s 127.0.0.1:5555

