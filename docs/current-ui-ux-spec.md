# Smalltalk Current UI/UX Specification

Last verified: 2026-07-07

This document is the current UI/UX source of truth for Smalltalk as implemented in the live desktop codebase. It describes the product surface that exists now, not the ideal future product and not the older session-recorder UI model.

Primary source files:

- `src/App.tsx` - React/Tauri application shell, Continue surface, Inspect mode, evidence panels, state derivation, and Tauri command calls.
- `src/App.css` - visual system, layout, responsive behavior, motion tokens, and component styling.
- `src-tauri/macos/SessionIslandPanel.swift` - native macOS floating Session Island presentation, motion, panel behavior, and Swift-side UI state mapping.
- `src-tauri/src/session_island.rs` - Rust snapshot/action bridge for the native island.

Historical context:

- `docs/continue-ui-reality.md` is an older audit. It is useful for understanding why the app moved away from a session/debug-heavy product surface, but it is no longer a complete description of the current UI.
- Current implementation now has a Continue-first default view, a companion memory panel, a secondary evidence disclosure, and a separate `Inspect` mode for diagnostic surfaces.

## 1. Product Model

Smalltalk is currently framed as a local-first continuation product.

The visible product promise is:

1. Keep local memory running or paused in the background.
2. Observe local evidence without storing raw typed characters or full clipboard contents.
3. Return one continuation answer that separates what is currently focused from where the user should actually continue.
4. Make evidence inspectable without making screenshots, frames, raw events, or diagnostics the first screen.

The app is not meant to feel like a recorder, a session browser, or a screenshot search product on first load. Capture sessions, frames, screenshots, raw events, timeline rows, workstream internals, evals, storage metrics, and local paths are still present, but they live behind `Inspect` or `Why this?`.

The primary product object in the UI is the current Continue answer:

- The work the user was doing.
- The return target.
- The last meaningful state.
- The next action.
- Evidence quality and uncertainty.
- Feedback/correction when the answer is wrong.

## 2. Main React App Shell

The root element is:

```tsx
<main className={`capture-shell ${viewMode === "developer" ? "developer-mode" : "continue-mode"}`}>
```

The name `capture-shell` is legacy. The current visual and UX hierarchy is Continue-first.

### 2.1 Root Layout

The shell is a full-height desktop app:

- `height: 100dvh`
- `overflow: hidden`
- `padding: 14px`
- `display: flex`
- `flex-direction: column`
- `gap: 14px`

The page has a fixed header and one scrollable content region:

- Header: `.capture-topbar`
- Scroll region: `.app-scroll`

This fixed-header plus one-scroll-region pattern is important. Avoid adding competing page-level scroll containers unless they are inside developer diagnostics.

### 2.2 Background

The app background uses layered subtle gradients over `--bg`:

```css
background:
  linear-gradient(180deg, rgba(255, 255, 255, 0.86), rgba(255, 255, 255, 0) 340px),
  linear-gradient(135deg, rgba(18, 106, 90, 0.07), transparent 34%),
  linear-gradient(315deg, rgba(93, 86, 140, 0.065), transparent 31%),
  var(--bg);
```

The current product does not use a dark app body, dramatic gradients, large hero imagery, or marketing layout. It is a calm operational product UI.

### 2.3 Topbar Anatomy

The topbar is `.capture-topbar`.

Desktop grid:

```css
grid-template-columns: minmax(190px, 1fr) auto minmax(180px, 1fr);
```

It contains:

1. Identity block.
2. View switch.
3. Status/meta area.

Identity block:

- `.brand-mark`: square `S`, 34 x 34 px, 8 px radius, accent-soft gradient.
- `.product-kicker`: `Smalltalk`.
- Main heading:
  - Continue mode: `Continue`
  - Developer/Inspect mode: `Evidence inspection`

View switch:

- `.view-switch`
- Two buttons:
  - `Continue`
  - `Inspect`
- The active button gets white raised background and subtle shadow.

Topbar meta in Continue mode:

- Single `.memory-dot`
- Text comes from `memoryCueLabel`:
  - `Needs attention` when `status.last_error` exists.
  - `Memory on` when `status.running` is true.
  - `Paused with evidence` when not running but evidence exists.
  - `Ready to start` when there is no evidence yet.
- Dot is active only when `status.running` is true.

Topbar meta in Inspect mode:

- Three `StatusPill` components:
  - `Local memory`: `Active`, `Paused`, `No evidence`, `Permission issue`, or `Deleting local memory`.
  - `Evidence age`: relative latest evidence age, such as `just now`, `5m ago`, or `No evidence yet`.
  - `Continue`: `Updating`, `New evidence`, `Current`, or `Ready`.

## 3. View Modes

The app has exactly two user-visible modes:

```ts
type ViewMode = "continue" | "developer";
```

The UI labels the developer mode as `Inspect`.

### 3.1 Continue Mode

Continue mode is the default:

```ts
const [viewMode, setViewMode] = useState<ViewMode>("continue");
```

In Continue mode:

- The main screen shows `.continue-home`.
- The primary stage is `.continue-stage`.
- `ContinueEvidencePanel` can appear below the primary stage only after the user chooses `Why this?`.
- The heavy diagnostics panel is not rendered.

CSS behavior:

```css
.continue-mode .app-scroll {
  align-content: center;
  justify-items: center;
  padding: 10px 0 44px;
}
```

### 3.2 Inspect Mode

Inspect mode is entered with the topbar `Inspect` button.

On entering Inspect mode, the UI refreshes:

- Workstreams.
- Search results.
- Recent timeline.
- Memory diagnostics.
- Selected workstream detail.

In Inspect mode:

- `.developer-mode .continue-home { display: none; }`
- The developer diagnostics surface is rendered.
- Search, frames, screenshots, raw events, overlays, storage, evals, and workstream internals are visible.

Inspect mode is not the product default. It is an evidence/debug workspace.

## 4. Continue Mode Primary Stage

Continue mode uses:

```tsx
<section className="continue-home" aria-label="Continue">
  <div className="continue-stage">
    <ContinuationAnswer />
    <ContinueCompanionPanel />
  </div>
</section>
```

`.continue-home`:

- Width: `min(100%, 1120px)`.
- Centered via the parent `.app-scroll`.
- Single-column wrapper.

`.continue-stage`:

- Two columns on desktop:
  - Main Continue answer: `minmax(0, 1fr)`
  - Companion panel: `minmax(260px, 310px)`
- Gap: 14 px.
- Collapses to one column below 1240 px.

## 5. Continue Answer Card

The main card is `ContinuationAnswer`.

Outer class:

```tsx
continue-card continuation-answer
```

Additional states:

- `empty` when no decision exists.
- `low-confidence` when `decision.confidence < 0.55` or when user-facing text looks like internal IDs/metadata.

The visual shell is `.answer-shell`:

- Border radius: 6 px.
- Padding: 34 px desktop, 24 px under 860 px, 20 px under 560 px.
- Inner white/highlight border.
- Warm white panel gradient.

### 5.1 Empty State

Rendered when `continueDecision` is null.

Eyebrow text:

- `Memory on` when memory is running.
- `Evidence ready` when evidence exists.
- `No local memory yet` when no evidence exists.

Small label above H2:

- `Ready to answer` when evidence exists.
- `Turn on local memory` when evidence does not exist.

H2 text comes from `continuePrimaryMessage`:

- `Start local memory to make Continue useful.`
- `Smalltalk is watching locally. Continue when there is enough evidence.`
- `Ready to find where to continue.`
- Or the selected workstream title when a decision exists elsewhere.

Supporting body:

- With evidence: `Continue can answer from local evidence without requiring you to stop or export anything first.`
- Without evidence: `Smalltalk watches local context, keeps privacy boundaries visible, and gives one continuation when you ask.`

Assurance list:

- `Current focus stays separate from the return target.`
- `Thin evidence is shown honestly.`
- `Raw typed characters and full clipboard contents are excluded.`

Actions:

- Primary button:
  - Idle: `Find where to continue`
  - Busy: `Finding where to continue`
  - Calls `get_continue_decision` with `writeAudit: true`.
- Secondary button appears only when there is no evidence:
  - Idle: `Start local memory`
  - Busy: `Starting`
  - Calls `start_capture`.

### 5.2 Decision State

Rendered when `continueDecision` exists.

Eyebrow:

- `New local evidence since this answer` when stale.
- `Best available answer` when low confidence or target text looks internal.
- `Continue answer` otherwise.

Provenance pill:

- `AI-assisted` when `decision.source === "cloud_micro_inference"` and `response_id` exists.
- `Local fallback` when `decision.source === "local_fallback"`.
- `Local only` otherwise.

Provenance visual tone:

- `ai`: blue/info styling.
- `fallback`: warm warning styling.
- `local`: neutral gray styling.

Hero:

- Small text: `You were working on`.
- H2: user-facing workstream line.
- The H2 is capped at `max-width: 18ch`, 46 px, `line-height: 1`.
- Internal IDs are suppressed by `safeProductLine` and `isInternalFacingText`.

Target block:

- Label:
  - `Best available place to continue` when low confidence.
  - `Continue at` otherwise.
- Strong line: target label.
- Small line: target meta, such as artifact kind and openability.

State grid:

- `Last meaningful state`
- `Next action`

Why strip:

- `.answer-why-strip`
- Up to 3 chips from `handoff.why_this` or presentation decision reason.

Current focus line:

- Rendered only when current focus exists and differs from return target.
- Text pattern: `Current focus: <strong>...</strong>`
- This preserves the product doctrine that current focus and return target are not necessarily the same.

Uncertainty line:

- `.answer-uncertainty`
- Rendered for low confidence or when a missing-evidence/user-visible uncertainty line exists.
- Default low-confidence copy: `Evidence is thin, so this is the best available local recommendation.`

### 5.3 Continue Answer Actions

Primary action:

- Button label:
  - `Opening` while `open_resume_point` is busy.
  - `Continue here` when target is directly openable.
  - `Needs evidence` when target is not directly openable.
- Disabled unless:
  - `busyAction === null`
  - target openability is `openable`
  - target has a `browser_url` or `document_path`.
- Calls `open_resume_point` with:
  - `continue_decision_id`
  - `target_artifact_id`
  - `strict_continue_target: true`

Secondary actions:

- `Why this?`
  - Opens `ContinueEvidencePanel`.
  - Loads the first evidence frame if possible.
- `Refresh`
  - Busy label: `Refreshing`
  - Calls `get_continue_decision` with `writeAudit: true`.

Correction control:

- Text button: `Wrong target?`
- Opens `.continue-correction-panel`.
- Correction buttons:
  - `Mark wrong target`
  - `Show alternatives` / `Hide alternatives`
  - `This was only evidence`
  - `Ignore workstream`
- Feedback kinds sent to backend:
  - `rejected`
  - `artifact_only_evidence`
  - `ignored_workstream`
  - alternatives use `corrected`.

Alternative list:

- Shows up to 4 alternatives only when alternatives are expanded.
- Each row has:
  - Candidate title/reason.
  - `Use this` button.
- Selecting an alternative records feedback and may open its evidence frame via `open_resume_point` with `target_frame_id`.

Open result:

- `.continue-open-result`
- Header: `Open target`
- Possible result messages:
  - `Opened the selected target.`
  - `Could not open directly; focused Smalltalk instead.`
  - `Attempted to open the selected target.`
  - Or a productized warning.

## 6. Companion Panel

The right-side panel is `ContinueCompanionPanel`.

Outer class:

```css
.continue-companion
```

Desktop behavior:

- Sticky at top of `.continue-stage`.
- Width constrained by the second grid column.
- Border radius: 8 px.
- Padding: 6 px.
- Soft shadow.

Responsive behavior:

- Below 1240 px, it becomes non-sticky and flows under the main answer.

### 6.1 Memory Orb

The panel starts with `.companion-orb`.

Tone class:

- `attention` when `status.last_error` exists.
- `active` when memory is running.
- `paused` when evidence exists but memory is not running.
- `quiet` when no evidence exists and memory is off.

Visual mapping:

- `active`: accent green dot with pulsing animation.
- `paused`: warning amber dot.
- `attention`: red dot.
- default/quiet: muted gray dot.

Animation:

- `memoryPulse` scales the dot from 1 to 1.12 and back.
- Duration: 2800 ms.
- Uses `--ease-drawer`.
- Disabled by reduced-motion media query.

### 6.2 Memory Copy

Headline:

- `Memory needs attention`
- `Local memory is on`
- `Local memory is paused`
- `Local memory is off`

Detail:

- Error text when there is an error.
- Running: `Smalltalk is maintaining context quietly in the background.`
- Paused with evidence: `You can still ask Continue from the evidence already stored locally.`
- Off: `Turn it on once, keep working, then ask Continue when you need a return point.`

Small top label uses `statusLabel`:

- `Deleting local memory`
- `Permission issue`
- `Active`
- `Paused`
- `No evidence`

### 6.3 Readiness Facts

`.companion-facts` is a two-column grid on desktop, four columns under 860 px, and one column under 560 px.

It shows:

- `Continue`: `Updating`, `Current`, `New evidence`, `Ready`, or `Waiting`.
- `Evidence`: relative evidence age.
- `Workstreams`: `continueMemory.counts.workstreams`.
- `Artifacts`: `continueMemory.counts.artifacts`.

### 6.4 Privacy Boundary

The panel always shows:

- Label: `Privacy boundary`
- Body: `Raw typed characters and full clipboard contents are not stored.`

This is a core UX promise and should not be removed casually.

### 6.5 Companion Actions

Actions:

- If running:
  - Button: `Pause memory`
  - Busy: `Pausing`
  - Calls `stop_capture`.
- If not running:
  - Button: `Turn on memory`
  - Busy: `Starting`
  - Calls `start_capture`.
- `Add evidence`
  - Disabled when memory is not running.
  - Calls `capture_once`.
- `Refresh Continue`
  - Text button.
  - Disabled with no evidence.
  - Calls `get_continue_decision` with `writeAudit: true`.

## 7. Continue Evidence Panel

The evidence panel is rendered when `evidenceOpen` is true.

It is intentionally secondary. It can appear in Continue mode and Inspect mode, but it is not shown by default.

Outer class:

```css
.continue-evidence-panel
```

Width:

```css
width: min(100%, 1120px);
```

### 7.1 Empty Evidence State

Rendered if the panel is opened before a decision exists.

Header:

- Kicker: `Continue evidence`
- H2: `Run Continue after local memory has evidence.`
- Button: `Close`

### 7.2 Evidence Facts

When a decision exists, the panel shows:

- Header kicker: `Continue evidence`
- H2: workstream title, target label, or `Selected workstream`
- Close button.

Facts:

- `Why this workstream`
- `Return target`
- `Current screen`
- `Last meaningful action`
- `Unresolved state`
- `Evidence`
- `Inference`

Fallback text:

- `Selected by local continuation scoring.`
- `No return target returned.`
- `No current screen returned.`
- `No action returned.`
- `No unresolved state returned.`
- `No missing evidence called out.`

### 7.3 Evidence Anchor Preview

Right side:

- Section title: `Evidence anchor`
- Shows selected frame title or `No evidence selected`.
- If `selectedFrame` and `imageData` exist, renders screenshot preview inside `.anchor-image`.
- If no preview:
  - Strong: `No preview loaded`
  - Body: selected frame title or `No evidence preview is selected.`

Image loading is gated:

- Full image variant loads only when a frame is selected and either diagnostics are open or evidence panel is open.

### 7.4 Evidence Notes

Warnings combine:

- `decision.missing_evidence`
- `decision.warnings`
- `decision.validation_failures`

They are productized through `productizeInternalLabel`.

Visible group:

- Title: `Evidence notes`
- Up to 4 items shown by `WarningGroup`.

## 8. Inspect Mode

Inspect mode is the diagnostic/evidence workspace. It is deliberately not the first screen.

Top-level section:

```tsx
<section className="developer-panel diagnostics-panel" aria-label="Developer diagnostics">
```

Header:

- Small label: `Developer diagnostics`
- Strong text: `Frame inspector, search, raw events, and local evidence substrate`

### 8.1 Inspect Header Controls

Primary button:

- `Continue`
- Busy: `Finding`
- Calls `get_continue_decision` with `writeAudit: true`.

Memory menu:

- Summary: `Memory`
- Menu buttons:
  - `Start local memory` / busy `Starting`
  - `Pause local memory` / busy `Pausing`
  - `Capture evidence now` / busy `Capturing`
  - `Delete local memory` / busy `Deleting`

Menu behavior:

- Uses a controlled `<details>`.
- Closes on outside pointer down.
- Closes on Escape.
- Closes on app scroll.

Delete confirmation:

```text
Delete all stored frames and screenshots? This creates a clean slate.
```

Developer reset confirmation:

```text
Reset local memory for developer testing? This clears frames, events, derived Continue rows, snapshots, and generated debug exports.
```

### 8.2 Diagnostics Workspace

Grid:

```css
.diagnostics-workspace {
  grid-template-columns: minmax(300px, 0.9fr) minmax(320px, 1fr);
}
```

It contains:

- Workstream list.
- Breadcrumb card.
- Workstream detail panel.

Under 1240 px it collapses to one column.

### 8.3 Workstream List

Component: `WorkstreamList`.

Outer label:

- `Workstreams`
- Subtitle: `Recent continuation candidates, not sessions.`
- Button: `Refresh`

Empty state:

- Strong: `No workstreams yet`
- Body: `Run Continue after local memory has evidence.`

Rows are grouped by `workstream.state`.

Each row shows:

- Title: `title_candidate`, `primary_artifact_title`, or `Recent workstream`.
- Secondary: primary artifact or `unresolved`.
- Small meta: state, confidence, last active time.

The active row receives `.active`.

### 8.4 Breadcrumb Note

Card label:

- H2: `Leave a next-step note for later`
- Body: `Attach a short local cue to the selected workstream.`

Textarea:

- Max length: 240.
- Placeholder: `e.g. check the failing test, then update the parser`
- Disabled if no selected workstream or while busy.

Saving calls `add_continue_breadcrumb` with:

- `workstream_id`
- text sliced to 240 chars
- `source: "desktop_ui"`

After saving, the UI also records feedback kind `user_next_step_note`.

### 8.5 Workstream Detail

Component: `WorkstreamDetailPanel`.

Empty state:

- Label: `Workstream detail`
- Body depends on missing detail or error.

Full state includes:

- Workstream summary metrics.
- Target/current-focus blocks.
- Feedback bar.
- Artifact role groups.
- Candidate targets.
- Episodes and actions.
- Evidence anchors and feedback.

Feedback buttons:

- `Correct target`
- `Wrong target`
- `Only evidence`
- `Ignore workstream`

Artifact rows:

- Show display title/path/app/window.
- Include kind, openability, privacy status, and reason.
- Button: `Inspect evidence`

Candidate rows:

- Show target title or candidate kind.
- Show reason, confidence, and openability.
- Button: `Continue from this`

Episodes:

- Show summary label or dominant action kind.
- Show episode state, primary artifact, start time.
- Button: `Inspect frame`
- Show up to 8 actions per episode.

Evidence anchors:

- Frames.
- Actions.
- Episodes.
- Artifacts.

Feedback list combines:

- Feedback events.
- Breadcrumbs.

### 8.6 Memory Diagnostics Panel

Section label:

- H3: `Local memory storage`
- Subtitle: `Developer-only retention, cleanup, and budget readout`

Actions:

- `Rebuild Continue` / busy `Rebuilding`
- `Refresh diagnostics`
- `Preview cleanup` / busy `Checking`
- `Apply cleanup` / busy `Cleaning`
- `Dev reset` / busy `Resetting`

Metrics shown when diagnostics exist:

- Database bytes.
- Snapshots bytes.
- Safe exports bytes.
- Cleanup potential.
- Frames.
- Events.
- Protected frames.
- Low-value duplicates.
- Self-capture.
- Heavy stored.
- Heavy skipped.
- Event signals.
- Cache hits.
- Oldest frame.
- Last cleanup.

Facts:

- Captured root.
- Database path.
- Heavy capture budget.
- Heavy evidence rows.
- Continue objects.
- Runtime diet counters.
- Continue calls.
- Cleanup result.

Empty state:

- `Open diagnostics or refresh to inspect local memory storage.`

### 8.7 Search

Form class:

- `.search-form developer-search`

Input:

- Placeholder: `Search captured evidence`
- Aria label: `Search captured evidence`

Button:

- `Search evidence`

Command:

- `search_captures`
- Args: query, limit `48`, session id.

### 8.8 Health Strip

Section label:

- `Capture health`

Status pills:

- `State`
- `Session`
- `Signals`
- `Frames`
- `Events`
- `Transitions`
- `Units`
- `Total sessions`
- `Latest`
- `Screen`
- `A11y`
- `OCR`

This is diagnostic. It should not migrate back into the Continue-first default screen.

### 8.9 Continue Eval Panel

Section label:

- `Continue eval diagnostics`

Header:

- H3: `Continue eval`
- Subtitle: `Developer-only scoring and validation metrics`

Button:

- `Run eval`
- Busy: `Running`

Metrics after eval:

- Cases.
- Target correct.
- Recall@k.
- MRR.
- Focus false positive.
- Hallucinated artifacts.
- Validation fallback.

Empty state:

- `Run the built-in Continue fixture set to inspect product-quality metrics.`

### 8.10 Inspector Grid

The frame inspector area is:

```css
.inspector-grid {
  grid-template-columns: minmax(300px, 360px) minmax(420px, 1fr) minmax(320px, 410px);
  height: min(740px, calc(100dvh - 180px));
  min-height: 560px;
}
```

It contains:

1. Timeline pane.
2. Viewer pane.
3. Evidence pane.

At <=1240 px:

- Timeline and viewer use two columns.
- Evidence pane spans full width.

At <=860 px:

- All three stack in one column.

### 8.11 Timeline Pane

Label:

- `Captured frames`

Heading:

- H2: `Evidence timeline`
- Subtitle:
  - `Filtered frames` when search query exists.
  - `Most recent local captures` otherwise.
- Count: `results.length`

Frame rows show:

- Preview thumbnail.
- Capture time.
- Capture trigger.
- Frame title.
- Text snippet.
- Badges:
  - `screen`
  - capture provider, such as `screen_capture_kit`.
  - text source.
  - privacy status.
  - `transition`.

Empty state:

- `No matching evidence` when frames exist and query has no results.
- `No captured frames yet` otherwise.
- Body for no query: `Start a session to collect screenshots, events, text sources, and missing-signal checks.`

Raw event stream:

- H3: `Raw event stream`
- Shows up to 8 recent events.
- Empty text: `No raw events in the last 10 minutes.`

### 8.12 Viewer Pane

Label:

- `Frame inspector`

Toolbar:

- Kicker: selected frame capture trigger or `waiting`.
- H2: selected frame title or `No frame selected`.
- Trust badge:
  - `complete`
  - `partial`
  - `thin`
  - `unverified`

Viewer stage:

- Shows full screenshot image when selected frame and image data exist.
- Shows `Loading screenshot` when selected frame exists but image is still loading.
- Shows `No frame selected` otherwise.

Screenshot stage:

- Uses actual pixel aspect ratio via `stageStyle`.
- Max width considers viewport height:

```css
width: min(100%, 1120px, calc((100dvh - 246px) * var(--frame-aspect, 1.6)));
```

Overlay modes:

```ts
type OverlayMode = "units" | "ocr" | "ax" | "privacy";
```

Overlay toolbar buttons:

- `Units`
- `OCR`
- `AX`
- `Privacy`

Legend:

- content units: accent green.
- OCR spans: info blue.
- AX nodes: warning amber.
- privacy regions: bad red.

### 8.13 Evidence Pane

Label:

- `Verification drawer`

Cards:

1. Verification card.
2. Selected evidence signal card.
3. Detail drawer.

Verification card:

- H2: `Last capture`
- Subtitle: selected capture time or `Nothing selected`.
- Badge: selected text source or `visual`.
- Signals:
  - Screenshot.
  - AX.
  - OCR.
  - Units.
  - Window graph.
  - Transition.
- Missing state:
  - `Missing signals`
  - Up to 5 missing signals.
- Complete state:
  - `All core verification signals are present for this frame.`

Selected evidence signal card:

- H2: `Selected evidence signal`
- Subtitle: `Diagnostic context only; Continue decides from workstreams.`
- Facts:
  - Current surface.
  - Likely object.
  - Transition.
  - Strongest text.

Detail drawer tabs:

```ts
type EvidenceTab = "text" | "events" | "context" | "paths";
```

Tab buttons display lowercase labels:

- `text`
- `events`
- `context`
- `paths`

Text tab:

- Shows up to 6 content-unit buttons.
- Shows raw selected text in a `pre`.
- Empty text: `No text stored for this frame.`

Events tab:

- Shows linked events.
- Shows transitions.
- Empty linked events state:
  - `No raw event linked`
  - `Manual captures may not have event provenance.`

Context tab:

- Shows app contexts.
- Shows up to 8 content units.
- Clicking a content unit switches overlay mode to `units` and highlights that box.

Paths tab:

- Snapshot.
- Window crop.
- Database trigger id.
- Session id.
- App bundle.
- Capture provider.
- SCK scope.
- URL/document.

## 9. Visual System

The visual system is defined entirely in `src/App.css`.

### 9.1 Color Tokens

Current `:root` tokens:

```css
--bg: #f3f5f2;
--panel: #fbfcfa;
--panel-raised: #ffffff;
--panel-subtle: #f6f8f5;
--ink: #101816;
--ink-soft: #2d3936;
--muted: #63716d;
--quiet: #87918e;
--line: #dce3df;
--line-strong: #c4cec9;
--accent: #126a5a;
--accent-strong: #0a4b40;
--accent-soft: #e2f2ed;
--violet: #5d568c;
--violet-soft: #eeecf7;
--info: #285f86;
--info-soft: #e5f0f6;
--warn: #865a16;
--warn-soft: #fff3da;
--bad: #963428;
--bad-soft: #feecea;
--shadow: 0 18px 48px rgba(18, 28, 25, 0.08);
--shadow-soft: 0 1px 2px rgba(18, 28, 25, 0.05), 0 22px 60px rgba(18, 28, 25, 0.07);
--ease-out: cubic-bezier(0.23, 1, 0.32, 1);
--ease-drawer: cubic-bezier(0.32, 0.72, 0, 1);
--focus: 0 0 0 3px rgba(18, 106, 90, 0.22);
```

Dominant UI feel:

- Warm gray-green background.
- White/near-white panels.
- Deep green primary accent.
- Amber warnings.
- Red danger/errors.
- Violet secondary evidence chips.

### 9.2 Typography

Global font:

```css
ui-sans-serif, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif
```

Global text rendering:

- `font-synthesis: none`
- `text-rendering: optimizeLegibility`
- `-webkit-font-smoothing: antialiased`
- `-moz-osx-font-smoothing: grayscale`

Headings:

- Letter spacing is 0.
- `text-wrap: balance`.
- Most compact headings use 18 to 20 px.
- Continue hero H2 uses 46 px desktop, 34 px under 860 px, 30 px under 560 px.

Monospace appears mainly in path/raw text surfaces:

```css
"SFMono-Regular", Consolas, "Liberation Mono", monospace
```

### 9.3 Shape Language

Most elements use 8 px radius or less:

- App topbar: 8 px.
- Brand mark: 8 px.
- Buttons: 8 px.
- Cards/panels: 8 px.
- Inner answer shell: 6 px.
- Companion subblocks: 6 px.
- Island native UI uses its own larger rounded rectangles and capsules.

Pills use `999px` radius:

- Memory dot.
- Evidence badges.
- Provenance/why chips.
- Status-like compact badges.

### 9.4 Buttons

Common button selectors:

```css
.primary-button,
.secondary-button,
.danger-button,
.search-form button
```

Shared:

- Min height: 38 px.
- Border radius: 8 px.
- Font size: 13 px.
- Font weight: 740.
- Active scale: `scale(0.98)`.
- Focus uses `--focus`.

Primary:

- Green vertical gradient.
- White text.
- Inset highlight.
- 10 px/18 px green shadow.
- Hover lifts by -1 px.

Secondary:

- White translucent background.
- Strong line border.
- Hover lifts by -1 px.

Danger:

- Red background.
- Red border.
- White text.

Text button:

- Transparent.
- Accent text.
- 12 px.
- Bold.
- Underlines on hover.

### 9.5 Cards and Panels

Common surface style:

- Border: `1px solid rgba(196, 206, 201, 0.76)` or `var(--line)`.
- Background: near-white or subtle translucent panel.
- Shadow: `--shadow-soft` for product surfaces, `--shadow` for diagnostics.
- Radius: 8 px.

Do not nest page sections as decorative cards inside cards. Current nesting exists for concrete framed tools such as answer shell, companion facts, evidence panels, and diagnostic cards.

### 9.6 Error and Warning Surfaces

`.error-box`:

- Red/bad border.
- `var(--bad-soft)` background.
- `role="alert"` in React.
- 13 px text.

Warnings:

- `.warning-group`
- Amber border/background.
- Up to 4 items.

Low-confidence Continue card:

- Adds amber-tinted background and border.

### 9.7 Motion and Reduced Motion

App CSS motion:

- Most transitions use `--ease-out`.
- Drawer-like/pulse motion uses `--ease-drawer`.
- Buttons use transform on hover/active.
- Companion active orb uses `memoryPulse`.

Reduced motion media query:

```css
@media (prefers-reduced-motion: reduce) {
  *,
  *::before,
  *::after {
    animation-duration: 1ms !important;
    animation-iteration-count: 1 !important;
    scroll-behavior: auto !important;
    transition-duration: 1ms !important;
  }
}
```

### 9.8 Responsive Breakpoints

At max width 1240 px:

- Topbar becomes one column.
- Continue stage becomes one column.
- Companion becomes non-sticky.
- Diagnostics workspace becomes one column.
- Evidence grid becomes one column.
- Inspector grid becomes two columns plus full-width evidence pane.

At max width 860 px:

- Shell padding reduces to 10 px.
- Answer shell padding reduces to 24 px.
- Hero H2 becomes 34 px.
- Answer state becomes one column.
- Companion facts become four columns.
- Inspector and evidence pane become one column.
- Viewer stage gets minimum height 300 px.
- Overlay toolbar becomes two columns.

At max width 560 px:

- Many action/status rows become two-column grids.
- Buttons and capture menu become full width.
- View switch buttons become full width.
- Memory dot can wrap.
- Answer shell padding reduces to 20 px.
- Hero H2 becomes 30 px.
- Target strong text becomes 21 px.
- Most grids collapse to one column.
- Frame thumbnail expands to full width and 110 px height.

## 10. Data Flow and React State

The app uses Tauri commands through `invoke`.

### 10.1 Core State

Important top-level state:

- `status`: `CaptureStatus`
- `continueMemory`: `ContinueMemoryStatus | null`
- `continueDecision`: `ContinueDecisionResult | null`
- `continueDecisionFrameCount`: frame count at decision time.
- `continueOpenResult`: result of opening target.
- `workstreams`: recent workstreams.
- `selectedWorkstreamId`
- `workstreamDetail`
- `memoryDiagnostics`
- `cleanupResult`
- `selectedFrame`
- `frameDetail`
- `timeline`
- `results`
- `imageData`
- `overlayMode`
- `evidenceTab`
- `evidenceOpen`
- `viewMode`
- `busyAction`
- `error`
- `continueError`

### 10.2 Initial Load

On mount:

- Calls `capture_status`.
- Calls `get_continue_memory_status`.

If `capture_status` has a latest frame and no selected frame, it selects the latest frame.

### 10.3 Polling

Polling interval:

- Running: 1500 ms.
- Not running: 6000 ms.

Every tick:

- Refresh capture status.
- If running:
  - Refresh Continue memory.
  - If Inspect mode:
    - Refresh search.
    - Refresh timeline.
    - Refresh workstreams.

### 10.4 Auto Continue

The app auto-runs Continue once when:

- `autoContinueRef.current` is false.
- Nothing is busy.
- No Continue decision exists.
- `status.frame_count > 0`.

Auto-run call:

- `runContinueDecision({ writeAudit: false })`

Manual Continue/Refresh/Rebuild calls use `writeAudit: true`.

### 10.5 Staleness

`continueIsStale` is true when:

- A decision exists.
- `continueDecisionFrameCount` is known.
- Current `status.frame_count` is greater than `continueDecisionFrameCount`.

This drives:

- Topbar `Continue` freshness pill.
- Continue answer eyebrow: `New local evidence since this answer`.

### 10.6 Image and Detail Loading

Full frame image loads when:

- A selected frame exists.
- And either Inspect mode is open or evidence panel is open.

Frame detail loads only when:

- A selected frame exists.
- Inspect mode is open.

This keeps the Continue-first surface lighter.

## 11. Tauri Command Contracts Used By UI

Commands called by `src/App.tsx`:

- `capture_status`
- `get_continue_memory_status`
- `get_local_memory_diagnostics`
- `get_recent_continue_workstreams`
- `get_continue_workstream_detail`
- `search_captures`
- `get_recent_timeline`
- `get_frame`
- `get_frame_detail`
- `get_continue_decision`
- `open_resume_point`
- `add_continue_breadcrumb`
- `record_continue_feedback`
- `run_continue_eval`
- `cleanup_local_memory`
- `dev_reset_local_memory`
- `start_capture`
- `stop_capture`
- `capture_once`
- `delete_all_frames`
- `get_frame_image_variant`

Events listened to by React:

- `capture-frame`

Events emitted by Rust that matter to the app/island:

- `capture-frame`
- `capture-status`
- `session-island-continue-ready`

### 11.1 Continue Decision Request From React

Manual normal Continue:

```ts
{
  mode: "normal",
  rebuild_layers: false,
  micro_inference_enabled: true,
  max_candidates_for_model: 5,
  audit_output_enabled: true
}
```

Auto Continue uses the same normal path but `audit_output_enabled: false`.

Rebuild Continue:

```ts
{
  mode: "rebuild",
  rebuild_layers: true,
  micro_inference_enabled: true,
  max_candidates_for_model: 5,
  audit_output_enabled: true
}
```

### 11.2 Open Continue Target From React

The primary `Continue here` path calls:

```ts
open_resume_point({
  continue_decision_id: continueDecision.decision_id,
  target_artifact_id: resumeTarget?.artifact_id || null,
  strict_continue_target: true
})
```

This is intentionally strict. If there is no direct URL/document target, the primary button becomes `Needs evidence` and is disabled.

## 12. Native macOS Session Island

The Session Island is implemented in SwiftUI/AppKit, not React.

Files:

- Swift UI/panel: `src-tauri/macos/SessionIslandPanel.swift`
- Rust bridge/actions: `src-tauri/src/session_island.rs`

The native island is a floating, nonactivating, borderless `NSPanel` that can join all spaces and full-screen auxiliary spaces.

Panel configuration:

- `styleMask: [.nonactivatingPanel, .borderless]`
- `isFloatingPanel = true`
- Level: floating window level + 2.
- `collectionBehavior = [.canJoinAllSpaces, .ignoresCycle, .fullScreenAuxiliary]`
- Transparent background.
- No AppKit shadow.
- `hidesOnDeactivate = false`
- Movable by window background.
- Accepts mouse moved events.
- Sharing type: read-only.

### 12.1 Island Presentations

Swift enum:

```swift
private enum IslandPresentation: Equatable {
    case micro
    case compact
    case expanded
}
```

Base dimensions:

```swift
kBaseCollapsedW = 222
kBaseCollapsedH = 48
kBaseMicroHitW = 86
kBaseMicroHitH = 24
kBaseMicroVisualW = 58
kBaseMicroVisualH = 10
kBaseExpandedW = 520
kBaseExpandedH = 268
```

The panel size is multiplied by `gOverlayScale`, currently `1.0`.

### 12.2 Panel Positioning

Initial anchor:

- Top center of the screen containing the mouse, or main screen.
- Y coordinate is visible frame top minus 4 px.

When preserving anchor:

- Uses current panel midX and maxY.
- This keeps the top anchor stable while presentation size changes.

Clamping:

- Panel is clamped inside `screen.visibleFrame`.

Animated frame transitions:

- Duration: `kPanelFrameAnimDur = 0.32`
- Timing curve: `(0.18, 0.92, 0.18, 1.0)`
- Disabled if macOS Reduce Motion is on.

### 12.3 Micro Presentation

Micro is only allowed when:

- `snapshot.state` is `ready`, `recording_compact`, `recording_expanded`, or `resume_ready`
- No `snapshot.lastError`

Visual:

- Tiny evidence tape capsule.
- Visual size: 58 x 10 px.
- Hit area: 86 x 24 px.
- Dark tape fill with subtle grid/scan treatment.
- White/black strokes and subtle black shadow.

Interactions:

- Click reveals compact.
- Hover reveals compact.
- Sends local action `reveal_compact`.

Idle behavior:

- When compact and micro is allowed, a timer returns to micro after `kIdleMicroDelay = 5.0` seconds.

### 12.4 Compact Presentation

Compact size:

- 222 x 48 px.

Layout:

1. Left evidence tape.
2. Center two-line copy block.
3. Right compact action button.

Left tape:

- `EvidenceTapeView`
- Compact-left width: 34 px.
- Compact-left height: 24 px.
- Density: 5.
- Uses the same state inputs as the expanded tape.

Center copy:

- Primary text uses `primaryDisplayText`.
- Secondary text uses `secondaryDisplayText`.
- Primary block width is fixed to 106 scaled px.

Right action:

- Text comes from `compactActionLabel`.
- Action comes from `primaryButtonAction`.
- Disabled state follows `primaryActionDisabled`.

Hover:

- Hover over the right action slightly scales it and raises contrast.
- Hover over compact island sends `keep_compact`, which refreshes compact/micro timing.

Background:

- Capsule fill.
- Active state warms the dark fill and uses the current state accent glow.
- Inactive state is dark graphite.
- Top white highlight.
- Bottom accent glow only when capture is active.

### 12.5 Expanded Presentation

Expanded size:

- 520 x 268 px.

Layout:

1. Header row:
   - Left evidence tape.
   - Primary/secondary text.
   - Larger right evidence tape.
2. Resume detail block if `resume_ready`.
3. Divider.
4. Two action buttons.

Resume detail block:

- Only shown in `resume_ready` when the current state is not privacy/excluded.
- Eyebrow: `resumeDetailEyebrow`.
- Text: `resumeDetailLine`.
- Provenance: `resumeProvenanceLine`.
- Uses translucent rounded rectangle with subtle stroke.

Action buttons:

- `GlassActionButton`
- Primary prominent.
- Secondary subdued.
- Height: 30 scaled px.
- Radius: 8 scaled px.

Outside click:

- When expanded, global and local mouse monitors collapse the panel if the click is outside.
- Collapse sends action `collapse`.

Drag behavior:

- `DraggableHostingView` detects left drag beyond 4 px.
- On drag start, it suppresses immediate expansion for 0.35 seconds.
- Calls `window.performDrag`.
- Keeps compact reveal stable during drag.

### 12.6 Native Motion

Motion functions:

- `quick`: 0.14 s, curve `(0.25, 1, 0.5, 1)`
- `settle`: 0.26 s, curve `(0.20, 0.88, 0.20, 1)`
- `reveal`: 0.34 s, curve `(0.18, 0.92, 0.18, 1)`

All become 0.01 s when Reduce Motion is enabled.

Transitions:

- Micro, compact, and expanded use asymmetric opacity/scale/blur/offset transitions.
- Reduce Motion falls back to opacity.

`AnimationTick`:

- Runs at 60 fps while island is visible.
- Stops when island hides/shuts down.
- Drives evidence tape scan, pulse, heat, and active breath.

## 13. Evidence Tape

`EvidenceTapeView` is the animated native island signal in micro, compact, and expanded presentations.

Inputs:

- `active`
- `processing`
- `error`
- `privateMode`
- `thinEvidence`
- `ready`
- `frameCount`
- `density`
- `pulseNonce`
- `scale`
- `width`
- `height`
- `cornerRadius`
- `compact`

Visual:

- Rounded dark tape.
- Internal horizontal and vertical grid lines.
- Scanhead.
- Frame ticks.
- Red capture heat while active.
- White shimmer while processing.
- Amber line/cross detail when error.
- Pulse bloom when `capturePulseNonce` changes.

State mapping:

- Active capture: red scan/heat/glow.
- Processing/trail reconstructing: white shimmer.
- Error: amber stroke and error mark.
- Thin evidence: amber tape accent.
- Privacy/excluded: muted white tape accent and boundary mark.
- Continue ready: green tape accent with slow scan.
- Idle: dark low-contrast tape.

Frame ticks:

- Compact shows up to 9 ticks.
- Expanded shows up to 18 ticks.
- Newest tick gets stronger alpha.
- Ticks are deterministically scattered using frame ordinal.

## 14. Island Text State Machine

Swift receives JSON snapshots with `snapshot.state` values from Rust.

Rust enum:

```rust
Hidden
Ready
Starting
RecordingCompact
RecordingExpanded
Processing
StoppedToast
TrailReconstructing
ResumeReady
Error
```

Serialized state strings:

- `hidden`
- `ready`
- `starting`
- `recording_compact`
- `recording_expanded`
- `processing`
- `stopped_toast`
- `trail_reconstructing`
- `resume_ready`
- `error`

### 14.1 Primary Display Text

If there is an error:

- `Needs attention`

Otherwise by state:

- `starting`: `Starting memory`
- `recording_compact` / `recording_expanded`: `Memory on`
- `processing`: `Pausing memory`
- `stopped_toast`: `Continue`
- `trail_reconstructing`: `Finding continuation`
- `resume_ready`: uses `resumeReadyTitle`
- default:
  - `Continue ready` if moments exist.
  - `Memory off` otherwise.

`resumeReadyTitle`:

- If `resumeSource == "continue"`:
  - `Evidence is thin` if warning contains `thin`, `missing`, or `no_`.
  - Else `resumeHeadline`, or `Ready to continue`.
- If `resumeSource == "cloud"`:
  - `Continue ready`.
- Other source/fallback:
  - `OpenAI key missing` if warning mentions key.
  - `OpenAI unavailable` otherwise.

### 14.2 Secondary Display Text

If there is an error:

- `Capture needs attention`

Otherwise by state:

- `recording_compact` / `recording_expanded`: trail summary.
- `starting`: `Preparing local memory`
- `processing`: trail summary.
- `trail_reconstructing`: `Checking local evidence`
- `resume_ready`: resume point line.
- default:
  - `Ask Smalltalk where to continue` if moments exist.
  - `Turn on local memory` otherwise.

Trail summary:

- If app count and moment count exist:
  - `<n> app/apps · <m> signal/signals`
- If only moment count exists:
  - `<m> signal/signals`
- Else:
  - `Building local memory`

Resume point line:

- `Continue target ready` if no point.
- `Continue at <point>` otherwise.

### 14.3 Island Action Labels

Primary action label:

- `Starting` when starting.
- `Finding` when trail reconstructing.
- `Pausing` when processing.
- `Continue here` when resume ready.
- `Add evidence` when recording.
- `Continue` when moments exist.
- `Start memory` otherwise.

Secondary action label:

- `Why this?` when resume ready.
- `Pause` when recording.
- `Open Smalltalk` otherwise.

Primary disabled:

- Busy states.
- Error state when not recording.

Secondary disabled:

- Busy states.
- No moments, not recording, and not resume ready.

### 14.4 Island Action IDs

Swift local UI actions:

- `reveal_compact`
- `keep_compact`
- `open_expanded`
- `toggle_meeting`
- `capture_once`
- `continue`
- `reconstruct_trail`
- `show_trail`
- `open_resume_point`
- `close`

Actions sent to Rust:

- `start_capture`
- `stop_capture`
- `capture_once`
- `continue`
- `show_trail`
- `open_resume_point`
- `open_main_window`
- `resume_me`
- `collapse`

Rust action enum:

```rust
Continue
StartCapture
StopCapture
CaptureOnce
ReconstructTrail
ShowTrail
OpenResumePoint
OpenMainWindow
ResumeMe
ToggleExpanded
Collapse
```

Action mapping:

- `toggle_meeting` sends `start_capture` or `stop_capture` based on `metrics.meetingActive`.
- `capture_once` sends `capture_once`.
- `continue` expands the island and sends `continue`.
- `reconstruct_trail` expands the island and sends `continue`.
- `show_trail` sends `show_trail`.
- `open_resume_point` sends `open_resume_point`.
- `open_timeline` and `open_search` send `open_main_window`.
- `open_chat` sends `resume_me`.
- `close` sends `collapse`.

## 15. Rust Island Snapshot Contract

Rust snapshot struct:

```rust
SessionIslandSnapshot {
  state,
  session_id,
  elapsed_ms,
  frame_count,
  trail_app_count,
  trail_moment_count,
  trail_labels,
  last_frame_id,
  current_app,
  current_window,
  current_surface_kind,
  last_trigger,
  last_capture_at_ms,
  capture_pulse_nonce,
  last_error,
  resume_headline,
  resume_detail,
  resume_point,
  resume_source,
  resume_model,
  resume_response_id,
  continue_decision_id,
  resume_warning,
  privacy_label,
  is_sensitive
}
```

Important behavior:

- `trail_app_count` comes from recent app labels count.
- `trail_moment_count` comes from `status.signal_count`, not raw frame count.
- `trail_labels` comes from `status.recent_app_labels`.
- If latest frame privacy status is sensitive, `current_app` and `current_window` are omitted.
- `capture_pulse_nonce` is latest frame id when positive.
- `privacy_label` and `is_sensitive` are derived from latest frame privacy status.

Sensitive privacy labels:

- Anything other than `normal`, `ok`, or `allowed` is treated as sensitive.

### 15.1 Island Continue Flow

When native island sends `continue`:

1. Rust reads `capture_status`.
2. Island snapshot becomes `trail_reconstructing`.
3. Background thread calls `get_continue_decision` with:
   - `mode: "normal"`
   - `rebuild_layers: false`
   - `micro_inference_enabled: true`
   - `max_candidates_for_model: 5`
   - `audit_output_enabled: true`
4. Rust remembers the decision id.
5. Rust refreshes capture status.
6. Snapshot becomes `resume_ready`.
7. `apply_continue_decision_to_snapshot` fills headline/detail/point/provenance/warning.
8. Event `session-island-continue-ready` is emitted.
9. Island updates and remains visible.

### 15.2 Island Resume Ready Snapshot

Decision-to-island mapping:

- `continue_decision_id`: decision id.
- `resume_source`: `continue`.
- `resume_model`: decision model.
- `resume_response_id`: decision response id.
- `resume_headline`:
  - `Ready to continue` for high confidence.
  - `Likely continuation found` for medium confidence.
  - `Evidence is thin` otherwise.
- `resume_detail`:
  - selected workstream title candidate, else next action.
- `resume_point`:
  - resume work target title/url/document/kind, falling back to return target.
- `resume_warning`:
  - first missing evidence note or first warning.

### 15.3 Island Open Resume Flow

When native island sends `open_resume_point`:

1. Rust uses remembered Continue decision id if available.
2. If no remembered decision exists, it runs `get_continue_decision` normal path.
3. Calls `open_resume_point` with:
   - `continue_decision_id`
   - `strict_continue_target: true`
   - no explicit target artifact id.
4. If warnings exist, Rust logs them.
5. If open strategy starts with `smalltalk_`, Rust opens/focuses the main window.
6. If opening fails, Rust logs and opens/focuses the main window.

## 16. UX Doctrine and Pitfalls

### 16.1 Preserve Current Focus vs Return Target

Do not collapse these concepts:

- `current_focus`: what is currently visible/focused.
- `current_activity`: current activity summary.
- `return_target`: where continuation points.
- `resume_work_target`: strict actionable target used for opening.

The UI intentionally shows current focus only when it differs from the return target. This prevents the product from acting like the current screen is always the right place to resume.

### 16.2 Stop/Pause Is Not Required For Continue

Continue can run while local memory is active or paused.

The empty state explicitly says:

```text
Continue can answer from local evidence without requiring you to stop or export anything first.
```

Do not reintroduce copy or UX that implies Stop/Pause is a prerequisite for Continue.

### 16.3 Diagnostics Are Secondary

The following are evidence/diagnostic surfaces, not primary product primitives:

- Sessions.
- Frames.
- Screenshots.
- Search.
- Raw event streams.
- Timeline events.
- Capture triggers.
- Transitions.
- OCR spans.
- AX nodes.
- Content units.
- Storage metrics.
- Cleanup controls.
- Eval metrics.
- Paths.

They belong in `Inspect` or behind `Why this?`, not on the first screen.

### 16.4 Thin Evidence Must Be Honest

The UI has several explicit thin/uncertain states:

- Continue card becomes low-confidence.
- Open button says `Needs evidence` and disables direct opening.
- Evidence panel shows evidence notes.
- Island can say `Evidence is thin`.

Do not hide uncertainty behind confident copy.

### 16.5 Avoid Internal IDs In User-Facing Lines

The React layer suppresses internal-facing text with:

- `looksLikeInternalId`
- `isInternalFacingText`
- `safeProductLine`
- `cleanHumanText`

Examples treated as internal:

- `continue-candidate-*`
- `workstream-*`
- `artifact-*`
- `frame-fallback`
- `target metadata`
- `selected candidate`
- `candidate id`
- `workstream id`
- `artifact id`
- `frame_id`
- `frame 123`

If internal text leaks, the UI falls back to safer product copy such as:

- `No reliable continuation target yet`
- `No reliable return target is grounded yet.`

### 16.6 Privacy Boundary Is Product UX

Both the product doctrine and visible UI rely on this promise:

- Raw typed characters are not stored.
- Full clipboard contents are not stored.

Do not add UI that suggests full keystroke or clipboard replay exists.

### 16.7 Island Is A Product Surface

The island is not only decorative. It can:

- Start memory.
- Pause memory.
- Capture evidence.
- Run Continue.
- Open the strict resume target.
- Focus/open the main window.
- Show why/evidence through `show_trail`.

Changes to island copy, disabled states, or action mapping are product behavior changes.

## 17. Current Known Tensions

These are current implementation tensions to understand before editing:

- Some class names still say `capture`, `developer`, or `session` because the app evolved from a capture/session foundation.
- Inspect mode still exposes session/frame terminology because diagnostics need it.
- `EmptyCaptureState` still says `Start a session...`, which is legacy diagnostic copy inside Inspect mode, not Continue mode.
- The native island state enum still includes `RecordingCompact`, `RecordingExpanded`, and `StoppedToast`, but visible copy is now memory/Continue oriented.
- `open_resume_point` still contains compatibility/fallback behavior, but the primary Continue button and island path both use strict Continue target semantics.
- The React app auto-runs Continue once on startup if frames exist, but it does not write an audit bundle for that automatic path.

## 18. Implementation Checklist For Future UI Agents

Before changing UI/UX, verify current behavior in this order:

1. Read `src/App.tsx` for actual render conditions, labels, and command payloads.
2. Read `src/App.css` for existing layout, breakpoints, and token constraints.
3. Read `src-tauri/macos/SessionIslandPanel.swift` for native island copy, motion, and interactions.
4. Read `src-tauri/src/session_island.rs` for island snapshot/action semantics.
5. Treat `docs/continue-ui-reality.md` as historical, not current.
6. Keep Continue mode as one primary answer plus companion panel.
7. Keep raw evidence and diagnostics behind `Why this?` or `Inspect`.
8. Preserve strict opener behavior for the primary Continue target.
9. Preserve privacy copy unless the backend privacy model changes.
10. Run at least `git diff --check` for documentation/style changes; run `npm run build`, `cargo check`, and `cargo test` for code changes.
