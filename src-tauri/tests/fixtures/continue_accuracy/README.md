# Continue accuracy corpus v1

This directory is the committed, privacy-safe Continue accuracy corpus. It contains the seven mandatory P6.01 Capture-button cases under `cases/`, the frozen evaluation policy in `eval-policy.v1.json`, and the milestone contract in `known-failures.v1.json`. The validator requires those seven cases as a stable subset and permits reviewed P6.09 cases to be added; it no longer caps the corpus at seven.

All retained language is synthetic. The fixtures preserve only the phase-reviewed semantic facts needed to test the Capture-button failure. They contain no screenshots, databases, local paths, URLs, source conversation identifiers, raw typed sequences, or verbatim private capture text. Relative timestamps and fixture-local identifiers replace source timestamps and stable identifiers.

`fixture-owner` is a reserved synthetic ownership marker used only in source-record provenance; the privacy linter permits this non-personal sentinel and rejects every other unhashed owner identifier.

## Injection boundary

Every case declares the schema boundary `capture_records`. Frames, source AX roles, OCR geometry, content-unit ownership, app context, event ordering, transitions, and typing counts are inserted before the production semantic rebuild. Expected region, conversational, task-turn, action, workstream, recap, and target labels appear only in `expected_checkpoints`; they are never copied into source records.

Historical feedback, branch, workstream, open-loop, and memory rows are inserted only when the case explicitly tests persisted contaminants. They are declared as `historical_state` and inserted after the production third-layer rebuild but before `get_continue_decision`, because the rebuild legitimately clears and regenerates those tables. A deterministic model response enters only at the model transport boundary; network access is not part of replay.

The local-only importer in `accuracy_fixture.rs` reads an allowlisted set of files from a private audit and emits only structural shapes, text hashes, and character counts. Its output is a review candidate, not a committed fixture. A human-authored synthetic or explicitly approved derived-redacted fixture must be created from that candidate. The importer never copies screenshots or databases.

These seven fixtures are synthetic-only development/validation artifacts. Their `privacy_review` metadata records repository ownership of the synthetic review, not a claim of independent human adjudication; P6.09 must add that sign-off before release.

## Initial cases

| Case | Source delta | Expected task identity |
| --- | --- | --- |
| `capture_button_fresh_task_only` | Latest question and agent trace only. | Active Capture-button investigation. |
| `capture_button_old_completion_visible` | Adds the older completed Continue-card task. | Active Capture-button investigation. |
| `capture_button_stale_inferred_feedback` | Adds stale inferred feedback for an unrelated research branch. | Active Capture-button investigation. |
| `capture_button_unrelated_open_loop` | Adds a strong unrelated media-player open loop. | Active Capture-button investigation. |
| `capture_button_all_contaminants` | Adds every stale contaminant. | Active Capture-button investigation. |
| `capture_button_adjacent_before_new_task` | Only the earlier completed Continue-card task is visible. | Completed prior task; no newer active task. |
| `capture_button_adjacent_after_support_detour` | Adds a later app-switch/search support surface. | Capture-button investigation remains active. |
| `tt2_session_013_control` | Legacy null post-frame Enter, matching later chat frame, prior completion, and actionable approval control. | The new right-aligned request is current; the control is ineligible and public target remains null. |

The before-to-canonical change is a legitimate task-identity change because a newer user goal and agent status appear. The canonical-to-after change is a legitimate current-surface/support delta, but not a task supersession: the later surface is a non-promotable search/support branch without a newer user goal.

Repeatability reuses `capture_button_all_contaminants` with `repeat_count` greater than one. It is not an eighth case.

## Current milestone

P6.03 now persists and resolves the typed current task turn through the production replay. All seven cases pass the `latest_task_turn` checkpoint, including execution state, current actor, waiting-on state, task-turn relation, and the two action/delta cases. The remaining exact first divergences are downstream: one `current_surface` case, one `eligible_open_loop` case, one `selected_workstream` case, and four `product_answer` cases. These are owned by later P6 phases; P6.09 rejects every remaining known-failure marker.

If the first raw replay measures a different earliest checkpoint, update the manifest to the measured result instead of changing production behavior to fit this baseline.

## Validation

The Rust contract parser rejects unknown fields and unsupported versions. The privacy linter rejects pending review, raw typed text, home paths, raw URLs or query strings, secret-like or long opaque tokens, screenshot paths, and text above the frozen per-field caps.

Use the repository tests to parse and lint the committed corpus:

```bash
cd src-tauri
cargo test accuracy_fixture
```

Run the explicit capture-to-answer evaluator and refresh its privacy-safe baseline report with:

```bash
cd src-tauri
cargo run --bin continue_accuracy_eval -- \
  --output tests/fixtures/continue_accuracy/baseline-report.v1.json \
  --repeat 3
```

The frozen local model-off baseline recorded on 2026-07-10 is 175.96 ms p95 across the seven initial cases after one warmup run. The regression budget is 1.25x that measured baseline, subject to any stricter existing absolute budget.

The existing `run_continue_eval` candidate-level metrics remain separate and retain their v1 semantics. Accuracy replay must report production-path coverage per checkpoint and may not count fixture-injected substitutes as full-pipeline coverage.
