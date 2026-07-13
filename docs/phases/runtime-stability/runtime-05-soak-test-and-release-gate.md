# Runtime Stability 05 — Prove Smalltalk Can Remain Open

## Codex task

Build and run the final runtime-stability release gate. Prove that the native app can remain open through quiet periods, normal work, repeated Continue usage, application switching, capture failures, and shutdown without crashing, leaking work, or growing storage without bounds.

This is a proof and hardening session. Complete Runtime Stability 01–04 first. If a prerequisite is incomplete or contradicted by the soak, fix it within its original contract and rerun the relevant verification.

## Read before editing

```text
AGENTS.md
PRODUCT.md
docs/phases/runtime-stability/
docs/data-capture-and-processing.md
docs/full-engine-flow.md
docs/smalltalk-capture-technical-details.md
docs/phases/p6-task-turn-accuracy/p6-09-longitudinal-release-gate.md
src/App.tsx
src-tauri/src/capture.rs
src-tauri/src/session_island/
src-tauri/src/continuation.rs
```

Inspect current source, tests, live diagnostics, generated developer outputs, crash-report counts, and git status. Do not assume previous session claims remain true.

## Required implementation

### 1. Add a reproducible soak harness

Create a developer-only harness that samples privacy-safe runtime metrics without collecting raw screen contents. It must record at fixed intervals:

```text
main process alive and restarts
child helper launches and abnormal exits
CPU and resident memory
thread and process count
open file descriptors when available
database and WAL bytes
snapshot bytes
audit-output bytes
row counts by important table class
active and queued workload counts
capture attempts, stores, and skips
OCR runs
Continue requests, computations, and cache reuse
audit export state
UI command latency probes
crash-report count baseline and delta
```

Write a machine-readable report and a concise Markdown summary. Never include raw OCR, Accessibility text, typed characters, URLs, file paths from captures, screenshots, or clipboard contents.

### 2. Add fault injection

Provide deterministic development-only fault injection for:

- screenshot helper abnormal exit;
- helper timeout;
- OCR failure;
- SQLite busy response;
- audit export interruption;
- background Continue cancellation;
- application shutdown with queued work.

Fault injection must be impossible to enable accidentally in a production build.

### 3. Define the release budgets

Create a versioned runtime-stability policy and report schema. Include explicit pass or fail thresholds for:

```text
new native crash reports: zero
main process restarts: zero
unhandled helper abnormal exits: zero
maximum active operation concurrency by class
event, frame, and derived-row retention
database and audit-output growth
memory growth after warm-up
CPU quiet baseline and recovery
status and manual-command latency
unchanged-evidence duplicate decisions
unclean session termination
```

Use measured values and explain the thresholds. Do not make them so loose that the diagnosed failure would pass.

### 4. Run the soak matrix

Run at least these scenarios:

#### A. Quiet layer

60 minutes with Smalltalk recording and minimal user interaction.

#### B. Normal mixed work

60 minutes covering browser research, editor work, terminal activity, typing, scrolling, app switching, and periods of inactivity.

#### C. Continue stress

30 minutes with controlled manual Continue requests, including repeated clicks, while capture remains active. Verify audit coalescing and decision reuse.

#### D. Native window churn

30 minutes covering window close and open, minimize and restore, Spaces or Mission Control, display changes when available, and Smalltalk foreground and background transitions.

#### E. Fault recovery

Run every supported injected failure and prove recovery without process restart or corrupt state.

#### F. Clean stop and restart

Stop capture, quit normally, restart, inspect prior session status, start a new session, and verify that neither session is incorrectly marked `interrupted`.

The scenarios may be separated to make failures easier to attribute, but all required durations and checks must be completed.

### 5. Verify product behavior, not only resource graphs

During the mixed-work and Continue-stress scenarios, review representative Continue results. Confirm that stability fixes did not remove the evidence needed to say:

```text
what the user was doing
where they were doing it
what state was left
what remains uncertain
whether a safe return target exists
```

This phase does not declare semantic accuracy solved. It proves that runtime hardening did not obviously degrade the existing answer and that audit evidence remains inspectable.

### 6. Update documentation to truth

Update `PRODUCT.md` and the relevant capture and runtime documentation with:

- provider fallback and circuit-breaker behavior;
- automatic retention policy;
- standard versus forensic audit modes;
- background workload rules;
- runtime diagnostic and soak commands;
- known hardware and macOS limitations;
- the exact release-gate status.

Do not describe a planned threshold or incomplete manual run as passed.

## Acceptance criteria

The runtime-stability program passes only when:

- all five soak scenarios and clean restart complete;
- zero new `sck_screenshot` and main-process crash reports appear;
- zero main-process restarts occur;
- zero sessions are incorrectly left `running` or marked `interrupted` after clean stop;
- storage and retained rows stay within the versioned budgets;
- standard audits remain within their size and concurrency budgets;
- unchanged evidence creates no duplicate persisted decisions;
- CPU returns to the quiet baseline after bounded operations;
- memory does not show sustained monotonic growth after warm-up;
- manual Continue remains responsive during capture and audit activity;
- fault injection recovers without corruption;
- SQLite integrity checks pass;
- the main card and island remain consistent;
- all automated checks pass;
- documentation matches implementation;
- no required runtime-stability work remains.

If any gate fails, keep the task active. Identify the responsible earlier phase, fix it, rerun its focused tests, and rerun the failed soak scenario.

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

Also run:

```text
the complete soak harness
every runtime fault-injection test
PRAGMA quick_check against the soak database
the existing Continue accuracy and release tests affected by retention or scheduling
the main-card and island parity tests
```

## Final response format

Report:

1. Requirement matrix for Runtime Stability 01–04.
2. Hardware, macOS version, build mode, and measurement method.
3. Every soak scenario with duration and pass or fail result.
4. Crash-report baseline and delta.
5. CPU, memory, process, thread, and latency results.
6. Database, snapshot, event, derived-row, and audit-output growth.
7. Continue computation, reuse, and audit-concurrency results.
8. Fault-injection results.
9. Clean stop, restart, and session-status result.
10. Product-behavior spot checks.
11. Automated commands and results.
12. Documentation updates.
13. Remaining limitations.

Do not call Smalltalk always-on stable unless every release gate above is proven.

