#!/usr/bin/env bash
# Build and package a standalone macOS .app bundle for splitype.
# Usage: ./packaging/macos/build_app.sh
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
DIST_DIR="$PROJECT_ROOT/dist"

BINARY_NAME="splitype"
APP_NAME="splitype"
APP_DIR="$DIST_DIR/$APP_NAME.app"

echo "==> Cleaning old distribution artifacts..."
rm -rf "$DIST_DIR"
mkdir -p "$DIST_DIR"

echo "==> Compiling optimized release binary (crates/app)..."
cargo build --release --manifest-path "$PROJECT_ROOT/Cargo.toml" -p app

echo "==> Assembling macOS App Bundle structure..."
mkdir -p "$APP_DIR/Contents/MacOS"
mkdir -p "$APP_DIR/Contents/Resources"

# Note: In Cargo.toml the crate binary is named "app", packaged as "splitype"
if [ -f "$PROJECT_ROOT/target/release/splitype" ]; then
    cp "$PROJECT_ROOT/target/release/splitype" "$APP_DIR/Contents/MacOS/$BINARY_NAME"
else
    cp "$PROJECT_ROOT/target/release/app" "$APP_DIR/Contents/MacOS/$BINARY_NAME"
fi
chmod +x "$APP_DIR/Contents/MacOS/$BINARY_NAME"

cp "$SCRIPT_DIR/Info.plist" "$APP_DIR/Contents/"
cp "$SCRIPT_DIR/$BINARY_NAME.icns" "$APP_DIR/Contents/Resources/$BINARY_NAME.icns"

if [ -f "$PROJECT_ROOT/README.md" ]; then
    cp "$PROJECT_ROOT/README.md" "$APP_DIR/Contents/Resources/"
fi

echo "==> ✅ App bundle created successfully at: $APP_DIR"
echo "    Launch with: open '$APP_DIR'"
