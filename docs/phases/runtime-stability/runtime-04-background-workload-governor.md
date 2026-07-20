# Runtime Stability 04 — Coordinate Capture, Continue, OCR, And Background Work

## Codex task

Introduce one explicit workload-governance layer so Smalltalk does not perform overlapping expensive work merely because raw evidence counters changed. Make idle operation genuinely light while preserving fresh, truthful Continue results.

This session owns scheduling, coalescing, cancellation, and priority. Do not weaken evidence semantics to make timing numbers look better.

## Read before editing

```text
AGENTS.md
PRODUCT.md
docs/phases/runtime-stability/runtime-00-always-on-stability-program.md
docs/phases/runtime-stability/runtime-01-screen-capture-crash-containment.md
docs/phases/runtime-stability/runtime-02-bounded-event-and-storage-runtime.md
docs/full-engine-flow.md
src/App.tsx
src-tauri/src/capture.rs
src-tauri/src/session_island/gateway.rs
src-tauri/src/continuation.rs
src-tauri/src/continuation/activity_recap_inputs.rs
src-tauri/src/continuation/task_turn.rs
```

## Verified failure

Current runtime work includes:

- a 15-second capture-status heartbeat while recording;
- capture events and capture settlement;
- full-display and active-window screenshot work;
- Accessibility collection and possible OCR;
- stale-evidence detection;
- background Continue with a one-minute minimum interval;
- main-card and island refresh paths;
- manual model-assisted Continue;
- audit export.

These paths have local guards, but there is no single runtime policy that explains which expensive operation may run, which request should coalesce, and which work should yield to a manual user action.

## Required implementation

### 1. Classify work by cost and priority

Create a small internal workload model covering at least:

```text
cheap status read
event ingest
Accessibility snapshot
screenshot capture
OCR
derived evidence update
background Continue
manual Continue
island refresh
audit export
maintenance cleanup
```

Assign each class a concurrency rule, cancellation rule, and priority. Manual Continue and explicit capture should receive prompt service. Background rebuilding, audit export, and cleanup should yield where safe.

### 2. Use semantic invalidation

Do not mark Continue stale merely because a raw event count increased. Build a material-evidence signature from evidence that can actually change the public answer, such as:

```text
new accepted frame
task-turn revision
meaningful app, window, or navigation transition
committed action evidence
feedback or open event
target validity change
relevant retention or privacy change
```

Low-value scroll or repeated Accessibility noise must not trigger a new decision by itself.

### 3. Make Continue single-flight across surfaces

The main card, native island, startup path, background refresh, and manual refresh must share one decision coordinator.

Required behavior:

- unchanged semantic watermark returns the existing decision;
- at most one decision computation runs;
- manual work can supersede queued background work;
- stale background results cannot replace newer manual results;
- cancelled or superseded computations do not persist duplicate derived rows;
- island and main card receive the same decision identity;
- model calls remain manual-only unless a later product contract explicitly changes that rule.

### 4. Prevent expensive overlap

Define safe overlap rules. In particular, measure and control simultaneous screenshot or OCR, Continue rebuild, audit export, and maintenance.

Do not solve this with one global lock that freezes the entire application. Cheap reads and event ingestion should remain responsive. Use bounded queues, read snapshots, short database transactions, cancellation tokens, and priority where appropriate.

### 5. Make status polling cheap

Audit what `capture_status` and related UI refreshes query. The 15-second heartbeat must not scan large tables or reconstruct Continue state.

Push event-driven status changes where practical. Keep a slow safety heartbeat, but ensure its query plan and response size are bounded.

### 6. Add runtime observability

Expose privacy-safe metrics for:

```text
active operation by class
queued operation count
coalesced requests
cancelled or superseded requests
operation duration percentiles
database busy time
decision cache and watermark reuse
OCR and screenshot counts
background decisions avoided
```

Diagnostics must not themselves become an expensive polling loop.

### 7. Add deterministic scheduler tests

Tests must cover:

- noisy raw events with unchanged semantic watermark;
- simultaneous island and main-card refresh;
- manual Continue arriving during background Continue;
- capture arriving during audit export;
- maintenance requested during a manual action;
- cancellation before persistence;
- stale result rejection;
- shutdown with queued work;
- unchanged evidence producing no new decision rows;
- failure of one work class not poisoning the coordinator.

## Performance gates

Establish reproducible measurements on the development Mac. At minimum prove:

- 30 quiet minutes create no repeated unchanged Continue decisions;
- status heartbeat latency remains bounded as the fixture database grows;
- manual Continue begins promptly even when low-priority work is queued;
- only one Continue computation and one audit export run at a time;
- memory does not show monotonic growth during the quiet run;
- CPU returns to a low baseline after each bounded operation.

Record hardware, build mode, sample interval, and exact commands. Do not use one instantaneous Activity Monitor screenshot as the whole proof.

## Acceptance criteria

- One coordinator governs Continue across main card, island, startup, background, and manual paths.
- Raw low-value event growth alone does not recompute Continue.
- Expensive work has explicit concurrency and priority rules.
- Manual actions are not trapped behind audit export or cleanup.
- Cancelled or superseded work does not persist duplicate decisions or derived rows.
- Status polling remains cheap on a large fixture.
- Runtime diagnostics explain queued, active, coalesced, and cancelled work.
- Quiet-run CPU and memory return to a stable baseline.
- Automated tests and live 30-minute verification pass.

## Verification commands

Run at minimum:

```bash
cd src-tauri && cargo fmt --check
cd src-tauri && cargo check
cd src-tauri && cargo test
npm run build
npm run tauri dev
git diff --check
git status --short
```

Also run and report the scheduler stress test, large-database status benchmark, semantic-watermark reuse test, and 30-minute quiet runtime measurement.

## Final response format

Report:

1. Work classes, priorities, and concurrency rules.
2. Semantic invalidation contract.
3. Main and island single-flight behavior.
4. Cancellation and stale-result policy.
5. Status-query cost.
6. Quiet-run CPU, memory, row-growth, and operation counts.
7. Commands and tests.
8. Remaining scheduling limitations.

