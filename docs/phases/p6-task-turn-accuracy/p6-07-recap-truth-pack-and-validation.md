# P6.07 — Task-Truth Recap Pack, Cross-Layer Validation, And Model Parity

## Codex task

Rebase P5 activity recap synthesis on the P6 current task turn and semantic-consistency result. Upgrade local and model validation so a narrative is rejected when it is temporally stale, belongs to another workstream, treats prior completion as current, or implies a target that is not directly openable.

The optional model may improve phrasing. It must not choose task identity, repair an incoherent local graph through prose, or introduce a different semantic center.

## Dependency gate

P6.01-P6.06 must be complete. Confirm:

- current task turn and execution/current-actor/waiting-on axes are correct;
- stale feedback is ineligible;
- workstream/open-loop/primary consistency is available;
- claim-level confidence is available;
- the critical fixture's first divergence is now in recap pack, synthesis, validation, or public answer.

Read:

```text
AGENTS.md
PRODUCT.md
docs/phases/p6-task-turn-accuracy/p6-00-task-turn-accuracy-program.md
docs/phases/p6-task-turn-accuracy/p6-03-current-task-turn-lifecycle.md
docs/phases/p6-task-turn-accuracy/p6-05-workstream-open-loop-consistency.md
docs/phases/p6-task-turn-accuracy/p6-06-confidence-and-observation-reliability.md
```

## Current verified failure

P5 currently runs after upstream semantic construction. Its validator checks local-pack consistency:

- confidence and uncertainty;
- evidence handles;
- safe public copy;
- allowed claim slots and terms;
- target policy as represented in the pack;
- detour membership.

It does not independently require:

- latest-turn supersession;
- current-task/prior-completion separation;
- selected-workstream/primary-segment agreement;
- objective/current-task agreement;
- direct openability for target-shaped explanations;
- model-on/model-off task identity agreement.

In the critical audit, the model pack itself contained Stremio/Helium, the model repeated those terms, and validation returned `valid`. This was pack-compliant but factually wrong.

## Files and symbols to inspect first

```text
src-tauri/src/continuation/activity_recap.rs
src-tauri/src/continuation/activity_recap_inputs.rs
src-tauri/src/continuation/activity_recap_segments.rs
src-tauri/src/continuation/activity_recap_objective.rs
src-tauri/src/continuation/activity_recap_detours.rs
src-tauri/src/continuation/activity_recap_open_loop.rs
src-tauri/src/continuation/activity_recap_model.rs
src-tauri/src/continuation/activity_recap_validation.rs
src-tauri/src/continuation/activity_recap_integration.rs
src-tauri/src/continuation/accuracy_eval.rs
src-tauri/src/continuation.rs
src-tauri/src/capture.rs
```

Search for:

```text
ActivityRecapInputs
ActivityRecapModelPack
ContinueActivityRecap
build_activity_recap_inputs
stitch_activity_segments
infer_activity_work_label
synthesize_activity_recap
validate_activity_recap_model_output
validate_claim_terms
validate_target_policy
ActivityRecapDecisionProof
apply_prior_activity_memory
promote_validated_activity_recap_memory
primary_work_summary
last_meaningful_state
unfinished_state
next_action_summary
why_this_target
why_no_safe_target
```

## Input-pack contract

Version the recap input schema. Add a canonical task-truth section containing:

```text
current task turn id and revision
latest user goal and task object
execution state, current actor, and waiting-on state
relation to prior turn
prior task summary/state when relevant
task/workstream/branch/open-loop consistency result
claim-level confidence vector
task-turn evidence handles
eligible support/detour facts
eligible current open loop
direct target policy from P6.05
local-only forbidden/ineligible semantic sources with reason codes or hashes
```

The pack may include historical context, but historical objects must carry roles such as `prior_completed`, `support`, `detour`, `superseded`, or `unrelated_rejected`. They must not enter the allowed primary-term bank.

## Deterministic local recap

The local fallback is a product path, not an error message. Build it from the current task turn first:

```text
what: current task goal/object
where: safe surface/app/artifact label
state: execution/current actor/waiting-on and last concrete progress
prior boundary: relevant completed/superseded context only when useful
support/detours: explicit relations
next: evidence-backed next action or an honest unknown
target: direct target or no-safe-target explanation
uncertainty: claim-specific missing evidence
```

Required properties:

- Useful when target is null.
- Specific task language when task evidence is strong.
- No generic `reviewing documentation` substitute when the user goal is known.
- No old open-loop objective when current task owns a newer goal.
- No target-shaped copy from frame fallback.
- No unsupported command/file/path/URL/conversation identity.
- Every field carries claim evidence and bounded confidence.

## Model pack restrictions

The optional model receives only:

- typed current task truth;
- eligible, redacted evidence handles;
- allowed role-labeled context;
- claim slots and confidence caps;
- direct target policy;
- only local-safe claim constraints that do not reveal rejected private text.

The model must not receive unrelated loops, stale feedback, rejected branch terms, or a literal list of stale private terms. Keep rejected text and forbidden-term matching local to validator/audit, preferably as ids/hashes. Include rejected wording only in deterministic synthetic adversarial fixtures, never as ordinary model-request priming.

The model output schema must preserve:

```text
task_turn_id
task_identity_key or bounded semantic label
claim evidence handles
claim confidence
target policy fields copied from local truth
```

The model cannot alter ids, execution/current-actor/waiting-on state, workstream, target eligibility, openability, or confidence caps.

## Prior recap-memory scope

Audit and harden both prior recap-memory reads and validated recap-memory writes. `apply_prior_activity_memory` and `promote_validated_activity_recap_memory`, or their current equivalents, must require compatible:

```text
task turn identity and revision
workstream relation
semantic-consistency status
policy/schema version
freshness
non-contradiction by current evidence
```

A prior artifact/workstream recap cannot overwrite a thin current recap merely because it was once validated. Historical recap memory may provide labeled prior context; it cannot resurrect Stremio/Helium or an old completed task as the new semantic center. Persist task-turn/consistency provenance with promoted recap memory and invalidate/ignore incompatible legacy memory conservatively.

## Validator upgrades

Validate more than token membership.

### Temporal invariants

- Output primary task must refer to the current task turn.
- A prior completed task cannot be described as active/unfinished.
- A newer task cannot be described as completed by an older completion cue.
- State and next action must be compatible with execution/current-actor/waiting-on axes.

### Cross-layer invariants

- Current task, selected workstream, primary segment, objective, last state, and next action agree or have typed support relation.
- A `conflicting` semantic-consistency result cannot yield a `valid` high-confidence recap.
- Rejected open loops/branches/feedback cannot supply primary terms.
- Workstream mismatch without explicit explanation is a rejection.

### Claim-evidence invariants

- Each material claim has evidence handles from allowed roles.
- Evidence belongs to the same task turn or an allowed related context.
- Confidence does not exceed claim-level local cap.
- Missing critical dimensions require uncertainty or omission.

### Target invariants

- `why_this_target` is absent unless a URL/path-backed target, or another deliberately supported typed direct locator, passed policy.
- `why_no_safe_target` is present when the task is known but no direct target exists.
- Frame/evidence preview is never called a return point.
- Model output cannot create or upgrade openability.

### Semantic identity parity

Derive a bounded normalized task-identity representation from local and model recaps. Critical cases require agreement. Phrasing may differ; task object, execution/current-actor/waiting-on state, workstream, and target policy may not.

## Validation outcomes

Use explicit outcomes such as:

```text
valid
repairable_copy_only
rejected_temporal_conflict
rejected_workstream_conflict
rejected_ineligible_source
rejected_unsupported_claim
rejected_target_policy
fallback_local
thin_local
```

Repairs may remove/cap phrasing only when the underlying semantic identity is already correct. Do not repair a different task into place by dropping one word.

## Quality-gate integration

Existing quality gates may reward structural completeness even when the semantic center is wrong. Add semantic checks for:

- task-turn match;
- execution/current-actor/waiting-on match;
- workstream consistency;
- forbidden stale-source leakage;
- next-action support;
- target truth;
- wrong-confident classification.

A structurally complete but wrong recap must fail.

## Required behavior matrix

| Local truth / model output | Expected validation |
| --- | --- |
| Capture task local; model repeats Capture task with better phrasing | Valid or copy-only repair. |
| Capture task local; model says Stremio research | Reject and use local recap. |
| Current task active; model says completed because prior tests passed | Reject temporal conflict. |
| Selected workstream Smalltalk; primary model summary Helium | Reject workstream conflict. |
| No direct target; model says safest return point | Reject target policy. |
| Task clear, target null, model gives honest no-safe-target explanation | Valid. |
| Evidence thin; model raises confidence | Cap/repair or reject according to severity. |
| Model disabled | Local task identity and target policy match model-enabled accepted result. |

## Tests

Add deterministic unit tests for:

1. Task-truth pack construction.
2. Historical/rejected terms excluded from primary allowed terms.
3. Prior completion/current active temporal conflict.
4. Workstream/primary mismatch.
5. Ineligible open-loop term leakage.
6. Claim evidence from wrong task turn.
7. Execution/current-actor/waiting-on versus next-action incompatibility.
8. Target explanation without direct opener.
9. Confidence cap by claim dimensions.
10. Model/local task-identity parity.
11. Copy-only repair versus semantic rejection.
12. Quality gate rejects structurally complete wrong recap.
13. Model timeout/invalid output returns useful local recap.
14. Cache identity includes task-turn, consistency, confidence, and validator policy revisions.
15. Prior recap memory from another task turn cannot overwrite current truth.
16. Validated recap-memory promotion stores task-turn/consistency provenance.
17. Rejected private terms remain local and are absent from normal model requests.

Full-pipeline critical assertions:

- deterministic local recap identifies the Capture-button investigation;
- where/state mention only supported safe facts;
- prior Continue-card completion is context, not current state;
- Stremio/Helium forbidden primary leakage is zero;
- no `why_this_target` without a direct target;
- model-on and model-off agree on task identity/execution/current-actor/waiting-on/workstream/target policy;
- model validator cannot call the old Stremio/Helium output valid;
- wrong-confident rate is zero.

## Audit requirements

Explicit audit output must include:

```text
task-truth input section
eligible and rejected semantic sources
local deterministic recap
model pack/request/parsed output when enabled
claim-to-evidence mapping
temporal and cross-layer validation
target-policy validation
model/local identity parity
fallback/repair reason
quality-gate result
```

## Acceptance criteria

P6.07 is complete when:

- P5 inputs treat current task turn as the canonical semantic center.
- Local recap is useful and specific without a model.
- Historical/support facts cannot enter primary claims unless their typed relation permits it.
- Prior recap-memory reads/writes are task-turn/revision/consistency scoped and cannot recontaminate the current recap.
- Validator enforces temporal, workstream, source-eligibility, claim-confidence, and direct-target invariants.
- Model can improve phrasing but cannot change task identity or policy.
- Critical model-on/off identity agreement is 100%.
- Forbidden stale-term leakage and wrong-confident outputs are zero.
- Existing P5 audit/cache/fallback tests remain passing or are deliberately version-migrated.

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

Run the accuracy suite with model disabled, deterministic valid model output, wrong-task output, stale-completion output, target-policy violation, timeout, and invalid structured output.

## Final response format

Report:

1. Files/schema/version changes.
2. Task-truth pack contract.
3. Local recap behavior.
4. New validator invariants/outcomes.
5. Model parity and fallback behavior.
6. Critical/broad metrics with denominators.
7. Tests and verification.
8. Remaining backend/UI target-copy contract drift for P6.08.
9. Exact recap/confidence/target fields P6.08 may rely on.

Do not claim the whole product is done. P6.07 makes the narrative semantically truthful; P6.08 must present it honestly and consistently.
