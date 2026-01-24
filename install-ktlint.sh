#!/bin/sh
# Install ktlint for formatting UniFFI-generated Kotlin bindings
# Without this there is a warning during the ffi build.
# You don't have to use this script, it's mostly a reminder.
# Use whatever install method you prefer, or don't bother at all.
# https://pinterest.github.io/ktlint/latest/install/cli/
set -e

curl -sSLO https://github.com/pinterest/ktlint/releases/download/1.5.0/ktlint
chmod a+x ktlint
sudo mv ktlint /usr/local/bin/
