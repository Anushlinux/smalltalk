# LCA-04 — One-Instruction Product Answer And React/Island Parity

## Codex implementation task

Make the first-screen Continue experience present one actionable instruction, one short resume-state line, and one safe action. React and the native island must consume the same admitted answer and preserve the same meaning, certainty, task state, and target policy. Diagnostic fields must move behind Inspect instead of requiring the user to synthesize the answer.

This is the fourth phase of the Launch Continue Accuracy repair. Implement it completely only after LCA-01 through LCA-03 have passed.

This phase is a semantic presentation repair, not a visual redesign. Preserve the current island geometry, visual cue, history behavior, capture controls, and overall layout unless a minimal compatibility change is required to present the canonical answer.

## Hard dependency gate

Read:

```text
docs/phases/launch-continue-accuracy/01-task-relevant-evidence-packet-completion-audit.md
docs/phases/launch-continue-accuracy/02-actionable-continuation-contract-completion-audit.md
docs/phases/launch-continue-accuracy/03-truthful-admission-and-authority.md
docs/phases/launch-continue-accuracy/03-truthful-admission-and-authority-completion-audit.md
```

Continue only when the audits end exactly with:

```text
PASS — LCA-01 evidence packet is proven and LCA-02 may begin
PASS — LCA-02 actionable contract is proven and LCA-03 may begin
PASS — LCA-03 admission and authority are proven and LCA-04 may begin
```

Independently confirm that the backend now exposes admitted unfinished task, task state, resume point, supported action, completed context, semantic status, target status, and atomic identity.

## Hard operating rules

- Read `AGENTS.md` and obey it.
- Preserve the current worktree and unrelated changes.
- Do not use Computer Use, AppleScript UI automation, Chrome control, the in-app browser, screenshots of the running app, mouse clicks, or keyboard automation.
- Do not launch a GUI and claim visual acceptance.
- The user will perform live and visual testing after LCA-05.
- Deterministic UI mapping tests, TypeScript builds, webview unit tests, Rust tests, and Swift type-checking are allowed.
- Do not alter model input, evidence selection, provider schema, or admission rules unless a contradiction is found. Fix contradictions in the owning earlier phase and keep its audit honest.
- Do not make a target openable through UI changes.
- Do not use `unfinished_state` as a substitute for `next_supported_action`.
- Do not expose confidence scores, role objects, support slots, IDs, validation reasons, or frame lists on the first screen.
- Do not revive diagnostic dashboards, sessions, timelines, or raw evidence as the primary UI.
- Do not change the current native-island geometry merely to fit verbose copy.
- Do not break the answer-linked visual cue or read-only output history.

## Required reading

Read current versions of:

```text
AGENTS.md
PRODUCT.md
docs/current-island-ui-ux.md
docs/phases/p6-task-turn-accuracy/p6-08-target-truth-and-answer-composition.md
docs/phases/p6-task-turn-accuracy/p6-08-manual-qa-results.md
docs/phases/p6-task-turn-accuracy/p6-09-completion-audit.md
docs/phases/proof-first-task-understanding/pftu-02-truthful-production-answer.md
```

Inspect current implementations and tests for:

```text
src/App.tsx
src/App.css
src/continuePresentation.ts
src-tauri/src/continuation/task_truth_v2/production.rs
src-tauri/src/continuation/history.rs
src-tauri/src/session_island.rs
src-tauri/src/session_island/contract.rs
src-tauri/src/session_island/gateway.rs
src-tauri/macos/SessionIslandPanel.swift
```

Locate the actual current presentation helper path. Do not assume a prompt-era file path is still correct.

## Current failures to remove

The current presentation path can:

1. Use `unfinished_state` as though it were an action.
2. Concatenate last progress and unfinished state into a dense status paragraph.
3. Show task, current activity, relationship, subtask, last progress, unfinished state, action, and location as separate rows.
4. Ask the user to reconcile overlapping or contradictory fields.
5. Display “Exact location unavailable” without still leading with the useful semantic instruction.
6. Translate new visit-role values through an older native enum and fall back to “relationship is not clear.”
7. Produce React/native semantic disagreement even when both consume the same backend answer.

LCA-04 must remove these problems without hiding uncertainty.

## Required canonical product projection

Create or repair one deterministic backend-owned or shared typed projection for product presentation. Use current architecture when it already has an equivalent contract. Do not create a second semantic answer engine.

The projection must carry these meanings:

```text
answer_identity
presentation_state
primary_instruction
resume_context
location_context
semantic_status
task_state
target_status
primary_action
inspect_available
unresolved_reason when applicable
```

React and the native island must consume the same strings and state/action meanings. They may truncate only through a shared documented policy and must not independently reinterpret semantic fields.

### `primary_instruction`

Use admitted `next_supported_action` as the first-screen headline.

Rules:

- Use one imperative or directly actionable sentence.
- Do not repeat the project name unless necessary for disambiguation.
- Do not prepend “Continue working on” to a long task summary.
- Do not construct an instruction from `unfinished_state`.
- Do not display support citations or internal qualifiers.
- Preserve material uncertainty through presentation state or restrained wording.
- Target length: at most 160 characters.

If no supported action exists but the task is known, use a truthful task-known/action-unknown state rather than manufacturing an instruction.

### `resume_context`

Use one short sentence explaining where the task stopped.

Rules:

- Prefer the admitted resume point.
- Include completed context only when it prevents temporal confusion.
- Do not concatenate every semantic field.
- Do not repeat the instruction.
- Target length: at most 180 characters.

### `location_context`

Show only when it helps the user orient and is evidence-backed. It does not imply openability.

### `primary_action`

Use a stable typed action:

```text
open_direct_target
inspect_evidence
refresh_continue
none
```

`open_direct_target` requires the existing strict decision-bound target. `inspect_evidence` may show the aligned visual cue/frame but must not be labeled as a return target. `refresh_continue` is for stale evidence or a retryable typed failure. `none` is allowed.

## Required first-screen states

### Action known, direct target ready

```text
Primary instruction
Short resume context
[Continue here] [Inspect]
```

### Action known, target unavailable

```text
Primary instruction
Short resume context
[View last screen] or [Inspect]
```

Do not replace the useful instruction with “Exact location unavailable.”

### Task known, action unknown

```text
I found the task, but not a safe next step.
Short task/resume context
[Inspect]
```

### Task unknown

```text
I couldn’t identify the unfinished task.
Short evidence-quality explanation when useful
[Try Continue again] or [Inspect]
```

### Provider or parser failure

Use truthful failure-specific product copy. Do not say “not enough evidence” when the provider timed out, output was invalid, or validation rejected one field.

### Stale decision

```text
The saved answer is older than the latest work.
[Refresh Continue]
```

Do not allow opening the stale target.

## Inspect boundary

Move these behind Inspect or the existing expanded diagnostic/evidence surface:

```text
completed context beyond one necessary sentence
visit roles and relationships
current activity classification
support slots and confidence details
field-level admission reasons
frame/event/artifact/action ids
recent surface timeline
evidence-quality notes
target ownership details
```

Inspect must not become the default first screen. The island's Show more may reveal useful context and the answer-linked visual cue, but the first meaning must remain the primary instruction.

## React/native parity

Fix all semantic enum and projection drift.

At minimum, both surfaces must understand:

```text
primary_work
supporting_work
detour_or_unrelated
unclear
```

If older values remain for compatibility, create one explicit mapping and test every value. An unknown role may become unclear, but a known `primary_work` value must never be rendered as unclear.

For the same answer identity, React and the island must agree on:

- primary instruction;
- resume context;
- semantic status;
- task state;
- target status;
- action type and label;
- whether Inspect is available;
- stale/failure state;
- direct-open eligibility.

Do not derive React copy from activity recap while the island derives it from the compact answer. Do not let the island fall back to native-only semantic prose.

## Preserve current visual and product behavior

Do not redesign the island. Preserve:

- existing compact/resting geometry;
- existing hover/expansion behavior;
- existing answer-linked visual cue;
- existing Show more interaction;
- existing read-only output history;
- capture-status behavior;
- Continue action routing;
- privacy redaction;
- strict target opening;
- current animation behavior unless a compile fix requires a minimal adjustment.

This phase changes information hierarchy and semantic mapping, not the visual direction.

## Expected product copy for the four critical logs

Exact wording may be refined, but the meaning and hierarchy must match.

### 05cd

```text
Continue reviewing the answer about whether the product solves a real need.

The answer has begun and continues beyond the visible section.

[View last screen] [Inspect]
```

### 0d1c

```text
Return to the Codex visual-cue task and inspect its implementation result.

The backend connection was already complete; the newer visual-cue request was still active.

[View last screen] [Inspect]
```

### 0056

```text
Test the new answer-linked visual cue in Smalltalk.

Implementation passed its checks; user verification remains.

[View last screen] [Inspect]
```

The first screen must not mention the PFTU release gate.

### 0e34

```text
Return to the drafted regression report and continue the investigation.

The latest Continue result was rejected as insufficient evidence; the cause is not yet proven.

[View last screen] [Inspect]
```

The first screen must not assert that the visual-cue change caused the regression.

## Required deterministic tests

Add or update tests for:

1. Supported action becomes the primary instruction.
2. `unfinished_state` cannot become the action fallback.
3. Last progress and unfinished state are not concatenated into the headline.
4. Action-known/target-unknown keeps a useful instruction.
5. Task-known/action-unknown uses the precise partial state.
6. Task-unknown uses a true semantic abstention.
7. Provider, parser, validation, semantic, and target states have distinct copy.
8. Stale decision blocks open and offers refresh.
9. Direct target action requires decision-bound target eligibility.
10. Frame preview is Inspect/View last screen, never Continue here.
11. Every new and legacy visit-role enum maps identically in React and native.
12. Known `primary_work` never becomes unclear.
13. React/island instruction, context, status, target state, and action parity.
14. Main screen excludes support slots, IDs, confidence numbers, and diagnostics.
15. Inspect retains useful evidence without changing semantic authority.
16. Answer history preserves the saved canonical copy and identity.
17. Visual cue remains linked to the exact answer evidence.
18. Four critical cases render the expected product meaning.
19. Existing keyboard/accessibility labels remain accurate in deterministic source/component tests.
20. Existing strict-open, no-bypass, P6 target, history, and island tests remain passing.

## User-owned manual test handoff

Prepare a short manual script in the completion audit, but do not execute it. It must tell the user how to verify after LCA-05:

- the compact island first line;
- Show more hierarchy;
- visual cue alignment;
- View last screen versus Continue here;
- React/native meaning parity;
- history preservation;
- stale and failure copy;
- keyboard/accessibility behavior.

Every row must remain `Not run — user-owned` in this phase.

## Phase acceptance criteria

LCA-04 is complete only when:

- One supported instruction leads the first screen.
- One resume-context sentence follows it.
- Missing target does not hide the instruction.
- Missing action does not create a fake instruction.
- Diagnostics are behind Inspect.
- React and native consume one canonical meaning.
- New visit-role enums no longer fall through the legacy unclear branch.
- Direct target, frame preview, stale, support-only, and suppressed states remain safe.
- The current island geometry, visual cue, history, and capture behavior remain intact.
- Four critical fixtures render the expected meaning without diagnostic contamination.
- Deterministic accessibility and parity tests pass.
- Automated builds and tests pass.
- Manual scenarios are documented but remain user-owned and unclaimed.
- No CUA or GUI automation was used.

## Verification commands

Run the current equivalents of:

```bash
cd src-tauri
cargo fmt --all -- --check
cargo check
cargo test production --lib
cargo test session_island --lib
cargo test history --lib
cargo test continuation --lib
cd ..
npm run build
npm run test:webview
swiftc -typecheck src-tauri/macos/SessionIslandPanel.swift
git diff --check
git status --short
```

Use the repository's actual Swift type-check command and required companion files if the one-file command is insufficient. Do not report a zero-test filter as a pass.

## Required completion audit

Create:

```text
docs/phases/launch-continue-accuracy/04-one-instruction-ui-parity-completion-audit.md
```

Include:

1. Before/after presentation flow.
2. Canonical projection contract.
3. React/native field and enum map.
4. Four critical fixture presentation snapshots as text, not screenshots.
5. Direct/inspect/stale/failure policy table.
6. Evidence that diagnostics moved behind Inspect.
7. Visual-cue/history/non-breakage deterministic proof.
8. Automated commands and exact results.
9. User-owned manual script with every scenario marked not run.
10. Limitations reserved for LCA-05.

End with exactly one verdict:

```text
PASS — LCA-04 product answer and parity are proven and LCA-05 may begin
```

or:

```text
INCOMPLETE — LCA-04 product answer or parity remains open and LCA-05 is blocked
```

## Final response format

Report:

1. Canonical first-screen behavior.
2. React/native parity changes.
3. Four critical-case visible text results.
4. Automated checks and exact results.
5. Completion-audit verdict.
6. What LCA-05 must prove.
7. Manual testing intentionally deferred to the user.

