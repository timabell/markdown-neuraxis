#!/bin/bash -v
set -e
# Connect to gui with scrcpy - https://github.com/Genymobile/scrcpy
# run port mapping ssh first
adb connect 127.0.0.1:5555
adb devices -l
scrcpy -s 127.0.0.1:5555

