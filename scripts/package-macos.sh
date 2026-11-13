#!/usr/bin/env bash
set -euo pipefail

ROOT=$(cd "$(dirname "$0")/.." && pwd)
cd "$ROOT"

APP_NAME="YTDer"
BUNDLE_ID="com.xiaovov.ytder"
# Ad-hoc is for local builds only. Release builds should select a stable
# Developer ID Application identity to preserve identity across updates.
SIGNING_IDENTITY=${SIGNING_IDENTITY:--}
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
  <string>$BUNDLE_ID</string>
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
# Sign only after all bundle contents are final. The linker's signature on the
# naked executable does not seal Info.plist/resources and fails bundle validation.
SIGN_ARGS=(--force --sign "$SIGNING_IDENTITY" --identifier "$BUNDLE_ID")
if [[ "$SIGNING_IDENTITY" == "-" ]]; then
  SIGN_ARGS+=(--timestamp=none)
  echo "Local ad-hoc signature: macOS privacy grants may need renewal after rebuilding/replacing the app." >&2
else
  SIGN_ARGS+=(--options runtime --timestamp)
fi
/usr/bin/codesign "${SIGN_ARGS[@]}" "$APP"
/usr/bin/codesign --verify --deep --strict --verbose=2 "$APP"
/usr/bin/codesign -dr - "$APP"
DMG_STAGE="$OUT/.dmg-stage"
rm -rf "$DMG_STAGE"
mkdir -p "$DMG_STAGE"
cp -R "$APP" "$DMG_STAGE/$APP_NAME.app"
ln -s /Applications "$DMG_STAGE/Applications"
if /usr/bin/hdiutil create -volname "$APP_NAME" -srcfolder "$DMG_STAGE" -ov -format UDZO "$DMG" >/dev/null 2>&1; then
  DISTRIBUTION="$DMG"
else
  ZIP="$OUT/$APP_NAME-$ARCH.zip"
  rm -f "$ZIP"
  ditto -c -k --sequesterRsrc --keepParent "$DMG_STAGE" "$ZIP"
  DISTRIBUTION="$ZIP"
  echo "hdiutil was unavailable; created ZIP fallback instead." >&2
fi

printf 'Created:\n  %s\n  %s\n' "$APP" "$DISTRIBUTION"
file "$APP/Contents/MacOS/$APP_NAME"
shasum -a 256 "$APP/Contents/MacOS/$APP_NAME" "$DISTRIBUTION"
rm -rf "$DMG_STAGE"
