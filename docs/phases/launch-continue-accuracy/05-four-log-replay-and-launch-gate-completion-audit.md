# LCA-05 Four-Log Replay and Launch Gate — Completion Audit

Date: 2026-07-19

Contract: `05-four-log-replay-and-launch-gate.md`

Scope of this verdict: deterministic implementation and automated verification only. The six normal-product-path scenarios remain user-owned and have not been run.

## 1. Dependency and requirement matrix

| Phase | Required contract carried into LCA-05 | Current proof | Result |
| --- | --- | --- | --- |
| LCA-01 | Task-relevant evidence, chronology, attribution, selected/rejected surfaces, bounded images, and privacy-safe provider input | Boundary A replays all eight cases through the production evidence and request builder. The LCA-01 audit still ends with its exact pass line. | Pass |
| LCA-02 | Model-authored actionable continuation fields, separate completion/current work, qualified causality, and one normal manual request | Boundary B starts from the versioned compact response contract. All supported fields survive, and all wrong/generic fields are removed without local replacement prose. The LCA-02 audit still ends with its exact pass line. | Pass |
| LCA-03 | Field-local monotonic admission, explicit/inferred separation, atomic result identity, correction scope, and independent target status | The real parser, support validator, admission mapper, cache identity, and correction watermark are exercised. Unresolved/refused answers cannot expose a surviving public action. The LCA-03 audit was updated with the completed cache proof and retains its exact pass line. | Pass |
| LCA-04 | One canonical instruction, resume/location context, typed failure states, React/native meaning parity, saved-copy identity, and strict-open safety | Boundary B uses the canonical product projection. Native consumes the same wire shape; strict open additionally requires authoritative `direct_target_ready`. The LCA-04 audit still ends with its exact pass line. | Pass |

The four required prior audit endings were independently checked before implementation. No threshold or expected truth was weakened to pass LCA-05.

## 2. Fixture inventory and provenance

The replay manifest is `src-tauri/tests/fixtures/continue_accuracy/lca-replay-manifest.v1.json`, schema `smalltalk.lca_05.replay_manifest.v1`.

### Critical cases

| LCA case | Fixture | Privacy-safe source label | Source SHA-256 |
| --- | --- | --- | --- |
| LCA-CRIT-01 | `lca_05cd_product_need_review` | `05cd` product-need answer review | `c75cfc4e93f669edfe471a4e823acf9dd02180af609f8d4d396afd378a172bcc` |
| LCA-CRIT-02 | `lca_0d1c_visual_cue_request` | `0d1c` visual-cue task after backend completion | `0665f4b001233a3773cfcd9629ba85d9a3e92b9874bdce79f8ad7398b41db410` |
| LCA-CRIT-03 | `lca_0056_verification` | `0056` implementation-complete/user-verification-remains | `ff4ea8b049c5d8b371b238c2dacf148c0422891bdbfcf344a8ca3edf05796a58` |
| LCA-CRIT-04 | `lca_0e34_unsent_draft` | `0e34` unsent regression draft | `b586978a1516b5ead8deb789689717e35c444769ed56f9ed2188ac53f636048e` |

The raw logs were inspected in place and were not changed or copied. Committed fixtures contain synthetic bounded evidence, not raw conversation content, screenshots, provider payloads, private paths, or credentials.

### Adversarial cases

| LCA case | Existing versioned fixture | Failure class |
| --- | --- | --- |
| LCA-ADV-01 | `capture_button_adjacent_after_support_detour` | A high-engagement adjacent/support pane must not outrank the current attributed request. |
| LCA-ADV-02 | `capture_button_old_completion_visible` | Old successful completion remains prior context beside a newer task. |
| LCA-ADV-03 | `capture_button_fresh_task_only` | A known task with no supported action remains task-known/action-unknown. |
| LCA-ADV-04 | `capture_button_adjacent_before_new_task` | Thin/passive evidence abstains without inventing a generic task or target. |

The manifest parser requires exactly four critical cases, four adversarial cases, unique fixture mappings, and the exact ten response variants. Unknown fields, duplicate cases, missing variants, or mismatched source hashes fail parsing.

## 3. Boundary A — evidence to request

Result: 8/8 cases passed, 0 excluded.

Boundary A uses the existing production replay and request-building path. It checks the fixture’s resolved text, pane/region and conversational attribution, chronological task-turn spans, selected/rejected task evidence, prior completion, current task relationship, public target truth, deterministic identity, and privacy lint. The compact builder then enforces:

- one or two chronological boundaries;
- no more than four prepared images, with no duplicate semantic image identity;
- original and sent dimensions recorded, with a 1,600-pixel maximum long edge;
- structured text no larger than 24 KiB and no more than 6,144 estimated text tokens;
- ownership, privacy, cutoff, region-role, and task-role checks;
- one new deterministic provider envelope per new manual boundary.

This proves that the deterministic harness gives the semantic stage the intended bounded problem. It does not claim that a live provider will interpret real pixels correctly.

## 4. Boundary B — response to product

Result: 40/40 response variants passed, 0 excluded.

Each of the four critical fixtures was exercised through all ten variants:

1. valid resolved-shaped response;
2. valid partly-resolved response;
3. one unsupported semantic field;
4. wrong task citing a real slot;
5. prior completion presented as current;
6. generic next action;
7. confidence inflation;
8. invalid inline citation text;
9. invalid structured output;
10. provider incomplete/empty output.

Every variant ran through the production structured parser, field-support validator, monotonic admission logic, public-answer mapper, canonical product projection, and native wire contract. The assertions prove:

- valid supported task and action fields survive;
- unsupported fields are removed locally without replacement semantics;
- real but non-primary/prior slots cannot authorize the unfinished task;
- wording that contradicts an explicit latest goal is rejected;
- completed-context wording cannot become the unfinished task;
- generic actions and inline diagnostic citations do not reach public copy;
- raw uncertainty is never upgraded;
- unresolved/refused semantic results do not expose a separately surviving action;
- parser and provider failures retain distinct typed copy and a refresh action instead of being called insufficient evidence;
- a frame remains evidence preview only and never becomes a direct target;
- canonical answer identity and React/native meaning remain aligned.

## 5. Launch-accuracy metrics

All exclusions are zero.

| Metric | Numerator | Denominator | Excluded | Rate | Gate result |
| --- | ---: | ---: | ---: | ---: | --- |
| critical evidence-checkpoint pass | 4 | 4 | 0 | 100% | Pass |
| critical semantic-slot pass | 4 | 4 | 0 | 100% | Pass |
| critical first-screen meaning pass | 4 | 4 | 0 | 100% | Pass |
| deterministic response-variant pass | 40 | 40 | 0 | 100% | Pass |
| completed work as active task count | 0 | 40 | 0 | 0% | Pass |
| unrelated-pane primary leakage count | 0 | 40 | 0 | 0% | Pass |
| false insufficient-evidence count | 0 | 40 | 0 | 0% | Pass |
| uncertainty upgrade count | 0 | 40 | 0 | 0% | Pass |
| inferred-as-explicit count | 0 | 40 | 0 | 0% | Pass |
| unsupported next-action count | 0 | 40 | 0 | 0% | Pass |
| supported next-action precision | 8 | 8 | 0 | 100% | Pass |
| status/action contradiction count | 0 | 40 | 0 | 0% | Pass |
| inline diagnostic/citation leakage count | 0 | 40 | 0 | 0% | Pass |
| canonical projection identity mismatch count | 0 | 40 | 0 | 0% | Pass |
| React/island semantic disagreement count | 0 | 40 | 0 | 0% | Pass |
| frame-preview-as-direct-target count | 0 | 40 | 0 | 0% | Pass |
| unsafe open or open-policy bypass count | 0 | 40 | 0 | 0% | Pass |
| one-manual-attempt provider-post maximum | 1 | 1 | 0 | 100% of allowed maximum | Pass |
| automatic reconciliation post count | 0 | 40 | 0 | 0% | Pass |
| background semantic upload count | 0 | 40 | 0 | 0% | Pass |
| privacy fixture violation count | 0 | 12 | 0 | 0% | Pass |
| deterministic replay disagreement count | 0 | 12 | 0 | 0% | Pass |

Automated LCA gate: passed with no violations.

## 6. Four critical first-screen answers

The canonical answer is an instruction followed by bounded resume and location context. Diagnostics remain behind Inspect.

### LCA-CRIT-01

- Instruction: `Continue reviewing the answer from the Does your product solve a real need section.`
- Resume context: `The answer has begun, affirms the need, and continues beyond the visible section.`
- Location: `the conversational answer`
- Action: `View last screen` (evidence preview only; no direct target)

### LCA-CRIT-02

- Instruction: `Return to the Codex visual-cue task and inspect its implementation result.`
- Resume context: `The agent is implementing the answer-linked visual cue while preserving the island output contract.`
- Location: `the Codex visual-cue task`
- Action: `View last screen` (evidence preview only; no direct target)

### LCA-CRIT-03

- Instruction: `Open the latest Continue answer and verify that Show more reveals the linked visual cue.`
- Resume context: `The visual-cue implementation is complete and focused checks passed; The implementation completed and focused checks passed; user verification remains.`
- Location: `the Smalltalk visual-cue verification task`
- Action: `View last screen` (evidence preview only; no direct target)

### LCA-CRIT-04

- Instruction: `Return to the draft and continue the regression report.`
- Resume context: `An unsent regression report is being drafted after the failed result.`
- Location: `the unsent regression-report draft`
- Action: `View last screen` (evidence preview only; no direct target)

## 7. Request, cache, privacy, cost, and performance

Request/cost contract:

- At most two boundaries, four images, 24 KiB structured text, 6,144 estimated text tokens, and a 1,600-pixel image long edge.
- The deterministic response matrix records exactly one provider post per new response envelope, zero automatic reconciliation posts, zero HTTP retries, and zero background uploads.
- No external provider call was made for this audit, so no live token usage or live dollar cost is claimed.

Cache contract:

- A repeated manual action still receives a fresh decision identity and revalidates target safety.
- The already admitted result is reused with `provider_post_count = 0` only when session, packet id, evidence watermark, current frame, model, deterministic request identity, request/response schemas, verifier/admission versions, and live correction watermark all match.
- `repeated_manual_decision_reuses_unchanged_admitted_result_without_provider_post` passed.
- `correction_change_invalidates_admitted_result_reuse` passed.
- Eval/preflight/privacy failures and legacy rows without the complete identity are not reusable.

Privacy result: 12/12 replay fixtures passed privacy lint; the committed manifest contains only bounded labels and hashes for the raw sources.

Deterministic evaluator timing on this machine:

- P50 model-off replay: 251.83 ms.
- P95 model-off replay: 544.36 ms.

That P95 does not satisfy the separately frozen P6 performance budget. It is reported unchanged below and is not hidden by the LCA pass.

## 8. React/native parity and strict-open safety

- React and the native island consume the same `smalltalk.continue_product_projection.v1` fields: answer identity, presentation state, instruction, resume context, location, semantic/task/target state, primary action, inspect availability, and unresolved reason.
- The Rust-to-Swift wire-shape test passed.
- Saved output history stores the exact projection and atomic identity rather than recomputing new semantics.
- Native open eligibility requires authoritative `target_status == direct_target_ready` in addition to the existing decision-id, locator-policy, freshness, and target checks.
- A leftover openable target object with unknown target status cannot create an open action.
- An event-only evidence advance marks the remembered result stale, preserves its prior displayed copy for history, exposes refresh, and blocks open.
- Swift type-check passed with five existing macOS 14 deprecation warnings and no type errors.

No visual, click, live island, or interaction acceptance was run or claimed.

## 9. Existing P6 release verdict

The LCA gate is separate from the P6 100-case release gate. The generated evaluator report leaves P6 `release_gate.passed = false`.

Current P6 evidence:

- 12 evaluated fixtures versus 100 required;
- 0 independently human-reviewed cases;
- no locked holdout evaluation;
- confidence-calibration gate false;
- manual macOS interruption/recovery QA false;
- model-off P95 544.36 ms versus the frozen 204.05 ms regression budget;
- several target/action metrics still have no labeled P6 samples.

No P6 threshold, baseline, fixture truth, or release verdict was changed.

## 10. Automated commands and results

Commands executed:

```text
cd src-tauri && cargo fmt --all -- --check
cd src-tauri && cargo check
cd src-tauri && cargo test
cd src-tauri && cargo run --features eval-binaries --bin continue_accuracy_eval -- --repeat 2 --output /tmp/smalltalk-lca05-report.json
PATH=/Users/bhaskarpandit/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/bin:/opt/homebrew/bin:/usr/bin:/bin npm run build
PATH=/Users/bhaskarpandit/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/bin:/opt/homebrew/bin:/usr/bin:/bin npm run test:webview
swiftc -typecheck src-tauri/macos/SessionIslandPanel.swift
git diff --check
git status --short
```

Recorded results:

- Rust formatting: exit 0 in 1.68 seconds.
- Rust compile check: exit 0 in 8.77 seconds (Cargo reported 8.82 seconds).
- LCA evaluator: exit 0; 12/12 deterministic fixture replays; 40/40 critical response variants; LCA gate passed; P6 gate remained false.
- Frontend build: exit 0 in 2.948 seconds; 34 modules transformed; Vite build phase 733 ms.
- Webview tests: exit 0 in 0.942 seconds; 36 passed, 0 failed, 0 skipped/cancelled/todo; test runner duration 185.415 ms.
- Swift type-check: exit 0 in 5.51 seconds; five existing deprecation warnings, zero type errors.
- `git diff --check`: exit 0 in 0.07 seconds; no whitespace errors.
- `git status --short`: exit 0 in 0.08 seconds; the pre-existing/shared LCA worktree remains intentionally dirty.
- Cache unchanged-evidence proof: 1 passed, 0 failed.
- Cache correction-invalidation proof: 1 passed, 0 failed.
- Native projection wire-shape proof: 1 passed, 0 failed.
- Strict authoritative-target contradiction proof: 1 passed, 0 failed.
- Native stale-output/open-block proof: 1 passed, 0 failed.

The first full `cargo test` attempt ran 905 library tests: 898 passed, 4 failed, and 3 were ignored. The four failures were localized to two valid flattened-evidence expectations, one LCA mutation assertion that compared two absent values, and one strict-open fixture missing the newly authoritative target status. The implementation/test fixtures were corrected and each exact failed test then passed individually (1 passed, 0 failed, 904 filtered out per command). A second and final full-suite result is recorded below; no third broad run is permitted by the bounded test policy.

The second and final `cargo test` attempt passed: exit 0; 902 library tests passed, 0 failed, and 3 were ignored (905 total). The library test phase took 143.95 seconds after 23.00 seconds of compilation, for 166.95 seconds reported together. The main binary and documentation test targets each contained zero tests and completed successfully; they are not counted as behavioral proof.

The first focused LCA attempt exposed a mechanically real wrong-task slot surviving admission. The task-support policy was corrected to require a current-task evidence role and explicit goal wording agreement. The next evaluator attempt exposed an adversarial fixture with no user-action event; the harness was corrected to accept any eligible real slot for that mutation. The following evaluator result exposed a real status/action contradiction after task rejection; the public mapper was corrected so unresolved/refused results cannot expose an action. The final incremental evaluator passed. These failures are recorded rather than hidden.

## 11. Documentation changes

- `PRODUCT.md` now describes the normal compact semantic Continue path, monotonic admission, canonical one-instruction projection, diagnostic-only legacy surfaces, the user-owned LCA gate, and the still-false P6 release gate.
- `docs/full-engine-flow.md` now documents the normal manual one-request flow and the two replay boundaries.
- `docs/current-island-ui-ux.md` now names the canonical projection fields and the shared React/native interface, including typed failure and strict-target behavior.
- The fixture README documents the LCA manifest, eight Boundary A cases, forty Boundary B variants, privacy-safe provenance, and the feature-enabled evaluator command.
- The LCA-03 completion audit now contains the completed atomic-result cache retention and correction-invalidation proof.

## 12. User-owned manual verification script

Run the normal product path:

```bash
npm run tauri dev
```

Do not use an evaluation-only database. Run the scenarios in this order.

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

For every scenario, record privately:

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

Do not commit screenshots, captures, provider payloads, private IDs, or full private paths.

## 13. Manual results

| Scenario | Result |
| --- | --- |
| Manual 1 — response review | Not run — user-owned after automated LCA completion |
| Manual 2 — new task after completion | Not run — user-owned after automated LCA completion |
| Manual 3 — implementation complete, verification remains | Not run — user-owned after automated LCA completion |
| Manual 4 — unsent draft | Not run — user-owned after automated LCA completion |
| Manual 5 — genuinely thin evidence | Not run — user-owned after automated LCA completion |
| Manual 6 — React/native and target parity | Not run — user-owned after automated LCA completion |

## 14. Remaining limitations and prohibited claims

- Boundary A proves deterministic evidence selection and request formation, not live model interpretation.
- Boundary B proves parser/admission/product corruption resistance, not real visual perception.
- No external provider, live capture, app-switching soak, restart recovery, mouse/keyboard interaction, visible island inspection, or end-to-end target opening was performed.
- The six manual scenarios are not passed merely because deterministic fixtures passed.
- The P6 release gate remains false.
- Therefore this audit does not claim visual approval, native-island interaction approval, live-provider approval, broad P6 release approval, manual launch approval, or general launch readiness.

AUTOMATED PASS — LCA implementation is ready for user-owned live verification; launch is not yet manually approved

## 2026-07-19 correction from LCA-06 live runtime recovery

The four-log replay began after task-relevant evidence had already been shaped well enough for packet and admission stages. Live manual Continue later showed that this prerequisite was not reliably produced: recent rows had no admitted latest-user goal, no current agent state, and no selected current task turn. Live target attachment and capture contention also exercised boundaries absent from the replay.

The earlier replay remains a valid component regression set. It is not a complete production-chain proof, an LCA-06 automated pass, a product pass, or launch approval. The P6 release gate remains closed. LCA-06 adds privacy-safe live-shaped cases and focused recovery tests; its remaining full-chain and user-owned evidence is recorded in the LCA-06 completion section.
