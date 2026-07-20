# Always-On Repair 02 — Bound Runtime Pressure And Prove The Product Can Stay Open

## Codex task

Remove the accumulating pressure that can make Smalltalk slow down, consume increasing memory, or appear bricked after several capture cycles. Bound the UI-event pipeline, remove schema work from database hot paths, make background workload queues cancellable and finite, keep status refresh cheap, preserve causal evidence, and run the complete always-on release proof.

This is the second and final prompt in this two-session repair pack. It assumes `always-on-repair-01-fail-safe-capture-runtime.md` has passed its automated gates. If Prompt 01 is incomplete or contradicted by current code, fix that prerequisite within its contract before continuing.

Do not declare success based on unit tests alone. This prompt ends only with a reproducible stress and soak result, or with a precise list of live gates the user explicitly chose to run themselves.

This session owns event backpressure, database lifecycle, transactional batching, workload scheduling, status cost, audit and maintenance concurrency, bounded retention compatibility, integrated pressure tests, and the final soak proof. Do not replace Prompt 01's capture-provider architecture merely to simplify this work. Reopen it only if a reproducible regression violates Prompt 01's contract.

## Why this session exists

The latest investigation found these concrete pressure paths:

1. The native event helper feeds Rust through an unbounded `std::sync::mpsc::channel`.
2. The capture loop drains the queue without an event-count or time budget.
3. Ordinary key-down and click events are emitted individually; source and ingest coalescing mainly protect scroll and Accessibility noise.
4. Every accepted event calls `open_db`, and `open_db` executes `init_db`.
5. `ensure_db` calls `open_db` and then calls `init_db` a second time.
6. Schema creation, column discovery, migrations, and the large Continue schema check therefore run repeatedly on event and capture paths.
7. Each event performs several separate writes rather than one bounded batch.
8. Background Continue, capture, maintenance, audit export, status refresh, and native-island refresh can all request work during the same period.
9. The workload governor has an unbounded waiting queue and no production shutdown or cancellation path.
10. Full audits spawn a detached thread before acquiring an audit permit.
11. Frequent status refresh reconstructs session counts and the latest frame and sends it to both React and the native island.

The current small live database was healthy during the investigation, so do not mislabel this as proven SQLite corruption or a simple five-minute memory leak. The failure family is accumulated unbounded work plus expensive hot-path behavior.

## Read before editing

Read these files completely:

```text
AGENTS.md
PRODUCT.md
docs/phases/runtime-stability/always-on-repair-01-fail-safe-capture-runtime.md
docs/phases/runtime-stability/runtime-00-always-on-stability-program.md
docs/phases/runtime-stability/runtime-02-bounded-event-and-storage-runtime.md
docs/phases/runtime-stability/runtime-03-bounded-continue-audit-exports.md
docs/phases/runtime-stability/runtime-04-background-workload-governor.md
docs/phases/runtime-stability/runtime-05-soak-test-and-release-gate.md
docs/runtime-retention-policy-v1.md
docs/full-engine-flow.md
docs/data-capture-and-processing.md
src-tauri/scripts/capture_events.swift
src-tauri/src/capture.rs
src-tauri/src/capture_core/
src-tauri/src/workload.rs
src-tauri/src/continuation.rs
src-tauri/src/continuation/
src-tauri/src/session_island.rs
src-tauri/src/session_island/
src/App.tsx
```

Before editing, inspect and record:

```text
git status --short
current user changes in every file you may touch
the real capture-event producer and consumer call graph
every production call to open_db, ensure_db, init_db, and ensure_continue_schema
all WorkClass acquisition sites
all detached thread::spawn call sites reachable during recording
all capture-status and Continue refresh triggers
the live database schema, row counts, journal files, and quick_check, read-only
current database, snapshot, audit, process, thread, and file-descriptor baselines
```

Use copied or synthetic databases for mutation, stress, retention, and migration tests. Never run destructive tests against the user's live data.

## Non-negotiable product-behavior contract

This session may change scheduling and storage mechanics. It must not change the product meaning.

Preserve:

- Continue as the primary product primitive.
- Capture while Continue is used; Stop is not a prerequisite.
- Event-driven still capture rather than continuous recording.
- Existing meaningful app, window, navigation, typing-burst, click, commit, error, and task-transition evidence.
- Stable frame, event, trigger, action, artifact, decision, and evidence identifiers.
- The distinction between current focus and return target.
- Evidence-backed abstention when evidence is thin.
- Main-card and native-island decision identity and behavior.
- Current privacy exclusions and redaction.
- No raw typed characters or full clipboard content.
- Smalltalk self-observation suppression.
- The versioned retention policy and its causal-reference protections.
- Truthful provider and capture metadata from Prompt 01.
- Existing public command and serialized response contracts unless all callers are migrated together and regression-tested.
- Manual model calls remaining manual-only unless the product contract explicitly says otherwise.

Do not improve stability by discarding evidence that Continue currently needs. Every coalescing, batching, retention, caching, and scheduling decision must include a preservation test.

## Required implementation

### 1. Replace the unbounded event channel with explicit backpressure

Introduce a bounded event transport between `capture_events.swift` and the Rust capture runtime.

The design must define:

```text
maximum queue capacity
event priority classes
overflow behavior by class
coalescing key by event class
maximum coalescing window
maximum drain count per loop turn
maximum drain time per loop turn
diagnostic counters
shutdown behavior
```

Do not use “drop newest” or “drop oldest” as one global policy.

Classify events by product value. Derive the exact mapping from current code and tests, but follow these principles:

- App changes, window changes, navigation or surface identity changes, errors, permission changes, explicit commits, and events already referenced by causal objects are high value.
- Repeated scroll, repeated Accessibility value changes, key auto-repeat, and ordinary character-category key events are coalescible pressure.
- Clicks and typing evidence may be summarized into bounded bursts only if timestamps, counts, surface identity, commit boundaries, and causal linkage required by Continue remain available.
- Overflow must preserve a privacy-safe aggregate explaining what was coalesced or dropped.
- High-value events must have reserved capacity or a separate bounded lane so noisy low-value traffic cannot starve them.

The capture loop must return to cancellation checks, pending capture settlement, and idle scheduling after a bounded amount of ingest work. A continuously busy producer must not prevent the two-minute idle boundary or Stop from being serviced.

Source and Rust gates must share documented coalescing semantics. Do not maintain two silently contradictory policies.

### 2. Remove schema work from event, capture, and status hot paths

Redesign database lifecycle so schema creation and migrations happen at an explicit initialization boundary, not every time a function needs a connection.

Required behavior:

1. App startup or an explicit database-open service initializes and migrates the selected database once per database generation.
2. Reset, replacement, test fixture, and path changes create a new generation and re-run initialization safely.
3. Do not use one naive process-global `OnceLock` that breaks tests or database replacement.
4. Event ingest, frame capture, status, counters, privacy evaluation, and Continue queries do not execute DDL or schema discovery after initialization.
5. `ensure_db` must not initialize the same connection twice.
6. The capture runtime uses a clearly owned long-lived connection, a small bounded pool, or a dedicated database worker. Choose one design and document ownership.
7. Do not hold SQLite transactions or connections while waiting in the workload governor.
8. Read-only status operations remain read-only.
9. Database replacement and migrations remain restart-safe.
10. Existing tests using temporary databases remain isolated.

Add instrumentation that counts schema initialization and migration executions. An event storm must not increase this count.

### 3. Batch event persistence into short transactions

Replace multiple independent per-event writes with bounded batches where doing so preserves causal ordering.

Each batch must:

- have a maximum event count and time window;
- preserve deterministic ordering;
- atomically persist the event, required aggregate, counter changes, typing-burst effects, and capture-trigger relationship where practical;
- use prepared statements or equivalent reusable query plans;
- keep transactions short;
- yield between batches;
- respond to cancellation;
- retry SQLite busy responses only within a bounded policy;
- report busy duration and retry count;
- never leave a half-linked causal object.

Do not use the current 30-second SQLite busy timeout as the main coordination strategy. Long waits must not make the product appear frozen.

### 4. Bound and complete the workload governor

The workload coordinator must become a production lifecycle component, not an unbounded condition-variable queue.

Required behavior:

```text
finite queue capacity by work class
request deadline
cancellation token
coalescing identity
supersede policy
shutdown and drain policy
no database resource held while queued
safe failure isolation
privacy-safe diagnostics
```

Preserve these priorities:

1. Capture cancellation and app shutdown.
2. Explicit user actions, including manual Continue.
3. Required screenshot capture and evidence persistence.
4. Cheap status and event ingest.
5. Background Continue and derived rebuilding.
6. Audit export and maintenance.

Specific guarantees:

- At most one Continue computation runs across main card, island, startup, and background paths.
- Unchanged semantic evidence returns or reuses the existing decision.
- Low-value raw-event count changes alone do not invalidate Continue.
- Manual Continue supersedes queued background Continue.
- A superseded or cancelled background result cannot persist duplicate derived rows or replace a newer manual result.
- A stalled background operation cannot block capture indefinitely.
- Capture cannot create an unbounded queue of Continue work.
- Shutdown cancels queued work and wakes every waiter.
- Failure of one work class does not poison the coordinator.

Add production tests for queue saturation, cancellation, superseding, shutdown, and recovery.

### 5. Make audit and maintenance execution single-flight

Do not spawn a detached operating-system thread for every audit request before capacity is granted.

Required behavior:

- Full audit export has one bounded executor and at most one active export.
- Equivalent queued exports coalesce.
- Background and startup Continue never enable full audit output.
- Manual MFTI review artifacts remain bounded and decision-scoped.
- Maintenance has one owner and one pending request.
- Audit and maintenance cancellation is safe at shutdown.
- Interrupted temporary output is cleaned or marked recoverable without deleting reviewed evidence.
- The current retention policy continues to protect current causal evidence.
- Audit and maintenance do not overlap capture or manual Continue in a way that creates long SQLite waits.

Preserve explicit full-forensic mode, but make its cost and concurrency truthful.

### 6. Make capture status bounded without breaking callers

Profile `capture_status_snapshot_inner`, React polling, capture-frame listeners, capture-status listeners, and native-island refresh.

Required behavior:

1. Status cost is independent of total historical table size.
2. Status does not repeatedly scan or reconstruct Continue state.
3. Frequently requested counts come from indexed bounded queries, maintained summaries, or a safe in-memory snapshot.
4. The latest-frame payload does not repeatedly serialize unbounded OCR or Accessibility trees merely to update a badge or island state.
5. Heavy frame detail is loaded only by an explicit detail or evidence request where possible.
6. Existing `CaptureStatus` consumers continue to receive every field they require. If a payload is split, migrate React and native-island callers atomically.
7. Event-driven updates are coalesced.
8. Keep a slow heartbeat as recovery insurance, but do not use it as the main work scheduler.
9. Developer diagnostics do not turn each status update into multiple overlapping queries.

Add a large-database benchmark and response-size assertion. Record p50 and p95 latency rather than one successful call.

### 7. Preserve bounded retention and causal evidence

Re-verify `smalltalk.retention.v1` after changing event aggregation and database ownership.

Tests must prove:

- low-value retained rows converge to their declared limit;
- high-value and currently referenced evidence remains;
- aggregate events retain required causal ids or an explicit replacement mapping;
- capture triggers, transitions, typing bursts, task-turn evidence, task actions, feedback, open events, and current Continue evidence do not dangle;
- frame child rows and image assets follow parent protection rules;
- repeated maintenance remains idempotent;
- cleanup remains chunked;
- no automatic `VACUUM` runs during capture;
- a copied oversized database can be opened and remediated safely;
- `PRAGMA quick_check` passes before and after remediation.

Do not broaden retention indefinitely to avoid writing reference-preservation logic.

### 8. Add one reproducible always-on harness

Implement the developer-only harness required by `runtime-05-soak-test-and-release-gate.md`. It must record fixed-interval, privacy-safe measurements without collecting captured contents.

Record at least:

```text
main process alive and restart count
capture runtime state
child helper count and abnormal exits
CPU and resident memory
thread count
open file descriptor count when available
event queue depth, capacity, high-water mark, coalesced count, and dropped count
workload active and queued counts
database busy time and retry count
schema initialization count
database and WAL bytes
snapshot and audit bytes
important table row counts
capture attempts, stores, skips, timeouts, and cancellations
OCR attempts and failures
Continue requests, computations, cache hits, and superseded requests
status p50 and p95 latency and response bytes
Stop latency
session state
crash-report baseline and delta
```

Write a machine-readable report plus a concise Markdown summary. The harness must not contain raw screenshot data, OCR or Accessibility text, captured window titles, URLs, document paths, typed text, or clipboard contents.

### 9. Add deterministic pressure and integration tests

At minimum cover:

```text
60-minute-equivalent high-rate event production
queue saturation with protected high-value events
continuous producer while idle capture becomes due
continuous producer while Stop is requested
typing burst aggregation and commit boundaries
schema initialization count under event load
batch rollback and retry
SQLite busy fault
large-database status benchmark
background Continue saturation
manual Continue superseding background work
capture arriving during audit and maintenance
shutdown with every queue populated
stale background result rejection
unchanged evidence producing no duplicate decision rows
retention after aggregation
copied oversized database remediation
Prompt 01 helper timeout and cancellation faults under pipeline pressure
clean Stop, restart, and new session
```

Tests must assert bounded memory structures directly. A test that merely completes is insufficient.

### 10. Run product-parity regression checks

Stability is not a pass if Continue becomes less truthful or loses its return evidence.

Against representative existing fixtures and a live manual sample, verify that the hardened runtime still provides:

```text
what the user was doing
where they were doing it
the last meaningful progress
the pending intention when evidence supports it
the work object and exact locus when available
the distinction between current focus and return target
honest uncertainty when evidence is weak
inspectable frame, event, artifact, and action evidence
the same decision identity on the main card and native island
```

Run the existing Continue accuracy, task-turn, privacy, provider-provenance, session-island, and release tests affected by the change. Do not rewrite expected results merely to make the suite green without explaining the product effect.

## Forbidden shortcuts

Do not:

- replace one unbounded queue with another unbounded thread or task list;
- drop all key, click, Accessibility, or scroll evidence;
- increase capture intervals to hide backlog;
- run schema creation or migration in event, frame, status, or counter functions;
- use a naive global one-time initializer that fails after database reset;
- hold a database connection or transaction while waiting for a work permit;
- use the 30-second SQLite timeout as workload scheduling;
- add one global mutex around the whole application;
- let background work outrank explicit user actions;
- spawn one thread per audit, maintenance, or timed-out probe;
- remove the native island or background Continue to lower activity;
- disable audit evidence globally;
- weaken retention references;
- delete or compact the live database during implementation;
- include private captured data in diagnostics or soak reports;
- claim a planned soak threshold passed without running it;
- treat compilation as proof of always-on stability.

## Quantitative release policy

Create or update one versioned runtime-stability policy used by tests and the harness. It must set explicit pass or fail thresholds for:

```text
new helper crash reports
new main-process crash reports
main-process restarts
unclean sessions
event and workload queue capacity
queue high-water marks
unhandled helper timeouts
Stop latency
status latency and response size
manual action queue delay
database busy time
schema initialization count
event, frame, derived-row, snapshot, and audit growth
memory behavior after warm-up
quiet CPU recovery
unchanged-evidence duplicate decisions
orphaned child processes, threads, assets, and database references
```

Derive resource thresholds from a measured baseline on the development Mac and explain them. The following are absolute gates and may not be relaxed:

- zero new main-process crash reports;
- zero main-process restarts;
- zero incorrectly running or interrupted sessions after a clean Stop;
- zero orphaned helper processes after cancellation or Stop;
- zero unbounded in-memory queues;
- zero schema initialization calls caused by ordinary event ingestion after startup;
- zero duplicate persisted Continue decisions for unchanged semantic evidence;
- `PRAGMA quick_check` returns `ok`;
- all current product-behavior invariants remain true.

## Required verification

### Automated commands

Run at minimum:

```bash
cd src-tauri && cargo fmt --check
cd src-tauri && cargo check
cd src-tauri && cargo test
swiftc -typecheck src-tauri/scripts/capture_events.swift
npm run build
git diff --check
git status --short
```

Run the focused queue, database lifecycle, workload, audit, status, retention, fault-injection, and product-parity tests individually and report their exact names.

### Synthetic pressure gates

Run and record:

1. A 60-minute-equivalent noisy event stream.
2. A mixed high-value and low-value saturation stream.
3. A large live-shaped database status benchmark.
4. Workload queue saturation with manual superseding.
5. Prompt 01 helper faults while queues are busy.
6. Copied oversized database remediation with integrity checks.
7. Clean Stop and restart with all queues previously occupied.

The test fails if memory structures, retained rows, threads, helpers, audit workers, schema initialization, or derived decisions grow without a configured bound.

### Live soak matrix

Run the normal product path with `npm run tauri dev`. Complete all scenarios from Runtime Stability 05:

#### A. Quiet layer

60 minutes recording with minimal interaction. Include captures around two and four minutes and continue beyond warm-up.

#### B. Normal mixed work

60 minutes of browser research, editor work, terminal use, typing, scrolling, app switching, and idle periods.

#### C. Continue stress

30 minutes with controlled repeated manual Continue requests while capture remains active.

#### D. Native window churn

30 minutes covering close and open, minimize and restore, Spaces or Mission Control, foreground and background, and display changes when available.

#### E. Fault recovery

Run every development-only helper, OCR, database-busy, audit, background-cancellation, and shutdown fault.

#### F. Clean Stop and restart

Stop capture, quit normally, restart, inspect the previous session, create a new session, and verify both statuses.

For every scenario, report duration, build mode, machine and macOS version, sample interval, crash-report delta, CPU, memory, queues, threads, helpers, file descriptors, database growth, row growth, status latency, Continue reuse, and Stop behavior.

If the user says they will perform the live soak themselves, provide the exact harness command and expected report locations, stop before the live run, and mark every live gate as pending. Do not call the product always-on stable.

## Acceptance criteria

This prompt passes only when:

- Event transport and workload queues are finite and observable.
- Low-value event pressure cannot starve high-value events, capture scheduling, or Stop.
- Event batching preserves typing, transition, trigger, and Continue causal semantics.
- Schema and migration work is absent from ordinary event, capture, status, and counter hot paths.
- The database is initialized exactly at its explicit lifecycle boundary.
- SQLite waits and retries are bounded and observable.
- Background Continue, audit, and maintenance cannot block capture or manual Continue indefinitely.
- Shutdown cancels and drains every queue without leaking threads or helpers.
- Status latency and payload size remain bounded on a large fixture.
- Audit export and maintenance are single-flight.
- Retention converges and preserves current causal evidence.
- Unchanged semantic evidence creates no duplicate persisted decisions.
- Prompt 01 failure isolation still works under pressure.
- The main card and native island remain consistent.
- Continue behavior and evidence quality do not regress.
- Synthetic pressure gates pass.
- The complete live soak matrix passes with zero new main-process or helper crash reports and zero process restarts.
- Memory does not show sustained monotonic growth after warm-up.
- CPU returns to the documented quiet baseline after bounded operations.
- Clean Stop and restart leave truthful session states.
- All automated checks pass.
- Documentation describes the implemented runtime truth.

If any acceptance criterion fails, keep the task active. Identify the responsible subsystem, fix it, rerun its focused tests, and rerun the failed pressure or soak scenario.

## Final response format

Report in this order:

1. Event queue capacity, priority, coalescing, overflow, and drain policy.
2. Database initialization, connection ownership, transaction, busy, and migration design.
3. Workload queue, cancellation, superseding, audit, and maintenance behavior.
4. Status-query cost and payload changes.
5. Retention and causal-evidence preservation results.
6. Product-behavior invariant results.
7. Synthetic pressure results with before and after metrics.
8. Every live soak scenario with duration and pass or fail status.
9. Crash, restart, session, memory, CPU, thread, helper, queue, database, and latency results.
10. Fault-injection and recovery results.
11. Automated commands and exact focused test names.
12. Documentation updates.
13. Any pending manual gate or remaining limitation.

Do not describe Smalltalk as always-on stable until every required live release gate is proven.
