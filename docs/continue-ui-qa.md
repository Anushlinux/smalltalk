# Continue Recovery QA

Last updated: 2026-07-04

Smalltalk is continuation-first. The first screen must show one Continue answer or a clean no-evidence state. Workstreams, frames, timelines, raw ids, evals, and storage details belong behind Developer diagnostics.

## Required Manual Run

Each recovery build must be launched through the Tauri GUI path, not only `cargo check` or frontend build. Record actual results before judging product quality.

| Scenario | Expected result | Actual result | Target correct | Storage delta | Frame delta | Raw internal text leaked | GUI launched |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Type a long prompt in another app for 5 minutes | Local memory records lightweight events; screenshots stay under the 10-minute budget and do not capture every few seconds | Pending manual run | Pending | Pending | Pending | Pending | Pending |
| Interact with Smalltalk UI for 3 minutes | Smalltalk self-observation is low-value evidence and does not become the main workstream unless explicitly selected | Pending manual run | Pending | Pending | Pending | Pending | Pending |
| Work in a target artifact, branch to browser/search, idle | Continue returns to the target artifact, not the branch | Pending manual run | Pending | Pending | Pending | Pending | Pending |
| Work, switch to messaging, idle | Continue returns to prior work, not messaging | Pending manual run | Pending | Pending | Pending | Pending | Pending |
| View an error, search, idle | Continue preserves the blocker and does not show repeated raw error rows | Pending manual run | Pending | Pending | Pending | Pending | Pending |
| Use the floating island | Island opens via `continue_decision_id`, not cloud resume or session trail | Pending manual run | Pending | Pending | Pending | Pending | Pending |
| Open diagnostics | Raw frames, ids, search, evals, storage, and timelines are available only there | Pending manual run | Pending | Pending | Pending | Pending | Pending |
| Run cleanup | Storage shrinks or remains under budget; diagnostics show last cleanup result | Pending manual run | Pending | Pending | Pending | Pending | Pending |

## Runtime Diet Checks

During the manual run, open Developer diagnostics and record:

- `Heavy stored`, `Heavy skipped`, `Event signals`, and `Cache hits`.
- Runtime diet counters: budget skips, dedupe skips, Smalltalk self skips, OCR runs, and AX snapshots.
- Continue calls: normal, rebuild, and cache hits.

Expected shape:

- Long typing/scrolling should increase `Event signals` faster than `Heavy stored`.
- Repeated Continue clicks without new evidence should increase `Cache hits`, not decision rows.
- Smalltalk UI interaction should increase Smalltalk self skips or low-value skips, not promote Smalltalk as the default return target.
- `Rebuild Continue` should be the only normal UI path that increases rebuild calls.

## Build-Time Checks

- `cargo check` from `src-tauri/`.
- `tsc --noEmit` from the repo root.
- `vite build` from the repo root.
- Tauri GUI launch or smoke test with screenshots inspected.

## UI Leak Checks

The default screen must not show raw JSON, frame ids, action ids, artifact ids, episode ids, scorer labels, `error_signal`, `typing_in_composer`, `frame_fallback`, `primary_artifact_fallback`, or `secondary_artifact_for_searching`.

Allowed default sections:

- Header/status line with product name, local memory status, latest evidence age, Continue button, and Memory menu.
- One primary Continue answer.
- Primary `Continue here` action plus one secondary `Inspect evidence` action.
- One small `Wrong target?` affordance that reveals correction controls only after interaction.
- Collapsed alternatives line when other continuation candidates exist; alternative choices appear only after `Show alternatives`.
- Collapsed Developer diagnostics entry.

## Diagnostics Checks

Developer diagnostics may show raw ids, frame timeline, search, screenshot inspector, Continue eval metrics, workstream detail, evidence anchors, database size, snapshot directory size, frame count, event count, oldest retained frame, cleanup last run, cleanup result, and budget constants.

## Hard QA Checklist

### UI Sanity

- App opens to one Continue answer or a clean no-evidence state.
- Primary UI shows no raw JSON, raw ids, internal scorer strings, repeated low-level rows, or unproductized warnings.
- Developer diagnostics are closed by default.
- Correction controls are secondary and require interaction before expanding.
- Alternatives are collapsed by default.
- Common window sizes show no layout overlap or clipped primary actions.

### Runtime Sanity

- Typing for two minutes increases event signals faster than heavy stored captures.
- Scrolling for two minutes does not create a heavy frame explosion.
- Smalltalk UI interaction does not dominate workstreams.
- Normal Continue returns quickly and reuses cache when evidence has not changed.
- Explicit Rebuild Continue is slower and remains diagnostics-only.
- Repeated Continue clicks do not create duplicate decision rows.

### Island Sanity

- Island uses Continue/local-memory language, not session/trail as primary product language.
- Island primary action calls Continue.
- Island opens by `continue_decision_id`.
- Island does not require Stop Session, cloud resume, or native resume card generation.

### Storage Sanity

- Storage report is visible only in diagnostics.
- Preview cleanup reports candidate frames, protected frames, orphan snapshots, and estimated reclaimable bytes without deleting.
- Apply cleanup preserves decision-linked, workstream-linked, manual, and evidence-anchor frames.
- Cleanup deletes unreferenced duplicate snapshots and old self-capture/low-value frames before anything else.
- Snapshot directory growth remains bounded during normal typing and scrolling.
