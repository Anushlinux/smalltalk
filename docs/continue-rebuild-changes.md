# Continue Rebuild Changes

Last updated: 2026-07-03

This document records the implementation changes made after the native Continue architecture existed but before the UI fully matched it. The purpose of these changes was to move Smalltalk from a capture/session/debug UI with a Continue card into a Continue-first workstream product.

The changes below cover the current source edits in:

- `src/App.tsx`
- `src/App.css`
- `src-tauri/src/continuation.rs`
- `src-tauri/src/capture.rs`
- `src-tauri/src/lib.rs`
- `docs/continue-ui-qa.md`

Generated `resume_query_exports/` folders were not part of this work and should remain treated as generated artifacts.

## Product Direction Implemented

The UI now treats these as primary product objects:

- Continue decision
- Workstream
- Workstream detail
- Current focus
- Return target
- Resume work target
- Artifact roles
- Episodes and task actions
- Candidate continuation targets
- Evidence anchors
- Feedback and corrections

The UI now treats these as diagnostics:

- raw frame timeline
- captured evidence search
- screenshot/frame inspector
- raw events
- app/window/path details
- session id
- cloud/native/legacy resume paths
- Continue eval tooling

## `src/App.tsx`

### Added frontend types for new Continue backend responses

Added:

- `ContinueScoreComponents`
- `ContinueFeedbackEventResult`
- `ContinueEvalReport`
- `ContinueWorkstreamArtifactDetail`
- `ContinueWorkstreamActionDetail`
- `ContinueWorkstreamEpisodeDetail`
- `ContinueWorkstreamCandidateDetail`
- `ContinueDecisionSummary`
- `ContinueBreadcrumbSummary`
- `ContinueWorkstreamDetailResult`

Why:

The frontend needed typed access to the real Continue objects returned by the backend: artifact roles, episode/action rows, candidate targets, feedback events, breadcrumbs, decision summaries, and eval metrics. Without these types, React would either stay limited to the old compact Continue card or fall back to frame/search-driven UI.

### Added Continue freshness state

Added state for:

- `continueDecisionFrameCount`
- `continueDecisionUpdatedAt`
- `continueRefreshTimerRef`

Why:

The UI needed to show whether the current Continue decision was made against the latest local evidence. This prevents a stale decision from looking authoritative after new frames arrive.

### Added workstream selection and detail state

Added state for:

- `selectedWorkstreamId`
- `workstreamDetail`
- `workstreamDetailError`

Why:

Workstreams needed to become navigable objects, not just a recent list beside the Continue card. Selecting a row now drives a detail surface.

### Added feedback and eval state

Added state for:

- `feedbackStatus`
- `evalReport`
- `evalError`

Why:

The product needed a visible correction loop and developer-only Continue eval metrics. Feedback status gives immediate confirmation after a correction, while eval results stay inside diagnostics.

### Added `loadWorkstreamDetail`

Added a React callback that invokes:

```ts
get_continue_workstream_detail
```

with:

- selected workstream id
- latest Continue decision id when available

Why:

The workstream detail UI needed a bounded backend query over one workstream. This avoids broad frame searches and keeps the frontend aligned with the Continue semantic layer.

### Updated `runContinueDecision`

Changed `runContinueDecision` to:

- set the selected workstream from the returned Continue decision
- store the frame count used for the decision
- store the decision update timestamp

Why:

The decision should become the source of truth for primary workstream selection and freshness. Current focus and return target remain separate, but the selected workstream must drive the detail view.

### Added automatic workstream detail loading

Added effects to:

- load detail when `selectedWorkstreamId` changes
- auto-select the first workstream when workstreams exist and no workstream is selected

Why:

The workstream detail surface should not sit empty when the backend already has workstreams. This makes the workstream model feel like the app's navigation model.

### Added debounced Continue refresh after new evidence

Added a debounced refresh path when:

- local memory is running
- a Continue decision exists
- the live frame count exceeds the decision frame count

Why:

Continue should keep up with new evidence without spamming the backend on every capture event.

### Moved local memory controls into a secondary menu

Changed the top bar so:

- `Continue` is the only primary action
- start/pause/capture/delete controls live under `Local memory`

Why:

The first screen should not read as a session/capture tool. Local capture remains necessary infrastructure, but it should not compete with Continue as the main workflow.

### Made Continue available in the no-evidence state

Changed the no-evidence empty state to show:

- primary `Continue`
- secondary `Start local memory`
- secondary `Show evidence`

Why:

Even when evidence is thin, the product should still read as Continue-first. The UI can say evidence is missing instead of hiding the Continue model.

### Added workstream-local breadcrumb behavior

Changed breadcrumb saving to attach to:

- selected workstream when one is selected
- otherwise the selected workstream from the latest Continue decision

Also records a `user_next_step_note` feedback event after saving a breadcrumb.

Why:

Breadcrumbs should be attached to continuation/workstream context, not to a global session note. Recording the feedback event makes notes part of the correction/history loop.

### Added explicit feedback actions

Added `recordContinueFeedback`, which invokes:

```ts
record_continue_feedback
```

Supported UI feedback kinds:

- `accepted`
- `rejected`
- `corrected`
- `artifact_only_evidence`
- `ignored_workstream`
- `user_next_step_note`

Why:

Continue needs to be correctable. A wrong target should become data for future scoring instead of being a dead end in the UI.

### Added alternative continuation flow

Added `continueFromAlternative`, which:

- records `corrected` feedback
- selects the alternative's workstream
- opens the alternative evidence frame through `open_resume_point` only when the backend supplied an `evidence_frame_id`

Why:

The UI must not fabricate URLs, paths, or targets. If the backend gives a frame anchor, Smalltalk can open/inspect that route. If not, it records the correction and keeps it inspectable.

### Added Continue eval diagnostics

Added `runContinueEval`, which invokes:

```ts
run_continue_eval
```

The diagnostics panel now shows:

- case count
- target correctness
- Recall@k
- MRR
- current-focus false-positive rate
- hallucinated artifact count
- model validation fallback rate

Why:

Eval metrics are useful for development, but they should not dominate the product surface. They now live only in Developer diagnostics.

### Added correction controls to the Continue decision card

Added buttons for:

- Useful target
- Wrong target
- Continue from this instead on alternatives

Why:

The primary Continue answer must be directly correctable. Alternatives are especially important because Continue will sometimes choose the wrong target.

### Added `WorkstreamDetailPanel`

Added a real detail surface with:

- selected workstream summary
- confidence
- last active timestamp
- primary artifact
- unresolved signal
- Continue target
- last meaningful state
- current focus relationship
- correction controls
- artifact roles
- candidate targets
- episodes and actions
- evidence anchors
- feedback events
- next-step breadcrumbs

Why:

This is the main product-architecture migration. The user can now understand a workstream as an object: what it is, what is unresolved, what target Continue would choose, which artifacts are primary/support/evidence, what actions happened, and what evidence supports the result.

### Changed workstream rows from "rerun Continue" to "select workstream"

Changed `WorkstreamList` so clicking a workstream:

- selects that workstream locally
- highlights the selected row
- does not rerun Continue

Why:

Workstreams are now navigation objects. Rerunning Continue on every row click made the rail feel like a trigger list instead of a navigable model.

### Added helper presentation functions

Added:

- `groupArtifactsByRole`
- `detailArtifactLabel`
- `MetricBlock`

Why:

The detail surface needs grouped artifact roles, readable artifact labels, and reusable compact metrics without duplicating formatting logic.

## `src/App.css`

### Added Continue decision freshness styles

Added styles for:

- `.continue-card-badges`
- `.continue-live-row`
- `.continue-state-grid`
- `.current-focus-target`

Why:

The UI needed to make freshness and current-focus-vs-return-target separation visible without adding another large card.

### Added workstream detail layout styles

Added styles for:

- `.workstream-detail`
- `.workstream-detail-head`
- `.workstream-detail-actions`
- `.workstream-summary-grid`
- `.workstream-target-grid`
- `.detail-section-grid`
- `.detail-section`
- `.detail-block`
- `.metric-block`

Why:

The new detail surface needed stable, dense dashboard layout that does not overlap inside the fixed app shell.

### Added artifact role, candidate, episode, and action styles

Added styles for:

- `.artifact-role-group`
- `.artifact-role-row`
- `.candidate-row`
- `.episode-stack`
- `.episode-card`
- `.episode-head`
- `.action-list`
- `.action-row`

Why:

Artifacts, candidates, episodes, and actions are now first-class user-facing Continue objects. They needed scannable, bounded rows instead of raw frame/timeline presentation.

### Added feedback and text-button styles

Added styles for:

- `.feedback-bar`
- `.feedback-list`
- `.feedback-row`
- `.text-button`

Why:

Correction controls should feel calm and inline, not modal-heavy. Text buttons support secondary inspect/select actions without making every item look like a primary CTA.

### Added Continue eval panel styles

Added styles for:

- `.continue-eval-panel`
- `.eval-grid`

Why:

Eval metrics are now visible in diagnostics and need to fit the existing diagnostic surface.

### Added responsive rules for new detail grids

Updated responsive behavior so these collapse on narrower windows:

- workstream summary grid
- workstream target grid
- detail section grid
- eval grid
- candidate rows
- feedback list

Why:

The app previously had overlap problems. New dense panels must collapse cleanly instead of expanding beyond the fixed shell.

## `src-tauri/src/continuation.rs`

### Extended feedback result shape

Added fields to `ContinueFeedbackEventResult`:

- `selected_candidate_id`
- `workstream_id`
- `note`
- `source`

Why:

Inferred feedback only needed decision/target/chosen artifacts. Explicit user corrections need to preserve candidate, workstream, note, and source information.

### Added explicit feedback request type

Added:

- `ContinueExplicitFeedbackRequest`

Fields:

- decision id
- selected candidate id
- workstream id
- target artifact id
- corrected artifact id
- feedback kind
- optional note
- source

Why:

The UI needs structured correction controls. Storing corrections as plain breadcrumbs would lose which candidate/workstream/artifact was corrected.

### Added workstream detail request and response types

Added:

- `ContinueWorkstreamDetailRequest`
- `ContinueWorkstreamArtifactDetail`
- `ContinueWorkstreamActionDetail`
- `ContinueWorkstreamEpisodeDetail`
- `ContinueWorkstreamCandidateDetail`
- `ContinueDecisionSummary`
- `ContinueBreadcrumbSummary`
- `ContinueWorkstreamDetailResult`

Why:

The frontend needed one bounded command that returns the semantic workstream detail needed by the product: artifact roles, episodes/actions, candidates, decisions, feedback, breadcrumbs, and anchors.

### Added `get_continue_workstream_detail`

Added a public function that:

- validates workstream id
- loads the selected workstream
- loads artifact role details
- loads episode/action details
- loads candidate targets
- loads the latest matching decision
- loads feedback events
- loads breadcrumbs
- builds evidence anchors

Why:

This exposes existing stored Continue data without rewriting the engine or asking the frontend to query raw frames.

### Added `record_continue_feedback`

Added a public function that stores explicit feedback kinds:

- `accepted`
- `rejected`
- `ignored`
- `corrected`
- `artifact_only_evidence`
- `ignored_workstream`
- `user_next_step_note`

Why:

Users need to correct Continue when it is wrong. This creates local structured feedback for future scoring/evaluation work.

### Added workstream detail loader helpers

Added helpers:

- `load_recent_continue_workstream`
- `load_continue_workstream_artifact_details`
- `load_continue_workstream_episode_details`
- `load_continue_episode_action_details`
- `load_continue_workstream_candidate_details`
- `load_continue_decision_summary`
- `load_continue_feedback_events`
- `load_continue_breadcrumbs`
- `workstream_detail_anchors`

Why:

Each helper keeps SQL bounded to a selected workstream, decision, or feedback scope. This keeps the React UI from recreating backend joins or relying on raw session/frame searches.

### Extended inferred feedback inserts

Changed `insert_feedback_event` so inferred feedback still writes:

- null selected candidate id
- null workstream id
- null note
- source `inferred`

Why:

The explicit-feedback schema additions should not break existing inferred feedback behavior.

### Added feedback schema migrations

Added schema upgrade columns on `continue_feedback_events`:

- `selected_candidate_id`
- `workstream_id`
- `note`
- `source`

Why:

Existing databases need to upgrade in place. The correction model needs these columns to persist structured local feedback.

## `src-tauri/src/capture.rs`

### Added Tauri command wrapper for workstream detail

Added:

```rust
get_continue_workstream_detail
```

Why:

React can only access backend functions through Tauri commands. This command exposes the new bounded detail loader.

### Added Tauri command wrapper for explicit feedback

Added:

```rust
record_continue_feedback
```

Why:

React correction controls need a backend command to persist structured feedback.

### Made thin stop-time resume bundle errors non-fatal

Added:

- `is_thin_resume_trail_error`
- test `thin_resume_trail_errors_are_non_fatal_on_stop`

Changed `stop_capture` so these errors do not fail stopping local memory:

- no non-internal frames to summarize
- no model-safe frames after privacy filtering

Why:

Pause/stop local memory should not feel like a product failure just because the legacy stop-time resume bundle path has thin evidence. Stop-time bundle generation is infrastructure, not the core Continue product.

### Avoided cloud-result warning when opening a Continue decision or explicit frame

Changed `open_resume_point_impl` so the warning about missing cloud resume results appears only when no Continue decision id and no target frame id were supplied.

Why:

Opening a Continue target should not show cloud-resume warnings. Cloud resume is no longer the primary path.

### Reduced misleading URL mismatch warning

Changed frame-opening warning behavior so a repaired non-browser surface does not emit the old `target_url_blocked_by_identity_mismatch` warning.

Why:

When Smalltalk repairs visible app identity away from a stale browser URL, that should be treated as local evidence handling, not as a scary URL mismatch in the Continue path.

## `src-tauri/src/lib.rs`

### Registered new Tauri commands

Added to the invoke handler:

- `capture::get_continue_workstream_detail`
- `capture::record_continue_feedback`

Why:

Without registration, the frontend cannot invoke the new backend commands.

## `docs/continue-ui-qa.md`

### Added manual QA checklist

Added a checklist covering:

- current-focus trap
- branch trap
- messaging interruption
- error branch
- copy evidence
- AI as support vs AI as primary
- thin evidence
- correction flow
- artifact role correction
- next-step notes
- diagnostics separation
- Continue eval diagnostics
- privacy checks

Why:

The rebuild changes product behavior, not just visual polish. These scenarios capture the failure modes the UI is supposed to make visible and correctable.

## Verification Run

Commands run successfully:

```bash
npm run build
cd src-tauri && cargo check
cd src-tauri && cargo test
```

Notes:

- `npm run build` required the bundled Node runtime because `node` is not on the shell `PATH`.
- Rust tests passed: 99 passed, 0 failed.
- The Tauri GUI was not launched manually during this implementation pass.
