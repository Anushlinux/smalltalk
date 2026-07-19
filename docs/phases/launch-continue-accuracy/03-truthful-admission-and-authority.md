# LCA-03 — Truthful Admission, Confidence, And Semantic Authority

## Codex implementation task

Repair the local admission and production-authority path so it preserves the meaning and uncertainty of the actionable compact response. Validation must reject unsupported fields without manufacturing a false abstention, must never upgrade an uncertain model result to resolved, and must keep task truth separate from target availability.

This is the third phase of the Launch Continue Accuracy repair. Implement it completely only after LCA-01 and LCA-02 have passed.

This phase owns semantic admission, status calculation, source-kind truth, field-level failure behavior, compact snapshot authority, and semantic-versus-target state separation. It must not redesign the visible first-screen layout; LCA-04 owns presentation.

## Hard dependency gate

Read:

```text
docs/phases/launch-continue-accuracy/01-task-relevant-evidence-packet-completion-audit.md
docs/phases/launch-continue-accuracy/02-actionable-continuation-contract.md
docs/phases/launch-continue-accuracy/02-actionable-continuation-contract-completion-audit.md
```

Continue only when the audits end exactly with:

```text
PASS — LCA-01 evidence packet is proven and LCA-02 may begin
PASS — LCA-02 actionable contract is proven and LCA-03 may begin
```

Independently verify that the new response schema and production mapping exist and that the four critical fixtures reach the parser with the expected semantic meanings.

## Hard operating rules

- Read `AGENTS.md` and obey it.
- Preserve the current worktree and unrelated changes.
- Do not use Computer Use, AppleScript UI automation, Chrome control, the in-app browser, screenshots of the running app, mouse clicks, or keyboard automation.
- Do not perform or claim manual UI, native-island, or live-provider testing.
- The user owns live testing after all five phases.
- Deterministic unit, integration, replay, build, Rust, and Swift type-check verification is allowed.
- Do not loosen privacy, freshness, chronology, ownership, or target safety to make more answers pass.
- Do not add local semantic prose as a fallback for rejected model fields.
- Do not add a second model request, reconciliation call, or retry.
- Do not treat citations as proof that the cited pixels semantically entail the model sentence.
- Do not turn inferred goals into explicit user goals.
- Do not let missing target identity erase a useful task answer.
- Do not let a useful task answer create target eligibility.
- Do not claim calibration from a denominator that is too small.

## Required reading

Read current versions of:

```text
AGENTS.md
PRODUCT.md
docs/phases/proof-first-task-understanding/pftu-02-truthful-production-answer.md
docs/phases/p6-task-turn-accuracy/p6-03-current-task-turn-lifecycle.md
docs/phases/p6-task-turn-accuracy/p6-06-confidence-and-observation-reliability.md
docs/phases/p6-task-turn-accuracy/p6-07-recap-truth-pack-and-validation.md
docs/phases/p6-task-turn-accuracy/p6-08-target-truth-and-answer-composition.md
docs/phases/p6-task-turn-accuracy/p6-09-completion-audit.md
docs/phases/task-truth-v2/tt2-05-completion-audit.md
docs/phases/model-first-task-inference/mfti-04-completion-audit.md
```

Inspect current code and tests for:

```text
src-tauri/src/continuation/task_truth_v2/semantic_probe.rs
src-tauri/src/continuation/task_truth_v2/production.rs
src-tauri/src/continuation/task_truth_v2/verifier.rs
src-tauri/src/continuation/task_truth_v2/task_snapshot.rs
src-tauri/src/continuation/task_truth_v2/task_thread.rs
src-tauri/src/continuation/task_truth_v2/review.rs
src-tauri/src/continuation/confidence.rs
src-tauri/src/continuation/semantic_consistency.rs
src-tauri/src/continuation/accuracy_eval.rs
src-tauri/src/session_island/contract.rs
src/App.tsx
```

Reuse current P6 claim-level confidence, current-task-turn, semantic consistency, atomic identity, strict target, correction, and cache contracts. This phase repairs the compact path's integration with those contracts rather than creating a second verifier architecture.

## Verified failures this phase must remove

Across the four supplied logs, every raw model response used `partly_resolved`. Local admission then produced:

| Case | Raw model status | Local result | Failure |
| --- | --- | --- | --- |
| 05cd | `partly_resolved` | `partly_resolved` | Meaning remained descriptive and incomplete. |
| 0d1c | `partly_resolved` | `resolved` | Local code increased certainty despite missing action/target and compound task wording. |
| 0056 | `partly_resolved` | `unresolved` | The primary task was erased as passive evidence even though the current image showed implementation complete and verification remaining. |
| 0e34 | `partly_resolved` | `resolved` | Local code increased certainty despite qualified wording and an unproven causal hypothesis. |

The validator currently proves support-slot existence, ownership, privacy, fingerprint, and chronology. Those are necessary mechanical checks. They do not prove that the model interpreted the image correctly.

LCA-03 must stop treating mechanical validity as semantic certainty and stop treating passive-image category rules as a substitute for field-specific semantic support.

## Required authority model

Persist and expose separate concepts for:

```text
raw_model_status
admitted_semantic_status
semantic_source_kind
field_admission
claim_confidence
target_status
unresolved_or_failure_reason
atomic_answer_identity
```

Do not overload one `status` field with provider transport, semantic completeness, and target availability.

### Raw model status

Store the exact parsed provider status. Do not rewrite it during parsing.

### Admitted semantic status

Use:

```text
resolved
partly_resolved
unresolved
refused
```

Local admission may preserve or downgrade model certainty. It must never upgrade beyond the raw model status.

Required monotonic rule:

```text
resolved          -> resolved, partly_resolved, or unresolved
partly_resolved   -> partly_resolved or unresolved
unresolved        -> unresolved
refused           -> refused
```

Provider transport and structured-output failures remain typed failures outside this semantic ordering.

### Semantic source kind

Use a stable distinction such as:

```text
verified_cloud_explicit_goal
verified_cloud_inferred_goal
human_correction
unresolved
```

An image-inferred task, including wording such as `likely` or `appears`, must never be labeled `explicit_goal`.

Human correction must remain scoped to the exact answer identity and field. It must not authorize a target or silently rewrite unrelated fields.

### Target status

Track target truth separately from semantic status. Use current repository names where possible, while preserving these meanings:

```text
direct_target_ready
task_known_target_unknown
frame_preview_only
target_support_only
target_suppressed
stale_decision
no_task
```

A semantically resolved answer may have `task_known_target_unknown`. A direct target may exist only through current strict locator ownership and open-time validation.

## Field-level admission contract

Every semantic field from LCA-02 must receive an independent verdict:

```text
accepted
rejected_unsupported
rejected_stale
rejected_private
rejected_wrong_surface
rejected_chronology
rejected_contradiction
rejected_generic
rejected_overlong
rejected_invalid_state
```

Use current stable reason strings where equivalent values already exist. Do not introduce aliases for the same meaning.

Required behavior:

- Rejection of `where_summary` must not erase the task, resume point, or action.
- Rejection of `next_supported_action` must downgrade semantic status and leave the supported task/resume point visible.
- Rejection of `completed_context` must not erase the active task.
- Rejection of `unfinished_task` prevents `resolved`, but supported resume/action facts may remain diagnostic and must not be rewritten into a task.
- A visit-role failure must not erase unrelated admitted semantic fields.
- Invalid inline citation tokens must not leak into public strings.
- The verifier must never write replacement semantic prose.

## Field-specific support policy

Replace broad “passive evidence cannot establish the primary task” behavior with a field-specific policy that uses attributed evidence.

### Evidence that may support `unfinished_task`

- attributed current or recent user request/draft;
- attributed agent response/status tied to that request;
- explicit visible task object plus owned task-turn evidence;
- an earlier task-bearing context image connected through a proven task turn or detour/return relation;
- a scoped human correction.

A bare page, app, title, or assistant article cannot establish the user's unfinished task.

### Evidence that may support `resume_point`

- visible partial answer or result;
- attributed agent/tool working state;
- explicit completion plus remaining verification;
- visible unsent draft state;
- explicit blocker or error;
- task-linked editor/terminal/artifact state.

### Evidence that may support `next_supported_action`

- an explicit unfinished instruction;
- a labeled user-verification step;
- a safe review/continue operation implied by a partial response;
- return to an unsent draft without claiming submission;
- wait for or inspect an active agent result;
- an already validated safe product action.

Task confidence alone cannot support an action.

### Evidence that may support `completed_context`

- a completion cue attributed to the prior task turn;
- a passed check or completed response linked to the prior task;
- local P6 lifecycle truth that agrees with cited visual/action evidence.

Old completion language must not complete a newer task.

## Semantic consistency checks

Before calculating final status, enforce:

1. `unfinished_task` is not already completed unless the current task is explicitly verification/review of that completed work.
2. `task_state` agrees with the resume point and action.
3. `next_supported_action` advances the unfinished task rather than the completed context.
4. Adjacent supporting/detour panes do not introduce a second primary objective.
5. A causal hypothesis remains qualified.
6. P6 current-task-turn identity and compact task identity do not silently contradict each other.
7. Target status does not influence task selection or semantic confidence.

When P6 and the compact provider disagree materially:

- record the conflict;
- cap or downgrade semantic status;
- do not select whichever result has an easier target;
- do not merge their prose;
- do not let local P6 prose replace a rejected cloud field on the public compact answer.

## Status calculation

`resolved` requires all of:

- admitted concrete `unfinished_task`;
- admitted `task_state` other than `unclear`;
- admitted concrete `resume_point`;
- admitted `next_supported_action`;
- no material cross-field contradiction;
- no unresolved competing primary task;
- raw model status of `resolved`;
- claim-level confidence that satisfies the current frozen policy;
- no qualified wording that materially weakens the task or action claim.

`partly_resolved` applies when a useful task is known but at least one of state, resume point, action, or certainty is incomplete. It also applies to qualified `likely`/`appears` task wording when retained.

`unresolved` applies when no concrete unfinished task survives admission or when a material conflict prevents a truthful primary answer.

Do not require a direct target for semantic resolution. Record target status separately.

Do not invent confidence thresholds in this phase if the existing frozen P6 policy already defines them. If sample sizes are insufficient for calibration, retain the frozen thresholds, report the limitation, and never claim calibration success.

## Transport, parse, semantic, and target failures

Keep distinct typed outcomes for:

```text
capture failure
stale frame
privacy block
credentials missing
provider disabled
provider timeout
provider rejection
provider empty/incomplete output
structured parse failure
support validation failure
semantic unresolved
target unavailable
no new evidence
```

The phrase “not enough evidence” may be used only for a genuine semantic unresolved result. It must not represent provider failure, parse failure, one rejected field, or missing target identity.

## Atomic authority and persistence

The admitted compact result must remain bound to:

```text
manual Continue decision/boundary
current frame and evidence watermark
packet id and policy version
provider request and response ids
response schema version
verifier/admission version
admitted result identity
field-level support map
human correction watermark
target identity when one independently exists
```

Do not create a content-hash task snapshot and then treat it as though it were a persisted explicit user goal.

Use existing task snapshot/thread persistence where it can preserve the exact response and evidence identity. If a compact snapshot cannot satisfy strict target ownership, retain a null direct target. Do not weaken target ownership to make the snapshot openable.

Repeated Continue with unchanged atomic evidence must reuse the same admitted semantic result without a new provider post, following the existing cache/freshness contracts. A materially new user request or task-state change must invalidate reuse.

## Expected admission outcomes for the four critical logs

### 05cd

```text
semantic status: partly_resolved unless every actionable field is unqualified and complete
task: product-need answer review
action: continue reviewing the visible answer section
target: unknown or frame preview only
```

### 0d1c

```text
semantic status: partly_resolved when exact implementation-result state remains uncertain
task: visual-cue request/result inspection
completed context: backend connection completed
launch checklist: supporting/detour only
target: unknown unless strict ownership independently succeeds
```

### 0056

```text
semantic status: at least partly_resolved
task: user verification of completed visual cue
resume point: implementation complete and checks passed
action: test the cue
forbidden result: whole-answer unresolved solely because the current evidence is image-based/passive
```

### 0e34

```text
semantic status: partly_resolved
task: investigate the insufficient-evidence regression
resume point: unsent regression draft
action: return to the draft
causality: unproven
forbidden result: local upgrade to resolved while causal wording remains qualified
```

## Required deterministic tests

Add tests for:

1. Status monotonicity across every raw/local combination.
2. Raw `partly_resolved` can never become local `resolved`.
3. `likely` or `appears` cannot become `explicit_goal` or resolved certainty.
4. Semantic status and target status vary independently.
5. Missing direct target does not erase a useful semantic answer.
6. A useful semantic answer does not create target eligibility.
7. Field-local rejection preserves unrelated admitted fields.
8. Visit-role rejection cannot erase the task/action fields.
9. Inline support-slot tokens do not reach public text.
10. Current attributed image evidence can support verification remaining.
11. Bare passive page content cannot establish user intent.
12. A context image is not automatically semantically authoritative merely because it exists.
13. Prior completion cannot complete a newer task.
14. Unsent draft remains unsubmitted and causal claims remain qualified.
15. P6/cloud task conflict is recorded and downgraded without prose merging.
16. Transport/parse/validation/semantic/target outcomes remain distinct.
17. Atomic answer identity round trip and stale-identity rejection.
18. Unchanged-evidence reuse versus material task change invalidation.
19. Four critical cases produce the expected admission matrix.
20. Existing correction, privacy, cache, strict-open, P6 confidence, and target-honesty suites remain passing.

## Phase acceptance criteria

LCA-03 is complete only when:

- Local admission cannot increase semantic certainty.
- Inferred goals are not labeled explicit.
- Semantic and target statuses are separate.
- The 0056-shaped answer is not erased by a category-level passive-evidence rule.
- The 0d1c and 0e34 cases are not upgraded beyond their evidence.
- Field-level validation cannot collapse a useful partial answer into a generic abstention.
- Mechanical support validity is not treated as semantic entailment.
- P6/compact conflicts are explicit and fail safely.
- Failure reasons remain accurately typed.
- Atomic identity is preserved through production persistence and cache reuse.
- Strict target ownership remains unchanged or stronger.
- No local semantic fallback enters the public compact answer.
- Existing P6, privacy, correction, cache, and open-safety tests pass.
- Automated verification passes.
- No CUA, live UI, or user-owned manual result is claimed.

## Verification commands

Run the current equivalents of:

```bash
cd src-tauri
cargo fmt --all -- --check
cargo check
cargo test semantic_probe --lib
cargo test verifier --lib
cargo test production --lib
cargo test task_thread --lib
cargo test confidence --lib
cargo test semantic_consistency --lib
cargo test session_island --lib
cargo test continuation --lib
cd ..
npm run build
npm run test:webview
git diff --check
git status --short
```

Run the current accuracy replay command and report denominators. Do not report a filter that ran zero tests as a pass.

## Required completion audit

Create:

```text
docs/phases/launch-continue-accuracy/03-truthful-admission-and-authority-completion-audit.md
```

Include:

1. Before/after status and authority model.
2. Field-level support matrix.
3. Source-kind and explicit-versus-inferred mapping.
4. Semantic-status versus target-status proof.
5. Four critical raw/parser/admission/public-state results.
6. False-positive and false-negative regression proof.
7. Atomic identity/cache/correction/target-safety proof.
8. Typed failure mapping.
9. Automated commands and exact results.
10. Limitations reserved for LCA-04.
11. A statement that manual app testing remains user-owned.

End with exactly one verdict:

```text
PASS — LCA-03 admission and authority are proven and LCA-04 may begin
```

or:

```text
INCOMPLETE — LCA-03 admission and authority remain open and LCA-04 is blocked
```

## Final response format

Report:

1. Admission and authority changes.
2. Four critical-case status results.
3. False-positive/false-negative fixes.
4. Test commands and exact results.
5. Completion-audit verdict.
6. Exact public contract LCA-04 may consume.
7. Manual testing intentionally deferred to the user.

