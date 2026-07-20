#!/bin/sh

set -eu

if [ -f ./.env ]; then
  set -a
  . ./.env
  set +a
fi

export SMALLTALK_TASK_TRUTH_PROVIDER_MODE=enabled
export SMALLTALK_TASK_TRUTH_MODEL=gpt-5.6-luna
export SMALLTALK_PFTU_COST_MODEL=gpt-5.6-luna
export SMALLTALK_PFTU_COMPACT_ONLY=1
export SMALLTALK_PFTU_SEMANTIC_PROBE_ENABLED=1

case_id="${1:-}"
if [ -z "$case_id" ]; then
  echo "PFTU compact-only launch refused: CASE_ID is required." >&2
  echo "Usage: ./scripts/tauri-dev-pftu.sh CASE_ID" >&2
  echo "Arm that exact case in the live capture database before launching." >&2
  exit 2
fi
case "$case_id" in
  *[!A-Za-z0-9._-]*)
    echo "PFTU compact-only launch refused: CASE_ID may contain only letters, numbers, dot, underscore, and hyphen." >&2
    exit 2
    ;;
esac
export SMALLTALK_PFTU_CASE_ID="$case_id"

database_path="${SMALLTALK_PFTU_DATABASE:-$HOME/Library/Application Support/com.smalltalk.app/capture/smalltalk-capture.sqlite}"
if [ ! -f "$database_path" ]; then
  echo "PFTU compact-only launch refused: capture database not found at $database_path" >&2
  echo "Set SMALLTALK_PFTU_DATABASE to the exact capture_status.database_path used when arming the case." >&2
  exit 2
fi
if ! command -v sqlite3 >/dev/null 2>&1; then
  echo "PFTU compact-only launch refused: sqlite3 is required to verify the armed case." >&2
  exit 2
fi
database_uri="file:$database_path?mode=ro&immutable=1"
if ! armed_row="$(sqlite3 -separator '|' "$database_uri" "SELECT expected_recorded_at_ms, COALESCE(consumed_decision_id, '') FROM task_truth_v2_semantic_probe_cases WHERE case_id='$case_id' LIMIT 1;")"; then
  echo "PFTU compact-only launch refused: could not verify case $case_id in $database_path" >&2
  exit 2
fi
if [ -z "$armed_row" ]; then
  echo "PFTU compact-only launch refused: case $case_id is not armed in $database_path" >&2
  exit 2
fi
IFS='|' read -r expected_recorded_at_ms consumed_decision_id <<EOF
$armed_row
EOF
if [ -n "$consumed_decision_id" ]; then
  echo "PFTU compact-only launch refused: case $case_id was already consumed by $consumed_decision_id" >&2
  exit 2
fi
now_ms="$(date +%s000)"
case_age_ms=$((now_ms - expected_recorded_at_ms))
if [ "$case_age_ms" -lt 0 ] || [ "$case_age_ms" -gt 900000 ]; then
  echo "PFTU compact-only launch refused: case $case_id expectation is not a fresh pre-output record (age_ms=$case_age_ms)." >&2
  echo "Update expected_recorded_at_ms, arm the case again, and relaunch within 15 minutes." >&2
  exit 2
fi

echo "Smalltalk PFTU compact-only dev: model=$SMALLTALK_TASK_TRUTH_MODEL probe=enabled legacy_request=blocked case=$case_id database=$database_path"

exec npm run tauri dev
