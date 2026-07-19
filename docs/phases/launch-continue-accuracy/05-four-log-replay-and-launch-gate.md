# LCA-05 — Four-Log Replay, Non-Regression, And User-Owned Launch Gate

## Codex implementation task

Prove that the complete normal manual `Continue` chain fixes the four supplied accuracy failures and cannot regress silently. Convert their privacy-safe causal shapes into permanent production-path replay fixtures, add adversarial cases for the recurring failure classes, run the full deterministic verification matrix, and prepare the exact manual test handoff for the user.

This is the fifth and final phase of the Launch Continue Accuracy repair. It is a proof and hardening phase. Do not add another semantic architecture, lower a threshold, weaken a fixture, or change expected truth to make the gate pass.

Codex must not perform the manual test. The automated verdict may say that the implementation is ready for user-owned live verification. It must not claim that the app is visually approved, live-provider approved, or launch-ready until the user supplies those results.

## What “fixed once and for all” means

This phase does not promise perfect task understanding for every possible application. It must make the specific recurring failure classes permanent, measurable non-regressions:

1. A busy or text-heavy unrelated pane cannot become the primary task.
2. Completed work cannot remain the active headline after a newer task or verification step exists.
3. A useful partial semantic answer cannot be erased into a false insufficient-evidence state by category-level validation.
4. An uncertain or qualified answer cannot be upgraded to resolved or explicit truth.
5. The public product cannot dump diagnostic fields and force the user to infer the action.
6. React and the native island cannot disagree about task meaning, state, action, or target policy.
7. Missing direct target cannot erase a useful instruction, and a useful instruction cannot invent a target.

Future changes must fail deterministic replay if they reintroduce one of these failures.

## Hard dependency gate

Read all five prompts and the first four completion audits:

```text
docs/phases/launch-continue-accuracy/01-task-relevant-evidence-packet.md
docs/phases/launch-continue-accuracy/01-task-relevant-evidence-packet-completion-audit.md
docs/phases/launch-continue-accuracy/02-actionable-continuation-contract.md
docs/phases/launch-continue-accuracy/02-actionable-continuation-contract-completion-audit.md
docs/phases/launch-continue-accuracy/03-truthful-admission-and-authority.md
docs/phases/launch-continue-accuracy/03-truthful-admission-and-authority-completion-audit.md
docs/phases/launch-continue-accuracy/04-one-instruction-ui-parity.md
docs/phases/launch-continue-accuracy/04-one-instruction-ui-parity-completion-audit.md
docs/phases/launch-continue-accuracy/05-four-log-replay-and-launch-gate.md
```

Continue only when the first four audits end exactly with:

```text
PASS — LCA-01 evidence packet is proven and LCA-02 may begin
PASS — LCA-02 actionable contract is proven and LCA-03 may begin
PASS — LCA-03 admission and authority are proven and LCA-04 may begin
PASS — LCA-04 product answer and parity are proven and LCA-05 may begin
```

Independently audit every acceptance criterion. If an earlier phase is incomplete or contradicted, repair the issue under that phase's contract, update its audit honestly, rerun its verification, and only then continue. Do not paper over an earlier failure in the final evaluator.

## Hard operating rules

- Read `AGENTS.md` and obey it.
- Preserve the current worktree and unrelated changes.
- Do not use Computer Use.
- Do not use AppleScript UI automation, Chrome control, the in-app browser, screenshots of the running app, mouse clicks, keyboard automation, or any equivalent GUI automation.
- Do not run the user-owned manual scenarios.
- Do not claim visual, native-island, interaction, or live-provider acceptance.
- Do not invent manual observations or replace `Not run` with a deterministic test result.
- Deterministic replay, unit/integration tests, builds, Rust checks, privacy lint, performance checks, and Swift type-checking are allowed.
- Do not make new external provider calls for the replay gate. Use deterministic response fixtures and the supplied completed logs as diagnostic sources.
- Do not commit raw logs into a second location, screenshots, local databases, raw URLs, raw paths, private conversation text, API payloads, or credentials.
- Do not modify the four supplied raw logs.
- Do not lower P6 release thresholds or change the existing P6 release verdict.
- Keep one manual provider request, zero reconciliation calls, zero HTTP retries, and zero background semantic uploads.
- Do not weaken privacy, target ownership, strict open, correction scope, stale-decision blocking, or cache identity.
- Do not use compile success as proof of semantic correctness.
- Do not call LCA complete while a critical replay failure remains.

## Required reading

Read current versions of:

```text
AGENTS.md
PRODUCT.md
docs/full-engine-flow.md
docs/current-island-ui-ux.md
docs/phases/p6-task-turn-accuracy/p6-00-task-turn-accuracy-program.md
docs/phases/p6-task-turn-accuracy/p6-01-ground-truth-replay-eval.md
docs/phases/p6-task-turn-accuracy/p6-08-target-truth-and-answer-composition.md
docs/phases/p6-task-turn-accuracy/p6-09-longitudinal-release-gate.md
docs/phases/p6-task-turn-accuracy/p6-09-completion-audit.md
docs/phases/lean-continue/01-compact-semantic-episode-input-completion-audit.md
docs/phases/lean-continue/03-single-request-production-cutover-completion-audit.md
```

Inspect current replay, fixture, privacy, production, presentation, and audit paths:

```text
src-tauri/src/bin/continue_accuracy_eval.rs
src-tauri/src/continuation/accuracy_eval.rs
src-tauri/src/continuation/accuracy_fixture.rs
src-tauri/src/continuation/task_truth_v2/observation_packet.rs
src-tauri/src/continuation/task_truth_v2/semantic_probe.rs
src-tauri/src/continuation/task_truth_v2/production.rs
src-tauri/src/continuation/task_truth_v2/verifier.rs
src-tauri/src/continuation/confidence.rs
src-tauri/src/continuation/task_turn*.rs
src-tauri/src/session_island/contract.rs
src-tauri/src/continuation/history.rs
src-tauri/macos/SessionIslandPanel.swift
src/App.tsx
src/continuePresentation.ts or its current equivalent
src-tauri/tests/fixtures/continue_accuracy/
```

Use the existing versioned accuracy harness. Extend it with LCA checkpoints and fixtures instead of creating a disconnected script whose transformations differ from production.

## Source logs

The diagnostic source logs are:

```text
resp_05cd15c928f2b22f006a5bf31d506481a0987b22d9e16500b9-full-log.json
resp_0d1ccaabe671fd37006a5bf51ade34819ebe5fa0f01a250b8e-full-log.json
resp_0056a4be4033902f006a5bfb214a50819ca241ce0c2a3ec577-full-log.json
resp_0e34cbea9d0858af006a5bfb615f7c81929e12bbc2bf4b5197-full-log.json
```

Inspect them locally. Do not copy raw conversation content or image material into committed fixtures. Create bounded, privacy-reviewed synthetic evidence that preserves:

- chronological boundaries;
- pane/region roles;
- user/agent attribution;
- completed versus current task relationship;
- task state;
- support-slot shape;
- raw model status and field-support patterns;
- expected public meaning.

Record the source log only through a safe case label and optional hash. Do not commit private absolute capture paths.

## Required two-boundary replay design

The final harness must test both places where the real failures occurred.

### Boundary A — evidence-to-request replay

Replay privacy-safe captured/structured facts through production evidence transformations up to the exact provider request.

Assert:

```text
selected task-turn evidence
selected/rejected surfaces and regions
selected image roles
completed prior context
current unfinished task evidence
support/detour relationship
near-duplicate handling
image preparation and dimensions
structured request facts
one-request accounting
privacy/cutoff/ownership checks
```

This boundary proves that the model is given the right problem.

### Boundary B — response-to-product replay

Feed deterministic provider-response variants through the real parser, support validator, field admission, production mapping, canonical presentation, React state, and native-island contract.

Assert:

```text
raw status
field admission
admitted semantic status
semantic source kind
task state
unfinished task
resume point
next supported action
completed context
target status
first-screen instruction
resume-context line
primary action
React/native parity
history copy and atomic identity
```

This boundary proves that the product does not corrupt a useful model answer after it is returned.

Do not claim Boundary A proves actual model interpretation. Do not claim Boundary B proves visual perception. The user-owned live test covers their combined real-provider behavior.

## Required critical fixtures

Create four versioned privacy-safe critical fixtures with stable meanings.

### LCA-CRIT-01 — product-need answer review

Expected:

```text
unfinished task:
Review the answer assessing whether the proposed context-reconstruction product solves a real need.

task state:
active

resume point:
The visible answer has begun and continues.

next action:
Continue reviewing the answer from the product-need section.

target state:
task known, direct target unavailable unless a real locator is independently supplied.
```

Forbidden:

- generic “reviewing output” as the task;
- declaring the review completed;
- invented product work or target.

### LCA-CRIT-02 — new visual-cue task after backend completion

Expected:

```text
unfinished task:
Add or inspect the answer-linked visual cue in the real island output.

completed context:
Backend connection/output flow completed.

launch checklist:
supporting or detour, not primary.

next action:
Return to the Codex visual-cue task and inspect its result.
```

Forbidden:

- compound headline that presents backend connection as unfinished;
- launch checklist as primary task;
- local upgrade beyond the evidence.

### LCA-CRIT-03 — implementation complete, user verification remains

Expected:

```text
unfinished task:
Verify the completed answer-linked visual cue in Smalltalk.

task state:
needs_user_verification

resume point:
Implementation completed and focused checks passed.

next action:
Open the latest Continue answer and verify that Show more reveals the linked cue.

PFTU discussion:
supporting or unrelated, never primary.
```

Forbidden:

- “implement the visual cue” as the unfinished task;
- false insufficient-evidence result;
- PFTU release gate in first-screen copy;
- inline support-slot citation text in public strings.

### LCA-CRIT-04 — unsent regression draft

Expected:

```text
unfinished task:
Investigate why the latest Continue result was rejected as insufficient evidence.

task state:
active

resume point:
An unsent regression report is being drafted.

next action:
Return to the draft and continue the report.

causal status:
The visual-cue work preceded the failure; causation is unproven.
```

Forbidden:

- claiming the draft was submitted;
- instructing a destructive or unsupported fix;
- asserting that the visual-cue change caused the regression;
- local upgrade to resolved when the causal claim remains qualified.

## Required adversarial fixtures

Add at least these four privacy-safe cases so the solution is not overfit to exact wording:

### LCA-ADV-01 — high-engagement unrelated pane

A long, heavily interacted-with adjacent chat appears beside a short current user request. The short attributed request must remain primary.

### LCA-ADV-02 — old completion beside new task

An old assistant result says tests passed. A newer user request starts another task in the same conversation. The completion belongs only to the prior task.

### LCA-ADV-03 — task known, action unsupported

The unfinished task is clear but the evidence does not justify a next step. Preserve the task, use the task-known/action-unknown state, and do not fabricate an action.

### LCA-ADV-04 — genuinely thin evidence

Only an application or passive generic page is visible. The correct outcome is semantic unresolved, with no target and no generic task invention.

Where practical, include existing P6 Capture-button contamination fixtures in the same report rather than duplicating them.

## Deterministic provider-response variants

For every critical case, exercise:

```text
valid resolved-shaped response
valid partly-resolved response
one unsupported semantic field
wrong-task response citing a real slot
prior-completion response
generic next-action response
confidence-inflating response
invalid inline citation text
invalid structured output
provider incomplete/empty status
```

Required behavior:

- valid supported fields survive;
- wrong or generic fields are removed locally without replacement prose;
- raw uncertainty is never upgraded;
- one rejected field does not erase the whole answer;
- a wrong-task response cannot become authoritative merely because its citation is mechanically valid;
- provider/parse failure is not called insufficient evidence;
- target policy never changes because a response is fluent or confident;
- React/native copy remains identical in meaning.

## Required launch-accuracy metrics

Report numerator, denominator, excluded count, and rate. The automated LCA gate requires:

```text
critical evidence-checkpoint pass = 4/4
critical semantic-slot pass = 4/4
critical first-screen meaning pass = 4/4
completed-work-as-active-task count = 0
unrelated-pane primary leakage count = 0
false insufficient-evidence count = 0 on recoverable critical cases
uncertainty upgrade count = 0
inferred-as-explicit count = 0
unsupported next-action count = 0
supported next-action precision = 100% on the critical fixtures
status/action contradiction count = 0
inline diagnostic/citation leakage count = 0
React/island semantic disagreement count = 0
frame-preview-as-direct-target count = 0
unsafe open or open-policy bypass count = 0
one-manual-attempt provider-post maximum = 1
automatic reconciliation post count = 0
background semantic upload count = 0
privacy fixture violation count = 0
deterministic replay disagreement count = 0
```

Do not define a denominator to exclude the difficult case. An unknown prediction is incorrect unless the fixture expects abstention.

This gate does not replace the existing P6 100-case release gate. Report both verdicts separately.

## Full-path non-regression requirements

Audit and prove:

### Input and cost

- At most two boundaries and four prepared images.
- No duplicate semantic images.
- Structured text remains within the existing compact limit.
- Original and sent image dimensions are audited.
- High-resolution mixed-pane images are cropped or explicitly region-labeled when ownership is reliable.
- One provider post per new manual boundary.
- Unchanged evidence reuses the existing atomic answer without a post.

### Semantic truth

- Newest unfinished task outranks project/workstream summaries.
- Prior completion remains prior.
- Verification becomes the task after implementation completes.
- Draft remains unsent.
- Causal hypotheses remain qualified.
- P6 and compact task disagreement cannot be silently merged.

### Admission

- Field-local validation remains field-local.
- Raw status cannot be upgraded.
- Inferred goals remain inferred.
- Typed provider/parse/validation/semantic/target outcomes remain separate.

### Product answer

- One instruction leads.
- One resume-context sentence follows.
- Diagnostics remain behind Inspect.
- Missing target does not erase useful text.
- Missing action does not create fake text.
- React and island agree.

### Target and history safety

- Strict direct open remains decision-id and locator-policy bound.
- Frame evidence remains preview/inspect only.
- Stale answer cannot open.
- Saved output history preserves the exact displayed copy and answer identity.

### Existing product behavior

- Continue remains the first-screen primitive.
- Capture privacy and current session boundaries remain intact.
- Answer-linked visual cue remains linked to the answer evidence.
- Read-only output history remains one-panel and non-authoritative.
- No browser-extension or legacy public semantic fallback is introduced.

## Required automated verification

Run the current exact equivalents of:

```bash
cd src-tauri
cargo fmt --all -- --check
cargo check
cargo test
cd ..
npm run build
npm run test:webview
swiftc -typecheck src-tauri/macos/SessionIslandPanel.swift
git diff --check
git status --short
```

Also run:

- the versioned P6 accuracy evaluator;
- the new LCA critical/adversarial replay gate through the same evaluator or a versioned extension;
- fixture privacy lint;
- one-request/cache accounting tests;
- strict-open/no-bypass tests;
- React/native parity tests;
- deterministic performance and request-size checks available in the repository.

Use the repository's real Swift type-check invocation and companion sources when required. Report exact commands, counts, ignored tests, elapsed time, and failures. A filter that ran zero tests is not a pass.

## Documentation truth

Update `PRODUCT.md`, `docs/full-engine-flow.md`, and `docs/current-island-ui-ux.md` only where current implementation behavior materially changed.

Document:

- task-relevant compact evidence selection;
- actionable continuation schema;
- monotonic admission and separate target status;
- one-instruction presentation;
- React/native parity;
- one-request behavior;
- current limitations;
- user-owned manual gate;
- unchanged P6 release verdict.

Do not describe intended behavior as implemented unless code and replay prove it.

## User-owned manual verification script

Create the final manual script inside the completion audit. Do not run it. Every row must initially say:

```text
Not run — user-owned after automated LCA completion
```

The script must use the normal product path, not an evaluation-only database:

```bash
npm run tauri dev
```

The user should test at least these scenarios in order.

### Manual 1 — response review

1. Open a conversational answer long enough that part of the answer remains below the current view.
2. Press the real Smalltalk Continue once.
3. Expected: one instruction to continue reviewing that answer, plus the exact visible stopping context.
4. Forbidden: broad product/project recap, generic reviewing label, invented target.

### Manual 2 — new task after completion

1. Complete one Codex task and leave its success output visible.
2. Enter a newer request in the same conversation.
3. Press Continue while the newer task is active or awaiting a result.
4. Expected: the newer request is primary; old completion is short prior context.
5. Forbidden: compound headline treating both as unfinished.

### Manual 3 — implementation complete, verification remains

1. Have Codex complete a change and explicitly state that user verification remains.
2. Keep an unrelated or supporting chat visible in another pane.
3. Press Continue.
4. Expected: the verification step is primary and the other pane does not enter first-screen copy.
5. Forbidden: implementation presented as unfinished, PFTU/support topic leakage, insufficient-evidence false abstention.

### Manual 4 — unsent draft

1. Type a regression or correction draft without submitting it.
2. Press Continue.
3. Expected: return to/continue the draft; the draft is described as unsent.
4. Forbidden: claim that it was sent, claim that an unproven cause is fact, invented technical fix.

### Manual 5 — genuinely thin evidence

1. Use a passive surface with no visible concrete task.
2. Press Continue.
3. Expected: honest task-unrecoverable copy.
4. Forbidden: generic browsing/viewing task or a stale previous task.

### Manual 6 — React/native and target parity

For each prior result:

1. Compare the main app and native island.
2. Expand Show more.
3. Inspect the visual cue.
4. Check output history.
5. Exercise Continue here only when a real direct target exists.
6. Expected: same instruction, state, target policy, and action meaning everywhere.
7. Forbidden: island role shown as unclear when React says primary; frame preview labeled as direct return; history recomputing new semantics.

For every scenario, ask the user to record privately:

```text
expected unfinished task
observed first instruction
observed resume context
observed island copy
observed target/action
provider full-log filename
Correct, Partly right, or Wrong
one-line correction
```

Do not require the user to commit screenshots, captures, provider payloads, or private IDs.

## Manual result policy

After the user performs the script, update only the manual-results section of the audit with their supplied observations and a privacy-safe summary.

The final product launch verdict requires:

- all six manual scenarios run;
- recoverable cases rated Correct or Partly right with no confidently wrong primary task;
- no false insufficient-evidence result on Manual 3;
- no unrelated-pane leakage;
- no unsupported next action;
- no React/native semantic disagreement;
- no unsafe target action;
- no regression in visual cue or history behavior.

If a scenario fails, identify the first divergent checkpoint and repair the owning LCA phase. Do not patch only the visible copy unless the backend canonical answer was already correct.

## Required completion audit

Create:

```text
docs/phases/launch-continue-accuracy/05-four-log-replay-and-launch-gate-completion-audit.md
```

Include:

1. Requirement matrix for LCA-01 through LCA-04.
2. Exact critical and adversarial fixture inventory.
3. Boundary A evidence-to-request results.
4. Boundary B response-to-product results.
5. Every LCA metric with numerator, denominator, exclusions, and rate.
6. Four critical first-screen answers as text.
7. One-request, cache, privacy, cost, and performance results.
8. React/native parity and strict-open results.
9. Existing P6 release verdict without alteration.
10. Exact automated commands and results.
11. Documentation changes.
12. The complete user-owned manual script.
13. A manual-results table with every scenario initially marked not run.
14. Remaining limitations and prohibited launch claims.

Before user testing, end with exactly one automated verdict:

```text
AUTOMATED PASS — LCA implementation is ready for user-owned live verification; launch is not yet manually approved
```

or:

```text
INCOMPLETE — LCA automated repair or replay gate remains open; user-owned live verification is blocked
```

After the user supplies all manual results, append exactly one manual verdict without deleting the automated verdict:

```text
USER VERIFIED — LCA launch scenarios passed
```

or:

```text
USER FOUND FAILURES — LCA launch remains blocked and the owning phase must be repaired
```

Do not write `USER VERIFIED` based on automated tests, Codex observation, CUA, or inferred user behavior.

## Final response format

Report:

1. LCA requirement-matrix verdict.
2. Critical and adversarial replay results.
3. Every zero-tolerance count.
4. Automated commands and results.
5. P6 release verdict separately.
6. LCA automated verdict.
7. Exact user-owned manual script location and next action.
8. Remaining limitations and launch claims that are still prohibited.

