#!/bin/sh -v
d=$(date +%Y-%m-%d) &&
e="$HOME/.android/avd/Medium_Phone.avd"
a="$e/archive-$d/"
mkdir -p "$a" &&
echo mv $e/*.img "$a"
