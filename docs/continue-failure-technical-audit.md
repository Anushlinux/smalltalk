# Smalltalk Continue Failure Technical Audit

Last updated: 2026-07-04

This document is a technical diagnosis of why the current Smalltalk product feels slower, more confusing, and less reliable after the Continue pivot.

The blunt version: the repo now has a real Continue/workstream backend, but the product did not remove the old recorder architecture from the primary user experience. Instead, the new Continue layer was added on top of the older session recorder, evidence timeline, cloud resume path, raw frame inspector, and floating island. That hybrid explains the current failure mode: the product is denser than before, still records too much, still exposes session-era controls, and does not yet behave like a simple continuation product.

## What The User Is Seeing

The screenshots taken around 2026-07-03 23:54-23:56 show several concrete product failures:

- The primary UI says `Smalltalk Continue`, but it still looks like a capture/debug dashboard.
- The top-level action is `Continue`, but the screen is dominated by workstream detail, candidate targets, artifact roles, evidence anchors, feedback, and diagnostics.
- The first screenshot shows overlapping translucent panels. The selected workstream detail is visually sitting over the older Continue/home layer instead of feeling like a stable page.
- The second screenshot shows many repeated rows named `Encountering error`, all with nearly identical `error_signal` and frame ids. This reads like raw scorer/debug output, not a user-facing continuation result.
- The third screenshot shows `Evidence anchors and feedback` with raw ids for frames, actions, episodes, and artifacts. That is useful for debugging, but it should not be part of the main product experience.
- The UI exposes a JSON-like unresolved state, for example a raw object starting with `{"confidence":0.86,"evidence_action_id"...}`. That is implementation detail leaking into user copy.
- Candidate rows and feedback actions compete with the primary action. The user sees `Continue from this`, `Correct target`, `Wrong target`, `Only evidence`, `Ignore workstream`, and `Developer diagnostics` all in the same product flow.

This is not just visual polish. It is an architecture/product boundary problem. The app is exposing multiple internal layers at once.

## What The Code Actually Does

The codebase currently has four overlapping product paths.

### 1. React Continue Shell

`src/App.tsx` has been refactored toward Continue, but it still renders the old diagnostic product:

- `ContinueDecisionCard`
- `WorkstreamList`
- `WorkstreamDetailPanel`
- `ContinueEvidencePanel`
- `Developer diagnostics`
- search form
- health strip
- Continue eval panel
- evidence timeline
- raw event stream
- frame inspector
- screenshot viewer
- overlay controls
- verification drawer
- evidence tabs

That means the UI did not become a simple Continue app. It became a larger app that shows both Continue objects and the older evidence substrate.

The current top-level layout also makes the problem worse. The app uses one scroll region, `.app-scroll`, and then stacks high-density sections inside it. `continue-home`, `WorkstreamDetailPanel`, optional evidence panel, and diagnostics all live in the same vertical surface. The workstream detail is not a separate route, page, modal, or constrained inspector. It is another large dashboard block added below the existing Continue card.

### 2. Continue Backend

`src-tauri/src/continuation.rs` has a substantial Continue layer:

- stable artifacts
- artifact observations
- task actions
- episodes
- workstreams
- candidates
- decisions
- feedback events
- breadcrumbs

The main command, `get_continue_decision`, does real work:

1. Ensures the Continue schema exists.
2. Infers pending feedback from prior decisions.
3. Rebuilds the semantic layer.
4. Rebuilds the workstream layer.
5. Loads current focus.
6. Loads scorer workstreams.
7. Generates and scores candidates.
8. Persists candidates.
9. Persists a decision.
10. Returns current focus, selected workstream, return target, resume work target, last action, unresolved state, next action, anchors, warnings, and alternatives.

That is the right direction architecturally, but the current invocation pattern is still prototype-like. The frontend calls `get_continue_decision` with `rebuild_layers: true`, then refreshes memory, refreshes workstreams, and loads detail. This is too much work to feel like a lightweight product interaction when the capture substrate is also actively inserting frames and events.

### 3. Capture/Recorder Backend

`src-tauri/src/capture.rs` is still the operational center of the app. It owns:

- `start_capture`
- `stop_capture`
- `capture_status`
- `capture_once`
- `search_captures`
- `get_frame`
- `get_frame_detail`
- frame image variants
- resume-query bundle generation
- cloud resume
- open resume point compatibility
- capture paths
- screenshot capture
- Accessibility capture
- OCR
- UI event capture
- triggers and transitions

The Continue layer is additive on top of this capture store. It did not replace the recorder runtime.

This matters because the capture loop still treats user interaction as something to record. While local memory is active, scrolls, AX notifications, keydown events, clicks, and app/window changes flow into the event source. Those events are coalesced into triggers, and triggers can store frames.

Each stored frame can involve:

- Accessibility snapshot collection.
- Privacy decision.
- Window graph collection.
- ScreenCaptureKit full screenshot.
- ScreenCaptureKit active-window capture when a window id is known.
- Image hashing.
- Optional OCR when Accessibility is thin.
- SQLite inserts into `frames`.
- OCR text/span persistence.
- AX node persistence.
- app context persistence.
- content unit persistence.
- sensitive region persistence.
- transition validation.
- Tauri `capture-frame` event emission.

That is a lot of work for something the product currently presents as a quiet local memory layer.

### 4. Floating Island

`src-tauri/src/session_island.rs` still speaks the old language:

- `SessionIslandState::RecordingCompact`
- `SessionIslandState::RecordingExpanded`
- `SessionIslandState::Processing`
- `SessionIslandState::StoppedToast`
- `SessionIslandState::TrailReconstructing`
- `SessionIslandState::ResumeReady`
- `SessionIslandActionKind::ReconstructTrail`
- `SessionIslandActionKind::ResumeMe`

Most importantly, the floating island is not wired to the new Continue decision path.

`ResumeMe` and `OpenResumePoint` route through `open_resume_point_from_island`, which calls `open_resume_point` with:

```rust
OpenResumePointInput {
    output_path,
    session_id: None,
    continue_decision_id: None,
    current_frame_id: None,
    target_frame_id: None,
}
```

That explicitly does not pass a Continue decision id.

The island's trail reconstruction path calls `run_cloud_resume`, not `get_continue_decision`. So the floating pill still lives in the cloud resume / native card / session trail architecture even though the main app is trying to become Continue-first.

This is why the floating pill feels like the same old session UI. Technically, it is still mostly the same old session UI.

## Old vs New Architecture Mismatch

The old architecture is session-recorder-first:

- Start local capture.
- Record frames/events.
- Show frame counts and captured moments.
- Stop or pause.
- Build resume-query bundles.
- Optionally ask OpenAI.
- Open a resume point from cloud/local resume artifacts.
- Use the floating island as a compact session/capture control.

The new architecture is continuation-first:

- Observe local evidence continuously.
- Resolve stable artifacts.
- Extract task actions.
- Segment episodes.
- Cluster workstreams.
- Score candidate continuation targets.
- Separate current focus from return target.
- Let the user continue or correct the target.

Both systems are currently active. They share the same capture store, but they do not share the same product contract.

The old system thinks in sessions, frames, screenshots, resume bundles, and cloud/local resume cards. The new system thinks in artifacts, actions, episodes, workstreams, candidates, decisions, and feedback. The UI exposes both vocabularies at the same time.

The result is a product that is harder to understand than either architecture would be alone.

## Why Continue Made The UX More Confusing

The intended move was: make Continue the product, demote sessions/frames/screenshots/bundles to evidence.

The implemented move was: add Continue panels while keeping most of the recorder/debug surfaces available in the main app shell.

That created several UX regressions.

### Too many primary objects

The screen now asks the user to understand all of these at once:

- local memory
- artifacts
- workstreams
- Continue freshness
- selected workstream
- current focus
- Continue target
- last meaningful state
- current focus relationship
- artifact roles
- candidate targets
- episodes
- actions
- evidence anchors
- feedback events
- breadcrumbs
- developer diagnostics
- frames
- raw events

That is not a simpler product model. It is an internal architecture diagram rendered as UI.

### Feedback controls are useful but overexposed

`Correct target`, `Wrong target`, `Only evidence`, and `Ignore workstream` are good primitives. The mistake is that they are shown as first-class controls beside the main continuation target before the product has earned a stable primary answer.

The user needs one obvious continuation path first. Correction should be available, but not compete with the core action.

### Workstream detail became a dashboard, not a product surface

The workstream detail panel exposes artifacts, candidates, episodes, actions, anchors, feedback, and breadcrumbs. This is useful for engineering validation. But as a first product surface it makes the user feel like they are inspecting a database, not resuming work.

### Debug information leaks into copy

The screenshots show raw `error_signal`, `typing_in_composer`, frame ids, artifact ids, action ids, episode ids, and raw unresolved-state JSON. These are important as evidence handles, but they should be behind "inspect evidence", not in the main reading path.

### Layout density made the architecture problem visible

The CSS added containment and grids, but the product still places too much dense content into the same scroll surface. The first screenshot's overlaying panels are a symptom of trying to fit a decision card, side rail, workstream detail, feedback, candidates, and diagnostics into one shell.

The old UI was not good enough, but it at least had a simpler mental model: capture and inspect. The new UI has better concepts but presents them all at once.

## Why It Is Slow And Memory Heavy

The live runtime evidence confirms that the app is still operating like a recorder.

Verified live capture path for this checkout:

```text
/Users/bhaskarpandit/Library/Application Support/com.smalltalk.app/capture/
```

Storage observed during this audit:

```text
capture directory: 2.9 GB
smalltalk-capture.sqlite: 540 MB
snapshots/: 895 MB
snapshot jpg files: 2,125
```

Live SQLite counts observed from the app-data capture store:

```text
frames: 1,643
ui_events: 88,641
capture_triggers: 1,771
content_units: 222,274
ax_nodes: 160,464
ocr_spans: 113,642
continue_artifacts: 18
continue_task_actions: 248
continue_episodes: 20
continue_workstreams: 3
continue_candidates: 7
continue_decisions: 149
continue_feedback_events: 152
```

This is the central technical mismatch: a small number of user-facing Continue objects are sitting on top of a very large recorder substrate.

Recent frame timing around the screenshots:

```text
1619 | 2026-07-03 23:52:34 | event_burst  | Helium    | screen_capture_kit | active_display
1620 | 2026-07-03 23:52:44 | event_burst  | Helium    | screen_capture_kit | active_display
1621 | 2026-07-03 23:52:48 | event_burst  | Helium    | screen_capture_kit | active_display
1622 | 2026-07-03 23:52:55 | event_burst  | Helium    | screen_capture_kit | active_display
1623 | 2026-07-03 23:53:02 | event_burst  | Helium    | screen_capture_kit | active_display
1624 | 2026-07-03 23:53:10 | event_burst  | Helium    | screen_capture_kit | active_display
1625 | 2026-07-03 23:53:15 | event_burst  | Helium    | screen_capture_kit | active_display
1626 | 2026-07-03 23:53:22 | event_burst  | Helium    | screen_capture_kit | active_display
1627 | 2026-07-03 23:53:25 | event_burst  | Helium    | screen_capture_kit | active_display
1628 | 2026-07-03 23:53:29 | event_burst  | Helium    | screen_capture_kit | active_display
1629 | 2026-07-03 23:53:31 | event_burst  | Helium    | screen_capture_kit | active_display
1630 | 2026-07-03 23:53:38 | event_burst  | Helium    | screen_capture_kit | active_display
1631 | 2026-07-03 23:53:40 | event_burst  | Helium    | screen_capture_kit | active_display
1632 | 2026-07-03 23:53:46 | event_burst  | Helium    | screen_capture_kit | active_display
1633 | 2026-07-03 23:53:51 | event_burst  | Helium    | screen_capture_kit | active_display
1634 | 2026-07-03 23:53:59 | event_burst  | Codex     | screen_capture_kit | active_window
1635 | 2026-07-03 23:54:11 | event_burst  | Codex     | screen_capture_kit | active_window
1636 | 2026-07-03 23:54:19 | event_burst  | smalltalk | screen_capture_kit | active_window
1637 | 2026-07-03 23:54:28 | event_burst  | Codex     | screen_capture_kit | active_window
1638 | 2026-07-03 23:54:38 | typing_pause | Codex     | screen_capture_kit | active_window
1639 | 2026-07-03 23:54:52 | idle         | Codex     | screen_capture_kit | active_window
1640 | 2026-07-03 23:55:01 | typing_pause | Codex     | screen_capture_kit | active_window
1641 | 2026-07-03 23:55:09 | event_burst  | Codex     | screen_capture_kit | active_window
1642 | 2026-07-03 23:55:21 | event_burst  | Codex     | screen_capture_kit | active_window
1643 | 2026-07-03 23:55:31 | event_burst  | Codex     | screen_capture_kit | active_window
```

This proves the product was actively recording frames while the user was typing the prompt and taking screenshots. It was not simply maintaining a lightweight continuation state.

UI event mix observed in the live store:

```text
scroll: 57,784
ax_notification: 19,781
key_down: 9,211
click: 1,560
app_switch: 301
clipboard: 4
```

The event source is noisy. The capture loop coalesces events, but the system still records a large amount of interaction metadata and stores many heavy derived rows.

The frame trigger distribution also shows the recorder nature:

```text
event_burst: 695
click: 279
scroll_stop: 237
idle: 127
accessibility_change: 104
typing_pause: 96
session_start: 86
app_switch: 18
manual: 1
```

This is the opposite of a calm product surface. Even if dedupe skips some frames, the app is still spending work to observe, classify, coalesce, and frequently capture.

## What Went Wrong In The Rebuild

The rebuild did not fail because Continue is the wrong direction. It failed because the implementation did not make the hard product cut.

The right product move was:

> Replace the session recorder UX with a Continue UX.

The implemented move was closer to:

> Keep the session recorder and add Continue objects, workstream detail, feedback controls, and eval diagnostics.

That changed the vocabulary without simplifying the experience.

### The old architecture remained authoritative

`capture.rs` still owns the runtime lifecycle. The UI status still depends heavily on frames, sessions, latest frame time, capture status, and evidence search. The floating island still uses session/resume states. `open_resume_point` still has a multi-stage compatibility path for cloud files, Continue decisions, local cards, frame ids, and resume-query candidates.

Compatibility is reasonable internally, but the product cannot let compatibility define the main experience.

### Continue rebuilds are too broad for live interaction

`get_continue_decision` currently rebuilds layers on demand by default. It clears and rebuilds the third layer, generates candidates, inserts candidate rows, inserts a decision row, and then the frontend refreshes surrounding state.

This is acceptable for a command-line eval or prototype button. It is not yet a stable runtime loop for an always-on memory product, especially while the capture loop is still ingesting high-volume events.

### The UI presents database layers instead of product decisions

The user asked for "where should I continue?" The UI answers with:

- selected workstream
- confidence
- last active
- primary artifact
- unresolved JSON
- target
- current focus relationship
- correction controls
- artifact roles
- candidate targets
- episodes/actions
- evidence anchors
- feedback events

That may be useful for debugging the engine, but it is not a humane default answer.

### The floating pill was not migrated

The floating pill is the most visible ambient product surface, but it still uses old `SessionIsland` state and cloud resume behavior. The main app says Continue, while the island still says session/trail/resume. That makes the product feel unchanged where it matters most.

### No retention policy is visible in the runtime path

The app has cleanup commands, but the live capture path still accumulated gigabytes of data. A local memory product needs a storage budget, retention windows, compaction, and a clear difference between lightweight local memory and heavy forensic evidence.

Right now the system keeps behaving as if every interaction might become a stored frame with screenshots, AX nodes, OCR spans, and content units.

## Technical Fix Direction

This section is not a redesign spec. It is the technical direction implied by the failure.

### 1. Make one surface primary

The main UI should default to one Continue answer:

- what workstream this is
- what the user was doing
- where to continue
- why this is the target
- confidence/evidence thinness
- one primary action

Everything else should be behind inspection.

### 2. Move diagnostics out of the main product path

Frames, raw events, screenshots, OCR, AX nodes, content units, paths, evals, and raw ids should remain available, but not in the primary screen. They should live behind a developer/debug mode.

### 3. Rewire the floating island to Continue

The island should call the Continue path, not cloud resume/session trail reconstruction, for its primary action.

The island should understand:

- current focus
- selected workstream
- return target
- evidence thinness
- confidence
- continue decision id

It should not require a cloud result or stopped session to resume.

### 4. Make local memory lighter than recording

The event stream should become first-class evidence without forcing screenshot frames for every meaningful interaction.

Good targets:

- typed-burst metadata as lightweight evidence
- app/window/focus changes as lightweight evidence
- scroll/click/AX notification aggregation
- semantic checkpoints only when state actually changes
- screenshot capture only for important transitions or explicit evidence

The product should not store a full screenshot/OCR/AX/frame stack every few seconds while the user types.

### 5. Bound Continue rebuild work

`get_continue_decision` should not always perform broad rebuilds during interaction. The system needs incremental or cached derived state:

- rebuild only changed windows of evidence
- avoid clearing and rebuilding all third-layer rows on every decision
- separate "refresh evidence" from "answer Continue"
- stop writing many new decision/feedback rows for repeated UI refreshes

### 6. Add storage budgets and retention

The capture store needs explicit local limits:

- max snapshot bytes
- max DB bytes or compaction strategy
- retention by age and evidence quality
- deletion of low-value repeated frames
- cleanup of safe export folders
- UI that clearly reports storage usage

Without this, Smalltalk will continue to feel like a heavy recorder even if the UI copy says local memory.

### 7. Hide raw scoring internals from user copy

The user-facing layer should never show unresolved JSON, internal artifact ids, action ids, or repeated low-level action rows as the main answer.

Use evidence ids only inside "inspect evidence".

## Bottom Line

The current system is not failing because Continue is the wrong product direction. It is failing because Continue was implemented as an additional layer inside the old recorder product instead of becoming the product.

The new architecture is real in the backend, but the old architecture still controls the runtime, the floating island, the storage behavior, and much of the UI. That is why the product became more confusing instead of simpler: the user is seeing both architectures at once.

Until the app makes a hard cut toward one Continue-first surface, a lightweight evidence model, a Continue-native island, and hidden diagnostics, it will continue to feel like a buggy session recorder with a workstream dashboard attached.
