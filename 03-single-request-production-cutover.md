# Prompt 03 — Cut Production Continue Over to One Luna Request

## Codex implementation prompt

Implement this phase completely in the current `smalltalk` repository. This is the third and final ordered implementation session. Its job is to make the proven compact Luna boundary and atomic continuation answer the only semantic path for explicit production Continue.

This phase removes the behavior demonstrated by `rsp1/` and `rsp2/`: one large interpretation request followed by another large reconciliation request. A new evidence boundary may produce at most one provider POST and at most one Luna semantic result.

## Hard dependency gates

Read:

```text
docs/phases/lean-continue/01-compact-semantic-episode-input-completion-audit.md
docs/phases/lean-continue/02-continuation-useful-output-contract-completion-audit.md
```

Stop unless their final lines are exactly:

```text
PASS — 02-continuation-useful-output-contract may begin
PASS — 03-single-request-production-cutover may begin
```

Independently verify both audits against the current checkout. Do not cut over if:

- the compact Luna input gate is based only on fixtures;
- live/held-back semantic review is absent;
- any confidently wrong primary task remains;
- late evidence can enter the request;
- the answer/target atomic identity is synthetic or unpersisted;
- React and island parity is unproven;
- local fallback still fills semantic fields.

If a dependency has regressed, repair that earlier phase within its original scope and update its audit honestly before attempting cutover.

## Product outcome

For an explicit manual Continue:

```text
new current evidence boundary
        -> zero requests when unchanged and safely reusable
        -> otherwise exactly one compact POST to gpt-5.6-luna
        -> deterministic local admission
        -> one atomic Continue answer or typed unresolved
```

There is no second semantic reconciliation request. There is no legacy full-packet semantic fallback. There is no stronger-model fallback. There is no local semantic replacement when Luna or validation fails.

## Preserve and audit the current dirty checkout

Before editing:

1. Run `git status --short`.
2. Read the complete current diff.
3. Trace one manual Continue from `src/App.tsx` through the Tauri command, current-frame capture, observation-packet construction, semantic inference, persistence, production projection, cache adoption, React, island, and open action.
4. Inspect all provider-call sites with `rg` rather than assuming the known paths are exhaustive.
5. Inspect:
   - `src-tauri/src/continuation/task_truth_v2.rs`
   - `src-tauri/src/continuation/task_truth_v2/model.rs`
   - `src-tauri/src/continuation/task_truth_v2/verifier.rs`
   - `src-tauri/src/continuation/task_truth_v2/semantic_probe.rs`
   - `src-tauri/src/continuation/task_truth_v2/production.rs`
   - `src-tauri/src/continuation.rs`
   - `src/continuePresentation.ts`
   - `src/App.tsx`
   - `src-tauri/src/session_island.rs`
   - `src-tauri/macos/SessionIslandPanel.swift`
   - `scripts/tauri-dev-pftu.sh`
   - `scripts/tauri-dev-mfti.sh`
6. Reinspect the supplied `rsp1/` and `rsp2/` logs and retain their measurements as the regression baseline. Do not commit those logs.

Keep correct uncommitted code. Do not reset or overwrite user changes.

## Proven legacy behavior to eliminate

Verify these facts in the current checkout:

- The legacy model builder serializes the full `ObservationPacketV2`, prior hypotheses, prior threads, evidence catalogue, and a large multi-hypothesis response schema.
- A verifier conflict such as `ax_ocr_visual_conflict` can construct a second reconciliation request containing the same packet plus the first response and verdicts.
- The supplied first request used 53,301 input tokens.
- The supplied second request used 58,549 input tokens.
- Both returned two ambiguous hypotheses.
- Both left `possible_next_action` and `return_anchor_record_id` null.
- The second request did not materially improve continuation usefulness.
- The normal MFTI development path can disable the compact PFTU probe and therefore still reach the legacy request.
- Current in-progress code may already let the compact probe “own” an attempt when enabled, but ownership is not equivalent to a complete production cutover.

The goal is not to tune reconciliation. The goal is to make it unreachable from production Continue.

## Required implementation

### 1. Make compact Luna inference the only manual semantic path

For every explicit Continue with new eligible evidence:

- build the Prompt 01 compact boundary;
- use the Prompt 02 lean response schema and atomic answer identity;
- call `gpt-5.6-luna` only;
- admit the response deterministically;
- persist one result;
- route that result or typed unresolved to all public surfaces.

Remove feature-flag combinations in which normal manual Continue silently falls back to the legacy full-packet request. A development flag may disable cloud inference entirely, but disabling compact inference must produce a typed disabled/unresolved result rather than invoke legacy semantics.

Update development scripts so the normal MFTI/production-like workflow cannot explicitly disable the only supported semantic path while claiming authority eligibility.

### 2. Enforce exactly one provider POST per new boundary

The product invariant is stricter than “one semantic interpretation after retries.” For an explicit Continue boundary that requires inference:

```text
provider_post_count <= 1
successful_semantic_response_count <= 1
semantic_reconciliation_count == 0
```

Configure the manual Continue provider call with zero automatic HTTP POST retries. A timeout, connection failure, provider rejection, invalid response, or validation rejection returns a typed unresolved result for that attempt. The user may explicitly request Continue again after new evidence or an intentional retry action, which creates a new auditable attempt.

Do not invoke another model because:

- AX and OCR disagree;
- a field lacks support;
- the answer is partial;
- the validator rejected a claim;
- another model might perform better;
- a legacy answer is available;
- the response has no target.

Keep counters separate:

- `provider_post_count` — actual HTTP POST executions;
- `semantic_response_count` — provider responses parsed as semantic output;
- `semantic_reconciliation_count` — must remain zero;
- `cache_reuse_count` — no provider call;
- user-initiated attempt number.

Do not label `request_audit.is_some()` as proof that a provider request occurred. Persist the actual transport execution count.

### 3. Remove production reconciliation and legacy fallback

Make these unreachable from explicit production Continue:

- the large legacy `build_multimodal_request` path;
- conflict-triggered semantic reconciliation;
- second-pass winner selection;
- `visible_legacy_model_answer` public fallback;
- legacy Task Truth snapshot prose filling a compact answer;
- any local activity/recap/work-truth fallback filling semantic fields.

Prefer deleting dead production branches when safe. If legacy code must remain temporarily for offline diagnostics or historical decoding, isolate it behind an explicitly named non-production replay/test interface and prove it cannot be called by manual Continue, startup adoption, background refresh, React, island, or open actions.

Do not preserve a misleading fallback merely to keep an old test green. Update tests to the new product contract while retaining historical decoding only where genuinely required.

### 4. Reuse unchanged evidence without another request

Before provider execution, compare the full atomic evidence identity required by Prompt 02.

When the evidence boundary, admitted policy versions, correction state, and model/schema identity are unchanged:

- make zero provider POSTs;
- reuse the existing verified answer;
- report `no_new_evidence` or an equivalent typed cache-reuse reason;
- keep the same semantic response identity;
- revalidate target freshness before allowing an open.

Do not reuse merely because the session id, app, window, or frame count matches. A new current frame, changed owned content, changed action result, changed privacy state, or changed correction state may require a new boundary.

A newer unresolved boundary must not silently display an older precise answer unless continuity and safe reuse are explicitly proven by the atomic identity policy.

### 5. Fail honestly without fallback

At minimum preserve distinct typed states for:

- current-frame capture failure;
- stale current frame;
- privacy blocked;
- request not built;
- credentials missing;
- Luna unavailable;
- provider timeout;
- transport failure;
- provider rejection;
- empty output;
- structured parse failure;
- support-slot validation failure;
- task not recoverable;
- unchanged evidence reuse.

React and the island must show the same simple user-facing meaning for each state. Diagnostic detail remains inspectable.

Do not convert a provider or validator failure into a local task label, legacy model answer, stale cached task, or openable target.

### 6. Keep background behavior local and non-semantic

Background/startup work may:

- capture allowed local evidence;
- construct local indexes;
- prepare a boundary candidate;
- adopt an already verified unchanged answer;
- validate target freshness.

It may not:

- upload screenshots;
- make a Luna request;
- run legacy semantic inference;
- create a new public semantic answer;
- reconcile a prior model response.

Only explicit Continue or an explicitly authorized test/replay command may perform cloud semantic inference.

### 7. Preserve exact-target safety

Cutover must not weaken Prompt 02 target binding.

- No target is safer than a mismatched target.
- An understood task may remain inspect-only.
- A target must match the exact atomic answer and remain locally openable.
- Reused semantic answers require fresh target validation.
- A support surface is not the default return target.
- A failed semantic attempt cannot retain an older direct-open action unless the reuse contract explicitly proves the older answer is still current.

### 8. Make release state truthful

Do not claim broad production readiness from compilation or fixture tests.

The cutover gate must report separately:

- compact input correctness;
- Luna semantic accuracy;
- answer/target identity correctness;
- one-request transport proof;
- fallback removal;
- React/island parity;
- live exact-resume usefulness.

If any required live gate is missing, retain a clearly named development authority state and mark the completion audit incomplete.

## Automated verification

Add or update tests proving at least:

1. One new eligible boundary executes exactly one provider POST.
2. The provider transport is configured with zero automatic POST retries for manual Continue.
3. AX/OCR conflict does not execute reconciliation.
4. Partial or validation-rejected output does not execute another model call.
5. Provider timeout/failure does not execute a fallback call.
6. Model unavailability does not select another model.
7. Compact-disabled mode produces typed unresolved rather than legacy inference.
8. Unchanged atomic evidence executes zero provider POSTs and reuses the same response identity.
9. Changed evidence executes one new request.
10. A newer unresolved boundary defeats unsafe stale answer adoption.
11. Legacy full-packet request and `visible_legacy_model_answer` are unreachable from manual Continue.
12. Local recap, work truth, focus, task turn, and candidate prose cannot fill semantic fields.
13. Background/startup paths execute zero provider calls.
14. React and island consume the same atomic decision.
15. Target opening remains bound to the same answer identity after fresh inference and cache reuse.
16. Request-size caps remain enforced after cutover.
17. Actual transport counters distinguish built request, executed POST, parsed response, and cache reuse.

Run at minimum:

```bash
npm run build
cd src-tauri
cargo fmt -- --check
cargo check
cargo test
```

Also run any existing webview, island, continuation, Task Truth, cache, feedback, and open-safety tests relevant to the changed path. Report exact commands, failures, fixes, and any test that could not run.

## Live cutover proof

Run at least twelve fresh manual Continue scenarios using the production-like Luna configuration. Expected meaning must be written before viewing output.

Include:

1. Resolved task with exact openable resume target.
2. Resolved task with no safe target.
3. Honest unresolved passive page.
4. Supporting research tied to prior work.
5. True detour and return to prior work.
6. New task replacing prior work.
7. Provider timeout or controlled transport failure.
8. Structured or validation failure without fallback.
9. Repeated Continue with unchanged evidence.
10. Repeated Continue after meaningful changed evidence.
11. React/island parity.
12. A reconstruction of the `rsp1`/`rsp2` workflow involving Codex/ChatGPT, browser/API logs, and adjacent work surfaces.

For every scenario record:

- session, decision, frame, boundary, packet, request, response, answer, and target-binding identities;
- expected and actual semantic fields;
- provider POST count;
- semantic response count;
- reconciliation count;
- cache reuse count;
- input/output/total tokens;
- React copy;
- island copy;
- target/open result;
- field-level human rating;
- whether any prohibited fallback appeared.

For scenario 12, prove that only one provider response log is created for the new evidence boundary. Compare its request bytes, tokens, schema size, and output usefulness with the supplied `rsp1` and `rsp2` baseline.

Do not commit private screenshots, databases, raw logs, paths, URLs, API keys, or provider payloads.

## Required pass gate

Every item must pass:

- All Prompt 01 and Prompt 02 gates remain passed.
- All semantic calls use `gpt-5.6-luna`.
- Every new eligible manual boundary executes exactly one provider POST.
- Every unchanged reusable boundary executes zero provider POSTs.
- Semantic reconciliation count is zero everywhere.
- No automatic transport retry executes a second POST.
- No legacy full-packet semantic request is reachable from production Continue.
- No legacy or local semantic fallback is visible publicly.
- Provider and validation failures remain typed unresolved.
- Zero confidently wrong primary tasks across the live corpus.
- Zero React/island semantic disagreements.
- Zero mixed answer/target identities.
- Zero unsafe or task-inconsistent opens.
- The `rsp1`/`rsp2` reconstruction creates one compact log, not two large logs.
- The primary card tells the user the task and useful resume point without exposing forensic internals.
- Broad verification passes, or every remaining failure is proven unrelated and documented without weakening the gate.

If any gate fails, do not call the cutover complete. Keep the release state honest and continue within the failed phase.

## Completion artifact

Create or update:

```text
docs/phases/lean-continue/03-single-request-production-cutover-completion-audit.md
```

Include:

- dependency verification;
- dirty-checkout preservation notes;
- before/after provider-call graph;
- proof that legacy and reconciliation paths are unreachable;
- transport retry configuration and actual call counters;
- unchanged-evidence reuse proof;
- fallback source audit;
- automated command results;
- twelve redacted live rows;
- `rsp1`/`rsp2` reconstruction comparison;
- React/island parity and open-safety proof;
- every failed or skipped gate;
- file-level change summary;
- exact final verdict.

Use exactly one final line:

```text
PASS — lean Luna Continue cutover is complete
```

or:

```text
INCOMPLETE — lean Luna Continue cutover is not proven
```

Do not write `PASS` unless the live one-POST invariant, zero-reconciliation invariant, zero-fallback invariant, semantic accuracy gate, target identity gate, and product-surface parity are all proven against the current build.

