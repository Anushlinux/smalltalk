# Task Truth v2.05 Completion Audit

Date: 2026-07-11  
Scope: task-first production contract, authority/rollback policy, shared React/island presentation, scoped feedback, and final release gate

## Outcome

The TT2-05 production machinery is implemented, but Task Truth v2 is not released and is not authoritative.

The final report is `smalltalk.task_truth_v2.final_release_report.v1`. Its authoritative field is `passed`. The current report says `passed = false` and `authority_state = eligible`. The authority policy also defaults to `shadow`; even an explicit `authoritative` request is reduced to `eligible` unless a configured final report says `passed = true` and has no violations.

The final gate does not trust that boolean by itself. It independently requires the exact report schema and frozen policy, 200 release-eligible live cases, 50 release-eligible locked holdout cases, all required slices, at least 15 evaluated cases in every required surface family, all 13 TT2-05 semantic metrics with non-zero denominators, Wilson 95% confidence intervals for overall and per-surface gates, all 14 named manual scenarios, a separately frozen baseline-linked budget policy, measured performance/cost/privacy evidence, and every zero-tolerance count at zero. The frozen pre-holdout baseline and holdout-enabled release evaluator are separate artifacts. The budget policy must bind to the exact SHA-256 identity of the baseline bytes. Adversarial tests prove that a hand-authored `passed: true`, a passed assessment without a denominator, a missing per-surface interval, a partial manual manifest, a self-declared performance pass, a budget bound to another baseline, or a budget above the architectural request cap cannot open authority.

This is required behavior, not a partial-success redefinition. The current corpus contains five pending live-redacted development cases, zero independently reviewed release cases, zero locked holdout cases, and one of ten required surface families. The performance/cost/privacy and 14-scenario manual macOS manifests do not exist. No metric with a human denominator can be claimed as passed.

## Dependency audit

| Requirement | Authoritative implementation evidence | Test or live proof | Denominator | Status | Remaining remediation |
| --- | --- | --- | ---: | --- | --- |
| TT2-01 causal containment | `capture.rs`, `task_turn_evidence.rs`, shared control policy, quality-dominant adoption | TT2-01 audit and regression suites | deterministic tests | proven | None for TT2-01 scope |
| TT2-02 frozen corpus/evaluator | `task_truth_v2.rs`, corpus builder, `eval-policy.v1.json` | Five path-C replays | 5 pending; 0 reviewed | incomplete | Independently review and expand live corpus; add locked holdout |
| TT2-03 packets/snapshots/checkpoints | `observation_packet.rs`, `task_snapshot.rs`, `checkpoint.rs`, `selection.rs` | Task Truth v2 tests | deterministic tests | proven | None for TT2-03 scope |
| TT2-04 resolver/verifier | `model.rs`, `verifier.rs`, multimodal shadow audit | Adversarial deterministic tests | deterministic tests; no live provider smoke test | proven | Live provider evidence remains a release input, not an architecture blocker |

## TT2-01 through TT2-04 numbered-goal matrix

This expands the dependency rows above to the numbered-goal granularity required by TT2-05. “Proven” here means that the implementation contract and its deterministic proof exist. It does not turn architecture proof into release proof. Rows that require independently reviewed or live evidence remain incomplete.

| Phase / goal | Requirement | Authoritative implementation evidence | Test or live proof | Metric denominator | Status | Remaining remediation |
| --- | --- | --- | --- | ---: | --- | --- |
| TT2-01.1 | Link committed typing to the post-action frame without storing typed text | `capture.rs` typing-burst association fields and bounded recovery | Capture migration, association, cross-window, and uncommitted-burst tests | deterministic cases | proven | None |
| TT2-01.2 | Replace a causal boolean with an evidence-linked result | `task_turn_evidence.rs` causal attribution object | Task-turn evidence suite | deterministic cases | proven | None |
| TT2-01.3 | Keep historical boundaries history-only | Task-turn prior-boundary eligibility rules | Accuracy and task-turn regressions | deterministic cases | proven | Live error rate remains part of TT2-05 evaluation |
| TT2-01.4 | Make controls ineligible as authored goals | Shared control/actionability policy | Session-013 and control-selection adversarial tests | deterministic cases | proven | Live control/navigation denominator remains missing |
| TT2-01.5 | Return typed ambiguity when no current task is supported | `no_clear_current_task` backend, React, and island behavior | Backend, webview, and island tests | deterministic cases | proven | Human usefulness denominator remains missing |
| TT2-01.6 | Make manual/background adoption quality-dominant | Shared React/island adoption policy | Webview and island downgrade tests | deterministic cases; 0 live pairs | incomplete | Collect reviewed live manual/background pairs |
| TT2-02.1 | Version a strict Task Truth fixture contract | `task_truth_v2.rs`; v2 fixture schemas | Strict parsing and privacy lint tests | 5 pending fixtures | proven | Expand to the release corpus |
| TT2-02.2 | Build a privacy-safe local corpus builder | Audit importer, hashes, review status, and redaction lint | Importer and privacy tests | 5 pending fixtures | proven | Run builder on independently reviewed live boundaries |
| TT2-02.3 | Make session-013 the first authoritative live-shaped family | `session-013-family.v2.json` | Five deterministic decision-boundary replays | 5 pending; 0 independently reviewed | incomplete | Independent review is still required |
| TT2-02.4 | Define human adjudication independently of product output | Fixture review contract and required human labels | Parser rejects incomplete committed review | 0 reviewed | incomplete | Obtain independent labels; feedback cannot self-promote to gold |
| TT2-02.5 | Use application-level partitions and locked holdout access | `eval-policy.v1.json` partition and access rules | Holdout default-deny tests | 0 holdout | incomplete | Populate and lock at least 50 holdout cases |
| TT2-02.6 | Build the three-path shadow evaluator | Path A/B/C replay in `task_truth_v2.rs` | Five deterministic path comparisons | 5 pending cases | proven | Run it on the full reviewed release corpus |
| TT2-02.7 | Freeze metrics before model tuning | Frozen evaluator policy, metric names, slice requirements, and known-failure rules | Policy and expiry tests | 0 release-eligible cases | incomplete | Meet every frozen denominator and threshold |
| TT2-03.1 | Create a focused module boundary | `continuation/task_truth_v2/` | Rust module tests | deterministic cases | proven | None |
| TT2-03.2 | Define `ObservationPacket` over existing evidence | `observation_packet.rs` | Packet, privacy, keyframe, and budget tests | deterministic cases | proven | None |
| TT2-03.3 | Add minimal canonical elements | `canonical.rs` and packet projections | Canonical conflict/control tests | deterministic cases | proven | None |
| TT2-03.4 | Evolve rather than duplicate `CurrentTaskTurn` | `task_snapshot.rs` projection and lifecycle reuse | Snapshot projection tests | deterministic cases | proven | None |
| TT2-03.5 | Persist bounded semantic checkpoints | `checkpoint.rs` schema, dedupe, lineage, and retention | Checkpoint tests | deterministic cases | proven | Measure live write rate for the final performance gate |
| TT2-03.6 | Select unfinished snapshots before targets | `selection.rs` task-only ranking | Selector tests exclude openability and target richness | deterministic cases | proven | Live wrong-task rate remains missing |
| TT2-03.7 | Shadow audit without product authority | `audit.rs` and shadow path | Path-C audit replay | 5 pending audits | proven | Broaden to reviewed live evidence |
| TT2-04.1 | Define provider-neutral interfaces | `model.rs` client trait and typed failures | Fixture-provider and failure tests | deterministic cases | proven | Optional live-provider smoke evidence remains absent |
| TT2-04.2 | Build a bounded multimodal request | Request builder with image, byte, token, and privacy limits | Request-bound and privacy tests | deterministic cases | proven | Measure real request distributions |
| TT2-04.3 | Require strict structured hypotheses | Versioned response schema with at most two hypotheses | Invalid-output and strict-parse tests | deterministic cases | proven | None |
| TT2-04.4 | Ask task-understanding rather than copy questions | Resolver prompt and evidence-reference contract | Session-013 and unfamiliar-app fixtures | deterministic cases | proven | Human task-quality metrics remain missing |
| TT2-04.5 | Verify every model claim locally | `verifier.rs` field verdicts and anchor subordination | Invented identity, control, history, evidence-id, and target mismatch tests | deterministic cases | proven | Live unsupported-claim rate remains missing |
| TT2-04.6 | Bound a second pass to genuine conflicts | Conflict triggers and second-pass audit | Close-hypothesis, AX/visual conflict, and task/anchor tests | deterministic cases | proven | Measure live trigger rate, latency, and cost |
| TT2-04.7 | Separate task understanding from wording | Deterministic wording and split provenance | Wording identity-preservation tests | deterministic cases | proven | None |
| TT2-04.8 | Evaluate model and local paths honestly | Model-on/off fields and critical-case parity review in the frozen evaluator | Deterministic path replays; gate rejects missing parity denominator | 0 reviewed critical pairs | incomplete | Run paired model-on/off critical local-solvable cases and independently review every disagreement |

## TT2-05 numbered-goal matrix

| Requirement | Authoritative implementation evidence | Test or live proof | Metric denominator | Status | Remaining remediation |
| --- | --- | --- | ---: | --- | --- |
| 1. Versioned authority and rollback policy | `task_truth_v2/production.rs`; authority audits; Task Truth versions and scoped-feedback watermark in the cache identity; persisted `task_truth_v2_decision_contracts`; open-time rollback/release recheck | Closed-gate, complete-report-shape, cache-watermark, and authoritative-null open tests | deterministic policy cases | proven | A real passed report is required before authority can switch |
| 2. TaskSnapshot before target selection | One selected snapshot supplies the public task; `attach_strict_target` requires matching task-turn identity and strict-open approval; matching target title may enrich `Where`, while mismatch only nulls the target | Target mismatch, matching target enrichment, and authoritative null-target open tests | deterministic cases | proven | Live corpus must measure this across surface families |
| 3. Versioned public answer | `smalltalk.task_truth_public_answer.v1` contains task/state/next/where, alternatives, target, evidence preview, field support, split provenance, snapshot revision, and watermark | serialization/build tests | deterministic cases | proven | Release-quality human evaluation missing |
| 4. React and island answer UI | React card/evidence panel and native island use only the authoritative answer for task, state, optional next, where, action, preview, and split provenance; missing fields do not fall through to legacy P6 | Webview authority/no-mixing tests; native contract authority/no-mixing tests; TypeScript build | deterministic tests | proven | Manual React/island parity QA missing |
| 5. Quality-dominant adoption and provenance | React and island adoption compare authoritative snapshot id/revision, field support, target policy, and wording source when enabled; cached/startup/background/island receivers share those paths | Webview authoritative-adoption and island suites | deterministic tests; 0 evaluated live background pairs | incomplete | Collect live manual/background adoption denominator |
| 6. Scoped feedback | Dedicated feedback rows bind to snapshot id, revision, field, and optional hypothesis; they never enter global artifact/workstream feedback. Rejection removes only the scoped field; explicit hypothesis choice promotes it as `human_correction`; cache watermark changes | Scoped isolation, distinct-feedback, hypothesis-promotion, rejected-task target-blocking tests | deterministic cases | proven | Live QA scenario 14 missing |
| 7. Locked release evaluation | v2 evaluator plus `task_truth_v2_release_gate` calculate exact TT2-05 metrics, per-surface rates, Wilson intervals, and reject unproven denominators | generated final report and adversarial gate tests | 0 reviewed live; 0 holdout | incomplete | Meet every corpus, slice, and metric minimum |
| 8. Performance, cost, privacy | Final gate keeps `baseline-report.v1.json` separate from `release-evaluator-report.v1.json`; budgets must bind to the baseline SHA-256 and measured performance/privacy must satisfy them; architectural caps cannot be weakened | Missing policies/manifests are violations; baseline-binding, budget-cap, and unmeasured-performance tests | 0 release runs | missing | Measure the pre-holdout baseline, freeze budgets, then run at least 30 release measurements |
| 9. Manual interruption recovery | Final gate requires `manual-macos-qa.v1.json`, at least 14 reviewed passed scenarios, evidence ids, build/commit, and reviewer | Missing manifest is a gate violation | 0/14 | missing | Run all real macOS scenarios without committing personal screenshots |

## Frozen hard-gate matrix

All metric rows below use independently reviewed live Path C cases only. A numerator of zero with a denominator of zero is not a pass.

| Hard gate | Required | Current numerator / denominator | Status | Remaining remediation |
| --- | ---: | ---: | --- | --- |
| Wrong primary task | at most 3% overall and 5% per required surface family | 0 / 0 | missing | 200 reviewed live cases and required surface denominators |
| Control/navigation as task | 0 | 0 / 0 | missing | Reviewed denominator required |
| Useful non-generic summary | at least 90% | 0 / 0 | missing | Reviewed labels required |
| Task-object accuracy | at least 88% | 0 / 0 | missing | Reviewed labels required |
| Execution-state accuracy | at least 90% | 0 / 0 | missing | Reviewed labels required |
| Supported next-action precision | at least 90% | 0 / 0 | missing | Reviewed labels required |
| Supported next-action coverage | at least 85% where labeled | 0 / 0 | missing | Coverage metric and reviewed labels required |
| Return-target precision | at least 98% | 0 / 0 | missing | Reviewed direct-target cases required |
| Unsupported specific claims | at most 1% | 0 / 0 | missing | Reviewed denominator required |
| Stronger manual result downgraded | 0 | 0 / 0 | missing | Evaluated live manual/background pairs required |
| Unseen-application useful summary | at least 80% | 0 / 0 | missing | 50-case locked holdout required |
| Human immediately useful | at least 85% | 0 / 0 | missing | Blinded ratings required |
| Model-on/off critical disagreement | 0 unexplained | 0 / 0 | missing | Paired critical local-solvable runs required |
| Privacy violations | 0 | no manifest | missing | Performance/privacy manifest required |
| Unsafe opens | 0 | no manifest | missing | Performance/privacy and manual QA required |

## Corpus and slice matrix

| Requirement | Required | Current | Status |
| --- | ---: | ---: | --- |
| Independently reviewed live boundaries | 200 | 0 | missing |
| Locked application-level holdout | 50 | 0 | missing |
| Surface families | 10 | 1 represented; 0 reviewed | missing |
| Interruption/resumption | 30 reviewed | 0 | missing |
| Ambiguous/privacy-blocked | 20 reviewed | 0 | missing |
| Waiting on agent/application | 20 reviewed | 0 | missing |
| Completed-versus-new-task | 20 reviewed | 0 | missing |

## Manual macOS scenarios

All 14 required scenarios are currently `missing`: older-completed/new-question recovery; weak-AX chat submit; edit/command/error; document/spreadsheet; cross-tab research; waiting output; custom-rendered/thin AX; privacy block; two close tasks; understood/no target; selected-task-only open; background non-downgrade; card/island parity; and scoped Not right feedback.

## Generated evidence

- `src-tauri/tests/fixtures/continue_accuracy/task_truth_v2/baseline-report.v1.json`
- `src-tauri/tests/fixtures/continue_accuracy/task_truth_v2/release-evaluator-report.v1.json`
- `src-tauri/tests/fixtures/continue_accuracy/task_truth_v2/final-release-report.v1.json`
- `src-tauri/tests/fixtures/continue_accuracy/task_truth_v2/manual-macos-qa.schema.v1.json`
- `src-tauri/tests/fixtures/continue_accuracy/task_truth_v2/release-budgets.schema.v1.json`
- `src-tauri/tests/fixtures/continue_accuracy/task_truth_v2/performance-cost-privacy.schema.v1.json`
- Generator: `cargo run --bin task_truth_v2_release_gate`

## Verification run

- `cargo test continuation::task_truth_v2`: 43 passed.
- `cargo test continuation::accuracy_eval`: 6 passed.
- `cargo test continuation::task_turn`: 34 passed.
- `cargo test continuation::activity_recap_validation`: 15 passed.
- `cargo test session_island`: 35 passed.
- Full `cargo test`: 605 passed, 0 failed.
- Release-gate binary tests: 6 passed, including forged-pass, missing-denominator/surface-interval, partial-manual, unmeasured-performance, baseline-binding, and weakened-budget cases.
- `cargo check`: passed.
- `cargo fmt --check`: passed.
- `npm run test:webview`: 16 passed, 0 failed.
- `npm run build`: passed.
- `git diff --check`: passed.

## Program status

TT2-05 implementation is complete up to the external evidence boundary. The Task Truth v2 program is not complete. Production semantic authority remains legacy P6, with Task Truth v2 in shadow/eligible mode. Do not set `SMALLTALK_TASK_TRUTH_AUTHORITY=authoritative` until the configured final release report truthfully says `passed = true`.
