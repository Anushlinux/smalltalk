# Runtime Stability Harness

## Purpose

The developer-only harness measures whether the normal Tauri app remains bounded while it is open. It records resource counts and runtime counters. It never records screenshot pixels, Optical Character Recognition (OCR) text, Accessibility text, window titles, URLs, document paths, typed characters, or clipboard contents.

The thresholds are versioned in `docs/runtime-stability-policy-v1.json`. A harness report proves only the scenario that was actually run. It does not make the whole product “always-on stable.”

## Runtime policies

- The native event transport has 320 total slots: 64 high-value, 96 normal, and 160 coalescible pressure slots.
- One capture-loop transaction contains at most 32 events. The queue drain has a 12 millisecond turn budget.
- A pending capture trigger retains at most 128 event identifiers. It also stores total and omitted counts using `smalltalk.capture_trigger_causal_aggregation.v1`.
- The workload waiting queue has 48 total slots and smaller class-specific limits. Maintenance has one waiting slot. Audit has two workload slots, but the audit executor itself has one active job and one pending job.
- SQLite waits at most 750 milliseconds per ordinary connection operation. Event batches retry busy responses at most three times with short cancellation-aware backoff.
- Status heartbeat recovery runs every 60 seconds while recording and every 120 seconds while idle. Capture and session events remain the primary update path.
- React keeps one stable subscription for each Tauri capture and Continue event. Status renders do not tear down and recreate event-bridge subscriptions.
- Full audit output is manual-only. Background and startup Continue requests cannot enable it.
- Automatic retention is chunked. Automatic capture-time maintenance never runs `VACUUM`.

## Run one scenario

Use the normal product path. Keep the same `SMALLTALK_SOAK_RUN_ID` if a process restart occurs; the harness increments `process-start-count.txt`, making restarts observable.

```bash
cd /Users/bhaskarpandit/Documents/smalltalk
SMALLTALK_SOAK_RUN_ID=quiet-01 \
SMALLTALK_SOAK_SCENARIO=quiet \
SMALLTALK_SOAK_AUTO_START_CAPTURE=1 \
SMALLTALK_SOAK_AUTO_STOP_CAPTURE=1 \
SMALLTALK_SOAK_DURATION_MINUTES=60 \
SMALLTALK_SOAK_SAMPLE_SECONDS=5 \
npm run tauri dev
```

Reports are written to:

```text
output/runtime-stability/<run-id>/samples.jsonl
output/runtime-stability/<run-id>/summary.md
output/runtime-stability/<run-id>/policy.json
output/runtime-stability/<run-id>/process-start-count.txt
```

`samples.jsonl` is machine-readable. Lifetime counters are baselined when the harness starts, so the report shows only changes during the measured run. `summary.md` records the measured duration, process starts, crash deltas, peak resource counts, queue state, Continue request and decision-row growth, helper timeout handling, status cost, and final SQLite integrity result.

`SMALLTALK_SOAK_AUTO_START_CAPTURE=1` is a debug-build harness control. It calls the same backend start path as the UI after database and native-island initialization. Startup fails instead of producing a misleading stopped-app report when screen-capture permission is unavailable. Omit it for the clean Stop/restart scenario when the UI start boundary itself is under test.

`SMALLTALK_SOAK_AUTO_STOP_CAPTURE=1` asks the harness to use the owned capture shutdown path when the measurement window ends. It then writes one final sample after cleanup. This keeps a completed run from leaving its session marked `running`, and makes the final capture state and Stop latency part of the evidence. Omit it only when the scenario intentionally measures a user-driven Stop boundary.

## Required live matrix

Run each scenario with its own run id:

```bash
# A. Quiet layer
SMALLTALK_SOAK_RUN_ID=quiet-01 SMALLTALK_SOAK_SCENARIO=quiet SMALLTALK_SOAK_DURATION_MINUTES=60 npm run tauri dev

# B. Normal mixed work
SMALLTALK_SOAK_RUN_ID=mixed-01 SMALLTALK_SOAK_SCENARIO=mixed SMALLTALK_SOAK_DURATION_MINUTES=60 npm run tauri dev

# C. Continue stress
SMALLTALK_SOAK_RUN_ID=continue-01 SMALLTALK_SOAK_SCENARIO=continue-stress SMALLTALK_SOAK_DURATION_MINUTES=30 npm run tauri dev

# D. Native window churn
SMALLTALK_SOAK_RUN_ID=window-01 SMALLTALK_SOAK_SCENARIO=window-churn SMALLTALK_SOAK_DURATION_MINUTES=30 npm run tauri dev

# E. Fault recovery
SMALLTALK_SOAK_RUN_ID=fault-01 SMALLTALK_SOAK_SCENARIO=fault-recovery SMALLTALK_SOAK_DURATION_MINUTES=30 npm run tauri dev

# F. Clean Stop and restart
SMALLTALK_SOAK_RUN_ID=restart-01 SMALLTALK_SOAK_SCENARIO=clean-stop-restart SMALLTALK_SOAK_DURATION_MINUTES=15 npm run tauri dev
```

The harness measures; it does not automate human interaction. During mixed work, Continue stress, window churn, fault recovery, and restart scenarios, perform the actions listed in `docs/phases/runtime-stability/runtime-05-soak-test-and-release-gate.md`.

## Interpreting results

The following are absolute failures:

- a new Smalltalk main-process crash report;
- a main-process restart;
- an incorrectly running or interrupted session after clean Stop;
- an orphaned helper after cancellation or Stop;
- an unhandled or unreaped helper timeout; a bounded timeout is visible but is not an orphan when the child was killed and reaped;
- a queue depth or high-water mark above capacity;
- schema initialization increasing because ordinary events arrived;
- duplicate persisted Continue decisions for unchanged semantic evidence;
- SQLite `quick_check` not returning `ok`;
- loss of a current Continue behavior invariant.

Resource thresholds are policy gates, not permission to hide a trend. Memory must stop rising monotonically after warm-up even if the final number is below the byte ceiling. CPU must return to the quiet baseline after bounded work finishes.

## Current release status

The bounded pipeline has deterministic automated and synthetic tests. The complete multi-hour live matrix must still be run and reviewed before Smalltalk can be described as always-on stable.
