# Continue UI Reality Audit

Last updated: 2026-07-03

This is an internal product and engineering audit of the gap between the intended Continue architecture and the UI that exists in the desktop app today.

The short version: the native Continue backend is real, but the current product surface is only partially wired to it. The app still mostly feels like a session/capture/debug tool with a Continue card added on top. That is why the product still reads as session-based even though the backend has a separate Continue layer.

## Reality Check

The documented architecture in `docs/continue-architecture.md` says Continue is the native desktop continuation engine. It does not require Stop Session and it does not use the browser extension. That is true at the backend command level.

The implemented backend includes:

- `get_continue_decision`
- stable artifacts
- artifact observations
- task actions
- episodes
- workstreams
- continuation candidates
- persisted decisions
- feedback events
- breadcrumbs

But the UI in `src/App.tsx` exposes only a thin slice of this. It calls the Continue command, shows one Continue decision card, shows a recent workstream list, lets the user save a small breadcrumb, and can open or inspect the selected evidence anchor.

That is not the same as a Continue-native product. A Continue-native product would make the workstream the main object in the app. The current app still makes capture state, sessions, frames, search results, timelines, screenshots, and diagnostics feel like the main objects.

## What Is Actually Built

The Continue backend is substantially built in `src-tauri/src/continuation.rs`.

`get_continue_decision` does real local work:

- ensures the Continue schema exists
- infers pending feedback for prior decisions
- rebuilds the second layer through `rebuild_continue_second_layer`
- rebuilds the third layer through `rebuild_continue_third_layer`
- loads factual `current_focus`
- loads recent scorer workstreams
- generates continuation candidates
- scores and sorts candidates
- persists candidate rows
- persists a Continue decision row
- returns `current_focus`, `current_activity`, `selected_workstream`, `return_target`, `resume_work_target`, `last_meaningful_action`, `unresolved_state`, `next_action`, confidence, warnings, missing evidence, and alternatives

The Tauri command bridge in `src-tauri/src/capture.rs` exposes the Continue layer through:

- `get_continue_memory_status`
- `rebuild_continue_second_layer`
- `rebuild_continue_third_layer`
- `get_recent_continue_artifacts`
- `get_recent_continue_task_actions`
- `get_recent_continue_episodes`
- `get_recent_continue_workstreams`
- `get_continue_decision`
- `add_continue_breadcrumb`
- `infer_continue_feedback`
- `run_continue_eval`

The UI now calls some of those commands. It calls `get_continue_decision`, `get_continue_memory_status`, `get_recent_continue_workstreams`, `add_continue_breadcrumb`, and `open_resume_point` with a Continue decision id.

So the backend is not fake. The layer exists, the schema exists, the scoring path exists, and the UI has a partial bridge into it.

## What Is Actually Used In The UI

The current UI uses Continue in these visible places:

- Top-level `Smalltalk Continue` header.
- Status pills for local memory, artifacts, workstreams, Continue freshness, and latest frame.
- A primary `Continue` button that invokes `get_continue_decision`.
- `ContinueDecisionCard`, which shows the selected workstream, current focus, return target, next action, confidence, evidence anchors, warnings, and alternatives.
- A basic `WorkstreamList`, populated from `get_recent_continue_workstreams`.
- A breadcrumb text area that saves a short note with `add_continue_breadcrumb`.
- A `Show evidence` path that reveals the selected frame/image anchor.
- A refresh path that re-runs Continue after new frame evidence arrives.

That is useful, but it is still a wrapper around the older capture UI. The Continue surface does not yet replace the session/capture model. It only sits above it.

The UI still uses the older capture/session model for most operational behavior:

- `start_capture` starts a capture session.
- `stop_capture` stops the active session and may build a resume-query bundle.
- `capture_status` drives session state, frame counts, latest frame, and tool availability.
- `search_captures` searches frames, scoped by the active or latest session.
- `get_recent_timeline` loads recent events, triggers, transitions, and frames.
- `get_frame`, `get_frame_detail`, and `get_frame_image_variant` drive the inspector.
- `open_resume_point` still contains compatibility logic for cloud resume results, native resume cards, explicit frames, resume-query candidates, and Continue decisions.

In practice, Continue is currently one product surface inside a session/capture shell.

## What Still Feels Session-Based

The app still feels session-based for concrete reasons.

First, capture controls are still first-class UI. The header still exposes `Start local memory`, `Pause local memory`, `Capture evidence now`, and `Delete local memory`. Even though the labels have moved away from "session", the behavior is still backed by `capture_sessions`, active session ids, latest session ids, and stop-time logic.

Second, the diagnostics panel still dominates the product model. `Developer diagnostics` contains search, capture health, session status, frame counts, event counts, transition counts, a frame timeline, screenshot viewer, overlays, verification cards, raw events, app/window context, and stored paths. That makes the app feel like a frame inspector with a Continue card, not a workstream product.

Third, search and timeline are still session-scoped. The UI's empty-query search returns latest frames for the active or latest session. The timeline query also uses the current session id. That reinforces "what happened in this session?" instead of "what workstream should I continue?"

Fourth, Stop/Pause still has product weight. `stop_capture` is not just a capture control; it can build a `smalltalk.resume_query.v2` bundle under `resume_query_exports/`. That bundle is cloud-resume infrastructure, not the core Continue layer, but the app still exposes behavior and errors from that path.

Fifth, old resume paths still exist beside Continue. `run_cloud_resume`, `get_native_resume_card`, resume-query bundle construction, and multi-stage `open_resume_point` fallback logic remain in `src-tauri/src/capture.rs`. Some of this is needed for compatibility and debugging, but it means the runtime behavior is still a blend of old resume architecture and new Continue architecture.

Sixth, workstreams are not yet the main navigation model. The UI shows a small recent workstream list, but it does not provide a real workstream home, workstream detail page, active/suspended grouping, history, unresolved state review, correction controls, or meaningful evidence drilldown by workstream.

## What Is Not Yet A Real Continue Product

Smalltalk is not yet a fully productized Continue app.

The missing pieces are not just visual polish. They are product structure:

- No first-class workstream home.
- No clear active, suspended, background, stale, or abandoned workstream UX.
- No real workstream detail surface.
- No useful episode/action/artifact drilldown from the Continue layer.
- No correction UI when Continue picks the wrong target.
- No visible feedback loop for accepted, rejected, ignored, corrected, or auto-resumed decisions.
- No ambient return cue that feels independent of manual capture/session controls.
- No clean separation between user-facing Continue and developer-only evidence inspection.
- No product-level explanation of why a workstream is actionable beyond a compact card.
- No clear affordance for "this is the work I was doing; this is where to resume; this is only supporting evidence."
- No complete removal or hiding of session/debug concepts from the primary path.

The backend supports many of these concepts better than the UI does. The UI is behind the architecture.

## Recommended Next Build Direction

The next product build should stop treating Continue as a card inside a capture app. It should make Continue the app.

The primary screen should be a Continue-first work surface:

- active workstream
- current focus
- return target
- resume work target
- unresolved state
- last meaningful action
- next action
- evidence quality
- confidence and missing evidence
- optional diagnostics behind a secondary disclosure

Sessions, frames, screenshots, timelines, resume-query bundles, and cloud result files should still exist, but they should be evidence and debug infrastructure. They should not define the first screen or the primary workflow.

A practical next UI cut:

1. Replace the top-level capture-control emphasis with a workstream status header.
2. Make the main center panel a workstream continuation surface, not a frame/resume card.
3. Move capture controls and raw diagnostics into a clearly secondary developer/debug area.
4. Give each workstream a detail surface showing its artifacts, episodes, unresolved state, candidate targets, and evidence anchors.
5. Add correction controls: "wrong target", "this was only evidence", "continue from this instead", and "ignore this workstream".
6. Treat Stop/Pause and resume-query bundle generation as infrastructure, not as the product's main state transition.

## Bottom Line

Continue is implemented enough to be real in the backend. It is not yet implemented enough to be the user's lived product.

The current product is best described as:

> A local capture/session/debug app with a partially wired Continue decision layer.

The target product should become:

> A Continue-first workstream app that uses capture, sessions, frames, screenshots, and bundles only as inspectable evidence.

Until the UI is rebuilt around workstreams and continuation decisions, users will keep feeling that the old session architecture is still the real product.
