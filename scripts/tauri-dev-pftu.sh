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

case_id="${1:-}"
if [ -n "$case_id" ]; then
  export SMALLTALK_PFTU_SEMANTIC_PROBE_ENABLED=1
  export SMALLTALK_PFTU_CASE_ID="$case_id"
else
  export SMALLTALK_PFTU_SEMANTIC_PROBE_ENABLED=0
  unset SMALLTALK_PFTU_CASE_ID 2>/dev/null || true
fi

echo "Smalltalk PFTU dev: model=$SMALLTALK_TASK_TRUTH_MODEL probe=$SMALLTALK_PFTU_SEMANTIC_PROBE_ENABLED case=${case_id:-none}"

exec npm run tauri dev
