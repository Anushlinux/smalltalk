# Always-On Repair 01 — Session Notes

## Behavior invariants

- Continue remains usable while local capture is running; Stop is not a prerequisite.
- Capture remains discrete and event-driven. Initial, event settlement, manual, surface-change, low-value, important, and 120-second idle behavior stays unchanged.
- Full-display and active-window images keep truthful provider, geometry, scale, window, application, session, event, trigger, and frame attribution.
- Smalltalk self-capture, privacy exclusions, redaction, and the prohibition on raw typed text and full clipboard text remain unchanged.
- Accessibility and OCR remain bounded evidence sources. Their failure may reduce enrichment but must not invalidate an otherwise usable image frame.
- Current focus, current activity, current task turn, return target, and resume work target remain separate.
- React and the native island read the same capture status. Neither may report running after the worker exits.
- No failure path may silently disable all capture for the process lifetime. Provider health remains operation-specific and recovers after a bounded cooldown or a new session.

This checklist is reused during final verification. Manual macOS soak results remain unverified until they are actually run.

## Automated verification — 2026-07-16

- `cargo fmt --check`: passed.
- `cargo check`: passed.
- `cargo check --release`: passed without warnings. This also proves the development-only fault hooks compile to no-ops in a release build.
- `cargo test`: 832 passed, 0 failed, 3 explicitly opt-in tests ignored.
- Every new process-runner, provider-health, fault-injection, lifecycle, persistence-cancellation, and behavior-invariant test was also run by its exact name and passed.
- All four capture Swift helpers passed `swiftc -typecheck`.
- `npm run build`: passed.
- `git diff --check`: passed.
- Static source audit found no remaining `capture_core_graphics_in_process`, `write_cg_image`, `desktopIndependentWindow`, or temporary development-autostart route.

## Manual macOS gate

The user explicitly reserved app-level testing. No Computer Use test is claimed. The 30-minute normal-app matrix, real permission transitions, multi-display and Spaces behavior, and before/after crash-report comparison remain manually unverified.
