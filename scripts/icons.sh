#!/bin/sh
# Regenerate every app icon.
#   app-icon.png        the logo on a transparent square: Windows, Linux, README
#   app-icon-macos.png  the squircle tile: macOS Dock and Command-Tab (icon.icns)
#   scripts/icons/tray.svg        the menu bar icon (a macOS template image)
#   scripts/icons/tray-color.svg  the notification area icon on Windows and Linux
# To rebuild those two from the original artwork (needs Pillow, NumPy, SciPy):
#   python3 scripts/icons/cutout.py scripts/icons/logo-source.webp app-icon.png 1024
#   python3 scripts/icons/fit-squircle.py squircle-art.png app-icon-macos.png
set -e
cd "$(dirname "$0")/.."
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

bunx tauri icon app-icon.png -o src-tauri/icons
rm -rf src-tauri/icons/android src-tauri/icons/ios
bunx tauri icon app-icon-macos.png -o "$TMP/macos" >/dev/null
cp "$TMP/macos/icon.icns" src-tauri/icons/icon.icns
bunx tauri icon scripts/icons/tray.svg -o "$TMP/tray" >/dev/null
cp "$TMP/tray/64x64.png" src-tauri/icons/tray-template@2x.png
bunx tauri icon scripts/icons/tray-color.svg -o "$TMP/color" >/dev/null
cp "$TMP/color/64x64.png" src-tauri/icons/tray-color.png
cp app-icon.png public/app-icon.png
echo "Icons written to src-tauri/icons"
