#!/usr/bin/env bash

set -euo pipefail

DEFAULT_DB="$HOME/Library/Application Support/com.smalltalk.app/capture/smalltalk-capture.sqlite"
DB_PATH="${SMALLTALK_CAPTURE_DB:-$DEFAULT_DB}"
AFTER_MS=0

usage() {
  cat <<'EOF'
Usage: npm run validate:suspended-boundary -- [--after-ms EPOCH_MS] [--db PATH]

Checks the latest persisted Luna request without printing screenshot contents,
semantic field text, URLs, paths, or captured application text.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --)
      shift
      ;;
    --after-ms)
      [[ $# -ge 2 ]] || { echo "Missing value for --after-ms" >&2; exit 2; }
      AFTER_MS="$2"
      shift 2
      ;;
    --db)
      [[ $# -ge 2 ]] || { echo "Missing value for --db" >&2; exit 2; }
      DB_PATH="$2"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

[[ "$AFTER_MS" =~ ^[0-9]+$ ]] || { echo "--after-ms must be an integer" >&2; exit 2; }
[[ -f "$DB_PATH" ]] || { echo "Capture database not found: $DB_PATH" >&2; exit 2; }
command -v sqlite3 >/dev/null || { echo "sqlite3 is required" >&2; exit 2; }
command -v jq >/dev/null || { echo "jq is required" >&2; exit 2; }

sqlite_json() {
  local query="$1"
  local output=""
  local attempt=1
  while [[ "$attempt" -le 5 ]]; do
    if output=$(sqlite3 -readonly -json "$DB_PATH" "$query" 2>/dev/null); then
      printf '%s' "$output"
      return 0
    fi
    sleep 0.2
    attempt=$((attempt + 1))
  done
  echo "Could not read the capture database after five attempts." >&2
  return 1
}

RUN_JSON=$(sqlite_json \
  "SELECT run_id, decision_id, diagnostic_status, model, request_audit_json,
          support_slot_map_json, admitted_output_json, validation_issues_json,
          failure_reason, created_at_ms
     FROM task_truth_v2_semantic_probe_runs
    WHERE created_at_ms >= $AFTER_MS
    ORDER BY created_at_ms DESC, run_id DESC
    LIMIT 1;")

if [[ "$(jq 'length' <<<"$RUN_JSON")" -eq 0 ]]; then
  echo "FAIL No Continue probe run was found after $AFTER_MS."
  exit 1
fi

AUDIT_JSON=$(jq -c '.[0].request_audit_json | if . == null then null else (fromjson? // null) end' <<<"$RUN_JSON")
SLOTS_JSON=$(jq -c '.[0].support_slot_map_json | if . == null then {} else (fromjson? // {}) end' <<<"$RUN_JSON")
OUTPUT_JSON=$(jq -c '.[0].admitted_output_json | if . == null then null else (fromjson? // null) end' <<<"$RUN_JSON")

PASS_COUNT=0
FAIL_COUNT=0
SAFE_COUNT=0

pass() {
  PASS_COUNT=$((PASS_COUNT + 1))
  printf 'PASS %s\n' "$1"
}

fail() {
  FAIL_COUNT=$((FAIL_COUNT + 1))
  printf 'FAIL %s\n' "$1"
}

safe() {
  SAFE_COUNT=$((SAFE_COUNT + 1))
  printf 'SAFE %s\n' "$1"
}

RUN_ID=$(jq -r '.[0].run_id' <<<"$RUN_JSON")
CREATED_AT_MS=$(jq -r '.[0].created_at_ms' <<<"$RUN_JSON")
DIAGNOSTIC=$(jq -r '.[0].diagnostic_status' <<<"$RUN_JSON")
MODEL=$(jq -r '.[0].model' <<<"$RUN_JSON")

printf 'Latest run: %s at %s\n' "$RUN_ID" "$CREATED_AT_MS"

[[ "$MODEL" == "gpt-5.6-luna" ]] \
  && pass "The request used gpt-5.6-luna." \
  || fail "The request used '$MODEL' instead of gpt-5.6-luna."

[[ "$DIAGNOSTIC" == "success" ]] \
  && pass "The provider request and response completed successfully." \
  || fail "The probe diagnostic status was '$DIAGNOSTIC'."

if [[ "$AUDIT_JSON" == "null" ]]; then
  fail "The run has no readable request audit."
  printf '\nRESULT: FAILED (%s failed checks)\n' "$FAIL_COUNT"
  exit 1
fi

SCHEMA=$(jq -r '.request_schema // ""' <<<"$AUDIT_JSON")
MODE=$(jq -r '.evidence_mode // ""' <<<"$AUDIT_JSON")
SCENE_FRAME=$(jq -r '.suspended_scene_frame_id // ""' <<<"$AUDIT_JSON")
CURRENT_FRAME=$(jq -r '.current_surface_frame_id // ""' <<<"$AUDIT_JSON")
CHECKPOINT_ID=$(jq -r '.suspended_checkpoint_id // ""' <<<"$AUDIT_JSON")
IMAGE_COUNT=$(jq -r '.image_count // 0' <<<"$AUDIT_JSON")
CAUSAL_COUNT=$(jq -r '.admitted_counts.causal_slot // 0' <<<"$AUDIT_JSON")
RECOGNITION_COUNT=$(jq -r '.raw_candidate_counts.checkpoint_recognition_candidates // 0' <<<"$AUDIT_JSON")
SLOT_KEYS=$(jq -c 'keys | sort' <<<"$SLOTS_JSON")

[[ "$SCHEMA" == "smalltalk.pftu_01.suspended_task_boundary_request.v1" ]] \
  && pass "The suspended-task-boundary request schema was used." \
  || fail "Unexpected request schema: '$SCHEMA'."

if jq -e '.missing_evidence | index("suspended_checkpoint_migration_fallback") != null' <<<"$AUDIT_JSON" >/dev/null; then
  fail "The removed legacy migration fallback appeared; the running binary is stale."
else
  pass "No legacy multi-image migration fallback was used."
fi

if [[ "$MODE" == "causal_suspended_checkpoint" ]]; then
  pass "A causal suspended checkpoint was selected."
  [[ -n "$CHECKPOINT_ID" ]] \
    && pass "The selected checkpoint has an identity." \
    || fail "The selected checkpoint identity is missing."
  [[ -n "$SCENE_FRAME" && -n "$CURRENT_FRAME" && "$SCENE_FRAME" != "$CURRENT_FRAME" ]] \
    && pass "The suspended scene and current surface are separate frames." \
    || fail "The suspended scene and current surface were not kept separate."
  jq -e 'index("SUSPENDED_SCENE") != null' <<<"$SLOT_KEYS" >/dev/null \
    && pass "SUSPENDED_SCENE was supplied." \
    || fail "SUSPENDED_SCENE was not supplied."
  jq -e 'index("CURRENT_SURFACE") != null' <<<"$SLOT_KEYS" >/dev/null \
    && pass "CURRENT_SURFACE was supplied separately." \
    || fail "CURRENT_SURFACE was not supplied."
  [[ "$CAUSAL_COUNT" -gt 0 ]] \
    && pass "At least one meaningful action or observable change survived." \
    || fail "The selected checkpoint collapsed into image-only evidence."
  [[ "$IMAGE_COUNT" -ge 2 && "$IMAGE_COUNT" -le 3 ]] \
    && pass "The request used only the suspended scene, optional focus crop, and current surface." \
    || fail "The causal request supplied $IMAGE_COUNT images; expected two or three."
elif [[ "$MODE" == "ambiguous_recognition_candidates" ]]; then
  [[ "$RECOGNITION_COUNT" -eq 2 ]] \
    && safe "Two recognition candidates were preserved instead of joining unrelated work." \
    || fail "Ambiguity did not contain exactly two bounded recognition candidates."
elif [[ "$MODE" == "visually_inferred_and_thin" ]]; then
  safe "The request explicitly declared itself visually inferred and thin."
  [[ "$SLOT_KEYS" == '["CURRENT_SURFACE"]' ]] \
    && pass "The thin request supplied only the factual current surface." \
    || fail "The thin request supplied unexpected image slots: $SLOT_KEYS"
  fail "No meaningful suspended checkpoint survived this test scenario."
else
  fail "Unknown evidence mode: '$MODE'."
fi

SUSPENDED_BUNDLE=""
for FRAME_ROLE in current suspended; do
  if [[ "$FRAME_ROLE" == "current" ]]; then
    FRAME_ID="$CURRENT_FRAME"
  else
    FRAME_ID="$SCENE_FRAME"
  fi
  [[ "$FRAME_ID" =~ ^[0-9]+$ ]] || continue
  OWNER_JSON=$(sqlite_json \
    "SELECT f.app_bundle_id AS expected_bundle,
            (SELECT w.bundle_id
               FROM window_snapshots ws
               JOIN windows w ON w.window_snapshot_id=ws.id
              WHERE CAST(ws.frame_id AS TEXT)=CAST(f.id AS TEXT)
                AND w.cg_window_id=f.window_id
                AND w.is_active=1
              ORDER BY ws.ts_ms DESC LIMIT 1) AS observed_bundle
       FROM frames f WHERE CAST(f.id AS TEXT)='$FRAME_ID' LIMIT 1;")
  EXPECTED_BUNDLE=$(jq -r '.[0].expected_bundle // ""' <<<"$OWNER_JSON")
  OBSERVED_BUNDLE=$(jq -r '.[0].observed_bundle // ""' <<<"$OWNER_JSON")
  if [[ "$FRAME_ROLE" == "suspended" ]]; then
    SUSPENDED_BUNDLE="$OBSERVED_BUNDLE"
  fi
  if [[ -n "$EXPECTED_BUNDLE" && "$EXPECTED_BUNDLE" == "$OBSERVED_BUNDLE" && "$OBSERVED_BUNDLE" != "com.smalltalk.app" ]]; then
    pass "The $FRAME_ROLE frame owner was independently verified as an external app."
  else
    fail "The $FRAME_ROLE frame owner was not independently verified (expected='$EXPECTED_BUNDLE', observed='$OBSERVED_BUNDLE')."
  fi
done

if [[ "$MODE" == "causal_suspended_checkpoint" ]]; then
  NORMALIZED_SUSPENDED_BUNDLE=$(printf '%s' "$SUSPENDED_BUNDLE" | tr '[:upper:]' '[:lower:]')
  if [[ "$NORMALIZED_SUSPENDED_BUNDLE" =~ codex|vscode|chrome|chromium|safari|firefox|helium|browser|edge|arc ]]; then
    jq -e 'index("SUSPENDED_FOCUS") != null' <<<"$SLOT_KEYS" >/dev/null \
      && pass "The dense suspended app included a readable same-scene focus crop." \
      || fail "The dense suspended app did not include SUSPENDED_FOCUS."
  fi
fi

if [[ "$OUTPUT_JSON" == "null" ]]; then
  fail "No admitted Luna output was persisted."
else
  OUTPUT_STATUS=$(jq -r '.status // ""' <<<"$OUTPUT_JSON")
  SEMANTIC_FIELD_COUNT=$(jq '[.primary_task,.current_step,.last_progress,.unfinished_state] | map(select(. != null and . != "")) | length' <<<"$OUTPUT_JSON")
  if [[ "$MODE" == "causal_suspended_checkpoint" ]]; then
    [[ "$SEMANTIC_FIELD_COUNT" -gt 0 ]] \
      && pass "Luna returned at least one evidence-backed semantic field." \
      || fail "Luna returned no semantic fields despite causal checkpoint evidence."
    [[ "$OUTPUT_STATUS" != "unresolved" ]] \
      && pass "Luna did not collapse the causal request to unresolved." \
      || fail "Luna returned unresolved despite causal checkpoint evidence."
  else
    safe "Luna output status was '$OUTPUT_STATUS' for a non-selected checkpoint result."
  fi
fi

VALIDATION_ISSUE_COUNT=$(jq '.[0].validation_issues_json | fromjson? | length // 0' <<<"$RUN_JSON")
[[ "$VALIDATION_ISSUE_COUNT" -eq 0 ]] \
  && pass "No output-validation issues were recorded." \
  || fail "$VALIDATION_ISSUE_COUNT output-validation issue(s) were recorded."

printf '\nSummary: %s passed, %s safe abstention notes, %s failed.\n' \
  "$PASS_COUNT" "$SAFE_COUNT" "$FAIL_COUNT"

if [[ "$FAIL_COUNT" -gt 0 ]]; then
  echo "RESULT: FAILED"
  exit 1
fi

if [[ "$MODE" == "ambiguous_recognition_candidates" ]]; then
  echo "RESULT: SAFE AMBIGUITY"
else
  echo "RESULT: PASSED"
fi
