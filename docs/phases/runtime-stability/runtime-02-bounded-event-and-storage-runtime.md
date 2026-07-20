# Runtime Stability 02 — Enforce Bounded Event And Storage Growth

## Codex task

Make continuous capture converge to a bounded steady state. Enforce event and derived-data retention during normal runtime, preserve the causal evidence required by Continue, and provide a safe migration path for an already oversized local database.

Complete Runtime Stability 01 first. This session must not redesign `continue_outputs`; that is Runtime Stability 03.

## Read before editing

```text
AGENTS.md
PRODUCT.md
docs/phases/runtime-stability/runtime-00-always-on-stability-program.md
docs/phases/runtime-stability/runtime-01-screen-capture-crash-containment.md
docs/data-capture-and-processing.md
docs/full-engine-flow.md
src-tauri/src/capture.rs
src-tauri/src/capture_core/event_governor.rs
src-tauri/src/capture_core/store.rs
src-tauri/scripts/capture_events.swift
src-tauri/src/continuation.rs
src-tauri/src/continuation/task_turn_evidence.rs
```

Inspect the live schema and row counts read-only. Use a copied or synthetic database for deletion and migration tests.

## Verified failure

The implementation declares `MAX_RETAINED_LOW_VALUE_UI_EVENTS = 5_000`, but this is currently used to identify rows only when `cleanup_local_memory` is invoked. It does not enforce retention during ordinary capture.

The diagnosed database contained 32,732 UI events and 20,192 low-value events beyond the declared budget. The latest 15.3-minute session added 1,128 events even though the user described little active work.

This is not a SQLite corruption problem. It is a lifecycle and retention-policy problem.

## Required implementation

### 1. Define explicit runtime budgets

Create a versioned retention policy covering at least:

```text
low-value UI events
high-value causal events
capture triggers
typing-burst metadata
frames and image assets
OCR rows and spans
Accessibility nodes
content units
window snapshots
derived Continue rows
decision history
```

Budgets may use count, age, and byte limits. Document why each class is retained and what protects it from deletion.

Do not treat every row linked to any historical decision as permanently protected. Protection must be bounded and tied to current product value, reviewed or pinned evidence, active task state, or an explicit developer-retention policy.

### 2. Enforce retention incrementally

Add low-cost automatic maintenance triggered by bounded conditions such as:

- every N accepted events;
- every N stored frames;
- a minimum elapsed maintenance interval;
- database or snapshot byte thresholds;
- capture-session stop.

Do not run a full-table cleanup on every event. Maintenance must be chunked, indexed, interruptible, and idempotent.

Keep the capture hot path short. If maintenance runs asynchronously, ensure only one maintenance job exists and that shutdown waits or cancels safely.

### 3. Preserve causal evidence

Before deleting an event or frame, account for references from:

```text
capture_triggers
event_transitions
typing_bursts
task-turn evidence
task actions
current task snapshots and checkpoints
feedback
open events
reviewed or pinned evidence
```

Use database constraints or an explicit protection query with tests. Do not leave dangling identifiers that make Continue explanations unverifiable.

Where detailed old rows are removed, retain only a compact privacy-safe summary when the product still needs longitudinal counts or task boundaries.

### 4. Control source pressure

Measure event types and rates by application and surface. Improve coalescing for noisy Accessibility notifications without hiding meaningful app, window, navigation, commit, error, or task-transition signals.

The source and Rust ingest gates must agree on dedupe keys and time windows. Add diagnostics for received, dropped-at-source, dropped-at-ingest, persisted, and promoted-to-capture counts.

### 5. Provide safe existing-database remediation

Implement a dry-run-first cleanup path that reports:

```text
rows by class before
protected rows
candidate rows
estimated bytes
actual deleted rows
orphaned assets
database bytes before and after
whether VACUUM is recommended or performed
```

Never automatically delete the live database or all screenshots. Destructive compaction must require the existing explicit user action or a clearly documented safe automatic retention policy.

Migration must be restart-safe. A crash halfway through cleanup must not corrupt the database.

### 6. Add growth and query-cost tests

Add deterministic tests that ingest a large synthetic event stream and prove:

- retained low-value events converge to the configured bound;
- high-value causal events remain available;
- protected references remain valid;
- repeated maintenance is idempotent;
- cleanup runs in chunks;
- unchanged duplicate surfaces do not create unbounded frames or derived rows;
- query plans use expected indexes;
- a copied oversized database can be remediated safely.

## Acceptance criteria

- The declared retention limits are enforced automatically, not only displayed in diagnostics.
- A 60-minute synthetic noisy stream converges to bounded row counts.
- A 30-minute quiet live run does not show an accelerating database-growth curve.
- No causal evidence required by the current Continue answer is deleted.
- No dangling references remain after maintenance.
- Source and ingest drop counters make event pressure explainable.
- Existing oversized databases have a dry-run and safe cleanup path.
- SQLite integrity checks pass before and after fixture remediation.
- Full Rust tests and frontend build pass.

## Verification commands

Run at minimum:

```bash
cd src-tauri && cargo fmt --check
cd src-tauri && cargo check
cd src-tauri && cargo test
npm run build
git diff --check
git status --short
```

Also run and report:

```text
the synthetic 60-minute-equivalent event-pressure test
the copied-database remediation test
PRAGMA quick_check on the copied database before and after cleanup
the 30-minute live quiet-growth measurement
```

## Final response format

Report:

1. Retention policy by data class.
2. Automatic-maintenance trigger and chunking design.
3. Causal-evidence protection rules.
4. Event-rate and drop-counter results.
5. Synthetic steady-state results.
6. Copied-database remediation results.
7. Live 30-minute database-growth results.
8. Commands and tests.
9. Remaining retention risks.

