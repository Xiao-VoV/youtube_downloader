#!/usr/bin/env bash
set -euo pipefail

ROOT=$(cd "$(dirname "$0")/.." && pwd)
cd "$ROOT"

APP_NAME="YTDer"
VERSION=$(awk -F '"' '/^version[[:space:]]*=/ { print $2; exit }' Cargo.toml)
if [[ -z "$VERSION" ]]; then
  VERSION=$(awk -F '"' '/^version[[:space:]]*=/ { print $2; exit }' crates/desktop/Cargo.toml)
fi
VERSION=${VERSION:-0.1.0}
ARCH=$(uname -m)
OUT="$ROOT/dist/macos"
APP="$OUT/$APP_NAME.app"
DMG="$OUT/$APP_NAME-$ARCH.dmg"

if [[ "$ARCH" != "arm64" && "$ARCH" != "x86_64" ]]; then
  echo "Unsupported macOS architecture: $ARCH" >&2
  exit 1
fi

rm -rf "$APP" "$DMG"
mkdir -p "$OUT/$APP_NAME.app/Contents/MacOS" "$OUT/$APP_NAME.app/Contents/Resources"

cargo build --release -p youtube-desktop
cp target/release/youtube-desktop "$APP/Contents/MacOS/$APP_NAME"
cp assets/YTDer.icns "$APP/Contents/Resources/YTDer.icns"
chmod 755 "$APP/Contents/MacOS/$APP_NAME"

cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key>
  <string>YTDer</string>
  <key>CFBundleDisplayName</key>
  <string>YTDer</string>
  <key>CFBundleExecutable</key>
  <string>YTDer</string>
  <key>CFBundleIdentifier</key>
  <string>com.xiaovov.ytder</string>
  <key>CFBundlePackageType</key>
  <string>APPL</string>
  <key>CFBundleIconFile</key>
  <string>YTDer</string>
  <key>CFBundleShortVersionString</key>
  <string>$VERSION</string>
  <key>CFBundleVersion</key>
  <string>$VERSION</string>
  <key>LSMinimumSystemVersion</key>
  <string>15.0</string>
  <key>NSHighResolutionCapable</key>
  <true/>
</dict>
</plist>
PLIST

/usr/bin/plutil -lint "$APP/Contents/Info.plist"
if /usr/bin/hdiutil create -volname "$APP_NAME" -srcfolder "$APP" -ov -format UDZO "$DMG" >/dev/null 2>&1; then
  DISTRIBUTION="$DMG"
else
  ZIP="$OUT/$APP_NAME-$ARCH.zip"
  rm -f "$ZIP"
  ditto -c -k --sequesterRsrc --keepParent "$APP" "$ZIP"
  DISTRIBUTION="$ZIP"
  echo "hdiutil was unavailable; created ZIP fallback instead." >&2
fi

printf 'Created:\n  %s\n  %s\n' "$APP" "$DISTRIBUTION"
file "$APP/Contents/MacOS/$APP_NAME"
shasum -a 256 "$APP/Contents/MacOS/$APP_NAME" "$DISTRIBUTION"
