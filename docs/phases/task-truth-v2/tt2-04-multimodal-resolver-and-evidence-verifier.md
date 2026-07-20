# Task Truth v2.04 — Multimodal Task Resolver And Evidence Verifier

## Codex task

Implement the component that actually interprets visible work: a provider-neutral multimodal Task Truth resolver operating on ObservationPackets, followed by a local claim/evidence verifier that does not validate against the legacy heuristic answer.

The model is a semantic sensor, not a prose rewriter. It must receive pixels plus structured temporal evidence and return a strict TaskSnapshot hypothesis with evidence references for every material claim.

Keep this path in shadow mode until the live release gates in v2.05 are satisfied.

## Dependency gate

Task Truth v2.01 through v2.03 must be complete. Read their completion reports, current schemas, the frozen evaluation policy, and the current provider/privacy configuration.

Inspect:

```text
src-tauri/src/continuation/task_truth_v2/
src-tauri/src/continuation/activity_recap_model.rs
src-tauri/src/continuation/activity_recap_validation.rs
src-tauri/src/continuation/activity_recap_truth.rs
src-tauri/src/continuation.rs
src-tauri/src/capture.rs
src-tauri/tests/fixtures/continue_accuracy/
```

The existing activity-recap model receives text only and is told to phrase an already-fixed fact pack. That behavior cannot recover a task lost by local attribution. Do not merely add more text to that prompt.

## Non-negotiable boundaries

- Explicit manual Continue only for initial multimodal inference.
- No continuous/background image upload.
- Existing privacy/exclusion policy applies before request construction.
- No raw typed characters or clipboard contents.
- No provider-specific types in the TaskSnapshot domain.
- No model authority over opening; strict-open stays local.
- No model authority over feedback, deletion, or other external actions.
- No acceptance merely because model output matches legacy P6.
- No site-specific chat geometry patches.
- No browser extension in this goal.

## Goal 1 — Define provider-neutral interfaces

Create narrow interfaces equivalent to:

```text
TaskTruthResolver
TaskTruthVerifier
TaskTruthModelClient
```

The resolver consumes a privacy-approved ObservationPacket plus previous valid snapshot and returns bounded hypotheses. The model client only handles transport/provider schema. The verifier is local and deterministic.

Support deterministic fixture responses for tests. A provider outage, timeout, invalid JSON, policy refusal, or unavailable model must produce a typed failure and safe local/ambiguity fallback, never a fake model-assisted answer.

## Goal 2 — Build the multimodal request

For an explicit Continue request, select:

```text
current active-window image
2-4 most relevant recent semantic keyframes
canonical element list with bounds/roles/source conflicts
causal interaction trace
before/after change regions
app/window/document identities
previous valid TaskSnapshot, when continuity is supported
privacy and missing-evidence notes
```

Use actual image inputs supported by the configured model API. Do not convert the screenshot into OCR text and call that multimodal.

Bound the request:

- crop or resize under a documented policy while retaining legibility;
- include only selected frames;
- exclude background/private displays;
- cap structured elements and text per source;
- publish byte/token/frame counts in audit;
- hash image references in durable audit unless policy explicitly permits a local path.

The current allowed-vocabulary mechanism must not cap the entire product's intelligence. The model may form grounded abstractions such as “debugging a login failure” from code, command, and error evidence. It still may not invent unsupported objects or actions.

## Goal 3 — Require strict structured hypotheses

Use a versioned strict schema. Each hypothesis must contain:

```text
task_summary
task_kind
task_object
user_goal
app/surface/document identity
execution_state
current_actor
waiting_on
last_meaningful_progress
unfinished_step
next_action
relation to prior snapshot
claim_evidence by field
alternative hypotheses
contradictions
confidence by field
resolution status
```

Every evidence reference must point to an element, frame region, event, transition, or previous snapshot present in the request. Free-form citations are invalid.

The resolver must be allowed to return:

```text
resolved
ambiguous with 2 bounded hypotheses
insufficient evidence
privacy blocked
model unavailable
```

Do not force all fields non-null. Missing one field must not erase well-supported fields.

## Goal 4 — Ask task-understanding questions, not copy questions

The system instruction must require the model to:

1. identify the primary task-bearing region;
2. separate user-created content, app/agent output, controls, navigation, and third-party content;
3. use interaction causality as stronger authorship evidence than geometry;
4. infer the user's goal and work object;
5. distinguish composing, editing, reviewing, waiting, debugging, searching, comparing, blocked, and completed states;
6. identify last meaningful progress and unfinished work;
7. give a next action only when it follows from the unfinished state;
8. preserve alternatives when two interpretations are close;
9. keep return anchors separate from task understanding;
10. cite evidence for every material field.

Do not tell it to restate a locally selected candidate or stay within a list of legacy task terms.

## Goal 5 — Implement a local evidence verifier

Validate support, not agreement with P6.

Required deterministic checks:

- every task noun/action has at least one eligible evidence reference;
- referenced ids exist in the packet;
- controls/navigation/browser chrome cannot be authored goals;
- historical text needs a temporal continuity edge before becoming current;
- user authorship needs causal evidence, explicit semantic role, or strong cross-modal agreement;
- the chosen snapshot is temporally newer than a superseded task;
- lifecycle state is compatible with the cited events/content;
- next action follows from unfinished state and is not generic invention;
- return anchor belongs to the selected task but cannot raise task confidence;
- contradictions reduce the affected field rather than silently invalidating unrelated supported fields;
- privacy-blocked sources cannot support public claims;
- unsupported specific claims are removed/rejected, not softened into confident prose.

Record per-field verdicts:

```text
accepted
downgraded
removed
ambiguous
unsupported
contradicted
```

Build the final verified TaskSnapshot only from accepted/downgraded fields.

## Goal 6 — Add a bounded second pass only for difficult cases

Do not double inference cost by default. A verifier/reconciliation pass may run only when:

```text
top two task hypotheses are close
critical field confidence is near threshold
AX/OCR/visual sources conflict
a selected claim references a control/navigation region
task and return anchor disagree
temporal ordering is contradictory
```

The second pass receives the conflict, competing hypotheses, and evidence subset. It cannot see or copy the legacy P6 answer as ground truth.

Audit why it ran, latency, cost estimate, and whether it changed any field.

## Goal 7 — Separate understanding from wording

After verification, generate deterministic first-screen wording from the verified TaskSnapshot. Optional model wording may improve readability only if it preserves:

```text
snapshot id/revision
task identity
field nullability
lifecycle axes
next-action support
target policy
claim evidence
```

This wording step is distinct from the Task Truth resolver and has separate provenance. Rejecting wording must not discard a valid snapshot.

## Goal 8 — Evaluate model and local paths honestly

Run the frozen v2.02 evaluator on:

```text
legacy P6
causally repaired local Task Truth
multimodal resolver before verification
multimodal resolver after verification
```

Report whether verification improves unsupported-claim rate without destroying task recall. Publish model name/version/config, but select models by locked task-truth results rather than brand.

Do not tune against locked holdout. Use development, then validation. Open the holdout only under the frozen policy.

## Required adversarial tests

1. Model selects a button label as the goal.
2. Model selects old completed text as current.
3. Model invents a file/thread identity.
4. Model gives a plausible but unsupported next step.
5. Model returns a target for another task.
6. Evidence ids do not exist.
7. Two hypotheses are close.
8. AX and screenshot interpretation conflict.
9. Current image is private/blocked.
10. Provider times out or returns invalid JSON.
11. Model wording changes task identity.
12. Correct task has no direct open target.
13. Unfamiliar application has pixels plus events but thin AX.
14. Session-013 contains the real user submission and `Approve for me` control.

## Verification

```text
cargo test continuation::task_truth_v2
cargo test continuation::activity_recap_validation
cargo test continuation::accuracy_eval
cargo check
cargo fmt --check
```

Run deterministic model fixtures without network. Then run an explicit manual provider smoke test only when credentials are configured safely. Never print keys or private image contents.

Record:

```text
request construction proof
image count and bounded sizes
schema validation
verifier decisions
fallback behavior
per-path corpus metrics
p50/p95 latency
estimated request cost
privacy exclusions
```

## Definition of done

- The model receives real images and structured temporal evidence.
- It returns strict evidence-linked task hypotheses, not recap prose.
- Local verification checks evidence support independently of P6.
- Ambiguity and provider failure are first-class results.
- Understanding, wording, and target selection have separate provenance.
- Shadow evaluation shows field-level outcomes before and after verification.
- No background multimodal calls occur.
- Session-013 no longer depends on geometry or prior-boundary fallback for user authorship.
- The production UI still uses the existing authority until v2.05 gates pass.

Do not mark the overall Task Truth v2 program complete.
