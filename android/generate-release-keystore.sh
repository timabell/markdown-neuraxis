#!/bin/bash
set -euo pipefail

keytool -genkey -v -keystore release.keystore -alias mdnx-gh-apk-signing -keyalg RSA -keysize 2048 -validity 30000 -dname "CN=Markdown Neuraxis,O=timabell,C=GB"

echo ""
echo "Now run: base64 -w0 release.keystore"
echo "Add these GitHub secrets:"
echo "  ANDROID_KEYSTORE_BASE64 - the base64 output"
echo "  ANDROID_KEYSTORE_PASSWORD - your password"
