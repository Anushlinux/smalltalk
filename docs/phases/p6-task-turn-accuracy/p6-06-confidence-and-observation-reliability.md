# P6.06 — Claim-Level Confidence And Observe-Before-Decide Reliability

## Codex task

Replace broad evidence-quality confidence with claim-level confidence dimensions, then make observe-before-decide target the missing dimension and calibrate failure honestly. A strong app identity must no longer imply a strong task, lifecycle, or target claim.

This goal hardens evidence sufficiency and confidence propagation. It does not yet redesign final recap validation or UI composition.

## Dependency gate

P6.01-P6.05 must be complete. Confirm that current task, execution/current-actor/waiting-on state, workstream/loop consistency, feedback applicability, and the internal direct-target policy expose typed confidence and missing-evidence inputs.

Read:

```text
AGENTS.md
PRODUCT.md
docs/phases/p6-task-turn-accuracy/p6-00-task-turn-accuracy-program.md
docs/phases/p6-task-turn-accuracy/p6-03-current-task-turn-lifecycle.md
docs/phases/p6-task-turn-accuracy/p6-05-workstream-open-loop-consistency.md
```

## Current verified failure

Weak-surface enrichment currently grades identity and general evidence quality, but the product often needs a different question answered: is the latest task goal and state understood?

In the critical audit:

- capture had clean active hybrid text;
- weak-surface identity/snapshot machinery had enough evidence to appear useful;
- the snapshot still lost the salient question;
- observe-before-decide attempted accessibility/window probes;
- probes timed out or produced no successful evidence;
- accessibility character count was zero;
- `evidence_changed` was false;
- the decision continued into a confident-looking but semantically wrong recap.

At the reviewed baseline, the Tauri wrapper also reruns the decision and records `reran_decision: true`/an evidence-refreshed warning even when the probe reports `evidence_changed = false`. P6.06 must make this trace reflect reality and avoid the unconditional rerun.

The product needs to say which claim is strong and which claim remains thin.

## Files and symbols to inspect first

```text
src-tauri/src/continuation.rs
src-tauri/src/continuation/enrichment.rs
src-tauri/src/continuation/task_turn*.rs
src-tauri/src/continuation/activity_recap*.rs
src-tauri/src/continuation/accuracy_eval.rs
src-tauri/src/capture.rs
src/App.tsx
src-tauri/src/session_island.rs
```

Search for:

```text
ContinueEvidenceQuality
P0QualitySignals
ContinueDecisionQualityGate
ActivityConfidence
identity_confidence
evidence_quality
attribution_confidence
EvidenceSufficiencyDecision
NeedMoreEvidenceRequest
NeedMoreEvidenceProbeResult
ObserveBeforeDecideTrace
observe_before_decide
continue_evidence_probes
max_probe_ms
evidence_changed
missing_evidence
confidence_label
```

## Required confidence vector

Add a versioned internal contract such as `smalltalk.continue_confidence.v2` with normalized values and evidence links for:

```text
surface_identity
active_window_ownership
region_segmentation
speaker_attribution
turn_order
latest_user_goal
task_object
current_actor_state
execution_state
current_actor
waiting_on
relation_to_prior
workstream_alignment
branch_role
open_loop_relevance
recap_claim_support
target_identity
target_openability
direct_target_policy
```

Each dimension must include:

```text
score
label
supporting evidence ids
missing evidence
quality flags
calculation reason
```

Use a small stable label set such as `none`, `low`, `medium`, `high`. Preserve scores for calibration and labels for product use.

## Claim dependency policy

Define which dimensions bound each public/internal claim.

Examples:

| Claim | Critical dimensions |
| --- | --- |
| “You were in ChatGPT/Codex” | surface identity, ownership. |
| “You were investigating the Capture button” | speaker attribution, turn order, latest user goal, task object. |
| “The agent was tracing Swift/Rust” | speaker attribution, current actor/activity state, task-turn relation. |
| “The earlier task was complete” | prior-turn relation, execution-state attribution. |
| “Continue in this page” | target identity, direct openability, policy eligibility. |
| “Next, wait for or review the tracing result” | current task, execution/current-actor/waiting-on state, next-action evidence. |

Public claim confidence must be the bounded aggregate of its critical dimensions. Do not average a zero target-openability dimension with strong task identity and call the target medium confidence.

Task confidence and target confidence remain separate. A high-confidence task recap with no direct target is valid.

## Evidence sufficiency

Replace a generic need-more-evidence decision with a typed missing-dimension plan.

The sufficiency evaluator should answer:

```text
which desired claim is below threshold
which confidence dimension is responsible
which evidence source could materially improve it
whether a bounded probe is permitted and likely to help
which probe is selected
what happens if the probe fails
```

Do not run a probe merely because overall confidence is low. If the missing dimension is direct URL openability, another screenshot may not help. If the missing dimension is speaker attribution, a window-list probe may not help.

## Probe contract

Extend probe results with explicit per-probe outcomes:

```text
probe kind
requested dimension
started/completed time
deadline
status
success criteria
evidence records created
dimensions changed
timeout/cancel/error/privacy reason
stale-result flag
```

Statuses should distinguish:

```text
succeeded_changed_evidence
succeeded_no_change
timed_out
privacy_blocked
permission_blocked
cancelled
failed
not_applicable
budget_exhausted
```

## Observe-before-decide policy

- Enforce real wall-clock deadlines and cancellation/cleanup.
- Keep total Continue latency bounded.
- Run only probes relevant to missing critical dimensions.
- Never treat `attempted` as `succeeded`.
- A timeout, error, privacy block, or no-change result cannot increase confidence.
- A failed probe adds explicit missing evidence and caps dependent public claims.
- A late result from an older decision cannot mutate the current decision without a new evidence watermark/rebuild.
- Re-run the decision only when evidence materially changed.
- Set `reran_decision` from actual control flow. Timeout, failure, and succeeded-no-change results must not rerun and must not emit an `evidence_refreshed` warning.
- Do not repeatedly probe the same unchanged surface in a tight loop.
- Audit request, result, duration, and confidence delta.

If the current task is already high confidence but target identity is missing, do not spend budget trying to turn a frame fallback into a target. Produce a useful recap with a null target.

## Confidence propagation

Integrate the vector into:

- current task-turn resolver;
- workstream/open-loop consistency gate;
- P5 recap input context;
- decision quality gate;
- cache identity when a material dimension changes;
- audit and eval checkpoints;
- compact public-safe confidence summary.

Do not expose every numeric score on the first screen. P6.08 will map them to restrained product copy.

Retain compatibility with existing `confidence`, `confidence_label`, `activity_confidence`, and `target_confidence` fields. Define their derivation from the new vector and deprecate ambiguous uses rather than silently changing meaning.

## Required behavior matrix

| Evidence state | Expected confidence/probe behavior |
| --- | --- |
| Strong app identity, no latest user role | Identity high; task low/none; probe speaker/turn evidence only if likely. |
| Latest goal/state clear, no URL/path | Task high/medium; target identity/openability none; no fake target. |
| AX probe times out | Attempt recorded; no confidence increase; missing AX/speaker evidence remains. |
| Window probe succeeds but adds same title | Identity may remain unchanged; task confidence does not increase. |
| OCR/AX adds attributed latest user span | Speaker/goal dimensions may increase with evidence ids. |
| Privacy blocks text probe | Keep safe identity metadata; task claim thin; explain missing evidence. |
| All task layers conflict | Consistency/task confidence capped even if each source individually appears medium. |
| Probe returns after decision watermark changed | Mark stale; require new decision cycle. |

## Tests

Add deterministic unit tests for:

1. Claim-to-dimension dependency mapping.
2. Weakest-critical-dimension confidence bounding.
3. Strong identity does not raise task confidence.
4. Strong task does not raise target openability.
5. Probe selection by missing dimension.
6. Timeout/no-change/privacy-blocked outcomes.
7. Failed probe cannot increase confidence.
8. Re-run only on material evidence change.
9. Late/stale probe result isolation.
10. Per-surface probe cooldown/budget behavior.
11. Compatibility derivation for legacy confidence fields.
12. Cache invalidation on material confidence/evidence revision.
13. Timeout/no-change produces `reran_decision = false` and no refreshed-evidence warning.

Full-pipeline critical assertions:

- Capture-button task confidence is driven by current turn evidence;
- Helium/Stremio identity/history cannot raise current task confidence;
- target openability is none without URL/path;
- simulated probe timeout leaves target confidence none and does not corrupt task truth;
- wrong-confident classification remains false;
- first divergence moves to recap validation or answer composition.

## Metrics and calibration

Extend the accuracy report with per-dimension coverage and calibration:

- count by predicted label and correctness;
- Brier score for binary checkpoint claims where labels permit;
- expected calibration error when the corpus is large enough;
- overconfident wrong count;
- underconfident correct count;
- probe success/change/timeout/privacy/failure rates;
- median and p95 probe duration;
- decision latency with and without probe.

Do not overinterpret calibration from the first small corpus. Report sample sizes. P6.09 will enforce final thresholds on a larger set.

## Audit requirements

Explicit audit output must include:

```text
confidence vector
claim dependency map
evidence/missing evidence per dimension
sufficiency decision
selected probe and rationale
per-probe result/status/duration
confidence before and after
rerun reason
stale/cancel state
```

## Acceptance criteria

P6.06 is complete when:

- Confidence dimensions are typed, evidence-linked, and claim-specific.
- Surface identity, task truth, execution/current-actor/waiting-on state, workstream alignment, and target openability are distinct.
- Public/internal legacy confidence fields have documented derivations.
- Observe-before-decide probes target missing dimensions and obey real budgets.
- Failed/no-change probes cannot raise confidence or force a pointless rerun.
- Probe trace and warnings state whether a rerun actually occurred.
- Critical fixture remains semantically correct under timeout/no-probe variants.
- Target confidence is none without a direct locator while task confidence may remain useful.
- Accuracy reports include confidence/probe metrics with sample sizes.
- Existing P0-P5 safety and latency tests remain passing.

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

Run accuracy cases for success, no-change, timeout, privacy-blocked, and stale-result probes. Report p50/p95 decision/probe timing when the harness supports it.

## Final response format

Report:

1. Files/contracts changed.
2. Confidence dimensions and claim dependencies.
3. Probe selection and outcome policy.
4. Compatibility mapping for old confidence fields.
5. Calibration/latency metrics with sample sizes.
6. Critical/counterfactual results.
7. Verification results.
8. Remaining P5 pack/validator weaknesses for P6.07.
9. Exact confidence/sufficiency contracts P6.07 may rely on.

Do not claim final user-facing confidence is complete. P6.06 makes confidence honest and usable by the recap and UI phases.
