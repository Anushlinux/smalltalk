#!/bin/sh

set -eu

if [ -f ./.env ]; then
  set -a
  . ./.env
  set +a
fi

# Run the existing model-first Task Truth path by itself. PFTU is a separate
# proof probe and must not issue a second model request in this mode.
export SMALLTALK_TASK_TRUTH_PROVIDER_MODE=enabled
export SMALLTALK_TASK_TRUTH_AUTHORITY=eligible
export SMALLTALK_PFTU_SEMANTIC_PROBE_ENABLED=0
unset SMALLTALK_PFTU_CASE_ID 2>/dev/null || true
unset SMALLTALK_PFTU_COST_MODEL 2>/dev/null || true

model="${SMALLTALK_TASK_TRUTH_MODEL:-gpt-5.6-luna}"
export SMALLTALK_TASK_TRUTH_MODEL="$model"

echo "Smalltalk MFTI-only dev: model=$SMALLTALK_TASK_TRUTH_MODEL PFTU_probe=disabled authority=eligible"
echo "Press Continue in the app to generate and show a fresh decision-bound MFTI result."

exec npm run tauri dev
