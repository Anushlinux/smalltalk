# Task Truth v2.01 Completion Audit

Date: 2026-07-11  
Scope: causal evidence repair and wrong-answer containment

## Outcome

Task Truth v2.01 is implemented. A committed typing burst can be linked to the frame that shows its result without retaining typed text; task-turn extraction consumes a structured causal-attribution record; prior-boundary content is history-only; shared role/action policy excludes controls from user-goal eligibility; and unsupported current-task states abstain as `no_clear_current_task` through the backend, React, and native island.

Manual/background result adoption is quality-dominant in both React and the island. A background challenger cannot replace a stronger manual answer merely because it completed later. Replacement requires causally newer evidence without a downgrade in task identity/revision, task confidence, supported task/state/next/where coverage, target safety, or wording provenance.

## Contract proof

- Capture persists privacy-safe burst event bounds, commit event, surface identity, trigger identity, post-frame association source/confidence, and rejection reasons. It splits bursts across app/window changes and after commit.
- Exact stored post-frame associations are preferred. Legacy null-post rows use a named, bounded, unique same-session/same-surface recovery rule and remain ambiguous when multiple candidates, a different surface, an uncommitted burst, privacy blocking, or excessive time distance prevents attribution.
- Task-turn evidence persists the complete causal-attribution object. Novel current text, surface match, temporal distance, source, confidence, event/trigger identifiers, and rejection reasons remain inspectable without raw typed text.
- Shared user-goal eligibility marks buttons, menus, model selectors, chips, links, tabs, toolbars, status/action labels, browser chrome, and composer-adjacent action text as controls rather than user goals.
- `prior_boundary_sample` can support chronology, completion, and relation-to-prior evidence only; it cannot create or select a current task turn.
- A missing eligible current goal clears public task/workstream/candidate/state/alternatives/targets and returns typed ambiguity plus supported surface context and inspect-first guidance.
- The session-013 fixture preserves the live AX weakness without synthetic conversation-role identifiers. Its real right-aligned user request survives; `Approve for me` is an AX control and is forbidden as the current task.

## Verification

- Six focused capture causal-association and migration tests pass.
- `cargo test continuation::task_turn_evidence`: 14 passed.
- `cargo test continuation::task_turn`: 34 passed.
- `cargo test continuation::accuracy_eval`: 6 passed.
- `cargo test session_island`: 33 passed.
- Full `cargo test`: 559 passed.
- `cargo check`: passed.
- `cargo fmt --check`: passed.
- `npm run test:webview`: 13 passed.
- `npm run build`: passed.
- `git diff --check`: passed.

The capture association test reopens its file-backed SQLite fixture with `SQLITE_OPEN_READ_ONLY` and proves that the committed burst is linked to the expected post frame with authoritative association provenance. The audit selects only counts and identifiers/provenance; it does not print or persist typed content.

## Remaining Task Truth v2 work

Task Truth v2.02 remains. It must implement and validate the broader multimodal resolver and expand live/native coverage beyond this containment layer. Task Truth v2 as a whole is not complete, and this result does not change the closed P6 release gate or its outstanding corpus, holdout, calibration, performance, and manual macOS QA requirements.
