#!/bin/sh

set -eu

ROOT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
APP_VERSION=$(sed -n 's/^[[:space:]]*"version":[[:space:]]*"\([^"]*\)".*/\1/p' "$ROOT_DIR/src-tauri/tauri.conf.json" | head -n 1)
case "$(uname -m)" in
  arm64) APP_ARCH=aarch64 ;;
  x86_64) APP_ARCH=x64 ;;
  *) APP_ARCH=$(uname -m) ;;
esac
APP_PATH=${1:-"$ROOT_DIR/src-tauri/target/release/bundle/macos/smalltalk.app"}
DMG_PATH=${2:-"$ROOT_DIR/src-tauri/target/release/bundle/dmg/smalltalk_${APP_VERSION}_${APP_ARCH}.dmg"}
UPDATER_ARCHIVE="$ROOT_DIR/src-tauri/target/release/bundle/macos/smalltalk.app.tar.gz"
UPDATER_SIGNATURE="$UPDATER_ARCHIVE.sig"
EXPECTED_IDENTIFIER=com.smalltalk.app

fail() {
  echo "Unsigned alpha verification failed: $1" >&2
  exit 1
}

[ "$(uname -s)" = "Darwin" ] || fail "verification requires macOS"
[ -d "$APP_PATH" ] || fail "app bundle does not exist at $APP_PATH"
[ -f "$DMG_PATH" ] || fail "DMG does not exist at $DMG_PATH"
[ -s "$UPDATER_ARCHIVE" ] || fail "updater archive does not exist at $UPDATER_ARCHIVE"
[ -s "$UPDATER_SIGNATURE" ] || fail "updater signature does not exist at $UPDATER_SIGNATURE"
[ -f "$APP_PATH/Contents/Info.plist" ] || fail "Info.plist is missing"
[ -x "$APP_PATH/Contents/MacOS/smalltalk" ] || fail "main executable is missing"

ACTUAL_IDENTIFIER=$(/usr/libexec/PlistBuddy -c 'Print :CFBundleIdentifier' "$APP_PATH/Contents/Info.plist" 2>/dev/null) ||
  fail "CFBundleIdentifier is missing"
[ "$ACTUAL_IDENTIFIER" = "$EXPECTED_IDENTIFIER" ] ||
  fail "expected bundle identifier $EXPECTED_IDENTIFIER, found $ACTUAL_IDENTIFIER"

/usr/libexec/PlistBuddy -c 'Print :NSScreenCaptureUsageDescription' "$APP_PATH/Contents/Info.plist" >/dev/null 2>&1 ||
  fail "NSScreenCaptureUsageDescription is missing"
for FORBIDDEN_PRIVACY_KEY in \
  NSAppleEventsUsageDescription \
  NSDocumentsFolderUsageDescription \
  NSDownloadsFolderUsageDescription \
  NSDesktopFolderUsageDescription \
  NSMicrophoneUsageDescription \
  NSAudioCaptureUsageDescription
do
  if /usr/libexec/PlistBuddy -c "Print :$FORBIDDEN_PRIVACY_KEY" "$APP_PATH/Contents/Info.plist" >/dev/null 2>&1; then
    fail "unexpected privacy declaration $FORBIDDEN_PRIVACY_KEY"
  fi
done

codesign --verify --deep --strict --verbose=2 "$APP_PATH" >/dev/null 2>&1 ||
  fail "ad-hoc code signature is invalid"
SIGNATURE_DETAILS=$(codesign -dvvv "$APP_PATH" 2>&1) || fail "could not inspect code signature"
echo "$SIGNATURE_DETAILS" | grep -q 'Signature=adhoc' ||
  fail "bundle is not explicitly ad-hoc signed"
echo "$SIGNATURE_DETAILS" | grep -q 'flags=.*runtime' ||
  fail "hardened runtime is not enabled"

EXPECTED_EXECUTABLES=$(printf '%s\n' \
  accessibility_snapshot \
  capture_events \
  image_mask \
  sck_screenshot \
  smalltalk \
  vision_ocr \
  window_snapshot | sort)
ACTUAL_EXECUTABLES=$(find "$APP_PATH/Contents/MacOS" -maxdepth 1 -type f -exec basename {} \; | sort)
[ "$ACTUAL_EXECUTABLES" = "$EXPECTED_EXECUTABLES" ] ||
  fail "Contents/MacOS executable set is unexpected. Found: $ACTUAL_EXECUTABLES"

for EXECUTABLE_NAME in $EXPECTED_EXECUTABLES
do
  EXECUTABLE_PATH="$APP_PATH/Contents/MacOS/$EXECUTABLE_NAME"
  codesign --verify --strict --verbose=2 "$EXECUTABLE_PATH" >/dev/null 2>&1 ||
    fail "$EXECUTABLE_NAME does not have a valid ad-hoc signature"
  strings "$EXECUTABLE_PATH" | grep -E -q '/usr/bin/osascript|System Events|tell application' &&
    fail "$EXECUTABLE_NAME contains forbidden Automation code"
done

hdiutil verify "$DMG_PATH" >/dev/null || fail "DMG verification failed"

echo "Verified unsigned technical-alpha bundle"
echo "  app: $APP_PATH"
echo "  dmg: $DMG_PATH"
echo "  bundle identifier: $ACTUAL_IDENTIFIER"
echo "  signature: ad-hoc"
echo "  executable set: smalltalk plus six approved capture sidecars"
