# Runtime Stability 01 — Stop Repeated ScreenCaptureKit Helper Crashes

## Codex task

Eliminate the repeated `sck_screenshot` crash loop without disabling useful capture. Make active-window capture fail safely, add a session-level circuit breaker, and prove that normal capture continues without repeatedly launching a known-crashing path.

This session owns native screenshot stability only. Do not redesign event retention, Continue audit exports, or background scheduling here.

## Read before editing

```text
AGENTS.md
PRODUCT.md
docs/phases/runtime-stability/runtime-00-always-on-stability-program.md
docs/data-capture-and-processing.md
docs/smalltalk-capture-technical-details.md
src-tauri/scripts/sck_screenshot.swift
src-tauri/src/capture.rs
src-tauri/src/capture_core/
src-tauri/src/session_island.rs
src-tauri/src/session_island/
```

Inspect the latest local `sck_screenshot*.ips` reports under `~/Library/Logs/DiagnosticReports/`, but do not modify, move, or commit them.

## Verified failure

The child helper aborts with `SIGABRT` inside Apple's ScreenCaptureKit stack while constructing:

```swift
SCContentFilter(desktopIndependentWindow: selectedWindow)
```

The Rust parent synchronously waits for the helper. When the helper aborts, the active-window path falls back to `/usr/sbin/screencapture`. Repeated captures can therefore cause repeated process crashes followed by repeated fallback work.

Swift `do/catch` cannot recover from `SIGABRT`. The parent process must treat abnormal helper termination as a health failure and stop retrying the same unsafe path repeatedly.

## Required implementation

### 1. Make helper execution observable and bounded

Introduce a named helper execution result that distinguishes at least:

```text
success
structured helper error
invalid response
non-zero exit
signal or abnormal termination
timeout
launch failure
```

Record only safe operational metadata: capture mode, provider, duration, exit category, fallback used, and circuit-breaker state. Do not record screenshot contents or private window text in diagnostic counters.

Add a hard timeout. A stuck helper must be terminated and reaped; it must not leave a zombie process or block capture indefinitely.

### 2. Add a capture-provider circuit breaker

After an abnormal ScreenCaptureKit active-window termination:

1. Mark active-window ScreenCaptureKit unhealthy for the current capture runtime.
2. Skip directly to the safe fallback for later active-window captures.
3. Do not launch the crashing helper again on every 45-second capture.
4. Allow a bounded recovery probe only after a documented cooldown or a new capture session.
5. Keep full-display capture available if it remains healthy.

Keep provider health separate by operation. An active-window failure must not automatically disable a healthy full-display path.

### 3. Remove or avoid the unsafe active-window construction path

Investigate a safer implementation using current ScreenCaptureKit APIs and current macOS behavior. Valid solutions may include:

- capturing a display and cropping to a validated window rectangle;
- using a safer application/display filter when exact window capture is unsafe;
- falling back directly for active-window mode on affected systems;
- rejecting invalid, empty, off-screen, stale, minimized, or cross-display window geometry before filter construction.

Do not assume that checking the Swift object is sufficient. The existing object comes from `SCShareableContent` and still reaches an internal assertion.

Prefer the simplest path that can be proven stable. Preserve correct scale, bounds, privacy exclusions, and app/window attribution.

### 4. Make fallback explicit

The stored frame metadata must say which provider actually produced each image. Do not label a fallback capture as ScreenCaptureKit. Expose safe counters in runtime diagnostics for:

```text
sck_display_successes
sck_active_window_successes
sck_active_window_abnormal_exits
sck_timeouts
sck_circuit_breaker_opens
screencapture_fallbacks
```

### 5. Add deterministic tests

Tests must simulate:

- helper success with valid JSON;
- helper non-zero exit with structured JSON;
- helper `SIGABRT` or equivalent abnormal termination;
- invalid or empty stdout;
- timeout;
- circuit breaker opening after an abnormal exit;
- later captures skipping the helper;
- independent display and active-window provider health;
- fallback metadata correctness;
- recovery only after the configured boundary.

Do not require a real macOS crash to run the automated suite.

## Manual macOS verification

Run a clean 30-minute capture test covering:

- idle desktop;
- normal typing and scrolling;
- rapid application switching;
- moving a window between displays if available;
- minimizing and restoring a window;
- Mission Control or Spaces changes;
- closing a window during capture settlement;
- Smalltalk in the foreground and background.

Record the count of relevant crash reports before and after. The test fails if a new `sck_screenshot` crash report appears.

## Acceptance criteria

- A ScreenCaptureKit child abort cannot create a repeated retry loop.
- Helper execution has a timeout and always reaps the child.
- Active-window failure does not disable healthy full-display capture.
- Fallback frames carry truthful provider metadata.
- No new `sck_screenshot` crash report appears during the 30-minute manual matrix.
- Capture remains usable after the injected abnormal-exit test.
- Existing privacy and self-exclusion behavior remains intact.
- Automated tests, Rust checks, frontend build, and manual QA pass.

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

Also run the new focused helper-runner and circuit-breaker tests individually and report their exact names.

## Final response format

Report:

1. Exact native crash mechanism.
2. Provider strategy chosen and why.
3. Circuit-breaker behavior.
4. Timeout and process-reaping behavior.
5. Metadata and diagnostics added.
6. Automated commands and results.
7. Manual 30-minute matrix and before/after crash-report counts.
8. Remaining macOS-specific limitations.

