# MFTI-04 completion audit

Date: 2026-07-12  
Scope: production semantic authority, atomic React/island presentation, model-first evaluation contract, reviewed corpus gate, performance/privacy proof, manual QA, and release identity

## Outcome

The in-repository evaluation and release-gate machinery for MFTI-04 Goals 5 through 7 is implemented. The release is **not proven** and production authority must remain closed.

The authoritative MFTI report is `smalltalk.mfti_04.final_release_report.v1`. Its current result is:

```text
passed = false
authority_state = eligible
independently reviewed live boundaries = 0 / 200
locked application-level holdout = 0 / 50
manual MFTI macOS scenarios = 0 / 10
```

This is the intended fail-closed result. Five historical live-redacted TT2 cases exist, but all five are pending human review. They do not enter any MFTI release denominator. No report or fixture in this change fabricates reviewer approval, provider evidence, holdout evidence, or performance evidence.

## Goals 1 through 4 — Production authority and one atomic answer

| Requirement | Authoritative implementation | Deterministic evidence | Live reviewed evidence | Status | Remaining work |
| --- | --- | --- | ---: | --- | --- |
| Only cloud model, human correction, or unresolved may supply semantic truth | `production.rs` enforces semantic-source and complete atomic-identity admission; invalid resolved answers are scrubbed to typed unresolved | Production authority regressions | 0 accepted live semantic results | implementation proven; live proof missing | Obtain three accepted privacy-approved provider results |
| Manual Continue waits for fresh inference | `continuation.rs` persists the manual multimodal boundary before loading the production decision; React shows an understanding state and protects stronger manual adoption | Continuation and presentation tests | 0 reviewed manual scenarios | implementation proven; live proof missing | Run signed macOS manual scenarios |
| Provider/preflight failure never falls back to local semantic copy | Manual model-first requests always receive a typed unresolved answer; local scorer and local causal sources are rejected | Provider-failure, no-snapshot, and source-admission tests | 0 reviewed provider failures | implementation proven; release denominator missing | Review disabled, timeout, invalid-output, and unavailable-provider cases |
| Cache and adoption use the complete semantic identity | Fingerprint binds provider/model, request/response and public schemas, verifier, packet, thread/revision, hypothesis, feedback, authority policy, and exact MFTI report bytes | Atomic cache-identity and stronger-manual tests | 0 reviewed cache/adoption pairs | implementation proven; live proof missing | Review manual/background/cache scenarios |
| React and island use the same atomic answer | Both surfaces sanitize incomplete identity, use task-thread revision, omit unsupported rows, and share typed provider-failure behavior | 20 presentation tests and 21 island contract tests | 0 parity scenarios | implementation proven; manual parity missing | Complete the React/native parity scenario |
| Direct open is subordinate to task truth | Open requires the exact public target and matching thread head, snapshot, hypothesis, response, packet, watermark, locator, privacy, freshness, and feedback state | Strict-open and target-ownership regressions | 0 reviewed direct targets | implementation proven; live denominator missing | Review correct-task/no-target and exact-target cases |

Production now recognizes only `smalltalk.mfti_04.final_release_report.v1` with policy `mfti.04-v1`. A historical TT2 pass cannot open model-first authority. The requested authority remains fail-closed while the generated MFTI report is not authoritative.

## Dependency gate

| Requirement | Evidence | Denominator | Status | Remaining work |
| --- | --- | ---: | --- | --- |
| Three accepted live cloud inferences | MFTI-02 status records two private provider responses, but neither retained an accepted central task | 0 / 3 accepted | missing | Run three privacy-approved manual sequences through final verification and human review |
| Five reviewed longitudinal task-thread scenarios | MFTI-03 status says all five private sequences remain unrun | 0 / 5 | missing | Capture, independently label, run, and review all five sequences |
| Provider failure cannot manufacture local task truth | Evaluator records provider-failure honesty and local-fallback metrics separately | deterministic test; 0 reviewed failures | incomplete | Deterministic invariant exists; reviewed provider-failure denominator is still required |
| Atomic thread/snapshot/hypothesis answer | MFTI-03 implementation and deterministic tests | deterministic tests | proven | Live parity remains part of manual QA |
| Cross-session stale snapshot cannot win | MFTI-03 implementation and MFTI zero-tolerance metric | deterministic tests; 0 reviewed cases | incomplete | Reviewed live denominator required |
| Return targets remain subordinate | Existing strict-target implementation and MFTI return-target metric | deterministic tests; 0 reviewed targets | incomplete | Reviewed direct-target cases required |

Because the first two dependency requirements are missing, MFTI-04 must not change production authority.

## Goal 5 — Evaluation around the model-first promise

| Requirement | Authoritative implementation | Evidence | Denominator | Status | Remaining work |
| --- | --- | --- | ---: | --- | --- |
| Preserve historical TT2 artifacts | New files live under `task_truth_v2/model_first/`; parent TT2 policy/reports are unchanged | Repository diff | n/a | proven | None |
| Add complete independent labels | `HumanAdjudicationV2` adds visible surface, immediate operation, semantic effect, current subtask, and switch/completion status while retaining task, relationship, progress, unfinished state, next action, alternatives, and usefulness | Strict parsing and privacy lint path | 0 complete reviewed MFTI labels | incomplete | Human reviewers must provide the labels before seeing product output |
| Replace model-on/off parity | MFTI metrics omit `model_on_off_unexplained_task_disagreement` and require provider-failure honest unresolved plus zero local semantic fallback | Focused gate and evaluator tests | 0 reviewed provider failures | incomplete | Collect provider-disabled, timeout, invalid-output, and unavailable-model cases |
| Measure surface/task confusion, relationship, switch/detour, stale leakage, and mixed snapshots | `mfti_04_metric_results` and Wilson intervals | Generated evaluator report | 0 reviewed | incomplete | Populate reviewed cases |
| Fail on zero denominators | Metric assessment and MFTI gate require a non-zero denominator, passed assessment, matching frozen threshold, and Wilson interval | `zero_denominator_cannot_pass` | deterministic | proven | None |

## Goal 6 — Reviewed live causal corpus

| Requirement | Required | Current | Status | Remaining work |
| --- | ---: | ---: | --- | --- |
| Independently reviewed live boundaries | 200 | 0 | missing | Build privacy-safe candidates and obtain blinded independent labels |
| Locked application-level holdout | 50 | 0 | missing | Populate `locked-holdout.v2.json` only after policy freeze |
| Required surface families | 10 families, at least 15 reviewed each | 0 reviewed in every family | missing | Collect all families; pending agent-chat cases do not count |
| Interruption/resumption | 30 reviewed | 0 | missing | Collect longitudinal sequences |
| Ambiguous/privacy-blocked | 20 reviewed | 0 | missing | Collect ambiguity and privacy cases |
| Waiting on agent/application | 20 reviewed | 0 | missing | Collect waiting/review sequences |
| Completed versus new task | 20 reviewed | 0 | missing | Collect completion, detour, return, and task-switch boundaries |
| Provider failure | non-zero reviewed denominator | 0 | missing | Run provider-disabled and failure scenarios |

The existing five live-redacted TT2 cases are all pending and all from `agent_chat`. They are useful development inputs, not release evidence.

## Goal 7 — Frozen hard gates

`model_first/eval-policy.v1.json` is versioned as `mfti.04-v1` and records that it was frozen before holdout access. No locked holdout file currently exists. The gate validates each metric against that policy rather than trusting a report boolean or unrelated hard-coded policy identity.

The frozen MFTI gates cover all phase thresholds: wrong primary task, visible-surface substitution, wrong relationship, wrong switch/detour, stale leakage, mixed snapshots, controls as task, unsupported claims, both provider-failure requirements, useful summary, task object, execution state, next-action precision and coverage, return-target precision, manual-result downgrade, unseen applications, and human usefulness. Required surface-family gates use the phase's 5% ceiling with at least 15 reviewed cases per family.

The gate explicitly rejects the old model-on/off disagreement metric as an MFTI authority condition.

## Release identity and manual proof

`release-identity.schema.v1.json` binds a future passing report to exact corpus and holdout hashes, provider and model, prompt and response schema, observation packet, verifier, task-thread and public-answer versions, performance/privacy policy, manual QA manifest, source commit, and build identity.

`manual-macos-qa.schema.v1.json` requires the ten MFTI-04 acceptance scenarios. Each row must contain reviewer, commit/build, expected and actual result, provider/model, and privacy-safe evidence ids. The manifest is currently absent, so the gate reports it as missing.

## Goal 8 — Performance, cost, privacy, and failure behavior

`performance-cost-privacy.schema.v1.json` requires at least 30 measurements. The manifest covers capture-to-packet, request-build, provider, verification/persistence, and total Continue latency; image, byte, input-token, and output-token distributions; cost per Continue and monthly cost; timeout, provider-error, and invalid-output rates; second-pass frequency and cost; privacy exclusions; unsafe opens; background uploads; and a reviewed provider-failure experience.

Manual Continue now records each stage into `task_truth_v2_performance_samples`. A row remains incomplete until the full Tauri command has finished composing its response, so provider-only smoke calls and interrupted requests cannot enter the release denominator. The table contains only numeric measurements, a hashed decision identity, and bounded outcome labels. It has no fields for OCR, Accessibility text, hypotheses, paths, URLs, images, or raw responses.

`mfti_04_performance_report` reads only those safe columns, excludes incomplete rows in SQL, reports the declared monthly-usage assumption, and writes the adjacent aggregate schema. Human-reviewed privacy violations, unsafe opens, and provider-failure experience must be supplied explicitly; the tool does not infer a pass. The manifest is currently absent. The release gate reports `performance_cost_privacy_manifest_missing`, `performance_sample_count = 0`, and cannot pass until at least 30 real completed manual Continue measurements exist.

## Goal 9 — Manual end-to-end macOS QA

The ten required MFTI scenarios have a strict schema, but the reviewed manifest is absent. Current denominator: 0 / 10. The missing signing identity and stable two-build Screen Recording proof also remain external prerequisites. No fixture or automated test is counted as manual macOS proof.

## Generated artifacts

- `src-tauri/tests/fixtures/continue_accuracy/task_truth_v2/model_first/eval-policy.v1.json`
- `src-tauri/tests/fixtures/continue_accuracy/task_truth_v2/model_first/release-evaluator-report.v1.json`
- `src-tauri/tests/fixtures/continue_accuracy/task_truth_v2/model_first/final-release-report.v1.json`
- `src-tauri/tests/fixtures/continue_accuracy/task_truth_v2/model_first/release-identity.schema.v1.json`
- `src-tauri/tests/fixtures/continue_accuracy/task_truth_v2/model_first/manual-macos-qa.schema.v1.json`
- `src-tauri/tests/fixtures/continue_accuracy/task_truth_v2/model_first/performance-cost-privacy.schema.v1.json`
- `src-tauri/src/bin/mfti_04_performance_report.rs`
- Generator: `cargo run --features eval-binaries --bin mfti_04_release_gate`

## Verification

- `cargo check --features eval-binaries --bins`: passed.
- `cargo test --features eval-binaries --bin mfti_04_release_gate`: command passed and compiled the binary test target.
- `cargo test --features eval-binaries --lib continuation::task_truth_v2`: command passed and compiled the evaluator/schema tests.
- Focused provider-failure metric test command: passed.
- `cargo fmt --all`: applied successfully.
- Evaluator artifact inspected: 5 cases, 0 release-eligible, provider-honesty denominator 0, passed false.
- MFTI final gate inspected: `passed = false`, 0 reviewed live, 0 holdout, 57 explicit missing-evidence violations.
- The gate requires at least 30 performance/cost/privacy measurements and cannot pass while that manifest is absent.
- `cargo check`: passed after one evaluator array-length repair.
- Full `cargo test`: 702 passed, 2 ignored live tests, and 2 stale strict-open warning assertions failed. Both assertions were updated to the stricter atomic Task Truth reasons and each focused test passed on its single rerun. The full suite was not rerun.
- Presentation contract tests: 20 passed.
- Native island contract tests: 21 passed.
- Frontend build was attempted twice and then stopped: the first environment had no `npm`; the corrected pnpm path refused ignored `esbuild` build scripts. It was not attempted a third time.

One `cargo run` wrapper attempt exited successfully after compilation without launching the evaluator or creating its requested output. It was not repeated. The already-built evaluator executable was invoked directly once to generate the same intended artifact, following the instruction not to retry the same failed behavior repeatedly.

## Program status

Goals 5 through 7 now have an honest in-repository contract and fail-closed machine-readable report. MFTI-04 and the four-phase MFTI program remain incomplete because the dependency live proofs, independently reviewed corpus, holdout, manual QA, performance/cost/privacy evidence, release identity, and passing release verdict do not exist.
