#!/bin/bash
set -euo pipefail

case "${TAURI_ENV_PLATFORM:-macos}" in
  macos|darwin)
    ;;
  *)
    exit 0
    ;;
esac

# A normal packaged QA or release build without a certificate-backed identity
# recreates the TCC bug this hook exists to prevent. The separately named
# unsigned-alpha lane is an explicit distribution exception and remains
# distinguishable from signed QA or release evidence.
if [[ "${TAURI_ENV_DEBUG:-false}" != "true" ]]; then
  if [[ "${SMALLTALK_VERIFIED_MACOS_BUILD:-}" != "1" ]]; then
    echo "macOS release bundles must be built through an approved macOS bundle script so stale bundle contents are removed and the result is verified." >&2
    exit 1
  fi
  if [[ "${SMALLTALK_UNSIGNED_ALPHA_BUILD:-}" == "1" ]]; then
    if [[ -n "${APPLE_SIGNING_IDENTITY:-}" && "${APPLE_SIGNING_IDENTITY}" != "-" ]]; then
      echo "Unsigned alpha builds must not use a certificate-backed APPLE_SIGNING_IDENTITY." >&2
      exit 1
    fi
  elif [[ -z "${APPLE_SIGNING_IDENTITY:-}" || "${APPLE_SIGNING_IDENTITY}" == "-" ]]; then
    echo "macOS release bundles require APPLE_SIGNING_IDENTITY to be set to an Apple Development or Developer ID Application certificate." >&2
    echo "Use npm run tauri:build:macos:qa for the verified packaged build." >&2
    exit 1
  fi
fi

case "${TAURI_ENV_ARCH:-$(uname -m)}" in
  aarch64|arm64)
    rust_arch="aarch64"
    swift_target="arm64-apple-macos13.0"
    ;;
  x86_64|x64)
    rust_arch="x86_64"
    swift_target="x86_64-apple-macos13.0"
    ;;
  *)
    echo "unsupported macOS sidecar architecture: ${TAURI_ENV_ARCH:-unknown}" >&2
    exit 1
    ;;
esac

root_dir="$(cd "$(dirname "$0")/../.." && pwd)"
source_dir="$root_dir/src-tauri/scripts"
output_dir="$root_dir/src-tauri/binaries"
module_cache="$root_dir/src-tauri/target/swift-sidecar-module-cache"
sdk_path="$(xcrun --sdk macosx --show-sdk-path)"
triple="${rust_arch}-apple-darwin"

mkdir -p "$output_dir" "$module_cache"

compile_helper() {
  local name="$1"
  shift
  local output="$output_dir/${name}-${triple}"
  local args=(
    -O
    -swift-version 5
    -sdk "$sdk_path"
    -target "$swift_target"
    -module-cache-path "$module_cache"
    "$source_dir/${name}.swift"
  )
  for framework in "$@"; do
    args+=( -framework "$framework" )
  done
  xcrun swiftc "${args[@]}" -o "$output"
  chmod 755 "$output"
}

compile_helper capture_events ApplicationServices AppKit
compile_helper window_snapshot AppKit CoreGraphics ApplicationServices
compile_helper accessibility_snapshot ApplicationServices AppKit
compile_helper vision_ocr Vision AppKit
compile_helper sck_screenshot AppKit CoreGraphics ScreenCaptureKit
compile_helper image_mask AppKit
