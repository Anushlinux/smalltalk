# Smalltalk Full Engine Flow

Last updated: 2026-07-04

This document explains the current native Smalltalk engine from the first local signal to the final `Continue` answer. It is written from the implemented code path in this repository, especially `src-tauri/src/capture.rs`, `src-tauri/src/continuation.rs`, `src-tauri/src/lib.rs`, `src-tauri/src/capture_core/resume_dossier.rs`, and `product.md`.

The active engine is the native Tauri desktop app. The older `browser-extension/` prototype is not the current MVP engine. `run_cloud_resume`, `get_native_resume_card`, stop-time `resume_query_exports`, and the floating island still exist, but they are secondary, legacy, diagnostic, or fallback surfaces. The core product engine is `get_continue_decision`.

## One Page Pipeline

```text
Native local signals
  -> ui_events, typing_bursts, clipboard_events
  -> coalesced capture_triggers
  -> sparse heavy frames when worthwhile
  -> SQLite evidence substrate
  -> Continue layer 1: schema and semantic memory tables
  -> Continue layer 2: artifacts, observations, task actions
  -> Continue layer 3: episodes, workstreams, durable roles, unresolved state
  -> final layer: candidates, scoring, confidence caps, warnings
  -> default bounded micro-inference over candidate ids only
  -> persisted Continue decision
  -> handoff payload for the UI
  -> open_resume_point by continue_decision_id when the user clicks Continue here
```

Smalltalk does not try to record everything and then ask a model what happened. It first builds local evidence and local semantic objects. The output is one continuation decision that separates:

- `current_focus`: what is factually visible now.
- `current_activity`: a short read of what seems to be happening now.
- `selected_workstream`: the durable cluster of related work.
- `return_target`: the artifact Smalltalk thinks is the best place to return.
- `resume_work_target`: the actionable work target inside the workstream, kept separate from support branches.

The engine must not invent artifacts, URLs, file paths, intent, or next actions. If evidence is thin, the output should say that evidence is thin and expose inspectable anchors.

## Public Command Surface

The commands are registered in `src-tauri/src/lib.rs`. The active native engine is exposed through these groups:

| Group | Commands | Role |
| --- | --- | --- |
| Capture runtime | `start_capture`, `stop_capture`, `capture_once`, `capture_status` | Start, pause, manually sample, and inspect local memory. |
| Reset and cleanup | `delete_all_frames`, `get_local_memory_diagnostics`, `cleanup_local_memory`, `dev_reset_local_memory`, `delete_recent_captures` | Developer and user cleanup controls. |
| Evidence read APIs | `search_captures`, `get_frame`, `get_frame_image`, `get_frame_image_variant`, `get_recent_timeline`, `get_frame_detail`, `validate_frame_consistency`, `get_transition`, `search_content_units` | Read raw or processed local evidence. |
| Legacy/native compatibility | `start_native_capture`, `stop_native_capture`, `capture_once_v2`, `get_frame_v2` | Compatibility wrappers around the current capture implementation. |
| Safe/export diagnostics | `build_safe_ai_export`, `build_session_index`, `build_resume_query_bundle`, `export_debug_episode`, `get_episode_dossier`, `get_native_storyboard_dossier` | Evidence export and diagnostics. |
| Older cloud resume lane | `run_cloud_resume`, `get_cloud_resume_status` | Stop/resume-query OpenAI path. Not the core Continue engine. |
| Continue memory | `get_continue_memory_status`, `rebuild_continue_second_layer`, `rebuild_continue_third_layer`, `get_recent_continue_artifacts`, `get_recent_continue_task_actions`, `get_recent_continue_episodes`, `get_recent_continue_workstreams`, `get_continue_workstream_detail` | Semantic memory inspection and rebuild APIs. |
| Continue decision | `get_continue_decision`, `open_resume_point` | Core product decision and action. |
| Continue feedback | `add_continue_breadcrumb`, `infer_continue_feedback`, `record_continue_feedback`, `run_continue_eval` | Manual notes, inferred/explicit feedback, and evals. |
| Legacy resume/eval | `classify_episode_transitions`, `get_native_resume_card`, `run_resume_eval` | Older local resume-card behavior. Not the core Continue engine. |
| Privacy rules | `add_exclusion_rule`, `remove_exclusion_rule`, `list_exclusion_rules` | Runtime privacy and exclusion configuration. |

The important boundary is this:

- `get_continue_decision` is the core Continue engine.
- `open_resume_point` is the primary opener for a Continue decision.
- `run_cloud_resume` is the older stop/resume-query OpenAI path.
- `get_native_resume_card` is a legacy local resume-card fallback.
- `browser-extension/` is not the current native MVP engine.

## Runtime Storage

The active code resolves the capture root through Tauri app data:

```text
<tauri app data>/capture/
  smalltalk-capture.sqlite
  snapshots/
  helpers/
  safe-ai-exports/
```

On this Mac, product docs describe the normal resolved location as:

```text
~/Library/Application Support/com.smalltalk.app/capture/
```

The frontend should trust `capture_status.data_dir` and `capture_status.database_path` rather than hard-coding a location. Some older generated artifacts may live under repo folders such as `resume_query_exports/`, but those are not the live capture store.

Important stored evidence tables include:

| Table | What it stores |
| --- | --- |
| `capture_sessions` | Local-memory session rows, status, timestamps, counts, and stop metadata. |
| `frames` | Heavy evidence frames: app/window/url/path, full text, hashes, screenshot paths, trigger, session, privacy, ScreenCaptureKit metadata. |
| `frames_fts` | Full-text search index over frame text and metadata. |
| `ui_events` | Lightweight native events such as app switches, focus changes, clicks, scrolls, key categories, clipboard events, and AX notifications. |
| `capture_triggers` | Coalesced event buckets that may produce a frame. |
| `event_transitions` | Transition summaries linking pre-frame, event, and post-frame evidence. |
| `typing_bursts` | Keyboard activity summaries without raw typed characters. |
| `clipboard_events` | Clipboard metadata without full clipboard text. |
| `ocr_text` | Joined OCR text and raw OCR payload for a frame. |
| `ocr_spans` | OCR span text, confidence, bounds, indexes, and raw span JSON. |
| `ax_nodes` | Accessibility nodes, roles, text, bounds, focus, selected state, actions, and raw node JSON. |
| `content_units` | Normalized units extracted from AX and OCR for semantic roles and search. |
| `app_contexts` | App/product adapters for browser tabs, docs, terminals, conversations, files, and other surfaces. |
| `window_snapshots` / `windows` | macOS window graph snapshots and observed windows. |
| `frame_diffs` | Simplified text/content changes between adjacent frames. |
| `presence_samples` | Local presence/activity samples. |
| `exclusion_rules` | Privacy and exclusion rules. |
| `sensitive_regions` | Sensitive visual/text regions and privacy action taken. |
| `frame_quality_warnings` | Evidence consistency and quality warnings. |
| `maintenance_counters` | Runtime counters for capture, storage, and Continue diagnostics. |

The Continue engine creates additional semantic tables through `ensure_continue_schema`, covered later.

## What Smalltalk Captures

Smalltalk has two kinds of local evidence:

1. Lightweight signals that are cheap and first-class.
2. Heavy frames that are expensive and intentionally sparse.

Lightweight signals include:

- Frontmost app changes.
- Focused window changes.
- Accessibility notifications.
- Clicks.
- Scrolls.
- Key-down categories.
- Clipboard changes.
- App name, bundle id, process id, window title, pointer coordinates, scroll deltas, modifier flags, and helper payload metadata.

Heavy frames can include:

- Full-screen JPEG screenshot.
- Active-window JPEG crop when a window id is known.
- Foreground app name.
- App process id and bundle id.
- Focused window title.
- Browser URL when available.
- Document path from Accessibility.
- Accessibility text and node tree.
- OCR text and spans when needed.
- Window graph snapshot.
- Content units.
- App contexts.
- Sensitive-region and privacy metadata.
- Frame diff against the previous frame.

Smalltalk does not capture:

- Raw typed characters.
- Full clipboard text.
- Audio.
- Microphone input.
- Continuous video.
- Speaker identity.

Keyboard evidence is stored as categories and counts, such as `char`, `enter`, `backspace`, `shortcut`, `modifier`, `escape`, and `arrow`. Clipboard evidence is stored as metadata such as content type, byte size, hash, pasteboard types, and redacted preview where available.

## Capture Priority And Preferences

The current preference order is:

1. Lightweight native events are first-class local memory signals.
2. Manual and session-start captures can force heavy evidence and bypass dedupe.
3. Important event triggers can produce heavy frames, but are rate-limited.
4. Low-value event triggers can produce heavy frames only when budgets allow.
5. Idle capture is only a fallback.
6. Accessibility is the preferred semantic text source.
7. OCR is secondary and runs only when Accessibility is missing or thin.
8. ScreenCaptureKit is the preferred screenshot provider on macOS.
9. `/usr/sbin/screencapture` is the screenshot fallback.
10. Event and idle captures are budgeted and deduped.

The current capture intervals and budgets in `capture.rs` are:

| Constant | Value | Meaning |
| --- | ---: | --- |
| `EVENT_LOOP_WAKE_INTERVAL` | 100 ms | Worker wake interval while listening for events. |
| `MIN_IMPORTANT_CAPTURE_INTERVAL` | 4 sec | Minimum spacing for important heavy captures. |
| `MIN_LOW_VALUE_CAPTURE_INTERVAL` | 45 sec | Minimum spacing for low-value heavy captures. |
| `IDLE_CAPTURE_INTERVAL` | 120 sec | Idle fallback interval. |
| `CAPTURE_BUDGET_ROLLING_WINDOW_MS` | 10 min | Rolling window for screenshot budget. |
| `MAX_SCREENSHOT_FRAMES_PER_10_MINUTES` | 24 | Max non-important screenshot frames in the rolling window. |
| `MAX_SCREENSHOT_FRAMES_PER_SURFACE_WITHOUT_CHANGE` | 3 | Max same-surface captures without meaningful semantic change. |
| `MAX_LOCAL_SNAPSHOT_DIR_BYTES` | 512 MB | Snapshot directory budget for low-value captures. |
| `MAX_LOCAL_SIGNAL_EVENTS` | 5,000 | Max local event signals considered for some status calculations. |

Important triggers are:

- `manual`
- `session_start`
- `app_switch`
- `window_focus`
- `clipboard`

Low-value triggers include:

- `typing_pause`
- `scroll_stop`
- `click`
- `accessibility_change`
- `event_burst`
- `idle`

This does not mean typing or scrolling are ignored. Their lightweight events still matter. It means Smalltalk avoids storing heavy screenshots for every tiny movement.

## Starting Local Memory

The UI calls `start_capture`.

The backend:

1. Ensures the database exists.
2. Checks whether the runtime is already running.
3. Updates and shows the native session island in a recording state.
4. Creates a `capture_sessions` row.
5. Stores the active session id in runtime state.
6. Starts the Rust capture worker thread.
7. Starts the native event helper if possible.
8. Immediately attempts a `session_start` heavy capture.
9. Emits status back to the UI.

The native event helper is compiled and run from the app-data `helpers/` directory. The Swift source is `src-tauri/scripts/capture_events.swift`.

If the event helper cannot start, Smalltalk can still run with idle/manual capture, but the engine loses important lightweight event signal.

## Native Events And Trigger Coalescing

The event helper emits newline-delimited JSON. Rust parses each event into an internal `UiEventRecord`.

Recognized event-to-trigger mapping:

| Event type | Capture trigger | Settle delay |
| --- | --- | ---: |
| `app_switch` | `app_switch` | 300 ms |
| `window_focus` | `window_focus` | 300 ms |
| `accessibility_change` / `ax_notification` | `accessibility_change` | 300 ms |
| `click` | `click` | 220 ms |
| `key_down` | `typing_pause` | 850 ms |
| `scroll` | `scroll_stop` | 500 ms |
| `clipboard` | `clipboard` | 220 ms |

Events are inserted into `ui_events`. The runtime also increments the `events_aggregated` maintenance counter.

Only one pending trigger bucket is kept at a time. If another event arrives before the pending trigger settles:

- Its event id is added to `capture_triggers.caused_by_event_ids`.
- The settle deadline is pushed out.
- If the new event has a different trigger type, the bucket becomes `event_burst`.

This makes the runtime event-driven without creating a heavy frame for every event.

## Typing And Clipboard Side Effects

Keyboard and clipboard events create side-effect rows:

- `key_down` updates or creates a `typing_bursts` row.
- `clipboard` creates a `clipboard_events` row.

Typing bursts store:

- Start and end timestamps.
- App/window context.
- Character count.
- Backspace count.
- Enter count.
- Paste count.
- Shortcut count.
- Whether a commit signal occurred.
- Pre-frame id.
- `raw_text_captured = 0`.

Clipboard events store:

- Change count.
- Content type.
- Text hash when available.
- Redacted preview when available.
- Byte size.
- Source frame id.
- Metadata JSON.

They do not store raw typed text or full clipboard text.

## Heavy Capture Pipeline

The heavy capture function is `capture_frame`. Every stored frame goes through this sequence:

1. Resolve `capture_paths`.
2. Ensure the snapshot directory exists.
3. Ensure the SQLite database exists.
4. Build a day-bucketed snapshot directory under `snapshots/`.
5. Collect Accessibility context from the frontmost app.
6. Build a semantic fingerprint from app, window, URL, document path, and text hash.
7. Apply privacy and exclusion rules.
8. If privacy says skip, return without storing a frame.
9. Apply heavy-capture budget gates.
10. Collect a macOS window snapshot if available.
11. Choose active window id from Accessibility or window snapshot.
12. Capture full screenshot.
13. Hash full screenshot bytes into `image_hash`.
14. Determine JPEG dimensions.
15. Try an active-window screenshot crop when a window id exists.
16. Check cancellation and delete just-captured images if cancelled.
17. Decide whether Accessibility text is strong or thin.
18. Run OCR if Accessibility is missing or thin.
19. Resolve one `text_source` and one `full_text`.
20. Compute `content_hash` from `full_text`.
21. Check cancellation again.
22. Apply dedupe for event/idle captures.
23. Collect ScreenCaptureKit metadata.
24. Insert the `frames` row.
25. Persist linked OCR, AX, window, app context, content unit, privacy, presence, and diff rows.
26. Validate frame consistency.
27. Load the stored frame back from SQLite.
28. Attach diagnostic text if both Accessibility and OCR failed.
29. Return a `CaptureOutcome`.

If a frame is stored, `capture_and_emit`:

- Updates runtime success state.
- Emits `capture-frame` to the UI.
- Updates session island state.
- Finalizes the trigger as stored.

If no frame is stored, it:

- Updates skip counters.
- Finalizes the trigger as skipped where applicable.

## Heavy Capture Budget Gates

Budget checks run before screenshots are taken. Manual and session-start captures bypass these gates.

A heavy capture can be skipped before screenshot work if:

- The context is Smalltalk itself.
- A low-value trigger exceeds the rolling screenshot budget.
- A low-value trigger would push snapshot storage beyond the configured snapshot directory budget.
- The semantic surface is unchanged and already has enough recent heavy frames.

Error signals are protected. If the current context appears to contain an error, budget gating should not skip that capture just because it is low-value.

Skip reasons include:

- `privacy`
- `smalltalk_self_observation`
- `rolling_screenshot_budget_exhausted`
- `snapshot_directory_budget_exhausted`
- `same_surface_without_meaningful_change`
- `dedupe`
- `cancellation`

These are not product failures by themselves. They are how Smalltalk keeps local memory lightweight enough to be useful.

## Screenshot Capture

On macOS, the preferred provider is ScreenCaptureKit through `src-tauri/scripts/sck_screenshot.swift`.

For full-screen capture:

```text
capture_sck_screenshot(mode = "display")
```

For active-window capture:

```text
capture_sck_screenshot(mode = "active_window", target_window_id = ...)
```

The request excludes the Smalltalk bundle id:

```text
exclude_bundle_ids = ["com.smalltalk.app"]
```

The JPEG quality is currently `0.82`, and cursor inclusion is disabled in the current calls.

If ScreenCaptureKit fails, Smalltalk falls back to:

```text
/usr/sbin/screencapture -x -t jpg <path>
/usr/sbin/screencapture -x -t jpg -l <window_id> <path>
```

Stored frame metadata includes:

- `capture_provider`
- `scope`
- `display_id`
- `window_id`
- `app_pid`
- `app_bundle_id`
- pixel dimensions
- full screenshot path
- active-window crop path
- perceptual hash placeholder / image hash
- privacy status
- ScreenCaptureKit display/window/bundle metadata
- ScreenCaptureKit filter/configuration/frame metadata JSON
- capture mode
- audio policy

The active screenshot scope is:

- `active_window` if an active window crop exists.
- `active_display` otherwise.

## Accessibility Capture

Accessibility is the primary semantic source because it gives structured UI context, not just pixels.

The primary helper is:

```text
src-tauri/scripts/accessibility_snapshot.swift
```

The fallback is the embedded AppleScript path in `capture.rs`.

Accessibility can provide:

- App name.
- Process id.
- Bundle id.
- Focused window title.
- Window id.
- Browser URL.
- Document path.
- Selected text.
- Focused node.
- AX node tree.
- Node roles, labels, values, descriptions, bounds, focus state, enabled/selected flags, actions, child counts, depth, and raw node JSON.

Smalltalk prefers Accessibility text over OCR when Accessibility is strong enough.

## Thin Accessibility And OCR

OCR is secondary. It runs only when Accessibility text is absent or thin.

Accessibility can be considered thin when:

- The surface is canvas-heavy.
- Total Accessibility text is short.
- Browser chrome or UI roles dominate.
- Content-like text is sparse.

Examples of canvas-heavy surfaces include:

- Google Docs
- Google Sheets
- Google Slides
- Figma
- Excalidraw
- Miro
- Canva
- tldraw

OCR priority:

1. Apple Vision OCR through `src-tauri/scripts/vision_ocr.swift`.
2. `tesseract` if Vision is unavailable, fails, or returns empty text.

OCR output is stored in `ocr_text` and `ocr_spans`.

Text source resolution:

| Inputs | Stored `text_source` | Stored `full_text` |
| --- | --- | --- |
| Strong Accessibility text | `accessibility` | Accessibility text. |
| Thin Accessibility text plus OCR | `hybrid` | Accessibility text plus OCR text. |
| No Accessibility text plus OCR | `ocr` | OCR text. |
| Neither source has text | `null` | `null`, with diagnostic errors attached where possible. |

`full_text` is used by FTS search, semantic extraction, scoring, and safe export paths.

## Deduplication

Event and idle captures use dedupe. Manual and session-start captures do not.

Dedupe compares:

- `image_hash` from full screenshot bytes.
- `content_hash` from resolved `full_text`.

If both image and content are unchanged:

- The just-created full screenshot is deleted.
- The active-window crop is deleted if it exists.
- No `frames` row is inserted.
- The trigger is finalized as skipped.

This keeps the engine from turning a quiet screen into hundreds of duplicate screenshots.

## Linked Evidence Rows

After a frame is inserted, Smalltalk persists linked rows:

- `ocr_text` for joined OCR text.
- `ocr_spans` for OCR boxes and confidence.
- `window_snapshots` and `windows` for the current window graph.
- `ax_nodes` for Accessibility structure.
- `app_contexts` for product-object adapters.
- `content_units` for normalized semantic text units.
- `sensitive_regions` for privacy handling.
- `presence_samples` for activity state.
- `frame_diffs` for changes from the previous frame.
- `frame_quality_warnings` if consistency validation finds problems.

This is the evidence substrate. Continue does not only look at screenshots. It reads frames, events, typed/clipboard metadata, app contexts, content units, transitions, diffs, and privacy status.

## Stop Capture And Resume Query Bundles

`stop_capture` stops the runtime and finalizes the session. It can also build a compact stop-time resume-query bundle under `resume_query_exports/`.

That stop-time bundle is not the primary Continue product path. It exists for older cloud resume and debugging flows.

The resume dossier policy in `capture_core/resume_dossier.rs` currently defines:

| Setting | Value |
| --- | ---: |
| Schema | `smalltalk.resume_query.v2` |
| Default max JSON chars | 25,000 |
| Default max model images | 12 |
| Default max episode cards | 8 |

`run_cloud_resume` builds a bounded resume-query bundle and calls OpenAI for the older Ask OpenAI path. That path can return `source = "cloud"` when a real response exists, or local fallback data when it cannot safely use the cloud result. It is separate from the core `get_continue_decision` engine.

## Continue Schema

The Continue schema is created by `ensure_continue_schema` in `src-tauri/src/continuation.rs`.

Main Continue tables:

| Table | Purpose |
| --- | --- |
| `continue_schema_migrations` | Continue schema version marker. |
| `continue_artifacts` | Stable work objects such as docs, URLs, code editors, terminals, PDFs, chats, messages, and fallback surfaces. |
| `continue_artifact_observations` | Per-frame observations of artifacts. |
| `continue_task_actions` | Derived user/task actions. |
| `continue_task_action_events` | Join table from task actions to raw events. |
| `continue_episodes` | Adjacent actions grouped into episodes. |
| `continue_episode_actions` | Episode-to-action joins. |
| `continue_episode_artifacts` | Artifact roles inside an episode. |
| `continue_workstreams` | Durable clusters of related episodes and artifacts. |
| `continue_workstream_episodes` | Workstream-to-episode joins. |
| `continue_workstream_artifacts` | Durable artifact roles inside a workstream. |
| `continue_candidates` | Scored candidate return targets. |
| `continue_decisions` | Persisted Continue decisions and handoff lines. |
| `continue_feedback_events` | Explicit and inferred feedback. |
| `continue_breadcrumbs` | Manual local next-step notes attached to workstreams. |

The Continue layer is built on local evidence. It does not require stopping capture.

## Continue Layer 2: Artifacts And Actions

`rebuild_continue_second_layer` loads evidence frames and event-only evidence, clears stale second-layer rows for those frames, then rebuilds:

1. Stable artifacts.
2. Artifact observations.
3. Task actions.

Artifact identity priority is:

1. Document path, unless privacy blocks it.
2. Browser URL.
3. App context object identity.
4. Window title.
5. Fallback surface hash.

Document-path artifacts can become:

- `pdf`
- `code_editor`
- `notes_doc`
- `unknown`

URL artifacts can become:

- `chat_conversation`
- `messaging`
- `notes_doc`
- `pdf`
- `browser_tab`

Context artifacts can become:

- `chat_conversation`
- `browser_tab`
- `code_editor`
- `terminal`
- `pdf`
- `finder`
- `messaging`
- `notes_doc`
- `unknown`

Artifact quality/openability is also assigned:

- Strong path identity normally has higher confidence.
- URL identity is generally openable and strong or medium.
- App context identity can be medium.
- Window-title fallback can be medium or thin.
- Surface fallback is thin and less openable.

Artifact observations store:

- Artifact id.
- Frame id.
- App context id.
- Text source.
- Content hash.
- Image hash.
- Focused-node evidence.
- Selected-text presence.
- Visible text length.
- Observation confidence.
- Reason.
- Timestamp.

## Task Action Extraction

The action extractor walks frames in order and classifies what each observation means.

It uses:

- Current artifact.
- Previous artifact.
- Recent primary artifacts.
- Last meaningful action.
- Trigger type.
- Transition label.
- UI events.
- Typing bursts.
- Clipboard events.
- Content roles.
- Frame diffs.
- Text signals.

Main action kinds include:

| Action kind | Meaning |
| --- | --- |
| `editing` | Typing in an editable artifact such as a code editor or note/doc. |
| `composing` | Typing in a composer, chat, message, or form-like context. |
| `copying_evidence` | Clipboard or paste evidence suggests moving information between surfaces. |
| `running_command` | Terminal enter/commit signal. |
| `observing_command_output` | Terminal output changed. |
| `reviewing_output` | Output, generated content, logs, chat output, or similar review signal. |
| `encountering_error` | Error, failure, stack trace, panic, failed build/test, or error content role. |
| `branching_away` | Switch from a primary work artifact to a support artifact after meaningful progress. |
| `searching` | Search result/query context. |
| `verification_branch` | Branch to terminal/browser/chat for verification after primary work. |
| `returning_to_origin` | Return from a support surface to a recent primary artifact. |
| `messaging_interrupt` | Switch into messaging. |
| `idle_after_progress` | Idle after a meaningful action. |
| `reading` | Visible content without edit or stronger action signal. |
| `switching_context` | Artifact switch without stronger local meaning. |
| `navigating` | Scroll/navigation-like activity. |
| `unknown` | Thin or missing evidence. |

Repeated adjacent actions can be collapsed when they are the same kind, same artifact, same secondary artifact, same base reason, same trigger, and close in frame/time distance. Collapsed actions preserve a count, first/last frame ids, strongest frame id, and merged evidence event ids.

Task actions are inserted into `continue_task_actions`, and their linked raw event ids are inserted into `continue_task_action_events`.

## Continue Layer 3: Episodes

`rebuild_continue_third_layer` loads task actions and builds episodes.

Episodes are contiguous action sequences. A new episode starts when a boundary appears.

Boundary reasons include:

- `returning_to_origin`
- `communication_interruption`
- `verification_branch`
- `error_to_support_branch`
- `support_branch`
- `artifact_switch_after_progress`
- `time_gap_after_progress`
- `artifact_switch`
- `idle_after_progress`

An episode records:

- State.
- Start and end frame id.
- Start and end timestamps.
- Primary artifact id.
- Dominant action kind.
- Boundary start and end reasons.
- Evidence quality.
- Confidence.
- Summary label.

Episode primary artifact selection prefers meaningful work actions:

- `editing`
- `composing`
- `running_command`
- `encountering_error`
- `returning_to_origin`
- `reading`

For support actions such as branching/search/verification, the primary target can be the secondary artifact, because the support surface is evidence for the real work target.

Episode artifact roles include:

| Episode role | Meaning |
| --- | --- |
| `primary_target` | Main thing the user was working on. |
| `branch_support` | Search/docs/support branch used while working. |
| `output_verification` | Terminal, output, or verification surface. |
| `blocker` | Error/failure surface. |
| `interruption` | Messaging or interrupting context. |
| `source_evidence` | Artifact copied from or used as evidence. |
| `current_focus_only` | Observed current focus but not necessarily the target. |
| `unknown` | Thin role. |

## Continue Layer 3: Workstreams

After episodes are built, Smalltalk clusters them into workstreams.

Workstream membership scoring uses:

- Same primary artifact.
- Episode links to existing primary target.
- Secondary artifact links to a workstream primary.
- Return-to-origin behavior.
- Shared artifacts.
- Terminal verification linked to primary work.
- Title/token similarity.
- Short time gaps.

If the best matching workstream score is at least `0.58`, the episode attaches to that workstream. Otherwise a new workstream is created.

Workstream durable artifact roles include:

| Durable role | Meaning |
| --- | --- |
| `primary_target` | The main thing to continue. |
| `blocker_surface` | Error or failure evidence. |
| `verification_surface` | Terminal/output/log/review evidence. |
| `support_source` | Source material or copied evidence. |
| `branch` | Search/docs/support branch. |
| `communication_surface` | Messaging surface relevant to a copy/reply flow. |
| `distractor` | Interruption or low-value surface. |
| `unknown` | Weak role. |

Workstreams also get a `title_candidate`, confidence, source, and unresolved signal.

Unresolved signals include:

- `idle_after_progress`
- `visible_error_or_failure`
- `draft_or_composer_active`
- `verification_without_return`
- `branch_without_return`

Workstream states include:

| State | Meaning |
| --- | --- |
| `active` | Most recent non-stale workstream. |
| `suspended` | Work appears paused with unresolved state or branch/idle boundary. |
| `resumed` | Evidence shows return to origin. |
| `stale` | Workstream is older than the current latest activity by more than the staleness threshold. |
| `abandoned` | Single messaging interruption style workstream. |
| `background` | Recent enough to exist but not selected as active/suspended. |

## Final Decision Layer

`get_continue_decision` is the core product engine.

Default request behavior:

- `lookback_ms`: 45 minutes.
- `limit`: 700.
- `mode`: `normal`.
- `rebuild_layers`: `false`.
- `micro_inference_enabled`: `true`.
- `max_candidates_for_model`: 5.

Normal mode may reuse a fresh cached decision when:

- Rebuild is not forced.
- The cached decision matches the requested inference policy.
- The cached decision is still fresh for the current evidence.

If there is no cache hit, the engine may infer feedback for pending prior decisions, then lazily rebuild Continue layers when needed. In rebuild mode, it rebuilds from the start instead of only from the latest processed frame.

The final decision flow is:

1. Ensure Continue schema.
2. Normalize request defaults.
3. Decide normal vs rebuild mode.
4. Try fresh cached decision if allowed.
5. Infer feedback for pending prior decisions when not cached.
6. Rebuild layer 2 and layer 3 if needed.
7. Load `current_focus`.
8. Load recent scorer workstreams.
9. Fall back to any recent scorer workstreams if scoped load is empty.
10. Generate candidates.
11. Score candidates.
12. Persist generated candidates when not cached.
13. Select the top local candidate.
14. Run bounded micro-inference by default when candidates exist, unless the request explicitly disables it.
15. Apply validation and fallback behavior.
16. Compose/persist the Continue decision.
17. Build evidence anchors and alternatives.
18. Return `ContinueDecisionResult`.

## Current Focus

`current_focus` is factual. It is the latest observed artifact/surface.

It must not automatically become the return target. The user may currently be in:

- Search results.
- A docs/support branch.
- A terminal/log surface.
- Messaging.
- Smalltalk itself.
- A distraction.
- A diagnostic surface.

The decision can warn when current focus differs from the return target. That warning is useful because the product is intentionally separating "what is on screen now" from "where work should continue."

## Candidate Generation

Candidate generation uses workstreams, unresolved signals, last meaningful actions, current focus, and primary artifacts.

Candidate kinds include:

| Candidate kind | Typical reason |
| --- | --- |
| `resolve_error` | Visible error or failure is unresolved. |
| `continue_edit` | Editing or idle-after-progress on editable artifact. |
| `continue_reply` | Draft/composer/chat/message is active or recently meaningful. |
| `rerun_command` | Last meaningful action was command execution. |
| `verify_output` | Verification/output branch has not returned. |
| `resume_chat_reasoning` | Chat conversation appears to be reasoning/work context. |
| `read_next_source` | PDF/source reading is the best continuation. |
| `finish_search` | Search has no clear primary target. |
| `return_to_primary_artifact` | Branch/support surface is evidence; return to the original artifact. |
| `evidence_only` | Evidence exists but is too thin to make a strong actionable claim. |

Candidate generation intentionally resists promoting support branches. For example, if a user edits code, switches to search, then stops, the candidate should usually point back to the code/work artifact, not the search page.

## Candidate Scoring

Candidate score is a weighted local score:

| Component | Weight |
| --- | ---: |
| `actionability_score` | 0.24 |
| `primary_target_score` | 0.20 |
| `unresolved_score` | 0.18 |
| `branch_origin_score` | 0.12 |
| `evidence_quality_score` | 0.12 |
| `openability_score` | 0.07 |
| `privacy_safety_score` | 0.04 |
| `recency_score` | 0.03 |

Actionability examples:

- `resolve_error`: high.
- `continue_edit`: high.
- `continue_reply`: high.
- `rerun_command`: high.
- `verify_output`: medium-high.
- `return_to_primary_artifact`: medium.
- `finish_search`: medium-low.
- `evidence_only`: low.

Primary-target scoring favors:

- Workstream primary artifact.
- Durable `primary_target` role.
- Blocker/verification/communication surfaces only when they are locally justified.

Branch-origin scoring penalizes branch targets unless the branch became primary through meaningful action.

Evidence-quality scoring favors strong/medium artifact or episode evidence and penalizes thin/unknown evidence.

Openability favors targets with browser URLs or document paths.

Privacy safety penalizes Smalltalk itself and sensitive/redacted/blocked surfaces.

## Confidence Caps And Warnings

After weighted scoring, confidence caps prevent overconfident answers.

Caps apply when:

- Candidate is `evidence_only`.
- There is no target artifact.
- Target is unknown or thin.
- Target is only a frame fallback.
- Target has no URL or document path.
- Target is Smalltalk itself.
- Evidence quality is low.
- Last meaningful action is only a collapsed repeated state.
- No last meaningful action exists and there is no unresolved signal.
- Current focus conflicts with return target.
- Recency is high but actionability/primary-target support is weak.

Warnings can include:

- `thin_evidence:no_continue_workstream_candidate`
- `current_focus_differs_from_return_target`
- `current_focus_mismatch`
- `micro_inference_missing_openai_api_key`
- `micro_inference_failed:<reason>`
- `micro_inference_validation_failed:<reason>`

Warnings are part of the product contract. They keep the answer honest instead of pretending the engine knows more than it does.

## Optional Bounded Micro-Inference

Micro-inference is optional. The default UI call disables it.

When enabled, Smalltalk builds a compact candidate pack:

- Schema: `smalltalk.continue_micro_inference_pack.v1`.
- Current focus facts.
- Top workstreams.
- Candidate ids.
- Candidate kinds.
- Target artifact ids and titles.
- Whether URL/path is available.
- Local score components.
- Last meaningful action summary.
- Unresolved state reason.
- Evidence frame/action/episode ids.
- Missing evidence notes.
- Local reasons.
- Artifact roles.
- Breadcrumbs.

The model does not receive:

- Raw screenshots.
- Broad raw history.
- Raw event timelines.
- Raw typed characters.
- Full clipboard text.
- Arbitrary local file contents.
- Permission to invent URLs, paths, artifacts, evidence, or intent.

The request uses the OpenAI Responses API with Structured Outputs. The default model is `gpt-4.1-mini`, unless overridden by:

- `SMALLTALK_CONTINUE_OPENAI_MODEL`
- `SMALLTALK_OPENAI_MODEL`
- `OPENAI_MODEL`

The API key can come from process environment or project `.env`.

The model can only choose from supplied candidate ids. Its output is accepted only if validation passes.

Validation rejects output when:

- Selected candidate id was not sent to the model.
- Selected workstream id does not match the selected candidate.
- Output contains unsupported URLs or paths.
- Required handoff lines are missing or too long.
- `why_this` is empty or has too many items.
- `next_action` is incompatible with an evidence-only candidate.
- High confidence is returned for thin local evidence.
- A branch/support target is promoted without strong local support.

If validation fails, the result source becomes `local_fallback` and the local scorer remains the source of truth.

## Continue Decision Output

`get_continue_decision` returns a `ContinueDecisionResult`.

Important fields:

| Field | Meaning |
| --- | --- |
| `decision_id` | Stable id for this decision and evidence watermark. |
| `mode` | Effective mode, normally `normal` or `rebuild`. |
| `cache_hit` | Whether this came from a fresh cached decision. |
| `source` | `local_scorer`, `cloud_micro_inference`, or `local_fallback`. |
| `model` | Model name when micro-inference ran. |
| `response_id` | OpenAI response id when a valid cloud micro-inference result exists. |
| `current_focus` | Factual current surface. |
| `current_activity` | Local summary of current behavior. |
| `selected_workstream` | Workstream chosen as the context for continuing. |
| `return_target` | Artifact Smalltalk thinks should be opened or returned to. |
| `resume_work_target` | Actionable work target, kept separate from branch/support evidence. |
| `candidate_kind` | Kind of selected continuation candidate. |
| `last_meaningful_action` | Best local action evidence supporting the candidate. |
| `unresolved_state` | Human-readable unresolved state when present. |
| `next_action` | Suggested next step. |
| `confidence` | Numeric confidence after scoring and caps. |
| `confidence_label` | User-facing confidence bucket. |
| `evidence_anchors` | Inspectable evidence anchors. |
| `missing_evidence` | What is absent or weak. |
| `warnings` | Internal/user-relevant caveats. |
| `validation_failures` | Micro-inference or validation failures. |
| `handoff` | User-facing handoff lines. |
| `alternatives` | Top alternative candidates. |
| `generated_candidates` | Count of local candidates generated. |
| `validation_status` | `valid`, `thin_evidence`, `fallback`, etc. |

The `handoff` contains:

- `headline`
- `return_line`
- `current_focus_line`
- `last_state_line`
- `next_action`
- `why_this`
- `missing_evidence_line`
- `confidence_label`
- `user_visible_uncertainty`

The local handoff is composed even when no model is used.

## Persisted Decisions

When the decision is not served from cache, it is inserted into `continue_decisions`.

Persisted decision rows store:

- Decision id.
- Requested timestamp.
- Source.
- Current focus frame/artifact.
- Selected workstream.
- Selected candidate.
- Return target artifact.
- Confidence.
- Reason.
- Next action.
- Warnings.
- Validation status.
- Response id and model when available.
- Validation notes.
- Handoff headline and lines.
- Handoff reasons.
- Missing evidence line.
- User-visible uncertainty.

Persisting decisions matters because `open_resume_point` can later open by `continue_decision_id`, and feedback can attach to the exact decision.

## Feedback And Breadcrumbs

Smalltalk supports both inferred and explicit feedback.

Explicit feedback can be recorded through `record_continue_feedback` with kinds such as:

- `accepted`
- `rejected`
- `ignored`
- `corrected`
- `artifact_only_evidence`
- `ignored_workstream`
- `user_next_step_note`

Breadcrumbs can be added with `add_continue_breadcrumb`. They are local notes attached to a workstream and can be included in later candidate packs.

`infer_continue_feedback` looks at post-decision observations to infer whether a decision was accepted, ignored, rejected, corrected, or auto-resumed. This makes feedback part of the engine rather than only a UI affordance.

## Opening The Target

The UI should open a Continue result by calling `open_resume_point` with:

```json
{
  "continue_decision_id": "<decision id>"
}
```

`open_resume_point` resolves targets in this order:

1. Explicit `target_frame_id`, if provided.
2. `continue_decision_id`.
3. Completed cloud resume result, if available and safe.
4. Legacy local resume-card fallback.
5. Resume-query candidate fallback.
6. Smalltalk window fallback.

For the primary Continue path, step 2 is the important one.

Resolving a Continue decision:

1. Look up `continue_decisions`.
2. Join selected candidate and return target artifact.
3. Prefer candidate evidence frame id.
4. Fall back to artifact last-seen frame id.
5. Load that frame.
6. Convert it into a resolved open point.

Before automating another app, Smalltalk checks frame privacy and sensitive regions. If the frame is excluded, it focuses Smalltalk instead and returns a privacy fallback.

If the resolved frame has a browser URL, Smalltalk attempts browser automation/opening. If it cannot safely open the URL or native automation fails, it falls back to showing Smalltalk evidence.

Possible open strategies include:

- Browser open strategy when safe and possible.
- `smalltalk_privacy_fallback`
- `smalltalk_fallback`
- `smalltalk_automation_fallback`

This is why the Continue decision stores evidence ids. Opening is grounded in a real frame/artifact, not in a model-written sentence.

## What The UI Should Treat As Primary

The first screen should show one Continue answer.

Primary:

- `get_continue_decision`.
- `ContinueDecisionCard`.
- `current_focus`.
- `return_target`.
- `resume_work_target`.
- last meaningful state.
- next action.
- confidence and evidence notes.
- `Continue here`.
- `Inspect evidence`.
- collapsed correction controls.

Secondary/diagnostic:

- Raw frames.
- Timelines.
- Search.
- Screenshot inspector.
- Storage details.
- Evals.
- Workstream internals.
- Candidate scoring internals.
- Cloud resume.
- Resume-query bundles.
- Native resume cards.
- Session island internals.

Smalltalk is continuation-first. Sessions, screenshots, frames, events, bundles, and diagnostics exist to support the Continue answer, not to become the product answer.

## Legacy And Secondary Paths

### `run_cloud_resume`

`run_cloud_resume` is the older Ask OpenAI path. It:

1. Resolves a session/current frame.
2. Checks for a cached cloud result.
3. Requires `OPENAI_API_KEY`.
4. Builds a bounded resume-query bundle.
5. Runs coherence lint.
6. Builds a legacy local card.
7. Calls OpenAI.
8. Parses cloud resume JSON.
9. Enforces anchor contracts.
10. Optionally asks for targeted follow-up evidence.
11. Persists a `cloud-resume-result.json`.

This is not the core Continue engine.

### `get_native_resume_card`

`get_native_resume_card` is older local resume-card behavior. It can still be used as a fallback by `open_resume_point` when Continue/cloud targets are unusable. It should not be treated as the primary product primitive.

### `resume_query_exports`

`resume_query_exports/` contains generated stop-time bundles. They can be useful for debugging or cloud resume, but they are generated artifacts and should not be committed.

### Floating island

The floating island is a native macOS surface around capture/resume state. It may still speak older session/resume language in some paths. It is not the core semantic engine.

### Browser extension

`browser-extension/` preserves an older browser-extension prototype. It is not the active native MVP path and should not be revived for current Continue work unless explicitly requested.

## Failure And Thin Evidence Behavior

Smalltalk should say evidence is thin when it is thin.

Thin evidence can happen when:

- There are no frames.
- There are events but no openable artifacts.
- Accessibility and OCR are weak.
- The target is only a fallback surface.
- The target is privacy-restricted.
- Current focus conflicts with the likely work target.
- The last meaningful action is missing.
- The only signal is repeated duplicate state.

The correct behavior is not to hallucinate. The correct behavior is to:

- Return the best local candidate if one exists.
- Cap confidence.
- Include warnings.
- List missing evidence.
- Provide inspectable anchors.
- Fall back to evidence-only or Smalltalk display when needed.

## Implementation Truth Checklist

When changing or debugging this engine, verify these boundaries:

- `get_continue_decision` remains the core Continue engine.
- `current_focus`, `current_activity`, `return_target`, and `resume_work_target` stay separate.
- Lightweight events remain first-class evidence, not just screenshot triggers.
- Accessibility remains preferred over OCR when strong.
- OCR remains fallback/enrichment for thin or absent Accessibility.
- ScreenCaptureKit remains preferred over `screencapture` on macOS.
- Manual/session-start captures bypass dedupe.
- Event/idle captures remain budgeted and deduped.
- Smalltalk self-observation is skipped as low-value heavy evidence.
- Raw typed characters are not stored.
- Full clipboard text is not stored.
- Audio and continuous video are not captured.
- Model calls, when enabled, remain candidate-bounded and locally validated.
- Branch/support surfaces do not become default return targets unless local evidence supports that.
- Diagnostics remain secondary to the first-screen Continue answer.
