# PFTU-02 — Make the Proven Semantic Answer the Only Public Truth

## Codex implementation prompt

Implement this phase completely. This phase finishes root problem 1: every public Continue surface must either show the proven cloud-understood task, a scoped human correction, or an honest unresolved state.

This is a production cutover phase. It is not another semantic architecture experiment.

## Hard dependency gate

Read `pftu-01-completion-audit.md` first.

Stop immediately unless its final verdict is exactly:

```text
PASS — PFTU-02 may begin
```

Independently verify that its real-case artifacts exist and that they contain:

- twelve fresh reviewed cases;
- real provider request and response identities;
- a passed held-back denominator;
- zero confident wrong primary tasks;
- a chosen request/schema/model combination;
- no local semantic repair.

If the audit is absent, internally inconsistent, or based only on fixtures, do not implement this phase. Report the dependency failure and finish PFTU-01 instead.

## Core idea

PFTU-01 proves a small semantic kernel. This phase makes that kernel authoritative without letting legacy fields make the answer look more certain than it is.

The public contract must be atomic. One answer comes from one explicit Continue boundary, one evidence packet, one provider response, one verified semantic result, and one correction state. React, the native island, cache adoption, and opening behavior must consume that same identity.

Observed application activity is still useful evidence. It is not task truth.

## Required reading and current-path audit

Before editing, trace the live path rather than trusting phase summaries:

- explicit Continue invocation in `src/App.tsx`;
- the Tauri Continue command in `src-tauri/src/continuation.rs` and its current callees;
- `src-tauri/src/continuation/task_truth_v2/production.rs`;
- `src-tauri/src/continuation/task_truth_v2/review.rs`;
- existing Task Truth snapshot, task-thread, cache, feedback, and open-time validation code;
- native-island answer construction in `src-tauri/src/session_island.rs` and `src-tauri/macos/`;
- all public-copy sources in activity recap, current focus, work truth, local scorer, handoff, startup adoption, background refresh, and compatibility projections;
- `docs/phases/task-truth-v2/tt2-05-completion-audit.md`;
- `docs/phases/model-first-task-inference/mfti-04-completion-audit.md`;
- session-038 outputs and the PFTU-01 audit.

Produce a field-level source map before changing behavior. For every line visible on React or the island, record the current supplier and the intended post-cutover supplier.

This repository already contains substantial atomic-identity, cache, feedback, island-parity, open-safety, and release-gate machinery from TT2 and MFTI. Do not reimplement it merely because this prompt restates the required behavior. For every requirement below, first prove whether the current implementation already satisfies it. Keep and test correct code. Change only the path that still allows legacy semantics, mixed identity, stale adoption, bad feedback scope, or unsafe opening.

## Required behavior

### 1. Repair the existing public answer contract

Use the current `TaskTruthPublicAnswerV1` and related production decision types where practical. Version a wire contract only when the shape is genuinely incompatible.

The authoritative identity must bind at least:

- explicit Continue boundary or decision id;
- current capture frame and evidence watermark;
- semantic-kernel request and response ids;
- response schema and verifier version;
- admitted semantic result id;
- scoped human correction watermark;
- selected target identity when one exists;
- cache and authority policy versions.

All public semantic fields must come from the same identity. Do not combine a primary task from a cloud response with progress from activity recap, an unfinished state from a stale task thread, or a target explanation from another decision.

### 2. Limit semantic authority to three source kinds

Only these may supply primary task, current step, progress, unfinished state, relationship, or grounded next action:

1. `verified_cloud` from the PFTU-01 semantic kernel;
2. `human_correction` scoped to the exact answer identity and field;
3. `unresolved`, represented as null semantic fields plus an honest reason.

Local activity classes, app names, page titles, file names, local candidate labels, activity recap, work truth, handoff projections, and task-thread compatibility fields may not fill missing semantic fields.

When unresolved, observed app, page, file, and activity can appear only inside an evidence or “Why this answer?” view and must be explicitly described as observation rather than inferred task meaning.

### 3. Route every public surface through the same answer

Manual Continue, React, the native island, cache reuse, startup adoption, and any background refresh must consume the same atomic public answer.

Background work may capture, prepare evidence, or reuse an already verified answer. It may not upload images or create new semantic meaning. Only explicit Continue can trigger cloud semantic processing.

React and the island must show the same:

- semantic source;
- task wording;
- current-step wording;
- progress and unfinished state;
- unresolved reason;
- target availability;
- correction state.

Presentation may differ in length, but not in meaning or confidence.

### 4. Remove all public semantic fallback

Find and block public fallback from:

- activity recap;
- local scorer labels;
- `verb + title` or `verb + object` templates;
- `current_focus`;
- `current_activity`;
- work truth;
- stale handoff fields;
- cached legacy decisions;
- compatibility projections;
- native-island-only copy;
- startup or background adoption.

Keep these systems as diagnostics if still useful. Do not delete historical evidence unless necessary.

Add source-admission tests proving that words such as `editing`, `viewing`, `browsing`, `reviewing_output`, `typing`, and `filling_form` cannot become the public primary task merely because a local classifier emitted them.

### 5. Use precise unresolved states

At minimum distinguish:

- `capture_current_frame_failed`;
- `capture_current_frame_stale`;
- `privacy_blocked`;
- `provider_disabled`;
- `credentials_missing`;
- `provider_timeout`;
- `provider_error`;
- `provider_empty_output`;
- `structured_output_invalid`;
- `evidence_validation_rejected`;
- `task_not_recoverable`;
- `no_new_evidence`.

The user-facing copy should be simple. Diagnostics must retain the typed reason. No failure may be replaced by a stronger-looking local answer.

### 6. Repair freshness and repeated-Continue behavior in the production path

An explicit Continue must use a newly persisted external frame representing the current screen state. If that cannot be obtained, return the precise capture failure. Do not silently reuse an old frame as current.

Bind cache reuse to the complete atomic identity. A newer unresolved boundary must defeat an older precise answer unless current evidence explicitly proves continuity and the public answer identifies that continuity.

When the evidence identity is unchanged, return `no_new_evidence` and the existing verified answer. Do not invoke another model judgment merely because Continue was pressed again.

Repeated QA presses, request timeouts, empty targets, or dismissals must not be converted into inferred semantic rejection or broad candidate suppression. Explicit `Not right` feedback remains strong but must be scoped to the exact answer, field, hypothesis if present, and target if present.

Reproduce the session-038 pattern where candidate count fell from many candidates to none after inferred feedback. Prove that the new public answer and unrelated future tasks are not suppressed by those events.

### 7. Keep opening subordinate to truth

Selecting a task does not automatically make a target safe or correct.

A direct open requires:

- a target explicitly attached to the same atomic public answer;
- current locator and privacy validation;
- no applicable explicit rejection;
- current target ownership by the selected task answer;
- no stale or mixed identity.

If the task is understood but the location is not safely supported, show the task answer without a direct open. Never choose an easy-to-open application as a substitute for the understood task.

## Development live gate

Do not modify or weaken the existing larger MFTI release gate. Its corpus requirements remain honest and may remain incomplete.

For this implementation session, create a separate `pftu_02_development_live_gate` that proves the behavior of the current development build. It must not claim public release readiness.

Run at least twelve fresh manual Continue scenarios on the current macOS build:

1. Correct recoverable task.
2. Honest unresolved task.
3. Provider disabled.
4. Provider timeout or controlled failure.
5. Stale current capture.
6. Repeated Continue with unchanged evidence.
7. Supporting browser research.
8. Wrong or unsafe return target.
9. Field-level human correction.
10. Valid cache reuse.
11. React and island parity.
12. A session-038-style workflow through Codex, VS Code, browser research, and an API dashboard.

For every scenario record:

- expected task meaning written before output;
- session, decision, answer, request, and response ids;
- public source kind;
- actual React copy;
- actual island copy;
- target and open outcome;
- `Correct`, `Partly right`, or `Wrong`;
- one-line correction;
- whether any public field came from a local semantic source;
- whether atomic identities matched;
- whether the current frame was fresh.

## Required pass gate

All must be zero:

- local-semantic fallback on public surfaces;
- mixed atomic identities;
- stale frame claimed as current;
- a failed provider result replaced by a local answer;
- React/island semantic disagreement;
- cross-task suppression from inferred feedback;
- unsafe or task-inconsistent opens;
- background image uploads;
- privacy violations.

Additionally:

- all twelve scenarios have real manual results;
- all recoverable semantic scenarios are `Correct` or `Partly right`;
- no scenario is confidently wrong about the primary task;
- unresolved scenarios remain useful and honest;
- unchanged evidence avoids a redundant model call;
- an explicit correction affects only its exact scope;
- the existing MFTI release report remains truthful about missing corpus evidence.

## Automated verification

Add or update narrow tests for:

- semantic-source admission;
- unresolved field non-filling;
- atomic identity mixing rejection;
- current-frame freshness;
- unchanged-evidence detection;
- cache reuse and newer-unresolved precedence;
- inferred-feedback isolation;
- explicit correction scope;
- target ownership and open-time revalidation;
- React presentation;
- native-island contract parity;
- every typed provider failure.

Run:

```bash
cd src-tauri
cargo fmt --all -- --check
cargo check
cargo test task_truth_v2 --lib
cargo test continuation --lib
cargo test session_island --lib
cd ..
npm run build
npm run test:webview
git diff --check
```

## Forbidden work

Do not:

- begin longitudinal episode architecture;
- add another public answer engine;
- treat legacy recap or task-thread fixture data as cloud truth;
- lower or fake the MFTI release denominator;
- upload in the background;
- add raw typed text or clipboard capture;
- commit captures, databases, provider payloads, or credentials;
- claim that a green schema test proves truthfulness.

## Required completion audit

Create `pftu-02-completion-audit.md` beside this file. Include:

- the before/after public field-source map;
- code paths changed;
- all twelve manual scenario results;
- exact counts for every zero-tolerance gate;
- React/island parity proof;
- freshness, cache, feedback, and open-safety proof;
- automated command results;
- the current larger MFTI release verdict without alteration;
- engineering behavior proven versus formal release evidence still missing.

End with exactly one verdict:

- `PASS — root problem 1 is proven and PFTU-03 may begin`, or
- `INCOMPLETE — root problem 1 remains open and PFTU-03 is blocked`.
