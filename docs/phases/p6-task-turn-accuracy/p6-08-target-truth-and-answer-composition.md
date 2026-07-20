# P6.08 — Direct-Target Truth And Interruption-Recovery Answer

## Codex task

Align backend, React, and native island with one strict target contract, then compose the P6 task truth into a compact interruption-recovery answer. The product must remain useful when it knows the task but cannot directly reopen the exact page, conversation, or file.

This goal is product integration, not a visual redesign. Preserve the Continue-first surface and keep diagnostics secondary.

## Dependency gate

P6.01-P6.07 must be complete. Confirm that the decision path now supplies:

- a current task turn and prior-turn relation;
- execution state, current actor, waiting-on state, and evidence-backed next action;
- coherent selected workstream/branch/open-loop state;
- claim-level confidence;
- validated local/model recap with task identity parity;
- target identity/openability dimensions and the P6.05 internal direct-target policy.

Read:

```text
AGENTS.md
PRODUCT.md
docs/phases/p6-task-turn-accuracy/p6-00-task-turn-accuracy-program.md
docs/phases/p6-task-turn-accuracy/p6-06-confidence-and-observation-reliability.md
docs/phases/p6-task-turn-accuracy/p6-07-recap-truth-pack-and-validation.md
```

## Current verified contract drift

`PRODUCT.md` says public `return_target` and `resume_work_target` stay null when the exact safe target is missing. The reviewed backend can still expose a human-labeled artifact as a public target when openability is `frame_fallback` and no URL/path exists. P5 can then describe it as a safe return point.

React's primary action is stricter: it requires `openability = openable` and a browser URL or document path before showing `Continue here`. This prevents the obvious unsafe UI open, but the backend payload and copy still leak target-shaped confidence.

The contract must be coherent across:

```text
backend decision
persisted decision
activity recap
handoff copy
React card
native island
strict open command
audit/eval
```

## Files and symbols to inspect first

```text
src-tauri/src/continuation.rs
src-tauri/src/continuation/activity_recap.rs
src-tauri/src/continuation/activity_recap_model.rs
src-tauri/src/continuation/activity_recap_open_loop.rs
src-tauri/src/continuation/activity_recap_validation.rs
src-tauri/src/session_island.rs
src-tauri/src/session_island/
src-tauri/macos/SessionIslandPanel.swift
src-tauri/src/capture.rs
src/App.tsx
src/App.css
src-tauri/src/lib.rs
```

Search for:

```text
ContinueReturnTarget
ContinueOpenability
candidate_exposes_public_return_target
candidate_target_is_directly_openable
return_target_summary
return_target
resume_work_target
frame_fallback
humanTargetLabel
humanTargetMeta
isDirectResumeTargetOpenable
getContinueCardActionState
Continue here
Inspect evidence
IslandContinueState
open_resume_point
continue_decision_id
why_this_target
why_no_safe_target
target_confidence
target_explanation_claims
presentContinueDecision
buildContinueProductStateCopy
insert_continue_decision
```

## Canonical target contract

### Direct return target

A public `return_target` or `resume_work_target` may exist only when all are true:

```text
artifact/candidate passed feedback and branch policy
target belongs to or is explicitly eligible for the current task/workstream
openability is direct/openable
a validated browser URL or document path exists, or a deliberately typed direct locator is supported end-to-end by strict-open policy
strict-open policy can act on the persisted decision id
target identity confidence meets policy
target is not stale relative to the current task turn
```

A human label alone is insufficient.

The current implementation's direct locators are browser URL and document path. Do not invent a new locator type merely to make the critical fixture openable. If a future typed locator is introduced, it must have its own schema, validation, strict-open strategy, frontend/island handling, and regression coverage. Frame id, screenshot path, session id, and generic app identity do not qualify.

### Evidence preview

Frames, screenshots, anchors, and non-direct surface snapshots are inspectable evidence. Represent them separately, for example through existing evidence anchors or a typed `evidence_preview` object.

They must not populate public return/resume targets and must not enable the primary open action.

### App focus only

If the product can focus an app but cannot locate the task object, treat that as a separate, explicitly weak capability. It is not the preferred `Continue here` target and must not imply the exact work will be restored.

### Null target with strong task truth

This is a valid, useful decision:

```text
task known
state known
next action known or partly known
direct target null
evidence preview available
```

Do not downgrade the entire recap to generic `inspect evidence` simply because openability is missing.

## Backend integration

1. Harden public target exposure at the backend boundary.
2. Keep candidate/internal artifact information for audit without leaking it as a public target.
3. Ensure persisted decisions deserialize compatibly; migrate/version target semantics if needed.
4. Make `why_this_target` impossible without a typed direct target.
5. Make `why_no_safe_target` explain the missing locator, not missing task understanding.
6. Derive target confidence solely from target identity/openability/policy dimensions.
7. Keep strict open keyed by persisted decision id and revalidate at open time.
8. Record blocked/inspect-only reason codes.

Required backend reason codes should distinguish:

```text
direct_target_available
missing_url_or_path
frame_preview_only
app_focus_only
target_stale
target_feedback_suppressed
target_support_only
target_identity_thin
task_known_target_unknown
no_clear_task_or_target
```

## Public answer contract

The primary answer should render five concise ideas, omitting empty sections without collapsing meaning:

1. **What you were doing** — current task goal/object.
2. **Where** — safe app/page/conversation/file label at supported confidence.
3. **Where you left off** — execution/current actor/waiting-on state and last concrete progress; relevant prior completion only if clarifying.
4. **Next** — evidence-backed next action or an honest unknown.
5. **Action** — `Continue here` only for direct target; otherwise `Inspect evidence` or no action.

The exact visual hierarchy should follow the current product design. Do not expose ids, score tables, branch states, evidence vectors, or raw warnings on the first screen.

## Critical expected answer

For the Capture-button fixture, the product meaning should be equivalent to:

```text
What you were doing
Investigating what the island Capture button does in Smalltalk.

Where you left off
The earlier Continue-card copy task was complete. The current agent had started tracing the Swift bridge and Rust handler to determine whether Capture starts continuous capture or saves one evidence point, so you were waiting for that result.

Next
If the agent is still working, wait for or return to the active investigation. Review the tracing result only when later evidence proves a result exists; otherwise inspect the linked implementation evidence.

Target
No direct conversation/page locator is available. Offer evidence inspection, not Continue here.
```

Do not hard-code this wording or fixture terms into production. It demonstrates the semantic contract.

## Confidence presentation

Use the P6 confidence vector to express uncertainty only where relevant.

Examples:

- Task high, target none: show confident task recap and restrained `Exact location unavailable` target note.
- Task medium, state low: state what is known and name the missing state evidence.
- Identity high, task low: say which surface was active but do not invent its task.
- Target direct/high: show `Continue here` with a concrete safe label.

Do not display a single high/medium/low badge that obscures opposite task and target confidence.

## React main-card behavior

- Render the validated task-turn recap as the main answer.
- Keep `current_focus` factual and secondary when it differs from primary work.
- Keep prior task completion compact and only when it helps recover context.
- Show support/detour notes only when relevant.
- Use `Continue here` only for a direct target.
- Use evidence inspection for frame/anchor previews.
- Prevent target label/meta helpers from making null/preview state sound openable.
- Keep diagnostics under the existing secondary developer surface.
- Preserve accessible labels, keyboard behavior, loading/stale/error states, and responsive behavior.

## Native island parity

Map the same persisted decision/task recap into `IslandContinueState`.

Required states/actions:

```text
direct_continue_ready -> Continue here
task_known_target_unknown -> View summary or Inspect evidence
thin_task_seen -> Inspect evidence
no_clear_task -> Need more evidence
stale_decision -> Refresh before action
suppressed/support-only -> No direct open
error -> Safe error state
```

The island must not re-run an independent semantic decision, surface legacy resume state, or open from frame/session ids. It consumes the same decision id and strict-open policy as the main card.

## Copy invariants

Forbidden when no direct target exists:

```text
Continue here
safest return point
return to this exact page
resume in this conversation
open the work
```

Allowed inspect-only meaning:

```text
Inspect evidence
View what was captured
Exact location unavailable
I know the task, but I do not have a direct page or file locator
```

Do not reveal raw app paths, URLs, identifiers, or private evidence text.

## Required behavior matrix

| Task/target state | Main card | Island | Open behavior |
| --- | --- | --- | --- |
| Task clear, direct safe target | Specific recap + Continue here. | Compact recap + Continue. | Open through decision id after strict revalidation. |
| Task clear, frame preview only | Specific recap + Inspect evidence. | Task known/inspect. | Never direct-open frame as return target. |
| Task clear, app focus only | Specific recap + explicit app-only limitation. | View summary/inspect. | No claim of exact restoration. |
| Task clear, target suppressed | Specific recap + suppression-safe explanation. | No direct open. | Blocked. |
| Identity clear, task thin | Surface fact + task uncertainty. | Need more evidence/inspect. | No direct open unless independent direct-target policy and truthful copy permit it. |
| No clear task or target | Honest no-clear-continuation state. | Need more evidence. | No open. |
| Decision stale | Refresh state. | Refresh state. | Re-decide before any open. |

## Tests

Add deterministic backend tests for:

1. Frame fallback never exposed as public return/resume target.
2. Human label without a supported direct locator is insufficient.
3. Direct URL/path target, or another deliberately supported typed direct locator, passes only with policy eligibility.
4. Task-known/target-null retains useful recap.
5. `why_this_target` absent for null/preview target.
6. Strict open blocks stale/suppressed/support/preview targets.
7. Persisted legacy decision compatibility.
8. Public target fields match the P6.05 internal direct-target policy.

Add React tests where the project supports them, or extract pure presentation helpers for tests:

1. Direct target shows `Continue here`.
2. Frame preview shows `Inspect evidence`.
3. Task recap remains specific with null target.
4. Current focus does not replace primary task.
5. Task/target confidence are presented separately.
6. Forbidden target copy is absent in inspect-only states.

Add island mapping/parity tests:

1. Main card and island share decision/task identity.
2. Direct target state/action parity.
3. Task-known/target-unknown parity.
4. Suppressed/support/stale parity.
5. Island cannot open without decision id or through legacy routes.

Full-pipeline critical assertions:

- final answer identifies the Capture-button investigation and tracing state;
- prior completion is correctly framed;
- public return/resume targets are null;
- frame evidence remains inspectable;
- no target-shaped copy appears;
- main card and island semantic/task/target state agree;
- target-honesty violations and unsafe opens are zero.

## Manual QA

Run the Tauri app on macOS and verify:

1. A clear chat/agent task with no direct conversation locator.
2. A direct openable file or browser page.
3. A frame-preview-only weak surface.
4. A support branch that remains evidence-only.
5. A feedback-suppressed target.
6. A stale decision after new task evidence.
7. Main card/island parity for each state.
8. Keyboard/accessibility behavior for Continue and Inspect actions.

Capture private screenshots only for local QA; do not commit them.

Persist the privacy-safe results in `docs/phases/p6-task-turn-accuracy/p6-08-manual-qa-results.md`. Record scenario, expected/observed task-state tuple, target/action state, main-card/island parity, pass/fail, and redacted evidence reference. Do not include screenshots, raw decision ids, paths, URLs, or private conversation text.

## Acceptance criteria

P6.08 is complete when:

- Backend public targets require direct openability and a supported typed locator; in the current implementation that means URL/path.
- Frame fallback is evidence preview only everywhere.
- Strong task truth remains useful with a null target.
- React and island render the same task/execution/current-actor/waiting-on/target policy.
- Strict open remains decision-id-based and fail-closed.
- Critical fixture renders the correct interruption-recovery meaning.
- Target-honesty and island-bypass counts are zero.
- First-screen UI remains one Continue answer, not a diagnostic dashboard.
- Build, Rust tests, UI tests, and manual macOS QA pass.
- The privacy-safe P6.08 manual-QA results file exists and reflects the actual run.

## Verification commands

Run:

```bash
cd src-tauri && cargo fmt --check
cd src-tauri && cargo check
cd src-tauri && cargo test
npm run build
npm run tauri dev
git diff --check
git status --short
```

If `npm run tauri dev` is interactive, report the manual scenarios and observed results. Do not claim manual QA passed without running it.

## Final response format

Report:

1. Backend/frontend/island files changed.
2. Canonical direct-target and evidence-preview contracts.
3. Public answer composition.
4. Confidence presentation.
5. Main-card/island parity proof.
6. Automated tests and full-pipeline metrics.
7. Manual QA results.
8. Remaining corpus/release-gate work for P6.09.
9. Exact integrated behavior P6.09 must verify.

Do not claim P6 release readiness yet. P6.08 integrates the truthful answer; P6.09 must prove it across time and varied real work.
