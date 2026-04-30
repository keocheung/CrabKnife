#!/bin/bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
APP_NAME="CrabKnife"
BUNDLE_DIR="$PROJECT_DIR/target/release/bundle"
APP_DIR="$BUNDLE_DIR/$APP_NAME.app"

cargo build --release

rm -rf "$APP_DIR"
mkdir -p "$APP_DIR/Contents/MacOS"
mkdir -p "$APP_DIR/Contents/Resources"

cp "$PROJECT_DIR/target/release/crab-knife" "$APP_DIR/Contents/MacOS/"
cp "$PROJECT_DIR/macos/Info.plist" "$APP_DIR/Contents/"

if [ -f "$PROJECT_DIR/macos/CrabKnife.icns" ]; then
    cp "$PROJECT_DIR/macos/CrabKnife.icns" "$APP_DIR/Contents/Resources/"
fi

echo "Built $APP_DIR"
