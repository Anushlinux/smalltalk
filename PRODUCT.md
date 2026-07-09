# Smalltalk Product And Technical Specification

Last updated: 2026-07-10

Implementation baseline: commits after `f241ceba` (`Refine Continue flow and evidence-backed continuation scoring`) through the current working tree. The repository is on a case-insensitive filesystem; `PRODUCT.md` is the canonical tracked filename even when tools display it as `product.md`.

This document describes the current Smalltalk product as implemented in this repository. It is written for another engineer or LLM that needs to understand what the product is, how it works, which parts are active, which parts are diagnostic, and where the current architecture still leaks older recorder behavior.

Smalltalk is a desktop-first, local-first continuation product built with Tauri, Rust, React, SQLite, macOS native capture APIs, Accessibility, OCR, and optional OpenAI calls. Its primary user-facing primitive is `Continue`: a single answer that separates what the user is looking at now from where they should return to continue meaningful work.

The active product lane is the native desktop app in the repository root. The older WXT browser extension remains in `browser-extension/`, but it is not the MVP path and should not be revived unless a task explicitly asks for browser-extension work.

## Changes Since The Previous Product Snapshot

The previous product snapshot was last changed in commit `f241ceba` on 2026-07-05 and still described the implementation as of 2026-07-04. Ten subsequent commits, plus the current P4 working-tree changes, materially changed the product:

- Capture is explicitly sparse and event-driven. Native app, focus, Accessibility, click, key-category, scroll, and clipboard signals are stored cheaply; screenshots, AX trees, OCR, window graphs, and normalized content are stored only for accepted heavy frames.
- Long-running local memory now has concrete pressure controls: event coalescing, native scroll/AX throttling, 4-second important and 45-second low-value heavy-capture intervals, a 24-frame rolling screenshot budget for low-value captures, three unchanged heavy frames per surface, a 512 MB snapshot pressure gate, image-plus-content deduplication, Smalltalk self-capture suppression, and cleanup caps for low-value frames and events.
- Text attribution now separates active owned text from background or display-only OCR through `frame_text_resolutions`. Mixed/background text can remain evidence without being trusted as primary task text.
- Continue gained semantic moments, boundary revisions, app-activity segments, open loops, workstream state snapshots/edges, memory cells/edges, pairwise preferences, ranking priors, and decision-open telemetry.
- Current focus is no longer equivalent to the latest screenshot. `resolve_current_surface` fuses frames, events, artifact observations, app contexts, window state, and typing activity; the evidence-freshness ledger then compares that current surface with the selected target.
- Fresh non-openable current work can outrank a stale openable target. `active_current_work_unresolved` is a factual first-class result, while public `return_target` and `resume_work_target` stay null when the exact safe target is missing.
- Repeated negative feedback is enforced before ranking, in alternatives, in the model candidate pack, during validation, during cache reuse, and again at strict open time. Fresh reconfirming local evidence is required before a suppressed target can return.
- Search, docs, messages, diagnostics, terminal support output, and other branches are evidence-only by default. Explicit local branch-promotion state is required before a support branch can become a public return target.
- Weak native/editor/terminal surfaces now have bounded enrichment attempts, normalized surface snapshots, stable hashed identities, evidence-quality grading, missing-evidence labels, and truthful thin-state handling.
- Continue audits are opt-in per explicit Continue action, asynchronous, proof-first, and lean by default. Startup/background refreshes do not create bundles; full SQLite/table/frame archives require an explicit full-raw mode.
- The floating island is now a Continue consumer instead of a legacy resume bypass. It renders the typed `IslandContinueState`, opens only through a persisted `continue_decision_id`, records source-aware feedback/open telemetry, and fails closed when the main Continue policy would suppress the target.
- SQLite access is hardened for long-running capture and concurrent status/Continue work: writable connections use WAL and `synchronous=NORMAL`, all connections use a 30-second busy timeout, read-only polling uses read-only connections, and `get_continue_decision` is serialized by a process-level lock.

Commit inventory reviewed for this update:

| Commit | Date | Subject |
| --- | --- | --- |
| `2e48278d` | 2026-07-05 | Rework Continue surface and continuation flow |
| `21843795` | 2026-07-07 | Refactor continuation flow and evidence rendering |
| `978dfafd` | 2026-07-07 | Refactor continue flow and evidence handling |
| `2273684d` | 2026-07-07 | Refactor continue evidence flow and UI state handling |
| `4850dfdf` | 2026-07-07 | Refine continue evidence recovery and candidate scoring |
| `9cf0e8b5` | 2026-07-07 | Refactor continue workflow and remove legacy resume code |
| `58fb77c7` | 2026-07-08 | Refactor continue flow and evidence handling |
| `2196cdc5` | 2026-07-08 | Refactor continuation flow and evidence handling |
| `abeb3622` | 2026-07-08 | Refine continuation flow and evidence handling |
| `b0e4d0a3` | 2026-07-08 | Refine continue flow and evidence scoring |

Because those commit subjects are broad, the rest of this document describes the current code paths and persisted contracts rather than trying to infer behavior from commit titles alone.

## Product Doctrine

Smalltalk is continuation-first, not session-recorder-first.

The first product screen should answer:

1. What is the factual current focus?
2. What workstream was the user probably trying to continue?
3. What is the actionable return target?
4. What was the last meaningful state?
5. What should the user do next?
6. What evidence supports this answer?
7. What evidence is missing or thin?

Sessions, screenshots, timelines, raw events, frame inspectors, cloud resume bundles, native resume cards, search, evals, candidate score components, artifact-role tables, and raw database ids are support infrastructure. They are useful for evidence inspection and developer diagnostics, but they are not the default product.

The product must keep these concepts separate:

| Concept | Meaning | Should it become the return target by default? |
| --- | --- | --- |
| `current_focus` | The latest factual screen or artifact observed locally. | No. It may be a distraction, support page, diagnostic surface, or current app. |
| `current_activity` | A local read of what appears to be happening now. | No. It explains current behavior only. |
| `selected_workstream` | The durable cluster of actions and artifacts Smalltalk thinks the user was working on. | Sometimes. It is the context for the decision. |
| `return_target` | The artifact Smalltalk thinks the user should go back to. | Yes, when evidence quality is sufficient. |
| `resume_work_target` | The actionable target inside the workstream, kept separate from support or branch evidence. | Yes. This is the preferred product target when present. |
| `branch/support surface` | Search results, docs, terminal output, messages, or other evidence used while doing work. | No, unless local evidence says the branch itself is the unfinished task. |

Smalltalk must not invent artifacts, URLs, file paths, user intent, or next actions. If the evidence is thin, the product should say that evidence is thin and show inspectable anchors.

Smalltalk must not send broad raw history to a model and ask the model to invent intent. Model calls, where used, must be bounded to local candidate ids, evidence-backed, and locally validated.

Smalltalk must not store raw typed characters or full clipboard text. Keyboard and clipboard evidence are represented as categories, counts, hashes, and metadata.

## Repository Shape

| Path | Role |
| --- | --- |
| `src/` | React/Vite frontend for the Tauri desktop app. |
| `src/App.tsx` | Main desktop UI, Continue card, diagnostics, evidence inspector, correction controls, local memory controls. |
| `src/App.css` | Desktop shell, fixed top bar, scroll containment, Continue card, diagnostics, workstream and inspector styling. |
| `src-tauri/` | Rust/Tauri backend, command registration, capture runtime, SQLite store, Continue engine, macOS island integration. |
| `src-tauri/src/capture.rs` | Active runtime facade for capture, storage, search, safe exports, cloud resume, local memory diagnostics, cleanup, and Tauri command wrappers. |
| `src-tauri/src/continuation.rs` | Native Continue semantic memory, rebuild layers, scoring, decisions, feedback, breadcrumbs, eval, and default bounded micro-inference. |
| `src-tauri/src/lib.rs` | Tauri builder and command registration. |
| `src-tauri/src/session_island.rs` | macOS floating-island Continue gateway, typed state contract, freshness memory, source-aware actions/feedback, and no-bypass audit hooks. |
| `src-tauri/src/session_island/` | Island contract/gateway modules and tests extracted from the bridge. |
| `src-tauri/macos/SessionIslandPanel.swift` | Native macOS panel UI. |
| `src-tauri/scripts/` | Swift helper scripts for Accessibility, OCR, window capture, native event observation, and ScreenCaptureKit support. |
| `src-tauri/src/capture_core/` | Newer modular capture-core code for event governance, quality, privacy, extraction, store behavior, episode policy, browser adapters, and resume dossier limits. The active facade remains `capture.rs`. |
| `docs/` | Technical docs, audits, architecture notes, QA notes, and product rebuild notes. |
| `browser-extension/` | Older browser-extension prototype. Not the active MVP lane. |
| `resume_query_exports/` | Generated stop-time resume-query bundles. Treat as generated unless explicitly asked to inspect. |
| `continue_outputs/` | Generated full Continue audit folders. Folder names start with the capture session, for example `session-001-session-id__continue-<timestamp>__normal__<decision>`. Private/debug only; do not commit. |
| `cloud_resume_exports/`, `output/`, `target/`, local snapshot folders | Generated artifacts. Do not commit. |

## Build And Verification Commands

Use these commands from the repository root unless noted otherwise:

```bash
npm install
npm run dev
npm run tauri dev
npm run build
cd src-tauri && cargo check
cd src-tauri && cargo test
```

The normal local app path is:

```bash
npm run tauri dev
```

The frontend-only Vite server is useful for UI work but does not exercise native capture:

```bash
npm run dev
```

For Rust backend changes, at least run:

```bash
cd src-tauri && cargo check
```

For deterministic Continue, parsing, storage, cleanup, and scoring changes, run the relevant Rust tests:

```bash
cd src-tauri && cargo test
```

## Current Product Surface

The visible app is now named `Smalltalk Continue`.

The top bar has:

- Brand block: `Smalltalk` and `Smalltalk Continue`.
- Status pills for local memory, evidence age, and Continue freshness.
- Primary `Continue` button.
- Secondary `Memory` menu.

The `Memory` menu contains:

- `Start local memory`
- `Pause local memory`
- `Capture evidence now`
- `Delete local memory`

This is intentional: local capture is necessary infrastructure, but it should not be the primary product action.

The main first screen is the `ContinueDecisionCard`. Depending on evidence state, it shows:

- No-evidence state.
- Local-memory-active state.
- Continue decision state.
- Fresh openable enriched work state.
- Fresh enriched-but-not-openable state.
- Truthful thin current-work state.
- Older context with fresher thin current work.
- No-clear-continuation state.
- Inference provenance: `AI-assisted`, `Local fallback`, or `Local only`.
- Current focus.
- Return target or best available return point.
- Last meaningful state.
- Next action.
- Confidence and evidence notes.
- Primary `Continue here` action.
- `Inspect evidence` action.
- Alternative continuation targets when present.
- Collapsed correction controls behind `Wrong target?`.

The product card invokes bounded AI Continue by default:

```ts
await invoke("get_continue_decision", {
  input: {
    mode: "normal",
    rebuild_layers: false,
    micro_inference_enabled: true,
    max_candidates_for_model: 5,
    audit_output_enabled: options.writeAudit === true
  }
});
```

The UI treats provenance as product-visible state. A cloud micro-inference result with a real `response_id` is shown as `AI-assisted`; a failed or unavailable model path is shown as `Local fallback`; local scorer-only output is shown as `Local only`.

The UI also filters user-facing handoff copy. If a backend or model handoff leaks internal ids or implementation labels such as candidate ids, workstream ids, artifact ids, frame fallback text, or target-metadata placeholders, the card falls back to honest thin-evidence copy instead of displaying those internals as product language.

The UI opens the selected target through:

```ts
await invoke("open_resume_point", {
  input: {
    continue_decision_id: continueDecision.decision_id,
    target_artifact_id: resumeTarget?.artifact_id || null,
    source: "desktop_continue_card",
    strict_continue_target: true
  }
});
```

`Continue here` is rendered only for `openable_return_target`. Fresh but non-openable work uses inspect-first copy such as `Most recent work seen`, `Fresh current work`, `Exact target missing`, or `No safe return target yet`; it does not quietly fall back to an old openable frame or generic `Continue here`.

The UI records explicit correction feedback through:

```ts
await invoke("record_continue_feedback", {
  input: {
    decision_id,
    selected_candidate_id,
    workstream_id,
    target_artifact_id,
    corrected_artifact_id,
    feedback_kind,
    note,
    source: "desktop_ui"
  }
});
```

Supported explicit feedback kinds are:

- `accepted`
- `rejected`
- `ignored`
- `corrected`
- `artifact_only_evidence`
- `ignored_workstream`
- `user_next_step_note`

The frontend also has a developer diagnostics `<details>` panel. It contains workstream lists, breadcrumb notes, workstream detail, local memory storage diagnostics, cleanup controls, search, capture health, Continue eval, frame timeline, raw event stream, screenshot inspector, overlays, verification drawer, and raw path/context tabs. These are diagnostics, not the primary product.

## Current Product Failure Boundary

The backend now has a real Continue/workstream architecture, but the visible product can still feel like an old recorder/debug dashboard when diagnostics are open. This is not just visual polish. It is a product-boundary issue.

The current repo has four overlapping paths:

| Path | Current state |
| --- | --- |
| React Continue shell | Primary screen is Continue-first, but diagnostics still expose many internal layers. |
| Continue backend | Real layered semantic memory and scoring engine. |
| Capture/evidence backend | Operational evidence substrate with lightweight-first signals and sparse, budgeted heavy frames. |
| Floating island | Typed Continue-first consumer using the same backend decision and strict-open policy as the main card; legacy session/cloud routes are diagnostic-only. |

The correct product direction is to make `Continue` the only first-screen answer and keep diagnostic internals behind secondary surfaces.

## Local Storage

The live native capture store is under the Tauri app-data directory:

```text
~/Library/Application Support/com.smalltalk.app/capture/
  smalltalk-capture.sqlite
  snapshots/
  helpers/
  safe-ai-exports/
```

The frontend receives exact live paths from `capture_status`:

- `data_dir`
- `database_path`

Screenshot assets are partitioned by day under `snapshots/<day>/`. An accepted heavy capture can write a full-display JPEG named `<timestamp>_full.jpg` and, when a window id is available, an active-window JPEG named `<timestamp>_window.jpg`. The SQLite row stores the asset paths and capture provenance; the image bytes are not embedded in SQLite.

SQLite sidecars can exist beside the main database while WAL is active:

```text
smalltalk-capture.sqlite
smalltalk-capture.sqlite-wal
smalltalk-capture.sqlite-shm
```

The database is the durable local evidence and semantic-memory store across capture sessions. A one-hour session does not produce a separate database or an hour-long video. It appends lightweight event rows and a bounded number of sparse heavy frames to the same database, all linked to a `capture_sessions.id`.

Stop-time resume-query bundles are separate generated artifacts under the repo root:

```text
/Users/bhaskarpandit/Documents/smalltalk/resume_query_exports/session-<sequence>-resume-query-<timestamp>-<suffix>/
  resume-query-bundle.json
  images/
```

Developer reset can also clear generated repo debug output:

- `output/`
- `resume_query_exports/`
- `continue_outputs/`

Generated capture data, SQLite files, screenshots, safe exports, resume-query bundles, and Continue output audits must not be committed.

`continue_outputs/` is not a live mirror of the database. It is written only when an explicit Continue action asks for an audit. The default bundle is a compact proof package built in a sibling `.building` directory and atomically renamed to its final session-readable folder after canonical proof files are complete. Background/startup decisions use `audit_output_enabled: false` and do not create output folders.

## Data Model Overview

The core local database is `smalltalk-capture.sqlite`.

Important substrate tables:

| Table | Purpose |
| --- | --- |
| `capture_sessions` | Session id, sequence, start/stop timestamps, status, export path, and per-session counts. |
| `frames` | Core captured evidence rows: app, window, URL/path, text, screenshot paths, hashes, trigger, privacy, session, ScreenCaptureKit metadata. |
| `frames_fts` | FTS5 index over frame text and metadata. |
| `ocr_text` | One row per frame for joined OCR text and raw OCR JSON. |
| `ocr_spans` | Per OCR span text, confidence, bounds, indexes, and raw JSON. |
| `ax_nodes` | Accessibility nodes with roles, text, bounds, focus, actions, and raw JSON. |
| `content_units` | Normalized product-facing units derived from AX and OCR. |
| `ui_events` | Lightweight native event stream. |
| `capture_triggers` | Coalesced trigger records linking UI events to attempted captures. |
| `event_transitions` | Classified state changes between pre-frame and post-frame evidence. |
| `window_snapshots` | macOS window graph snapshots. |
| `windows` | Individual windows observed in a window graph. |
| `frame_diffs` | Simplified frame-to-frame changes. |
| `app_contexts` | Product-object adapters for apps, tabs, docs, terminals, conversations, and other surfaces. |
| `clipboard_events` | Clipboard metadata without full clipboard text. |
| `typing_bursts` | Keyboard activity summaries without raw typed characters. |
| `presence_samples` | Activity samples. |
| `exclusion_rules` | Privacy and exclusion rules. |
| `sensitive_regions` | Sensitive visual/text regions and actions taken. |
| `frame_quality_warnings` | Frame-level evidence warnings. |
| `frame_text_resolutions` | Active/background text split, quality label, attribution flags, hashes, and the resolution payload used by Continue. |
| `ai_export_audit` | Safe AI export audit rows. |
| `local_memory_maintenance` | Runtime counters and maintenance values for storage, capture, cleanup, enrichment, audit, and Continue diagnostics. |

Continue tables created by `ensure_continue_schema`:

| Table | Purpose |
| --- | --- |
| `continue_schema_migrations` | Continue schema version marker. |
| `continue_artifacts` | Stable work objects such as browser tabs, conversations, code editors, terminals, PDFs, messages, and docs. |
| `continue_artifact_observations` | Per-frame or event-derived observations of artifacts. |
| `continue_task_actions` | Derived local actions such as editing, searching, encountering an error, branching away, or returning to origin. |
| `continue_task_action_events` | Join table from task actions to native UI events. |
| `continue_semantic_moments` | Debounced meaningful changes and event/frame boundaries that can invalidate stale decisions. |
| `continue_boundary_revisions` | Persisted semantic-boundary revisions used by cache freshness. |
| `continue_episodes` | Adjacent task actions grouped into local episodes. |
| `continue_episode_actions` | Episode-to-action join rows. |
| `continue_episode_artifacts` | Artifact roles inside each episode. |
| `continue_workstreams` | Durable clusters of related episodes and artifacts. |
| `continue_workstream_episodes` | Workstream-to-episode join rows. |
| `continue_workstream_artifacts` | Durable artifact roles inside a workstream. |
| `continue_workstream_state_snapshots` | Historical workstream state/unresolved-state snapshots. |
| `continue_workstream_edges` | Relationships among workstreams. |
| `continue_branch_contexts` | Origin, branch, return, and explicit branch-promotion state for support surfaces. |
| `continue_open_loops` | Evidence-backed unfinished, blocked, completed, or unclear work between workstreams and candidates. |
| `continue_open_loop_artifacts` | Artifact roles inside an open loop. |
| `continue_open_loop_evidence` | Evidence handles supporting an open loop. |
| `continue_app_activity_segments` | Bounded app/surface activity segments used by scoring. |
| `continue_activity_classifications` | Local activity classifications and provenance. |
| `continue_candidates` | Scored continuation candidates. |
| `continue_decisions` | Persisted Continue decisions and provenance. |
| `continue_decision_open_events` | Privacy-preserving open attempts/outcomes keyed by decision and source; no raw URL/path payload. |
| `continue_feedback_events` | Inferred and explicit feedback about a decision. |
| `continue_breadcrumbs` | Manual local next-step notes attached to workstreams. |
| `continue_evidence_probes` | Bounded requests/attempts to acquire missing evidence. |
| `continue_memory_cells` / `continue_memory_edges` | Durable support/contradiction memory used during retrieval and ranking. |
| `continue_pairwise_preferences` / `continue_ranking_priors` | Learned local preference and ranking features derived from feedback/evidence. |
| `continue_surface_enrichment_attempts` | Normalized bounded weak-surface enrichment attempts. |
| `continue_surface_snapshots` | Normalized weak-surface state, identity, quality, openability, and missing-evidence snapshots. |
| `continue_eval_fixtures` | Local deterministic eval fixture storage. |

The Continue schema name is:

```text
smalltalk.continue_memory.v1
```

## Capture Runtime

The active capture runtime is in `src-tauri/src/capture.rs`.

Main Tauri commands:

| Command | Purpose |
| --- | --- |
| `start_capture` | Start local memory, create or resume a capture session, launch runtime worker, start native event source, capture initial evidence. |
| `stop_capture` | Pause local memory, stop worker, finalize session, build stop-time resume-query bundle. |
| `capture_once` | Manual heavy evidence capture. |
| `capture_status` | Return local memory status, counts, paths, latest frame, tool availability, and runtime diagnostics. |
| `delete_all_frames` | Stop runtime and clear the live capture store for a clean slate. |
| `get_local_memory_diagnostics` | Return storage, row counts, budgets, cleanup potential, and runtime diet counters. |
| `cleanup_local_memory` | Preview or apply retention cleanup of low-value local evidence. |
| `dev_reset_local_memory` | Stop runtime, clear live store, optionally clear generated debug exports. |
| `search_captures` | Search frames with SQLite FTS or return latest frames. |
| `get_frame` | Load one frame row. |
| `get_frame_image_variant` | Return screenshot data for preview or full frame rendering. |
| `get_recent_timeline` | Return recent events, triggers, transitions, and frames. |
| `get_frame_detail` | Return a frame with AX nodes, OCR spans, content units, transitions, app contexts, sensitive regions, and verification summary. |
| `validate_frame_consistency` | Verify a frame has expected linked evidence. |
| `search_content_units` | Search normalized content units. |
| `add_exclusion_rule` | Add privacy/exclusion rule. |
| `remove_exclusion_rule` | Remove privacy/exclusion rule. |
| `list_exclusion_rules` | List privacy/exclusion rules. |
| `delete_recent_captures` | Delete recent capture rows by range. |
| `export_debug_episode` | Export debug episode data. |
| `get_episode_dossier` | Build local episode dossier. |
| `build_safe_ai_export` | Build redacted safe export for model use. |
| `get_native_storyboard_dossier` | Legacy local storyboard dossier. |
| `classify_episode_transitions` | Legacy transition classifier path. |
| `get_native_resume_card` | Legacy local resume-card path. |
| `build_resume_query_bundle` | Build bounded resume-query bundle. |
| `build_session_index` | Build stop/session index metadata. |
| `run_cloud_resume` | Stop/resume-query OpenAI path. Separate from Continue. |
| `get_cloud_resume_status` | Report OpenAI key presence and model for cloud resume. |
| `get_continue_decision` | Core product decision; optionally schedules an explicit lean audit. |
| `get_continue_decision_trace` | Inspect the persisted decision pipeline without dumping broad unsafe history. |
| `get_continue_memory_status` | Return semantic-memory counts and layer state. |
| `assess_continue_evidence_sufficiency` / `request_more_continue_evidence` | Evaluate missing evidence and perform bounded probe requests. |
| `record_continue_feedback` / `infer_continue_feedback` | Persist explicit feedback or infer it from later local activity/open lifecycle. |
| `run_continue_eval` / `run_continue_replay_eval` | Deterministic fixture and replay evaluation. |
| `get_island_continue_state` / `perform_island_continue_action` | Typed island state gateway and source-aware action dispatch. |
| `open_resume_point` | Strict product open by persisted decision id plus explicitly gated diagnostic compatibility opens. |

`start_native_capture`, `stop_native_capture`, `capture_once_v2`, and `get_frame_v2` are compatibility wrappers over the active capture implementation.

## Capture Status Contract

`capture_status` returns:

- `running`
- `frame_count`
- `recent_app_labels`
- `signal_count`
- `event_count`
- `transition_count`
- `content_unit_count`
- `session_count`
- `active_session`
- `latest_session`
- `last_export`
- `started_at`
- `last_error`
- `latest_frame`
- `skipped_samples`
- `last_skipped_at`
- `data_dir`
- `database_path`
- `screenshot_tool`
- `accessibility_tool`
- `ocr_tool`
- `runtime_diagnostics`

`runtime_diagnostics` contains:

- `heavy_captures_stored`
- `heavy_captures_skipped`
- `heavy_captures_skipped_budget`
- `heavy_captures_skipped_dedupe`
- `heavy_captures_skipped_privacy`
- `heavy_captures_skipped_cancellation`
- `heavy_captures_skipped_smalltalk_self`
- `events_aggregated`
- `ocr_runs`
- `ax_snapshots`
- `continue_normal_calls`
- `continue_rebuild_calls`
- `decision_cache_hits`
- weak-surface enrichment attempt/success/skip/failure counters
- latest weak-surface attempt and snapshot ids when present

These fields matter because the current product should not measure memory only by screenshot frames. It also has lightweight local signals and Continue objects.

## Local Memory Diagnostics Contract

`get_local_memory_diagnostics` returns:

- `database_path`
- `captured_root`
- `database_bytes`
- `snapshot_bytes`
- `safe_export_bytes`
- `frame_count`
- `event_count`
- `heavy_evidence_rows`
- `continue_object_counts`
- `low_value_duplicate_frames`
- `excess_low_value_events`
- `self_capture_frames`
- `self_capture_events`
- `decision_linked_frames`
- `estimated_cleanup_potential_bytes`
- `oldest_retained_frame_ms`
- `latest_frame_ms`
- `cleanup_last_run_ms`
- `cleanup_last_result`
- `budgets`
- `runtime_diagnostics`

`heavy_evidence_rows` includes:

- `content_units`
- `ax_nodes`
- `ocr_text_rows`
- `ocr_spans`
- `app_contexts`
- `window_snapshots`
- `windows`

`continue_object_counts` includes:

- `artifacts`
- `artifact_observations`
- `semantic_moments`
- `task_actions`
- `episodes`
- `workstreams`
- `open_loops`
- `candidates`
- `decisions`
- `open_events`
- `feedback_events`
- `breadcrumbs`

`budgets` includes:

- `min_important_capture_interval_ms`
- `min_low_value_capture_interval_ms`
- `idle_capture_interval_ms`
- `rolling_window_ms`
- `max_screenshots_per_10_minutes`
- `max_screenshots_per_surface_without_change`
- `max_snapshot_dir_bytes`
- `max_retained_low_value_duplicate_frames`
- `max_retained_low_value_ui_events`
- `max_diagnostic_rows_per_cleanup`

Current budget constants in `capture.rs`:

| Constant | Value | Meaning |
| --- | ---: | --- |
| `MIN_IMPORTANT_CAPTURE_INTERVAL` | 4 seconds | Minimum spacing for important heavy captures. |
| `MIN_LOW_VALUE_CAPTURE_INTERVAL` | 45 seconds | Minimum spacing for low-value heavy captures. |
| `IDLE_CAPTURE_INTERVAL` | 120 seconds | Idle capture interval. |
| `CAPTURE_BUDGET_ROLLING_WINDOW_MS` | 10 minutes | Rolling budget window. |
| `MAX_SCREENSHOT_FRAMES_PER_10_MINUTES` | 24 | Heavy screenshot budget per rolling window. |
| `MAX_SCREENSHOT_FRAMES_PER_SURFACE_WITHOUT_CHANGE` | 3 | Maximum retained heavy frames for unchanged surface. |
| `MAX_LOCAL_SNAPSHOT_DIR_BYTES` | 512 MB | Target local snapshot directory budget. |
| `MAX_RETAINED_LOW_VALUE_DUPLICATE_FRAMES` | 400 | Duplicate cleanup threshold. |
| `MAX_RETAINED_LOW_VALUE_UI_EVENTS` | 5,000 | Retain the newest low-value scroll/AX event rows before older rows become cleanup candidates. |
| `MAX_STORED_DIAGNOSTIC_ROWS_PER_CLEANUP` | 5,000 | Cleanup diagnostic cap. |
| `MAX_RETAINED_FRAME_AGE_MS` | 7 days | Retention age for frame cleanup candidates. |
| `MAX_RETAINED_CONTINUE_DECISION_AGE_MS` | 24 hours | Retention age used to protect recent Continue decisions. |
| `LOCAL_SIGNAL_BUCKET_WINDOW_MS` | 30 seconds | Window for aggregating recent local signals. |
| `LOCAL_SIGNAL_RECENT_WINDOW_MS` | 20 minutes | Recent event window used for visible local signal counts. |
| `MAX_LOCAL_SIGNAL_EVENTS` | 600 | Maximum raw local signal events considered. |
| `MAX_VISIBLE_SIGNAL_MOMENTS` | 48 | Product-facing cap for bucketed signal moments. |
| `ISLAND_EVENT_STATUS_INTERVAL` | 900 ms | Minimum interval for refreshing island/status after native events. |

These are pressure controls, not a promise that every hour stores exactly the same amount. Important triggers such as app/window changes, clipboard activity, explicit captures, session start, and error-bearing surfaces can exceed the low-value screenshot cadence. Conversely, unchanged surfaces can produce many lightweight events and very few heavy frames.

For an hour of sustained low-value activity, the 45-second spacing permits at most about 80 heavy-capture attempts before same-surface dedupe and storage pressure reduce it further. The 24-frame rolling count can be more restrictive when important frames also occurred in the preceding 10 minutes, because those frames count toward the observed total even though important/manual/error captures themselves bypass the low-value rejection. This is not a global frame maximum. The event stream is also not capped at 5,000 during capture; 5,000 is the retention threshold used by cleanup for low-value scroll/AX rows.

## What Happens During An Hour-Long Session

There is no special one-hour mode and no hour-boundary rollover. A long session follows the same event-driven lifecycle continuously:

1. `start_capture` creates a `capture_sessions` row, starts the Rust worker and native Swift event source, and attempts a `session_start` heavy frame.
2. Native events append lightweight `ui_events`, typing-burst, clipboard, trigger, and transition evidence linked to the active session.
3. Events are throttled/coalesced. They can update freshness and semantic memory without a screenshot.
4. When a trigger settles, the worker decides whether heavy evidence is valuable. Important/manual/error triggers are favored; low-value/unchanged/self/private activity is skipped or deduped.
5. Accepted heavy evidence writes day-partitioned JPEG assets and normalized SQLite rows. AX is preferred; OCR runs only when AX is missing/thin; active/background attribution is persisted.
6. Weak surfaces can create bounded enrichment attempts/snapshots from already persisted local evidence.
7. Continue can run at any time without stopping the session. It incrementally rebuilds semantic rows, reuses a cache only when the full watermark is unchanged, and persists the exact decision.
8. Status/diagnostic polling uses short read-only connections where possible. Writes use WAL plus a busy timeout; decision work is serialized so the island and main card do not run conflicting rebuilds.
9. No `continue_outputs/` folder is produced merely because capture is running. Only an explicit audit-enabled Continue schedules a compact asynchronous bundle.
10. `stop_capture` stops the event source/worker, finalizes the session row/counts, and can build the older bounded stop-time `resume_query_exports` diagnostic bundle. Stop is not required for Continue.
11. Historical data remains in the shared local database after stop. Cleanup is user-triggered and evidence-aware; it is not tied to session duration.

The main growth sources are JPEG assets and heavy AX/OCR/window/content rows, not the small event metadata alone. The product exposes database bytes, snapshot bytes, row counts, skip counters, excess low-value rows, cleanup potential, and oldest/latest retained evidence so an hour-long session can be evaluated from measured local state rather than estimated from duration.

## SQLite Concurrency And Durability

All database access goes through shared helpers:

- writable opens configure `journal_mode = WAL` and `synchronous = NORMAL`;
- writable and read-only opens set a 30-second SQLite busy timeout;
- polling/watermark paths use `SQLITE_OPEN_READ_ONLY` where they do not need schema mutation;
- `CONTINUE_DECISION_LOCK` permits one in-process `get_continue_decision` at a time;
- cleanup checkpoints the WAL and can optionally run `VACUUM`;
- background audit export reuses the configured connection helpers instead of ad hoc SQLite opens.

These changes address the observed `Continue failed: database is locked` class of failure during active sessions. They reduce lock contention but do not turn SQLite into a multi-process server: another process holding an exclusive write or an unexpected crash can still produce an error, which must remain visible rather than being silently treated as a valid Continue result.

## Capture Trigger Model

Smalltalk uses lightweight native events to decide when heavy evidence is worth storing.

On macOS, `src-tauri/scripts/capture_events.swift` is compiled into the app-data `helpers/` directory and runs as a child process. It combines:

- `NSWorkspace.didActivateApplicationNotification` for app changes.
- `AXObserver` notifications for focused-window, focused-element, value, selected-text, window, and title changes.
- A `CGEvent` tap for mouse-down, categorized key-down, and scroll-wheel activity.
- Pasteboard change polling for clipboard metadata.

The helper caches frontmost app/window lookup for 250 ms, coalesces scroll emission to roughly one row per 650 ms, and applies notification-specific AX throttles. Rust applies a second storage throttle: scroll rows are retained no faster than 650 ms, generic AX rows no faster than 900 ms, and AX value-change rows no faster than 1.8 seconds for the same surface. This keeps long sessions from turning native notification churn into an unbounded database write stream.

Heavy captures are expensive because they can include screenshots, AX snapshots, window graphs, OCR, content-unit extraction, and SQLite writes. The product should therefore treat heavy frames as bounded evidence, not as the only memory.

Trigger types include:

| Trigger | Source | Meaning |
| --- | --- | --- |
| `session_start` | Rust runtime | Initial frame when local memory starts. |
| `manual` | UI command | User explicitly requested evidence capture. |
| `app_switch` | Native event helper | Frontmost app changed. |
| `window_focus` | Native event helper | Focused window changed. |
| `accessibility_change` | Native event helper | AX notification indicated UI content changed. |
| `click` | Native event helper | Pointer click. |
| `typing_pause` | Native event helper | Keyboard activity paused. |
| `scroll_stop` | Native event helper | Scrolling settled. |
| `clipboard` | Native event helper | Clipboard metadata changed. |
| `idle` | Runtime timer | Idle fallback when useful capture has not happened recently. |
| `event_burst` | Runtime coalescing | Multiple event kinds merged before capture fired. |

The runtime stores event rows even when a heavy screenshot is skipped. Continue can use event-backed evidence and synthetic ids such as `event-<hash>` to avoid collapsing local memory to only screenshot frames.

Visible local signal counts are intentionally bounded. The status path looks at recent event rows, buckets them into signal moments, and caps the product-facing count so a long event stream does not make the app look heavier or more precise than it is.

Native event ingestion can refresh `capture-status` and the floating island even when no heavy frame is stored. This keeps local-memory state alive during event-only periods without requiring continuous screenshots.

Trigger coalescing keeps one pending bucket. Key activity settles into `typing_pause` after about 850 ms; scrolling settles into `scroll_stop` after about 500 ms. More events arriving before the deadline extend the bucket, and mixed trigger types become `event_burst`. The event rows remain evidence even if the eventual heavy capture is skipped.

## Native Capture Pipeline

A stored heavy frame follows this pipeline:

1. Resolve app-data paths, ensure the database/schema, and create the day-partitioned snapshot directory.
2. Reuse pre-collected Accessibility context when the trigger already has it, otherwise collect foreground app, window, URL/path, selected/focused state, nodes, and text.
3. Compute a semantic fingerprint from the foreground context.
4. Apply privacy and exclusion rules before image capture; return a privacy skip without writing a frame when capture is disallowed.
5. Apply the pre-capture pressure gate. Manual/session-start and visible-error captures bypass the low-value gate; Smalltalk self-observation, exhausted rolling/snapshot budgets, or too many unchanged frames can skip heavy capture.
6. Collect the macOS window graph when available.
7. Capture a full-display JPEG with ScreenCaptureKit first and `/usr/sbin/screencapture` as fallback.
8. Attempt a ScreenCaptureKit active-window JPEG when a window id is known.
9. Hash the screenshot and record dimensions/provider/scope/SCK provenance.
10. Determine whether Accessibility text is strong or thin.
11. Run OCR against the active-window crop when present, otherwise the full display, only when Accessibility is missing or thin.
12. Attribute OCR spans to windows/surfaces, separate active owned text from background/display-only text, and persist the resolution quality/flags.
13. Resolve `text_source`, `full_text`, active `content_hash`, and image hash.
14. Apply final image-plus-content deduplication. A frame is skipped only when both image and content match; skipped temporary JPEGs are deleted.
15. Insert the frame and FTS row in SQLite.
16. Persist OCR text/spans, AX nodes, window graph, app contexts, content units, sensitive regions, presence, frame text resolution, quality warnings, frame diff, capture-trigger result, and transition rows.
17. Run bounded weak-surface enrichment over the persisted frame and store an attempt/snapshot when applicable.
18. Increment runtime counters and emit `capture-frame` to React.

Manual captures bypass normal duplicate suppression because the user explicitly requested evidence.

Event and idle captures can be skipped for budget, duplicate, privacy, cancellation, or Smalltalk self-observation reasons. These skips increment runtime counters.

Important trigger types are `manual`, `session_start`, `app_switch`, `window_focus`, and `clipboard`. They are not rejected by the 24-per-10-minute low-value budget. Non-important captures are skipped when the rolling frame count is already 24, the snapshot directory is over 512 MB, or an unchanged semantic surface already has three recent frames. Any surface with a locally detected error signal bypasses those low-value pressure gates so blocker evidence is not discarded merely to save space.

## Screenshot Capture

The current code has ScreenCaptureKit metadata fields on `frames`:

- `sck_display_id`
- `sck_window_id`
- `sck_owning_bundle_id`
- `sck_filter_summary_json`
- `sck_configuration_summary_json`
- `sck_frame_metadata_json`
- `sck_capture_mode`
- `sck_audio_policy`

The UI displays capture provider and SCK scope in diagnostics.

ScreenCaptureKit is the preferred live provider, implemented by `src-tauri/scripts/sck_screenshot.swift` through `SCScreenshotManager.captureImage`. The provider is still-image capture inside the event-driven scheduler, not continuous screen recording. A successful primary-path frame reports `capture_provider = screen_capture_kit`; an active-window capture uses `scope = active_window` and normally has a `_window.jpg` asset. If the helper fails, Smalltalk can fall back to the command-line screenshot path and records that provider instead of pretending SCK was used.

The older command-line screenshot fallback is:

```text
/usr/sbin/screencapture -x -t jpg
```

The product should treat screenshot provider details as diagnostics. The product answer should talk about workstreams and evidence quality, not which screenshot backend ran.

Each `CaptureFrame` includes:

- `id`
- `captured_at`
- `snapshot_path`
- `app_name`
- `window_name`
- `browser_url`
- `document_path`
- `focused`
- `capture_trigger`
- `text_source`
- `accessibility_text`
- `accessibility_tree_json`
- `full_text`
- `content_hash`
- `image_hash`
- `capture_provider`
- `scope`
- `display_id`
- `window_id`
- `app_pid`
- `app_bundle_id`
- `screen_scale`
- `pixel_width`
- `pixel_height`
- `full_screenshot_path`
- `active_window_crop_path`
- `active_element_crop_path`
- `phash`
- `privacy_status`
- `capture_trigger_id`
- `previous_frame_id`
- `session_id`
- ScreenCaptureKit metadata fields

## Text Extraction

Smalltalk extracts text through Accessibility first and OCR second.

### Accessibility

The primary helper is:

```text
src-tauri/scripts/accessibility_snapshot.swift
```

The Rust backend compiles or prepares Swift helpers through helper setup code and parses structured helper output.

Accessibility captures:

- frontmost app name
- app PID
- app bundle id
- focused window title
- window id
- browser URL when available
- document path from `AXDocument` when available
- selected text
- focused node
- Accessibility nodes
- node roles
- node labels
- node values
- descriptions
- bounds
- actions
- depth

Accessibility nodes in Rust include:

- `local_id`
- `parent_id`
- `role`
- `subrole`
- `role_description`
- `title`
- `value`
- `description`
- `help`
- `identifier`
- `document`
- `url`
- `selected_text`
- `selected_text_range`
- `visible_character_range`
- `number_of_characters`
- `focused`
- `enabled`
- `selected`
- `bounds`
- `actions`
- `children_count`
- `text`
- `depth`

If the Swift helper fails or returns weak signal, the backend has an AppleScript fallback path embedded in `capture.rs`.

### Thin Accessibility

Accessibility is treated as thin when the visible surface is likely canvas-heavy, browser-chrome-heavy, or low-text. Thin Accessibility is not discarded. It causes OCR to run and can produce a `hybrid` text source.

Examples of surfaces where Accessibility may be thin:

- Google Docs
- Google Sheets
- Google Slides
- Figma
- Excalidraw
- Miro
- Canva
- tldraw
- other canvas-heavy or custom-rendered UIs

Thinness can also come from too little content-like text or too much toolbar/chrome-like text.

### OCR

OCR runs when Accessibility text is missing or thin.

The primary OCR helper is:

```text
src-tauri/scripts/vision_ocr.swift
```

Apple Vision OCR is preferred on macOS. `tesseract` is the fallback when available.

OCR rows are stored in:

- `ocr_text`
- `ocr_spans`

OCR spans preserve text, confidence, block/line/word indexes, pixel bounds, normalized bounds, and raw JSON.

### Text Source Resolution

The backend preserves the legacy `text_source`/`full_text` view for search and inspection, but Continue uses the newer `frame_text_resolutions` attribution when available. That row contains active text, background text, diagnostic text, `full_text_quality`, quality flags, resolution JSON, and active-content hash.

| Evidence state | `text_source` | `full_text` |
| --- | --- | --- |
| Accessibility strong | `accessibility` | Accessibility text. |
| Accessibility thin plus OCR | `hybrid` | Accessibility text plus OCR text. |
| OCR only | `ocr` | OCR text. |
| Neither source available | `null` | `null`. |

`full_text` powers FTS search, frame inspection, content-unit extraction, and downstream Continue evidence.

For Continue, `active_frame_text_for_continue` prefers attributed active text. Raw `full_text` is blocked as primary task evidence when quality is `mixed_active_and_background`, `background_only`, `display_only_unattributed`, or `unknown`. Background OCR can therefore remain locally inspectable without being allowed to invent the current task, error, or return target.

## Content Units

Content units are normalized evidence units derived from Accessibility nodes and OCR spans.

They are stored in `content_units` with:

- `source`: `ax` or `ocr`
- `unit_type`: button, input, link, menu item, table cell, image, heading, paragraph, or unknown
- text and text hash
- semantic role
- linked AX node id or OCR span ids
- bounds
- confidence
- raw JSON

Semantic roles include:

- `toolbar`
- `browser_chrome`
- `app_sidebar`
- `composer`
- `error`
- `search_result`
- `code_editor`
- `terminal_output`
- `chat_message`
- `main_content`

Focused nodes get higher confidence than generic nodes. OCR-only units are lower confidence than Accessibility-derived units unless Accessibility is absent.

## Window And App Context

The window graph helper is:

```text
src-tauri/scripts/window_snapshot.swift
```

Window snapshots store:

- active window id
- active app PID
- active app bundle id
- screen count
- window rows

Individual windows store:

- CoreGraphics window id
- owner PID
- owner name
- bundle id
- window title
- layer
- alpha
- onscreen flag
- active flag
- bounds
- workspace
- raw metadata

App contexts adapt raw app/window/URL/path evidence into product objects.

| Surface | Object type |
| --- | --- |
| ChatGPT, Claude, or similar browser conversation | `chat_conversation` |
| Normal browser tab | `browser_tab` |
| Cursor, VS Code, Xcode, IntelliJ | `code_editor` |
| Terminal, iTerm, Warp | `terminal` |
| Preview or PDF surface | `pdf` |
| Finder | `finder` |
| Slack, Discord, Messages, WhatsApp | `messaging` |
| Notion, Linear, Notes | `notes_doc` |
| Unknown app | `unknown` |

App contexts are evidence objects. Continue can use them to resolve artifacts, but the app context itself is not automatically a return target.

## Events, Typing, Clipboard, Transitions, And Diffs

`ui_events` store native event metadata:

- event id
- timestamp
- event type
- app name
- window title
- key category
- pointer location
- scroll deltas
- modifiers
- repeat flags
- payload JSON
- session id when available

Keyboard events do not store raw typed characters. They store categories such as:

- `char`
- `enter`
- `backspace`
- `shortcut`
- `modifier`
- `escape`
- `arrow`

`typing_bursts` summarize keyboard activity without raw typed text. They can record counts, paste count, enter count, commit signal, and whether a burst looked committed.

`clipboard_events` store clipboard metadata without full clipboard text.

`capture_triggers` connect events to heavy capture attempts. They store:

- trigger id
- trigger type
- event ids that caused the trigger
- settle delay
- dedupe policy
- pre-frame id
- post-frame id
- status
- errors

`event_transitions` summarize what happened between pre-frame and post-frame evidence.

Transition labels include:

- `switched_app`
- `scrolled_to_new_section`
- `entered_input`
- `copying_evidence`
- `same_screen_idle`
- `continuing_same_task`
- `new_task`
- `unknown`

Higher-level story labels used by local resume/storyboard paths include:

- `returning_to_previous_task`
- `verification_branch`
- `possible_distraction`
- `background_media`

`frame_diffs` store simplified changes such as same app/window, changed text hashes, diff type, confidence, and summary.

## Search

The UI calls:

```ts
await invoke("search_captures", {
  query,
  limit,
  sessionId
});
```

If the query is empty, the backend returns latest frames for the active or latest session. If the query has terms, the backend builds an FTS query across:

- `full_text`
- `app_name`
- `window_name`
- `browser_url`
- `document_path`

Results include:

- frame row
- SQLite FTS snippet
- BM25 rank

Search is a diagnostic and evidence-inspection tool. It should not be the primary product model.

## Continue Architecture

Native Continue is implemented in `src-tauri/src/continuation.rs` and exposed through command wrappers in `src-tauri/src/capture.rs`.

Continue is additive on top of the local capture store. It does not require Stop Session. It does not use the browser extension. It does not send broad raw history to a model.

The implementation is easiest to understand as seven stages:

1. Evidence substrate.
2. Text attribution and weak-surface enrichment.
3. Semantic memory: artifacts, observations, actions, semantic moments, and boundary revisions.
4. Episodes, workstreams, branch contexts, state snapshots, and open loops.
5. Current-surface resolution, freshness, candidate generation, retrieval, scoring, feedback, and quality gates.
6. Optional bounded micro-inference over locally supplied candidates, with local validation and fallback.
7. Persisted decision consumed by React, the native island, strict open, feedback, trace, eval, and optional audit output.

### Layer 1: Evidence Substrate

The substrate is local SQLite evidence from capture:

- frames
- app contexts
- content units
- UI events
- triggers
- transitions
- frame diffs
- typing bursts
- clipboard metadata
- privacy markers
- search indexes
- window graph rows
- OCR rows
- AX rows
- frame text resolutions
- weak-surface enrichment attempts and snapshots

This layer is factual. It should not infer broad intent.

### Layer 2: Semantic Memory

The semantic memory layer is rebuilt with:

```text
rebuild_continue_second_layer
```

It performs:

1. Load evidence frames for a session/lookback/limit.
2. Clear second-layer rows for those frames.
3. Load normalized weak-surface snapshots and event-only surface evidence in addition to frames.
4. Resolve a stable artifact for each frame, snapshot, or event-backed evidence item.
5. Upsert `continue_artifacts` and link snapshots back to resolved artifacts.
6. Upsert `continue_artifact_observations`.
7. Extract task actions and classify conservative branch roles.
8. Collapse repeated task actions.
9. Insert `continue_task_actions` and task-action event links.
10. Build semantic moments and boundary revisions from meaningful frame/event deltas.

Result fields include:

- `processed_frames`
- `artifact_count`
- `observation_count`
- `task_action_count`
- `start_frame_id`
- `end_frame_id`

### Artifact Resolution

Artifacts are stable local work objects.

Artifact kinds:

- `browser_tab`
- `chat_conversation`
- `code_editor`
- `terminal`
- `pdf`
- `finder`
- `messaging`
- `notes_doc`
- `unknown`

Artifact identity prefers durable keys. The priority is:

1. Meaningful safe browser URL.
2. Document path.
3. App-context object id.
4. App plus window title.
5. Stable hash fallback.

Weak editor/terminal/native-agent surfaces use privacy-safe identity adapters when a direct URL/path is not available. For example, code-editor identity can combine a hashed repository root with a hashed relative file identity. Missing identity fields and merge keys are persisted for auditability; raw selected text is hashed rather than stored by the enrichment layer.

Artifacts store:

- `id`
- `artifact_kind`
- `stable_key`
- `app_name`
- `bundle_id`
- `window_title`
- `browser_url`
- `document_path`
- `display_title`
- first/last seen frame ids
- first/last seen timestamps
- `identity_confidence`
- `evidence_quality`
- `privacy_status`
- `openability`
- timestamps

Evidence quality values:

- `strong`
- `medium`
- `thin`
- `unknown`

Openability values:

- `openable`
- `frame_fallback`
- `blocked`
- `unknown`

Text source values:

- `accessibility`
- `ocr`
- `hybrid`
- `missing`

### Task Actions

Task actions are derived from local evidence. They do not store raw typed characters.

Action kinds:

- `reading`
- `editing`
- `composing`
- `searching`
- `copying_evidence`
- `reviewing_output`
- `running_command`
- `observing_command_output`
- `encountering_error`
- `navigating`
- `switching_context`
- `branching_away`
- `returning_to_origin`
- `idle_after_progress`
- `messaging_interrupt`
- `verification_branch`
- `possible_distraction`
- `unknown`

Action roles:

- `primary`
- `support`
- `branch`
- `return`
- `interrupt`
- `unknown`

Task actions store:

- action id
- frame id
- previous frame id
- artifact id
- secondary artifact id
- action kind
- action role
- trigger type
- transition label
- evidence event ids
- confidence
- local reason
- created timestamp
- collapse count
- first frame id
- last frame id
- strongest frame id

The classifier is intentionally conservative. Search branches, verification tabs, and messaging surfaces should usually become support evidence, not the default return target.

Each branch action can carry a deterministic taxonomy and promotion state. Origin -> branch -> return relationships are persisted in `continue_branch_contexts`. Openability, a good title, recency, or the model's preference are not promotion evidence. Public promotion requires newer local proof such as direct editing/composing, a visible unresolved blocker, sustained work after abandoning the origin, explicit correction/acceptance, a fresh breadcrumb, or a confident primary action on the branch.

### Weak-Surface Enrichment

Weak surfaces are screens where a normal URL/path/title is missing or insufficient, especially code editors, terminals, Codex/native agent windows, and custom-rendered tools. The enrichment subsystem lives in `src-tauri/src/continuation/enrichment.rs`.

It is bounded and metadata-driven; it is not another capture loop. Focus/event triggers can schedule an attempt, and `get_continue_decision` can perform a bounded synchronous fallback before deciding. Adapters read already persisted frames, AX/OCR/content units, app contexts, recent events, typing summaries, clipboard metadata, and window state.

Normalized rows are stored in:

- `continue_surface_enrichment_attempts`: reason, domain/adapter, budget/privacy/outcome, timestamps, missing evidence, and optional snapshot link.
- `continue_surface_snapshots`: stable surface key, app/window identity, hashed repo/file/conversation hints, task/activity state, bounded visible sample, quality, identity confidence, openability, privacy, missing evidence, and artifact link.

`evaluate_surface_snapshot_quality` deterministically produces evidence quality, identity confidence, candidate eligibility, stale-target suppression strength, openability, missing-evidence labels, and warnings. Thin/unknown snapshots may describe fresh current work but cannot become primary return targets merely because they are recent.

### Semantic Moments And Boundary Revisions

Raw event/frame sequences are converted into meaningful moments such as content change, progress, event-only activity, task/surface transition, and invalidating evidence. `continue_semantic_moments` stores the evidence-backed delta; `continue_boundary_revisions` stores revisions that invalidate stale decisions. This lets a meaningful event change Continue freshness even when the screenshot count did not change.

### Event-Only Continue Evidence

The current code includes regression coverage for Continue using event-only moments without new screenshot frames. This matters because the product no longer wants to depend on a constant stream of screenshots.

Event-only evidence can:

- update latest evidence timestamp
- influence cached decision freshness
- create artifact observations or actions using synthetic event ids
- help recent app labels and moment counts avoid getting stuck
- preserve privacy and storage budgets by not forcing heavy screenshots for every interaction

Continue must treat frames and events as evidence, but only heavy frames have screenshot previews.

### Layer 3: Episodes And Workstreams

The workstream layer is rebuilt with:

```text
rebuild_continue_third_layer
```

It performs:

1. Load task actions.
2. Clear third-layer rows.
3. Group actions into episodes.
4. Assign artifact roles inside episodes.
5. Cluster episodes into workstreams.
6. Assign durable artifact roles inside workstreams.
7. Store unresolved state and historical workstream state snapshots.
8. Build workstream relationships and branch origin/return context.
9. Build evidence-backed open loops between workstreams and candidates.
10. Insert episode/workstream/artifact join rows.

Result fields include:

- `processed_actions`
- `episode_count`
- `episode_action_count`
- `episode_artifact_count`
- `workstream_count`
- `workstream_episode_count`
- `workstream_artifact_count`
- `start_frame_id`
- `end_frame_id`

Episode states:

- `open`
- `closed`
- `merged`
- `discarded`

Episode artifact roles:

- `primary_target`
- `source_evidence`
- `branch_support`
- `output_verification`
- `blocker`
- `interruption`
- `current_focus_only`
- `unknown`

Workstream states:

- `active`
- `suspended`
- `resumed`
- `background`
- `stale`
- `abandoned`

Workstream sources:

- `local_heuristic`
- `micro_inference`

A workstream stores:

- workstream id
- state
- title candidate
- inferred intent
- primary artifact id
- created timestamp
- last active timestamp
- suspended timestamp
- confidence
- unresolved signal
- source

Unresolved signals are local JSON or local string reasons. They should not become raw product copy. The frontend productizes common internal labels before display.

Examples of unresolved states:

- idle after meaningful progress
- visible error still unresolved
- draft or composer active
- verification branch without return
- search branch without return
- copied evidence not yet applied

Open loops summarize the actual continuation boundary: last concrete progress, unfinished/blocked/completed state, next evidence-backed action when known, current-focus relation, artifact roles, quality, and supporting evidence. Candidate generation uses open loops rather than treating every recent surface as equally resumable.

### Layer 4: Local Continue Decision

The main command is:

```text
get_continue_decision
```

Default backend request values:

- `lookback_ms`: 45 minutes
- `limit`: 700
- `mode`: `normal`
- `rebuild_layers`: false
- `micro_inference_enabled`: true
- `max_candidates_for_model`: 5
- `audit_output_enabled`: false

The frontend normal path sends:

```ts
await invoke("get_continue_decision", {
  input: {
    mode: "normal",
    rebuild_layers: false,
    micro_inference_enabled: true,
    max_candidates_for_model: 5,
    audit_output_enabled: options.writeAudit === true
  }
});
```

The developer diagnostic rebuild path sends:

```ts
await invoke("get_continue_decision", {
  input: {
    mode: "rebuild",
    rebuild_layers: true,
    micro_inference_enabled: true,
    max_candidates_for_model: 5,
    audit_output_enabled: true
  }
});
```

`effective_continue_decision_mode` treats `rebuild_layers: true` as `rebuild`. Modes `rebuild`, `force_rebuild`, and `diagnostic_rebuild` force rebuild. Other modes are normal.

Normal mode can reuse a cached decision when no newer local evidence exists. Cache hits increment `decision_cache_hits`. Normal calls increment `continue_normal_calls`. Rebuild calls increment `continue_rebuild_calls`.

The Tauri wrapper serializes `get_continue_decision` with `CONTINUE_DECISION_LOCK`. This avoids two overlapping main-card/island decision rebuilds fighting over the same SQLite writer. Watermark/status reads use read-only connections, and React's post-decision island sync passes `allow_refresh: false` so it does not immediately launch a second decision.

`get_continue_decision` does this:

1. Ensure Continue schema.
2. Normalize request defaults.
3. Determine normal versus rebuild mode.
4. Build an evidence watermark from frames, events, semantic moments, boundary revisions, feedback, opens, and surface snapshots.
5. Try a cached decision only when inference policy, watermark, boundary state, feedback/open watermarks, and freshness still match.
6. Infer matured pending feedback for prior open events when no cache is reused.
7. Run bounded pre-decision weak-surface enrichment when current evidence needs it.
8. Rebuild semantic layer 2 and workstream layer 3 incrementally when needed.
9. Resolve current surface by fusing frames, events, artifact observations, app contexts, window state, typing activity, and enriched snapshots.
10. Derive `active_current_work_unresolved` separately from any return target.
11. Load workstreams, state snapshots, graph relationships, and open loops.
12. Generate candidates, including fresh non-openable `continue_current_work` candidates.
13. Apply app-activity features and retrieve local memory support/contradiction cells.
14. Apply ranking priors, feedback aggregation, hard suppression, branch-promotion eligibility, scoring, and risk caps.
15. Persist locally generated candidates and select the best eligible local candidate.
16. Evaluate the quality gate and initial output mode: `strong_continue`, `thin_continue`, or `no_clear_continuation`.
17. Build a candidate-bounded model pack after removing feedback-suppressed and unpromoted branch candidates.
18. Run micro-inference when enabled and eligible candidates exist, then validate the selected ids, semantics, evidence quality, feedback state, branch state, and public copy.
19. Build the evidence-freshness ledger and suppress stale target revival when fresher current work exists.
20. Compose locally governed handoff copy and gate public `return_target` / `resume_work_target` independently from diagnostic candidates.
21. Build a stable decision id and persist `continue_decisions` when appropriate.
22. Return the decision, current-work fact, quality signals, anchors, support evidence, alternatives, freshness, retrieval, provenance, warnings, and optional audit path.
23. If and only if `audit_output_enabled` is true, schedule the lean proof-first audit asynchronously after the decision is ready.

Candidate kinds:

- `continue_edit`
- `continue_current_work`
- `return_to_primary_artifact`
- `resolve_error`
- `verify_output`
- `continue_reply`
- `read_next_source`
- `finish_search`
- `rerun_command`
- `resume_chat_reasoning`
- `evidence_only`

Scoring components:

- `actionability_score`
- `primary_target_score`
- `unresolved_score`
- `branch_origin_score`
- `evidence_quality_score`
- `recency_score`
- `openability_score`
- `privacy_safety_score`
- app-activity, memory support/contradiction, feedback-prior, work-value, resume-likelihood, divergence, objective-relation, interaction-depth, and evidence-sufficiency features

A candidate stores:

- candidate id
- workstream id
- target artifact
- candidate kind
- last meaningful action
- evidence frame id
- supporting episode id
- total score
- score components
- local reason
- missing evidence
- warnings
- resume work target

Branch and support targets can be evidence without being default return targets.

Hard suppression is applied before sorting/selection. Candidates rejected by feedback or blocked branch-promotion state are excluded from public alternatives and from the model pack, and a live suppression check runs again in `open_resume_point` so an old persisted decision cannot reopen a newly rejected target.

### Continue Decision Result

`ContinueDecisionResult` includes:

- `decision_id`
- `mode`
- `cache_hit`
- `cache_bypass_reasons`
- `source`
- `model`
- `response_id`
- `current_focus`
- `active_current_work_unresolved`
- `p0_quality_signals`
- `current_activity`
- `selected_workstream`
- `return_target`
- `resume_work_target`
- `candidate_kind`
- `last_meaningful_action`
- `unresolved_state`
- `next_action`
- `confidence`
- `confidence_label`
- `evidence_anchors`
- `missing_evidence`
- `warnings`
- `validation_failures`
- `alternatives`
- `generated_candidates`
- `validation_status`
- `handoff`
- `support_evidence`
- feedback/open watermarks and suppression/filter counters
- branch-selection/filter counters and validation failures
- `continue_output_mode`
- evidence watermark and latest boundary revision
- `current_surface_resolution`
- `evidence_freshness_ledger`
- Continue dossier and memory-retrieval report
- observe-before-decide and weak-surface enrichment diagnostics
- app-activity summary and quality gate
- micro-inference requested/attempted/result-kind fields
- optional `continue_output_path`

Decision sources:

- `local_scorer`
- `cloud_micro_inference`
- `local_fallback`

Validation statuses:

- `valid`
- `fallback`
- `rejected`
- `thin_evidence`

Confidence labels are derived from numeric confidence. Low confidence should be presented as best available evidence, not as certainty.

### Evidence Anchors

A Continue answer must be explainable through anchors:

- frame ids
- action ids
- episode ids
- artifact ids

The UI should productize these into evidence previews and concise explanations. Raw ids belong in diagnostics.

## Default Bounded Micro-Inference

OpenAI micro-inference is the default Continue path. It is still bounded to local candidate ids, locally validated, and cache-aware. It does not receive broad raw history.

The normal request is:

```ts
await invoke("get_continue_decision", {
  input: {
    mode: "normal",
    rebuild_layers: false,
    micro_inference_enabled: true,
    max_candidates_for_model: 5
  }
});
```

OpenAI key lookup:

- process environment `OPENAI_API_KEY`
- project `.env`

Model selection priority:

1. request `model`
2. `SMALLTALK_CONTINUE_OPENAI_MODEL`
3. `SMALLTALK_OPENAI_MODEL`
4. `OPENAI_MODEL`
5. default `gpt-4.1-mini`

The model receives a compact candidate pack only. It contains:

- current focus facts
- top workstreams
- top continuation candidates
- candidate ids generated locally
- target artifact ids
- target kinds and titles
- booleans for URL/path availability
- local score components
- last meaningful action summaries
- unresolved-state reasons
- evidence frame/action/episode ids
- missing evidence notes
- artifact role map
- short manual breadcrumbs
- factual unresolved current-work state when present
- bounded current-surface/enrichment quality and freshness facts
- support evidence that is visible to reasoning but not selectable as a return target
- feedback policy and branch-promotion eligibility already computed locally

The model does not receive:

- raw screenshots by default
- raw timelines
- raw database dumps
- raw typed characters
- full clipboard text
- unredacted URLs
- unredacted file paths
- frames excluded by privacy policy

Structured output fields:

- `selected_candidate_id`
- `selected_workstream_id`
- `intent_label`
- `next_action`
- `reason`
- `confidence`: `low`, `medium`, or `high`
- `uncertainty_notes`

The model output is validated locally. The validator rejects output when:

- selected candidate id was not supplied locally
- selected workstream id does not match the selected candidate
- selected candidate was not sent to the model
- output mentions unsupported URLs or paths
- output leaks internal candidate, workstream, artifact, frame, or fallback identifiers into handoff copy
- `next_action` is empty, too long, or incompatible with candidate semantics
- high confidence is returned for thin evidence
- a branch/support target is promoted without a strong local candidate
- the candidate was suppressed by feedback, excluded by branch policy, or omitted from the filtered model pack
- a stale target is selected over fresh strong/medium current work

If the API fails, the key is missing, parsing fails, or validation fails, the decision source becomes `local_fallback` and the local scorer result is returned.

Fallback decisions are still cached when their evidence watermark and inference policy match the next normal request. This matters because default micro-inference should not repeatedly attempt network/model work when the same local evidence already produced a validated local fallback.

The decision layer also refuses to present a selected candidate as a clear continuation when there is no human-readable return target. In that case it adds `thin_evidence:no_human_return_target`, lowers confidence, suppresses model handoff output, and returns no-clear-continuation copy rather than exposing internal target metadata.

Micro-inference cannot override the local safety gates. A model choice does not create promotion evidence, restore a feedback-suppressed candidate, make a thin snapshot openable, or turn diagnostic/support evidence into a public target.

## Feedback And Breadcrumbs

Continue feedback has two forms:

1. Inferred feedback.
2. Explicit UI feedback.

The inferred command is:

```text
infer_continue_feedback
```

It can infer:

- `accepted`: user returned to suggested target and stayed or acted there
- `rejected`: user opened target but quickly left with no meaningful action
- `ignored`: no target activity appeared inside the observation window
- `corrected`: user chose another artifact shortly after Continue
- `auto_resumed`: user naturally returned to the workstream without using the suggestion

The explicit command is:

```text
record_continue_feedback
```

It supports:

- `accepted`
- `rejected`
- `ignored`
- `corrected`
- `artifact_only_evidence`
- `ignored_workstream`
- `user_next_step_note`

Explicit feedback is deduped through deterministic ids. Notes are capped at 500 characters in the backend. The frontend currently caps breadcrumb text to 240 characters before sending.

Feedback is aggregated through the versioned `feedback_obedience.v1` policy rather than a count-only rule. The reducer weighs explicit and inferred events, distinguishes soft score caps from hard suppression, records the last negative event, and looks for fresh reconfirming evidence after that event. Repeated rejection/ignore signals can make a target ineligible for primary selection; `artifact_only_evidence` keeps an artifact as evidence while suppressing public promotion.

`continue_decision_open_events` records the lifecycle needed to infer feedback and invalidate caches: decision/candidate/workstream/artifact ids, source, whether an open was attempted/allowed/succeeded, strategy, timestamp, and bounded warnings. It intentionally does not store raw URL or path text. A matured open without confirming activity, or new explicit feedback, changes the feedback/open watermark and prevents a stale cached decision from being reused.

Negative feedback from the main card or island triggers a rebuilt Continue answer instead of leaving the rejected target visible. The same live feedback state is checked again during strict open.

Breadcrumbs are stored through:

```text
add_continue_breadcrumb
```

A breadcrumb is a short local-only note on a workstream. It can be included in later bounded candidate packs. It must not be treated as an external artifact.

## Workstream Detail

The frontend loads workstream detail only in diagnostics:

```ts
await invoke("get_continue_workstream_detail", {
  input: {
    workstream_id: selectedWorkstreamId,
    decision_id: continueDecision?.decision_id || null
  }
});
```

The detail result contains:

- workstream summary
- artifact details
- episode details
- candidate details
- latest decision summary
- feedback events
- breadcrumbs
- evidence anchors

This is excellent for debugging but too dense for the default product surface.

## Continue Eval

The eval command is:

```text
run_continue_eval
```

Default fixture invocation:

```ts
await invoke("run_continue_eval", {
  evalFilePath: null
});
```

Custom fixture invocation:

```ts
await invoke("run_continue_eval", {
  evalFilePath: "/absolute/path/to/continue-eval.json"
});
```

Eval report fields:

- schema
- case count
- target artifact correctness
- Recall@k
- MRR
- current-focus false-positive rate
- hallucinated artifact count
- model validation fallback rate
- last-state and next-action specificity
- support-branch handling, false-promotion rate, origin recall, and promoted-branch precision
- thin-evidence honesty, truthful thin-mode rate, and no-clear-continuation correctness
- current-focus/return clarity, open-loop recovery, source provenance, and quality-gate correctness
- cache freshness, fresh-current-work retention, and stale-target suppression
- feedback suppression exposure, corrected-artifact preference, stale feedback-cache hits, model feedback violations, and suppressed-target open attempts
- weak-surface enrichment attempt/success/quality, truthful thin rendering, candidate recall, stale-URL false positives, fake-open targets, missing-evidence rendering, and privacy violations
- P1 feedback-gate and P2 support-gate regression counters
- island bypass, legacy-primary-route, missing-decision-id open, suppressed-target open, main-card disagreement, and valid-open success counters
- per-case results

Eval belongs in Developer diagnostics.

## Stop-Time Resume Query Path

The stop-time cloud resume path is separate from Continue.

When the user pauses local memory through `stop_capture`, the backend:

1. Stops runtime.
2. Marks session stopped.
3. Refreshes status/counts.
4. Builds a bounded resume-query bundle.
5. Writes generated artifacts under `resume_query_exports/`.
6. Returns `StopCaptureOutput`.

`StopCaptureOutput` includes:

- `status`
- `session`
- `export`
- `resume_query`
- `preview`

The resume-query schema in `capture_core/resume_dossier.rs` is:

```text
smalltalk.resume_query.v2
```

Default resume-query policy:

- `max_json_chars`: 25,000
- `max_model_images`: 12
- `max_episode_cards`: 8

Requested JSON and image limits are capped at those defaults.

Resume-query bundles can include `recent_surface_context`. This is context-only evidence extracted from rejected browser-chrome anchors, especially tab-strip titles. These labels can explain that another browser tab was briefly visible, but they are explicitly not resume anchors and should not become the return target.

The stop-time path is useful for bounded cloud reasoning, but it is not the core Continue engine.

## Cloud Resume Path

`run_cloud_resume` is the older stop/resume-query model path. It should not be confused with `get_continue_decision`.

Cloud resume:

- builds or reuses a bounded resume-query bundle
- makes an OpenAI Responses API call when configured
- can request targeted follow-up evidence when the model says `need_more_evidence`
- validates anchor contracts locally
- persists source/provenance
- requires a real `response_id` for a trusted cloud result

Trusted cloud output has:

- `source: "cloud"`
- non-empty `response_id`

Local fallback output is explicitly `source: "local_fallback"`.

The user has previously treated fake cloud success as a correctness failure, so the UI and docs must preserve provenance.

## Open Resume Point

`open_resume_point` still supports diagnostic compatibility inputs, but product opens are now source-aware and policy-gated. It can resolve targets from:

- Continue decision id
- cloud resume output path
- session id
- current frame id
- target frame id

The primary React card sends `continue_decision_id`, optional displayed `target_artifact_id`, `source: "desktop_continue_card"`, and `strict_continue_target: true`. The island sends `source: "island_primary"`, `strict_continue_target: true`, and only a persisted decision id.

Opening can use:

- browser URL when allowed and openable
- document path when allowed and openable
- diagnostic frame fallback only when the caller explicitly uses a diagnostic source with `diagnostic_allowed: true`
- Smalltalk focus fallback when opening is blocked

Open result includes:

- strategy
- opened URL/path flags
- warnings

The UI should not fabricate targets. If the backend only provides a frame anchor, the UI should inspect that frame rather than inventing a URL or path.

Strict Continue open resolves the exact target associated with the persisted decision, proves that artifact belongs to the decision/workstream, rechecks current feedback suppression, requires a real direct locator for public opening, and refuses stale legacy fallback fields. `island_primary` fails closed unless all strict conditions hold. Every attempt is recorded best-effort in `continue_decision_open_events` without raw URL/path text.

## Privacy And Security

Privacy boundaries:

- Do not store raw typed characters.
- Do not store full clipboard text.
- Store keyboard categories, counts, and commit signals instead.
- Store clipboard metadata, hashes, and provenance instead of content.
- Apply exclusion and privacy rules before storing heavy frames.
- Mark or skip sensitive frames.
- Preserve `privacy_status`.
- Store sensitive regions and actions taken.
- Exclude `never_send_to_ai` frames from model-facing exports.
- Redact raw URLs and file paths from bounded micro-inference packs.
- Do not commit `.env`, API keys, SQLite DBs, screenshots, capture exports, or resume-query exports.

Safe AI export means derived/redacted evidence plus audit rows, not raw screenshot dumps.

`continue_outputs/` is different. It is a private developer proof bundle created only for explicit Continue actions. The default audit is lean and includes the decision trace, final decision/handoff, quality and freshness gates, candidate/feedback/branch state, current-surface and weak-surface diagnostics, model/cache/copy validation, selected evidence closure, manifest, integrity metadata, and `explain.md`. It does not copy the whole SQLite database, every table, or every frame by default.

Full SQLite snapshots, streaming raw-table NDJSON, schema dumps, and all-frame capture archives are opt-in through `SMALLTALK_CONTINUE_AUDIT_FULL_RAW=1` or an effective mode containing `full_raw`. Audit output may still contain sensitive local evidence and paths; it is generated private output and must never be committed or uploaded accidentally.

## Cleanup And Retention

The current app includes developer-facing cleanup controls:

- `Preview cleanup`
- `Apply cleanup`
- `Dev reset`

`cleanup_local_memory` accepts:

- `include_debug_exports`
- `vacuum`
- `dry_run`

It returns:

- diagnostics
- dry-run flag
- candidate frame count
- protected frame count
- deleted frame count
- deleted event-row count
- deleted snapshot file count
- reclaimed bytes
- summary

Cleanup is explicit; the capture worker does not silently delete history merely because a session reaches one hour. Preview is the default.

Frame cleanup candidates are:

- frames older than seven days, in bounded batches;
- old Smalltalk self-captures;
- low-value `typing_pause`, `scroll_stop`, `click`, `accessibility_change`, `event_burst`, and `idle` frames beyond the newest 400.

Event cleanup candidates are:

- Smalltalk self-events;
- low-value scroll/AX/accessibility rows beyond the newest 5,000.

Cleanup protects frame ids referenced by candidates, task-action first/last/strongest evidence, semantic moments, episodes, artifact observations, explicit/manual/hotkey evidence, and decision/high-value privacy markers. It deletes dependent relational rows and JPEG assets, removes orphan snapshots, prunes old unreferenced decisions/candidates, checkpoints the WAL, and optionally vacuums. Recent decisions keep at least the newest 100 decision rows even beyond the 24-hour age boundary; selected candidates and feedback links are preserved consistently.

The 512 MB snapshot constant is a pressure gate for new non-important heavy captures, not automatic deletion. If the directory is over budget, important/manual/error evidence can still be accepted and the user can preview/apply cleanup later.

`dev_reset_local_memory` is stronger. It stops runtime, clears live frames/events/derived Continue rows/snapshots, and can clear debug exports.

## Frontend Runtime Behavior

Important frontend state includes:

- `status`
- `continueMemory`
- `memoryDiagnostics`
- `cleanupResult`
- `continueDecision`
- `continueDecisionFrameCount`
- `continueDecisionUpdatedAt`
- `workstreams`
- `selectedWorkstreamId`
- `workstreamDetail`
- `feedbackStatus`
- `evalReport`
- `selectedFrame`
- `frameDetail`
- `timeline`
- `imageData`
- `evidenceOpen`
- `diagnosticsOpen`

The app refreshes status immediately on mount and then polls:

- every 1.5 seconds while local memory is running
- every 6 seconds while stopped

When running and diagnostics are open, it also refreshes:

- Continue memory
- search results
- timeline
- workstreams

When a `capture-frame` event arrives, the frontend refreshes status and Continue memory. If no frame is selected, it selects the new frame. When diagnostics are open, it refreshes workstreams.

The frontend auto-runs Continue once when:

- no Continue decision exists
- no busy action is active
- frame count is greater than zero
- the auto-continue guard has not already fired

The UI marks a decision stale when the live frame count exceeds the frame count used when the decision was made. This is only a freshness hint; event-only evidence can also affect backend cache decisions.

The production freshness path is broader than frame count. React maintains a `ContinueEvidenceSnapshot`/`ContinueFreshness` signature covering frames, events/signals, Continue-memory counts, and island/backend update information. Refreshes are debounced, guarded against overlap, and avoid recomputing the same stale signature. The backend remains authoritative through its evidence watermark, boundary revisions, surface snapshots, feedback watermark, and open watermark.

Startup/background Continue calls use `writeAudit: false`. Main Continue, Refresh Continue, diagnostic Rebuild Continue, and explicit island Continue can request `writeAudit: true`. After the main card receives a decision, it synchronizes the island using the existing decision id with `allow_refresh: false` rather than launching duplicate work.

## Diagnostics UI

Developer diagnostics include:

- Workstream list.
- Next-step note form.
- Workstream detail.
- Local memory storage metrics.
- Cleanup controls.
- Rebuild Continue button.
- Search captured evidence.
- Capture health strip.
- Continue eval panel.
- Evidence timeline.
- Raw event stream.
- Frame screenshot viewer.
- Overlay controls for content units, OCR, AX, and privacy.
- Verification drawer.
- Text/events/context/path tabs.

Diagnostics are intentionally detailed but should not be mistaken for the first-run product experience.

## Session Island

The macOS floating island is now a typed Continue-first consumer. Rust exposes `IslandContinueState` (`smalltalk.island_continue_state.v1`) with:

- display state;
- decision id;
- current focus/activity;
- selected workstream title;
- return and resume-work target summaries;
- next action and confidence label;
- missing evidence, warnings, and suppression reasons;
- typed `available_actions`.

Swift decodes this nested DTO and dispatches typed actions such as `refresh_continue`, `open_continue_target`, `mark_wrong_target`, `mark_not_useful`, `inspect_evidence`, `open_smalltalk`, `start_local_memory`, and `capture_evidence_now`. Legacy cloud/session/trail/native-resume routes remain diagnostic-only and cannot supply the island's primary target/open behavior.

The island obtains state from the same `get_continue_decision` backend contract as the main card or from a fresh remembered decision. Its primary open requires `source = island_primary`, `strict_continue_target = true`, and a non-empty `continue_decision_id`; legacy path/session/frame fallbacks are rejected before resolution. Feedback uses existing feedback kinds with `source = island_primary`. Frame/event, feedback, and open watermarks invalidate remembered island state.

P4 no-bypass coverage writes sanitized `decision/island_continue_audit.json` metadata and tracks island bypass, legacy primary route, open-without-decision-id, suppressed-target open, main-card disagreement, and valid-open counters. The repeatable manual checklist is `docs/p4-island-no-bypass-manual-qa.md`.

## Current Implementation Truth

What is real now:

- Native desktop app is the active lane.
- Continue schema exists in SQLite.
- Capture events, sparse heavy frames, attributed text, weak-surface snapshots, artifacts/actions/semantic moments/episodes/workstreams/open loops/branches/candidates/decisions/opens/feedback are persisted locally.
- Continue can run without stopping local memory.
- Normal Continue mode can reuse cached decisions.
- Default micro-inference can fall back locally and reuse that cached fallback when evidence has not changed.
- Continue handoff copy is persisted and filtered so internal candidate/workstream/artifact/frame ids do not become user-facing product text.
- Only explicit audit-enabled Continue actions schedule a `continue_outputs/` bundle. Startup/background calls do not write one. Folder names begin with the resolved capture session label.
- Audit generation runs asynchronously and is lean/proof-first by default; full raw archives are opt-in.
- Continue micro-inference audit events record the candidate pack, OpenAI request body without secrets, raw response, parsed output, validation result, failures, and fallback reason.
- Diagnostic rebuild can force semantic layer rebuild.
- The UI opens Continue targets by `continue_decision_id`.
- Correction feedback and next-step breadcrumbs are persisted.
- Local memory diagnostics expose storage size, row counts, budgets, skip counters, and cache counters.
- Heavy capture budgets and duplicate/self-capture skipping exist.
- Long sessions retain lightweight event-only evidence without forcing a screenshot per event, and cleanup can prune excess low-value event rows while preserving semantic/decision evidence.
- Event-only evidence is a first-class part of current-surface resolution, semantic moments, cache freshness, and weak-surface recovery.
- Visible local signal counts are recent-windowed and capped.
- Stop-time resume-query preserves rejected browser tab-strip titles as context-only evidence.
- Stop-time resume-query bundles still exist and are generated artifacts.
- Cloud resume is distinct from Continue.
- Support branches, stale openable targets, feedback-suppressed targets, and thin weak surfaces are hard-gated before public selection/open.
- The floating island is aligned to the backend Continue contract and fails closed on legacy primary-open bypasses.
- SQLite uses WAL, a 30-second busy timeout, read-only polling connections, and serialized Continue decision work to reduce long-session lock contention.
- The diagnostics panel still exposes too much internal architecture when opened.

What should not be claimed:

- Do not claim the browser extension is the active MVP.
- Do not claim Stop Session is required for Continue.
- Do not claim cloud resume is the primary product engine.
- Do not claim screenshots are the only memory.
- Do not claim model output is trusted without validation.
- Do not claim storage is lightweight unless diagnostics prove it.
- Do not claim a one-hour session stores a continuous replay, a fixed frame count, or automatic hour-boundary cleanup.
- Do not claim the 512 MB snapshot budget is a hard disk cap; it gates new non-important heavy captures and requires explicit cleanup to reclaim space.
- Do not claim the island can open a legacy session/frame/path target as a primary Continue action.

## Product Copy Rules For Future LLMs

Use these product words:

- Continue
- local memory
- current focus
- return target
- workstream
- evidence
- next action
- confidence
- missing evidence
- correction

Avoid making these words first-class product copy on the primary screen:

- session
- recorder
- frame id
- action id
- episode id
- artifact id
- raw event stream
- bundle
- scorer
- candidate score
- resume query
- cloud resume
- FTS
- SQLite

Those terms belong in diagnostics and technical docs.

## Implementation Checklist For Future Changes

When changing Smalltalk, check these boundaries:

1. Does the primary screen still produce one continuation answer?
2. Does Continue work without Stop Session?
3. Are `current_focus`, `return_target`, and `resume_work_target` still separate?
4. Are branch/support surfaces prevented from becoming default return targets unless evidence supports it?
5. Does every answer have frame/action/episode/artifact anchors?
6. Does thin evidence remain explicit?
7. Are raw typed characters and full clipboard text still excluded?
8. Are generated exports ignored?
9. Are model calls bounded to candidate ids and locally validated?
10. Are diagnostics kept out of the first-run product surface?
11. Are storage budgets, cleanup, and skip counters preserved?
12. Are tests added for deterministic classifier/scoring/storage changes?

## Recommended Verification For A Product Change

For frontend-only changes:

```bash
npm run build
```

For backend command, schema, capture, storage, or Continue changes:

```bash
cd src-tauri && cargo check
cd src-tauri && cargo test
```

For UI/product behavior:

```bash
npm run tauri dev
```

Manual QA should verify:

- The first screen reads as Continue, not recorder/debug.
- Continue runs while local memory is active.
- Continue also returns a thin-evidence answer when evidence is insufficient.
- Current focus and return target are visibly separate.
- The primary target can be opened or falls back to evidence inspection.
- Wrong-target correction records feedback.
- Alternatives can be selected without inventing missing URLs.
- Diagnostics are hidden until opened.
- Memory cleanup preview does not delete protected decision-linked frames.
- Delete/reset clears live UI state after clearing backend state.

## Glossary

| Term | Meaning |
| --- | --- |
| Continue | Main product action that returns the user to the next actionable point. |
| Local memory | Local evidence store built from events, frames, text, app context, and derived semantic rows. |
| Frame | Heavy captured evidence row, usually with screenshot and text sources. |
| Signal | Lightweight event or evidence count that may not include a screenshot. |
| Artifact | Stable local work object such as tab, doc, conversation, editor, terminal, message thread, or PDF. |
| Observation | Evidence that an artifact appeared in a frame or event-backed moment. |
| Task action | Local inferred action such as editing, searching, encountering an error, or returning to origin. |
| Episode | Adjacent actions grouped by continuity and boundary reasons. |
| Workstream | Durable cluster of related episodes and artifacts. |
| Candidate | A scored possible continuation target. |
| Decision | Persisted Continue answer with source, confidence, validation, warnings, and anchors. |
| Current focus | Latest factual observed screen/artifact. |
| Return target | Where Smalltalk thinks the user should go back. |
| Resume work target | The actionable target inside the workstream. |
| Breadcrumb | Manual local next-step note attached to a workstream. |
| Feedback event | Explicit or inferred signal about whether a Continue decision was useful. |
| Resume-query bundle | Stop-time bounded export for cloud resume, separate from native Continue. |
| Cloud resume | Older OpenAI path over resume-query bundles. |
| Micro-inference | Optional candidate-bounded OpenAI ranking/phrasing layer for Continue. |
| Evidence anchor | Frame/action/episode/artifact id that explains a Continue result. |
