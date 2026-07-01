# Smalltalk Capture Technical Details

Last updated: 2026-06-25

This document explains what the current Smalltalk desktop app captures, how capture is triggered, how each capture is processed, what is stored locally, what is exported for resume inference, and what is intentionally not captured.

The current active product lane is the native Tauri desktop app in the repo root. The older browser-extension flow still exists under `browser-extension/`, but the capture system described here is the native app implemented primarily in `src-tauri/src/capture.rs` with macOS helpers in `src-tauri/scripts/`.

## Short Version

Smalltalk captures sparse, event-driven evidence about the user's current work surface:

- Full-screen screenshots.
- Active-window screenshot crops when a window id is available.
- Foreground app, bundle id, process id, window title, browser URL, and document path.
- macOS Accessibility tree text and node metadata.
- OCR text and OCR span boxes when Accessibility is missing or thin.
- Native UI events such as app switches, clicks, scrolls, key-down categories, Accessibility changes, and clipboard changes.
- Derived triggers, transitions, content units, app-context classifications, frame diffs, privacy markers, and resume-query bundles.

Smalltalk does not capture audio, microphone input, meetings, speaker identity, continuous video, raw typed characters, or raw clipboard text.

## User Flow

1. The user starts a session from the app UI.
2. `start_capture` creates a `capture_sessions` row, starts the Rust capture worker, starts the native event helper, shows the Session Island, and immediately captures a `session_start` frame.
3. While the session is running, native macOS events are stored in `ui_events` and coalesced into capture triggers.
4. Each trigger waits a short settle delay before capture so the UI can finish changing.
5. The user can force an immediate `manual` capture with `capture_once`.
6. The capture worker uses `idle` capture as a fallback every 10 seconds when no useful event capture has happened recently.
7. On stop, `stop_capture` stops the worker, marks the session stopped, and builds a compact resume-query bundle under `resume_query_exports/`.

## Runtime Storage

Live native capture data is stored in the Tauri app-data directory:

```text
~/Library/Application Support/com.smalltalk.app/capture/
  smalltalk-capture.sqlite
  snapshots/
  helpers/
  safe-ai-exports/
```

The UI should trust `capture_status.data_dir` and `capture_status.database_path` for the exact runtime paths on the current machine.

Stop-time resume-query bundles are written separately under the repo:

```text
/Users/bhaskarpandit/Documents/smalltalk/resume_query_exports/
  session-XXX-resume-query-<id>/
    resume-query-bundle.json
    images/
```

The current `stop_capture` path does not write the older exhaustive `output/session-001` folder export. It returns a compact `smalltalk.resume_query.v1` bundle result.

## Capture Triggers

Smalltalk is event-driven first and idle-driven second.

| Cause | Stored trigger | Source | Settle delay | Dedupe |
| --- | --- | --- | ---: | --- |
| Session starts | `session_start` | Rust worker | 0 ms | no |
| User clicks Capture now | `manual` | Tauri command | 0 ms | no |
| App changes | `app_switch` | `NSWorkspace.didActivateApplicationNotification` | 300 ms | yes |
| Focused window changes | `window_focus` | Swift helper event | 300 ms | yes |
| Accessibility notification | `accessibility_change` | `AXObserver` notifications, normalized from `ax_notification` | 300 ms | yes |
| Mouse click | `click` | listen-only `CGEvent` tap | 220 ms | yes |
| Keyboard input pauses | `typing_pause` | listen-only `CGEvent` key-down category | 850 ms | yes |
| Scroll activity settles | `scroll_stop` | listen-only `CGEvent` scroll-wheel event | 500 ms | yes |
| Clipboard changes | `clipboard` | `NSPasteboard.general.changeCount` polling | 220 ms | yes |
| No recent stored event frame | `idle` | capture worker timer | 10 sec interval | yes |

Only one pending event bucket is kept at a time. If several events arrive before the settle delay expires, their event ids are merged into one `capture_triggers.caused_by_event_ids` JSON array. If the merged events have different trigger types, the trigger becomes `event_burst`.

Captures are also rate-limited by `MIN_CAPTURE_INTERVAL = 600 ms`.

## Native Event Helper

The event helper is `src-tauri/scripts/capture_events.swift`. Rust compiles and runs it from the app-data `helpers/` directory.

It emits newline-delimited JSON events containing:

- Timestamp in milliseconds.
- Event type.
- Frontmost app process id, bundle id, app name, and focused window title.
- Pointer coordinates and button for clicks.
- Scroll deltas for scroll events.
- Keyboard category, modifier flags, and repeat status for key-down events.
- Clipboard metadata for clipboard changes.
- Additional JSON payload for helper-specific details.

Keyboard events store categories, not typed text. Categories include `char`, `enter`, `backspace`, `shortcut`, `modifier`, `escape`, and `arrow`.

Clipboard events store metadata. For text clipboard data, the helper stores a hash, redacted preview, byte size, and pasteboard types. It does not store the raw clipboard text.

## Capture Pipeline

Each stored frame goes through this pipeline:

1. Resolve app-data paths and ensure the SQLite database exists.
2. Create a day-bucketed screenshot directory under `snapshots/`.
3. Collect Accessibility context from the frontmost app.
4. Build a semantic fingerprint from app/window/URL/document/text.
5. Apply privacy and exclusion rules.
6. If privacy says `skip_capture`, return without storing a frame.
7. Collect a window graph snapshot.
8. Capture a full-screen JPEG screenshot using macOS `screencapture`.
9. Capture an active-window JPEG crop if a CoreGraphics window id is available.
10. Hash the full screenshot bytes into `image_hash`.
11. Determine image dimensions from the JPEG bytes.
12. Decide whether Accessibility text is strong or thin.
13. Run OCR only when Accessibility text is missing or thin.
14. Resolve one `text_source` and one `full_text`.
15. Compute `content_hash` from `full_text`.
16. For event and idle captures, dedupe against the previous image and text hashes.
17. Insert the frame row into `frames`.
18. Persist related OCR, Accessibility, window, content-unit, app-context, sensitive-region, presence, and frame-diff rows.
19. Validate frame consistency and record warnings if needed.
20. Finalize the trigger, create an `event_transitions` row, and emit `capture-frame` to the UI.

Manual and session-start captures bypass dedupe. Event and idle captures use dedupe.

## Screenshot Capture

The preferred screenshot provider is ScreenCaptureKit one-shot capture:

```text
ScreenCaptureKit SCScreenshotManager display capture
```

The full screenshot is saved under:

```text
<app_data_dir>/capture/snapshots/day-<bucket>/<timestamp>_full.jpg
```

When a window id is known, Smalltalk also attempts:

```text
ScreenCaptureKit SCScreenshotManager active-window capture
```

That active-window crop is saved as:

```text
<timestamp>_window.jpg
```

The frame records screenshot paths, image hash, perceptual-hash placeholder value, dimensions, scope, provider, display id, window id, app pid, and bundle id.

Current `capture_provider` is `screen_capture_kit` when the one-shot SCK helper succeeds. The legacy `/usr/sbin/screencapture` path remains a fallback and records `screencapture_cli`.

Current `scope` is:

- `active_window` when an active window id exists.
- `active_display` otherwise.

## Accessibility Capture

Accessibility is the primary semantic source because it can capture UI structure and text without relying only on pixels.

The primary helper is:

```text
src-tauri/scripts/accessibility_snapshot.swift
```

The fallback is the embedded AppleScript `ACCESSIBILITY_SCRIPT` in `src-tauri/src/capture.rs`.

Accessibility capture can collect:

- Frontmost app name.
- App process id.
- App bundle id.
- Focused window title.
- Window id.
- Browser URL when available.
- Document path from `AXDocument`.
- Selected text.
- Focused node.
- Accessibility nodes.
- Node role, subrole, role description, title, value, description, help, identifier, document, URL, selected text, character ranges, focus/enabled/selected flags, bounds, actions, child count, depth, and raw node JSON.

The helper output is parsed from records such as:

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

If the Swift helper returns useful signal, it is used. If it fails or returns no useful signal, Smalltalk falls back to AppleScript.

## Thin Accessibility And OCR

OCR is secondary. It runs only when Accessibility text is absent or considered thin.

Accessibility is considered thin when:

- The current URL/window looks like a canvas-heavy surface such as Google Docs, Google Sheets, Google Slides, Figma, Excalidraw, Miro, Canva, or tldraw.
- Total Accessibility text is under 100 characters.
- Browser-chrome-like roles dominate and content-like text is sparse.
- Content-like text is less than 30 percent of the total node text.

OCR priority:

1. Apple Vision OCR through `src-tauri/scripts/vision_ocr.swift`.
2. `tesseract` on `PATH` if Vision is empty, fails, or is unavailable.

OCR output is stored in:

- `ocr_text`: one row per frame with joined OCR text, raw OCR JSON, and engine.
- `ocr_spans`: one row per OCR span with text, confidence, indexes, pixel bounds, normalized bounds, and raw JSON.

## Text Source Resolution

Smalltalk resolves one text source per frame.

| Inputs | `text_source` | `full_text` |
| --- | --- | --- |
| Strong Accessibility text | `accessibility` | Accessibility text |
| Thin Accessibility text plus OCR | `hybrid` | Accessibility text plus OCR text |
| No Accessibility text plus OCR | `ocr` | OCR text |
| Neither source has text | `null` | `null`, with diagnostic errors attached for UI display when possible |

`full_text` is indexed by FTS and used by search, resume selection, evidence strength scoring, and AI-safe export.

## Deduplication

Event and idle captures are skipped when they repeat the previous frame.

The current dedupe check compares:

- `image_hash`, derived from full screenshot bytes.
- `content_hash`, derived from resolved `full_text` when text exists.

When a capture is deduped, Smalltalk deletes the just-created screenshot and active-window crop, finalizes the trigger as `skipped`, increments skipped sample state, and does not insert a frame.

Manual captures and session-start captures bypass dedupe.

## Privacy And Exclusion Rules

Privacy decisions run before screenshots are stored.

Rules live in `exclusion_rules` and have:

- `rule_type`
- `pattern`
- `action`
- `enabled`

The default rules include:

| Rule id | Match area | Pattern | Action |
| --- | --- | --- | --- |
| `default-private-apps` | app bundle/name | `1password|bitwarden|keychain|authenticator` | `skip_capture` |
| `default-private-auth` | window title | `password|passcode|verification code|authentication|sign in` | `store_redacted` |
| `default-sensitive-sites` | URL | `bank|checkout|payment|health|medical|1password|bitwarden` | `store_redacted` |
| `default-api-secrets` | content | `sk-...`, `api_key`, `token`, `secret` style patterns | `never_send_to_ai` |

Privacy statuses include:

- `normal`
- `redacted`
- `skipped_sensitive`

Privacy regions are stored in `sensitive_regions` with region type, optional bounds, source, confidence, action taken, and metadata JSON.

If a rule says `skip_capture`, no frame is stored. If a rule says `store_redacted` or `never_send_to_ai`, the frame can be stored locally with privacy metadata, but safe export and resume-query code exclude or redact it before model use.

## SQLite Tables

The core database file is:

```text
smalltalk-capture.sqlite
```

### `capture_sessions`

Stores session boundaries and counts:

- `id`
- `sequence`
- `started_at_ms`
- `stopped_at_ms`
- `status`
- `export_path`
- `frame_count`
- `event_count`
- `transition_count`
- `content_unit_count`
- `summary_json`
- `created_at_ms`

### `frames`

Stores the core frame:

- `id`
- `captured_at`
- `created_at`
- `session_id`
- `snapshot_path`
- `full_screenshot_path`
- `active_window_crop_path`
- `active_element_crop_path`
- `app_name`
- `window_name`
- `browser_url`
- `document_path`
- `focused`
- `capture_trigger`
- `capture_trigger_id`
- `previous_frame_id`
- `text_source`
- `accessibility_text`
- `accessibility_tree_json`
- `full_text`
- `content_hash`
- `image_hash`
- `phash`
- `capture_provider`
- `scope`
- `display_id`
- `window_id`
- `app_pid`
- `app_bundle_id`
- `screen_scale`
- `pixel_width`
- `pixel_height`
- `privacy_status`
- `sck_display_id`
- `sck_window_id`
- `sck_owning_bundle_id`
- `sck_filter_summary_json`
- `sck_configuration_summary_json`
- `sck_frame_metadata_json`
- `sck_capture_mode`
- `sck_audio_policy`

### `frames_fts`

FTS5 index over:

- `full_text`
- `app_name`
- `window_name`
- `browser_url`
- `document_path`

Search uses `frames_fts MATCH` and returns recent frame rows with snippets and rank.

### `ui_events`

Stores native events emitted by the helper:

- `id`
- `session_id`
- `ts_ms`
- `event_type`
- `app_pid`
- `app_bundle_id`
- `app_name`
- `window_id`
- `window_title`
- `x`
- `y`
- `button`
- `scroll_dx`
- `scroll_dy`
- `key_category`
- `modifier_flags`
- `is_repeat`
- `payload_json`
- `created_at_ms`

### `capture_triggers`

Stores the decision to capture:

- `id`
- `session_id`
- `ts_ms`
- `trigger_type`
- `caused_by_event_ids`
- `settle_delay_ms`
- `rate_limited`
- `dedupe_policy`
- `pre_frame_id`
- `post_frame_id`
- `status`
- `error`

`dedupe_policy` is currently `manual_bypass`, `event_bucket`, or `layered`.

### `event_transitions`

Links a trigger to before/after frames:

- `id`
- `session_id`
- `trigger_id`
- `primary_event_id`
- `pre_frame_id`
- `post_frame_id`
- `ts_start_ms`
- `ts_end_ms`
- `transition_type`
- `confidence`
- `summary`
- `changed_region_json`

Current transition labels include:

- `switched_app`
- `scrolled_to_new_section`
- `entered_input`
- `copying_evidence`
- `same_screen_idle`
- `continuing_same_task`
- `new_task`
- `unknown`

### `window_snapshots` And `windows`

The window helper is:

```text
src-tauri/scripts/window_snapshot.swift
```

`window_snapshots` stores active window id, active app pid, active bundle id, screen count, and raw window graph JSON.

`windows` stores per-window details:

- CoreGraphics window id.
- Owner pid/name.
- Bundle id.
- Window title.
- Layer and alpha.
- Onscreen and active flags.
- Bounds.
- Workspace.
- Raw JSON.

### `ax_nodes`

Stores Accessibility nodes with:

- Node id and parent id.
- App pid and window id.
- Role, subrole, role description.
- Title, value, description, help, identifier.
- Document and URL.
- Selected text.
- Selected-text and visible-character ranges.
- Character count.
- Focused, enabled, selected flags.
- Bounds.
- Actions.
- Children count.
- Depth.
- Raw node JSON.

### `ocr_text` And `ocr_spans`

`ocr_text` stores frame-level OCR text, raw OCR JSON, and OCR engine.

`ocr_spans` stores per-span OCR evidence:

- Engine.
- Text.
- Confidence.
- Language when present.
- Block, line, and word indexes.
- Pixel bounds.
- Normalized bounds JSON.
- Raw JSON.

### `content_units`

Content units are product-facing evidence objects derived from Accessibility nodes and OCR spans.

Fields include:

- `id`
- `frame_id`
- `window_id`
- `source`: `ax` or `ocr`
- `unit_type`: `button`, `input`, `link`, `menu_item`, `table_cell`, `image`, `heading`, `paragraph`, or `unknown`
- `text`
- `text_hash`
- `semantic_role`
- `ax_node_id`
- `ocr_span_ids`
- `adapter_object_id`
- Bounds.
- `crop_path`
- `visible_ratio`
- `center_distance`
- `confidence`
- `created_at_ms`
- `raw_json`

Current semantic roles include examples such as:

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

### `app_contexts`

App contexts classify the current surface into a product object.

Common mappings:

| Surface | Adapter | Object type |
| --- | --- | --- |
| ChatGPT, Claude, Perplexity URL | `ai_chat_url_adapter` | `chat_conversation` |
| Notion URL | `notion_browser_adapter` | `notes_doc` |
| Linear URL | `linear_browser_adapter` | `notes_doc` |
| YouTube, Spotify, media URLs | `media_browser_adapter` | `media` |
| PDF URL | `pdf_browser_adapter` | `pdf` |
| Browser app | `browser_adapter` | `browser_tab` |
| Native ChatGPT, Claude, Perplexity | `native_chat_adapter` | `chat_conversation` |
| Cursor, VS Code, Xcode, IntelliJ | `ide_adapter` | `code_editor` |
| Terminal, iTerm, Warp | `terminal_adapter` | `terminal` |
| Preview/PDF app | `pdf_adapter` | `pdf` |
| Finder | `finder_adapter` | `finder` |
| Slack, Discord, Messages, WhatsApp | `messaging_adapter` | `messaging` |
| Notion, Linear, Notes app | `notes_task_adapter` | `notes_doc` |
| Unknown | `generic_ax_adapter` | `unknown` |

### `clipboard_events`

Stores clipboard metadata:

- `id`
- `session_id`
- `ts_ms`
- `change_count`
- `content_type`
- `text_hash`
- `redacted_preview`
- `byte_size`
- `source_frame_id`
- `source_content_unit_id`
- `target_frame_id`
- `pasted_within_ms`
- `metadata_json`

Raw clipboard text is not stored.

### `typing_bursts`

Summarizes typing without raw text:

- Start/end timestamps.
- App/window identity.
- Character, backspace, enter, paste, and shortcut counts.
- Commit signal.
- `raw_text_captured`, currently false.
- Optional text hash and redacted preview fields.
- Pre/post frame ids.

### `presence_samples`

Stores coarse presence state:

- Timestamp.
- Idle seconds.
- Display-asleep flag.
- Screen-locked flag.
- Recent input and cursor movement flags.
- Frontmost app bundle id.

Current persisted samples are simple: display asleep and screen locked are stored as false, recent input and cursor movement as true, plus the frontmost bundle id.

### `frame_diffs`

Stores differences between consecutive frames:

- From/to frame ids.
- Same app and same window flags.
- Visual pHash distance placeholder.
- Changed-region JSON.
- Added, removed, and stable text hashes.
- AX/OCR delta JSON placeholders.
- Diff type.
- Confidence.
- Summary.

Current diff type is mainly trigger-derived: scroll, app switch, typing, same-screen idle, or unknown.

### `sensitive_regions`

Stores privacy findings:

- Region type.
- Optional bounds.
- Source.
- Confidence.
- Action taken.
- Metadata JSON.

### `frame_quality_warnings`

Stores trust warnings generated by frame consistency validation:

- Warning type.
- Severity.
- Message.
- Evidence JSON.

These warnings feed the inspector and lower safe-export evidence confidence.

### `ai_export_audit`

Stores safe-export audit records:

- Export type.
- Lookback range.
- Input/exported/excluded frame counts.
- Masked image count.
- Redacted text count.
- Warnings JSON.

### `episodes`, `episode_nodes`, `episode_edges`, `episode_evidence`

These tables support episode/resume indexing and derived evidence graphs. Resume-query generation can persist episode cards with metadata that identify work segments, likely branches, artifacts, and resume candidates.

## Search And Inspection

The UI can call:

- `capture_status` for counts, latest frame, session state, tool availability, data path, and DB path.
- `search_captures` for FTS search over frames.
- `get_recent_timeline` for events, triggers, transitions, and frames.
- `get_frame_detail` for frame plus verification signals, events, AX nodes, OCR spans, content units, app contexts, sensitive regions, windows, and transitions.
- `validate_frame_consistency` for frame quality warnings.
- `search_content_units` for structured content-unit search.

Frame detail includes verification signals such as screenshot presence, AX presence, OCR presence, content-unit presence, app-context presence, window-graph presence, transition presence, event provenance, sensitive-region presence, counts, missing signals, trust label, and trust score.

## Safe AI Export

`build_safe_ai_export` creates a bounded, privacy-filtered local bundle under:

```text
<app_data_dir>/capture/safe-ai-exports/<ai-export-id>/
  safe-ai-export.json
  images/
```

The safe export:

- Scopes frames by session, current frame, lookback minutes, or explicit range.
- Caps frame count.
- Excludes frames marked by privacy rules as unsafe.
- Redacts text, URLs, paths, emails, secret-like strings, and sensitive fields.
- Copies or masks screenshots before export.
- Emits compact content units instead of dumping all raw frame text.
- Includes transitions and warnings.
- Writes an audit row to `ai_export_audit`.

Safe frames include:

- Frame id and timestamp.
- App/window metadata.
- Redacted browser URL/document path.
- Safe image paths.
- Compact top content units.
- Text source and redacted text.
- Evidence strength.
- Privacy status.
- Warnings.

## Resume-Query Bundle

On stop, Smalltalk builds a `smalltalk.resume_query.v1` bundle under `resume_query_exports/`.

The resume-query bundle is designed for a cloud model or local inspection. It is compact and bounded, not a full raw database dump.

It contains:

- Budget limits.
- Session id, start, stop, and duration.
- Session index.
- Candidate episode cards.
- Resume candidate frame.
- Diagnostic artifacts.
- Branch evidence.
- Dropped frame summary.
- Timeline summary.
- Evidence conflicts.
- Confidence breakdown.
- Keyframes.
- Transitions.
- Privacy contract.
- Quality flags.
- Missing evidence.
- Ask object with `task: infer_resume_target`.

The privacy contract explicitly says:

- `raw_urls_sent: false`
- `raw_paths_sent: false`
- `raw_clipboard_sent: false`
- `raw_keystrokes_sent: false`

The bundle may include selected safe keyframe images in an `images/` folder, copied from the active-window crop when available or full screenshot otherwise. Model-facing images are capped at 12 selected keyframes; raw local screenshots and native capture volume stay in the app-data store.

## Resume Target Selection

Resume-query selection prefers frames with useful work evidence, not export/debug artifacts or browser chrome.

Signals used include:

- Evidence strength.
- Actionable text.
- Return-to-origin behavior.
- User interaction.
- Recency.
- Recognized surface type.
- Line-anchor quality.
- Branch penalty.
- Artifact penalty.
- Browser-chrome penalty.
- Unknown-surface penalty.
- Metadata-suspect penalty.

Smalltalk tries to choose a line-level anchor when possible. A strong anchor can come from selected text or ordered content units with bounds. Browser toolbar text, file titles, generated JSON views, and export folders are treated as weak or diagnostic evidence.

## What Smalltalk Does Not Capture

Smalltalk currently does not capture:

- Audio.
- Microphone input.
- Meeting transcripts.
- Speaker labels.
- Continuous video.
- Webcam images.
- Raw typed characters.
- Raw clipboard text.
- Password-manager windows when default skip rules match.
- Raw browser DOM from the native app.
- Browser extension page chunks in the native capture pipeline.

The native app can infer browser URL/title/text through Accessibility, AppleScript, screenshot OCR, and window metadata. It does not have the same DOM-level visibility as the older browser extension.

## Permission Dependencies

Capture quality depends on macOS permissions and available tools:

- Screen Recording permission is required for `screencapture`.
- Accessibility permission is required for AX helpers and System Events fallback.
- Input Monitoring may be required for global event taps.
- Apple Vision is used for OCR on macOS.
- Tesseract is optional fallback if installed on `PATH`.
- Swift helpers are compiled into the app-data `helpers/` directory with `/usr/bin/swiftc`.

If a permission is missing, the frame may still store partial evidence or diagnostic errors, but screenshot, event, Accessibility, or OCR signal may be absent.

## Current Implementation Boundaries

- The active capture system is local-first and SQLite-backed.
- Most raw capture stays in the app-data capture directory.
- Model-facing paths go through safe export or resume-query bundle construction.
- The current stop path builds `resume_query_exports/.../resume-query-bundle.json`; it does not currently produce the older exhaustive repo-root `output/session-XXX` artifact.
- The native path is cross-app, but browser semantics are weaker than a browser extension because it relies on Accessibility, OCR, and browser URL metadata instead of DOM instrumentation.
