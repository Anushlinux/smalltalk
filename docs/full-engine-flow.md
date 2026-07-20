# Smalltalk Full Engine Flow

Last updated: 2026-07-07

This document describes the current native Smalltalk engine as implemented in this working tree. It is meant for debugging bad `Continue` output, not for marketing the product. It names what the system actually does, where data is processed, where it can become vague or wrong, and which artifacts to inspect when the UI points at the wrong thing.

Task Truth v2.03 also runs a shadow-only semantic path before workstream and target selection. The second-layer rebuild constructs a bounded `smalltalk.observation_packet.v2` from existing frames, content units, OCR, events, triggers, transitions, typing bursts, and diffs. It projects the selected P6 `CurrentTaskTurn` into `smalltalk.task_snapshot.v2`, or persists an unresolved snapshot when current task evidence is insufficient. Semantic checkpoints are deduplicated and retained locally. Manual Continue writes a bounded comparison audit. This path does not replace the visible Continue answer, and target openability is excluded from snapshot selection.

Primary source files:

- `src/App.tsx`
- `src-tauri/src/capture.rs`
- `src-tauri/src/continuation.rs`
- `src-tauri/src/session_island.rs`
- `src-tauri/macos/SessionIslandPanel.swift`

The active product path is the native Tauri desktop app. The older `browser-extension/` prototype is not the current MVP engine. `run_cloud_resume`, `get_native_resume_card`, stop-time `resume_query_exports`, and the floating island still exist, but they are compatibility, diagnostic, fallback, or consumer surfaces. The core product decision is `get_continue_decision`.

## One Page Pipeline

```text
Native local signals
  -> ui_events, typing_bursts, clipboard_events
  -> coalesced capture_triggers
  -> sparse heavy frames when budgets and triggers allow
  -> SQLite evidence substrate
  -> text/source attribution and active/background text resolution
  -> Continue semantic layer: artifacts, observations, actions
  -> semantic moments and boundary revisions
  -> episodes, workstreams, state snapshots, workstream graph
  -> open loops and unresolved state
  -> candidate generation
  -> app-activity, memory-retrieval, and feedback features
  -> local scoring, risk caps, and candidate demotion
  -> current-surface resolution and evidence-freshness ledger
  -> quality gate: strong_continue, thin_continue, no_clear_continuation
  -> optional bounded micro-inference over supplied candidate ids
  -> validated or rejected model output
  -> local handoff composition
  -> persisted Continue decision
  -> React card, Why this panel, island snapshot, and optional audit export
```

Smalltalk does not send broad raw history to a model and ask it to invent intent. The local engine first builds local evidence and local semantic objects. The model path, when it runs, can only choose among supplied local candidates and its copy is accepted only after local validation.

The final answer intentionally separates:

- `current_focus`: what appears to be current or recently visible.
- `current_activity`: a short local read of current behavior.
- `selected_workstream`: the durable cluster of related work.
- `return_target`: the public target Smalltalk is willing to expose as a return point.
- `resume_work_target`: the actionable work target, kept separate from support branches.

These fields can be null or divergent. That is not automatically a UI bug. It is often the correct answer when evidence is thin or the current screen is only support context.

The engine must not invent artifacts, URLs, file paths, user intent, or next actions. If evidence is thin, the correct behavior is to say it is thin, cap confidence, and expose inspectable evidence.

## Current Live Continue Path

The first-screen product answer comes from `src/App.tsx`.

Current React call shape:

```ts
invoke("get_continue_decision", {
  input: {
    mode: options.forceRebuild === true ? "rebuild" : "normal",
    rebuild_layers: options.forceRebuild === true,
    micro_inference_enabled: true,
    max_candidates_for_model: 5,
    audit_output_enabled: options.writeAudit === true,
  },
});
```

Important details:

- Startup/background calls use normal mode and do not write audit output.
- Explicit manual UI actions can set `audit_output_enabled: true`.
- Rebuild uses `mode: "rebuild"` and `rebuild_layers: true`.
- The current UI does enable micro-inference by default.
- The island also requests `get_continue_decision` with `micro_inference_enabled: true`, `max_candidates_for_model: 5`, and `audit_output_enabled: false`.

Opening is separate from deciding. The primary open action calls `open_resume_point` with:

```ts
{
  continue_decision_id: continueDecision.decision_id,
  target_artifact_id: resumeTarget?.artifact_id || null,
  strict_continue_target: true,
}
```

`get_continue_decision` decides what the answer is. `open_resume_point` later tries to open a persisted decision target. A good-looking answer does not prove opening will work; opening still depends on a safe target, URL/path/frame evidence, and native/browser automation.

## Public Command Surface

Commands are registered through `src-tauri/src/lib.rs` and implemented mostly in `capture.rs` and `continuation.rs`.

| Group | Commands | Role |
| --- | --- | --- |
| Capture runtime | `start_capture`, `stop_capture`, `capture_once`, `capture_status` | Start, stop, manually sample, and inspect local memory. |
| Reset and cleanup | `delete_all_frames`, `get_local_memory_diagnostics`, `cleanup_local_memory`, `dev_reset_local_memory`, `delete_recent_captures` | Developer and user cleanup controls. |
| Evidence read APIs | `search_captures`, `get_frame`, `get_frame_image`, `get_frame_image_variant`, `get_recent_timeline`, `get_frame_detail`, `validate_frame_consistency`, `get_transition`, `search_content_units` | Read raw or processed local evidence. |
| Legacy/native compatibility | `start_native_capture`, `stop_native_capture`, `capture_once_v2`, `get_frame_v2` | Compatibility wrappers around the current capture implementation. |
| Safe/export diagnostics | `build_safe_ai_export`, `build_session_index`, `build_resume_query_bundle`, `export_debug_episode`, `get_episode_dossier`, `get_native_storyboard_dossier` | Evidence export and diagnostics. |
| Older cloud resume lane | `run_cloud_resume`, `get_cloud_resume_status` | Stop/resume-query OpenAI path. Not the core Continue engine. |
| Continue memory | `get_continue_memory_status`, `rebuild_continue_second_layer`, `rebuild_continue_third_layer`, `get_recent_continue_artifacts`, `get_recent_continue_task_actions`, `get_recent_continue_episodes`, `get_recent_continue_workstreams`, `get_continue_workstream_detail` | Semantic memory inspection and rebuild APIs. |
| Continue decision | `get_continue_decision`, `get_continue_decision_trace`, `open_resume_point` | Core product decision, developer trace, and open action. |
| Continue feedback | `add_continue_breadcrumb`, `infer_continue_feedback`, `record_continue_feedback`, `run_continue_eval` | Manual notes, inferred/explicit feedback, and evals. |
| Evidence probing | `assess_continue_evidence_sufficiency`, `request_more_continue_evidence` | Current evidence sufficiency and bounded probe support. |
| Legacy resume/eval | `classify_episode_transitions`, `get_native_resume_card`, `run_resume_eval` | Older local resume-card behavior. Not the core Continue engine. |
| Privacy rules | `add_exclusion_rule`, `remove_exclusion_rule`, `list_exclusion_rules` | Runtime privacy and exclusion configuration. |

## Runtime Storage

The live capture root is resolved through Tauri app data:

```text
<tauri app data>/capture/
  smalltalk-capture.sqlite
  snapshots/
  helpers/
  safe-ai-exports/
```

On this Mac the usual resolved app-data location is:

```text
~/Library/Application Support/com.smalltalk.app/capture/
```

The frontend should trust `capture_status.data_dir` and `capture_status.database_path` rather than hard-coding a location. Repo-local folders such as `resume_query_exports/` and `continue_outputs/` are generated artifacts. They are useful for debugging, but they are not the live capture store.

Important evidence tables include:

| Table | What it stores |
| --- | --- |
| `capture_sessions` | Local-memory session rows, status, timestamps, counts, and stop metadata. |
| `frames` | Sparse heavy evidence frames: app/window/url/path, full text, hashes, screenshot paths, trigger, session, privacy, ScreenCaptureKit metadata. |
| `frames_fts` | Full-text search index over frame text and metadata. |
| `ui_events` | Lightweight native events such as app switches, focus changes, clicks, scrolls, key categories, clipboard events, and AX notifications. |
| `capture_triggers` | Coalesced event buckets that may produce a heavy frame. |
| `event_transitions` | Transition summaries linking pre-frame, event, and post-frame evidence. |
| `typing_bursts` | Keyboard activity summaries without raw typed characters. |
| `clipboard_events` | Clipboard metadata without full clipboard text. |
| `ocr_text` / `ocr_spans` | OCR text and spans with bounds/confidence. |
| `ax_nodes` | Accessibility nodes, roles, text, bounds, focus, selected state, actions, and raw node JSON. |
| `content_units` | Normalized units extracted from AX/OCR for semantic roles and search. |
| `app_contexts` | App/product adapters for browser tabs, docs, terminals, conversations, files, and other surfaces. |
| `window_snapshots` / `windows` | macOS window graph snapshots and observed windows. |
| `frame_diffs` | Simplified text/content changes between adjacent frames. |
| `frame_text_resolutions` | Active/background text split and quality flags for Continue. |
| `frame_quality_warnings` | Evidence consistency and quality warnings. |
| `maintenance_counters` | Runtime counters for capture, storage, and Continue diagnostics. |

Continue-specific tables are created by `ensure_continue_schema` in `continuation.rs`.

## What Smalltalk Captures

Smalltalk has two evidence classes:

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

## Capture Priority

The capture system does not store a fresh screenshot for every local event.

Current behavior:

- Lightweight native events are first-class memory signals.
- Manual and session-start captures can force heavy evidence and bypass dedupe.
- Important event triggers can produce heavy frames, but are rate-limited.
- Low-value event triggers can produce heavy frames only when budgets allow.
- Idle capture is fallback behavior.
- Accessibility is preferred for semantic text when strong.
- OCR is secondary and should not be treated as authoritative when mixed or background-owned.
- ScreenCaptureKit is the preferred screenshot provider on macOS.
- `/usr/sbin/screencapture` is the screenshot fallback.
- Event and idle captures are budgeted and deduped.

Important heavy-frame triggers include `manual`, `session_start`, `app_switch`, `window_focus`, and `clipboard`.

Low-value triggers include `typing_pause`, `scroll_stop`, `click`, `accessibility_change`, `event_burst`, and `idle`.

This means the freshest evidence may be an event or typing burst rather than a screenshot. That is a central cause of screenshot/output mismatch: the engine may know the current surface changed from events, while the newest available frame image still belongs to an older surface.

Committed typing bursts now retain privacy-safe event, app/window, trigger, and pre/post-frame provenance. Capture associates a committed burst with the authoritative post frame when the same session and surface caused that frame; older null-post rows may be recovered only by a bounded, unique same-surface predecessor rule. Task-turn extraction consumes a structured causal-attribution object rather than a boolean typing hint, and never stores raw typed characters. Missing or conflicting attribution remains explicit ambiguity.

Current user-goal eligibility is shared across task-turn selection: controls, navigation, model pickers, approval chips, browser chrome, status labels, and other actionable UI affordances are history/context rather than user-authored goals. `prior_boundary_sample` is history-only and cannot seed a new current goal. If no eligible current goal survives, the backend emits `no_clear_current_task`; React and the native island suppress task, state, alternative, and direct-target claims while retaining only supported surface context and inspect-first guidance.

## Native Events And Trigger Coalescing

The native event helper emits newline-delimited JSON. Rust parses events into `UiEventRecord` rows and stores them in `ui_events`.

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

Only one pending trigger bucket is kept at a time. If another event arrives before the pending trigger settles, the event id is added to the bucket and the deadline is pushed out. If trigger types differ, the bucket becomes `event_burst`.

This avoids recording every small movement, but it also means screenshots are sampled evidence, not a complete visual replay.

## Heavy Frame Processing

When a heavy frame is accepted, the capture layer may store:

1. Screenshot path.
2. Active-window crop path.
3. App/window/url/path metadata.
4. Accessibility context.
5. OCR text/spans when needed.
6. Window graph snapshot.
7. App context rows.
8. Content units.
9. Privacy/sensitive-region metadata.
10. Frame diff and quality warnings.

The important debugging point is that a frame is a snapshot of one moment. It may be older than the current UI event stream. A frame image is evidence, not necessarily the final current focus or final return target.

## Text Attribution

Text attribution is a major correctness boundary.

Continue should use active, owned text before raw screenshot/OCR text. The current code loads `frame_text_resolutions` when available and gives each frame:

- `active_text`
- `background_text`
- `full_text_quality`
- `quality_flags_json`

`active_frame_text_for_continue(frame)` prefers `active_text`. It blocks raw `full_text` when the quality is:

- `mixed_active_and_background`
- `background_only`
- `display_only_unattributed`
- `unknown`

This prevents background OCR, mixed display text, or another window's text from becoming primary task evidence. The tradeoff is that Continue can become vague if the only available text is blocked as unsafe.

Error/task classification also uses attribution gates:

- `classify_error_context` looks for artifact-owned error evidence.
- OCR inside another window, display-only OCR, media/background targets, Smalltalk product echo, and missing active-window evidence can block or downgrade a task action.
- `gate_task_action_attribution` can demote an `encountering_error` action to support/reviewing output when ownership is not clear.

This is intentional. It reduces false confident answers, but it can also leave the final handoff thin.

## Continue Schema

Main Continue tables:

| Table | What it stores |
| --- | --- |
| `continue_schema_migrations` | Continue schema version marker. |
| `continue_artifacts` | Stable work objects such as docs, URLs, code editors, terminals, PDFs, chats, messages, and fallback surfaces. |
| `continue_artifact_observations` | Per-frame observations of artifacts. |
| `continue_task_actions` | Derived user/task actions. |
| `continue_task_action_events` | Join table from task actions to raw events. |
| `continue_semantic_moments` | Meaningful boundaries and deltas across frames/events. |
| `continue_boundary_revisions` | Revision markers that invalidate stale decisions when semantic boundaries change. |
| `continue_episodes` | Adjacent actions grouped into episodes. |
| `continue_episode_actions` | Episode-to-action joins. |
| `continue_episode_artifacts` | Artifact roles inside an episode. |
| `continue_workstreams` | Durable clusters of related episodes and artifacts. |
| `continue_workstream_episodes` | Workstream-to-episode joins. |
| `continue_workstream_artifacts` | Durable artifact roles inside a workstream. |
| `continue_workstream_state_snapshots` | State snapshots for workstream transitions. |
| `continue_workstream_edges` | Relationships between workstreams. |
| `continue_open_loops` | Unfinished/completed/blocked state derived from workstreams. |
| `continue_candidates` | Scored candidate return targets and scoring components. |
| `continue_decisions` | Persisted Continue decisions and final handoff lines. |
| `continue_feedback_events` | Explicit and inferred feedback. |
| `continue_breadcrumbs` | Manual local next-step notes attached to workstreams. |
| `continue_evidence_probes` | Bounded probe attempts for missing evidence. |
| `continue_memory_cells` / retrieval summaries | Memory support/contradiction features used in ranking. |

The Continue layer is local and can be rebuilt without stopping capture.

## Semantic Layer: Artifacts And Actions

`rebuild_continue_second_layer` loads evidence frames and event-only evidence, then rebuilds artifacts, observations, task actions, semantic moments, and related joins.

Artifacts are normalized work objects. They may represent:

- Browser tabs.
- Code editors.
- Terminals.
- Documents.
- PDFs.
- Chat conversations.
- Messaging surfaces.
- Files.
- Fallback frame surfaces.

Task actions are inferred local actions. Examples include:

- `editing`
- `composing`
- `running_command`
- `observing_command_output`
- `reviewing_output`
- `encountering_error`
- `searching`
- `reading`
- `copying_evidence`
- `branching_away`
- `idle_after_progress`
- `unknown`

Task actions can be collapsed when adjacent actions are effectively the same. Collapsing reduces noise but can also remove specificity from the final handoff when the only remaining action is generic.

## Semantic Moments

Semantic moments are meaningful boundaries and deltas. They exist because raw frame/event sequences are too low-level for Continue.

They can represent:

- Meaningful content change.
- Boundary after progress.
- Event-only meaningful activity.
- App/work-surface transitions.
- Stale or invalidating evidence markers.

Semantic moments feed boundary revisions, open loops, cache freshness, and candidate generation. A high-value UI event can invalidate cached decisions even when there is no new screenshot.

## Episodes And Workstreams

`rebuild_continue_third_layer` groups task actions into episodes and workstreams.

Episodes group adjacent actions with related artifact context. Workstreams cluster episodes and artifacts into durable work context.

Workstream states include:

| State | Meaning |
| --- | --- |
| `active` | Recent non-stale workstream. |
| `suspended` | Work appears paused with unresolved state or branch/idle boundary. |
| `resumed` | Evidence shows return to origin. |
| `stale` | Workstream is older than newer activity by more than the threshold. |
| `abandoned` | Single interruption-like workstream. |
| `background` | Recent enough to exist but not selected as active/suspended. |

Artifact roles inside workstreams matter. A browser tab, terminal, chat, or search page can be evidence for another target instead of being the target itself.

## Open Loops

`continue_open_loops` sits between workstreams and candidates.

Open loops summarize unresolved or completed state:

- What the workstream appears to be.
- Last concrete progress.
- Unfinished state.
- Next evidence-backed action when known.
- Current focus relation.
- Boundary kind.
- Quality.
- Evidence spans.

This layer is supposed to prevent the engine from treating every recent surface as equally actionable. If open loops are missing or generic, candidate generation becomes weaker and the final output often becomes vague.

## Current Surface Resolver

`resolve_current_surface(...)` is the current-focus/freshness resolver. It is more important than the older `load_continue_current_focus(...)` fallback.

It fuses recent evidence from:

- Frames.
- Artifact observations.
- App contexts.
- UI events.
- Window snapshots.
- Typing bursts.

It groups rows by surface identity, scores them, excludes Smalltalk/self/debug surfaces where possible, and selects the best current surface. It also records rejected surfaces and warnings.

Important behavior:

- It can select a current surface backed by events without a heavy frame.
- It warns with `current_surface:event_backed_no_screenshot` when selected evidence has UI events but no frame.
- It warns with `current_surface:latest_surface_is_self_or_debug` when the newest observed surface is Smalltalk/debug output.
- It can return `no-clear-current-surface`.
- It is returned in `ContinueDecisionResult.current_surface_resolution`.

This means current focus is not just "latest screenshot." It is a fused local guess. It can be right while the screenshot shown in an evidence panel is old, and it can also be wrong if event/window/app evidence is noisy.

## Evidence Freshness Ledger

`build_evidence_freshness_ledger(...)` compares the selected candidate against current surface evidence.

It records:

- Latest any evidence timestamp.
- Latest non-self evidence timestamp.
- Latest heavy-frame timestamp.
- Latest event timestamp.
- Latest fresh openable timestamp.
- Selected candidate evidence timestamp.
- Selected candidate age.
- Whether newer non-self evidence exists than the selected target.
- Whether the selected target is stale.
- Warnings.

Key behavior:

- If newer non-self evidence exists after the selected candidate, it warns.
- If the selected target is an old frame fallback with no URL/path and newer non-self evidence exists, the engine can downgrade from strong output to thin output.
- It is returned in `ContinueDecisionResult.evidence_freshness_ledger`.

This ledger is one of the first places to inspect when the answer points to old work.

## Final Decision Layer

`get_continue_decision` is the core product engine.

Default backend request behavior after normalization:

- `lookback_ms`: 45 minutes.
- `limit`: 700.
- `mode`: `normal`.
- `rebuild_layers`: `false`.
- `micro_inference_enabled`: `true` unless explicitly disabled by the caller.
- `max_candidates_for_model`: 5.
- `audit_output_enabled`: `false`.

The live UI currently passes `micro_inference_enabled: true`.

Normal mode may reuse a cached decision when:

- Rebuild is not forced.
- The cached decision matches the requested inference policy.
- The cached decision has a matching evidence watermark.
- Boundary revisions and freshness checks do not invalidate it.

If there is no cache hit, the engine may infer feedback for pending prior decisions, then rebuild semantic layers when needed.

Current decision flow:

1. Ensure Continue schema.
2. Normalize request defaults.
3. Decide normal vs rebuild mode.
4. Build current evidence watermark.
5. Try fresh cached decision if allowed.
6. Infer feedback for pending prior decisions when not cached.
7. Rebuild layer 2 and layer 3 if needed.
8. Resolve current surface.
9. Build `current_focus` from resolved surface or fallback loader.
10. Load scorer workstreams.
11. Build app-activity intelligence.
12. Rebuild evidence watermark after semantic work.
13. Generate local candidates.
14. Apply app-activity features.
15. Build memory retrieval query and retrieve memory cells.
16. Apply memory features.
17. Apply feedback ranking priors.
18. Score and sort candidates.
19. Build Continue dossier.
20. Persist candidates when not cached.
21. Select local candidate after app-activity adjustments.
22. Evaluate quality gate and initial output mode.
23. Optionally build model pack and call OpenAI.
24. Validate model output.
25. Apply model selection only if validation passes or soft recovery is allowed.
26. Build evidence-freshness ledger.
27. Block stale/risky strong outputs.
28. In local fallback, optionally select a safer candidate.
29. Block candidates with no human return target.
30. Compose handoff.
31. Suppress handoff copy with internal ids.
32. Gate public `return_target` and `resume_work_target`.
33. Persist decision.
34. Build evidence anchors and alternatives.
35. Return `ContinueDecisionResult`.

## Candidate Generation

Candidates are generated from workstreams, unresolved signals, open loops, last meaningful actions, current focus, and primary artifacts.

Candidate kinds include:

| Candidate kind | Typical reason |
| --- | --- |
| `resolve_error` | Visible error or failure is unresolved. |
| `continue_edit` | Editing or idle-after-progress on editable artifact. |
| `continue_reply` | Draft/composer/chat/message is active or recently meaningful. |
| `rerun_command` | Last meaningful action was command execution. |
| `verify_output` | Verification/output branch has not returned. |
| `review_completed_changes` | Completed work needs review. |
| `commit_completed_changes` | Completed work appears ready to commit. |
| `manual_verify_app_behavior` | App behavior should be checked manually. |
| `resume_chat_reasoning` | Chat conversation appears to be reasoning/work context. |
| `read_next_source` | PDF/source reading is the best continuation. |
| `finish_search` | Search has no clear primary target. |
| `return_to_primary_artifact` | Branch/support surface is evidence; return to the original artifact. |
| `evidence_only` | Evidence exists but is too thin to make a strong actionable claim. |

Candidate generation intentionally resists promoting support branches. A search page, chat, terminal, or diagnostic output should not become the default return target unless local evidence says it became the primary work.

## Candidate Scoring And Risk Gates

Candidate scoring is local and weighted. It uses:

- Actionability.
- Primary-target support.
- Unresolved state.
- Branch-origin relation.
- Evidence quality.
- Openability.
- Privacy safety.
- Recency.
- Memory support and contradiction.
- Feedback priors.
- Work value.
- Resume likelihood.
- Divergence/diagnostic signals.
- Objective relation.
- Interaction depth.
- Evidence sufficiency.

Risk gates and caps can demote candidates when:

- The target is evidence-only.
- There is no target artifact.
- The target is unknown/thin.
- The target is only a frame fallback.
- The target has no URL or document path.
- The target is Smalltalk/self/debug output.
- Evidence quality is low.
- Last meaningful action is missing or too generic.
- Current focus conflicts with return target.
- The candidate is a support branch without strong proof it became primary.
- App-activity classification marks the surface as divergence, diagnostic-only, interruption, background consumption, support context, current-focus-only, suppressed, or needing fresh capture.

Openability is only one signal. A target with a URL is not automatically the right target. A target without a URL/path may still be inspectable through a frame, but it is riskier and often cannot be exposed as a public return target.

## Quality Gate And Output Modes

Each selected candidate is evaluated through `ContinueDecisionQualityGate`.

The gate checks:

- `target_grounded`
- `target_openable_or_inspectable`
- `last_state_specific`
- `next_action_specific`
- `current_vs_return_relation_clear`
- `evidence_has_boundary`
- `evidence_has_content_delta`
- `source_provenance_clear`
- `fatal_missing`
- `warnings`

Output modes:

| Mode | Meaning |
| --- | --- |
| `strong_continue` | Enough grounded evidence exists for a recommendation. |
| `thin_continue` | There is a best available target, but evidence is weak or incomplete. |
| `no_clear_continuation` | No reliable human return target is grounded. |

Confidence caps:

- `strong_continue`: can use the selected score.
- `thin_continue`: capped around thin confidence.
- `no_clear_continuation`: capped lower and should not expose a normal return target.

The local handoff intentionally becomes generic when output mode is thin or no-clear. That is not just copy. It reflects missing or blocked evidence.

## Optional Bounded Micro-Inference

Micro-inference is enabled by the current UI and island, but it is still bounded and optional at the backend request level.

When enabled, Smalltalk builds a compact pack:

- Schema: `smalltalk.continue_micro_inference_pack.v2`.
- Current focus summary.
- Current surface summary.
- Selected workstreams.
- Candidate ids.
- Candidate kinds.
- Target artifact ids/titles/kinds.
- URL/path availability booleans.
- Local score components.
- Last meaningful action summary.
- Open-loop or unresolved-state summary.
- Evidence frame/action/episode ids.
- Missing evidence notes.
- Memory support/contradiction scores.
- App-activity fields.
- Continuation role and why-not-primary fields.
- Evidence packs v2.
- Artifact roles.
- Breadcrumbs.
- Continue dossier.

The model does not receive:

- Raw screenshots.
- Broad raw history.
- Raw event timelines.
- Raw typed characters.
- Full clipboard text.
- Arbitrary local file contents.
- Permission to invent URLs, paths, artifacts, evidence, intent, or next actions.

The request uses the OpenAI Responses API with Structured Outputs. The default model is `gpt-4.1-mini`, unless overridden by:

- `SMALLTALK_CONTINUE_OPENAI_MODEL`
- `SMALLTALK_OPENAI_MODEL`
- `OPENAI_MODEL`

The API key can come from process environment or project `.env`.

The model can return:

- `selected_candidate`
- `need_more_evidence`
- `no_clear_continuation`

Validation rejects or downgrades output when:

- Selected candidate id was not sent to the model.
- Selected workstream id does not match the selected candidate.
- Output contains unsupported URLs or paths.
- Output exposes internal ids or frame/candidate references.
- Required handoff lines are missing or too long.
- `why_this` is empty or has too many items.
- `next_action` is incompatible with the candidate.
- High confidence is returned for thin local evidence.
- Candidate evidence sufficiency is too low.
- A suppressed candidate role is selected.
- A branch/support target is promoted without strong local support.
- The result claims an escape hatch but still selects a candidate.

Some presentation failures are soft-recoverable: the engine can keep the selected candidate but replace model copy with local-safe copy and cap confidence. Hard failures produce local fallback behavior.

Micro-inference cannot rescue bad local candidates. If candidate generation or recall omits the real work, the model never sees it.

## Handoff Composition

The final user-facing copy is `ContinueHandoff`.

Fields:

- `headline`
- `return_line`
- `current_focus_line`
- `last_state_line`
- `next_action`
- `why_this`
- `missing_evidence_line`
- `confidence_label`
- `user_visible_uncertainty`

Rules:

- Thin/no-clear output uses local handoff copy, not model copy.
- Strong output can use model copy only after validation.
- Internal ids, candidate ids, frame ids, artifact ids, and raw frame-fallback phrases are suppressed.
- If no reliable human return target is grounded, the handoff says so.
- If evidence is thin, the handoff says evidence is thin instead of pretending.

This means vague copy can be the intended result of the gates. The deeper question is why the engine lacked a specific grounded candidate or why it blocked the evidence it had.

## Continue Decision Output

`get_continue_decision` returns `ContinueDecisionResult`.

Important fields:

| Field | Meaning |
| --- | --- |
| `decision_id` | Stable id for this decision/evidence state. |
| `mode` | Effective mode, normally `normal` or `rebuild`. |
| `cache_hit` | Whether this came from a fresh cached decision. |
| `source` | `local_scorer`, `cloud_micro_inference`, or `local_fallback`. |
| `model` | Model name when micro-inference was configured. |
| `response_id` | OpenAI response id when a valid cloud response exists. |
| `current_focus` | Factual current/recent surface summary. |
| `current_activity` | Local summary of current behavior. |
| `selected_workstream` | Workstream chosen as continuation context. |
| `return_target` | Public target Smalltalk is willing to expose. May be null. |
| `resume_work_target` | Actionable work target, separate from support evidence. May be null. |
| `candidate_kind` | Kind of selected continuation candidate. |
| `last_meaningful_action` | Best local action evidence supporting the candidate. |
| `unresolved_state` | Human-readable unresolved state when present. |
| `next_action` | Suggested next step or fallback instruction. |
| `confidence` | Numeric confidence after scoring and caps. |
| `confidence_label` | User-facing confidence bucket. |
| `evidence_anchors` | Frame/action/episode/workstream ids for inspection. |
| `missing_evidence` | What is absent or weak. |
| `warnings` | Caveats and gates that affected the decision. |
| `validation_failures` | Model/config/validation failures. |
| `handoff` | Final user-facing handoff lines. |
| `alternatives` | Top alternative candidates. |
| `generated_candidates` | Count of local candidates generated. |
| `validation_status` | `valid`, `thin_evidence`, `fallback`, etc. |
| `continue_output_mode` | `strong_continue`, `thin_continue`, or `no_clear_continuation`. |
| `evidence_watermark_hash` | Hash used for cache/freshness decisions. |
| `latest_boundary_revision` | Latest semantic boundary revision. |
| `current_surface_resolution` | Full current-surface audit JSON. |
| `evidence_freshness_ledger` | Freshness comparison between selected target and current evidence. |
| `continue_dossier` | Local dossier used by scoring/model pack. |
| `memory_retrieval` | Memory support/contradiction report. |
| `app_activity` / `activity_summary` | Current app-activity intelligence. |
| `quality_gate` | Quality-gate booleans, fatal missing fields, warnings, and output mode. |
| `evidence_pack_v2_used` | Whether evidence pack v2 influenced quality gating. |
| `micro_inference_requested` | Whether request asked for model path. |
| `micro_inference_attempted` | Whether a model call was actually attempted. |
| `micro_inference_result_kind` | Model result kind when available. |
| `continue_output_path` | Optional audit path set by `capture.rs` when audit output is scheduled. |
| `audit_inference_events` | Inference request/response/validation audit events, without API keys. |

## Persisted Decisions

When a decision is not served from cache, it is inserted into `continue_decisions`.

Persisted rows store:

- Decision id.
- Requested timestamp.
- Source.
- Current focus frame/artifact.
- Selected workstream.
- Selected candidate.
- Public return target artifact.
- Confidence.
- Decision reason.
- Next action.
- Warnings.
- Validation status.
- Response id and model when available.
- Validation notes.
- Handoff lines.
- `continue_decision_quality_json`.
- `continue_output_mode`.
- Evidence pack flag.
- Evidence watermark JSON/hash.
- Latest boundary revision.
- Micro-inference requested/attempted/result kind.
- Continue dossier JSON.
- Memory retrieval summary JSON.

Persisting decisions matters because `open_resume_point` and feedback attach to exact decision ids.

## UI Presentation

The React Continue card renders one answer.

Key behavior:

- It prefers `decision.handoff` over older presentation fallback.
- It chooses `resume_work_target || return_target` as the resume target.
- It disables the primary open button when there is no direct openable target.
- It shows current focus only when current focus differs from the resume target or warnings indicate mismatch.
- It productizes internal warnings for visible copy.
- It can show low-confidence styling when confidence is low or copy looked internal.

The `Why this?` panel is not the same as the old resume-query bundle. It displays:

- Workstream reason.
- Return target.
- Current focus.
- Last meaningful action.
- Warnings/missing evidence/validation failures.
- The currently selected frame/image state in React when evidence inspection is open.

If the panel shows a wrong screenshot, debug the selected React frame and `decision.evidence_anchors` first. Do not assume `resume_query_exports/.../current-focus.jpg` is involved unless the path in the UI or audit output actually points there.

## Audit Output

`audit_output_enabled` is handled in `capture.rs`, not inside the core `continuation.rs` decision function.

Flow:

1. `capture.rs` calls `continuation::get_continue_decision`.
2. If `audit_output_enabled` is true, `capture.rs` schedules `schedule_continue_output_audit`.
3. The audit writes under repo-local `continue_outputs/`.
4. `result.continue_output_path` is set to the planned/final audit directory.
5. The export runs asynchronously.

The current audit is lean by default. It focuses on decision proof rather than dumping every raw table and frame. Important audit entrypoints include:

- `explain.md`
- `decision/decision_trace.json`
- `decision/final_decision.json`
- `decision/current_surface_resolution.json`
- `decision/evidence_freshness_ledger.json`
- `decision/candidates.json`
- `final/handoff.json`
- `model/validation.json`
- `cache/cache_decision.json`
- `evidence/evidence_closure.json`

Full raw dumps are disabled by default because they duplicate local capture data and are not useful as normal LLM-readable evidence.

## Developer Trace

`get_continue_decision_trace` is the developer-safe route for inspecting a persisted decision without dumping unsafe raw history.

It returns:

- Evidence watermark.
- Latest frames.
- Latest events.
- Semantic moments.
- Artifacts.
- Task actions.
- Episodes.
- Workstreams.
- Workstream state snapshots.
- Workstream edges.
- Open loops.
- Candidates.
- Scoring.
- Quality gate.
- Micro-inference summary.
- Final handoff.
- Warnings.
- Missing evidence.

Use this when the app output is vague and you need to prove whether the problem is capture, semantic extraction, candidate generation, scoring, model validation, or UI presentation.

## Legacy And Diagnostic Paths That Confuse Debugging

### `resume_query_exports`

`resume_query_exports/` contains generated stop-time bundles for the older cloud resume path and debugging flows. Images named like `current-focus.jpg`, `origin.jpg`, `resume-candidate.jpg`, or `side-branch.jpg` belong to that bundle path.

That path is not the primary live Continue engine.

If the visible UI is showing something that looks like `resume_query_exports/.../current-focus.jpg`, identify why:

- Is the UI actually using a stop/cloud-resume artifact?
- Is an audit/export viewer open?
- Is the image from a prior generated folder?
- Is the island or open fallback using older resume-card behavior?
- Is React showing a selected frame from `evidence_anchors`, not the resume-query bundle?

Do not treat `resume_query_exports/current-focus` as proof of the live `current_focus` field. They are different concepts.

### `run_cloud_resume`

`run_cloud_resume` builds a bounded resume-query bundle and can call OpenAI for the older Ask OpenAI path. It can return `source = "cloud"` when a real response exists, or fallback data when it cannot safely use the cloud result.

It is not the core `get_continue_decision` engine.

### `get_native_resume_card`

`get_native_resume_card` is older local resume-card behavior. It can still be used as fallback by open/resume paths when Continue/cloud targets are unusable. It should not be treated as the primary product primitive.

### Floating Island

The floating island is a native macOS consumer of Continue/capture state.

Current behavior:

- It can request `get_continue_decision`.
- It remembers the latest decision id.
- It emits `smalltalk-continue-updated` back to React.
- It sets island fields such as `continue_decision_id`, `continue_freshness`, `continue_openable`, and evidence timestamps.
- It derives a compact target from `resume_work_target || return_target`.

The island can drift from backend truth if its snapshot fields or Swift wording lag behind current backend semantics. Treat it as a consumer surface, not as the source of truth.

### Browser Extension

`browser-extension/` preserves an older browser-extension prototype. It is not the active native MVP path and should not be revived for current Continue work unless explicitly requested.

## Where Vagueness Enters

This section is the root-cause map for vague or bad output.

### 1. Fresh Events Without Fresh Screenshots

Current surface may be event-backed without a fresh heavy frame. The engine may know the app/window changed from UI events, while the screenshot available for inspection is older. This can make the output and screenshot look misaligned.

Inspect:

- `current_surface_resolution.selected.evidence_kinds`
- `current_surface:event_backed_no_screenshot`
- Latest UI event timestamp.
- Latest heavy frame timestamp.
- React selected frame/image state.

### 2. Stale Frame Fallback Targets

A selected target may be only a `frame_fallback` with no browser URL or document path. If newer non-self evidence exists, the freshness ledger can downgrade it.

Inspect:

- `evidence_freshness_ledger.stale_selected_target`
- `evidence_freshness_ledger.has_newer_non_self_than_selected`
- `evidence_freshness_ledger.selected_candidate_age_ms`
- Candidate target `openability`
- `warnings` containing `evidence_freshness:*`

### 3. OCR Or AX Attribution Blocks Evidence

The system may have text, but refuse to use it as active evidence because it is mixed, background-owned, display-only, or not artifact-owned.

Inspect:

- `frame_text_resolutions`
- `active_text`
- `background_text`
- `full_text_quality`
- `quality_flags_json`
- `evidence_attribution_json`
- `attribution_confidence`
- `classifier_context_json`

### 4. Semantic Actions Are Missing Or Generic

If the engine only extracts `unknown`, `reading`, repeated states, or low-confidence support actions, the workstream and open loop will be vague.

Inspect:

- `continue_task_actions`
- `action_kind`
- `action_role`
- `confidence`
- `semantic_delta_kind`
- `semantic_after_hint`
- `quality_flags_json`

### 5. Open Loops Are Weak

If `continue_open_loops` lacks `last_concrete_progress`, `unfinished_state`, or `next_evidence_backed_action`, the quality gate often cannot produce strong output.

Inspect:

- `continue_open_loops`
- `boundary_kind`
- `quality`
- `last_concrete_progress`
- `unfinished_state`
- `next_evidence_backed_action`
- `current_focus_relation`

### 6. Candidate Recall Misses The Real Work

The model only sees candidates selected by `select_recall_first_candidates`. If local generation/ranking misses the true target, the model cannot select it.

Inspect:

- `generated_candidates`
- `alternatives`
- `continue_candidates`
- Candidate `continuation_role`
- Candidate `why_not_primary`
- Candidate score components.
- `memory_support_score` and `memory_contradiction_score`
- `feedback_prior_score`

### 7. Quality Gate Downgrades The Answer

The quality gate can force `thin_continue` or `no_clear_continuation` even when a candidate exists.

Inspect:

- `quality_gate.output_mode`
- `quality_gate.fatal_missing`
- `quality_gate.warnings`
- `target_grounded`
- `last_state_specific`
- `next_action_specific`
- `evidence_has_boundary`
- `evidence_has_content_delta`
- `source_provenance_clear`

### 8. Public Target Gating Removes The Target

The engine can select a diagnostic/local candidate internally but refuse to expose it as `return_target` or `resume_work_target`.

Inspect:

- `candidate_kind`
- `continue_output_mode`
- `warnings`
- `return_target`
- `resume_work_target`
- selected candidate target artifact in trace/audit.

### 9. Model Validation Falls Back

A model response can be real but rejected or soft-recovered. In that case `source`, `response_id`, and handoff copy need careful reading.

Inspect:

- `source`
- `response_id`
- `micro_inference_requested`
- `micro_inference_attempted`
- `micro_inference_result_kind`
- `validation_failures`
- `audit_inference_events`
- `model/validation.json` in audit output.

### 10. UI Presentation Selects The Wrong Evidence Image

The final decision and the visible screenshot can diverge in the React evidence panel.

Inspect:

- `decision.evidence_anchors.frame_ids`
- selected React frame id.
- `get_frame_image` / `get_frame_image_variant` result.
- Whether the panel is showing a current-focus frame, selected-candidate frame, or manually selected timeline frame.
- Whether the image path belongs to `continue_outputs`, `resume_query_exports`, or live app-data snapshots.

## Debugging Checklist For Bad Output

Start with the returned `ContinueDecisionResult`.

Check:

- `decision_id`
- `source`
- `response_id`
- `cache_hit`
- `mode`
- `validation_status`
- `continue_output_mode`
- `confidence`
- `current_focus`
- `current_activity`
- `selected_workstream`
- `resume_work_target`
- `return_target`
- `candidate_kind`
- `last_meaningful_action`
- `warnings`
- `missing_evidence`
- `validation_failures`
- `quality_gate`
- `current_surface_resolution`
- `evidence_freshness_ledger`
- `micro_inference_requested`
- `micro_inference_attempted`
- `micro_inference_result_kind`

If the output is vague:

1. Confirm whether the mode is `thin_continue` or `no_clear_continuation`.
2. Read `quality_gate.fatal_missing` and `quality_gate.warnings`.
3. Check whether `return_target` and `resume_work_target` are null.
4. Check whether the selected target is stale, frame-fallback-only, or not openable.
5. Check whether current surface is event-backed without screenshot evidence.
6. Check whether text was blocked by `frame_text_resolutions` or attribution flags.
7. Check whether open loops have concrete unfinished state.
8. Check whether the real target appears only in alternatives or not at all.

If `Why this?` shows the wrong screenshot:

1. Identify the actual image path or frame id displayed.
2. Compare it with `decision.evidence_anchors.frame_ids`.
3. Compare it with `current_focus.frame_id`.
4. Compare it with selected candidate `evidence_frame_id` in trace/audit.
5. Check whether React kept a previously selected frame.
6. Check whether the image comes from app-data snapshots, `continue_outputs`, or `resume_query_exports`.
7. If it is `resume_query_exports/.../current-focus.jpg`, you are likely looking at legacy stop/cloud-resume output, not the live Continue decision.

If the answer claims model involvement:

1. Check `micro_inference_requested`.
2. Check `micro_inference_attempted`.
3. Check `source`.
4. Check `response_id`.
5. Check `validation_failures`.
6. Check `micro_inference_result_kind`.
7. Check whether handoff copy was local because output mode was thin/no-clear.

If capture seems misaligned:

1. Compare latest heavy frame timestamp.
2. Compare latest meaningful UI event timestamp.
3. Compare latest non-self evidence timestamp.
4. Compare selected candidate evidence timestamp.
5. Check whether Smalltalk/debug surfaces were excluded.
6. Check whether the current surface was selected from events, typing, windows, app contexts, or frames.

## Known Limitations And Systemic Risk

These are current risks, not future promises.

- The engine is heavily dependent on local candidate quality. If candidate generation misses the real work, scoring and micro-inference cannot recover it.
- Screenshots are sparse and may lag event evidence. A screenshot can be useful evidence while still being the wrong visual explanation for the current output.
- Current focus and return target are intentionally separate, but UI/debug artifacts can still conflate them.
- The engine can know that something changed without having enough active text to name the task.
- OCR can be blocked for good reasons and leave the handoff vague.
- Accessibility can be missing, stale, or too generic.
- Workstream/open-loop summaries can become generic when actions collapse or when semantic deltas are weak.
- Public target gating can hide an internally selected candidate, producing a no-clear handoff.
- The island can lag backend semantics because it is a consumer surface.
- `continue_outputs` and `resume_query_exports` are diagnostic/generated outputs and can confuse interpretation if treated as the live product path.
- A vague handoff is often a symptom of missing/blocked evidence or candidate-generation failure, not just bad copy.

## Product Doctrine To Preserve

- `Continue` is the primary product primitive.
- Sessions, screenshots, exports, timelines, and raw events are evidence or diagnostics.
- `Stop Session` must not be required for `Continue`.
- Current focus is factual and must stay separate from return target.
- Branch/support surfaces are evidence unless local proof says they became primary.
- Raw typed characters are not stored.
- Full clipboard text is not stored.
- Model calls, when enabled, must remain candidate-bounded and locally validated.
- The engine must not invent URLs, paths, artifacts, user intent, or next actions.
- If evidence is thin, say it is thin.

## Quick Verification Commands

P6 release evaluation is deliberately stricter than the per-phase milestone manifest. Generate the privacy-safe machine report with:

```bash
cd src-tauri
cargo run --bin continue_accuracy_eval -- \
  --output tests/fixtures/continue_accuracy/release-report.json \
  --repeat 3
```

Read `release_gate.passed`, not only `milestone_contract_passed`. Missing labeled denominators, human adjudication, locked-holdout evidence, calibration samples, performance budget, or manual macOS QA keep the release gate closed.

For docs-only changes:

```sh
git diff -- docs/full-engine-flow.md
rg -n "micro_inference_enabled|resume_query_exports|current_surface_resolution|evidence_freshness_ledger|continue_output_mode|frame_text_resolutions|Why this" docs/full-engine-flow.md
git diff --check docs/full-engine-flow.md
```

No frontend or Rust build is required for this doc-only update.

## Bounded runtime scheduling and storage

The production flow now has explicit pressure boundaries before semantic processing:

1. `capture_events.swift` summarizes native callback floods without recording characters.
2. Rust admits each JSON line to a reserved high, normal, or pressure lane. Total capacity is 320.
3. One capture-loop turn consumes at most 32 events and yields after a 12 millisecond drain budget.
4. The capture worker writes the batch through one generation-owned SQLite connection and one short transaction.
5. The transaction persists events, typing effects, capture-trigger links, and maintained session counters together. Cancellation rolls the batch back.
6. A pending trigger retains 128 causal ids plus a versioned aggregate mapping instead of growing forever.
7. Capture, Continue, audit, and maintenance enter the finite workload governor before expensive work starts.

The governor has a total waiting capacity of 48 plus smaller class limits. Manual Continue cancels queued background and island work. Capture and manual actions can displace queued maintenance, audit, or derived work. Every waiter has a deadline and cancellation token. Shutdown cancels active and queued tokens and wakes all waiters.

Continue opens its preflight connection only long enough to resolve session scope, establish the manual capture boundary, and compute the semantic watermark. It drops that connection before waiting for admission. After admission it opens a fresh connection. A superseded background result is rejected before audit or visible-surface publication. Unchanged semantic evidence continues to reuse the existing decision identity.

Full audit output is manual-only. One audit worker owns one active export and one pending export. Equivalent decision ids coalesce. A newer pending decision supersedes the older pending job. Export construction remains in a temporary directory and becomes complete only through atomic rename.

`capture_status` reads maintained session counters, bounded recent signal windows, one maintenance snapshot query, and a lightweight latest-frame row. The status frame contains identity and provider metadata but not OCR text, Accessibility text or trees, URL/path contents, or image paths. Explicit evidence commands load heavy detail. Status records p50 and p95 latency and response bytes.

The privacy-safe soak harness is described in `docs/runtime-stability-harness.md`. The complete live matrix remains a release gate; deterministic tests alone do not establish always-on stability.
