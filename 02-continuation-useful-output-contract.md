# Prompt 02 — Build the Continuation-Useful Atomic Answer Contract

## Codex implementation prompt

Implement this phase completely in the current `smalltalk` repository. This is the second of three ordered implementation sessions. Its job is to turn the proven compact Luna interpretation into one lean, useful Continue answer without mixing semantic fields, evidence boundaries, or return targets from different identities.

This session may build and verify the new answer path behind an explicit development flag. It must not make the final production cutover or delete the legacy request path. Prompt 03 owns that final switch.

## Hard dependency gate

Read this file first:

```text
docs/phases/lean-continue/01-compact-semantic-episode-input-completion-audit.md
```

Stop unless its final line is exactly:

```text
PASS — 02-continuation-useful-output-contract may begin
```

Independently verify the proof behind the verdict. At minimum, confirm:

- twelve fresh Luna cases exist;
- held-back cases exist;
- there are zero confidently wrong primary tasks;
- late evidence was excluded by an explicit current-frame cutoff;
- request size and deduplication gates passed;
- no local semantic fallback repaired Luna’s output.

If any dependency evidence is missing, contradictory, fixture-only, or stale relative to current code, do not work around it. Return to Prompt 01 and leave this phase incomplete.

## Product outcome

The user should receive one answer to one question:

```text
What was I actually doing, what exact useful point had I reached,
what remains unfinished, and where can Smalltalk safely return me?
```

The answer should be concise enough to understand immediately. Diagnostics remain inspectable but must not dominate the primary surface.

The semantic answer and the open target have different authorities:

- Luna supplies task meaning from the compact evidence boundary.
- Local deterministic code validates the exact browser URL, document path, or frame locator.
- Smalltalk may combine them only when both are proven to belong to the same atomic answer identity.

## Preserve and audit current work before editing

This checkout contains uncommitted user work, including a large in-progress projection from PFTU probe rows into `TaskTruthPublicAnswerV1`, React copy changes, and native-island copy changes.

Before editing:

1. Run `git status --short`.
2. Read the full current diff, especially:
   - `src-tauri/src/continuation/task_truth_v2/production.rs`
   - `src-tauri/src/continuation/task_truth_v2/semantic_probe.rs`
   - `src-tauri/src/continuation/task_truth_v2.rs`
   - `src-tauri/src/continuation.rs`
   - `src/continuePresentation.ts`
   - `src/App.tsx`
   - `src-tauri/src/session_island.rs`
   - `src-tauri/macos/SessionIslandPanel.swift`
3. Read completely:
   - the Prompt 01 completion audit;
   - `docs/phases/proof-first-task-understanding/pftu-02-truthful-production-answer.md`;
   - relevant TT2 and MFTI completion audits;
   - the current public answer, cache, feedback, and strict-target tests.
4. Produce a private working map of every primary React and island line and its current supplier.

Do not reset, overwrite, or recreate correct uncommitted work. Do not build another public-answer subsystem if `TaskTruthPublicAnswerV1` can be safely evolved.

## Proven current gaps to verify

Confirm these against the current checkout rather than assuming they remain true:

1. The PFTU projection can mark any non-null model `primary_task` as `task_basis: explicit_goal`, even when the task was inferred from screen evidence.
2. The PFTU projection creates synthetic task-thread and snapshot identities derived from output content.
3. The strict target owner looks for a matching persisted Task Truth snapshot and therefore cannot attach a target to an unpersisted synthetic PFTU snapshot.
4. That behavior is safe against an incorrect open but leaves the compact answer inspect-only, so it does not yet fulfill the product’s exact-resume-point promise.
5. A legacy model-answer visibility fallback can still expose the old forensic answer when the compact result is absent.
6. React and the island have multiple compatibility paths that can display local activity, recap, current-focus, or legacy answer copy.
7. Validation-limited compact fields can remain visible. The product contract needs an explicit rule for when partial fields are useful and when the entire answer must be unresolved.

Record material differences in the completion audit.

## Required public semantic contract

Design the smallest practical versioned contract. Reuse existing types when they can express the following without ambiguous compatibility behavior.

The primary Continue answer may contain only these semantic fields:

1. `primary_task` — the concrete evidenced purpose, nullable.
2. `resume_point` — the concrete step or state from which work can continue, nullable.
3. `last_meaningful_progress` — the last evidenced material progress, nullable.
4. `unfinished_state` — what remained incomplete, nullable.
5. `relationship_to_previous_verified_task` — one bounded relationship enum, nullable or `unknown`.
6. `next_supported_action` — nullable and present only when evidence supports it.

It also carries non-semantic contract data:

- resolution status;
- typed unresolved or partial reason;
- field-level support and confidence;
- semantic source;
- answer identity;
- optional validated return target;
- safe evidence-preview identity;
- provider/request/response provenance for diagnostics.

Do not put multiple competing hypotheses, forensic contradiction prose, sixteen confidence fields, policy essays, application identity hashes, or internal candidate tables on the primary product contract.

Diagnostics may retain richer internal records behind inspection surfaces.

## Required implementation

### 1. Extend the proven Luna schema only as much as necessary

Use `gpt-5.6-luna` only. Do not compare with Sol or create automatic model fallback.

Reuse the four Prompt 01 semantic fields. Add only:

- `relationship_to_previous_verified_task` when a prior verified task is actually supplied through a bounded prior-answer reference;
- `next_supported_action` when explicit plan, pending operation, or clearly supported continuation evidence exists.

Do not send raw prior history. A prior reference may contain only the prior verified semantic fields, stable local identity, time, and bounded support needed to ask whether the current boundary continues, supports, detours from, interrupts, returns to, or starts separately from that task.

Allowed relationship values should be a small fixed enum such as:

- `continuation`;
- `supporting_work`;
- `temporary_detour`;
- `interruption`;
- `return_to_previous_task`;
- `new_task`;
- `unrelated_or_unknown`.

Do not require a relationship when there is no prior verified task or insufficient evidence.

### 2. Represent task basis honestly

Do not equate “Luna returned a non-null task” with “the user explicitly stated a goal.”

Use a deterministic provenance classification based on the admitted support:

- `explicit_user_goal` only when an owned, user-authored objective or instruction directly supports the task;
- `inferred_from_owned_work` when Luna inferred purpose from owned work evidence;
- `observed_activity_only` when only a step or visible activity is supported;
- `unresolved` when task meaning is not admitted.

This classification may describe evidence provenance. It must not rewrite Luna’s semantic task text.

### 3. Persist one real atomic answer identity

Do not invent synthetic snapshot or task-thread identifiers that have no matching persisted record.

One answer identity must bind:

- session id;
- explicit Continue decision id;
- current frame id and cutoff time;
- observation packet id;
- evidence watermark;
- compact boundary-selection version;
- Luna request id and response id or a stable provider-response envelope when the provider omits an id;
- response-schema version;
- admission-policy version;
- admitted semantic-result id and revision;
- prior verified answer id, when relationship is evaluated;
- scoped correction fingerprint;
- optional target-binding identity.

Persist the admitted compact result as a first-class verified semantic snapshot or evolve the current snapshot store so `strict_target_owner` can prove it. Do not create a second disconnected snapshot architecture.

All React, island, cache, feedback, and opening behavior must refer to this same identity.

### 4. Bind the exact return target to the same answer

Luna must never invent or return raw URLs, document paths, file paths, opaque artifact ids, or application-open commands.

Local code may attach a direct return target only when it proves:

- the locator was observed locally and is still allowed by privacy policy;
- the target belongs to the same semantic task/revision as the admitted answer;
- the target is current enough for the direct-open policy;
- no scoped user rejection applies;
- the target is not merely a support branch or easy-to-open surface;
- the same answer identity is used by the displayed task and the open action.

If task meaning is understood but the target cannot be bound, show the semantic resume point without an open action. This is a valid inspect-only answer, not a reason to substitute a legacy target.

If the current screen is a detour but the prior verified task is still the supported continuation, the return target may belong to the prior task only when the relationship and target ownership are both explicitly proven through the atomic answer record.

### 5. Define field-level partial-answer policy

An answer may remain useful when one field is unavailable, but the UI must not imply more certainty than the admitted fields support.

At minimum:

- a non-null primary task requires valid semantic support;
- a resume point without a primary task may be shown only as observed current work, not as a resolved task;
- last progress and unfinished state must not be combined across different responses or task revisions;
- a next action with no support must be null;
- a validation issue affecting one field nulls that field;
- chronology, privacy, response identity, or packet-identity failure invalidates the whole answer;
- a confidently wrong or user-rejected primary task invalidates task-dependent target opening.

Use simple product statuses such as `resolved`, `partly_resolved`, and `unresolved`, backed by precise diagnostic reason codes.

### 6. Admit only three semantic source classes

Public semantic fields may come from:

1. the admitted Luna answer for the exact atomic identity;
2. a human correction scoped to that exact identity and field;
3. unresolved/null.

They may not come from:

- `visible_legacy_model_answer` or the legacy multi-hypothesis response;
- activity recap;
- work truth;
- current focus or current activity;
- local task-turn prose;
- candidate labels or scores;
- app/window/page titles;
- `verb + object` templates;
- cached legacy Continue decisions;
- React-only or island-only compatibility copy.

Local observations may appear under evidence inspection with explicit observation wording. They may not fill semantic gaps in the primary answer.

### 7. Make React and the native island semantic peers

Behind the explicit development flag, project the same atomic answer into React and the island.

They must agree on:

- primary task;
- resume point;
- last progress;
- unfinished state;
- relationship;
- next supported action;
- resolution/unresolved status;
- whether a target is openable;
- whether a correction is active.

Presentation length may differ. Meaning and confidence may not.

The first screen remains one Continue answer. Diagnostics, alternatives, evidence tables, raw events, screenshots, sessions, and timelines remain inspect-only.

### 8. Keep production cutover out of this phase

The new contract may be exercised through the existing PFTU development path or a narrowly scoped explicit flag.

Do not yet:

- make compact Luna inference the unconditional default;
- remove the legacy model request;
- delete reconciliation code;
- change the normal MFTI script to enable the new path;
- claim the one-request production invariant;
- broaden the release gate.

Prompt 03 owns these actions after this contract passes.

## Automated verification

Add or update focused tests for at least:

1. Honest task-basis classification.
2. Persisted atomic identity round trip.
3. Rejection of synthetic or unpersisted snapshot identity.
4. Rejection of mixed packet, watermark, response, task revision, or correction identity.
5. Field-local nulling without semantic fallback.
6. Whole-answer rejection for privacy, chronology, or identity failure.
7. `next_supported_action` remains null without explicit support.
8. Relationship is null/unknown without a prior verified answer.
9. Direct target attaches only to the exact answer identity.
10. A support-branch target cannot attach as the primary return target.
11. An understood task without a safe locator stays visible and inspect-only.
12. Legacy model, recap, work-truth, focus, task-turn, and candidate prose cannot fill public semantic fields.
13. React and island projections have semantic parity.
14. Scoped correction changes only its exact field and identity.
15. A newer boundary cannot reuse a stale precise answer unless continuity is explicitly verified.

Run at minimum:

```bash
npm run build
cd src-tauri
cargo fmt -- --check
cargo check
cargo test task_truth_v2
cargo test production
cargo test session_island
```

Use valid crate filters and report any necessary adjustment. Run the full Rust suite when shared contracts or persistence schemas change.

## Development live gate

Run at least twelve fresh manual Continue scenarios through the development-gated compact answer path:

1. Resolved coding task with safe exact target.
2. Resolved task with no safe target.
3. Partly resolved current step with no recoverable primary task.
4. Passive page that must stay unresolved.
5. Supporting browser research related to a prior verified task.
6. Temporary detour with a safe return to the prior task.
7. Genuine new task replacing prior work.
8. Completed work with no unfinished state or next action.
9. Waiting for command or agent output.
10. Field-level human correction.
11. Target rejected while semantic task remains valid.
12. React/native-island parity on the same decision.

For each scenario record:

- expected answer written before output;
- session, decision, frame, packet, request, response, semantic result, and target-binding ids;
- admitted semantic fields and sources;
- actual React copy;
- actual island copy;
- target/open result;
- field-level human rating;
- whether any field came from a prohibited source;
- whether all atomic identities matched.

Do not commit private captures or raw provider payloads.

## Required pass gate

All must pass:

- Prompt 01 remains passed on the current code.
- Zero public semantic fields come from local or legacy fallback in the development path.
- Zero mixed atomic identities.
- Zero unsafe or task-inconsistent direct opens.
- Zero confidently wrong primary tasks.
- Every open target is bound to the exact displayed answer identity.
- Understood tasks remain visible when no safe target exists.
- Unsupported next actions remain null.
- React and island have zero semantic disagreements across all twelve scenarios.
- Corrections remain field- and identity-scoped.
- The primary answer is concise and does not expose the legacy forensic hypothesis contract.

If any gate fails, keep the feature development-only and leave the phase incomplete.

## Completion artifact

Create or update:

```text
docs/phases/lean-continue/02-continuation-useful-output-contract-completion-audit.md
```

Include:

- dependency proof;
- dirty-checkout preservation notes;
- before/after field-source map;
- final wire and persistence contract;
- atomic identity and target-binding data flow;
- automated results;
- twelve redacted live rows;
- React/island parity evidence;
- all failed or skipped gates;
- file-level change summary;
- exact final verdict.

Use exactly one final line:

```text
PASS — 03-single-request-production-cutover may begin
```

or:

```text
INCOMPLETE — 03-single-request-production-cutover is blocked
```

Do not pass this phase when the compact answer is still synthetic, inspect-only because identity cannot be proven, mixed with local fallback, or unverified on both product surfaces.

