# P6.03 — First-Class Current Task Turn And Lifecycle

## Codex task

Build the missing semantic primitive: a first-class, evidence-backed current task turn that represents the latest user goal, the current agent/user work state, its relation to the prior task, and its lifecycle. Refactor task-action and semantic-delta derivation to consume this object so old visible completion cues cannot override a newer request.

This goal establishes current-task truth. It does not yet repair stale feedback scope, workstream/open-loop consistency, P5 validation, or final UI composition.

## Dependency gate

P6.01 and P6.02 must be complete. Confirm:

- the accuracy harness reports checkpoint divergence;
- ordered evidence spans preserve region, speaker, ownership, and reading order;
- the critical fixture selects the Capture-button question and Swift/Rust tracing state as the latest turn evidence.

Read:

```text
AGENTS.md
PRODUCT.md
docs/phases/p6-task-turn-accuracy/p6-00-task-turn-accuracy-program.md
docs/phases/p6-task-turn-accuracy/p6-01-ground-truth-replay-eval.md
docs/phases/p6-task-turn-accuracy/p6-02-role-region-turn-evidence.md
```

## Current verified failure

At the reviewed baseline:

- `classify_error_context` can find a resolution/completion cue anywhere in flattened frame text.
- `classify_task_action` maps `resolved_or_verification_context` to `reviewing_output` before later composer/editing evidence can win.
- `derive_semantic_delta` can produce `completed_successfully` for a reviewing/reading frame containing any completion hint.
- the completion-hint helper scans the whole frame rather than the task turn that produced the completion.

The critical frame therefore became `reviewing_output` plus `completed_successfully` and `Verification passed`, even though the latest user had started a new Capture-button investigation and the agent was actively tracing code.

## Files and symbols to inspect first

```text
src-tauri/src/continuation.rs
src-tauri/src/continuation/task_turn_evidence.rs
src-tauri/src/continuation/task_turn_regions.rs
src-tauri/src/continuation/activity_recap_inputs.rs
src-tauri/src/continuation/activity_recap_segments.rs
src-tauri/src/continuation/accuracy_eval.rs
```

Search for current equivalents of:

```text
ContinueTaskAction
classify_task_action
classify_error_context
gate_task_action_attribution
derive_semantic_delta
dev_completion_hint_for_frame
latest_completed_action_index
collapse_task_actions
continue_task_actions
continue_open_loops
ContinueSemanticMoment
ordered evidence spans
latest user goal
```

## Required task-turn contract

Add a versioned contract such as `smalltalk.current_task_turn.v1`. Use repository-consistent names but keep these fields and distinctions.

```text
task_turn_id
session_id
surface_key
artifact_id
workstream_id when established
started_at_ms
last_observed_at_ms
latest_user_goal_summary
task_object
task_kind
current_actor
actor_activity_state
execution_state
waiting_on
relation_to_prior
prior_task_turn_id
supersedes_task_turn_id
parent_task_turn_id
latest_user_span_ids
current_state_span_ids
prior_boundary_span_ids
supporting_action_ids
supporting_event_ids
evidence_quality
confidence dimensions
missing_evidence
quality_flags
reason_codes
revision
```

`latest_user_goal_summary`, `task_object`, and actor-state wording must be bounded public-safe semantic summaries with source span ids, hashes, redaction status, and documented maximum lengths. Do not durably duplicate verbatim user messages, agent messages, or typed sequences into task-turn rows.

### Lifecycle axes

Do not mix task execution with who the user is waiting for. Support three independent axes:

```text
execution_state:
active
blocked
completed
superseded
suspended
idle_after_progress
unclear

current_actor:
user
assistant_or_agent
tool
system
unknown

waiting_on:
none
user
agent
external
unknown
```

A compatibility/public lifecycle label may be derived from these axes, but it must not replace them in persistence, evals, or confidence calculations. The critical fixture requires `execution_state = active`, `current_actor = assistant_or_agent`, and `waiting_on = agent`.

### Relation to prior turn

Support stable values for:

```text
new_task
continuation
clarification
child_support_step
correction
supersedes
unknown
```

### Confidence dimensions

At minimum:

```text
goal_confidence
task_object_confidence
actor_state_confidence
execution_state_confidence
waiting_on_confidence
relation_confidence
attribution_confidence
```

Do not use openability as task-turn confidence.

## Lifecycle rules

### New request after completed work

When a high-confidence newer user message follows a completed assistant/agent result:

- create a new current task turn;
- keep the completed turn as prior context;
- set `relation_to_prior = new_task`, `clarification`, or `child_support_step` based on evidence;
- never apply the prior completion cue to the new task;
- do not automatically close an entire durable workstream if the new task belongs to the same project.

The Capture-button case is `new_task` within the Smalltalk project after a completed Continue-card task. Its exact axes are `execution_state = active`, `current_actor = assistant_or_agent`, and `waiting_on = agent`.

### Continuation and clarification

Keep the same task turn only when evidence supports continuity, such as:

- explicit reference to the same object/question;
- agent status and response sequence within the same request;
- a follow-up clarification that does not establish a new goal;
- tool/editor/terminal work directly serving the same task.

Do not infer continuity solely because the same app or conversation is open.

### Child support step

A docs/search/terminal subtask can be a child of the current task turn without becoming the primary task. Preserve parent/child relation and evidence. Promotion to primary remains a later policy decision.

### Completion

A turn becomes completed only when a completion cue is attributable to that turn and supported by role/order/state evidence. Generic visible strings such as `done`, `passed`, or `verification passed` cannot complete a newer unrelated turn.

### Supersession

A new high-confidence user goal may supersede an unfinished prior task. Preserve the prior turn as superseded rather than silently rewriting it as completed.

### Unclear evidence

If speaker, order, or relation confidence is too low:

- keep lifecycle/relation `unclear` or `unknown`;
- retain competing hypotheses in developer audit if useful;
- do not choose the hypothesis that creates the most actionable target;
- cap public task confidence.

## Persistence and rebuild

Add idempotent persistence for task turns and evidence links. A likely schema family is:

```text
continue_task_turns
continue_task_turn_evidence
continue_task_turn_relations
```

It is acceptable to use fewer tables if the typed contract, provenance, revisioning, and query requirements remain clear.

Required properties:

- deterministic ids from stable local evidence rather than random rebuild ids;
- idempotent rebuilds;
- revision when materially new evidence changes the turn;
- source/evidence links for every summarized field;
- lifecycle history or transition reason sufficient for audit;
- bounded retention consistent with local-memory cleanup;
- compatibility for old databases with no task-turn rows.

## Two-pass task-turn and action pipeline

Avoid a circular resolver in which stale actions create a task turn and that task turn then validates the same stale actions. Implement two passes:

```text
ordered role/region evidence
  -> provisional task-turn boundary and relation
  -> turn-scoped action attribution and semantic delta
  -> finalized task execution/current actor/waiting-on state and revision
```

Historical actions may inform prior-task context only after the provisional boundary is established. They cannot decide the new boundary or bootstrap themselves into the current turn.

Required changes:

1. Classify completion/error/resolution cues within attributed spans and task-turn boundaries.
2. Evaluate high-confidence latest user/agent status before generic whole-frame completion fallback.
3. Associate every derived task action with `task_turn_id` when confidence permits.
4. Associate semantic delta, subject, before/after hints, and evidence quote with the same turn.
5. Add quality flags when an action comes from flattened fallback or ambiguous multi-turn evidence.
6. Prevent old completed actions from being collapsed into a newer turn merely because artifact id/app is shared.
7. Make current-task selection a deterministic output consumed by later workstream and recap phases.
8. Finalize execution/current-actor/waiting-on state from the provisional turn plus newly scoped actions; record when finalization changed the provisional hypothesis.

Do not delete useful P0-P5 action kinds. Make them temporally attributable.

## Current-task resolver

Implement a focused resolver that consumes:

- ordered role/region spans;
- typing/commit/status evidence;
- artifact/surface identity;
- prior task turns and historical lifecycle transitions as context;
- privacy and evidence-quality flags.

After provisional boundaries exist, a second pass may consume semantic moments and task actions that have been scoped to those boundaries.

The resolver should output:

```text
selected current task turn
alternative task-turn hypotheses when material
selection reason codes
conflict flags
missing evidence
```

Selection priority must favor a clearly newer user goal plus current work state over older completion, open-loop, feedback, and memory evidence. Later phases may use old state as context, but old state cannot define the current task turn.

Keep this logic in focused modules, for example:

```text
src-tauri/src/continuation/task_turn.rs
src-tauri/src/continuation/task_turn_lifecycle.rs
src-tauri/src/continuation/task_turn_resolver.rs
```

## Required behavior matrix

| Evidence sequence | Expected task-turn result |
| --- | --- |
| Completed task A, then new user asks task B | A completed; B current with execution active; relation new task. |
| Task A result, then user asks clarification about A | Same or linked clarification turn; no false new project. |
| Task A active, user asks unrelated task B | A superseded or suspended; B current. |
| User asks B, agent says it will trace code | B current; execution active; current actor assistant/agent; waiting on agent; not completed. |
| Old terminal says tests passed beside current chat task B | Terminal completion belongs to prior/support context, not B. |
| Search/docs opened for B | Child support step linked to B; B remains primary. |
| Only old completion and no new goal | Completed prior turn may be latest meaningful state; no fabricated active goal. |
| Multiple ambiguous chat turns | Relation/lifecycle unclear and confidence capped. |

## Tests

Add deterministic unit tests for:

1. New task after completed task.
2. Clarification versus new-task distinction.
3. Superseding an unfinished prior task.
4. Agent working status versus completed answer.
5. Completion cue scoped to matching turn.
6. Side-panel terminal completion not applied to chat turn.
7. Child support step and parent task relation.
8. Action collapse does not cross task-turn boundary.
9. Deterministic task-turn ids/revisions.
10. Old database fallback with no task-turn table.
11. Privacy-safe summaries and evidence links.
12. Two-pass resolver prevents historical actions from bootstrapping a new turn.
13. Execution/current-actor/waiting-on axes round-trip independently.

Full-pipeline critical assertions:

- current goal is the island Capture-button investigation;
- prior Continue-card copy work is completed context;
- execution is active, current actor is assistant/agent, and waiting-on is agent, never completed;
- `reviewing_output/completed_successfully/Verification passed` is not the active turn result;
- all contamination variants preserve task identity and relation;
- first divergence moves downstream of task-turn/action derivation.

## Integrations required in this phase

Expose the current task turn in developer/audit outputs and in the internal Continue build context. It may be added to `ContinueDecisionResult` as a typed internal/public-safe summary if compatibility is handled, but do not redesign the primary UI yet.

Add task-turn watermark/revision material to decision cache identity so a new request or lifecycle change invalidates stale decisions.

Do not allow task-turn memory to create target eligibility. It establishes semantic truth, not openability.

## Acceptance criteria

P6.03 is complete when:

- A typed, persisted, evidence-linked current task-turn contract exists.
- Execution state, current actor, waiting-on state, and prior-turn relation are explicit.
- Completion cues are scoped to their attributed task turn.
- Task actions and semantic deltas link to task turns.
- A newer high-confidence user request outranks older visible completion state.
- The critical fixture produces the correct current task, prior task, relation, and active state.
- Cache identity changes on material task-turn changes.
- Thin/ambiguous evidence remains thin.
- P6.01 L1/L2 checkpoints show the expected improvement.
- Existing P0-P5 behavior, privacy, and target gates remain passing.

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

Run the accuracy evaluator in model-off and deterministic model-response modes. Report current-task, relation, lifecycle, action, and semantic-delta results for every critical counterfactual.

## Final response format

Report:

1. Files and schema changed.
2. Task-turn and lifecycle contracts.
3. Resolver priority and ambiguity policy.
4. Action/semantic-delta refactor.
5. Cache and audit integration.
6. Tests and P6 metrics with denominators.
7. Verification results.
8. Remaining stale feedback/workstream errors.
9. Exact task-turn contracts P6.04 and P6.05 may rely on.

Do not claim the final recap is accurate yet. P6.03 establishes the current semantic truth that later layers must obey.
