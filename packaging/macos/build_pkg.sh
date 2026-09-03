#!/usr/bin/env bash
# Build an installable macOS .pkg package for splitype.
# Usage: ./packaging/macos/build_pkg.sh <version>
# Example: ./packaging/macos/build_pkg.sh 0.0.1
set -euo pipefail

if [ $# -eq 0 ]; then
    echo "Usage: $0 <version>"
    echo "Example: $0 0.0.1"
    exit 1
fi

VERSION="$1"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
DIST_DIR="$PROJECT_ROOT/dist"

APP_NAME="splitype"
APP_BUNDLE="${APP_NAME}.app"
BINARY_NAME="splitype"
BUNDLE_ID="com.hengvvang.splitype"

PKG_DIR="$DIST_DIR/pkg"
PKG_NAME="${APP_NAME}-${VERSION}.pkg"
COMPONENT_PKG="${APP_NAME}-component.pkg"

INSTALL_LOCATION="/Applications"
CLI_LINK="/usr/local/bin/${BINARY_NAME}"

# Ensure .app exists; if not, build it first
if [ ! -d "$DIST_DIR/$APP_BUNDLE" ]; then
    echo "==> $APP_BUNDLE not found. Running build_app.sh first..."
    "$SCRIPT_DIR/build_app.sh"
fi

if [ ! -f "$SCRIPT_DIR/pkg/Distribution.xml" ]; then
    echo "Error: Distribution.xml not found at $SCRIPT_DIR/pkg/"
    exit 1
fi

if [ ! -f "$SCRIPT_DIR/pkg/postinstall" ]; then
    echo "Error: postinstall script not found at $SCRIPT_DIR/pkg/"
    exit 1
fi

echo "==> Building PKG installer for $APP_NAME $VERSION..."
echo "    Bundle ID: $BUNDLE_ID"
echo "    Install location: $INSTALL_LOCATION"
echo "    CLI link target: $CLI_LINK"

rm -rf "$PKG_DIR"
mkdir -p "$PKG_DIR/root/Applications"
mkdir -p "$PKG_DIR/scripts"

echo "==> Preparing installation payload..."
cp -R "$DIST_DIR/$APP_BUNDLE" "$PKG_DIR/root/Applications/"

echo "==> Copying installation lifecycle hooks..."
cp "$SCRIPT_DIR/pkg/postinstall" "$PKG_DIR/scripts/"
cp "$SCRIPT_DIR/pkg/preuninstall" "$PKG_DIR/scripts/"
chmod +x "$PKG_DIR/scripts/"*

echo "==> Signing app bundle with ad-hoc signature..."
xattr -cr "$PKG_DIR/root/Applications/$APP_BUNDLE" 2>/dev/null || true
codesign --force --deep --sign - "$PKG_DIR/root/Applications/$APP_BUNDLE" 2>&1 || {
    echo "Warning: Code signing failed. This may prevent installation on locked-down environments."
}

echo "==> Creating component package..."
pkgbuild --identifier "$BUNDLE_ID" \
    --version "$VERSION" \
    --scripts "$PKG_DIR/scripts" \
    --root "$PKG_DIR/root" \
    --install-location "/" \
    "$PKG_DIR/$COMPONENT_PKG"

echo "==> Generating distribution package..."
cp "$SCRIPT_DIR/pkg/Distribution.xml" "$PKG_DIR/"
sed -i '' "s/__SPLITYPE_VERSION__/${VERSION}/g" "$PKG_DIR/Distribution.xml"

productbuild --distribution "$PKG_DIR/Distribution.xml" \
    --package-path "$PKG_DIR" \
    "$DIST_DIR/$PKG_NAME"

echo "==> Normalizing package metadata..."
pkgutil --expand "$DIST_DIR/$PKG_NAME" "$PKG_DIR/expanded" || true
if [ -f "$PKG_DIR/expanded/$COMPONENT_PKG/PackageInfo" ]; then
    sed -i '' '/<relocate>/,/<\/relocate>/d' "$PKG_DIR/expanded/$COMPONENT_PKG/PackageInfo"
    pkgutil --flatten "$PKG_DIR/expanded" "$DIST_DIR/$PKG_NAME"
    rm -rf "$PKG_DIR/expanded"
fi

echo "==> ✅ PKG installer successfully built at: $DIST_DIR/$PKG_NAME"
