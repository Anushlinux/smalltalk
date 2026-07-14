#!/bin/sh

set -eu

if [ -f ./.env ]; then
  set -a
  . ./.env
  set +a
fi

# The normal model-first development path now uses the compact Luna request.
# Keep evaluation-case mode off so pressing Continue does not require an armed
# PFTU case; the app creates its own internal production audit record.
export SMALLTALK_TASK_TRUTH_PROVIDER_MODE=enabled
export SMALLTALK_TASK_TRUTH_AUTHORITY=eligible
export SMALLTALK_PFTU_SEMANTIC_PROBE_ENABLED=0
export SMALLTALK_PFTU_COMPACT_ONLY=0
unset SMALLTALK_PFTU_CASE_ID 2>/dev/null || true
unset SMALLTALK_PFTU_COST_MODEL 2>/dev/null || true

model="${SMALLTALK_TASK_TRUTH_MODEL:-gpt-5.6-luna}"
export SMALLTALK_TASK_TRUTH_MODEL="$model"

echo "Smalltalk model-first dev: semantic_path=compact_luna armed_case=not_required authority=eligible"
echo "Press Continue in the app to generate and show one fresh compact Luna result."

exec npm run tauri dev
