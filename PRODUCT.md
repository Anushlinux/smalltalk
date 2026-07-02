# Smalltalk Product And Technical Snapshot

Last updated: 2026-07-03

Smalltalk is currently a desktop-first, local screen-memory app built with Tauri, Rust, React, and SQLite. The older Chrome extension still exists in `browser-extension/`, but the active product direction is the native app in the repo root.

The product goal is simple: when a user stops working, detours, or comes back later, Smalltalk should have enough local evidence to explain what they were doing, what changed, what text or UI mattered, and where they should resume. It should be inspectable before it is intelligent. Every resume cue should point back to frames, events, text sources, app/window context, and exportable evidence. The current product distinction is important: `current_focus` is the factual current screen, `current_activity` is what the user appears to be doing now, and `resume_work_target` or `return_target` is where the actionable workstream should resume.

## Current Product Shape

Smalltalk has two product lanes in this repo.

| Lane | Status | Purpose | Main files |
| --- | --- | --- | --- |
| Native desktop capture | Active | Capture screen/app evidence locally, inspect it, search it, prepare cloud-ready resume bundles, and generate resume cues. | `src/`, `src-tauri/src/capture.rs`, `src-tauri/scripts/` |
| Browser extension resume flow | Older but still present | Capture explicit browser research sessions and ask a local OpenAI proxy for a return-to-origin resume card. | `browser-extension/` |

The native lane should be treated as the main product unless we explicitly reopen the browser-extension lane.

The current native product primitive is `Continue`, not "session recording." Sessions, screenshots, timelines, native resume cards, and stop-time bundles are evidence and debug infrastructure. Continue is the layer that turns local evidence into a workstream, a factual current focus, an actionable return target, and an inspectable next step.

## What We Have Built So Far

- Root Tauri desktop app with app id `com.smalltalk.app`.
- React UI called `Session Capture` with:
  - `Start session`
  - `Stop session`
  - `Capture now`
  - `Delete all`
  - text search over captured evidence
  - evidence timeline
  - screenshot viewer
  - overlays for content units, OCR spans, Accessibility nodes, and privacy regions
  - frame verification panel
  - native `Resume me` card
  - cloud-ready stop bundle summary and `Open bundle JSON`
  - `Ask OpenAI` cloud resume call with provenance tracking
  - `Open return target` flow for resolved resume points
- Native Rust capture backend in `src-tauri/src/capture.rs`.
- Swift helpers for macOS Accessibility, OCR, window graph capture, and native event capture.
- Local SQLite store with FTS search, sessions, frames, OCR, Accessibility nodes, UI events, transitions, content units, privacy metadata, and export audit data.
- Stop-time cloud-ready resume query bundles under repo-root `resume_query_exports/`.
- Compact cloud bundles with selected safe keyframe images, episode cards, resume candidate metadata, quality flags, redactions, and missing-evidence notes.
- Clean-slate delete path that stops capture, clears frames/sessions/events/search rows, removes snapshots, and resets the runtime UI state.
- Safe AI export path for native evidence that redacts text/URLs/paths, masks sensitive images where possible, excludes `never_send_to_ai` frames, and writes an audit row.
- Native storyboard/resume-card builder based on local evidence.
- Optional cloud resume reasoner that reads a bounded resume-query bundle, preserves `response_id` when a real cloud call succeeds, and records whether the result came from `cloud` or `local_fallback`.
- Native Continue semantic memory in `src-tauri/src/continuation.rs` with artifacts, observations, task actions, episodes, workstreams, continuation candidates, decisions, feedback events, and breadcrumbs.
- Local Continue command that can run without Stop Session, rebuild derived layers, score local candidates, persist a decision, and return current focus separately from return/resume targets.
- Optional bounded OpenAI micro-inference for Continue that can only select among locally generated candidate ids and is locally validated before use.
- Continue feedback and eval support for measuring continuation decisions rather than prettier session summaries.

## Main User Flow

1. The user opens the Tauri app.
2. The user clicks `Start session`.
3. `start_capture` creates a row in `capture_sessions`, starts a background Rust worker, starts the native event source, and immediately captures a `session_start` frame.
4. While the session is running, native UI events schedule captures after short settle delays.
5. The user can click `Capture now` for an explicit manual frame.
6. The UI continuously refreshes status, search results, recent timeline, frame details, and screenshot previews.
7. The user can invoke `Continue` without stopping the session. The backend rebuilds local derived state, scores continuation candidates, and returns an actionable target with evidence anchors.
8. If the user clicks `Stop session`, `stop_capture` stops the worker, marks the session stopped, refreshes counts, builds a cloud-ready resume query bundle under `resume_query_exports/`, and returns the bundle summary.
9. The user can inspect the compact bundle, ask OpenAI from the Stop bundle path, open the resolved return target, or use local Continue/native resume cues inside the app.

## Runtime Storage

Native capture data is stored in the Tauri app data directory:

```text
~/Library/Application Support/com.smalltalk.app/capture/
  smalltalk-capture.sqlite
  snapshots/
  helpers/
  safe-ai-exports/
```

The UI gets the exact live paths from `capture_status`:

- `data_dir`
- `database_path`

The stop-time cloud bundles are different from the live app-data store. They are written to the project resume-query root:

```text
/Users/bhaskarpandit/Documents/smalltalk/resume_query_exports/session-041-resume-query-.../
  resume-query-bundle.json
  images/
```

## Capture Triggers

The background capture worker uses event-driven capture first and idle capture as fallback.

| Cause | Stored trigger | Source | Settle delay | Dedupe |
| --- | --- | --- | ---: | --- |
| Session starts | `session_start` | Rust worker | 0 ms | no |
| User clicks `Capture now` | `manual` | Tauri command | 0 ms | no |
| App changes | `app_switch` | Swift event helper | 300 ms | yes |
| Window focus changes | `window_focus` | Swift event helper | 300 ms | yes |
| Accessibility notification | `accessibility_change` | Swift event helper | 300 ms | yes |
| Mouse click | `click` | CGEvent tap | 220 ms | yes |
| Keyboard activity pauses | `typing_pause` | CGEvent tap | 850 ms | yes |
| Scroll settles | `scroll_stop` | CGEvent tap | 500 ms | yes |
| Clipboard changes | `clipboard` | pasteboard polling | 220 ms | yes |
| Nothing useful happened recently | `idle` | worker timer | 10 sec interval | yes |

Only one pending event bucket is kept. If multiple native events arrive before the capture fires, their event ids are merged into one trigger. If their trigger types differ, the stored trigger becomes `event_burst`.

Captures are also rate-limited by `MIN_CAPTURE_INTERVAL = 600 ms`.

## Native Capture Pipeline

Each stored native frame follows this pipeline:

1. Resolve capture paths under app-data.
2. Collect macOS Accessibility context.
3. Apply exclusion/privacy rules against app, bundle id, title, URL, path, and visible text.
4. If the privacy decision says `skip_capture`, do not store the frame.
5. Collect a window graph snapshot.
6. Capture a full-screen JPEG through `/usr/sbin/screencapture`.
7. Try to capture an active-window JPEG crop when a CoreGraphics window id exists.
8. Hash screenshot bytes into `image_hash`.
9. Decide whether Accessibility text is strong or thin.
10. Run OCR only when Accessibility text is missing or thin.
11. Resolve one `full_text` value and one `text_source`.
12. Compute `content_hash`.
13. Dedupe event/idle captures when both image and content match the prior capture.
14. Insert the frame and linked evidence rows into SQLite.
15. Insert content units, app contexts, sensitive regions, presence sample, frame diffs, trigger finalization, and transitions.
16. Emit `capture-frame` to the Tauri UI.

Manual captures bypass dedupe. Event and idle captures use dedupe.

## How Text Extraction Works

Smalltalk has two text sources: Accessibility and OCR. Accessibility is preferred because it provides semantic UI structure, not just pixels.

### Accessibility First

The primary path is the Swift helper:

```text
src-tauri/scripts/accessibility_snapshot.swift
```

Rust embeds and compiles it through `ensure_swift_helper`. The helper collects:

- frontmost app name
- app PID
- app bundle id
- focused window title
- window id
- browser URL when available
- document path from `AXDocument`
- selected text
- focused node
- Accessibility nodes
- node roles, labels, values, descriptions, bounds, actions, and depth

The Rust parser accepts helper output lines such as:

- `APP`
- `APP_PID`
- `APP_BUNDLE_ID`
- `WINDOW`
- `WINDOW_ID`
- `BROWSER_URL`
- `DOCUMENT`
- `NODE_JSON`
- `NODE`
- `ERROR`

If the Swift helper fails or returns weak signal, Smalltalk falls back to the embedded AppleScript `ACCESSIBILITY_SCRIPT` in `src-tauri/src/capture.rs`. The AppleScript path asks System Events for the frontmost app, front window, browser URL, document field, and a bounded Accessibility tree.

### When Accessibility Is Considered Thin

Accessibility is treated as thin when the current surface is likely canvas-heavy or browser-chrome-heavy. The current thinness heuristics include:

- app/window/URL patterns for Google Docs, Google Sheets, Google Slides, Figma, Excalidraw, Miro, Canva, and tldraw
- total Accessibility text under 100 characters
- less than 30 percent of Accessibility text coming from content-like roles after filtering toolbar/chrome roles

Thin Accessibility does not get discarded. It causes OCR to run and then combines with OCR.

### OCR Second

OCR runs only when Accessibility text is missing or thin.

Priority order:

1. Apple Vision OCR helper:

```text
src-tauri/scripts/vision_ocr.swift
```

2. `tesseract` on `PATH` as fallback.

On macOS, Apple Vision runs first. If Vision returns usable text, it wins. If Vision is empty or fails and Tesseract is installed, Tesseract is used.

OCR output is stored in two levels:

- `ocr_text`: one row per frame with joined text, raw OCR JSON, and OCR engine.
- `ocr_spans`: per-span text with engine, confidence, block/line/word indexes, pixel bounds, normalized bounds, and raw JSON.

### Text Source Resolution

After Accessibility and optional OCR complete, Rust calls `resolve_text`.

| Input state | Stored `text_source` | Stored `full_text` |
| --- | --- | --- |
| Accessibility text exists and is not thin | `accessibility` | Accessibility text |
| Accessibility exists, is thin, and OCR exists | `hybrid` | Accessibility text plus OCR text |
| Accessibility is missing and OCR exists | `ocr` | OCR text |
| Neither exists | `null` | `null` |

`full_text` is what goes into the FTS index and what powers native search.

## Screenshots And Images

The current screenshot provider is the macOS command-line tool:

```text
/usr/sbin/screencapture -x -t jpg
```

The frame stores:

- `snapshot_path`
- `full_screenshot_path`
- `active_window_crop_path`
- `active_element_crop_path`
- `image_hash`
- `phash`
- dimensions and scale

The active-window crop is attempted with:

```text
/usr/sbin/screencapture -x -t jpg -l <window_id>
```

The UI requests screenshot bytes through `get_frame_image_variant` and renders the full screenshot with overlay boxes.

## Window And App Context

The window graph helper is:

```text
src-tauri/scripts/window_snapshot.swift
```

It stores a window snapshot plus window rows in:

- `window_snapshots`
- `windows`

The app-context adapter converts raw app/window/URL evidence into rough product objects:

| Surface | Adapter/object type |
| --- | --- |
| ChatGPT or Claude in browser | `chat_conversation` |
| normal browser tab | `browser_tab` |
| Cursor, VS Code, Xcode, IntelliJ | `code_editor` |
| Terminal, iTerm, Warp | `terminal` |
| Preview/PDF | `pdf` |
| Finder | `finder` |
| Slack, Discord, Messages, WhatsApp | `messaging` |
| Notion, Linear, Notes | `notes_doc` |
| unknown app | `unknown` |

These rows live in `app_contexts` and are used by the UI verification panel and native resume-card logic.

## Content Units

Content units are product-facing evidence objects derived from Accessibility nodes and OCR spans.

They are stored in `content_units` with:

- source: `ax` or `ocr`
- unit type: `button`, `input`, `link`, `menu_item`, `table_cell`, `image`, `heading`, `paragraph`, or `unknown`
- text and text hash
- semantic role
- linked AX node id or OCR span ids
- bounds
- confidence
- raw JSON

Current semantic roles include:

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

Focused Accessibility nodes get higher confidence than generic nodes. OCR-only units are lower confidence.

## Events, Triggers, Transitions, And Diffs

Smalltalk records not just frames, but why frames happened.

### `ui_events`

Native events store app/window metadata, pointer location, scroll deltas, key categories, modifiers, repeat flags, and payload JSON.

Keyboard events do not store raw typed characters. They store categories like:

- `char`
- `enter`
- `backspace`
- `shortcut`
- `modifier`
- `escape`
- `arrow`

### `capture_triggers`

Triggers connect events to captures. They store:

- trigger id
- trigger type
- event ids that caused the capture
- settle delay
- dedupe policy
- pre-frame id
- post-frame id
- status
- errors

### `event_transitions`

Transitions summarize what happened between pre-frame and post-frame.

Current transition labels include:

- `switched_app`
- `scrolled_to_new_section`
- `entered_input`
- `copying_evidence`
- `same_screen_idle`
- `continuing_same_task`
- `new_task`
- `unknown`

The native classifier also has higher-level episode labels for storyboard/resume work, including:

- `returning_to_previous_task`
- `verification_branch`
- `possible_distraction`
- `background_media`

### `frame_diffs`

Frame diffs record simplified frame-to-frame changes such as same app/window, changed text hashes, diff type, confidence, and summary.

## SQLite Schema Overview

The live database is:

```text
smalltalk-capture.sqlite
```

Important tables:

| Table | Purpose |
| --- | --- |
| `capture_sessions` | Session id, sequence, start/stop timestamps, status, export path, counts |
| `frames` | Core frame rows, screenshot paths, app/window/URL/path, text, hashes, privacy status, trigger ids |
| `frames_fts` | FTS5 index over text and metadata |
| `ocr_text` | Per-frame OCR text and raw OCR JSON |
| `ocr_spans` | OCR spans with text, confidence, bounds, and raw JSON |
| `ax_nodes` | Accessibility nodes with role, text, bounds, focus, actions, and raw JSON |
| `content_units` | Normalized product-facing evidence units |
| `ui_events` | Native event stream |
| `capture_triggers` | Scheduled/captured/skipped trigger provenance |
| `event_transitions` | Transition summaries between frames |
| `window_snapshots` | Window graph snapshot metadata |
| `windows` | Individual windows in a window graph |
| `frame_diffs` | Simplified frame-to-frame diffs |
| `app_contexts` | Product-object adapters for apps/tabs/docs |
| `clipboard_events` | Clipboard metadata without full clipboard text |
| `typing_bursts` | Keyboard activity summaries without raw typed text |
| `presence_samples` | Basic activity samples |
| `exclusion_rules` | Privacy/exclusion rules |
| `sensitive_regions` | Detected sensitive regions and actions |
| `frame_quality_warnings` | Evidence-quality warnings |
| `ai_export_audit` | Safe AI export audit trail |
| `embeddings`, `episodes`, `episode_nodes`, `episode_edges` | Future/partial episode memory tables |

`frames_fts` is maintained by insert/delete triggers on `frames`.

## Search

The UI calls:

```text
search_captures(query, limit, sessionId)
```

If the query is empty, the backend returns latest frames for the active/latest session. If the query has terms, it builds an FTS query against:

- `full_text`
- `app_name`
- `window_name`
- `browser_url`
- `document_path`

Results include:

- full frame row
- snippet from SQLite FTS
- BM25 rank

## Native Continue Architecture

Native Continue is implemented in `src-tauri/src/continuation.rs` and exposed through Tauri commands in `src-tauri/src/capture.rs`. It is additive on top of the local capture store. It does not require Stop Session, does not read the browser extension store, and does not ask a model to summarize raw history.

The Continue pipeline has five layers:

1. **Evidence substrate**: native capture persists frames, app contexts, content units, UI events, triggers, transitions, frame diffs, typing burst metadata, clipboard metadata, privacy markers, and search indexes.
2. **Semantic memory layer**: `rebuild_continue_second_layer` resolves stable artifacts and observations, then extracts task actions from local evidence.
3. **Workstream layer**: `rebuild_continue_third_layer` groups actions into episodes, assigns artifact roles, clusters episodes into workstreams, records unresolved state, and marks active/suspended/background work.
4. **Local decision layer**: `get_continue_decision` generates continuation candidates, scores them, persists `continue_candidates`, and writes a `continue_decisions` row.
5. **Optional final layer**: bounded OpenAI micro-inference may choose among supplied candidate ids and phrase a cautious next action. The local validator remains the authority.

The schema is local SQLite. `ensure_continue_schema` creates or upgrades these tables:

- `continue_artifacts`
- `continue_artifact_observations`
- `continue_task_actions`
- `continue_task_action_events`
- `continue_episodes`
- `continue_episode_actions`
- `continue_episode_artifacts`
- `continue_workstreams`
- `continue_workstream_episodes`
- `continue_workstream_artifacts`
- `continue_candidates`
- `continue_decisions`
- `continue_feedback_events`
- `continue_breadcrumbs`

### Continue Artifacts

Artifacts are stable local work objects. They can represent browser tabs, chat conversations, code editors, terminal surfaces, PDFs, Finder views, messaging threads, notes/documents, or unknown surfaces.

The resolver prefers durable identity:

- canonical URL when safe and meaningful
- document path when available
- app/window identity when no stronger object exists
- stable hash-derived ids for deterministic local joins

Artifacts keep identity confidence, evidence quality, privacy status, openability, first/last seen frame ids, and timestamps. This lets Continue talk about a target artifact without collapsing it into a screenshot.

### Task Actions

Task actions are derived from local event and frame evidence. Current action kinds include:

- reading
- editing
- composing
- searching
- copying evidence
- reviewing output
- running command
- observing command output
- encountering error
- navigating
- switching context
- branching away
- returning to origin
- idle after progress
- messaging interrupt
- verification branch
- possible distraction
- unknown

Actions keep the evidence frame id, previous frame id, artifact ids, secondary artifact ids, event ids, confidence, and a local reason. Keyboard evidence is categorical and aggregate only; raw typed characters are not stored.

### Episodes And Workstreams

Episodes group adjacent task actions with boundary reasons such as idle after progress, interruption, branch, or return. Episode artifact roles include primary target, source evidence, branch support, output verification, blocker, interruption, current-focus-only, and unknown.

Workstreams cluster related episodes and durable artifacts. A workstream has:

- state: active, suspended, resumed, background, stale, or abandoned
- title candidate
- primary artifact id
- created and last-active timestamps
- confidence
- unresolved signal
- source

Unresolved signals are local JSON notes, not model inventions. Examples include an error needing resolution, copied evidence not yet applied, an unfinished search branch, or idle after progress.

### Local Continue Decision

The main command is:

```text
get_continue_decision
```

Default behavior is local only:

```ts
await invoke("get_continue_decision", {
  input: {
    lookback_ms: 2700000,
    rebuild_layers: true
  }
});
```

The request defaults to a 45 minute lookback, rebuilds the semantic/workstream layers, loads current focus, generates candidates, scores candidates, persists them, and returns a `ContinueDecisionResult`.

The result separates:

- `current_focus`: factual latest screen/artifact/frame
- `current_activity`: local read of what appears to be happening now
- `selected_workstream`: the workstream being continued
- `return_target`: target artifact to return to
- `resume_work_target`: actionable work target, kept separate from support/branch surfaces
- `candidate_kind`: why this candidate exists
- `last_meaningful_action`
- `unresolved_state`
- `next_action`
- `confidence` and `confidence_label`
- `evidence_anchors`: frame ids, action ids, episode ids, artifact ids
- `missing_evidence`
- `warnings`
- `validation_status`

Candidate scoring combines actionability, primary-target strength, unresolved state, branch/origin behavior, evidence quality, recency, openability, and privacy safety. Branch and support surfaces can be evidence without becoming the default return target.

### Bounded OpenAI Micro-Inference

OpenAI is optional for Continue. It is enabled only when the caller explicitly asks for it:

```ts
await invoke("get_continue_decision", {
  input: {
    lookback_ms: 2700000,
    rebuild_layers: true,
    micro_inference_enabled: true,
    max_candidates_for_model: 5
  }
});
```

The backend reads `OPENAI_API_KEY` from process environment or project `.env`. Model selection defaults to `gpt-4.1-mini` and can be overridden by request `model`, `SMALLTALK_CONTINUE_OPENAI_MODEL`, `SMALLTALK_OPENAI_MODEL`, or `OPENAI_MODEL`.

The request uses the OpenAI Responses API with Structured Outputs through a strict JSON schema. The model-facing pack is compact and candidate-bounded. It contains only:

- current focus facts
- top candidate workstreams
- top continuation candidates
- local candidate ids
- target artifact ids/kinds/titles
- URL/path availability booleans, not raw paths or raw URLs
- local score components
- last meaningful action summaries
- unresolved-state reasons
- evidence frame/action/episode ids
- missing evidence notes
- artifact role map
- short manual breadcrumbs

It does not include raw screenshots by default, raw timelines, raw database dumps, raw typed characters, full clipboard text, unredacted URLs, unredacted file paths, or frames excluded by privacy policy.

The Structured Output fields are:

- `selected_candidate_id`
- `selected_workstream_id`
- `intent_label`
- `next_action`
- `reason`
- `confidence`: `low`, `medium`, or `high`
- `uncertainty_notes`

### Continue Model Validation

The model result is never trusted directly. `validate_micro_inference_output` checks:

- selected candidate id exists in the supplied local pack
- selected workstream id matches the selected candidate
- selected candidate was actually sent to the model
- output does not contain unsupported URLs or paths
- `next_action` is present only when compatible with the candidate
- high confidence is rejected when local evidence is thin or missing
- branch/support targets cannot override the primary target without a strong local candidate

If the API call fails or validation fails, the decision is persisted with `source: "local_fallback"` and the local scorer result is returned. If validation succeeds, the decision source is `cloud_micro_inference`. Model name, `response_id`, validation notes, warnings, and validation status are stored on the decision row.

### Continue Feedback

Continue feedback is inferred locally after a decision. No UI prompt or nag is required.

The command is:

```text
infer_continue_feedback
```

Feedback event kinds:

- `accepted`: the user returned to the suggested target and stayed or acted there
- `rejected`: the user opened the target but quickly left with no meaningful action
- `ignored`: no target activity appeared inside the observation window
- `corrected`: the user chose another artifact shortly after Continue
- `auto_resumed`: the user naturally returned to the workstream without using the suggestion

`get_continue_decision` also opportunistically infers feedback for pending prior decisions before creating a new decision. Feedback rows reference decision ids, observed frame ids, target artifact ids, chosen artifact ids, timestamps, confidence, and local reasons.

### Breadcrumbs

Breadcrumbs are local-only prospective notes attached to a workstream:

```text
add_continue_breadcrumb
```

They are backend storage only for now, not a product UI. A breadcrumb is capped, local, and later included as short evidence in bounded candidate packs. It should be used for concise context, not raw secrets or large pasted text.

### Continue Eval Harness

The command is:

```text
run_continue_eval
```

It can run the built-in fixture set or a JSON fixture file. The default harness covers:

1. edit -> docs/search branch -> idle
2. work target -> Slack/messaging interruption -> idle
3. error -> search/docs -> no return
4. source copied -> target edited
5. source copied -> no target edit
6. AI chat as support
7. AI chat as primary
8. terminal verification after edit
9. terminal error as blocker
10. thin OCR-only evidence
11. nested dependent task

The report includes:

- target artifact correctness
- Recall@k
- MRR
- current-focus false-positive rate
- hallucinated artifact count
- model validation fallback rate
- per-case selected candidate, selected target, rank, validation status, and validation failures

The current default fixture metrics are:

- cases: 11
- target artifact correctness: 11/11
- Recall@k: 1.0
- MRR: 1.0
- current-focus false-positive rate: 0.0
- hallucinated artifact count: 0
- model validation fallback rate: 0.0

### Continue Opening

`open_resume_point` now accepts `continue_decision_id`. When present, it resolves the stored Continue decision target through `continue_decisions`, `continue_candidates`, and `continue_artifacts`, then uses the existing frame privacy and openability checks before opening anything. If the target is unsafe, missing, or not openable, the command falls back to local Smalltalk evidence instead of inventing a route.

## Resume-Query Bundle

`Stop session` writes only the compact model-facing bundle:

```text
resume_query_exports/session-041-resume-query-.../
  resume-query-bundle.json
  images/
    frame-000269-resume-candidate.jpg
    frame-000251-origin.jpg
```

Bundle details:

- `resume-query-bundle.json` is the payload intended for cloud resume inference or local inspection.
- The current schema is `smalltalk.resume_query.v2`.
- It includes session timing, a compact session index, current activity, current focus, resume work target, return target, candidate anchors, candidate episodes, the chosen resume candidate, selected keyframes, transition labels, privacy metadata, quality flags, and missing-evidence notes.
- Images are capped at 12 selected cloud-safe keyframes and copied only for those frames.
- Raw table dumps, SQLite snapshots, full per-frame folders, PNG duplicates, and repo-root `output/session-*` folders are no longer produced by the Stop path.

The old exhaustive `output/` folder is a legacy/debug artifact. Runtime Stop behavior should now preserve only the compact data that is ready for cloud use.

## Ask OpenAI And Open Return Target

The native UI can ask OpenAI after a resume-query bundle exists. The Tauri command is:

```text
run_cloud_resume
```

This command builds or reuses a bounded resume-query bundle, sends the compact evidence to the OpenAI Responses API when credentials are available, parses strict JSON, enforces local anchor contracts, persists a result file, and returns a `smalltalk.cloud_resume_result.v1` object to the UI.

Cloud resume results explicitly track provenance:

- `source: "cloud"` means a real model response was parsed and persisted.
- `source: "local_fallback"` means the app produced a conservative local fallback instead of a real cloud result.
- Cached cloud results are only reused when they match the current request and include cloud provenance rather than an older fallback.
- Real cloud responses preserve request metadata such as `response_id` when available.

The cloud reasoner must keep these concepts separate:

- `current_focus`: the factual current screen.
- `current_activity`: what the user appears to be doing now.
- `resume_work_target`: the actionable workstream target.
- `return_target`: the target to open when returning to previous work.
- `resume_target`: compatibility alias for `resume_work_target`.

Opening a resume point is handled by:

```text
open_resume_point
```

It resolves the best target from the cloud result, local native card, explicit frame id, or local resume-query candidate, then opens the underlying URL, document path, or local frame evidence when possible. It also carries warnings when it has to fall back from a cloud target to local evidence.

## Native Resume Card

The native `Resume me` button calls:

```text
get_native_resume_card({
  lookback_minutes: 20,
  current_frame_id,
  max_keyframes: 10
})
```

The native resume card path does not require OpenAI. It builds a safe local export, selects keyframes, classifies transitions, and returns a conservative `NativeResumeCard`.

The card includes:

- what the user was doing
- what they were reading, when available
- the current focus cue
- why that cue was selected
- a `continue_from` frame with app/window/title/URL/path/quote
- what changed
- useful evidence frame ids
- likely distractions
- behavior read with confidence
- next action
- warnings

The selection logic prefers:

- current frame
- earliest frame in lookback
- last app/window/surface switch
- substantial readable text
- typing, clipboard, scroll, and return-to-previous-surface transitions
- recent non-duplicate evidence

The output is intentionally cautious. It should say the evidence is thin rather than inventing intent.

## Safe AI Export Path

The native backend has a safe export path used by storyboard/resume tooling:

```text
build_safe_ai_export
get_native_storyboard_dossier
classify_episode_transitions
get_native_resume_card
export_debug_episode
get_episode_dossier
```

Safe export behavior:

- builds a lookback window around the current frame or current time
- caps max frames
- excludes frames with `skip_capture`, `never_send_to_ai`, or equivalent sensitive actions
- redacts common secrets, emails, phone-like values, token-like strings, URLs, and file paths before export
- copies or masks images into `safe-ai-exports/<export-id>/images`
- writes `safe-ai-export.json`
- records an `ai_export_audit` row

This path prepares evidence for downstream AI use, but the current native resume card is local heuristic logic.

## Privacy Boundaries

Current native boundaries:

- Raw screenshots, Accessibility text, OCR text, and SQLite data stay local under app data.
- Keyboard events store categories and counts, not raw typed characters.
- Clipboard events store metadata, hash, byte size, and redacted preview, not full clipboard text.
- Password-manager bundles are skipped by default rules.
- Sign-in/auth-like titles and sensitive URLs are marked redacted.
- API-key/token/secret-like content is marked `never_send_to_ai`.
- Raw local screenshots can still contain sensitive visible content if the surface is not skipped or redacted, so the live capture store should be treated as sensitive.

Important current limitation:

- `store_redacted` records privacy metadata and sensitive regions. The live stored screenshot is not always pixel-masked at capture time. Pixel masking is applied in the safe export path when those images are exported for AI-facing use.

## UI Trust Model

The app should not ask the user to trust a summary blindly. The UI exposes evidence quality:

- screenshot present
- AX present and node count
- OCR present and span count
- content units present and count
- app context present
- window graph present
- transition present
- event provenance present
- sensitive regions present
- missing signals
- trust label and trust score

The inspector overlays the screenshot with:

- content units
- OCR spans
- AX nodes
- privacy regions

The evidence drawer shows:

- extracted text
- raw events
- app/window context
- stored paths

## Browser Extension Lane

The older browser extension is still useful as prior product context. It captures explicit browser research sessions and focuses on returning the user to the original page/thread after a detour.

Key extension pieces:

- WXT MV3 extension in `browser-extension/`
- popup-first `Start research`, `Stop`, and `Resume me`
- content-script page snapshots
- readable text chunks
- attention events: snapshot, scroll, selection, cursor dwell, link click, visibility
- navigation graph from Chrome webNavigation events
- local extension storage for sessions, visits, events, edges, chunks, cards
- sanitized `ResumeDossier`
- localhost proxy at `http://localhost:8787/api/resume`
- OpenAI Responses API call from `browser-extension/server/inference-proxy.ts`
- strict resume-card schema

Important product rule from that lane: branch pages are evidence, not default resume targets. The best resume target should point back to the origin page when a safe origin anchor exists. If no safe anchor exists, `resumeTarget: null` is acceptable.

## What Smalltalk Is Not Capturing Today

The native app does not currently capture:

- audio
- microphone input
- meeting transcripts
- speaker diarization
- continuous video
- raw typed characters
- full clipboard text as a clipboard event
- browser DOM from the native path
- cloud embeddings
- unbounded remote AI summaries over the raw native capture store
- raw Continue candidate packs with unredacted paths or URLs

## Runtime Commands

The Tauri command surface exposed in `src-tauri/src/lib.rs` currently includes:

- `start_capture`
- `stop_capture`
- `capture_once`
- `capture_status`
- `delete_all_frames`
- `search_captures`
- `get_frame`
- `get_frame_image`
- `get_frame_image_variant`
- `build_resume_query_bundle`
- `run_cloud_resume`
- `get_cloud_resume_status`
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
- `open_resume_point`
- `start_native_capture`
- `stop_native_capture`
- `capture_once_v2`
- `get_frame_v2`
- `get_recent_timeline`
- `get_frame_detail`
- `validate_frame_consistency`
- `get_transition`
- `search_content_units`
- `build_safe_ai_export`
- `get_native_storyboard_dossier`
- `classify_episode_transitions`
- `get_native_resume_card`
- `run_resume_eval`
- `export_debug_episode`
- `get_episode_dossier`
- `add_exclusion_rule`
- `remove_exclusion_rule`
- `list_exclusion_rules`
- `delete_recent_captures`

The `*_v2` capture names currently route to the same native implementation.

## Development Notes

Run the desktop app with:

```text
npm run tauri dev
```

The observed dev flow starts Vite on port `1420`, watches `src-tauri`, and launches the Tauri app.

If port `1420` is busy, stale `vite` or `target/debug/smalltalk` processes may still be running from an earlier dev session.

## Product Direction

The sharp wedge is not "record everything" and not "generic AI memory." The wedge is:

> Capture enough local evidence to answer where the user should restart, and make every answer inspectable.

The next product work should keep improving:

- evidence quality per frame
- origin/branch/return reasoning
- visual keyframe selection
- privacy-safe export
- readable resume cards
- confidence and missing-evidence reporting
- local-first trust
