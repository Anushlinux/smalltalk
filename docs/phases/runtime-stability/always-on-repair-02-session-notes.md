# Always-On Repair 02 Session Notes

## Baseline recorded before source edits

- Live database was inspected read-only.
- Database bytes: `57,159,680`.
- Write-ahead log bytes: `0`.
- `PRAGMA quick_check`: `ok`.
- Rows: 12 sessions, 76 frames, 2,447 UI events, 100 capture triggers, 50 typing bursts, 83 transitions, and 27 Continue decisions.
- Capture storage: 102,736 KiB.
- No running Smalltalk app or capture helper was found at the process baseline.
- Prompt 01 helper runner tests: 8 passed.
- Prompt 01 provider-health tests: 5 passed.
- Swift helper type checks passed before Prompt 02 changes.

## Behavior invariants

- Continue remains the first-screen primitive and runs while capture is active.
- Stop is not required before Continue.
- Capture remains event-driven still capture.
- High-value app, window, navigation, error, permission, commit, typing, click, transition, and trigger evidence remains represented.
- Current focus and return target remain separate.
- Main card and native island consume the same decision identity.
- Thin evidence still produces abstention rather than invented intent or target.
- No raw typed characters or full clipboard contents are added.
- Smalltalk self-observation suppression remains active.
- Provider metadata and Prompt 01 failure isolation remain unchanged.
- `smalltalk.retention.v1` continues to protect current causal evidence.
- Full model calls remain manual-only.

## Implemented policy

- Event queue: 64 high-value, 96 normal, 160 pressure; total 320.
- Event batch: 32 maximum; queue drain: 12 milliseconds maximum per turn.
- Trigger causal ids: 128 maximum plus `smalltalk.capture_trigger_causal_aggregation.v1`.
- SQLite busy timeout: 750 milliseconds.
- Event busy retry: 3 attempts with cancellation-aware 15/30 millisecond backoff.
- Workload queue: 48 total with smaller Continue, capture, audit, maintenance, and derived limits.
- Audit executor: one active, one pending; equivalent decision ids coalesce.
- Maintenance: single-flight and non-waiting admission from event ingest.
- React heartbeat: 60 seconds recording, 120 seconds idle.
- React registers the three Tauri event listeners once. Stable handlers read
  current status, memory, selection, and developer-mode state through refs, so
  ordinary capture-status renders cannot create listen/unlisten IPC churn.
- Status payload excludes heavy text, trees, URLs, document paths, and image paths.

## Verification ledger

Completed focused checks:

- `capture::event_pipeline::tests::*`: 5 passed, including continuous-producer fairness.
- `workload::tests::*`: 9 passed after correcting the test to accept the truthful cancelled or superseded result category.
- `capture::tests::database_generation_initializes_once_and_event_load_does_not_run_schema_work`: passed with 3,200 events and no additional schema initialization.
- `capture::tests::event_batch_busy_retry_is_bounded_and_rolls_back`: passed.
- `capture::tests::large_database_status_queries_and_payload_remain_bounded`: 50,000 events, p50 2,767 microseconds, p95 3,022 microseconds, 975-byte projected response.
- `capture::tests::synthetic_sixty_minute_event_pressure_converges_and_preserves_causal_events`: passed.
- `capture::tests::copied_oversized_database_remediation_is_restart_safe_and_integrity_checked`: passed.
- Prompt 01 helper timeout under event pressure: passed, with the child killed and reaped.
- Continue decision-derived memory timestamp churn: passed without changing the evidence identity.
- A no-target Continue does not create inferred ignored feedback: passed.
- Full Rust suite: 852 passed, 3 explicitly ignored live-provider tests, 0 failed.
- `cargo fmt --all -- --check`: passed.
- `cargo check`: passed.
- Every Swift helper under `src-tauri/scripts/*.swift`: type-check passed.
- React production build: passed with TypeScript and Vite.

## Normal-path smoke evidence

The normal `npm run tauri dev` path was exercised on arm64 macOS 26.1 with the developer harness enabled.

The final unchanged-evidence comparison report is `output/runtime-stability/cache-proof-id-b/summary.md`:

- Requested duration: 6 seconds; measured duration: 7 seconds.
- Process starts: 1.
- Main and helper crash deltas: 0.
- Peak resident memory: 124,207,104 bytes.
- Peak threads: 25.
- Peak file descriptors: 21.
- Status p95: 10,470 microseconds.
- Status response: 6,157 bytes.
- SQLite `quick_check`: `ok`.
- Continue requests: 1.
- Continue decision row growth: 0.
- New schema initializations after harness start: 0.
- One native Accessibility helper hit its bounded deadline. It was killed and reaped. The fallback returned a typed non-zero result. Unhandled or unreaped helper timeouts: 0. Active child processes at the final sample: 0.

This short run proves the harness and the unchanged-evidence decision gate on the normal app path. It is not a substitute for the multi-hour matrix.

## Five-to-six-minute exit regression

The reported failure was reproduced in a debug app that remained open after a
five-minute harness sample. A user-started capture session then ran from
21:59:18 to 22:05:13 and finalized as `stopped`; the app exited one second later
with macOS launch-services exit status `0`. There was no Smalltalk crash report
and no Rust panic. Immediately before exit, WebKit reported 12,272 pending
incoming messages and named `WebPageProxy_StartURLSchemeTask` as the first
pending message.

The React event-listener effect depended on `applyContinueDecision`, and that
callback depended on the full capture status and Continue-memory objects. Each
capture-status update therefore changed the callback identity, tore down three
Tauri listeners, and registered all three again. Tauri listener registration
and removal both cross the custom URL-scheme IPC bridge. The fix makes the
listener callbacks stable and moves current mutable values into refs.

The normal `npm run tauri dev` path was then run with capture enabled for 420
requested seconds. The completed report is
`output/runtime-stability/listener-lifecycle-7m/summary.md`:

- Measured duration: 422 seconds; process starts: 1.
- Session 17 stopped cleanly with 11 frames and 1,203 events.
- Main and helper crash deltas: 0; helper timeouts: 0.
- Event maximum depth/capacity/high-water mark: 3/320/5; dropped events: 0.
- Workload maximum depth/capacity/high-water mark: 0/48/1.
- Peak resident memory: 114,950,144 bytes; post-warm-up memory slope:
  -28,693,796.66 bytes/hour.
- Status p95: 34,706 microseconds; final status response: 6,442 bytes.
- SQLite `quick_check`: `ok`; final capture state: `stopped`.
- The Smalltalk process remained open after the harness stopped capture.
- macOS unified logs contained no IPC-throttling, pending-incoming-message,
  `StartURLSchemeTask`, or process-exit entry for this run.

The 11.6 percent CPU p95 came from an active regression run with continuous
event traffic. It is not relabeled as a pass against the 10 percent quiet-CPU
gate. That gate remains owned by the required 60-minute quiet scenario.

## Live release status

Pending. The quiet 60-minute, mixed 60-minute, Continue-stress 30-minute, native-window-churn 30-minute, fault-recovery 30-minute, and clean-stop/restart 15-minute live scenarios still require human interaction and full reports. Do not describe Smalltalk as always-on stable until every scenario in `runtime-05-soak-test-and-release-gate.md` has a complete report and zero absolute-gate failures.
