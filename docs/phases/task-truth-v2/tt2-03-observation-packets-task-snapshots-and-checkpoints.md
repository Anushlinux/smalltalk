# Task Truth v2.03 — Observation Packets, Task Snapshots, And Semantic Checkpoints

## Codex task

Create the versioned Task Truth v2 domain model and a shadow production path that preserves multimodal temporal evidence until task understanding is complete.

Do not duplicate P6 structures under new names. Reuse proven capture, privacy, target-safety, evidence-handle, confidence, audit, and task-turn work where semantics match. Add only the missing contracts required for causal, multimodal task understanding.

This path must remain shadow-only in this goal. It must not replace the visible Continue answer yet.

## Dependency gate

Task Truth v2.01 and v2.02 must be complete. Read their implementation reports and current code.

Inspect:

```text
AGENTS.md
PRODUCT.md
docs/full-engine-flow.md
docs/phases/task-truth-v2/tt2-01-causal-evidence-and-containment.md
docs/phases/task-truth-v2/tt2-02-live-corpus-and-shadow-evaluation.md
src-tauri/src/continuation.rs
src-tauri/src/continuation/task_turn.rs
src-tauri/src/continuation/task_turn_evidence.rs
src-tauri/src/continuation/confidence.rs
src-tauri/src/continuation/activity_recap_truth.rs
src-tauri/src/capture.rs
```

Search for `EvidenceFrame`, `EvidenceContentUnit`, `OrderedEvidenceSpan`, `CurrentTaskTurn`, `ContinueConfidenceVector`, frame diffs, typing bursts, UI events, capture triggers, event transitions, and current audit tables.

## Architectural rule

Task truth must be established before workstream selection, candidate ranking, and return-target resolution.

```text
capture evidence
→ ObservationPacket
→ causal/semantic interpretation
→ selected TaskSnapshot
→ associated workstream and anchors
→ target resolution
→ Continue answer
```

The existence of an openable URL/path must not influence task selection.

## Non-goals

- Do not build a universal perfect scene graph before the first shadow result.
- Do not add browser-extension collection.
- Do not replace the existing SQLite capture/storage/privacy system.
- Do not perform continuous cloud inference.
- Do not delete P6 or its audit path.
- Do not expose v2 output on the first screen.
- Do not store image bytes in SQLite.

## Goal 1 — Create a focused module boundary

Add a small module tree rather than expanding `continuation.rs` or `task_turn_evidence.rs`. Use current repository conventions; a likely shape is:

```text
src-tauri/src/continuation/task_truth_v2/
  mod.rs
  observation_packet.rs
  task_snapshot.rs
  checkpoint.rs
  selection.rs
  audit.rs
```

Later goals may add resolver and verifier modules. Keep provider/network code outside domain contracts.

## Goal 2 — Define `ObservationPacket` as a view over existing evidence

Create `smalltalk.observation_packet.v2`. It should reference existing capture rows/assets instead of copying them.

Required semantics:

```text
packet_id
observed_at_ms
session_id
active surface identity
current frame reference and privacy status
2-4 selected semantic keyframe references
AX/content/OCR element references with bounds/order/source ownership
focused/editable/selected element references
window/app/tab/file/document metadata
causal typing/submit/click/focus/command events
capture triggers and before/after transitions
frame/change-region diffs
existing return-anchor facts, isolated from task inference
previous valid TaskSnapshot reference
evidence quality and missing-source notes
```

The packet builder must:

1. be deterministic for a fixed evidence watermark;
2. apply privacy filtering before model eligibility;
3. select keyframes by semantic boundary/change, not simple recency alone;
4. keep current, prior, background, and support evidence explicitly separate;
5. preserve conflicting source observations rather than flattening them;
6. emit bounded data and size accounting;
7. work locally without any model.

Screenshot paths must remain local and short-lived/model-eligible only under existing privacy policy. Audit exports should store hashes, metadata, redacted thumbnails only when policy permits, and evidence handles—not private image duplication by default.

## Goal 3 — Add a minimal canonical element layer

Build only enough cross-source alignment for task reasoning. Extend/reuse `EvidenceContentUnit` and `OrderedEvidenceSpan` where possible.

Each canonical element needs:

```text
element_id
frame_id
bounds
text or redacted text reference
visual description when later supplied
native role/subrole/actionability
region role
focused/editable/selected/interactive flags
parent/child relations when known
AX/OCR/visual source votes
source conflicts
first seen / changed time
authorship status and causal evidence refs
task eligibility status and rejection reasons
```

Required region vocabulary includes primary content, user-authored content, application/agent output, composer/editor, navigation, toolbar, control, status, notification, sidebar, modal, browser chrome, terminal input, terminal output, document canvas, and unknown.

Unknown is a valid result. Do not infer authorship solely from left/right position.

## Goal 4 — Evolve, do not duplicate, `CurrentTaskTurn`

P6 `CurrentTaskTurn` already contains goal, object, lifecycle axes, relation, evidence ids, field confidence, and revision. Audit it field by field before adding `TaskSnapshot`.

Create `smalltalk.task_snapshot.v2` as either a versioned evolution or a clean projection that adds the genuinely missing semantics:

```text
snapshot_id and revision
observed_at_ms and evidence watermark
task_summary
task_kind
task_object
user_goal
app/surface/document-or-thread identity
execution_state
current_actor
waiting_on
last_meaningful_progress
unfinished_step
next_action
relation to prior snapshot
selection status
claim-level evidence references
alternative hypotheses
contradictions
confidence by field and evidence source
return-anchor candidate/status kept independent
resolver/version provenance
```

Do not copy `CurrentTaskTurn` into a second table and let both drift. Define migration, ownership, and authority explicitly:

- P6 remains legacy/read compatibility.
- v2 snapshot is the shadow semantic product.
- shared lifecycle enums have one definition where feasible.
- target fields remain subordinate and cannot raise task confidence.

## Goal 5 — Persist semantic checkpoints at meaningful boundaries

Do not reconstruct everything only when Continue is pressed. Create/update a local TaskSnapshot checkpoint on bounded semantic events:

```text
submit/Enter/send
material edit pause
app/window/document/tab switch
command execution
visible error or blocker
agent/application output completion
idle after meaningful progress
explicit user correction
manual Continue
```

Checkpointing rules:

1. Run local packet construction continuously as permitted; cloud interpretation remains off unless explicitly enabled later.
2. Deduplicate semantically unchanged checkpoints.
3. Preserve revision lineage and supersession.
4. Store the final meaningful pre-switch packet/snapshot.
5. Never let background support activity silently supersede the primary task.
6. Bound retention and use existing cleanup/privacy policy.
7. Persist uncertainty rather than carrying a stale precise task forward.

If a meaningful boundary lacks enough evidence for a new task, preserve the previous valid snapshot only with an explicit continuity relation and confidence decay. Otherwise create an unresolved checkpoint.

## Goal 6 — Select unfinished task snapshots before targets

Implement a local shadow selector that ranks TaskSnapshots using task evidence only:

```text
temporal continuity
causal user action
unfinished/blocked/waiting state
explicit correction
surface continuity
semantic relation to prior snapshot
confidence and contradictions
```

Do not include URL existence, path existence, openability, artifact richness, or candidate score in this selection.

Only after a snapshot is selected may the system associate existing workstreams, open loops, and return anchors. Record when legacy candidate selection disagrees with snapshot selection.

## Goal 7 — Shadow audit without product authority

On every explicit Continue, persist a bounded audit containing:

```text
ObservationPacket summary
selected keyframe reasons
canonical elements and conflicts
causal interaction edges
TaskSnapshot hypotheses
selected snapshot or unresolved result
field evidence and confidence
legacy P6 comparison
first divergence
latency and byte/token estimates
```

Do not replace `ContinueDecisionResult` yet. Expose diagnostics only under the existing inspect/audit surface or local export.

## Required tests

1. Same watermark produces byte-stable packet semantics.
2. Keyframe selection prefers submit/switch/error/change boundaries over unchanged recency.
3. Private frames never become model-eligible.
4. AX/OCR duplicates merge without losing source conflict/provenance.
5. Controls remain task-ineligible.
6. Snapshot revisions preserve lineage across submit → waiting → response → review.
7. Support activity does not supersede the primary task.
8. Openable stale target cannot change selected snapshot.
9. No-evidence checkpoint becomes unresolved, not stale precise truth.
10. Session-013 produces a v2 packet and shadow snapshot record.

## Verification

```text
cargo test continuation::task_truth_v2
cargo test continuation::accuracy_eval
cargo test continuation::task_turn
cargo check
cargo fmt --check
```

Run the v2.02 shadow evaluator and confirm path C reaches packet/snapshot checkpoints while multimodal resolution remains explicitly `not_implemented` where applicable.

Measure packet size, build latency, checkpoint write volume, and retention. A correct unbounded packet is not production-ready.

## Definition of done

- Versioned ObservationPacket and TaskSnapshot contracts exist.
- Existing P6 concepts are reused rather than duplicated blindly.
- Keyframes, events, source conflicts, and causal links remain available until task resolution.
- Semantic checkpoints persist revisions at meaningful boundaries.
- Task selection is independent of return-target/openability features.
- The path is shadow-only and fully auditable.
- Session-013 and the live corpus execute through the new checkpoints.
- Privacy, boundedness, determinism, and old strict-open behavior are verified.

Do not mark the overall Task Truth v2 program complete.
