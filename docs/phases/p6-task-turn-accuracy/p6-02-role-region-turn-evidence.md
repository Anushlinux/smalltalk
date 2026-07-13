# P6.02 — Role, Region, And Ordered Turn Evidence

## Codex task

Preserve the structure that currently disappears between capture and Continue. Build a privacy-safe evidence layer that distinguishes active-window ownership, visual region, conversational speaker, and within-surface order so the latest user request and current agent state survive into semantic extraction.

This goal fixes evidence representation and salient snapshot selection. It does not yet make final lifecycle, feedback, workstream, recap, or UI policy decisions.

## Dependency gate

P6.01 must be complete. Confirm that the full-pipeline accuracy harness can replay the Capture-button fixture and report checkpoint divergence.

Read:

```text
AGENTS.md
PRODUCT.md
docs/phases/p6-task-turn-accuracy/p6-00-task-turn-accuracy-program.md
docs/phases/p6-task-turn-accuracy/p6-01-ground-truth-replay-eval.md
```

If the fixture/replay contract is absent or only evaluates pre-scored candidates, finish P6.01 before editing production extraction.

## Current verified failure

Capture already retains useful ingredients:

- AX nodes with hierarchy and bounds.
- OCR spans with geometry, ownership, and active-artifact match confidence.
- Content units with semantic role and bounds.
- `frame_text_resolutions` separating active, background, and diagnostic text.

The loss happens when those ingredients are flattened:

- `resolve_frame_text` joins active accessibility text and owned OCR into one string.
- Continue appends active content units into a whole-frame lowercase blob.
- semantic role classification has generic chat/composer/chrome labels but no reliable user/agent/status role.
- content-unit loading does not preserve a robust conversational reading order.
- weak-surface visible samples join the first sources and cap the first 800 characters, so old chrome and prior task text can displace the latest turn.

In the critical frame, the correct user question and current agent response were present, but the downstream action saw old `Verification passed` text as current completion.

## Files and symbols to inspect first

```text
src-tauri/src/capture.rs
src-tauri/src/continuation.rs
src-tauri/src/continuation/enrichment.rs
src-tauri/src/continuation/accuracy_eval.rs
src-tauri/src/continuation/accuracy_fixture.rs
```

Search for current equivalents of:

```text
FrameTextResolution
TextSpanProvenance
OcrSpanDraft
AxNodeSummary
OcrSpanSummary
ContentUnitSummary
EvidenceContentUnit
semantic_role_for_text
resolve_frame_text
active_frame_text_for_continue
frame_text_lower
visible_text_sources
extract_visible_main_content_sample
redact_and_cap_visible_sample
continue_surface_snapshots
visible_text_sample
content-unit SQL ordering
```

At the reviewed baseline, important seams were near `capture.rs` span/AX persistence and semantic-role logic, `continuation.rs` content-unit loading and frame-text flattening, and `enrichment.rs` visible-sample construction. Treat symbol names as authoritative.

## Required new contract

Add a versioned, serializable, auditable evidence contract such as `smalltalk.task_turn_evidence.v1`. Use repository-consistent naming while preserving these concepts.

### Ordered evidence span

Each span must carry:

```text
span_id
frame_id
surface_key or artifact_id when known
observed_at_ms
source_kind
source_record_id
source_text_reference
text_hash
bounded_public_safe_summary when needed
text_storage_class
source_scope
ownership_kind
owner_window_id
owner_app_id or bundle id
region_role
conversational_role
reading_order
geometry
parent_or_group_id
focused
selected
active_artifact_match_confidence
ownership_confidence
region_confidence
speaker_confidence
order_confidence
privacy_status
quality_flags
reason_codes
```

Do not duplicate verbatim conversational text into the new durable span table by default. Reuse already-authorized source rows ephemerally during rebuild, persist source references/hashes/roles/geometry, and store a bounded public-safe semantic summary only when a downstream contract genuinely requires durable wording. Define maximum lengths and redaction status per field. Raw keylogged text and full clipboard text remain forbidden.

### Region roles

Support at least:

```text
app_chrome
navigation
sidebar
conversation_history
user_message
agent_message
agent_status
system_status
tool_output
editor_content
terminal_input
terminal_output
composer
dialog
notification
unknown
```

### Conversational roles

Support at least:

```text
user
assistant_or_agent
system
tool
non_conversation
unknown
```

Do not collapse `agent_status` into completed assistant output. A status such as “I’ll trace the Swift bridge” usually represents active work, not a finished answer.

## Evidence construction rules

### Preserve source truth

- Prefer AX hierarchy/identifier/role/bounds when trustworthy.
- Use OCR geometry when AX is missing or structurally thin.
- Use content-unit semantic roles as evidence, not unquestioned truth.
- Merge duplicates by provenance and overlap while retaining all contributing source ids.
- Keep active-window-owned and background/display-only evidence separate.
- Do not infer speaker solely from first-person grammar or generic words such as `you` and `I`.

### Preserve order

- Derive reading order within a region using AX tree order when reliable.
- Otherwise use normalized geometry with surface-aware column/pane grouping.
- Do not globally sort a mixed multi-pane window top-to-bottom without pane segmentation.
- Record order confidence and ambiguity.
- Preserve multiple turns visible in one frame.
- Prefer later conversation turns only after identifying the conversation region; bottom-of-window text outside that region is not automatically current.

### Use bounded adapters

Add conservative adapters for surface families already identifiable locally:

- Chat/agent conversations.
- Codex/native agent surfaces.
- Browser chat surfaces.
- Code editors with side panels.
- Terminal panes.

Adapters may use AX role/identifier patterns, geometry, known local app/bundle identity, and repeated layout structure. They must fail to `unknown` when uncertain. Do not build a brittle list of one-off phrases from the golden fixture.

### Do not treat whole-frame text as a semantic unit

Whole active text may remain diagnostic evidence. It must not be the primary input for current task-state classification when ordered spans exist.

Any legacy whole-text fallback must:

- carry low attribution confidence;
- set a `flattened_text_fallback` quality flag;
- avoid confident speaker/turn claims;
- be prevented from overriding stronger ordered evidence.

## Salient snapshot selection

Replace prefix-only sampling with an evidence-aware bounded sample.

The snapshot should select, in priority order:

1. Latest high-confidence user message in the active conversation region.
2. Latest high-confidence agent response or active status following that user message.
3. Focused composer/control evidence when it represents ongoing work.
4. Task-relevant editor or terminal content.
5. Prior task boundary context when space remains.
6. Minimal identity context.

Do not simply take the last 800 characters either. The selector must choose typed spans, preserve roles/order, deduplicate repeated text, redact, and then enforce total and per-span caps.

Extend surface snapshots or add a companion object with:

```text
salient_span_ids
salient_user_goal_sample
salient_agent_state_sample
prior_boundary_sample
sampling_strategy
sampling_confidence
missing_roles
```

Do not expose raw private text to public UI or model packs merely because it was selected as salient. Existing redaction and pack policies still apply.

`salient_user_goal_sample`, `salient_agent_state_sample`, and `prior_boundary_sample` are bounded public-safe summaries or ephemeral build values, not permission to duplicate full messages. Persist source ids/hashes and a storage-class marker so audits can prove what was retained.

## Step-by-step implementation

1. Define typed enums/structs and stable serialization strings.
2. Add idempotent schema for source-linked ordered spans or the smallest durable companion representation needed by replay, rebuild, and audits, without duplicating verbatim conversation text.
3. Build spans from AX nodes, OCR spans, and content units without duplicating raw text unnecessarily.
4. Add region grouping and reading-order resolution.
5. Add conservative conversational-role assignment with explicit confidence/reason codes.
6. Add a latest-turn evidence selector that outputs candidate user/agent/status spans but does not decide task lifecycle.
7. Replace weak-surface prefix sampling with salient typed-span sampling.
8. Feed ordered spans and latest-turn candidates into the P6 accuracy checkpoints.
9. Keep legacy fallback for older databases and thin surfaces, visibly downgraded.
10. Add audit output that explains span provenance, role, order, selection, and uncertainty.

Keep new policy in focused modules, for example:

```text
src-tauri/src/continuation/task_turn_evidence.rs
src-tauri/src/continuation/task_turn_regions.rs
```

Use current module conventions and avoid adding another large block to `continuation.rs` or `capture.rs`.

## Required behavior matrix

| Input | Expected evidence result |
| --- | --- |
| Old completed assistant result above new user question | Old result is prior conversation history; new user question is latest user candidate. |
| New user question followed by agent working status | User and agent/status spans are ordered and paired; status is active work evidence. |
| Sidebar contains stale project names | Sidebar is navigation, not user goal or agent state. |
| Terminal side panel contains completion output | Terminal output remains separate from chat task unless explicit relation evidence exists. |
| AX has speaker structure but OCR duplicates text | Merge provenance; do not count duplicate turns. |
| AX is thin and OCR has geometry | Use conservative OCR region/order with lower speaker confidence. |
| Multiple panes have overlapping vertical coordinates | Segment panes before reading order. |
| Only flattened active text exists | Produce low-confidence fallback and no confident speaker role. |
| Private/sensitive surface | Redact or omit text while retaining safe role/identity metadata if permitted. |

## Tests

Add deterministic unit tests for:

1. AX-based user/agent grouping.
2. OCR-based region and reading order.
3. Multi-pane segmentation.
4. Duplicate AX/OCR merge with provenance retention.
5. Sidebar/chrome exclusion from latest user goal.
6. Agent status versus completed assistant result.
7. Prefix-truncation regression: the Capture-button question survives even when old chrome exceeds the legacy cap.
8. Tail-only regression: unrelated bottom terminal text does not become latest chat turn.
9. Unknown-role fallback and confidence downgrade.
10. Redaction and total/per-span caps.
11. Deterministic span ids and order across replay.

Add integration/full-pipeline assertions for the critical fixture:

- latest user-goal evidence contains the Capture-button task;
- current agent-state evidence contains the Swift/Rust tracing state;
- `Verification passed` remains tied to prior context;
- Stremio/Helium sidebar or stale state is not selected as latest turn evidence;
- the first divergence moves downstream from role/region extraction;
- all counterfactual variants preserve the same latest-turn evidence.

## Audit requirements

Explicit accuracy/audit output must include:

```text
ordered span summary
source and ownership
region and conversational role
reading order and grouping
confidence vector
selected latest user span ids
selected current agent/status span ids
rejected competing spans and reason codes
fallback flags
```

Keep public Continue payloads compact. Full span diagnostics remain developer evidence.

## Acceptance criteria

P6.02 is complete when:

- Ordered, role-aware evidence is available to downstream Continue logic.
- Whole-frame text is no longer the preferred semantic input when structured spans exist.
- Weak-surface snapshots retain the latest task-relevant turn rather than a prefix of chrome.
- The critical fixture's latest question and agent state are correctly selected.
- Prior completion text is structurally distinguishable from the active turn.
- Multi-pane and thin-evidence ambiguity is represented rather than guessed away.
- Privacy boundaries and bounded text caps remain intact.
- P6.01 reports measurable L1 improvement without tuning later policy layers.
- Existing capture, Continue, and P0-P5 tests still pass.

## Verification commands

Run:

```bash
cd src-tauri && cargo fmt --check
cd src-tauri && cargo check
cd src-tauri && cargo test
npm run build
git diff --check
git status --short
```

Run the accuracy evaluator and report the critical/counterfactual L1 metrics and first divergent checkpoint.

## Final response format

Report:

1. Files and schema changed.
2. Span, region, speaker, and order contracts.
3. Snapshot-selection policy.
4. Legacy fallback behavior.
5. Unit/integration fixtures added.
6. L1 metric changes with denominators.
7. Verification results.
8. Remaining lifecycle errors expected for P6.03.
9. Exact ordered-evidence contracts P6.03 may rely on.

Do not claim current-task accuracy is solved. P6.02 preserves the evidence required to solve it.
