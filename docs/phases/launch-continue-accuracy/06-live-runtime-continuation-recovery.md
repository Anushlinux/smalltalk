# LCA-06 — Live Runtime Continuation Recovery

## Codex implementation task

Repair the complete normal manual `Continue` path so a fresh answer cannot be presented as stale, a visible current task can reach the semantic model with trustworthy attribution, useful model output is not erased by target failure, impossible provider requests are not sent, and an automatic screenshot cannot repeatedly prevent a user-requested Continue boundary.

This is a production-path correction to LCA-01 through LCA-05. It exists because live use contradicted the deterministic completion audits. Do not treat this as copy polish, prompt tuning, or a reason to add another semantic engine. Repair the broken boundaries in their owning components and update the existing contracts honestly.

The work is complete only when deterministic tests, live-shaped database replay, provider-request accounting, React/native parity, workload contention tests, and the user-owned normal-app verification all agree.

## Why this phase exists

The live runtime on 2026-07-19 proved four failures that the deterministic LCA gate did not catch.

### Failure A — fresh decisions were falsely presented as stale

Recent live evidence showed:

```text
new answer contracts inspected: 9
contracts persisted as stale_decision: 9
admitted useful semantic answers: 0
```

The stale first-screen sentence came from local product projection, not from GPT. `attach_strict_target(...)` converted an unproven or absent target owner into `target_status = stale_decision`. The canonical projection then allowed target status to replace the complete semantic instruction with:

```text
The saved answer is older than the latest work.
```

This violates the already documented rule that semantic truth and target readiness are independent.

### Failure B — task evidence was visible but never became authoritative

Recent live evidence showed:

```text
salient evidence rows inspected: 38
rows with an admitted latest-user-goal span: 0
rows with an admitted current-agent-state span: 0
selected current task turns: 0
```

The live frame contained a likely user message, but the geometric heuristic assigned confidence `0.58` while typed task selection required `0.64`. The manual boundary did not retain the causal typing attribution available on preceding exact-app/window frames. The observation packet therefore sent:

```text
current_task_turn: null
latest_user_goal_sample: null
current_agent_state_sample: null
```

The model correctly abstained or proposed fields that the local verifier later removed.

### Failure C — larger provider requests could not produce admissible value

The inspected live window contained:

```text
provider posts: 6
total tokens: 66,478
estimated cost: about $0.102853
support-slot validation failures: 5
publicly useful admitted answers: 0
```

The larger chronological packet and images were bounded, but the request was still sent when no semantic task field could pass local admission. More context therefore increased cost without increasing product usefulness.

### Failure D — automatic screenshot work timed out inside the governor

The live capture store recorded:

```text
workload governor timed out waiting for screenshotcapture
```

At least one timeout belonged to an automatic click capture, while a following manual boundary succeeded. The timeout is therefore not the sole cause of the false stale message, but it can remove the exact evidence that task attribution needs. Manual boundary capture and low-value automatic capture currently compete inside the same capture work group and deadline policy.

## Product invariant

After this phase, a user pressing Continue must receive exactly one of these truthful outcomes:

1. A supported next instruction with a short resume state and a safe direct or inspect action.
2. A known task with no supported action, stated plainly without inventing a next step.
3. A task-acquisition failure that explains that the current task could not be verified from captured evidence.
4. A typed provider, parser, validation, privacy, or capture failure.
5. A genuinely stale saved answer, only when the answer's material evidence identity is older than current material evidence.

Target unavailability, target ownership mismatch, missing URL/path, frame-preview-only evidence, or a null return candidate must never be presented as an old answer.

## Hard dependency and contradiction audit

Before editing code, read and compare:

```text
AGENTS.md
PRODUCT.md
docs/full-engine-flow.md
docs/current-island-ui-ux.md
docs/phases/launch-continue-accuracy/01-task-relevant-evidence-packet.md
docs/phases/launch-continue-accuracy/01-task-relevant-evidence-packet-completion-audit.md
docs/phases/launch-continue-accuracy/02-actionable-continuation-contract.md
docs/phases/launch-continue-accuracy/02-actionable-continuation-contract-completion-audit.md
docs/phases/launch-continue-accuracy/03-truthful-admission-and-authority.md
docs/phases/launch-continue-accuracy/03-truthful-admission-and-authority-completion-audit.md
docs/phases/launch-continue-accuracy/04-one-instruction-ui-parity.md
docs/phases/launch-continue-accuracy/04-one-instruction-ui-parity-completion-audit.md
docs/phases/launch-continue-accuracy/05-four-log-replay-and-launch-gate.md
docs/phases/launch-continue-accuracy/05-four-log-replay-and-launch-gate-completion-audit.md
docs/phases/p6-task-turn-accuracy/p6-02-ordered-role-region-evidence.md
docs/phases/p6-task-turn-accuracy/p6-03-current-task-turn-lifecycle.md
docs/phases/p6-task-turn-accuracy/p6-08-target-truth-and-answer-composition.md
docs/phases/p6-task-turn-accuracy/p6-09-longitudinal-release-gate.md
docs/phases/runtime-stability/runtime-04-background-workload-governor.md
```

Create a written contradiction matrix in the completion section of this same file. At minimum reconcile these contradictions:

- LCA-03 says target failure must not erase semantic meaning, while current target attachment can force the stale headline.
- LCA-04 reserves stale presentation for genuinely old evidence, while current code uses it for target identity mismatch.
- LCA-01 expects attributed current task evidence, while the live manual boundary loses recent causal typing attribution.
- LCA-05 fixture replay begins with evidence quality that the live capture path did not produce.
- Runtime-04 gives explicit user actions priority, while an automatic capture can consume the screenshot deadline needed by manual Continue.
- The P6 release gate remains false, but current manual shadow inference is visible in the normal product surface.

Do not preserve a completion-audit claim that live evidence has disproved. Update the affected audit with a dated correction rather than deleting prior evidence.

## Hard operating rules

- Preserve the dirty worktree and all unrelated user changes.
- Do not add a second task-understanding, freshness, target, or presentation engine.
- Do not fix this by changing only the stale sentence.
- Do not fix task evidence by globally lowering confidence thresholds.
- Do not treat visual prominence, app name, dwell time, frame count, or interaction count as task authority.
- Do not let screenshot evidence alone claim that a visible sentence was submitted by the user.
- Do not weaken privacy, ownership, chronology, strict-open, correction scope, or target validation.
- Do not add a second provider request, reconciliation request, HTTP retry, or background semantic upload.
- Do not send a provider request when local preflight proves that no semantic field can be admitted.
- Do not expose raw support slots, confidence values, frame IDs, or internal failure codes on the first screen.
- Do not mutate immutable history rows when a live answer later becomes stale.
- Do not claim live, visual, provider, or launch approval from deterministic tests.
- Do not declare completion while the P6 release report remains falsely described as passed.
- Do not spend time redesigning island geometry, animation, or unrelated visual styling.

## Required implementation order

Implement in this order. Each stage has a dependency gate. Do not compensate for an earlier failure in a later layer.

## Stage 0 — Capture a permanent live-shaped failing replay

Before changing behavior, create privacy-safe fixtures that reproduce the current failures through production transformations.

The fixture must not contain raw private conversation text, screenshots, local paths, URLs, credentials, or database copies. Preserve only the causal shape and bounded synthetic text.

Required cases:

### LCA-06-01 — fresh unresolved answer with no target

```text
fresh manual boundary
model status unresolved
no admitted unfinished task
no return candidate
expected presentation: task acquisition or semantic unresolved
forbidden presentation: stale_decision
```

### LCA-06-02 — fresh useful answer with frame preview only

```text
fresh manual boundary
admitted task, resume point, and next action
no strict direct target owner
aligned evidence preview exists
expected presentation: useful instruction plus View last screen or Inspect
forbidden presentation: stale_decision
```

### LCA-06-03 — fresh useful answer with mismatched target candidate

```text
fresh semantic answer
candidate target cannot prove ownership of the semantic answer
expected: semantic answer preserved, target_suppressed
forbidden: stale answer headline or open action
```

### LCA-06-04 — genuinely stale remembered answer

```text
saved answer material watermark is older than current material evidence
expected: stale_decision, Refresh Continue, no direct open
```

### LCA-06-05 — typed goal followed by manual boundary

```text
committed user input on an exact app/window surface
post-commit frame
manual Continue frame on the same unchanged surface
expected: attributed latest-user-goal span and selected current task turn
```

### LCA-06-06 — unconfirmed right-aligned text without typing proof

```text
geometric user-like text
no exact-app/window causal typing evidence
expected: no explicit-user claim
expected: no unsafe threshold promotion
```

### LCA-06-07 — provider request cannot pass admission

```text
no attributed goal, draft, task object, or task-linked agent state
expected provider posts: 0
expected product result: typed task-evidence acquisition failure
```

### LCA-06-08 — automatic capture contends with manual boundary

```text
automatic click/scroll capture active or queued
user presses Continue
expected: manual boundary receives next safe service or safely reuses an exact recent frame
forbidden: repeated screenshotcapture timeout loop
```

The initial replay must fail for the same reasons as the live runtime. Record the first divergent checkpoint for every case.

### Stage 0 gate

- The current code reproduces false stale projection in LCA-06-01, 02, or 03.
- The current code reproduces missing current task attribution in LCA-06-05.
- The current code proves zero unsafe promotion in LCA-06-06.
- The current code demonstrates the unnecessary-call or contention behavior in LCA-06-07 and 08.
- Fixtures pass privacy lint.

## Stage 1 — Make freshness and target readiness independent

Audit and repair:

```text
src-tauri/src/continuation/task_truth_v2/production.rs
src-tauri/src/continuation.rs
src-tauri/src/session_island.rs
src-tauri/src/session_island/contract.rs
src/continuePresentation.ts
src/App.tsx
src-tauri/macos/SessionIslandPanel.swift
```

### Required state semantics

Use one authoritative freshness calculation based on answer identity and current material evidence.

`stale_decision` is allowed only when at least one of these is proven:

```text
answer evidence watermark is older than the current material evidence watermark
answer current-frame identity no longer matches the material boundary it claims
a correction/feedback watermark invalidates the saved admitted result
a privacy/retention change invalidates evidence required by the saved answer
```

These conditions are not stale-answer proof:

```text
target owner is missing
target owner does not match the semantic task thread
target candidate is null
target is not openable
URL/path is unavailable
only a frame preview exists
semantic answer is unresolved
release authority remains shadow
```

Map target-only failures as follows:

```text
useful task plus aligned preview        -> frame_preview_only
useful task without aligned preview     -> task_known_target_unknown
candidate exists but cannot be trusted  -> target_suppressed
no admitted task                        -> no_task
genuinely old answer                    -> stale_decision
```

### Target attachment rules

- Call strict target attachment only with an actual candidate target.
- A failed ownership proof removes the target and open action only.
- A failed ownership proof records a diagnostic reason without changing semantic freshness.
- Direct target attachment still requires exact decision, task, revision, locator, privacy, and openability proof.
- The canonical projection must choose its semantic instruction before considering target action availability.
- Target status may change the button. It must not replace a useful semantic instruction except for a genuinely stale answer.

### Stage 1 gate

- LCA-06-01, 02, and 03 never display stale copy.
- LCA-06-04 displays stale copy and cannot open.
- React and native island render the same instruction and target action.
- A fresh validation failure remains a validation failure.
- A fresh provider failure remains a provider failure.
- Existing strict-open and stale-open-block tests remain green.

## Stage 2 — Restore trustworthy current-task evidence at the manual boundary

Audit and repair:

```text
src-tauri/src/capture.rs
src-tauri/src/continuation/task_turn_evidence.rs
src-tauri/src/continuation/task_turn.rs
src-tauri/src/continuation/task_truth_v2/observation_packet.rs
src-tauri/src/continuation/task_truth_v2/semantic_probe.rs
src-tauri/src/continuation/semantic_consistency.rs
```

### Preserve causal typing evidence across the boundary

When manual Continue creates an exact external frame, it may carry forward recent causal typing attribution only when all of the following are true:

```text
same session
same exact app identity
same exact window identity, or a separately proven stable window replacement
commit event precedes the manual boundary
post-commit frame exists
manual frame is not older than the commit
the attributed span or its stable text hash remains visible on the manual frame
no intervening task-bearing app/window transition
privacy remains eligible
association is inside a documented short time bound
```

Do not copy attribution merely because the app name matches. Persist the linkage and its reason so replay can verify it.

### Select the latest eligible task evidence, not merely the last row

The packet builder must not let a newer empty sampling row erase a recent valid task boundary on the same proven surface.

Search backward only within a strict bounded window and require:

```text
same session and proven surface lineage
chronology before or at manual cutoff
no superseding attributed user goal
no private or ownership-invalid evidence
materially relevant task role
confidence at or above the existing typed threshold
```

If these conditions cannot be proven, remain unresolved. Do not use recency, engagement, or app labels as substitutes.

### Preserve confidence safety

- Keep the global typed-role threshold unless labeled replay proves it is wrong.
- Do not raise the unconfirmed geometric user message from `0.58` to the typed threshold.
- Exact causal typing may promote the correctly associated span using its measured confidence.
- A user-like span without causal or native role proof remains non-authoritative.
- The current task turn must reference the actual admitted span IDs and session/surface identity.

### Required packet result

For LCA-06-05, the provider request must contain:

```text
current_task_turn: non-null
latest_user_goal_sample: bounded and non-null when privacy allows
latest_user goal evidence role: present
current agent state: present when an attributed later state exists
explicit_goal_support_slots: non-empty only for submitted attributed user evidence
```

### Stage 2 gate

- LCA-06-05 produces a selected current task turn.
- LCA-06-06 remains safely unresolved.
- A newer empty evidence row cannot erase a still-current proven task boundary.
- A real app/window/task switch invalidates the carried task boundary.
- Typed text is never stored raw beyond existing bounded privacy-safe summaries.
- Current task, current activity, resume target, and return target remain separate objects.

## Stage 3 — Add admission-feasibility preflight and reduce wasted provider input

Audit and repair:

```text
src-tauri/src/continuation/task_truth_v2/semantic_probe.rs
src-tauri/src/continuation/task_truth_v2/verifier.rs
src-tauri/src/continuation/task_truth_v2/production.rs
src-tauri/src/continuation/task_truth_v2/review.rs
src-tauri/src/continuation/accuracy_eval.rs
```

### Admission-feasibility preflight

Before any provider request, compute which semantic fields have at least one eligible support path under the same validator rules that will run after the response.

Record:

```text
eligible_fields
ineligible_fields with typed reasons
eligible support-slot categories
whether unfinished_task can be admitted
whether resume_point can be admitted
whether next_supported_action can be admitted
whether a useful partly-resolved result is possible
```

Provider posting is allowed only when a useful semantic result is possible. At minimum, one of these must hold:

```text
unfinished task has attributed support
current unsent draft has attributed support
task-linked agent/result state can support a resume point for an already proven task
completed implementation plus user verification state has attributed support
```

If no useful field can be admitted:

- do not call the provider;
- persist `provider_post_count = 0`;
- expose a typed local acquisition failure;
- keep the evidence inspectable;
- do not manufacture task prose;
- do not label the answer stale.

### Request usefulness and size

Keep hard privacy and request bounds. Then remove payload that cannot affect any admissible field.

Required behavior:

- Send only task-relevant or boundary-relevant images.
- Preserve a factual final screen only when it can support an allowed field or explain a typed unresolved state.
- Do not send repeated near-duplicate images.
- Do not serialize dozens of rejected OCR spans when their only effect is diagnostics.
- Keep diagnostic selection counts in local audit, not necessarily in provider input.
- Keep no more than the existing hard maximum of four images; prefer fewer when the semantic roles are already covered.
- Keep one provider call, zero reconciliation calls, zero automatic retries, and zero background uploads.

Add metrics:

```text
provider_calls_avoided_no_admissible_fields
provider_input_tokens_per_admitted_field
provider_posts_with_no_admitted_semantics
support_validation_failure_rate
task_authority_missing_rate
image_count by outcome
structured bytes by outcome
```

### Admission behavior

- Reject unsupported fields independently.
- Preserve every separately supported field.
- Do not let visit-role failure erase unrelated semantic fields.
- Do not convert target failure into semantic failure.
- Do not convert semantic validation failure into stale state.
- Do not admit an unfinished task from a bare context image.
- A task-linked image may support resume state or location only under the field-specific policy.
- The model remains the semantic author; local code may validate, preserve, weaken, or reject, but not invent replacement meaning.

### Stage 3 gate

- LCA-06-07 produces zero provider posts.
- All provider-posting fixtures have at least one field that could be admitted before the call.
- A valid partly-resolved answer preserves its supported fields.
- Provider, parser, validation, insufficient-evidence, capture, and privacy failures remain distinct.
- Request token and image counts are reported by result class.
- No privacy regression occurs.

## Stage 4 — Give manual boundary capture truthful priority

Audit and repair:

```text
src-tauri/src/workload.rs
src-tauri/src/capture.rs
src-tauri/src/capture/runtime_stability.rs
src-tauri/src/session_island/gateway.rs
```

### Required scheduler behavior

Manual boundary capture is not equivalent to an automatic click, scroll, timer, or deduplicated background screenshot.

Implement an explicit distinction using the smallest coherent change to the current workload model. Acceptable designs include a dedicated manual-boundary capture class or a documented priority override on a capture request. The result must provide:

```text
manual boundary has user priority
queued low-value automatic capture may be coalesced, superseded, or cancelled safely
active capture receives bounded cancellation or completion handling
manual request is next when capture ownership is released
automatic capture cannot repeatedly requeue ahead of manual capture
no SQLite connection or transaction is held while waiting
no global lock blocks cheap status or event ingest
shutdown wakes and cancels every waiter
```

Do not merely increase the three-second timeout. Deadline tuning may follow measurement, but it cannot substitute for correct priority and cancellation.

### Safe exact-frame reuse

If an immediately recent frame already satisfies the manual boundary's exact app/window, post-event, privacy, readable-asset, and chronology requirements, reuse it rather than forcing another screenshot. Record that reuse explicitly. Never reuse a Smalltalk-self, wrong-window, pre-event, private, missing-file, or stale frame.

### Failure propagation

When manual boundary acquisition still fails:

- mark the trigger failed with the real typed reason;
- do not perform a provider call for an unbuilt boundary;
- do not leave the remembered answer permanently in `NeedsRefresh`;
- expose a retryable capture-specific product state;
- preserve the prior immutable answer in history;
- clear transient runtime error state only after a successful relevant operation.

### Stage 4 gate

- LCA-06-08 passes under deterministic contention.
- Manual capture begins within the documented bound when automatic work is queued.
- Automatic capture failure does not poison the next manual request.
- Repeated capture-pressure tests do not produce an unbounded queue.
- Screenshot, OCR, Accessibility, derived evidence, and Continue permits are always released on success, error, cancellation, and panic-safe unwind.
- Workload diagnostics distinguish manual-boundary capture from automatic capture outcomes.

## Stage 5 — Repair React, island, history, and cache lifecycle

### Canonical presentation rules

Both React and the native island must render the backend-owned product projection without independently recreating semantic meaning.

Required first-screen mapping:

```text
fresh supported action + direct target     -> instruction + Continue here
fresh supported action + preview           -> instruction + View last screen
fresh supported action + no target         -> instruction + Inspect when available
fresh known task + no action                -> task-known/action-unknown copy
fresh unknown task                          -> task-acquisition copy
provider/parser/validation/capture failure  -> exact typed failure copy
genuinely stale saved answer                -> stale copy + Refresh Continue
```

### Cache rules

- Semantic-result reuse requires the existing complete cache identity.
- A new manual decision revalidates freshness and target safety.
- Reused semantics do not inherit an old target object without revalidation.
- A target mismatch cannot change the cached semantic result into stale.
- A changed material evidence watermark invalidates semantic-result reuse.
- Low-value raw event noise does not invalidate a result.
- A failed or cancelled background result cannot replace a newer manual result.

### History rules

- History stores the exact final canonical projection and identity shown at save time.
- Later freshness changes do not mutate historical copy.
- Selecting history never opens a target.
- History does not make a saved answer the live current answer.

### Stage 5 gate

- React and island agree for every LCA-06 state.
- No fresh state shows the stale sentence.
- No typed failure falls back to stale copy.
- No target-unavailable state exposes `Continue here`.
- History remains read-only and immutable.
- Existing visual cue, geometry, capture controls, hover, expansion, and motion behavior remain unchanged.

## Stage 6 — Full production-path verification

Run the LCA-06 fixtures through the real chain:

```text
event and typing attribution
capture trigger and manual boundary
frame/AX/OCR ownership resolution
ordered role/region evidence
salient turn selection
current task turn
observation packet
admission-feasibility preflight
provider request envelope or intentional no-call
deterministic provider response
field admission
semantic production answer
strict target attachment
freshness calculation
canonical product projection
React mapping
native island wire mapping
history persistence
open-action eligibility
```

Do not begin the final replay at a prebuilt task turn, prebuilt provider request, or prebuilt product projection. Component tests remain useful, but they do not satisfy this gate.

### Required zero-tolerance metrics

```text
false stale classification count = 0
fresh target mismatch shown as stale count = 0
target failure erasing supported semantics count = 0
provider call with zero admissible fields count = 0
manual boundary lost behind queued automatic capture count = 0
unsupported public semantic claim count = 0
unsafe direct-open count = 0
React/island semantic disagreement count = 0
history mutation count = 0
privacy-lint violation count = 0
deterministic replay disagreement count = 0
```

### Required coverage metrics

Report numerator, denominator, exclusions, and rate for:

```text
manual committed-goal attribution
current task turn creation
current agent state attribution
supported task admission
supported next-action admission
provider calls avoided by feasibility preflight
provider posts yielding at least one admitted semantic field
manual boundary acquisition success
automatic capture timeout recovery
true stale detection
fresh non-stale detection
```

An unknown or missing denominator is incomplete, not passing.

## Automated verification commands

Run focused tests during implementation, then no more than two broad final attempts unless the user explicitly authorizes more.

```bash
cd src-tauri && cargo fmt --all -- --check
cd src-tauri && cargo check
cd src-tauri && cargo test lca_06 --lib
cd src-tauri && cargo test product_projection --lib
cd src-tauri && cargo test semantic_probe --lib
cd src-tauri && cargo test task_turn --lib
cd src-tauri && cargo test workload --lib
cd src-tauri && cargo test session_island --lib
cd src-tauri && cargo test history --lib
cd src-tauri && cargo test
npm run build
npm run test:webview
swiftc -typecheck src-tauri/macos/SessionIslandPanel.swift
cd src-tauri && cargo run --features eval-binaries --bin continue_accuracy_eval -- --repeat 3
git diff --check
git status --short
```

If a broad command fails, diagnose the first failure, fix the owning contract, run focused verification, and use the second broad attempt as the final broad proof. Do not repeatedly rerun a broad suite until it becomes green by chance.

## Live database verification

Use the database path returned by `capture_status`. Open it read-only. Do not guess a stale database path and do not modify the live database during diagnosis.

For each user-owned live scenario, record a privacy-safe row containing:

```text
scenario label
manual decision id
manual boundary trigger status
current frame id
current task turn present or absent
provider post count
input/output token counts
raw model status
admitted semantic status
target status
presentation state
primary instruction category, not private verbatim text
React/native identity agreement
open action kind
first divergent checkpoint when failed
```

Required runtime assertions:

- A fresh decision never persists `stale_decision` solely because target ownership is missing.
- A true watermark advance marks the remembered answer stale until refresh.
- A successful refresh produces a new fresh result or a typed current failure.
- A capture failure does not silently reuse stale semantics as a new answer.
- A provider request with no admissible semantic field has `provider_post_count = 0`.
- Token cost and support-validation outcome are visible in developer diagnostics.

## User-owned normal-app verification

Codex must prepare this script but must not claim its results. The user runs the normal `npm run tauri dev` application.

### Manual 1 — explicit current goal in Codex or ChatGPT

1. Submit a concrete task.
2. Let the agent visibly begin work.
3. Switch away briefly and return.
4. Press Continue.
5. Expected: the task or supported next instruction is recovered.
6. Forbidden: saved-answer stale copy for the new decision.

### Manual 2 — task known, direct target unavailable

1. Work in a surface that has no safely openable URL/path.
2. Press Continue.
3. Expected: useful semantic instruction remains visible with Inspect or no direct action.
4. Forbidden: target mismatch erases the instruction.

### Manual 3 — true stale remembered answer

1. Generate a useful Continue answer.
2. Perform materially different new work.
3. Observe the remembered state before refresh.
4. Expected: stale copy and Refresh Continue.
5. Refresh.
6. Expected: stale state clears into a new answer or exact typed current failure.

### Manual 4 — capture contention

1. Generate ordinary click/scroll activity.
2. Immediately press Continue.
3. Expected: manual boundary succeeds or reports one capture-specific retryable failure.
4. Forbidden: repeated screenshot-governor timeout loop or permanent NeedsRefresh state.

### Manual 5 — thin evidence

1. Open a surface without an attributable user goal or task-linked state.
2. Press Continue.
3. Expected: precise task-acquisition abstention.
4. Expected provider posts: zero when feasibility preflight finds no admissible field.
5. Forbidden: invented task or stale saved-answer copy.

### Manual 6 — React/native parity and history

1. Produce one fresh answer and one genuinely stale state.
2. Compare React and island instruction/action meaning.
3. Open read-only history.
4. Expected: live surfaces agree; historical answer remains unchanged; history cannot open a target.

The user records `Pass`, `Fail`, or `Not run` for each scenario. `Not run` is not approval.

## Release and rollback rules

### Automated implementation pass

May be reported only when:

- all LCA-06 deterministic cases pass through the production chain;
- all zero-tolerance metrics are zero;
- builds, Rust tests, webview tests, and Swift type-check pass;
- provider-call and token-accounting assertions pass;
- existing LCA and strict-open regressions remain green;
- completion evidence is appended to this file.

Required verdict:

```text
AUTOMATED PASS — LCA-06 implementation is ready for user-owned live verification
```

### Product recovery pass

May be reported only after all six user-owned manual scenarios pass and their privacy-safe results are recorded.

Required verdict:

```text
PRODUCT PASS — fresh Continue recovery, truthful stale state, capture priority, and React/island parity are live-verified
```

### Launch claim

Do not call the product launch-ready merely because LCA-06 passes. The existing P6 release gate remains authoritative. If it remains false, report that explicitly.

### Immediate rollback conditions

Rollback the LCA-06 change set or disable its public authority if any of these occur:

```text
fresh answers again show stale copy
target mismatch changes semantic meaning
unsupported task claims become public
manual Continue performs more than one provider post
provider calls occur with zero admissible fields
manual capture repeatedly times out behind automatic work
direct target opens without exact authority
React and island show different instructions for one answer identity
privacy-safe text or image bounds are exceeded
```

Rollback must preserve the last known truthful thin/unresolved behavior. Do not fall back to invented local semantic prose.

## Completion record to append in this same file

Do not create a separate completion-audit Markdown file for LCA-06. Append these sections here after implementation:

```text
Implementation date
Before/after behavior
Contradiction matrix
Files changed and responsibility of each
Freshness versus target state table
Task-evidence continuity proof
Admission-feasibility and provider-cost proof
Workload priority and timeout proof
LCA-06 fixture results
Zero-tolerance metrics
Coverage metrics
Automated commands and exact results
Live database privacy-safe observations
User-owned manual results
Remaining limitations
P6 release-gate status
Final verdict
```

Do not write a passing final verdict while any required result is failed, unknown, excluded without justification, or not run.

---

# Implementation record

## Implementation date

2026-07-19

## Before and after behavior

Before this repair, a missing or mismatched target could turn a fresh semantic answer into stale copy. The manual frame could lose the submitted user turn visible on the preceding post-commit frame. Provider transport could begin even when no semantic field had an admissible support path. Automatic and manual screenshot work used the same capture class.

After this repair, semantic freshness and target readiness are separate. A material evidence watermark is the freshness identity. Exact causal typing may cross the manual boundary, while geometric text without that proof remains below the typed threshold. Provider feasibility is computed before transport. Manual boundary capture has a separate user-priority class, and the manual preflight holds no SQLite connection while it waits.

## Contradiction matrix

| Earlier claim | Live contradiction | LCA-06 correction | Current truth |
| --- | --- | --- | --- |
| LCA-03 target failure is field-local | Target mismatch could set `stale_decision` and erase semantics | Strict attachment runs only for a candidate; mismatch becomes `target_suppressed` | Component regression passes; live scenario not run |
| LCA-04 stale means genuinely old evidence | React/native used counts and timestamps and inherited target-created stale state | Backend material watermark is shared; ambient count churn is ignored | React, Rust wire, and Swift type checks pass; visual parity not run |
| LCA-01 supplies attributed current task evidence | Manual boundary lost recent causal typing | Exact post-frame user hash may carry across an unchanged manual surface for 15 seconds | Live-shaped database test passes |
| LCA-05 replay represented the normal path | Replay began after evidence quality the live path failed to produce | Added eight privacy-safe LCA-06 causal shapes and boundary-focused production tests | Full eight-case production-chain replay is not yet implemented |
| Runtime-04 gives explicit work priority | Automatic screenshot could consume the capture service window | Added `ManualBoundaryCapture`, supersession, cancellation, and requeue blocking | Deterministic contention passes; live contention not run |
| P6 release authority is closed | Manual shadow output remained visible in the normal surface | LCA-06 preserves truthful unresolved/typed failure states and does not alter the release gate | P6 remains false |

## Files changed and responsibility

| Area | Files | Responsibility |
| --- | --- | --- |
| Capture boundary | `src-tauri/src/capture.rs`, `src-tauri/src/workload.rs` | Exact post-event reuse, no held database connection, manual capture class, cancellation and diagnostics |
| Task evidence | `src-tauri/src/continuation/task_turn_evidence.rs`, `task_turn.rs` | Exact causal carry-forward, stable user hash, same-surface empty-boundary preservation, wrong-surface/prior-only rejection |
| Provider admission | `src-tauri/src/continuation/task_truth_v2/semantic_probe.rs` | Validator-aligned feasibility audit and zero-post acquisition failure |
| Semantic/target state | `src-tauri/src/continuation.rs`, `task_truth_v2/production.rs` | Candidate-only strict attachment, target-only suppression, capture failure projection |
| Freshness and parity | `src-tauri/src/session_island.rs`, `session_island/contract.rs`, `session_island/gateway.rs`, `src/App.tsx`, `src-tauri/macos/SessionIslandPanel.swift` | Material watermark freshness, stale open blocking, shared backend projection |
| Fixture and doctrine | `lca-06-live-runtime-replay.v1.json`, LCA audits, runtime audit, `PRODUCT.md`, engine-flow and island docs | Privacy-safe causal cases and dated correction of disproved claims |

## Freshness versus target state

| Semantic freshness | Target result | Presentation/action |
| --- | --- | --- |
| Fresh useful answer | Exact validated direct target | Preserve instruction; `Continue here` |
| Fresh useful answer | Aligned preview | Preserve instruction; `View last screen` |
| Fresh useful answer | Missing target | Preserve instruction; Inspect or no direct action |
| Fresh useful answer | Ownership mismatch | Preserve instruction; `target_suppressed`; no open |
| Fresh unresolved/failure | Any non-proven target | Exact unresolved or typed failure; never stale |
| Material watermark advanced | Any prior target | Stale copy; `Refresh Continue`; no open |

## Task-evidence continuity proof

The live-shaped three-frame test stores a committed typing burst on frame 2, with both the user bubble and a later agent state visible. Frame 3 is a manual boundary on the same exact app/window. The resolver selects the previously admitted user hash, not the equally novel agent text, persists `manual_boundary_visible_hash_carry_forward`, and produces a selected current task turn. Separate tests prove an empty same-surface boundary may preserve that turn, while a wrong surface and a prior-only row clear it. The existing unconfirmed geometric path remains below the typed threshold.

## Admission feasibility and provider-cost proof

`ProbeRequestAudit` now records eligible/ineligible fields, typed reasons, eligible support categories, field-specific feasibility, and whether a useful partly resolved result is possible. `run_probe` checks this before credentials or transport. The LCA-06 no-authority case uses a non-empty sentinel credential and still records `provider_post_count = 0` with `task_evidence_acquisition_failure:no_admissible_semantic_fields`. A separate test proves that an admissible request without credentials remains a typed provider-unavailable failure.

No live provider request was made. Token-per-admitted-field and result-class cost denominators therefore remain unknown.

## Workload priority and timeout proof

`ManualBoundaryCapture` runs at user priority. Enqueueing it supersedes queued automatic screenshots and requests cancellation of an active automatic screenshot. Automatic screenshots cannot requeue while a manual boundary waits. Three LCA-06 workload tests cover queued supersession, active cancellation/requeue prevention, and distinct outcome diagnostics. The manual resolver closes its first database connection before entering capture/governor work.

The tests prove deterministic service ordering. They do not prove a live wall-clock acquisition bound or a soak result.

## LCA-06 fixture results

| Case | Deterministic evidence | Result |
| --- | --- | --- |
| LCA-06-01 | Fresh unresolved/no-target projection | Pass in combined production projection test |
| LCA-06-02 | Useful preview-only projection | Pass in combined production projection test |
| LCA-06-03 | Useful mismatched-target projection | Pass in combined production projection test |
| LCA-06-04 | True material-watermark stale state | Pass in production/island tests |
| LCA-06-05 | Typed goal, agent output, manual boundary | Pass in live-shaped SQLite evidence test |
| LCA-06-06 | Unconfirmed geometric text | Existing abstention tests pass; no threshold changed |
| LCA-06-07 | No admissible provider semantics | Pass; 0/1 provider posts |
| LCA-06-08 | Automatic capture contention | Pass in three workload tests |

The manifest contains all eight privacy-safe causal shapes and its lint/completeness test passes. These results are component and live-shaped database proofs. They are not one eight-case replay through every Stage 6 checkpoint; that required denominator remains incomplete.

## Zero-tolerance metrics

| Metric | Numerator / denominator | Result |
| --- | ---: | --- |
| False stale classification | 0 / 3 fresh projection cases | Pass at component boundary |
| Fresh target mismatch shown as stale | 0 / 1 | Pass |
| Target failure erasing supported semantics | 0 / 2 target-failure shapes | Pass |
| Provider call with zero admissible fields | 0 / 1 | Pass |
| Manual boundary lost behind queued/active automatic capture | 0 / 2 contention shapes | Pass |
| Unsupported public semantic claim | 0 / 3 directly exercised authority-negative shapes | Pass at component boundary |
| Unsafe direct open | 0 / 4 LCA freshness/target shapes | Pass |
| React/island semantic disagreement | Unknown / no live paired render | Incomplete |
| History mutation | 0 / 19 focused history tests | Pass |
| Privacy-lint violation | 0 / 8 manifest cases | Pass |
| Deterministic full-chain replay disagreement | Unknown / full-chain replay not run | Incomplete |

## Coverage metrics

| Coverage item | Numerator / denominator | Exclusion or rate |
| --- | ---: | --- |
| Manual committed-goal attribution | 1 / 1 | 100% synthetic live-shaped case |
| Current task-turn creation/preservation | 2 / 2 | 100% same-surface positive and wrong-surface negative |
| Current agent-state attribution | 1 / 1 | 100% in three-frame evidence shape |
| Supported task admission | Unknown | Full production-chain LCA replay not run |
| Supported next-action admission | Unknown | Full production-chain LCA replay not run |
| Provider calls avoided by feasibility | 1 / 1 | 100% no-admissible case |
| Provider posts with admitted semantic field | Unknown | No live or deterministic transport executed |
| Manual boundary acquisition success | 2 / 2 | 100% deterministic queued/active arbitration shapes |
| Automatic capture timeout recovery | 2 / 2 | 100% deterministic arbitration shapes; no wall-clock soak |
| True stale detection | 1 / 1 | 100% material-watermark case |
| Fresh non-stale detection | 3 / 3 | 100% fresh projection cases |

## Automated commands and exact results

| Command | Result |
| --- | --- |
| `cargo fmt --all -- --check` | Pass |
| `cargo check` | Pass; one existing/deferred dead-field warning for `newest_evidence_ms` |
| `cargo test lca_06 --lib` | Final: 9 passed |
| `cargo test product_projection --lib` | 5 passed |
| `cargo test semantic_probe --lib` | Final broad semantic run reached 69 passed, one corrected expectation failed, one ignored; the exact corrected test then passed. The later broad Rust run included the complete semantic set green. |
| `cargo test task_turn --lib` | Final: 41 passed |
| `cargo test workload --lib` | 14 passed |
| `cargo test session_island --lib` | Final: 56 passed |
| `cargo test history --lib` | 19 passed |
| `cargo test` | One bounded attempt: 916 passed, 1 failed, 3 ignored. The one prior-only selection regression was fixed; the owning 41-test suite then passed. A second 140-second broad run was not used. |
| frontend build | Pass via bundled `pnpm run build`; 34 modules built |
| `npm run test:webview` | Wrapper could not find nested `npm`; its exact Node test payload passed 37/37 |
| `swiftc -typecheck src-tauri/macos/SessionIslandPanel.swift` | Pass with existing macOS 14 `onChange` deprecation warnings |
| accuracy evaluator `--repeat 3` | Not run to honor the bounded test budget; P6 remains closed |
| `git diff --check` | Recorded after final diff audit below |

## Live database privacy-safe observations

Not run. This implementation did not open or modify the user's live capture database. No live decision, provider, token, target, or React/native observation is claimed.

## User-owned manual results

| Scenario | Result |
| --- | --- |
| Manual 1 — explicit current goal | Not run — user-owned |
| Manual 2 — task known, direct target unavailable | Not run — user-owned |
| Manual 3 — true stale remembered answer | Not run — user-owned |
| Manual 4 — capture contention | Not run — user-owned |
| Manual 5 — thin evidence and zero post | Not run — user-owned |
| Manual 6 — React/native parity and history | Not run — user-owned |

`Not run` is not approval.

## Remaining limitations

- The eight cases are not yet one replay through every Stage 6 production checkpoint.
- The broad Rust suite was not rerun after the final focused fix; the owning focused suite passed.
- Provider transport/token denominators, live database observations, wall-clock capture contention, GUI parity, and all six user scenarios are not run.
- Feasibility prevents useless transport, but the strict response schema still retains the stable six-field envelope; ineligible fields are rejected locally rather than removed from the schema shape.
- The native `newest_evidence_ms` compatibility field remains serialized for existing callers but no longer participates in stale calculation.

## P6 release-gate status

Closed. No LCA-06 code or test changes `release_gate.passed`, supplies missing human labels, fills locked-holdout denominators, or completes manual macOS verification.

## Final verdict

IMPLEMENTATION RECOVERY APPLIED — automated LCA-06 pass is withheld because the full production-chain replay, final broad rerun, repeat-three evaluator, live database observations, and user-owned scenarios are incomplete. PRODUCT PASS and launch readiness are not claimed.

---

# Implementation record correction — 2026-07-20

This dated correction supersedes test counts and completeness claims in the 2026-07-19 record where they differ. The earlier record is retained as historical evidence rather than silently rewritten.

## Before and after behavior

The repair now reaches the normal manual decision entry point for the four semantic/provider fixture cases. Those cases use a request-scoped deterministic provider transport, persist the real `ProbeAttempt`, reload it from SQLite, and inspect the production answer and canonical product projection. The zero-admissible case reaches the same entry point, performs zero transport calls, persists zero provider posts, and carries the exact typed acquisition failure into the canonical projection.

Freshness and target readiness remain separate. Open lifecycle events no longer change the material evidence identity. Enabled privacy-policy changes do. Missing freshness identity is treated as unknown and cannot reuse or open a remembered direct target. React consumes the backend current or stale projection; if that projection is absent, it shows a protocol-level refresh state instead of composing semantic copy locally.

Manual capture recovery now validates a real post-event external frame, exact app/window identity, privacy eligibility, recency, and a complete readable JPEG or PNG asset. A failed trigger is reconciled into one captured exact-frame-reuse trigger. The reuse clock starts at the original button press, so the capture wait does not age out a frame produced during that wait.

## Corrected contradiction matrix

| Earlier contract | Runtime contradiction | Implemented correction | Current proof boundary |
| --- | --- | --- | --- |
| Target failure is field-local | Missing/mismatched ownership erased useful semantics with stale copy | Candidate attachment may suppress only the target; semantic fields and projection remain | Production projection and LCA-06 replay tests pass; live app not run |
| Stale means material evidence advanced | Counts, lifecycle events, missing hashes, and target state could affect freshness/open behavior | Material hash owns freshness; lifecycle events are diagnostic; missing hash fails closed; privacy-policy fingerprint is material | Rust watermark, gateway, React contract, and island tests pass |
| Manual Continue preserves causal typing | The boundary could lose a preceding committed user turn | Same-surface causal carry-forward is bounded to 15 seconds and requires exact attribution; geometry alone is not promoted | Live-shaped SQLite positive and negative tests pass |
| LCA-05 represented runtime provider flow | Replay started after evidence quality and provider boundaries | Four LCA-06 cases now enter manual `get_continue_decision`, use deterministic transport, persist/reload the attempt, and inspect canonical projection | Four semantic/provider cases use the unified path; all eight cases do not yet share every checkpoint |
| Explicit work outranks automatic work | An automatic screenshot could consume the manual boundary window | Manual class supersedes queued automatic screenshots, cancels/reaps active helper work, blocks requeue, and can reuse an exact recent frame | Deterministic governor, helper-process, and exact-frame tests pass; live contention not run |
| History and live surfaces show one answer | History stored reconstructed summary fields and stale UI could recompute meaning | Immutable history stores the full canonical projection; native stale state swaps to the backend stale projection | History migration/round-trip and native/React source contracts pass; paired visual test not run |
| P6 release authority is closed | Normal manual inference still needs truthful public behavior | Failure/unresolved output is typed without opening the release gate | P6 remains closed |

## Files changed and responsibility

| Responsibility | Main files |
| --- | --- |
| Manual boundary acquisition, exact-frame recovery, trigger reconciliation | `src-tauri/src/capture.rs` |
| Helper cancellation and child-process reaping | `src-tauri/src/capture/process_runner.rs` |
| Priority, supersession, cancellation, and class diagnostics | `src-tauri/src/workload.rs` |
| Causal user-turn carry-forward and legacy-schema-safe reads | `src-tauri/src/continuation/task_turn_evidence.rs`, `task_turn.rs` |
| Material evidence identity and privacy-policy invalidation | `src-tauri/src/continuation.rs` |
| Admission preflight, deterministic provider seam, persisted attempt | `task_truth_v2/semantic_probe.rs`, `production.rs` |
| Unified fixture replay and schema-compatible older replay | `accuracy_eval.rs`, `accuracy_fixture.rs`, LCA fixture manifests |
| Canonical current/stale projection, direct-open policy, fail-closed UI | `src/continuePresentation.ts`, `src/App.tsx`, `session_island/contract.rs`, `session_island/gateway.rs`, `session_island.rs` |
| Immutable canonical output history | `src-tauri/src/continuation/history.rs`, native island history wiring |
| Corrected product/runtime doctrine | `PRODUCT.md`, `docs/full-engine-flow.md`, `docs/current-island-ui-ux.md`, affected completion audits |

## Freshness versus target state

| Freshness proof | Target proof | Result |
| --- | --- | --- |
| Current material hash | Exact validated direct owner and canonical `open_direct_target` action | Preserve instruction; direct open allowed |
| Current material hash | Preview only | Preserve instruction; inspect/view-only action |
| Current material hash | Missing or mismatched owner | Preserve instruction; target unavailable/suppressed; no open |
| Material hash advanced | Any remembered target | Backend stale projection; refresh; no open |
| Material hash missing | Any remembered target | Freshness unknown; refresh; no open |
| Semantic/provider/capture failure | No proven direct target | Exact typed failure; never converted to stale copy |

## Task-evidence continuity proof

The live-shaped three-frame case carries only a causally attributed committed user turn across an unchanged manual surface. It keeps the later agent state distinct and selects the current task turn. Wrong-surface, prior-only, private, corrupt-asset, pre-event, and over-age cases fail closed. An exact-frame reuse trigger is recognized as the manual boundary without requiring a second contradictory trigger row. Minimal legacy test schemas remain readable when the newer trigger metadata column is absent.

## Admission feasibility and provider-cost proof

Preflight runs before credentials and transport. The zero-admissible manual case records `provider_post_count = 0`, invokes transport zero times, and produces `task_evidence_acquisition_failure:no_admissible_semantic_fields`. Admissible requests without credentials remain a distinct provider-unavailable failure. The deterministic fixture transport records synthetic request/response identity and token usage but does not contact a provider, so no live cost claim is made.

## Workload priority and timeout proof

`ManualBoundaryCapture` receives user priority. It supersedes queued automatic screenshots, links cancellation into an active helper process, kills and reaps that child, prevents automatic requeue ahead of the boundary, and separates cancelled/superseded/failed diagnostics by workload class. Exact-frame recovery provides a bounded fallback after a capture failure. These are deterministic service and process-lifecycle proofs, not a live wall-clock or soak proof.

## LCA-06 fixture results

| Case | Result | Execution depth |
| --- | --- | --- |
| LCA-06-01 fresh unresolved/no target | Pass | Manual decision entry, persisted attempt, production answer/projection |
| LCA-06-02 useful preview only | Pass | Manual decision entry, persisted attempt, production answer/projection |
| LCA-06-03 mismatched target | Pass | Manual decision entry, persisted attempt, production answer/projection |
| LCA-06-04 genuinely stale remembered answer | Pass | Material hash, backend stale projection, gateway/native/React receiving tests |
| LCA-06-05 typed goal then boundary | Pass | Live-shaped SQLite task-evidence path |
| LCA-06-06 unconfirmed geometry | Pass | Authority-negative evidence tests; no threshold lowering |
| LCA-06-07 no admissible provider request | Pass | Manual decision entry; 0 transport invocations and 0 persisted posts |
| LCA-06-08 capture contention | Pass | Governor, helper cancellation/reaping, trigger reconciliation, exact-frame fallback |

All eight privacy-safe shapes are present and linted. Cases 01, 02, 03, and 07 use the unified manual/provider/persistence/product chain. The remaining acquisition, stale receiving, and contention cases use their owning production components. Therefore the stricter requirement that all eight traverse every Stage 6 checkpoint is still incomplete.

## Zero-tolerance and coverage metrics

| Metric | Deterministic result |
| --- | ---: |
| Fresh target failure shown as stale | 0 / 3 |
| Target failure erases supported semantics | 0 / 2 |
| Provider transport with zero admissible fields | 0 / 1 |
| Persisted provider posts with zero admissible fields | 0 / 1 |
| Unsafe direct open across fresh/stale/unknown target shapes | 0 / 5 |
| Privacy-lint violations | 0 / 8 |
| History mutation in focused round-trip suite | 0 / 11 tests |
| Manual scheduler ordering violations | 0 / 2 contention shapes |
| Live React/native disagreement | Unknown — paired visual run not performed |
| Live manual acquisition success | Unknown — user-owned normal-app run not performed |
| Live automatic timeout recovery | Unknown — no soak or wall-clock run |

## Automated commands and exact results

| Command | Result |
| --- | --- |
| `cargo fmt --all -- --check` | Pass |
| `cargo check` | Pass; one existing/deferred `newest_evidence_ms` dead-field warning |
| `cargo test lca_06 --lib` | 11 passed |
| `cargo test manual_continue_ --lib` | 14 passed |
| `cargo test workload --lib` | 15 passed |
| `cargo test continuation::history::tests --lib` | 11 passed |
| `cargo test session_island --lib` | 57 passed |
| `cargo test product_projection --lib` | 5 passed |
| `cargo test material_evidence_watermark --lib` | 2 passed |
| `cargo test continuation::tests --lib` | 192 passed |
| `cargo test continuation::task_truth_v2::tests --lib` | 11 passed, 1 ignored live-provider test |
| `cargo test continuation::accuracy_eval::tests --lib -- --test-threads=1` | 8 passed |
| presentation contract script | 34 passed |
| TypeScript `tsc -b` | Pass |
| Vite production build | Pass; 34 modules transformed |
| `swiftc -typecheck src-tauri/macos/SessionIslandPanel.swift` | Pass with five existing macOS 14 `onChange` deprecation warnings |
| first bounded `cargo test` attempt | 867 passed, 57 failed, 3 ignored; all failures traced to two legacy/minimal schema reads introduced by LCA-06 |
| post-fix owning suites | Pass as listed above; a second full 927-test run was intentionally not spent |
| repeat-three accuracy evaluator | Not run; P6 remains closed |

One concurrent multi-process accuracy-suite run produced `current_frame_model_ineligible`; the same full eight-test suite passed serially. Production requests are request-scoped, but deterministic accuracy evaluation remains safest with one test thread because fixtures share process-global test state.

## Live database and user-owned manual results

No live capture database or provider was read or modified for this correction. All six normal-app scenarios remain `Not run — user-owned`. No live, visual, provider-cost, soak, product-pass, or launch claim is made.

## Remaining limitations

- Four non-provider cases do not yet traverse the exact same unified Stage 6 checkpoint chain as the four semantic/provider cases.
- The full 927-test suite was not repeated after fixing the two schema-compatibility causes; the owning suites covering every observed failure family passed.
- Accuracy replay is verified serially because concurrent test processes can interfere through shared fixture state.
- Live provider/token denominators, live database observations, capture wall-clock behavior, GUI parity, and the six user scenarios remain unverified.
- The stable provider response schema still contains the six known fields; preflight prevents transport when none can be admitted, and local validation rejects unsupported fields independently.

## P6 release-gate status

Closed. LCA-06 does not supply missing human labels, unlock holdouts, complete repeat-three evaluation, or perform manual macOS verification.

## Final verdict

IMPLEMENTATION RECOVERY APPLIED — focused and owning deterministic suites pass after the schema-compatibility fixes. The stricter `AUTOMATED PASS` is withheld because all eight cases do not traverse every Stage 6 checkpoint, the repeat-three evaluator and post-fix full-suite rerun were not performed, and live/user-owned verification remains outstanding. `PRODUCT PASS` and launch readiness are not claimed.

---

# Live attribution and false-stale correction — 2026-07-20

This correction records the two production bugs found after the earlier LCA-06 recovery. It supersedes the earlier eight-case count and the earlier statement that every typed failure was already preserved after a material watermark change.

## Root causes and fixes

1. Helium/ChatGPT role attribution could treat right-aligned browser text as a possible user turn, but it had no structural authorship proof. The live frame showed why: `AXWebArea` landed exactly at the Accessibility helper’s old depth-eight limit, so its reported child was never traversed and the stored tree contained browser chrome but no page message groups. The helper now permits eight additional levels only after entering `AXWebArea`, while preserving the global 450-node cap. The task-turn repair uses that web area as the browser-content boundary, associates eligible chat content with a bounded Accessibility message group, requires exact active-window ownership, and excludes browser chrome before role classification. A structurally proven user message receives bounded `0.72` confidence. Geometry alone remains `0.58`, so `current_task_turn` remains absent and provider preflight makes zero requests when Helium still exposes no structural authorship proof.
2. Evidence-watermark changes were used both to invalidate reuse and to rewrite presentation meaning. React and the native island therefore replaced a saved acquisition failure with the semantic stale sentence. The repair keeps invalidation and open blocking, but the backend now precomputes a failure-preserving stale projection for task-acquisition, provider, parser, validation, and capture failures. Only action-known or task-known semantic answers receive `stale_decision` copy.

The manual-boundary bridge can carry only a previously structurally proven chat role. It requires the same session and surface, an exact matching window, a still-visible stable text hash, privacy eligibility, no intervening task-bearing surface, and age at or below 15 seconds. Browser-search typing and LinkedIn detours are not task authority.

## Added regression cases

| Case | Deterministic result |
| --- | --- |
| LCA-06-09 Helium chat turn with structural proof | Selected user turn; admission feasible; exactly 1 injected provider transport call |
| LCA-06-10 right-aligned chat-like text without structural proof | Confidence remains 0.58; no selected user turn; provider preflight remains closed |
| LCA-06-11 typed acquisition failure followed by watermark advance | Cache/open identity invalidated; typed failure copy and reason preserved; stale sentence absent |

The privacy-safe manifest now contains 11 cases and its completeness/lint test passes.

## Verification results

| Command | Result |
| --- | --- |
| `cargo test lca_06 --lib` | Pass: 12 passed |
| `cargo test task_turn_evidence --lib` | Pass: 18 passed |
| `cargo test product_projection --lib` | Pass: 5 passed |
| `cargo test session_island --lib -- --test-threads=1` | Pass: 58 passed |
| `cargo check` | Pass; existing `newest_evidence_ms` dead-field warning remains |
| `cargo test -- --test-threads=1` | Pass: 931 passed, 3 ignored |
| frontend production build | Pass: TypeScript and Vite, 34 modules transformed |
| webview presentation tests | Pass: 38 passed |
| `swiftc -typecheck src-tauri/macos/SessionIslandPanel.swift` | Pass; five existing macOS 14 `onChange` deprecation warnings |
| `swiftc -module-cache-path /tmp/smalltalk-swift-module-cache -typecheck src-tauri/scripts/accessibility_snapshot.swift` | Pass |
| `git diff --check` | Pass |

## Approval boundary

No live provider request was made for this correction. The one-post proof uses the request-scoped deterministic transport. The normal-app Helium/ChatGPT run, zero-post unprovable-surface run, app-switch failure-persistence check, and paired visual approval remain user-owned. P6 release authority remains closed; no live approval, product pass, or launch readiness is claimed.
