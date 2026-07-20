# P6.09 Completion Audit

Date: 2026-07-11  
Evaluator: `smalltalk.continue_accuracy_report.v2`  
Policy: `p6.01-v1`

## Release verdict

P6 is **not release-complete**. The implemented deterministic seven-case Capture-button corpus passes its semantic checkpoints, but the P6.09 release gate correctly remains closed. The repository does not contain the required 100-case, independently human-reviewed corpus, a locked holdout, enough positive target/no-clear/next-action labels, calibration sample sizes, or completed manual macOS interruption-recovery QA. The measured local model-off p95 also exceeds the frozen regression budget.

The machine-readable evidence is `src-tauri/tests/fixtures/continue_accuracy/release-report.json`. `milestone_contract_passed` describes the phase milestone manifest only; `release_gate.passed` is the authoritative P6 release verdict.

## P6.01-P6.08 requirement matrix

| Phase / requirement | Authoritative implementation evidence | Test or metric proof | Status | Remediation |
| --- | --- | --- | --- | --- |
| P6.01 versioned redacted fixture, policy, privacy lint, first-divergence replay | `accuracy_fixture.rs`, `accuracy_eval.rs`, fixture policy and cases | 7/7 parse/privacy; deterministic replay 7/7 | Proven for initial corpus | Expand without changing frozen semantics |
| P6.01 development/validation/locked-holdout release corpus | Fixture partition contract and default-denied holdout access | Development 4, validation 3, holdout 0 | Missing | Human-reviewed holdout required |
| P6.02 ordered AX/OCR/content spans, roles, ownership, reading order, conservative fallback | `task_turn_evidence.rs`, capture persistence and replay checkpoints | Region-role 14/14; conversational-role 3/3; span precision/recall 2/2 | Proven narrowly | Add at least five cases per required modality/slice |
| P6.03 current task-turn identity, independent lifecycle axes, prior relation, scoped actions/deltas, cache watermark | `task_turn.rs`, task-turn persistence and Continue integration | Goal 6/6; state 21/21; boundary 4/4; action 2/2; delta 2/2 | Proven narrowly | Broaden lifecycle corpus |
| P6.04 feedback provenance, scope, decay, per-target effect, no stale promotion | `feedback_policy.rs` and feedback audit in replay | Stale-feedback false promotions 0/7 | Proven for critical contamination fixtures | Add broad feedback modality coverage |
| P6.05 task/workstream/open-loop semantic center and support-branch restrictions | `semantic_consistency.rs`, open-loop/objective integrations | Workstream alignment 3/3; unrelated primary loops 0/7; contradictions 0/7 | Proven narrowly | Add successive/cross-project and branch lifecycle cases |
| P6.06 split confidence dimensions and observe-before-decide counterfactual outcomes | `confidence.rs`, probe observations and calibration report | Probe statuses cover success/no-change/timeout/failure/privacy/stale; wrong-confident 0/7 | Weak for release | At least 100 labeled calibration predictions per frozen policy |
| P6.07 truth-pack bounded model input, identity-preserving validation, local fallback | `activity_recap_truth.rs`, model/validation/integration modules | Model-on/off identity 7/7; forbidden primary leakage 0/7 | Proven for deterministic valid response | Add every required adversarial response mode across all critical cases |
| P6.08 direct target versus evidence preview, React/island semantic contract, strict-open no-bypass | Continue result, `continuePresentation.ts`, React and island contract | Frame fallback public targets 0/7; presentation and island unit tests | Automated proof only | Positive direct-target, stale-open, and native parity manual cases required |
| P6.08 manual macOS QA | `p6-08-manual-qa-results.md` | Scenarios explicitly recorded as not run | Missing | Run and record all P6.09 native scenarios |

## Current release metrics

All seven current cases pass, with zero known-failure markers. The following measured metrics are 100% on their current denominators: region roles 14/14, conversational roles 3/3, latest-user span precision/recall 2/2, agent-status precision/recall 2/2, latest goal 6/6, execution/current actor/waiting-on 7/7 each, task-turn boundary 4/4, task action 2/2, semantic delta 2/2, workstream alignment 3/3, task summary 7/7, model identity parity 7/7, privacy 7/7, and deterministic replay 7/7.

Zero-tolerance counts currently measured are all zero: prior-completion override, stale-feedback false promotion, unrelated-open-loop primary selection, cross-layer contradiction, forbidden stale-term leakage, wrong-confident output, and frame-fallback public targeting.

These results do not satisfy P6.09 because the denominators and surface-family coverage are too small. Supported-next-action precision/recall, no-clear accuracy, direct-openability precision, and direct-target recall have denominator zero.

## Calibration, performance, privacy, and longitudinal proof

- Every reported confidence dimension has insufficient samples for ECE; the largest current sample is seven versus the frozen minimum of 100.
- The generated release run measured model-off replay p95 at 287.30 ms versus a frozen regression budget of 204.05 ms. This is a local debug replay measurement and is a failing release signal until reproduced and fixed or explained under the already-frozen environment policy.
- Deterministic replay agreement is 7/7. Probe counterfactuals preserve task identity across success, unchanged evidence, timeout, failure, privacy block, and stale result.
- Fixture privacy lint passes 7/7. The fixtures are synthetic and bounded, but their repository-owner metadata is not independent human adjudication.
- Manual macOS interruption recovery remains unproven; automated tests cannot substitute for it.

## Required work before P6 can be closed

1. Obtain real human-reviewed, privacy-approved labels and grow the corpus to at least 100 cases with development, validation, and locked-holdout partitions and the required slice minimums.
2. Add the missing critical scenarios and labeled positive direct-target, no-clear, next-action, interruption-retention, modality, and target-state coverage.
3. Run all adversarial model modes on every critical case and report each mode independently.
4. Meet the frozen calibration and performance gates.
5. Complete and record the real macOS interruption-recovery matrix, including main-card/island parity and strict-open results.
6. Regenerate `release-report.json`; P6 is complete only when `release_gate.passed` is true with no violations.

