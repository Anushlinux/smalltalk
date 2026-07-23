#!/bin/sh

set -eu

ROOT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
APP_VERSION=$(sed -n 's/^[[:space:]]*"version":[[:space:]]*"\([^"]*\)".*/\1/p' "$ROOT_DIR/src-tauri/tauri.conf.json" | head -n 1)
case "$(uname -m)" in
  arm64) APP_ARCH=aarch64 ;;
  x86_64) APP_ARCH=x64 ;;
  *) APP_ARCH=$(uname -m) ;;
esac
APP_PATH="$ROOT_DIR/src-tauri/target/release/bundle/macos/smalltalk.app"
DMG_DIR="$ROOT_DIR/src-tauri/target/release/bundle/dmg"
DMG_PATH="$DMG_DIR/smalltalk_${APP_VERSION}_${APP_ARCH}.dmg"
DMG_STAGING_PATH="$DMG_DIR/unsigned-alpha-staging"

if [ "$(uname -s)" != "Darwin" ]; then
  echo "Unsigned alpha bundles can only be built on macOS." >&2
  exit 1
fi

if [ -n "${APPLE_SIGNING_IDENTITY:-}" ] && [ "$APPLE_SIGNING_IDENTITY" != "-" ]; then
  echo "Unsigned alpha builds must not use a certificate-backed APPLE_SIGNING_IDENTITY." >&2
  exit 1
fi

if [ -z "${TAURI_SIGNING_PRIVATE_KEY:-}" ] && [ -n "${TAURI_SIGNING_PRIVATE_KEY_PATH:-}" ]; then
  TAURI_SIGNING_PRIVATE_KEY=$TAURI_SIGNING_PRIVATE_KEY_PATH
  export TAURI_SIGNING_PRIVATE_KEY
fi

if [ -z "${TAURI_SIGNING_PRIVATE_KEY:-}" ]; then
  DEFAULT_UPDATER_KEY=${SMALLTALK_UPDATER_KEY_PATH:-"$HOME/.tauri/smalltalk.key"}
  if [ ! -f "$DEFAULT_UPDATER_KEY" ]; then
    echo "A Tauri updater key is required at $DEFAULT_UPDATER_KEY or through TAURI_SIGNING_PRIVATE_KEY." >&2
    exit 1
  fi
  TAURI_SIGNING_PRIVATE_KEY=$DEFAULT_UPDATER_KEY
  export TAURI_SIGNING_PRIVATE_KEY
fi
TAURI_SIGNING_PRIVATE_KEY_PASSWORD=${TAURI_SIGNING_PRIVATE_KEY_PASSWORD-}
export TAURI_SIGNING_PRIVATE_KEY_PASSWORD

# These are generated outputs. Remove only the exact previous release bundle
# targets so stale helpers cannot survive inside the new technical-alpha build.
if [ -d "$APP_PATH" ]; then
  rm -rf -- "$APP_PATH"
fi
if [ -f "$DMG_PATH" ]; then
  rm -f -- "$DMG_PATH"
fi
if [ -d "$DMG_STAGING_PATH" ]; then
  rm -rf -- "$DMG_STAGING_PATH"
fi

cd "$ROOT_DIR"
SMALLTALK_VERIFIED_MACOS_BUILD=1 \
SMALLTALK_UNSIGNED_ALPHA_BUILD=1 \
APPLE_SIGNING_IDENTITY=- \
npm run tauri -- build --bundles app --config '{"bundle":{"macOS":{"signingIdentity":"-"}}}'

codesign --force --deep --sign - --options runtime --timestamp=none "$APP_PATH"

mkdir -p "$DMG_STAGING_PATH"
ditto "$APP_PATH" "$DMG_STAGING_PATH/smalltalk.app"
ditto "$ROOT_DIR/docs/unsigned-macos-alpha.md" "$DMG_STAGING_PATH/INSTALL-UNSIGNED.md"
ln -s /Applications "$DMG_STAGING_PATH/Applications"
hdiutil create \
  -volname "smalltalk" \
  -srcfolder "$DMG_STAGING_PATH" \
  -ov \
  -format UDZO \
  "$DMG_PATH"
rm -rf -- "$DMG_STAGING_PATH"

"$ROOT_DIR/scripts/verify-macos-unsigned-alpha.sh" "$APP_PATH" "$DMG_PATH"
