#!/bin/bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/../.." && pwd)"

cd "$repo_root"
npm run build

bash "$repo_root/src-tauri/scripts/build_swift_sidecars.sh"
