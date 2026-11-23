#!/bin/sh
# Regenerate every app icon.
#   app-icon.png        the logo on a transparent square: Windows, Linux, README
#   app-icon-macos.png  the logo on the olive squircle: macOS (icon.icns)
# To rebuild those two from the original artwork (needs Pillow, NumPy, SciPy):
#   python3 scripts/icons/cutout.py scripts/icons/logo-source.webp app-icon.png 1024
#   python3 scripts/icons/squircle.py app-icon.png app-icon-macos.png
set -e
cd "$(dirname "$0")/.."
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

bunx tauri icon app-icon.png -o src-tauri/icons
rm -rf src-tauri/icons/android src-tauri/icons/ios
bunx tauri icon app-icon-macos.png -o "$TMP/macos" >/dev/null
cp "$TMP/macos/icon.icns" src-tauri/icons/icon.icns
cp app-icon.png public/app-icon.png
echo "Icons written to src-tauri/icons"
