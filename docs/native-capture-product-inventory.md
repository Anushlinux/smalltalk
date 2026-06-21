# Smalltalk Native Capture Product Inventory

This document describes the current Tauri desktop capture product: what causes a capture, what methods run, and exactly what evidence Smalltalk stores locally.

It is implementation-accurate for the native app in `src-tauri/src/capture.rs` and the helper scripts in `src-tauri/scripts/`. It does not describe the older browser-extension resume flow except where browser metadata is captured by the native app.

## Product Goal

Smalltalk is a local screen-memory inspector. It captures enough evidence to answer:

- What app, window, document, or browser tab was active?
- What changed recently?
- Was the capture backed by screenshot, Accessibility, OCR, window graph, and event evidence?
- Which local evidence supports a resume cue?

The product is not trying to record meetings, audio, microphone input, speaker identity, or continuous video.

## Local Storage Location

The native app stores capture data under the Tauri app data directory:

```text
<app_data_dir>/capture/
  smalltalk-capture.sqlite
  snapshots/
  helpers/
```

On macOS this is typically under:

```text
~/Library/Application Support/com.smalltalk.app/capture/
```

The UI reads `data_dir` and `database_path` from `capture_status`, so the live app can show the exact runtime paths.

## What Causes Capture

Capture is caused by one of three paths: explicit user action, native UI events, or idle fallback.

| Cause | Stored trigger | Source | Delay before capture | Dedupe |
| --- | --- | --- | ---: | --- |
| User presses `Capture now` | `manual` | Tauri command `capture_once` | none | no |
| Capture loop starts | `manual` | `start_capture` worker initial frame | none | no |
| App changes | `app_switch` | `NSWorkspace.didActivateApplicationNotification` | 300 ms | yes |
| Focused window/UI element changes | `window_focus` or `accessibility_change` | macOS Accessibility observer notifications | 300 ms | yes |
| Mouse click | `click` | listen-only `CGEvent` event tap | 220 ms | yes |
| Keyboard input pauses | `typing_pause` | listen-only `CGEvent` key-down event | 850 ms | yes |
| Scroll activity settles | `scroll_stop` | listen-only `CGEvent` scroll-wheel event | 500 ms | yes |
| Clipboard changes | `clipboard` | `NSPasteboard.general.changeCount` polling | 220 ms | yes |
| No event has produced a frame recently | `idle` | background worker timer | 10 sec interval | yes |

Only one pending event bucket is kept at a time. If several events happen before the settle delay expires, Smalltalk merges them into one pending capture. If the merged events have different trigger types, the trigger becomes `event_burst`.

Captures are also rate-limited by `MIN_CAPTURE_INTERVAL` at 600 ms.

## Capture Pipeline

Each stored frame follows this pipeline:

1. Collect current Accessibility context.
2. Apply exclusion/privacy rules against app, bundle id, window title, URL, and text.
3. If the rule says `skip_capture`, do not store a frame.
4. Collect a window graph snapshot.
5. Capture a full-screen JPG screenshot.
6. Try to capture an active-window JPG crop.
7. Hash screenshot bytes.
8. Decide whether Accessibility text is strong or thin.
9. Run OCR only when Accessibility text is missing or thin.
10. Resolve `full_text` from Accessibility, OCR, or both.
11. Dedupe event/idle frames against prior image and text hashes.
12. Insert frame metadata and related evidence rows into SQLite.
13. Emit a `capture-frame` event to the Tauri UI.

## Methods Used

| Method | File/function | What it captures |
| --- | --- | --- |
| macOS screenshot CLI | `/usr/sbin/screencapture -x -t jpg` | Full-screen JPG snapshot |
| macOS screenshot CLI with window id | `/usr/sbin/screencapture -x -t jpg -l <window_id>` | Active-window crop when a window id exists |
| Swift Accessibility helper | `accessibility_snapshot.swift` | Foreground app, bundle id, PID, focused window, browser URL, document path, AX nodes, bounds, selected text, focused element |
| AppleScript Accessibility fallback | embedded `ACCESSIBILITY_SCRIPT` in `capture.rs` | App/window/browser/document fields and flattened AX text if the Swift helper is unavailable or weak |
| Swift event helper | `capture_events.swift` | App switches, AX notifications, clicks, scrolls, key categories, clipboard metadata |
| Swift window helper | `window_snapshot.swift` | Window graph, active window id, app PID/bundle id, screen count, window bounds |
| Apple Vision OCR | `vision_ocr.swift` | OCR text spans and normalized bounding boxes on macOS |
| Tesseract fallback | `tesseract` on `PATH` | OCR text when Vision is unavailable or empty |
| SQLite + FTS5 | `smalltalk-capture.sqlite` | Structured evidence storage and search index |

## Exact Data Captured

### Frame Core

Stored in `frames`.

| Field group | Exact data |
| --- | --- |
| Identity/time | `id`, `captured_at`, `created_at` |
| Screenshot paths | `snapshot_path`, `full_screenshot_path`, `active_window_crop_path`, `active_element_crop_path` |
| Surface metadata | `app_name`, `window_name`, `browser_url`, `document_path`, `focused` |
| Trigger lineage | `capture_trigger`, `capture_trigger_id`, `previous_frame_id` |
| Text | `text_source`, `accessibility_text`, `accessibility_tree_json`, `full_text` |
| Hashes | `content_hash`, `image_hash`, `phash` |
| Capture metadata | `capture_provider`, `scope`, `display_id`, `window_id`, `app_pid`, `app_bundle_id`, `screen_scale`, `pixel_width`, `pixel_height` |
| Privacy | `privacy_status` |

Current `capture_provider` is `screencapture_cli`. Current `scope` is `active_window` when an active window id is known, otherwise `active_display`.

### Native UI Events

Stored in `ui_events`.

| Event data | Exact fields |
| --- | --- |
| Event identity | `id`, `ts_ms`, `event_type`, `created_at_ms` |
| Foreground app/window | `app_pid`, `app_bundle_id`, `app_name`, `window_id`, `window_title` |
| Pointer | `x`, `y`, `button` |
| Scroll | `scroll_dx`, `scroll_dy` |
| Keyboard category | `key_category`, `modifier_flags`, `is_repeat` |
| Extra payload | `payload_json` |

Keyboard capture does not store raw typed characters. It stores categories such as `char`, `enter`, `backspace`, `shortcut`, `modifier`, `escape`, and `arrow`, plus modifier flags.

### Capture Triggers

Stored in `capture_triggers`.

| Field | Meaning |
| --- | --- |
| `id` | Trigger id linked from a frame |
| `ts_ms` | When the trigger was scheduled |
| `trigger_type` | `manual`, `idle`, `app_switch`, `click`, `typing_pause`, `scroll_stop`, `clipboard`, `accessibility_change`, or `event_burst` |
| `caused_by_event_ids` | JSON array of `ui_events.id` values |
| `settle_delay_ms` | Delay before capture |
| `rate_limited` | Whether the trigger was rate-limited |
| `dedupe_policy` | `manual_bypass`, `event_bucket`, or `layered` |
| `pre_frame_id` / `post_frame_id` | Frame before and after the trigger |
| `status` | `scheduled`, `captured`, `skipped`, or `failed` |
| `error` | Failure text when capture failed |

### Transitions

Stored in `event_transitions`.

Transitions link the cause to the result. Current classification is simple:

| Trigger/evidence | Transition type |
| --- | --- |
| `app_switch` | `switched_app` |
| `scroll_stop` | `scrolled_to_new_section` |
| `typing_pause` | `entered_input` |
| `clipboard` | `copying_evidence` |
| Stored post-frame missing | `same_screen_idle` |
| Same app/window/URL as previous frame | `continuing_same_task` |
| Different surface from previous frame | `new_task` |
| Click without stronger classification | `unknown` |

Stored fields include `trigger_id`, `primary_event_id`, `pre_frame_id`, `post_frame_id`, `ts_start_ms`, `ts_end_ms`, `transition_type`, `confidence`, `summary`, and `changed_region_json`.

### Accessibility Nodes

Stored in `ax_nodes`.

The Swift helper walks the focused app/window Accessibility tree up to bounded depth/count limits. For each node it can store:

- Node id and parent id.
- App PID and window id.
- Role, subrole, role description.
- Title, value, description, help, identifier.
- Document URL/path fields.
- Selected text.
- Selected-text range and visible-character range JSON.
- Number of characters.
- Focused, enabled, selected flags.
- Bounds: `bounds_x`, `bounds_y`, `bounds_w`, `bounds_h`.
- Available Accessibility actions.
- Children count.
- Depth.
- Raw node JSON.

This is the preferred semantic source because it captures UI structure, not just pixels.

### OCR Text And OCR Spans

Stored in `ocr_text` and `ocr_spans`.

`ocr_text` stores:

- `frame_id`
- extracted OCR `text`
- raw OCR JSON payload as `text_json`
- `ocr_engine`

`ocr_spans` stores per-span OCR evidence:

- span id and frame id
- OCR engine
- text
- confidence
- language when present
- block/line/word indexes
- pixel bounds
- normalized bounds JSON
- raw JSON

OCR is not always run. It runs only when Accessibility text is missing or considered thin.

### Resolved Text Source

Smalltalk resolves one `full_text` per frame.

| Condition | `text_source` | `full_text` |
| --- | --- | --- |
| Accessibility text exists and is not thin | `accessibility` | Accessibility text |
| Accessibility text exists but is thin, and OCR exists | `hybrid` | Accessibility text plus OCR text |
| Accessibility text is missing and OCR exists | `ocr` | OCR text |
| Neither exists | `null` | `null`, unless diagnostic errors are attached in memory for UI display |

Accessibility is considered thin for canvas-heavy or browser-chrome-heavy surfaces, including patterns such as Google Docs, Google Sheets, Google Slides, Figma, Excalidraw, Miro, Canva, and tldraw.

### Content Units

Stored in `content_units`.

Content units are the product-facing evidence objects used by the inspector. They are derived from AX nodes and OCR spans.

Stored fields include:

- `id`
- `frame_id`
- `window_id`
- `source`: `ax` or `ocr`
- `unit_type`: `button`, `input`, `link`, `menu_item`, `table_cell`, `image`, `heading`, `paragraph`, or `unknown`
- `text`
- `text_hash`
- `semantic_role`: current examples include `composer`, `error`, `search_result`, `code`, and `assistant_answer`
- linked `ax_node_id`
- linked OCR span ids
- bounds
- optional crop path
- visible ratio
- distance from screen center
- confidence
- creation timestamp
- raw JSON

AX content units usually have higher confidence than OCR-only units. Focused AX nodes get the strongest current confidence.

### App Contexts

Stored in `app_contexts`.

This converts raw app/window/URL evidence into a rough product object.

| App/surface | Adapter | Object type |
| --- | --- | --- |
| ChatGPT / Claude in browser | `chat_adapter` | `chat_conversation` |
| Other browser tab | `browser_adapter` | `browser_tab` |
| Cursor / VS Code / Xcode / IntelliJ | `ide_adapter` | `code_editor` |
| Terminal / iTerm / Warp | `terminal_adapter` | `terminal` |
| Preview / PDF | `pdf_adapter` | `pdf` |
| Finder | `finder_adapter` | `finder` |
| Slack / Discord / Messages / WhatsApp | `messaging_adapter` | `messaging` |
| Notion / Linear / Notes | `notes_task_adapter` | `notes_doc` |
| Unknown | `generic_ax_adapter` | `unknown` |

Stored fields include adapter id, object type, primary id, title, URL, file path, selected text, focused object, confidence, and metadata JSON.

### Window Graph

Stored in `window_snapshots` and `windows`.

The window snapshot helper captures:

- active window id
- active app PID
- active app bundle id
- screen count
- raw window graph JSON

Each window row can store:

- CoreGraphics window id
- owner PID/name
- bundle id
- title
- layer
- alpha
- onscreen flag
- active flag
- bounds
- workspace
- raw JSON

This lets the UI verify whether a frame has window-level evidence beyond a screenshot.

### Clipboard Metadata

Stored in `clipboard_events`.

Smalltalk detects clipboard changes by polling `NSPasteboard.general.changeCount`. It does not store full clipboard text. For text clipboard content, the helper stores:

- content type
- text hash
- redacted preview
- byte size
- pasteboard types
- change count
- source frame id
- metadata JSON

For files/images/rich text it stores content type and pasteboard type metadata.

### Typing Bursts

Stored in `typing_bursts`.

Typing bursts summarize keyboard activity without raw typed text:

- start/end timestamps
- app/window identity
- counts for char, backspace, enter, paste, and shortcuts
- committed flag
- commit signal such as `enter`
- `raw_text_captured`, currently false
- optional pre/post frame ids

### Presence Samples

Stored in `presence_samples`.

Current presence rows are basic activity hints:

- timestamp
- display asleep flag
- screen locked flag
- active input recently
- cursor moved recently
- frontmost app bundle id

The current implementation inserts simple active samples; it is not a full idle-time tracker yet.

### Sensitive Regions And Exclusion Rules

Stored in `exclusion_rules` and `sensitive_regions`.

Default rules currently include:

- password-manager app bundles: `skip_capture`
- authentication/sign-in-like window titles: `store_redacted`
- banking/payment/health/password-manager URLs: `store_redacted`
- API key/token/secret-like content: `never_send_to_ai`

Important current behavior:

- `skip_capture` prevents a frame from being stored.
- Other actions set `privacy_status = redacted` and write `sensitive_regions` metadata.
- The current code does not yet pixel-mask the stored screenshot for `store_redacted`.
- There is no AI upload path in the native capture code shown here; `never_send_to_ai` is recorded as privacy metadata for future downstream use.

### Search Index

Stored in `frames_fts`.

SQLite FTS5 indexes:

- `full_text`
- `app_name`
- `window_name`
- `browser_url`
- `document_path`

The UI uses `search_captures` to search this index and return ranked frames with snippets.

## What Is Not Captured

The native Tauri app does not currently capture:

- Audio.
- Microphone input.
- Meeting transcripts.
- Speaker diarization.
- Raw typed characters from keyboard events.
- Full clipboard text as a clipboard event.
- Continuous video.
- Browser DOM structure from an extension.
- Cloud embeddings or remote AI summaries in the native capture path.

Screenshots and Accessibility text can still contain sensitive visible content if the surface is not skipped by privacy rules. Treat raw local capture as sensitive local evidence.

## Runtime Commands Exposed To The UI

The Tauri UI can call:

- `start_capture`
- `stop_capture`
- `capture_once`
- `capture_status`
- `delete_all_frames`
- `search_captures`
- `get_frame`
- `get_frame_image`
- `get_frame_image_variant`
- `get_recent_timeline`
- `get_frame_detail`
- `get_transition`
- `search_content_units`
- `export_debug_episode`
- `get_episode_dossier`
- `add_exclusion_rule`
- `remove_exclusion_rule`
- `list_exclusion_rules`
- `delete_recent_captures`

The `*_v2` command names currently route to the same native capture implementation.

## Trust Model

Smalltalk’s trust model is evidence-first:

- A frame is stronger when it has screenshot, AX nodes, content units, app context, window graph, transition, and linked UI events.
- The UI can call `get_frame_detail` to inspect all linked evidence for a frame.
- The `VerificationSignals` readout marks missing sources rather than pretending every capture is equally strong.
- Dedupe skips repeated idle/event frames only when both image and content hashes match.

The intended product behavior is not “trust the summary.” It is “show the local evidence that caused the summary.”
