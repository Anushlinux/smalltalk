# Runtime Stability 00 — Always-On Product Stability Program

## Purpose

Smalltalk is intended to behave like an always-available local layer. The current implementation does not meet that standard. Recent live testing showed repeated native screenshot-helper crashes, growing event and derived-data tables, large cumulative `continue_outputs` exports, and recurring background work while the user appears idle.

This program fixes those failures in five ordered Codex sessions. Each numbered file is a standalone implementation prompt. Use a new Codex session for each file and complete the verification in that file before starting the next one.

## Verified starting evidence

The July 12 diagnosis found:

- 26 macOS crash reports for the `sck_screenshot` child helper.
- The repeated abort occurs in `SCContentFilter(desktopIndependentWindow:)` from `src-tauri/scripts/sck_screenshot.swift`.
- The most recent 15.3-minute capture session stored 19 frames and 1,128 UI events.
- The live SQLite database contained 32,732 `ui_events`, including 20,192 low-value rows beyond the declared 5,000-row budget.
- The live database was about 114 MB and passed `PRAGMA quick_check`.
- Seven manual Continue audits occupied about 227 MB; individual audit folders occupied roughly 26–43 MB.
- Standard manual audits exported cumulative Continue tables, including multi-megabyte `continue_decisions` and `continue_ordered_evidence_spans` files.
- Capture status polling, stale-evidence detection, background Continue, capture, OCR, and audit export can all run during the same period.
- Several recent capture sessions were stored as `interrupted`, so shutdown was not clean even though the available macOS crash reports named the child helper rather than the main Tauri binary.

Treat these numbers as a baseline to improve, not as permanent truth. Re-measure the current checkout and live database at the start of every session.

## Execution order

1. `runtime-01-screen-capture-crash-containment.md`
2. `runtime-02-bounded-event-and-storage-runtime.md`
3. `runtime-03-bounded-continue-audit-exports.md`
4. `runtime-04-background-workload-governor.md`
5. `runtime-05-soak-test-and-release-gate.md`

Do not start with generic performance tuning. The native abort must be contained first so later measurements are trustworthy.

## Program-wide rules

- Work in the native Tauri/macOS lane. Do not edit or revive `browser-extension/`.
- Inspect `git status`, current source, the live SQLite schema, and current diagnostics before editing.
- Preserve unrelated worktree changes.
- Do not delete the user's live database, screenshots, or `continue_outputs` while implementing. Use copied or synthetic fixtures for destructive tests.
- Do not commit personal captures, live databases, crash reports, audit bundles, API keys, or raw local text.
- Preserve Smalltalk's privacy rule: no raw typed characters or full clipboard text.
- Preserve evidence truth. Performance work must not silently remove the evidence required to explain a Continue answer.
- Add deterministic tests for every policy and failure path.
- Do not call a session complete based only on compilation.
- If a proposed optimization changes product semantics, document and test that change explicitly.

## Program terminal condition

This program is complete only when the final soak gate proves all of the following:

- no new `sck_screenshot` or main-process crash report;
- no unclean app termination during the required soak;
- event, frame, derived-row, audit-output, memory, and disk growth stay within explicit budgets;
- unchanged evidence does not repeatedly rebuild or persist Continue decisions;
- manual Continue remains responsive while capture is active;
- standard audit output still contains enough decision-scoped evidence to explain the answer;
- the main card and native island remain behaviorally consistent;
- all automated checks pass;
- manual macOS QA passes;
- `PRODUCT.md` and relevant technical documentation describe the implemented runtime limits honestly.

