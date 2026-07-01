# Smalltalk Product And Technical Snapshot

Last updated: 2026-06-25

Smalltalk is currently a desktop-first, local screen-memory app built with Tauri, Rust, React, and SQLite. The older Chrome extension still exists in `browser-extension/`, but the active product direction is the native app in the repo root.

The product goal is simple: when a user stops working, detours, or comes back later, Smalltalk should have enough local evidence to explain what they were doing, what changed, what text or UI mattered, and where they should resume. It should be inspectable before it is intelligent. Every resume cue should point back to frames, events, text sources, app/window context, and exportable evidence.

## Current Product Shape

Smalltalk has two product lanes in this repo.

| Lane | Status | Purpose | Main files |
| --- | --- | --- | --- |
| Native desktop capture | Active | Capture screen/app evidence locally, inspect it, search it, prepare cloud-ready resume bundles, and generate resume cues. | `src/`, `src-tauri/src/capture.rs`, `src-tauri/scripts/` |
| Browser extension resume flow | Older but still present | Capture explicit browser research sessions and ask a local OpenAI proxy for a return-to-origin resume card. | `browser-extension/` |

The native lane should be treated as the main product unless we explicitly reopen the browser-extension lane.

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
- Native Rust capture backend in `src-tauri/src/capture.rs`.
- Swift helpers for macOS Accessibility, OCR, window graph capture, and native event capture.
- Local SQLite store with FTS search, sessions, frames, OCR, Accessibility nodes, UI events, transitions, content units, privacy metadata, and export audit data.
- Stop-time cloud-ready resume query bundles under repo-root `resume_query_exports/`.
- Compact cloud bundles with selected safe keyframe images, episode cards, resume candidate metadata, quality flags, redactions, and missing-evidence notes.
- Clean-slate delete path that stops capture, clears frames/sessions/events/search rows, removes snapshots, and resets the runtime UI state.
- Safe AI export path for native evidence that redacts text/URLs/paths, masks sensitive images where possible, excludes `never_send_to_ai` frames, and writes an audit row.
- Native storyboard/resume-card builder based on local evidence, not a remote model call in the native path.

## Main User Flow

1. The user opens the Tauri app.
2. The user clicks `Start session`.
3. `start_capture` creates a row in `capture_sessions`, starts a background Rust worker, starts the native event source, and immediately captures a `session_start` frame.
4. While the session is running, native UI events schedule captures after short settle delays.
5. The user can click `Capture now` for an explicit manual frame.
6. The UI continuously refreshes status, search results, recent timeline, frame details, and screenshot previews.
7. The user clicks `Stop session`.
8. `stop_capture` stops the worker, marks the session stopped, refreshes counts, builds a cloud-ready resume query bundle under `resume_query_exports/`, and returns the bundle summary.
9. The user can inspect the compact bundle, ask OpenAI from the app, or use the native `Resume me` cue inside the app.

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

## Cloud-Ready Stop Bundle

`Stop session` writes only the compact model-facing bundle:

```text
resume_query_exports/session-041-resume-query-.../
  resume-query-bundle.json
  images/
    frame-000269-resume-candidate.jpg
    frame-000251-origin.jpg
```

Bundle details:

- `resume-query-bundle.json` is the payload intended for cloud resume inference.
- It includes session timing, a compact session index, candidate episodes, the chosen resume candidate, selected keyframes, transition labels, privacy metadata, quality flags, and missing-evidence notes.
- Images are capped at 12 selected cloud-safe keyframes and copied only for those frames.
- Raw table dumps, SQLite snapshots, full per-frame folders, PNG duplicates, and repo-root `output/session-*` folders are no longer produced by the Stop path.

The old exhaustive `output/` folder is a legacy/debug artifact. Runtime Stop behavior should now preserve only the compact data that is ready for cloud use.

## Native Resume Card

The native `Resume me` button calls:

```text
get_native_resume_card({
  lookback_minutes: 20,
  current_frame_id,
  max_keyframes: 10
})
```

The native path does not call OpenAI right now. It builds a safe local export, selects keyframes, classifies transitions, and returns a conservative `NativeResumeCard`.

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
- remote AI summaries in the native capture path

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
