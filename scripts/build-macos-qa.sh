#!/bin/sh

set -eu

ROOT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
APP_PATH="$ROOT_DIR/src-tauri/target/release/bundle/macos/smalltalk.app"
PROFILE=${SMALLTALK_SIGNING_PROFILE:-qa}

if [ "$(uname -s)" != "Darwin" ]; then
  echo "macOS QA bundles can only be built on macOS." >&2
  exit 1
fi

if [ -z "${APPLE_SIGNING_IDENTITY:-}" ]; then
  echo "APPLE_SIGNING_IDENTITY must name an installed Apple Development or Developer ID Application certificate." >&2
  echo "List available identities with: security find-identity -v -p codesigning" >&2
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

if ! security find-identity -v -p codesigning | grep -F -- "$APPLE_SIGNING_IDENTITY" >/dev/null; then
  echo "APPLE_SIGNING_IDENTITY does not match an available code-signing identity." >&2
  exit 1
fi

case "$PROFILE" in
  qa)
    ;;
  release)
    case "$APPLE_SIGNING_IDENTITY" in
      Developer\ ID\ Application:*) ;;
      *)
        echo "Release builds require a Developer ID Application identity." >&2
        exit 1
        ;;
    esac
    HAS_API_CREDENTIALS=false
    if [ -n "${APPLE_API_ISSUER:-}" ] && [ -n "${APPLE_API_KEY:-}" ] && [ -n "${APPLE_API_KEY_PATH:-}" ]; then
      [ -f "$APPLE_API_KEY_PATH" ] || {
        echo "APPLE_API_KEY_PATH does not point to a readable App Store Connect private key." >&2
        exit 1
      }
      HAS_API_CREDENTIALS=true
    fi
    HAS_APPLE_ID_CREDENTIALS=false
    if [ -n "${APPLE_ID:-}" ] && [ -n "${APPLE_PASSWORD:-}" ] && [ -n "${APPLE_TEAM_ID:-}" ]; then
      HAS_APPLE_ID_CREDENTIALS=true
    fi
    if [ "$HAS_API_CREDENTIALS" != true ] && [ "$HAS_APPLE_ID_CREDENTIALS" != true ]; then
      echo "Release builds require notarization credentials: either APPLE_API_ISSUER + APPLE_API_KEY + APPLE_API_KEY_PATH, or APPLE_ID + APPLE_PASSWORD + APPLE_TEAM_ID." >&2
      exit 1
    fi
    ;;
  *)
    echo "Unknown SMALLTALK_SIGNING_PROFILE '$PROFILE'; use qa or release." >&2
    exit 1
    ;;
esac

# Tauri can otherwise leave files from an older bundle in Contents/MacOS. The
# directory is generated build output, so remove only this exact app before
# producing the signed QA artifact.
if [ -d "$APP_PATH" ]; then
  rm -rf -- "$APP_PATH"
fi

cd "$ROOT_DIR"
SMALLTALK_VERIFIED_MACOS_BUILD=1 npm run tauri -- build --bundles app

SMALLTALK_SIGNING_PROFILE="$PROFILE" "$ROOT_DIR/scripts/verify-macos-bundle.sh" "$APP_PATH"
