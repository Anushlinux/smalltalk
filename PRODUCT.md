# Smalltalk Product And Technical Specification

Last updated: 2026-07-04

This document describes the current Smalltalk product as implemented in this repository. It is written for another engineer or LLM that needs to understand what the product is, how it works, which parts are active, which parts are diagnostic, and where the current architecture still leaks older recorder behavior.

Smalltalk is a desktop-first, local-first continuation product built with Tauri, Rust, React, SQLite, macOS native capture APIs, Accessibility, OCR, and optional OpenAI calls. Its primary user-facing primitive is `Continue`: a single answer that separates what the user is looking at now from where they should return to continue meaningful work.

The active product lane is the native desktop app in the repository root. The older WXT browser extension remains in `browser-extension/`, but it is not the MVP path and should not be revived unless a task explicitly asks for browser-extension work.

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
| `src-tauri/src/continuation.rs` | Native Continue semantic memory, rebuild layers, scoring, decisions, feedback, breadcrumbs, eval, and optional bounded micro-inference. |
| `src-tauri/src/lib.rs` | Tauri builder and command registration. |
| `src-tauri/src/session_island.rs` | macOS floating island bridge. Still speaks older session/resume language in several paths. |
| `src-tauri/macos/SessionIslandPanel.swift` | Native macOS panel UI. |
| `src-tauri/scripts/` | Swift helper scripts for Accessibility, OCR, window capture, native event observation, and ScreenCaptureKit support. |
| `src-tauri/src/capture_core/` | Newer modular capture-core code for event governance, quality, privacy, extraction, store behavior, episode policy, browser adapters, and resume dossier limits. The active facade remains `capture.rs`. |
| `docs/` | Technical docs, audits, architecture notes, QA notes, and product rebuild notes. |
| `browser-extension/` | Older browser-extension prototype. Not the active MVP lane. |
| `resume_query_exports/` | Generated stop-time resume-query bundles. Treat as generated unless explicitly asked to inspect. |
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
- Current focus.
- Return target or best available return point.
- Last meaningful state.
- Next action.
- Confidence and evidence notes.
- Primary `Continue here` action.
- `Inspect evidence` action.
- Alternative continuation targets when present.
- Collapsed correction controls behind `Wrong target?`.

The product card invokes local Continue by default:

```ts
await invoke("get_continue_decision", {
  input: {
    mode: "normal",
    rebuild_layers: false,
    micro_inference_enabled: false
  }
});
```

The UI opens the selected target through:

```ts
await invoke("open_resume_point", {
  input: {
    continue_decision_id: continueDecision.decision_id
  }
});
```

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
| Capture/recorder backend | Still the operational evidence substrate and still heavy. |
| Floating island | Still mostly wired to older session/cloud resume language in some paths. |

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

Stop-time resume-query bundles are separate generated artifacts under the repo root:

```text
/Users/bhaskarpandit/Documents/smalltalk/resume_query_exports/session-<sequence>-resume-query-<timestamp>-<suffix>/
  resume-query-bundle.json
  images/
```

Developer reset can also clear generated repo debug output:

- `output/`
- `resume_query_exports/`

Generated capture data, SQLite files, screenshots, safe exports, and resume-query bundles must not be committed.

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
| `ai_export_audit` | Safe AI export audit rows. |
| `maintenance_counters` | Runtime counters for storage, capture, and Continue diagnostics. |

Continue tables created by `ensure_continue_schema`:

| Table | Purpose |
| --- | --- |
| `continue_schema_migrations` | Continue schema version marker. |
| `continue_artifacts` | Stable work objects such as browser tabs, conversations, code editors, terminals, PDFs, messages, and docs. |
| `continue_artifact_observations` | Per-frame or event-derived observations of artifacts. |
| `continue_task_actions` | Derived local actions such as editing, searching, encountering an error, branching away, or returning to origin. |
| `continue_task_action_events` | Join table from task actions to native UI events. |
| `continue_episodes` | Adjacent task actions grouped into local episodes. |
| `continue_episode_actions` | Episode-to-action join rows. |
| `continue_episode_artifacts` | Artifact roles inside each episode. |
| `continue_workstreams` | Durable clusters of related episodes and artifacts. |
| `continue_workstream_episodes` | Workstream-to-episode join rows. |
| `continue_workstream_artifacts` | Durable artifact roles inside a workstream. |
| `continue_candidates` | Scored continuation candidates. |
| `continue_decisions` | Persisted Continue decisions and provenance. |
| `continue_feedback_events` | Inferred and explicit feedback about a decision. |
| `continue_breadcrumbs` | Manual local next-step notes attached to workstreams. |

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
| `open_resume_point` | Open a Continue, cloud resume, native resume, session, or frame target. |

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
- `self_capture_frames`
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
- `task_actions`
- `episodes`
- `workstreams`
- `candidates`
- `decisions`
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
| `MAX_STORED_DIAGNOSTIC_ROWS_PER_CLEANUP` | 5,000 | Cleanup diagnostic cap. |
| `MAX_RETAINED_FRAME_AGE_MS` | 7 days | Retention age for frame cleanup candidates. |
| `MAX_RETAINED_CONTINUE_DECISION_AGE_MS` | 24 hours | Retention age used to protect recent Continue decisions. |
| `LOCAL_SIGNAL_BUCKET_WINDOW_MS` | 30 seconds | Window for aggregating recent local signals. |
| `MAX_LOCAL_SIGNAL_EVENTS` | 5,000 | Maximum local signal events considered. |

## Capture Trigger Model

Smalltalk uses lightweight native events to decide when heavy evidence is worth storing.

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

## Native Capture Pipeline

A stored heavy frame follows this pipeline:

1. Resolve local capture paths.
2. Read macOS Accessibility context.
3. Apply privacy and exclusion rules.
4. Skip the frame when privacy says `skip_capture`.
5. Collect a window graph snapshot.
6. Capture full-screen or scoped screenshot evidence.
7. Attempt active-window capture when a window id is available.
8. Hash image bytes into `image_hash`.
9. Determine whether Accessibility text is strong or thin.
10. Run OCR only when Accessibility is missing or thin.
11. Resolve one `full_text` value and one `text_source`.
12. Compute `content_hash`.
13. Apply duplicate and budget checks.
14. Insert the frame row into SQLite.
15. Insert OCR rows, AX rows, content units, app contexts, sensitive regions, presence sample, frame diff, trigger finalization, and transition rows.
16. Emit `capture-frame` to the React UI.

Manual captures bypass normal duplicate suppression because the user explicitly requested evidence.

Event and idle captures can be skipped for budget, duplicate, privacy, cancellation, or Smalltalk self-observation reasons. These skips increment runtime counters.

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

The backend resolves one `text_source` and one `full_text`.

| Evidence state | `text_source` | `full_text` |
| --- | --- | --- |
| Accessibility strong | `accessibility` | Accessibility text. |
| Accessibility thin plus OCR | `hybrid` | Accessibility text plus OCR text. |
| OCR only | `ocr` | OCR text. |
| Neither source available | `null` | `null`. |

`full_text` powers FTS search, frame inspection, content-unit extraction, and downstream Continue evidence.

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

The pipeline has five layers:

1. Evidence substrate.
2. Semantic memory layer.
3. Workstream layer.
4. Local decision layer.
5. Optional bounded micro-inference layer.

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

This layer is factual. It should not infer broad intent.

### Layer 2: Semantic Memory

The semantic memory layer is rebuilt with:

```text
rebuild_continue_second_layer
```

It performs:

1. Load evidence frames for a session/lookback/limit.
2. Clear second-layer rows for those frames.
3. Resolve a stable artifact for each frame or event-backed evidence item.
4. Upsert `continue_artifacts`.
5. Upsert `continue_artifact_observations`.
6. Extract task actions.
7. Collapse repeated task actions.
8. Insert `continue_task_actions`.
9. Insert task-action event links.

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
7. Store unresolved state.
8. Insert episode/workstream join rows.

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
- `micro_inference_enabled`: false
- `max_candidates_for_model`: 5

The frontend normal path sends:

```ts
await invoke("get_continue_decision", {
  input: {
    mode: "normal",
    rebuild_layers: false,
    micro_inference_enabled: false
  }
});
```

The developer diagnostic rebuild path sends:

```ts
await invoke("get_continue_decision", {
  input: {
    mode: "rebuild",
    rebuild_layers: true,
    micro_inference_enabled: false
  }
});
```

`effective_continue_decision_mode` treats `rebuild_layers: true` as `rebuild`. Modes `rebuild`, `force_rebuild`, and `diagnostic_rebuild` force rebuild. Other modes are normal.

Normal mode can reuse a cached decision when no newer local evidence exists. Cache hits increment `decision_cache_hits`. Normal calls increment `continue_normal_calls`. Rebuild calls increment `continue_rebuild_calls`.

`get_continue_decision` does this:

1. Ensure Continue schema.
2. Normalize request defaults.
3. Determine normal versus rebuild mode.
4. Try to load a fresh cached decision when not rebuilding and micro-inference is disabled.
5. Infer pending feedback for previous decisions when no cache is used.
6. Check whether semantic layers need rebuild.
7. Rebuild second layer incrementally when needed.
8. Rebuild third layer when needed.
9. Load current focus from latest frame and artifact observation.
10. Load active, suspended, or unresolved workstreams.
11. Fall back to any recent workstream if no active/suspended workstreams exist.
12. Generate local continuation candidates.
13. Score candidates.
14. Persist candidates when not using cached decision metadata.
15. Select the highest local candidate.
16. Add warnings such as current focus differing from return target.
17. Optionally run bounded micro-inference.
18. Validate any model result.
19. Build a stable decision id.
20. Persist a `continue_decisions` row when no cached decision was used.
21. Return the decision, anchors, missing evidence, warnings, alternatives, and provenance.

Candidate kinds:

- `continue_edit`
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

### Continue Decision Result

`ContinueDecisionResult` includes:

- `decision_id`
- `mode`
- `cache_hit`
- `source`
- `model`
- `response_id`
- `current_focus`
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

## Optional Bounded Micro-Inference

OpenAI micro-inference is optional and disabled by the current frontend primary path.

It can be requested with:

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
- `next_action` is empty, too long, or incompatible with candidate semantics
- high confidence is returned for thin evidence
- a branch/support target is promoted without a strong local candidate

If the API fails, the key is missing, parsing fails, or validation fails, the decision source becomes `local_fallback` and the local scorer result is returned.

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

`open_resume_point` is the compatibility opener. It can open targets from several sources:

- Continue decision id
- cloud resume output path
- session id
- current frame id
- target frame id

The primary Continue UI now sends `continue_decision_id`.

Opening can use:

- browser URL when allowed and openable
- document path when allowed and openable
- frame fallback when no direct target is safe
- Smalltalk focus fallback when opening is blocked

Open result includes:

- strategy
- opened URL/path flags
- warnings

The UI should not fabricate targets. If the backend only provides a frame anchor, the UI should inspect that frame rather than inventing a URL or path.

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
- deleted snapshot file count
- reclaimed bytes
- summary

Cleanup should protect decision-linked frames and recent Continue evidence. Low-value duplicates, stale frames, Smalltalk self-captures, old snapshots, and oversized local evidence are cleanup candidates.

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

The macOS floating island still contains older session vocabulary:

- recording compact/expanded states
- processing state
- stopped toast
- trail reconstructing
- resume ready
- reconstruct trail action
- resume me action

The island currently still has paths that call `run_cloud_resume` or `open_resume_point` without passing a Continue decision id. That means it is not yet fully aligned with the new Continue architecture.

Correct future direction:

- Island should display a compact Continue cue, not recorder status.
- Island should route primary action through `get_continue_decision` or an existing `continue_decision_id`.
- Trail reconstruction/cloud resume should remain diagnostic or secondary.

## Current Implementation Truth

What is real now:

- Native desktop app is the active lane.
- Continue schema exists in SQLite.
- Artifacts/actions/episodes/workstreams/candidates/decisions are persisted.
- Continue can run without stopping local memory.
- Normal Continue mode can reuse cached decisions.
- Diagnostic rebuild can force semantic layer rebuild.
- The UI opens Continue targets by `continue_decision_id`.
- Correction feedback and next-step breadcrumbs are persisted.
- Local memory diagnostics expose storage size, row counts, budgets, skip counters, and cache counters.
- Heavy capture budgets and duplicate/self-capture skipping exist.
- Event-only evidence is part of the Continue recovery direction.
- Stop-time resume-query bundles still exist and are generated artifacts.
- Cloud resume is distinct from Continue.
- The floating island is not fully migrated to Continue.
- The diagnostics panel still exposes too much internal architecture when opened.

What should not be claimed:

- Do not claim the browser extension is the active MVP.
- Do not claim Stop Session is required for Continue.
- Do not claim cloud resume is the primary product engine.
- Do not claim screenshots are the only memory.
- Do not claim model output is trusted without validation.
- Do not claim storage is lightweight unless diagnostics prove it.
- Do not claim the island is fully Continue-first yet.

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
