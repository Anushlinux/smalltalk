# P6.04 — Feedback Scope, Provenance, Freshness, And Decay

## Codex task

Prevent historical or inferred feedback from silently changing the current task. Make feedback influence explicit about provenance, scope, freshness, task-turn relation, and allowed effect. Preserve P1 hard negative suppression while preventing stale inferred positive signals from promoting unrelated branches.

This goal repairs the feedback layer. It must not use feedback to rewrite role/turn evidence, fabricate task continuity, or create public target eligibility by itself.

## Dependency gate

P6.01-P6.03 must be complete. Confirm that every eligible current action and branch can be linked to a `task_turn_id` when evidence supports it.

Read:

```text
AGENTS.md
PRODUCT.md
docs/phases/p6-task-turn-accuracy/p6-00-task-turn-accuracy-program.md
docs/phases/p6-task-turn-accuracy/p6-01-ground-truth-replay-eval.md
docs/phases/p6-task-turn-accuracy/p6-02-role-region-turn-evidence.md
docs/phases/p6-task-turn-accuracy/p6-03-current-task-turn-lifecycle.md
```

## Current verified failure

In the critical audit:

- an inferred correction/positive feedback event targeted a Helium artifact roughly six hours before the decision;
- branch promotion later searched positive feedback by artifact, stable key, and workstream;
- it did not require the feedback to occur after the current branch began;
- it did not require the same session or task turn;
- it did not require explicit user feedback;
- inferred positive feedback still had enough weight to count;
- the new branch became `promoted_user_corrected` with reason `positive_feedback_targeted_branch`.

This made stale behavioral inference look like a durable user correction and helped move the recap's primary semantic center away from the current Smalltalk task.

## Files and symbols to inspect first

```text
src-tauri/src/continuation.rs
src-tauri/src/continuation/
src-tauri/src/continuation/accuracy_eval.rs
src/App.tsx
src-tauri/src/session_island.rs
```

Search for:

```text
ContinueFeedbackEventResult
ContinueFeedbackRequest
ContinueExplicitFeedbackRequest
FeedbackTargetScope
FeedbackEventTargets
FeedbackEventRow
feedback_targets_for_feedback_event
load_continue_feedback_event_rows_for_target_keys
feedback_source_is_explicit
feedback_positive_weight
feedback_negative_weight
record_continue_feedback
latest_positive_feedback_kind_for_branch
evaluate_branch_promotion
candidate_branch_public_return_gate
continue_feedback_events
continue_pairwise_preferences
continue_ranking_priors
feedback watermark
cache invalidation
```

## Required policy distinction

Feedback is not one undifferentiated score. Add a typed policy result that answers:

```text
what happened
who or what produced the signal
which decision/candidate/artifact/workstream/task turn it applies to
whether it is positive, negative, neutral, or informational
whether it may suppress, rank, promote, or only annotate
when it expires or requires reconfirmation
which evidence proves its applicability
```

## Provenance contract

Use stable source classes such as:

```text
explicit_user_accept
explicit_user_reject
explicit_user_correct
explicit_user_ignore
explicit_user_next_step
explicit_open_action
inferred_navigation
inferred_return
inferred_timeout
system_migration
unknown
```

Preserve existing public/source strings through compatibility mapping if needed. Do not treat an inferred navigation pattern as equivalent to explicit correction.

Each feedback event or normalized feedback policy record must carry:

```text
event_id
event_kind
provenance
decision_id
selected_candidate_id
target_artifact_id
chosen_artifact_id
artifact_stable_key when safe
workstream_id
task_turn_id
session_id
branch_context_id when applicable
occurred_at_ms
applies_from_ms
expires_at_ms or reconfirmation rule
confidence
per-target polarity and allowed effects
reason
evidence ids
```

Do not copy private note text into model packs or public copy.

## Orthogonal applicability dimensions

Do not model feedback scope as one narrow-to-broad hierarchy. Decision, candidate, task turn, branch, artifact, workstream, session, and time are orthogonal axes that often require conjunctive matching.

Represent and evaluate:

```text
target scope: candidate | target artifact | chosen artifact | stable key | workstream | global policy
decision id
task turn id
branch context id
session id
time window/freshness
provenance requirement
polarity for this target
allowed effect for this target
```

Broader target scope requires stronger, more explicit evidence. Inferred feedback should default to narrow target scope and same decision/task/session windows. A signal applies only when every required orthogonal dimension matches.

Feedback polarity is target-specific. For a `corrected` event, the rejected `target_artifact_id` receives negative/suppression evidence while `chosen_artifact_id` may receive positive preference evidence. Do not assign one event-level positive/negative value to both targets. Normalize through `feedback_targets_for_feedback_event` or its current equivalent and return per-target polarity/effect records.

## Positive feedback rules

- Explicit correction/acceptance may influence later decisions within a matching task/workstream scope, subject to contradictions and freshness policy.
- Explicit open action proves that a user opened something, not that the artifact became the primary task forever.
- Inferred navigation/return may provide a small ranking prior inside a short observation window.
- Inferred positive feedback alone must never promote a support branch to public-return eligibility.
- Promotion requires fresh post-branch task evidence or explicit feedback applicable to the current task turn/branch.
- Positive feedback older than the branch start cannot count as evidence that the current branch was promoted.
- Cross-session reuse requires explicit provenance, stable semantic identity, compatible workstream/task relation, and no fresher contradiction.

## Negative feedback rules

Preserve the current hard-suppression doctrine:

- Repeated explicit negative feedback remains a hard control.
- Suppression must continue to apply before ranking, alternatives, model candidate packing, validation, cache reuse, and strict open.
- A target cannot reappear until fresh local reconfirming evidence satisfies the existing strict policy.
- P6 scoping must not accidentally weaken P1.

At the same time, ensure a negative event scoped to a specific artifact/candidate does not suppress an unrelated current task merely because a broad app title matches.

## Freshness and decay

Implement policy-driven freshness rather than one universal TTL.

Required behavior:

| Signal | Default effect |
| --- | --- |
| Explicit reject/correct | Durable within validated scope; contradiction/reconfirmation aware. |
| Explicit accept | Durable ranking evidence within scope; not unconditional branch promotion. |
| Explicit open | Short/medium-lived confirmation of open usefulness; not task identity proof. |
| Inferred navigation/return | Short-lived, low-weight, same-task/session preference only. |
| Inferred correction/acceptance from behavior | Must not be labeled equivalent to explicit correction; short-lived and narrow. |
| Timeout/ignore inference | Weak and narrow; never hard suppression by itself unless existing policy explicitly proves repeated behavior. |

Centralize duration/effect constants in a versioned feedback policy object. Audit the evaluated age and cutoff. Avoid scattered magic numbers.

## Branch promotion integration

Replace `latest positive kind` style lookup with an applicability evaluation that receives:

```text
current task turn
branch context and branch start time
origin workstream
branch artifact identity
candidate/target identity
current time
fresh action evidence
all potentially matching feedback records
per-target polarity/effect records
```

Return:

```text
eligible events
rejected events with reason codes
allowed promotion effect
confidence contribution
freshness/expiry decision
orthogonal applicability results
```

Required rejection reasons include:

```text
predates_branch
different_task_turn
different_session_without_durable_scope
inferred_source_cannot_promote
expired
scope_mismatch
superseded_by_newer_feedback
contradicted_by_current_evidence
missing_provenance
```

## Migration and compatibility

Existing rows may lack task turn, provenance class, expiry, or scope. Migrate idempotently and evaluate legacy unknown/inferred rows conservatively.

Do not reinterpret legacy inferred feedback as explicit. Record a migration policy version. Keep existing APIs compatible or update all Rust/Tauri/React/island callers together.

Feedback policy changes must invalidate cached Continue decisions through the existing feedback watermark/policy fingerprint mechanisms.

## Required behavior matrix

| Feedback and current context | Expected effect |
| --- | --- |
| Six-hour-old inferred Helium correction; new Smalltalk task | Rejected for branch promotion and current-task influence. |
| Explicit correction in same current task turn | May correct candidate/workstream within declared scope. |
| Inferred open in same session shortly after decision | Small bounded ranking evidence; no unconditional primary promotion. |
| Explicit accept from prior session, same stable artifact, incompatible new task | Does not rewrite new task; may remain historical preference only. |
| Explicit repeated reject for exact target | Existing hard suppression remains active. |
| New strong local editing on formerly rejected target | Reconfirmation path evaluated under P1; no silent bypass. |
| Positive event before branch start | Cannot prove branch promotion. |
| Positive event after branch start but different task turn | Cannot promote current branch. |
| Legacy row with unknown source | Conservative, narrow, low/zero promotion effect. |

## Tests

Add deterministic unit tests for:

1. Provenance normalization.
2. Conjunctive matching across target, decision, task turn, branch, session, and time dimensions.
3. Freshness/decay boundaries.
4. Positive event before branch start.
5. Same artifact but different task turn.
6. Cross-session explicit versus inferred behavior.
7. Inferred positive cannot promote support branch.
8. Explicit correction can apply inside correct scope.
9. Newer contradiction supersedes older positive feedback.
10. Legacy row conservative migration.
11. Feedback policy fingerprint/cache invalidation.
12. Existing hard negative suppression at ranking, model pack, cache, alternatives, and open time.
13. Corrected event is negative for rejected target and positive only for chosen target when eligible.

Full-pipeline critical assertions:

- the old inferred Helium feedback is rejected with inspectable reason codes;
- no critical counterfactual changes the Capture-button current task;
- stale-feedback false promotion count is zero;
- formerly passing P1 suppression fixtures remain passing;
- first divergence moves downstream of feedback eligibility.

## Audit requirements

For explicit accuracy/audit runs, export:

```text
candidate feedback events considered
normalized provenance and scope
event age and freshness boundary
task-turn/workstream/branch match
eligible effects
rejection reason codes
promotion decision
policy version/fingerprint
```

Public UI should not expose raw policy internals. Developer diagnostics may explain why a correction was or was not applied.

## Acceptance criteria

P6.04 is complete when:

- Feedback provenance, scope, freshness, and allowed effects are typed and auditable.
- Inferred positive behavior cannot act as durable explicit correction.
- Branch promotion uses current task turn and branch-start time.
- The critical stale Helium feedback cannot promote the branch.
- P1 hard negative suppression remains intact across every enforcement point.
- Legacy feedback is migrated/evaluated conservatively.
- Feedback changes invalidate decision cache correctly.
- P6 accuracy metrics report zero stale false promotion on critical fixtures.
- Existing explicit feedback, island feedback, ranking prior, and replay tests pass.

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

Run the full accuracy suite, the existing feedback/replay eval, and targeted branch-promotion tests. Report explicit numerators/denominators and the rejection reason for the legacy Helium event.

## Final response format

Report:

1. Files/schema changed.
2. Provenance, scope, decay, and effect contracts.
3. Branch-promotion integration.
4. Legacy migration behavior.
5. P1 regression proof.
6. Critical/counterfactual metrics.
7. Verification results.
8. Remaining workstream/open-loop contradictions for P6.05.
9. Exact feedback applicability contract P6.05 may rely on.

Do not claim recap truth is fixed. P6.04 removes stale feedback as an unauthorized semantic authority.
