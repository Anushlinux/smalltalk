#!/bin/sh

set -eu

ROOT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
APP_PATH=${1:-"$ROOT_DIR/src-tauri/target/release/bundle/macos/smalltalk.app"}
PROFILE=${SMALLTALK_SIGNING_PROFILE:-qa}
EXPECTED_IDENTIFIER=com.smalltalk.app

fail() {
  echo "macOS bundle verification failed: $1" >&2
  exit 1
}

if [ "$(uname -s)" != "Darwin" ]; then
  fail "codesign verification requires macOS"
fi

[ -d "$APP_PATH" ] || fail "app bundle does not exist at $APP_PATH"
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
  fail "codesign deep verification failed"

SIGNATURE_DETAILS=$(codesign -dvvv "$APP_PATH" 2>&1) || fail "could not inspect code signature"
echo "$SIGNATURE_DETAILS" | grep -q 'Signature=adhoc' &&
  fail "bundle is ad-hoc signed; its TCC identity will change across builds"
echo "$SIGNATURE_DETAILS" | grep -q 'TeamIdentifier=not set' &&
  fail "signed bundle has no Apple team identifier"
echo "$SIGNATURE_DETAILS" | grep -q 'flags=.*runtime' ||
  fail "hardened runtime is not enabled"
APP_TEAM_IDENTIFIER=$(echo "$SIGNATURE_DETAILS" | sed -n 's/^TeamIdentifier=//p')
[ -n "$APP_TEAM_IDENTIFIER" ] || fail "could not read the app team identifier"

DESIGNATED_REQUIREMENT=$(codesign -dr - "$APP_PATH" 2>&1) ||
  fail "could not inspect the designated requirement"
echo "$DESIGNATED_REQUIREMENT" | grep -q '# designated => cdhash ' &&
  fail "designated requirement is CDHash-only and is not stable across builds"

case "$PROFILE" in
  qa)
    ;;
  release)
    echo "$SIGNATURE_DETAILS" | grep -q 'Authority=Developer ID Application:' ||
      fail "release profile requires a Developer ID Application signature"
    spctl --assess --type execute --verbose=4 "$APP_PATH" >/dev/null 2>&1 ||
      fail "Gatekeeper rejected the release app; complete notarization and stapling before release"
    ;;
  *)
    fail "unknown SMALLTALK_SIGNING_PROFILE '$PROFILE'; use qa or release"
    ;;
esac

if [ -n "${APPLE_SIGNING_IDENTITY:-}" ]; then
  echo "$SIGNATURE_DETAILS" | grep -F -- "$APPLE_SIGNING_IDENTITY" >/dev/null ||
    fail "bundle was not signed by APPLE_SIGNING_IDENTITY"
fi

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
  fail "Contents/MacOS executable set differs from the main app and six approved capture sidecars. Found: $ACTUAL_EXECUTABLES"

for EXECUTABLE_NAME in \
  accessibility_snapshot \
  capture_events \
  image_mask \
  sck_screenshot \
  smalltalk \
  vision_ocr \
  window_snapshot
do
  EXECUTABLE_PATH="$APP_PATH/Contents/MacOS/$EXECUTABLE_NAME"
  codesign --verify --strict --verbose=2 "$EXECUTABLE_PATH" >/dev/null 2>&1 ||
    fail "$EXECUTABLE_NAME does not have a valid strict code signature"
  EXECUTABLE_SIGNATURE=$(codesign -dvvv "$EXECUTABLE_PATH" 2>&1) ||
    fail "could not inspect the $EXECUTABLE_NAME signature"
  echo "$EXECUTABLE_SIGNATURE" | grep -q 'Signature=adhoc' &&
    fail "$EXECUTABLE_NAME is ad-hoc signed"
  EXECUTABLE_TEAM_IDENTIFIER=$(echo "$EXECUTABLE_SIGNATURE" | sed -n 's/^TeamIdentifier=//p')
  [ "$EXECUTABLE_TEAM_IDENTIFIER" = "$APP_TEAM_IDENTIFIER" ] ||
    fail "$EXECUTABLE_NAME is not signed by app team $APP_TEAM_IDENTIFIER"
  EXECUTABLE_REQUIREMENT=$(codesign -dr - "$EXECUTABLE_PATH" 2>&1) ||
    fail "could not inspect the $EXECUTABLE_NAME designated requirement"
  echo "$EXECUTABLE_REQUIREMENT" | grep -q '# designated => cdhash ' &&
    fail "$EXECUTABLE_NAME has a CDHash-only designated requirement"
  strings "$EXECUTABLE_PATH" | grep -E -q '/usr/bin/osascript|System Events|tell application' &&
    fail "$EXECUTABLE_NAME contains forbidden Automation code"
done

echo "Verified $APP_PATH"
echo "  bundle identifier: $ACTUAL_IDENTIFIER"
echo "  team identifier: $APP_TEAM_IDENTIFIER"
echo "  signing profile: $PROFILE"
echo "  executable set: smalltalk plus six approved capture sidecars"
