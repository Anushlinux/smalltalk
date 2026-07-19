# LCA-02 — Actionable Continuation Output Contract

## Codex implementation task

Replace the live compact model's retrospective observation contract with one atomic, evidence-backed continuation contract. The normal manual `Continue` result must identify the newest unfinished task, the exact state where work stopped, and one next action that is directly supported by evidence. Completed work must remain context instead of becoming the active headline.

This is the second phase of the Launch Continue Accuracy repair. Implement it completely only after LCA-01 has passed.

This phase changes the provider response schema, prompt, parsing, compact-result representation, and backend public-answer mapping. It must not redesign the visible React/native layout; LCA-04 owns presentation. It must not add a second provider request or invent a direct return target.

## Hard dependency gate

Read:

```text
docs/phases/launch-continue-accuracy/01-task-relevant-evidence-packet.md
docs/phases/launch-continue-accuracy/01-task-relevant-evidence-packet-completion-audit.md
```

Stop if the audit does not end exactly with:

```text
PASS — LCA-01 evidence packet is proven and LCA-02 may begin
```

Independently confirm that the normal compact request uses the new task-relevant selection policy and the four critical fixture shapes pass at the evidence checkpoint. Do not trust the verdict without checking code and tests.

## Hard operating rules

- Read `AGENTS.md` and obey it.
- Inspect and preserve the current worktree.
- Do not use Computer Use, AppleScript UI automation, Chrome control, the in-app browser, screenshots of the running app, mouse clicks, or keyboard automation.
- Do not run or claim manual visual QA. The user owns live testing after LCA-05.
- Deterministic tests, replay, build, Rust checks, and Swift type-checking are allowed.
- Keep one provider request, zero reconciliation requests, and zero automatic retries.
- Do not send broad raw history to the model.
- Do not add raw typed characters, clipboard text, private paths, raw URLs, or captures to persistence or fixtures.
- Do not use local activity labels, app names, titles, workstreams, or legacy recap prose to fill missing semantic fields.
- Do not make `Stop Session` a prerequisite for `Continue`.
- Do not create a second public answer engine.
- Do not weaken direct-target ownership or open-time validation.
- Do not claim full launch accuracy from deterministic fixtures alone.

## Required reading

Read current versions of:

```text
AGENTS.md
PRODUCT.md
docs/phases/proof-first-task-understanding/pftu-02-truthful-production-answer.md
docs/phases/p6-task-turn-accuracy/p6-00-task-turn-accuracy-program.md
docs/phases/p6-task-turn-accuracy/p6-03-current-task-turn-lifecycle.md
docs/phases/p6-task-turn-accuracy/p6-06-confidence-and-observation-reliability.md
docs/phases/p6-task-turn-accuracy/p6-07-recap-truth-pack-and-validation.md
docs/phases/p6-task-turn-accuracy/p6-08-target-truth-and-answer-composition.md
docs/phases/p6-task-turn-accuracy/p6-09-completion-audit.md
docs/phases/lean-continue/03-single-request-production-cutover-completion-audit.md
```

Inspect current implementations and tests for:

```text
src-tauri/src/continuation/task_truth_v2/semantic_probe.rs
src-tauri/src/continuation/task_truth_v2/production.rs
src-tauri/src/continuation/task_truth_v2/task_snapshot.rs
src-tauri/src/continuation/task_truth_v2/task_thread.rs
src-tauri/src/continuation/task_truth_v2/verifier.rs
src-tauri/src/continuation/task_turn.rs
src-tauri/src/continuation/confidence.rs
src-tauri/src/continuation/accuracy_eval.rs
src/App.tsx
src-tauri/src/session_island/contract.rs
```

Reuse existing `CurrentTaskTurn`, `TaskSnapshotV2`, confidence, evidence support, atomic identity, correction, cache, and target contracts where they are correct. Do not recreate them inside the compact probe.

## Current failure to remove

The current compact response asks for:

```text
primary_task
current_step
last_progress
unfinished_state
visit_roles
```

The prompt forbids inventing next actions, and the compact production mapper leaves `next_action`, `where_summary`, and `direct_return_target` empty. The visible product must then reinterpret descriptive fields as though they were a continuation instruction.

This creates four failures:

1. `primary_task` can remain a broad workstream after that task has completed.
2. `current_step` becomes a screen description rather than an exact resume point.
3. `unfinished_state` names a condition such as “user testing remains” but not the action the user should take.
4. The user must combine several overlapping fields to decide what to do.

LCA-02 must remove that mismatch at the model contract rather than hiding it with UI wording.

## Required atomic semantic contract

Version the compact provider response and public answer only where the live shape is genuinely incompatible. Use repository-consistent names. The authoritative semantic result must include the following meanings.

### `unfinished_task`

The newest incomplete user-owned or agent-served objective.

Rules:

- Name the exact task object and intended outcome.
- Do not use a broad project or workstream when a newer child task exists.
- Do not name completed implementation as active work when only verification remains.
- Do not name supporting research, an adjacent pane, or a temporary detour as primary.
- Return null when evidence cannot establish a concrete unfinished objective.
- Maximum public-safe length: 220 characters unless the current public contract has a smaller proven cap.

### `task_state`

Use a stable enum:

```text
active
waiting_for_result
needs_user_verification
blocked
superseded
completed
unclear
```

Map to existing P6 execution/current-actor/waiting-on axes without collapsing those internal distinctions. If the compact provider sees enough evidence to infer a state, it must cite that evidence. Local P6 truth may bound or reject the claim but must not silently replace it with unrelated semantics.

### `resume_point`

The exact meaningful state where the unfinished task stopped.

Good resume points include:

- the answer section currently being reviewed;
- the implementation result awaiting inspection;
- checks passed and user verification remaining;
- an unsent draft awaiting continued editing;
- an agent still working and the user waiting for its result.

Bad resume points include:

- the app name alone;
- a generic activity such as browsing, reviewing, or editing;
- a broad recap of the entire project;
- the current screen without its relationship to the task.

Maximum public-safe length: 260 characters unless a smaller current cap is proven.

### `next_supported_action`

One concise next action directly supported by the evidence.

Allowed support classes:

- an explicitly unfinished user request;
- a visible partial result that can be continued or reviewed;
- an explicit user-verification requirement;
- an agent/tool state that the user must wait for or inspect;
- an unsent draft that can be resumed;
- an explicit error or blocker with a supported recovery action;
- a safe return/inspect action already validated by the local target policy.

Prohibited actions:

- speculative implementation steps not visible in the evidence;
- sending an unsent message when the evidence proves only that the draft exists;
- destructive operations;
- fabricated files, URLs, identifiers, or commands;
- “Continue working,” “Keep browsing,” “Review it,” or other generic filler;
- an action that contradicts `task_state`;
- an action derived only from a workstream label, app name, page title, or local activity class.

When evidence supports a draft but not submission, say “Return to the draft” or “Continue editing the draft,” not “Send it.”

When implementation is complete and verification remains, the action must be verification, not implementation.

Return null when no safe, concrete action is supported.

Maximum public-safe length: 180 characters unless a smaller current cap is proven.

### `completed_context`

A short statement of the immediately relevant work that has already completed and must not become the active task.

Rules:

- Include it only when needed to distinguish prior completion from current work.
- Do not turn it into a second headline.
- Do not include unrelated historical project summaries.
- Maximum public-safe length: 180 characters.

### `where_summary`

A factual, evidence-backed location description such as an application, conversation type, document, or visible answer section.

Rules:

- It is descriptive and may be null.
- It does not create openability.
- It must not contain unapproved raw paths, query-bearing URLs, secrets, or invented locators.
- It must remain separate from `direct_return_target`.

### Evidence, confidence, and status

For every non-null semantic field, retain:

```text
request-local support slots
field confidence
missing evidence
field-level verifier result
```

Retain per-visit roles as diagnostics when still needed, but do not require their prose to become part of the public answer.

The model status remains:

```text
resolved
partly_resolved
unresolved
refused
```

LCA-03 will finalize local admission rules. In this phase, parsing must preserve the model's raw status and wording without silently upgrading it.

## Temporal reasoning rules

Add explicit instructions and deterministic contract tests for:

### New task after completed work

```text
completed task A
new user request B
```

Output B as `unfinished_task`. Put A in `completed_context` only when it helps explain the transition.

### Completed implementation with verification remaining

```text
implementation complete
checks passed
user testing remains
```

Output verification as the unfinished task, `needs_user_verification` as the state, and a concrete supported test as the next action.

### Agent still working

Output the user objective as `unfinished_task`, `waiting_for_result` as state, the agent's latest working status as `resume_point`, and an evidence-supported wait/inspect action.

### Unsent draft

Output the draft's purpose as the unfinished task only when visible evidence supports it. Mark the draft as unsent in `resume_point`. Do not claim that it was submitted or that the alleged cause is proven.

### Supporting research or adjacent chat

Keep the primary unfinished task. Represent the other surface through visit roles or completed/supporting context only when its relationship is evidenced.

### Truly completed work

If no follow-up, verification, review, or open loop remains, use `completed`. Do not fabricate a next action merely to make the answer actionable.

### Thin evidence

Return null fields and `unresolved` when the concrete task cannot be recovered. A precise abstention is better than a plausible broad project label.

## One-request provider instruction

Rewrite the compact system instruction so it clearly asks:

```text
What is the newest unfinished task?
What relevant work is already complete?
What exact state was left behind?
What one next action is directly supported by the supplied evidence?
Where was that state observed, without inventing a locator?
```

The instruction must also say:

- read boundaries and panes chronologically;
- prefer attributed user/agent task evidence over visually prominent prose;
- distinguish workstream from current task;
- distinguish implementation from verification;
- distinguish a draft from a submitted request;
- do not turn a causal hypothesis into a fact;
- return null instead of generic wording;
- cite request-local support for every non-null field;
- never invent targets, paths, URLs, commands, intentions, or unsupported actions.

Do not add a second model request to refine or shorten the answer.

## Production mapping requirements

Map the compact result atomically into the existing public answer and task-state contracts.

Required behavior:

- `unfinished_task` supplies the exact active-task meaning.
- `resume_point` supplies the current-step/resume meaning.
- `next_supported_action` supplies `next_action` only after field-level admission.
- `completed_context` remains prior context.
- `where_summary` remains descriptive and non-openable.
- `direct_return_target` remains null unless the existing strict target system independently binds a real target to the same answer identity.
- Do not synthesize target eligibility from `where_summary`.
- Do not combine the compact task with legacy recap progress or a stale task-thread action.
- Do not convert inferred task wording into `explicit_goal`; preserve source kind for LCA-03.
- Preserve the explicit Continue boundary, packet, request, response, verifier, admitted result, and correction identity.

If existing compatibility fields must remain, derive them from the new meanings and document the mapping. Do not allow compatibility fields to override the new contract.

## Expected semantic results for the four critical logs

The wording may differ, but the meaning must match.

### 05cd

```text
unfinished_task:
Assess whether the proposed screen-aware context-reconstruction product solves a real need by reviewing the conversational answer.

task_state:
active

resume_point:
The answer has begun, affirms the need, and continues beyond the visible section.

next_supported_action:
Continue reviewing the answer from the “Does your product solve a real need?” section.
```

### 0d1c

```text
unfinished_task:
Add and inspect the answer-linked visual cue in the real island output.

task_state:
waiting_for_result or active, based on the exact last frame

completed_context:
The backend connection and output flow were already implemented and verified.

next_supported_action:
Return to the Codex visual-cue task and inspect its implementation result.
```

The product-launch checklist must not be part of the unfinished task.

### 0056

```text
unfinished_task:
Verify the completed answer-linked visual cue in Smalltalk.

task_state:
needs_user_verification

resume_point:
The implementation completed and focused checks passed; user verification remains.

next_supported_action:
Open the latest Continue answer and verify that Show more reveals the linked visual cue.
```

The task must not say that implementation still needs to be performed. The PFTU side conversation must not become part of the task.

### 0e34

```text
unfinished_task:
Investigate why the latest Continue result was rejected as insufficient evidence.

task_state:
active

resume_point:
An unsent regression report is being drafted after the failed result.

next_supported_action:
Return to the draft and continue the regression report.
```

The output may say the failure followed the visual-cue work. It must not assert that the visual-cue work caused the failure.

## Required deterministic tests

Add tests for:

1. Strict response-schema parsing and round trip for every new field and enum.
2. Field-length limits that null or reject overlong content without semantic rewriting.
3. New task after completed work.
4. Completed implementation with verification remaining.
5. Agent still working and user waiting.
6. Unsent draft versus submitted request.
7. Supporting research and adjacent-pane exclusion.
8. Completed task with no invented next action.
9. Thin evidence with honest nulls.
10. Generic next-action rejection.
11. Unsupported destructive or invented next-action rejection.
12. Field-local support: one invalid field does not erase valid fields.
13. Atomic production mapping with no legacy semantic fill.
14. Target remains null when only `where_summary` exists.
15. One provider request and no reconciliation.
16. Backward compatibility for stored older compact rows, with explicitly downgraded public behavior.
17. Four privacy-safe critical replay fixtures producing the expected semantic slots.
18. Existing P6 task-turn, next-action, and target-honesty fixtures remain passing.

## Phase acceptance criteria

LCA-02 is complete only when:

- The provider schema directly represents unfinished task, task state, resume point, supported action, completed context, and factual location.
- The prompt asks for a continuation rather than a screen report.
- Completed work cannot become the active task when a newer incomplete task exists.
- Verification remaining becomes the task after implementation completes.
- Unsent drafts remain drafts.
- Supporting panes do not become primary without evidence.
- Every non-null field is support-cited and independently admissible.
- The compact public mapping exposes an admitted `next_action` instead of leaving it empty by construction.
- `where_summary` does not create a direct target.
- Atomic answer identity is preserved.
- Local/legacy semantics cannot fill missing compact fields.
- One-request and privacy invariants remain passing.
- The four critical fixtures match their expected semantic meanings.
- Existing P6 and target-safety tests do not regress.
- Automated checks pass.
- No CUA, GUI, or user-owned manual pass is claimed.

## Verification commands

Run the current equivalents of:

```bash
cd src-tauri
cargo fmt --all -- --check
cargo check
cargo test semantic_probe --lib
cargo test production --lib
cargo test verifier --lib
cargo test task_turn --lib
cargo test accuracy --lib
cargo test continuation --lib
cd ..
npm run build
npm run test:webview
git diff --check
git status --short
```

If a filter matches zero tests, run the exact current suite and report it honestly.

## Required completion audit

Create:

```text
docs/phases/launch-continue-accuracy/02-actionable-continuation-contract-completion-audit.md
```

Include:

1. Old versus new provider and public contracts.
2. Field semantics, size limits, and support rules.
3. Compatibility mapping.
4. Atomic identity mapping.
5. Four critical fixture inputs and semantic-slot results.
6. Generic/invented-action rejection proof.
7. One-request and no-legacy-fill proof.
8. Automated commands and exact results.
9. Known limitations reserved for LCA-03.
10. A statement that manual app testing remains user-owned.

End with exactly one verdict:

```text
PASS — LCA-02 actionable contract is proven and LCA-03 may begin
```

or:

```text
INCOMPLETE — LCA-02 actionable contract remains open and LCA-03 is blocked
```

## Final response format

Report:

1. Contract and prompt changes.
2. Production mapping changes.
3. Four critical-case semantic outputs.
4. Tests and exact results.
5. Completion-audit verdict.
6. Exact contracts LCA-03 may rely on.
7. Manual testing intentionally deferred to the user.

