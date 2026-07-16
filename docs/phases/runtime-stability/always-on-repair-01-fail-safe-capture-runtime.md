# Always-On Repair 01 — Make Capture Fail Safe Without Changing Product Behavior

## Codex task

Make Smalltalk's native capture runtime incapable of killing or permanently blocking the main application. Restore a real failure-isolation boundary around macOS screen capture, put a deadline and cancellation path around every external helper, and make capture start, degradation, stop, panic, and restart transitions deterministic.

This is an implementation session, not a research-only session. Work through every requirement in order. Do not stop after changing one screenshot function or after compilation succeeds.

This prompt is the first of two ordered repair sessions. Complete this file before starting `always-on-repair-02-bounded-pipeline-and-release-proof.md`.

This session owns native capture, helper lifecycle, capture-worker state, cancellation, Stop, shutdown, and provider-failure recovery. Do not redesign event retention, database connection ownership, status-query shape, Continue scheduling, audit execution, or the workload queue here except for the narrow interfaces required to make capture cancellation and shutdown correct. Prompt 02 owns those systems.

## Why this session exists

The latest read-only investigation did not find a literal five-minute timer or a fresh crash report for the reported incident. It did find several concrete failure paths:

1. Normal macOS screenshots run inside the main Tauri process through `capture_core_graphics_in_process`.
2. The in-process display and active-window branches return before the older helper timeout, reaping, fallback, and circuit-breaker code can run.
3. A native assertion, abort, or process-level failure in this path can therefore terminate Smalltalk itself.
4. Accessibility, window-snapshot, Vision OCR, Tesseract, AppleScript, and command-line screenshot calls use blocking process waits without a shared timeout and cancellation contract.
5. `stop_runtime` signals cancellation and then waits for the capture worker without a deadline. A blocked helper can make Stop wait forever.
6. Capture-loop cleanup runs only on the normal exit path. A worker panic can leave runtime and session state inconsistent.
7. The apparent four-to-five-minute boundary can correspond to the initial capture plus idle captures around two and four minutes encountering a bad native or helper condition.

Treat these as the starting diagnosis, not as permission to skip live verification. Re-check the current checkout and current artifacts before editing because the worktree is active and may have changed.

## Read before editing

Read these files completely before changing code:

```text
AGENTS.md
PRODUCT.md
docs/phases/runtime-stability/runtime-00-always-on-stability-program.md
docs/phases/runtime-stability/runtime-01-screen-capture-crash-containment.md
docs/phases/runtime-stability/runtime-05-soak-test-and-release-gate.md
docs/data-capture-and-processing.md
docs/smalltalk-capture-technical-details.md
src-tauri/src/capture.rs
src-tauri/src/workload.rs
src-tauri/src/capture/swift_helpers.rs
src-tauri/scripts/sck_screenshot.swift
src-tauri/scripts/accessibility_snapshot.swift
src-tauri/scripts/window_snapshot.swift
src-tauri/scripts/vision_ocr.swift
src-tauri/src/session_island.rs
src-tauri/src/session_island/
src/App.tsx
```

Also inspect:

```text
git status --short
git diff -- src-tauri/src/capture.rs src/App.tsx
the current capture-provider rows in the live SQLite database, read-only
the current helper and app crash-report count under ~/Library/Logs/DiagnosticReports, read-only
the packaged helper/signing and Screen Recording permission identity
```

Preserve every unrelated worktree change. Do not reset, clean, overwrite, or reformat unrelated files.

## Non-negotiable product-behavior contract

Runtime hardening must not quietly change what Smalltalk does.

Preserve all of the following:

- Smalltalk remains a native, local-first, Continue-first product.
- Continue does not require Stop Session.
- Capture remains event-driven still-image evidence. Do not turn it into continuous video recording.
- Keep the existing initial, event-triggered, low-value, important, surface-change, manual-Continue, and idle capture semantics unless a change is required for safety and is explicitly tested.
- Continue must still distinguish factual current focus from an actionable return target.
- Existing frame, event, artifact, action, decision, and evidence identifiers remain explainable.
- Provider metadata must name the provider that actually produced each image.
- Full-display and active-window attribution, geometry, scale, privacy status, and provenance remain truthful.
- Existing Smalltalk self-exclusion remains effective.
- Do not store raw typed characters or full clipboard contents.
- Do not weaken privacy exclusions, redaction, causal evidence, or retention protections to improve performance numbers.
- Do not remove Accessibility or OCR evidence globally as a shortcut.
- Do not hide a failure by silently disabling capture for the rest of the app lifetime.
- The React card and native island must continue to observe the same capture/session state.
- Preserve existing command names and serialized response fields unless all callers are migrated atomically and regression-tested.
- Stay in the native Tauri/macOS MVP lane. Do not revive the browser extension.

Before implementation, write a short behavior-invariant checklist in the session notes. Use it again during verification.

## Required implementation

### 1. Establish one safe native-capture architecture

Trace the real production call graph from `capture_frame` to the provider that creates the image. Remove any normal production path where an abort-prone or indefinitely blocking native capture operation runs inside the main Smalltalk process.

The final design must satisfy these rules:

1. Native screen-image acquisition runs behind a process boundary that can fail without terminating the Tauri process.
2. The production provider uses current ScreenCaptureKit behavior where supported. Do not preserve deprecated in-process Core Graphics calls merely because they are convenient.
3. Active-window capture must not restore the known-crashing `SCContentFilter(desktopIndependentWindow:)` construction.
4. Prefer the previously proven display-capture-and-validated-crop strategy for active-window evidence unless current Apple behavior and tests prove a safer alternative.
5. Validate stale, empty, minimized, off-screen, cross-display, and closed-window geometry before native filter construction or crop work.
6. Smalltalk windows remain excluded.
7. The helper or sidecar must use a stable packaged identity and a documented Screen Recording permission model. Do not trade crash isolation for a permission workaround.
8. Keep still capture discrete. Do not add an always-running high-frame-rate stream unless the existing product contract is explicitly revised.

If a correctly signed helper, sidecar, XPC service, or other isolated architecture is needed, implement the smallest maintainable option that works in both `npm run tauri dev` and the packaged app. Explain the process and permission identities in code comments and technical documentation.

Delete or make structurally unreachable any superseded unsafe provider route. A later maintainer should not be able to re-enable it accidentally by moving one `return` statement.

### 2. Introduce one bounded child-process runner

Create one reusable process-lifecycle abstraction for capture-related helpers. It must replace direct blocking `Command::output()` usage on the capture path.

It must support:

```text
operation name
absolute helper path and bounded arguments
hard execution deadline
external cancellation token
bounded stdout capture
bounded stderr capture
exit status and signal classification
kill on timeout or cancellation
unconditional wait/reap after kill
duration measurement
safe diagnostic result
```

Use explicit result categories at least equivalent to:

```text
success
structured_helper_error
invalid_response
non_zero_exit
signal_or_abnormal_termination
timeout
cancelled
launch_failure
output_limit_exceeded
```

Apply it to every external operation reachable from normal capture:

- ScreenCaptureKit screenshot helper or sidecar request.
- Accessibility snapshot.
- Window snapshot.
- Vision OCR.
- Tesseract fallback.
- AppleScript fallback.
- `/usr/sbin/screencapture` fallback, if retained.

Do not solve timeouts by spawning detached threads and abandoning them after `recv_timeout`. A timeout must cancel the underlying operation and reclaim the thread and child process.

Choose and document an operation-specific deadline for each helper. Stop cancellation must not wait for the ordinary deadline; it must interrupt the current operation promptly.

### 3. Make capture cancellation and Stop bounded

Replace the current “set a flag, then wait forever” behavior with an explicit cancellation protocol.

Required behavior:

1. Stop marks the runtime as stopping exactly once.
2. The active helper receives cancellation immediately.
3. A child process is terminated and reaped.
4. The worker reaches cleanup even if cancellation occurs during Accessibility, screenshot, OCR, persistence, or provider fallback.
5. A normal Stop returns within a documented bound. Use an initial gate of two seconds on the development Mac unless the implementation proves and documents a different strict bound.
6. A cleanly cancelled session is finalized as stopped, not incorrectly left running or marked interrupted.
7. Repeated Stop calls are idempotent.
8. Start cannot create two workers.
9. Start after Stop creates one clean new session.
10. App shutdown uses the same bounded cancellation path.

Do not hold a runtime mutex while waiting for a worker or child process. Do not block the main UI event loop while joining.

### 4. Add a real internal capture state machine

Represent runtime transitions explicitly. The internal model should cover at least:

```text
stopped
starting
running
degraded
stopping
failed
```

The public API may preserve its current fields, but every internal transition must have one owner and one cleanup path.

Required guarantees:

- Exactly one capture worker owns a session.
- Worker completion always clears the worker handle, stop token, and active state.
- An ordinary provider error moves capture to degraded operation when a truthful fallback exists.
- A provider error does not automatically poison unrelated providers.
- A Rust panic is contained at the worker boundary, recorded safely, and followed by cleanup.
- A poisoned lock or failed worker does not permanently prevent a later clean Start.
- A native helper failure cannot panic the parent through an `expect` or impossible-state assumption.
- Session finalization is attempted exactly once.
- The native island and React status never show “running” after the worker has exited.

Use a guard or equivalent structured cleanup so success, error, panic, cancellation, and shutdown cannot bypass state repair.

### 5. Preserve operation-specific provider health and fallback truth

Keep independent health state for at least:

```text
full-display screenshot
active-window screenshot
Accessibility snapshot
window snapshot
OCR
```

A failing active-window provider must not disable healthy full-display capture. A failing OCR helper must not discard an otherwise valid frame. A failing Accessibility snapshot may use a bounded fallback, but that fallback must also have a deadline.

Circuit breakers must have:

- a documented opening threshold;
- a bounded cooldown or new-session recovery boundary;
- at most one recovery probe at a time;
- no retry on every event while unhealthy;
- safe diagnostics;
- deterministic tests.

Every stored frame must retain truthful provider and fallback provenance. Do not label a CLI or cropped display image as a direct ScreenCaptureKit active-window image.

### 6. Add privacy-safe runtime diagnostics

Expose enough data to distinguish a crash, helper hang, timeout, cancellation, fallback, and worker panic without collecting private content.

Include at least:

```text
capture runtime state
worker generation or session id
current operation class
operation start time and duration
provider by operation
helper launches
successes
timeouts
cancellations
abnormal exits
output-limit failures
circuit-breaker opens and probes
fallback counts
last safe error category
stop latency
worker panic count
child process count when measurable
```

Never put screenshot pixels, OCR text, Accessibility text, window titles, URLs, file paths from captured content, typed text, or clipboard contents into these diagnostics.

### 7. Add deterministic failure injection

Add development-only fault injection that cannot be enabled in a production build. Cover:

```text
helper sleep beyond deadline
helper abnormal exit or SIGABRT equivalent
invalid JSON
oversized stdout
oversized stderr
Accessibility helper hang
window helper hang
OCR hang and failure
screenshot provider failure
fallback failure
cancellation during every major capture stage
worker panic
Stop during helper execution
app shutdown with capture active
```

Fault injection must prove recovery without crashing or restarting the main process.

### 8. Add regression tests for current behavior

Do not limit tests to failure classification. Add regression coverage proving that hardening did not alter the product contract.

At minimum test:

- initial capture still occurs;
- event-trigger settlement and capture intervals remain correct;
- the 120-second idle boundary remains correct;
- full-display and active-window evidence preserve attribution and dimensions;
- provider metadata names the actual provider;
- Smalltalk self-capture remains suppressed;
- privacy exclusions still skip or redact as before;
- OCR failure does not invalidate a usable non-OCR frame;
- frame, trigger, event, and session identifiers remain connected;
- Continue can run while capture remains active;
- Stop is not a prerequisite for Continue;
- main-card and island capture state agree;
- clean Stop and restart preserve session truth.

## Forbidden shortcuts

Do not:

- keep normal capture in the main process and only wrap it in Rust error handling;
- claim `Result` or Swift `do/catch` can recover from `SIGABRT`;
- use a detached timeout thread that leaves the helper alive;
- disable active-window evidence, OCR, or Accessibility globally;
- remove event-triggered capture;
- increase capture intervals merely to make crashes less frequent;
- swallow errors while reporting a healthy provider;
- mark every provider unhealthy because one operation failed;
- introduce a global lock that freezes status and Continue;
- change user-facing Continue semantics;
- delete live captures, databases, crash reports, or audits;
- test destructive behavior against the live database;
- replace the user's dirty worktree with a clean checkout;
- call the task complete after `cargo check` alone.

## Required verification

### Automated verification

Run at minimum:

```bash
cd src-tauri && cargo fmt --check
cd src-tauri && cargo check
cd src-tauri && cargo test
swiftc -typecheck src-tauri/scripts/sck_screenshot.swift
swiftc -typecheck src-tauri/scripts/accessibility_snapshot.swift
swiftc -typecheck src-tauri/scripts/window_snapshot.swift
swiftc -typecheck src-tauri/scripts/vision_ocr.swift
npm run build
git diff --check
git status --short
```

Run every new fault-injection and lifecycle test individually and report its exact test name.

### Process and lifecycle verification

Using synthetic helpers or a safe development harness, prove:

- timeout kills and reaps the child;
- cancellation kills and reaps the child;
- no helper remains after Stop;
- no worker remains after Stop;
- Stop respects the bound;
- a failed capture can be followed by a successful capture;
- a failed session can be followed by a clean new session;
- the main process survives every injected provider failure;
- runtime status and stored session state agree after every transition.

### Manual macOS matrix

After automated verification, run the normal Tauri app through `npm run tauri dev` and perform a 30-minute capture matrix covering:

- quiet operation through at least the two-minute and four-minute idle captures;
- continuous typing and scrolling;
- rapid app switching;
- Smalltalk foreground and background;
- minimizing and restoring windows;
- closing a window during settlement;
- Mission Control and Spaces changes;
- moving a window between displays when available;
- screen-lock, sleep, or permission transition when safely testable;
- Stop during an active helper;
- clean restart and a new capture session.

Record before and after counts for both helper and main-process crash reports. The test fails if any new report or main-process restart appears.

If the user says they will perform manual testing themselves, stop before that part. Report every unexecuted manual gate explicitly and do not mark this prompt fully passed.

## Acceptance criteria

This session is complete only when all applicable criteria are proven:

- No normal production capture call can abort or indefinitely block inside the main Tauri process.
- The unsafe in-process Core Graphics capture path is removed from normal production execution.
- The known-crashing desktop-independent active-window filter is not restored.
- Every external helper has a hard deadline, output bounds, cancellation, kill, and reap behavior.
- Stop and app shutdown are bounded and cannot wait forever for capture.
- Success, error, timeout, cancellation, panic, and shutdown all converge to truthful runtime and session state.
- Provider health and fallback are operation-specific.
- Frame metadata names the actual provider.
- Injected helper failures do not restart or crash the main process.
- No child process, thread, worker, or session is leaked after the lifecycle tests.
- Existing privacy, capture cadence, evidence, Continue, main-card, and island behavior remains intact.
- All automated checks pass.
- The 30-minute manual matrix passes, unless the user explicitly reserves that work; in that case the session remains manually unverified.

## Final response format

Report in this order:

1. The exact production capture call graph before and after.
2. The isolation architecture and permission identity.
3. Every helper deadline and cancellation rule.
4. Stop, panic, shutdown, and restart state transitions.
5. Provider health, circuit breaker, and fallback behavior.
6. Product-behavior invariants and how each was verified.
7. Fault-injection test names and results.
8. Full automated command results.
9. Manual matrix duration, scenarios, and crash-report deltas.
10. Any unverified gate or remaining macOS limitation.

Do not start Prompt 02 until this prompt's automated gates pass and every deferred manual gate is clearly identified.
