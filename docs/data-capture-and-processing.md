# Data Capture And Processing

This document describes the data capture and processing paths that exist in Smalltalk today. There are two related but separate systems:

- The native Tauri local capture app in `src-tauri/src/capture.rs`.
- The browser-extension research and resume flow in `browser-extension/`.

The native app captures local screen/application context into a local SQLite store. The browser extension captures explicit research-session evidence, builds a sanitized resume dossier, sends that dossier through a localhost proxy, and stores the returned resume card.

## Native Tauri Local Capture

The native capture system is controlled through Tauri commands exposed from `src-tauri/src/capture.rs`:

- `start_capture` starts a background capture loop.
- `stop_capture` stops the background worker.
- `capture_once` captures a single manual frame.
- `capture_status` reports runtime state and tool availability.
- `search_captures` searches stored frame text.
- `get_frame_image` returns the stored screenshot for a frame.
- `clear_captures` removes stored frames/screenshots and recreates the capture database.

### Capture Triggers

When background capture starts, the worker immediately attempts an initial manual capture. After that, it listens for native UI events and falls back to periodic idle capture.

The trigger priority is:

1. Manual capture: explicit user action through `capture_once`, or the initial capture when the worker starts.
2. Event-triggered capture: native macOS events are queued and captured after a short settle delay.
3. Idle capture: periodic fallback when no event-triggered capture has been stored recently.

Native events are produced by the Swift helper embedded from `src-tauri/scripts/capture_events.swift`. Raw helper events are normalized before capture:

| Raw event | Stored trigger | Settle delay |
| --- | --- | --- |
| `app_switch` | `app_switch` | 300 ms |
| `window_focus` | `window_focus` | 300 ms |
| `accessibility_change` | `accessibility_change` | 300 ms |
| `click` | `click` | 200 ms |
| `key_down` | `typing_pause` | 500 ms |
| `scroll` | `scroll_stop` | 400 ms |
| `clipboard` | `clipboard` | 200 ms |

Only one pending event trigger is kept at a time. A newer trigger replaces the previous pending trigger before capture. Captures are also rate-limited by `MIN_CAPTURE_INTERVAL`, so frequent UI events collapse into fewer stored frames.

### Screenshot Capture

Every native frame starts with a screenshot. The app calls macOS `screencapture` directly:

```text
/usr/sbin/screencapture -x -t jpg <snapshot_path>
```

The image is written under the app capture data directory, grouped by day. The screenshot file name uses the capture timestamp and ends in `_main.jpg`. After writing, Smalltalk hashes the screenshot bytes into `image_hash`; this hash is used for deduplication.

If `screencapture` fails, the capture fails with a message that usually points to missing Screen Recording permission.

### Accessibility Capture

After the screenshot, the app collects foreground application context through macOS Accessibility. This is the preferred text and metadata source.

Priority order:

1. Native Swift Accessibility helper from `src-tauri/scripts/accessibility_snapshot.swift`.
2. AppleScript fallback embedded in `src-tauri/src/capture.rs`.

The Accessibility context can include:

- Foreground app name.
- Front window name.
- Browser URL for Safari, Chrome, Brave, Edge, Arc, Chromium, Vivaldi, and Opera.
- Document path from `AXDocument`.
- Accessibility nodes with role, depth, and text.
- Flattened accessibility text.
- Any capture error from the helper path.

The Swift helper is used first. If it returns useful app/window/URL/text/node signal, that result is accepted. If it fails or returns no useful signal, the AppleScript fallback runs. If both paths fail to produce signal, the stored frame may include diagnostic text instead of normal content.

### OCR Capture

OCR is not the primary text source. It is an enrichment and fallback path.

OCR runs only when either:

- Accessibility text is missing, or
- Accessibility text is considered thin.

Accessibility is treated as thin when it looks like a canvas-heavy app, has very little text, or mostly contains browser/UI chrome roles instead of content roles. The current canvas-heavy patterns include Google Docs, Google Sheets, Google Slides, Figma, Excalidraw, Miro, Canva, and tldraw.

OCR priority order:

1. Apple Vision OCR through the Swift helper from `src-tauri/scripts/vision_ocr.swift`.
2. `tesseract`, if available in `PATH`.

On macOS, Apple Vision is attempted first. If Vision returns text, that result wins. If Vision returns empty text and Tesseract is installed, Tesseract is used as a fallback. If Vision fails and Tesseract is installed, Tesseract is used. Outside macOS, Tesseract is the OCR path.

### Text Source Resolution

After Accessibility and optional OCR complete, Smalltalk resolves the frame text source:

| Inputs | Stored `text_source` | Stored `full_text` |
| --- | --- | --- |
| Accessibility text exists and is not thin | `accessibility` | Accessibility text |
| Accessibility text exists and OCR text exists because Accessibility was thin | `hybrid` | Accessibility text plus OCR text |
| No Accessibility text, OCR text exists | `ocr` | OCR text |
| Neither source has text | `null` | `null`, unless diagnostics are later attached |

This means Accessibility has priority when it is strong enough. OCR only replaces Accessibility when Accessibility is absent, and combines with it when Accessibility exists but appears weak.

### Deduplication

Event and idle captures use deduplication. Manual captures do not.

Deduplication compares both:

- `image_hash`, from the screenshot bytes.
- `content_hash`, from resolved `full_text` when text exists.

A deduped capture deletes the just-created screenshot file and does not insert a frame. This avoids filling the store with repeated idle/event samples when neither pixels nor text changed.

### Storage And Search

Native capture data is stored under the repo-local `captured/` folder. The important paths are:

- `snapshots/`: day-bucketed screenshot JPG files.
- `smalltalk-capture.sqlite`: SQLite database.
- `helpers/`: generated Swift helper source/binaries for Accessibility, OCR, and event capture.

The SQLite database stores:

- `frames`: frame metadata, screenshot path, app/window/browser/document fields, trigger, selected text source, Accessibility text, Accessibility tree JSON, resolved full text, hashes, and timestamps.
- `ocr_text`: OCR text and OCR JSON payloads linked to a frame.
- `frames_fts`: SQLite FTS5 index over full text and metadata fields.

Search uses `frames_fts MATCH` and returns frame rows ordered by recency. Status reads the latest frame, frame counts, skipped sample counts, tool availability, data directory, and database path. Clearing captures replaces the database and removes snapshot files so the next run starts from a clean slate.

## Browser Extension Research Capture

The browser extension is the return-to-task system. It captures an explicit research session after the user starts it from the popup. Its goal is not to record everything on the machine; it records enough browser evidence to help the user resume the original task.

### Session Start

`Start research` sends `START_SESSION` to the background script. The background script:

1. Reads the active web tab.
2. Creates a `ResearchSession` with `status: "active"`.
3. Stores the origin tab ID, origin URL, origin title, and counts.
4. Creates the first `PageVisit` for the origin page.
5. Sets `activeSessionId` in extension storage.
6. Requests an immediate page snapshot from the content script.

Capture starts only after this explicit session start.

### Page Snapshots

The content script handles `CAPTURE_PAGE_SNAPSHOT` and returns a `PageSnapshot`. A snapshot includes:

- Current URL and title.
- Inferred app type.
- Visible text from readable page blocks.
- Active message when the page type supports it.
- Selected text when the selection is long enough.
- Center text near the middle of the viewport.
- Text chunks with heading, selector, scroll position, word offsets, and text quotes.
- Current `scrollY`.
- Capture timestamp.

Readable blocks are collected mainly from `article`, `main`, or `body`, and then from headings, paragraphs, list items, blockquotes, and sections. The chunker filters out very short or link-heavy text, normalizes whitespace, preserves heading context, and creates stable quotes for matching.

When the background script records a snapshot, it replaces the previous chunks for that visit, stores the new chunks with `attentionScore: 0`, adds a `snapshot` attention event, refreshes session counts, and rescores chunks.

If the snapshot URL matches the session origin URL, the session's `originSnapshot` is updated with the latest origin evidence.

### Attention Events

The content script captures these attention events while capture is enabled:

| Event | Mechanism |
| --- | --- |
| `snapshot` | Added when a page snapshot is recorded. |
| `scroll` | Throttled scroll handler records center readable text and scroll progress. |
| `selection` | Mouse/key selection handler records selected text and nearby readable text. |
| `cursor_dwell` | Throttled mousemove handler records readable text under the cursor. |
| `link_click` | Capturing click listener records link target and nearby readable text. |
| `visibility` | Visibility changes record visible/hidden state; hidden pages also send a snapshot. |

Attention events are sent to the background script, assigned IDs, attached to the current active session, matched to a visit, optionally matched to a chunk, stored, and then used to rescore chunks.

### Navigation Graph

The background script listens to:

- `browser.webNavigation.onCommitted`
- `browser.webNavigation.onHistoryStateUpdated`

Before navigation, link clicks are stored briefly in `pendingClicksByTab`. When navigation arrives, the background script uses the pending click and Chrome transition type to classify how the new visit opened:

- `clicked`
- `typed`
- `reload`
- `other`

It then creates or updates a `PageVisit`, stores a `NavigationEdge` from the source URL/visit to the destination URL/visit, refreshes counts, and requests a fresh snapshot for the destination tab.

### Extension Storage Shape

The extension stores raw session evidence in browser extension storage:

- `sessions`: research sessions, including active/stopped state and origin snapshot.
- `visits`: pages seen during a session.
- `events`: attention events.
- `edges`: navigation edges between visits.
- `chunks`: readable page chunks with attention scores.
- `cards`: resume cards returned by analysis.
- `activeSessionId`: the currently active session pointer.

When the user stops a session, the session is marked stopped, `activeSessionId` is cleared, and content scripts are told to disable capture.

## Priority And Scoring

Smalltalk has separate priority rules for native capture and extension resume capture.

### Native Priority

Native capture prioritizes explicit and eventful moments over background sampling:

1. Manual capture always captures without dedupe.
2. Native event-triggered capture runs after a short settle delay and uses dedupe.
3. Idle capture runs only as a fallback and also uses dedupe.

For text extraction:

1. Strong Accessibility text is preferred.
2. Thin or missing Accessibility text allows OCR.
3. Apple Vision OCR is preferred over Tesseract.
4. Final text is resolved as `accessibility`, `hybrid`, `ocr`, or no text.

### Extension Attention Weights

The extension scores chunks from attention events. Current base weights are:

| Event kind | Base weight |
| --- | ---: |
| `selection` | 3.2 |
| `link_click` | 1.8 |
| `cursor_dwell` | 1.1 |
| `scroll` | 0.6 |
| `visibility` | 0.5 |
| `snapshot` | 0.25 |

Selections can gain up to 2 extra points based on selected-text length. Scroll events can gain up to 0.8 extra points from scroll progress.

Event-to-chunk matching uses:

1. Text quote matching first. A quote match needs a score of at least 0.45.
2. Scroll position fallback. If there is no usable quote match, the nearest chunk by scroll position is used.

Chunk scores are recomputed after snapshots and attention events. Branch pages are ranked by page attention, but branch pages are evidence only. Resume targets are built from origin-page anchors, not from branch pages.

## Resume Dossier Processing

When the user asks to resume, the background script runs `ANALYZE_RESUME`.

### Dossier Build

The background script first asks known session tabs for fresh snapshots, then builds a `ResumeDossier` from extension storage.

The dossier includes:

- Mode: `origin_only`, `away_from_origin`, or `returned_to_origin`.
- Session metadata.
- Origin snapshot.
- Departure event from the last origin-page link click.
- Return event when branch visits exist and the latest visit is back on the origin URL.
- Branch visits ranked by attention score.
- Navigation edges.
- Candidate origin anchors.
- Instrumentation warnings.
- Evidence lines.

Before text enters the dossier, sensitive text is redacted. The redactor replaces emails, phone-like numbers, common secret/token prefixes, and long token-like strings.

### Candidate Origin Anchors

Candidate resume targets are built only from chunks whose URL matches the origin URL. The candidate order is:

1. First unread origin chunk after the last strong origin attention signal.
2. Top-attention origin chunk.
3. First origin chunk.

The last strong origin attention signal is the latest origin `selection`, `cursor_dwell`, or `link_click` with a usable text quote. Duplicate candidates are removed.

This is the key resume-target rule: side pages can influence what the resume card says, but the actual `resumeTarget` should point back to the origin page when an origin anchor exists.

### Local Proxy And OpenAI Processing

The extension sends the sanitized dossier to:

```text
http://localhost:8787/api/resume
```

The API key is not stored in the extension. The local proxy in `browser-extension/server/inference-proxy.ts` reads `OPENAI_API_KEY` from the environment or `.env`, validates the dossier shape, and calls the OpenAI Responses API.

The proxy asks for a strict JSON response using `resumeCardSchema()`. The returned card must include:

- Original intent.
- Journey summary.
- New knowledge.
- Summary.
- Confidence.
- Evidence.
- Branch findings.
- Suggested next message.
- Instrumentation warnings.
- Resume target or `null`.

The system prompt tells the model that branch pages are evidence only and that, when the user returned to origin, the resume target must equal the origin URL. If no candidate origin anchors exist, the model should set `resumeTarget` to `null` and explain the missing instrumentation.

After OpenAI returns JSON, the proxy normalizes the resume card against the dossier. The extension stores the card in `cards` and shows it in the popup.

### Opening And Highlighting A Resume Target

When the user opens a resume target, the background script finds or creates a tab for `target.url`, focuses the tab/window, and sends `APPLY_RESUME_HIGHLIGHT` to the content script.

The content script tries to find the target element in this order:

1. Exact selector plus text-match validation.
2. Best readable paragraph/list/blockquote/section by text quote.
3. Heading fallback, using the element after a matching heading.

If an element is found, it scrolls into view and receives the `smalltalk-resume-target` class. If no element is found, the page scrolls to `target.scrollY` when available and shows the resume overlay anyway.

## Privacy Boundaries

The native Tauri capture store is local: screenshots, Accessibility text, OCR text, and SQLite data are written under the app's local data directory.

The browser extension keeps raw session evidence in browser extension storage. OpenAI does not receive the raw extension store. It receives only the compact, sanitized dossier through the local proxy at `http://localhost:8787/api/resume`.

The OpenAI API key lives in the local proxy environment, not in the browser extension. If the proxy or API key is unavailable, the extension reports an error instead of pretending an AI resume card was generated.
