# Task Truth v2.01 — Repair Causal Evidence And Contain Wrong Answers

## Codex task

Fix the proven evidence-link and containment failures that currently allow real user work to disappear and old UI text to become the current task.

This is the first step of the final Task Truth v2 accuracy program. It is not permission to tune the existing geometry heuristic until one fixture passes. It must repair the temporal evidence chain, prohibit known-invalid task sources, and make the product abstain specifically when no current task is supported.

Do not build the multimodal resolver in this goal. Do not claim that Task Truth v2 is complete when this goal passes.

## Terminal success condition

This goal is complete only when all of the following are proven:

1. A committed typing burst is causally associated with the frame that shows its result, including live-shaped cases where `post_frame_id` was previously null.
2. Current task extraction can use that causal edge without storing raw typed characters.
3. `prior_boundary_sample` can never become the current user goal.
4. Controls, navigation, model pickers, approval chips, browser chrome, and status labels can never become `user_goal` merely because of position or recency.
5. When no valid current task exists, the backend and UI return a typed ambiguity/no-clear-task state rather than a generic or historical headline.
6. A weaker background result cannot replace a stronger explicit manual result.
7. The session-013 failure is represented by a live-shaped regression fixture and the incorrect `Approve for me` task is impossible.

## Read before editing

```text
AGENTS.md
PRODUCT.md
docs/full-engine-flow.md
docs/phases/p6-task-turn-accuracy/p6-00-task-turn-accuracy-program.md
docs/phases/p6-task-turn-accuracy/p6-09-completion-audit.md
src-tauri/src/capture.rs
src-tauri/src/continuation.rs
src-tauri/src/continuation/task_turn_evidence.rs
src-tauri/src/continuation/task_turn.rs
src-tauri/src/continuation/accuracy_eval.rs
src-tauri/src/continuation/accuracy_fixture.rs
src/App.tsx
src/continuePresentation.ts
src-tauri/src/session_island/contract.rs
```

Inspect the current worktree first. Preserve unrelated changes. The checkout is authoritative; names in this prompt are orientation, not permission to overwrite newer work.

Inspect the latest evidence bundles:

```text
continue_outputs/session-013-session-17837254__continue-1783725536934__normal__continue-decision-/
continue_outputs/session-013-session-17837254__continue-1783725612170__normal__continue-decision-/
continue_outputs/session-013-session-17837254__continue-1783725625378__normal__continue-decision-/
continue_outputs/session-013-session-17837254__continue-1783725655970__normal__continue-decision-/
continue_outputs/session-013-session-17837254__continue-1783725753708__normal__continue-decision-/
```

Do not commit those bundles, screenshots, the live SQLite database, raw conversation text, URLs, paths, or personal identifiers.

## Verified root failure

The live capture database contained committed ChatGPT typing bursts with `commit_signal = enter`, but their `post_frame_id` values were null. Current task evidence asks whether a burst is committed with a query equivalent to:

```sql
SELECT EXISTS(
  SELECT 1
  FROM typing_bursts
  WHERE post_frame_id = ?1 AND committed = 1
)
```

Therefore the real user action exists in storage but is invisible to the role resolver. On the affected chat surface, geometry alone gives the right-aligned span confidence `0.58`; the acceptance threshold is `0.64`. The genuine user message is rejected.

The next failure compounds it. When there is no accepted current user span, `build_provisional` can use `prior_boundary_sample` as `goal_summary`. In session-013, a UI control/history string such as `Approve for me` became the apparent current task.

This is a broken causal join followed by an unsafe historical fallback. Do not describe it as a model-quality problem.

## Non-negotiable constraints

- Do not lower `MIN_TYPED_ROLE_CONFIDENCE` from `0.64` to `0.58`.
- Do not add a Codex-, ChatGPT-, Helium-, or right-side-specific task rule.
- Do not store raw keystrokes, typed strings, or full clipboard text.
- Do not make a URL/path/openable candidate evidence of task intent.
- Do not revive or edit `browser-extension/`.
- Do not make a model call in background capture.
- Do not hide ambiguity behind fluent fallback copy.
- Preserve the strict direct-target and evidence-preview separation from P6.

## Goal 1 — Make typing-to-frame causality real

Trace the complete lifecycle of:

```text
ui_event
→ typing_burst start/update/commit
→ capture trigger
→ post-capture frame
→ event transition
→ task-turn evidence
```

Implement one authoritative association path. A safe design must:

1. Bind a burst to the post-action frame when the capture trigger settles.
2. Match by session, bounded time, surface/window identity, and causal trigger/event ids where available.
3. Reject cross-app, cross-window, stale, private, and ambiguous associations.
4. Remain idempotent when a trigger finalizes twice.
5. Never overwrite a stronger existing association with a weaker later guess.
6. Record why the association was made and its confidence/provenance.
7. Support a bounded compatibility lookup for legacy null `post_frame_id` rows without permanently pretending every nearby frame is causal.

Prefer completing the stored relationship when the capture trigger or transition receives its `post_frame_id`. If a compatibility lookup remains necessary, isolate it behind a named function and make the time/surface constraints explicit.

Add indexes only when query plans show they are needed. Do not introduce broad scans on every Continue request.

## Goal 2 — Replace the boolean with an evidence-linked causal result

The task resolver must not receive only `has_committed_typing: bool`. Add a compact causal attribution object or equivalent internal structure containing at least:

```text
typing_burst_id
commit_signal
started_at_ms
ended_at_ms
pre_frame_id
post_frame_id or bounded inferred frame
surface match result
temporal distance
association source
confidence
rejection reasons
```

Use this evidence to strengthen authorship only for content that appeared in the causally related region/frame. A committed Enter somewhere in the same session must not convert every right-aligned span into user-authored text.

Geometry may remain a weak vote. It must not independently establish authorship.

## Goal 3 — Make historical boundaries history-only

Change the current-task contract so that:

- `salient_user_goal_sample` may support the current goal only when it comes from an eligible current user-authored span.
- `prior_boundary_sample` may support relation-to-prior, completion, chronology, or history diagnostics only.
- no-user-goal plus prior completion yields `no_current_goal` or an equivalent typed state; it does not reuse the completed text as the current goal.
- task identity and task revision cannot be generated from a prior-boundary hash when there is no current continuity edge.
- a prior task can continue only through explicit continuity evidence, never because its text is visible.

Remove or migrate any tests that encode the unsafe fallback as intended behavior. Add a schema/version marker if persisted semantics change.

## Goal 4 — Make controls categorically ineligible as authored goals

Create one reusable eligibility policy applied before task selection. At minimum, the following must be ineligible for `user_goal`:

```text
button
menu item
model picker
approval chip
toolbar
navigation
sidebar label
browser chrome
tab chrome
status badge
notification action
composer placeholder
system instruction
generic dialog action
```

Use AX role/subrole, actionability, hierarchy, bounds, focus/editability, OCR/AX cross-source agreement, and visual/control metadata already available. Unknown remains unknown; it does not silently become user-authored.

If a control label contains task-like verbs, it remains a control. `Approve for me`, `Continue`, `Run`, `Send`, and `Try again` are regression examples, not special-case production strings.

## Goal 5 — Add a typed ambiguity result

When the resolver cannot support a current task, emit an explicit internal state such as:

```text
task_resolution_status = no_clear_current_task
reason_codes = [...]
supported_surface = ...
alternative_hypotheses = [] or bounded candidates
```

The primary card and island must not manufacture a headline from:

- prior boundary text;
- app/window title alone;
- an unrelated open loop;
- candidate labels;
- controls/navigation;
- generic strings such as `The agent is working on the current task`.

The user-facing state should say that recent activity was captured but the exact task could not be determined. Evidence inspection may remain available. Do not call this a successful task recovery in metrics.

## Goal 6 — Make result adoption quality-dominant

Add an explicit comparison policy for manual, startup, background, and island refresh results. A background result may replace the visible manual result only when all critical conditions hold:

```text
same task identity at same/newer revision, or clearly newer valid task
no lower task-identity confidence
no weaker claim/evidence coverage
no loss of a previously supported task/state/next/where field
no model-validation downgrade
no target-policy downgrade
evidence watermark is causally newer
```

If those conditions are not met, retain the stronger result and record the rejected adoption in diagnostics. Do not compare only timestamps.

Split provenance internally now, even if the final product presentation lands later:

```text
task_understanding_source
wording_source
target_selection_source
```

The existing `source === cloud_micro_inference` check is not sufficient evidence that displayed wording was model-generated.

## Required regression cases

Add live-shaped fixtures without synthetic conversation-role ids:

1. Committed Enter, null legacy `post_frame_id`, matching later frame.
2. Committed Enter with an explicit post frame.
3. Nearby typing in another app/window must not transfer authorship.
4. Uncommitted typing must not establish submitted user content.
5. `Approve for me` exposed as an actionable control below a composer.
6. Prior completed task visible above a new user request.
7. Prior completed task visible with no new valid task.
8. Mixed AX/OCR where geometry disagrees with temporal causality.
9. Manual model-assisted result followed by weaker local background result.
10. Genuinely newer, stronger background evidence replacing an older result.

The session-013 fixture must preserve the actual weakness of the live AX tree: do not add `conversation-user-message` or `conversation-assistant-message` identifiers that were absent in production.

## Verification

Run the narrow tests first, then the required repository checks:

```text
cargo test <each new narrow Rust test filter>
cargo test continuation::task_turn_evidence
cargo test continuation::task_turn
cargo test continuation::accuracy_eval
cargo test session_island
cargo check
npm run test:webview
npm run build
```

Use the bundled Node runtime on `PATH` if the workspace requires it. Run `cargo fmt --check` or format only touched Rust files. Do not reformat unrelated dirty files.

Perform a local read-only query or audit proving that a newly committed burst is linked to its post frame. Never print or commit raw typed text.

## Definition of done

- The causal association is persisted or boundedly recovered and evidence-linked.
- The session-013 current task cannot become `Approve for me`.
- Prior-boundary fallback is history-only in production and tests.
- Control exclusion is shared policy, not a one-string patch.
- No supported task produces a typed ambiguity state all the way through React and island contracts.
- Background adoption cannot downgrade a stronger manual answer.
- Old P6 target safety and privacy tests still pass.
- The completion report states exactly what remains for Task Truth v2.02.

Do not mark the overall Task Truth v2 program complete.
