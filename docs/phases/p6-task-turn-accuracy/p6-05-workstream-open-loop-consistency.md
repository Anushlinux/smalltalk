# P6.05 — Workstream, Branch, Open-Loop, And Objective Consistency

## Codex task

Make the durable semantic graph obey the current task turn. Rebuild workstream membership, branch roles, open-loop lifecycle, objective selection, and primary activity selection so they cannot silently disagree about what the user is doing.

This goal creates a single semantic center with explicit support/detour relationships. It does not yet rework P5 model validation or final UI copy.

## Dependency gate

P6.01-P6.04 must be complete. Confirm:

- the current task turn is correct in the critical fixture;
- task actions link to task turns;
- stale inferred feedback is rejected for current branch promotion;
- the accuracy harness reports workstream, open-loop, and primary-segment checkpoints.

Read:

```text
AGENTS.md
PRODUCT.md
docs/phases/p6-task-turn-accuracy/p6-00-task-turn-accuracy-program.md
docs/phases/p6-task-turn-accuracy/p6-03-current-task-turn-lifecycle.md
docs/phases/p6-task-turn-accuracy/p6-04-feedback-scope-provenance-decay.md
```

## Current verified failure

The critical decision had several competing semantic centers:

- selected workstream: Smalltalk/current chat work;
- recap primary segment: Helium, promoted as primary;
- open-loop objective: `Continue Stremio`;
- current user goal: investigate the island Capture button;
- public return target: a non-direct frame fallback.

Existing P5 segment stitching can prefer a promoted branch over an ordinary primary segment. Open-loop objective terms can outrank action subjects. Open loops may retain generic objectives from older workstream artifacts. Proof/audit records these facts side-by-side but does not reject their contradiction.

## Files and symbols to inspect first

```text
src-tauri/src/continuation.rs
src-tauri/src/continuation/task_turn.rs
src-tauri/src/continuation/task_turn_resolver.rs
src-tauri/src/continuation/activity_recap_inputs.rs
src-tauri/src/continuation/activity_recap_segments.rs
src-tauri/src/continuation/activity_recap_objective.rs
src-tauri/src/continuation/activity_recap_open_loop.rs
src-tauri/src/continuation/activity_recap_integration.rs
src-tauri/src/continuation/accuracy_eval.rs
```

Search for:

```text
ContinueWorkstream
ContinueBranchContext
ContinueOpenLoop
continue_workstream_artifacts
continue_workstream_state_snapshots
continue_workstream_edges
rebuild_continue_open_loops
workstream_has_resolved_lifecycle
build_open_loop_for_workstream
best_open_loop_for_workstream
evaluate_branch_promotion
promotion_state
select_primary_index
open_loop_matches_primary
collect_term_candidates
selected_workstream_open_loop_wins
primary_work_summary_seed
objective_hint
candidate_exposes_public_return_target
candidate_target_is_directly_openable
```

## Required ownership contract

Every semantic object that can influence the primary answer must have explicit task/workstream ownership or a typed relation to it.

Add or normalize these fields where appropriate:

```text
task_turn_id
workstream_id
parent_task_turn_id
origin_task_turn_id
relation_to_current_task
freshness relative to current turn
eligible_for_primary
eligible_for_objective
eligible_for_last_state
eligibility reason codes
```

Typed relations should include:

```text
same_task
same_workstream_prior_turn
child_support
detour
interruption
returned_support
superseded_task
unrelated
unknown
```

## Workstream rules

- The current task turn is the primary input to current workstream selection.
- A task turn may join an existing workstream when project/artifact/action evidence supports continuity.
- Same app or generic title alone is insufficient to merge task turns.
- A new task inside the same project may share a workstream without inheriting the prior task's completed state or objective.
- Selected workstream must expose which current task turn justified selection.
- If current task/workstream alignment is uncertain, produce an explicit conflict/thin state rather than selecting a stale but coherent workstream.

## Branch rules

- Support/search/docs/terminal/messages remain evidence-only by default.
- Branch promotion must use P6.04 applicability results and fresh post-branch task evidence.
- A promoted branch from an older task turn cannot become primary for the current turn.
- A branch can be primary only if the current task itself moved there or the user explicitly made it the current task.
- `promoted_primary` must include the task turn, promotion evidence ids, and promotion timestamp.
- Returned-to-origin evidence must not leave the branch permanently primary.

## Open-loop lifecycle rules

Open loops must be task-turn-aware.

Required states:

```text
open
blocked
waiting_on_agent
waiting_on_user
completed
superseded
closed
unclear
```

Rules:

- A completed prior task turn closes or completes its corresponding loop; it does not remain the current unfinished objective.
- A new task turn creates or selects its own loop boundary when evidence supports one.
- A support child can contribute blocker/verification evidence without replacing the parent loop objective.
- Old loops remain durable history but are ineligible for the current objective when relation is unrelated/superseded or freshness is outside policy.
- Open-loop selection must evaluate current task-turn relation before quality and recency.
- A generic `Continue {artifact title}` objective cannot outrank a specific current task goal.
- Loop updates must be deterministic and idempotent.
- `best_open_loop_for_workstream` or its current equivalent must filter by task-turn/lifecycle eligibility before comparing quality and recency; a strong stale/resolved loop cannot re-enter after rebuild.

## Objective selection rules

Replace the old effective priority with task-truth-first eligibility.

Required source order after eligibility filtering:

1. Current task-turn goal/object with high attribution confidence.
2. Current task action/semantic subject linked to that turn.
3. Current task's eligible open loop.
4. Selected workstream title/intent linked to the current turn.
5. Bounded surface identity as location context, not task objective.

An ineligible unrelated open loop receives no objective score, regardless of quality. Do not merely lower it from 100 to another high number.

Every selected term must carry:

```text
source kind
source id
task turn/workstream
eligibility decision
attribution confidence
freshness
rejected alternatives and reason codes
```

## Cross-layer consistency gate

Add a deterministic local consistency result before P5 synthesis:

```text
schema
current_task_turn_id
selected_workstream_id
primary_segment_id
selected_open_loop_id
selected_objective_source
selected_candidate_id
public_target_artifact_id
agreement status
conflicts
repairs or downgrades
missing evidence
```

Required agreement states:

```text
consistent
consistent_with_support_relation
thin_but_non_conflicting
conflicting
unresolved
```

Hard conflicts include:

- primary recap segment belongs to a different unrelated task/workstream;
- objective comes from an unrelated/superseded open loop;
- branch is promoted by inapplicable feedback;
- last state comes from a completed prior task while current task is active;
- target explanation names a different task than the selected current task.

On conflict:

- prefer current high-confidence task truth;
- filter ineligible objective/loop/branch facts from the current decision and recap projection;
- if no coherent center remains, downgrade to truthful thin mode;
- never choose the stale center merely because it has more historical evidence.

Conflict repair is decision-scoped. Do not delete or rewrite durable historical workstream, branch, loop, feedback, or memory rows merely because they are ineligible for the current task. Persist eligibility/rejection evidence separately.

P6.07 will consume this consistency result in recap validation. Implement and test it now at the semantic graph boundary.

## Canonical internal direct-target policy

Confidence and recap validation in P6.06-P6.07 need a truthful target primitive before UI integration. Add one typed internal policy result now:

```text
schema and policy version
candidate/artifact id
task-turn/workstream eligibility
feedback and branch eligibility
freshness eligibility
openability
locator kind
validated direct locator present
evidence preview available
direct target allowed
reason codes
supporting evidence ids
```

For the current product, a direct locator is a validated browser URL or document path. A future locator kind is eligible only if it is explicitly typed and supported end-to-end by strict-open policy. Frame id, screenshot path, session id, app identity, and human label are evidence/identity, not direct locators.

This phase does not redesign React or the island. It establishes `DirectTargetPolicy` as the sole internal truth consumed by P6.06 confidence and P6.07 recap validation. P6.08 must align public backend fields and UI/island presentation with it.

## Required behavior matrix

| Current task and historical graph | Expected result |
| --- | --- |
| New Capture task, old completed Continue-card task | Same Smalltalk workstream may remain, but current objective/state comes from Capture task. |
| New Smalltalk task, old unrelated Stremio loop | Stremio loop ineligible for objective/last state. |
| Old Helium branch promoted by stale feedback | Promotion removed/rejected for current task. |
| Current task uses docs as support | Docs linked as child support; parent task remains primary. |
| User explicitly switches task to docs research | New/current task turn may make docs primary with fresh evidence. |
| Current task and selected workstream genuinely conflict | Consistency conflict; repair or thin output, never silent disagreement. |
| Current task clear, exact target missing | Semantic center remains clear; target policy stays separate. |
| Current task unclear, old loop strong | Do not substitute old loop as current truth without relation evidence. |

## Tests

Add deterministic unit tests for:

1. Same project/new task workstream continuity without state inheritance.
2. Unrelated old loop ineligibility.
3. Completed prior loop closure.
4. Child support loop relation.
5. Generic artifact-title objective loses to current task goal.
6. Ineligible open loop receives no score.
7. Promoted old branch cannot become current primary.
8. Return-to-origin clears stale branch primacy.
9. Cross-layer consistency status and conflict reason codes.
10. Conflict repair versus truthful thin fallback.
11. Deterministic/idempotent loop rebuild.
12. Cache/watermark changes for loop/workstream/task ownership updates.
13. Historical conflict repair filters the current projection without deleting durable rows.
14. Strong stale/resolved loop cannot win `best_open_loop_for_workstream`.
15. Direct-target policy rejects frame fallback/human-label-only candidates and preserves evidence preview.

Full-pipeline critical assertions:

- current/selected workstream is Smalltalk and linked to the Capture task turn;
- Helium is not the primary segment;
- Stremio loop is ineligible and cannot supply objective, last state, or next action;
- prior Continue-card task remains completed context;
- cross-layer consistency is `consistent` or `consistent_with_support_relation`;
- contamination invariance holds;
- cross-layer contradiction count is zero;
- first divergence moves to recap pack/validation or target presentation.

## Audit requirements

Export for explicit accuracy audits:

```text
task-turn/workstream membership evidence
branch promotion applicability
open-loop lifecycle transitions
eligible/ineligible loops
objective candidates and reason codes
selected primary segment
cross-layer consistency result
repairs/downgrades
policy versions
```

Do not expose this graph as first-screen UI.

## Acceptance criteria

P6.05 is complete when:

- Current task turn owns or explicitly relates every primary semantic object.
- Workstream selection cites the current task turn.
- Branch promotion cannot leak across unrelated/older turns.
- Completed/superseded loops cannot supply the current objective.
- Current task evidence outranks eligible historical abstractions.
- A deterministic consistency gate detects and repairs/downgrades disagreement.
- Conflict repair is decision-scoped and preserves durable history.
- A versioned internal direct-target policy distinguishes direct locator from evidence preview before confidence/recap work.
- The critical fixture contains one Smalltalk/Capture semantic center.
- P6 L2 metrics meet critical zero-tolerance gates.
- P0-P5 support-branch and stale-target protections remain intact.

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

Run the accuracy evaluator, existing open-loop/branch/workstream tests, and model-off critical replay. Report every consistency conflict and selected objective source.

## Final response format

Report:

1. Files/schema changed.
2. Task/workstream/loop ownership contract.
3. Objective eligibility and priority.
4. Cross-layer consistency gate.
5. Critical/counterfactual checkpoint results.
6. Tests and verification.
7. Remaining confidence/probe and P5 validation gaps.
8. Exact consistency and direct-target policy results P6.06 and P6.07 may rely on.

Do not claim final user-facing accuracy is complete. P6.05 makes the local graph coherent before narrative synthesis.
