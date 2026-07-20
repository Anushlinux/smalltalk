# MFTI-01 — Build A Truthful Model-Ready Observation Stream

## Codex task

Repair Smalltalk's existing capture-to-Task-Truth pipeline so a cloud multimodal model receives a recent, correctly ordered, privacy-approved sequence containing readable screens, Accessibility and OCR semantics, grounded interactions, and before/after state changes.

This is an implementation session. Do not stop after writing a design, adding schemas, or making deterministic fixtures pass. Inspect the current dirty worktree first, preserve unrelated changes, reproduce the live failures described below, implement the repair in the existing pipeline, and verify it against a newly captured real sequence.

This prompt is the first of four ordered sessions. It does not ask the model to infer the user's task yet. Its job is to make the evidence given to that model truthful and complete enough for MFTI-02.

## Product outcome

At the end of this session, a Task Truth request for a recent cross-application sequence must contain:

1. A readable current active-window image.
2. A bounded set of relevant earlier keyframes.
3. Accessibility and OCR elements owned by the correct window and region.
4. Click, scroll, focus, submit, typing-commit, navigation, and app-switch events grounded to the element and before/after frames when evidence permits.
5. Explicit missing-evidence and ambiguity notes when grounding is not possible.
6. A session id and evidence window that cannot silently mix unrelated sessions.

Local code may capture, redact, crop, order, deduplicate, bind events, calculate geometry, validate identifiers, and build the request. Local code must not decide the semantic task, goal, work object, or relationship between a detour and the main task. Those decisions belong to the cloud multimodal inference path implemented in MFTI-02.

## Why this session exists

The repository already collects many useful signals, but the current request is not a truthful representation of recent work.

Verify these failures against the current code and current local evidence before editing:

- `src/App.tsx` invokes `get_continue_decision` without a `session_id`, allowing later snapshot selection to cross session boundaries.
- `src-tauri/src/continuation/task_truth_v2/observation_packet.rs` uses `active_window_crop_path` as the model image and does not reliably fall back to a privacy-approved full screenshot or derived active-window crop.
- Browser frames often have a full screenshot but no active-window crop, causing `no_privacy_approved_readable_images`.
- Canonical-element and causal-event caps are filled while iterating older frames first. In the diagnosed live packet, nearly all semantic elements and all causal-event capacity were consumed by old frames, leaving none from the current X frames.
- Accessibility and OCR evidence can include browser chrome, other windows, other displays, or stale nodes. A toolbar phrase such as `Tab search - pinned` can outweigh the actual page.
- UI events exist, but many are retained as counts or generic event labels rather than being linked to the element acted on and the state produced by the action.
- Typing privacy must remain intact: Smalltalk must not store raw typed characters or full clipboard contents.

Do not assume an existing completion audit proves these live properties. Inspect source, SQLite rows, and a new manual Continue audit.

## Required reading and current-state inspection

Read before editing:

```text
AGENTS.md
PRODUCT.md or product.md
docs/phases/p6-task-turn-accuracy/p6-00-task-turn-accuracy-program.md
docs/phases/p6-task-turn-accuracy/p6-09-completion-audit.md
docs/phases/task-truth-v2/tt2-01-causal-evidence-and-containment.md
docs/phases/task-truth-v2/tt2-03-observation-packets-task-snapshots-and-checkpoints.md
docs/phases/task-truth-v2/tt2-04-multimodal-resolver-and-evidence-verifier.md
docs/phases/task-truth-v2/tt2-05-completion-audit.md
src/App.tsx
src-tauri/src/capture.rs
src-tauri/src/continuation.rs
src-tauri/src/continuation/task_truth_v2/observation_packet.rs
src-tauri/src/continuation/task_truth_v2/model.rs
```

Find the current live SQLite path through `capture_status.database_path`. Treat the live database and current worktree as authoritative. Do not commit the database, screenshots, private OCR, Accessibility text, raw URLs, or exported personal audits.

## Non-negotiable architecture

Use the existing Task Truth v2 module. Repair or replace incorrect pieces there. Do not create a third parallel Task Truth system and do not revive the browser extension.

The observation flow must be:

```text
capture boundary
→ privacy/exclusion decision
→ active application/window/display ownership
→ screenshot plus AX/OCR semantic state
→ event-to-element and event-to-frame grounding
→ before/after semantic delta
→ recency- and relevance-preserving observation packet
→ model request input
```

The packet must distinguish four concepts instead of flattening them:

```text
visible surface
user operation on that surface
semantic change produced by the operation
possible relationship to earlier work — left unresolved for the model
```

## Goal 1 — Make every Continue request session-scoped

Pass the current capture/session identifier from React and every native/island/background invocation into `get_continue_decision`.

Requirements:

- A manual Continue request must identify the active capture session.
- Background, startup, cached, and island paths must use the same scope contract.
- If no session can be established, use an explicit unscoped state and prevent old task snapshots from becoming current merely because they are selected.
- Persist the effective session scope in the audit.
- Add tests proving two adjacent sessions cannot contaminate one another.

Do not solve this by globally filtering all older evidence. Cross-session continuity may later be supported only through explicit task-thread continuity evidence in MFTI-03.

## Goal 2 — Guarantee a readable current image

For every model-eligible current foreground frame, resolve the best privacy-approved visual input in this order:

1. Native active-window capture.
2. A derived crop from the full screenshot using verified active-window bounds and display geometry.
3. A full-display image only when ownership, privacy, and multi-display policy explicitly permit it.
4. No image, with an exact typed reason.

Requirements:

- Preserve image legibility after crop/resize.
- Record source kind, frame id, application, window id, capture time, dimensions, scope, and rejection reason.
- Do not label a full-display image as an active-window crop.
- Exclude private/background windows before request construction.
- Correctly map logical Accessibility coordinates to screenshot pixels, including screen scale and multiple displays.
- Treat mixed-window or mixed-display OCR as conflicted evidence, not current-window truth.
- Do not durably duplicate private image bytes in audit output.

Add a model-input preview available only under the private developer audit surface so a developer can see exactly which frames would be sent.

## Goal 3 — Replace oldest-first packet truncation

Observation limits must preserve the newest state and the causal path into it.

Implement explicit capacity partitions or an equivalent proven policy. At minimum reserve space for:

```text
current foreground state
immediately preceding transition states
recent task-bearing states
older background/context states
```

Required properties:

- The current foreground frame cannot receive zero element capacity because an old frame was verbose.
- Current and immediately preceding causal events cannot be displaced by old event noise.
- Browser chrome cannot consume the entire element budget.
- Every cap reports retained and dropped counts by frame, partition, source, role, and age.
- Keyframes, canonical elements, causal events, and semantic deltas use the same temporal ordering.
- Packet construction remains bounded in bytes, elements, events, images, and build latency.

Add a regression reproducing the diagnosed shape: an old frame with more than 160 elements followed by several current frames. Assert that the current frames retain elements and causal events.

## Goal 4 — Establish source ownership before semantic use

For every Accessibility node, OCR span, content unit, and visual region, preserve:

```text
frame id
display id
window id
owning app/bundle
bounds and coordinate space
source: AX, OCR, visual, or merged
region: page content, browser chrome, app chrome, dialog, overlay, background, unknown
freshness
conflicts
```

Requirements:

- Other-window OCR remains available for diagnostics but is ineligible as current foreground meaning.
- Browser/app chrome is not discarded, because it may receive a real click, but it cannot describe page meaning unless the user interacted with it.
- AX/OCR disagreement is preserved for the model and verifier.
- Stale Accessibility nodes must be marked stale or excluded.
- Ownership must be established before content-role classification.

Add regressions for browser tab search, another visible window, a permission dialog, a custom-rendered page with thin AX, and multi-display capture.

## Goal 5 — Ground interactions to elements and state transitions

Build a bounded causal interaction record. It should support records equivalent to:

```text
event id and timestamp
event kind
source frame
target frame
target element or region, when known
focused element before and after
application/window ownership
pre-state reference
post-state reference
semantic delta reference
grounding confidence
missing or conflicting evidence
```

Required behavior:

- A click should be linked to the top eligible element under the pointer, using ownership and z-order where available.
- A scroll should identify the affected window/region and resulting content movement when observable.
- Typing bursts must remain content-free but link to focused editable elements, commit signals, and post-action frames.
- Submit/Enter, navigation, new-window, app-switch, focus, and terminal-command boundaries must retain ordering.
- A generic `search_result` element elsewhere on screen cannot turn an unrelated click into searching.
- Failed grounding must remain `unknown`; do not fabricate a target element.

The local layer may call an interaction a `click`, `scroll`, `typing_commit`, or `navigation`. It must not locally decide that the user was “researching”, “debugging”, or “working on Smalltalk.”

## Goal 6 — Produce explicit before/after semantic deltas

For selected event boundaries, compare the prior and next semantic state and record:

- appeared/disappeared/changed regions;
- URL, document, tab, window, dialog, selection, focus, or output changes;
- whether the content moved while chrome stayed stable;
- whether the action produced no observable change;
- source agreement and conflict;
- event ids that plausibly caused the change.

Do not infer the user's goal in this layer. Describe observable changes only.

## Required audit

One private manual Continue audit must make the following inspectable without opening SQLite manually:

- effective session id and time window;
- every selected keyframe in chronological order;
- exact image supplied or excluded and why;
- retained/dropped element counts by frame;
- retained/dropped events by frame;
- ownership and region distribution;
- causal interaction records;
- before/after deltas;
- packet byte/token estimate;
- privacy exclusions;
- whether the current frame had readable visual and structured evidence.

## Required tests

Add focused deterministic tests for:

1. Current-frame capacity survives an oversized old frame.
2. Current causal events survive old event pressure.
3. Missing active-window crop derives a safe crop from verified bounds.
4. Unsafe or uncertain crop produces a typed missing-image reason.
5. Other-window OCR cannot become current foreground content.
6. Browser chrome stays separate from page content.
7. Click-to-element grounding uses ownership and coordinates.
8. Scroll is attached to the correct region/window.
9. Privacy-safe typing metadata links to focus and post-frame without storing text.
10. Two sessions cannot silently mix.
11. Multi-display coordinate mapping is correct.
12. Packet audit totals match actual retained packet contents.

## Live verification scenario

Create a fresh private development sequence containing:

1. Editing or reviewing Task Truth code in VS Code.
2. Opening a browser page related to AI/tool behavior.
3. Clicking a link and scrolling the resulting page.
4. Returning to Smalltalk and pressing Continue.

Inspect the generated model-input audit. It must prove:

- the latest browser screen is readable;
- the earlier VS Code state is represented;
- click and scroll events are in the correct order;
- page content is separated from browser chrome;
- the current frames retain semantic elements and causal events;
- no other-window content is presented as foreground truth;
- the session id is present.

This session does not need to judge whether the browser visit was research or distraction. That is deliberately deferred to MFTI-02.

## Verification commands

Run the narrowest relevant tests while iterating, then run:

```bash
cd src-tauri && cargo fmt --check
cd src-tauri && cargo check
cd src-tauri && cargo test task_truth_v2
cd src-tauri && cargo test continuation
cd src-tauri && cargo test capture
npm run build
```

If a broad filter does not select the expected tests, run the exact test targets one at a time and report them. Do not claim success from a command that ran zero relevant tests.

## Definition of done

This session is complete only when:

- the current worktree implements the corrected observation path;
- all required tests pass;
- a new real manual sequence produces the expected private audit;
- the current foreground frame has readable pixels or an honest typed exclusion;
- current structured evidence cannot be starved by old evidence;
- interactions are grounded to elements and before/after states where evidence permits;
- session scope is explicit;
- no raw typed text or clipboard contents are stored;
- no local code introduced here infers the semantic task;
- the completion report links exact source, test, and live-audit evidence for every item above.

Do not mark this goal complete because schemas exist or fixture packets serialize. The model-ready stream must be proven against a newly captured real sequence.

## Handoff to MFTI-02

Report:

- changed files;
- versioned packet and causal-record schemas;
- image-selection policy;
- packet budget policy;
- exact tests and counts;
- private live audit path;
- remaining capture limitations;
- the exact API by which MFTI-02 receives chronological images, semantics, interactions, and deltas.
