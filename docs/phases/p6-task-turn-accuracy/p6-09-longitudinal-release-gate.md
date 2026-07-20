# P6.09 — Longitudinal Accuracy, Calibration, Privacy, And Release Gate

## Codex task

Complete P6 by proving the full evidence-to-answer chain across real interruption patterns, counterfactual stale contamination, model modes, repeated decisions, target states, and UI surfaces. Expand the privacy-safe corpus, enforce release thresholds, run performance/privacy regression, complete manual macOS QA, and update product documentation to implementation truth.

This is a proof and hardening goal. Do not lower thresholds, remove difficult fixtures, or redefine metrics to make the phase appear complete.

## Dependency gate

P6.01-P6.08 must be implemented and individually verified. Before editing, audit every prior prompt's acceptance criteria against current code and test evidence. If a prerequisite is incomplete, fix it within its original contract before proceeding.

Read all files in:

```text
docs/phases/p6-task-turn-accuracy/
```

Also read:

```text
AGENTS.md
PRODUCT.md
docs/full-engine-flow.md
docs/continue-ui-reality.md
docs/p4-island-no-bypass-manual-qa.md
```

Inspect current code, tests, fixture corpus, git status, and the latest explicit Continue audit bundles. Current state overrides prompt-era assumptions.

## P6 release claim

P6 may be called complete only if Smalltalk can reliably preserve and explain the latest meaningful task turn through interruptions:

```text
what the user was doing
where they were doing it
what was complete versus still active
what state was left
what to do next
whether a direct safe target exists
which claims are uncertain
```

The answer must remain stable when stale completion text, old feedback, unrelated open loops, support branches, model phrasing, or frame previews are introduced.

## Completion audit before new work

Create a requirement matrix covering every explicit contract from P6.01-P6.08:

```text
requirement
authoritative implementation evidence
unit/integration/full-pipeline test
current metric or audit proof
status: proven / contradicted / weak / missing
remediation
```

Do not use a compile pass as proof of semantic correctness. Do not use component tests as proof of full-pipeline behavior. Fix contradicted, weak, and missing items before declaring release readiness.

## Corpus requirements

### Critical golden set

Include the Capture-button case and every contamination variant from P6.01. Add zero-tolerance critical cases for:

1. New task after completed task.
2. Clarification after completion.
3. New task superseding unfinished work.
4. Agent active status versus final answer.
5. Old terminal/test completion beside new chat task.
6. Stale inferred feedback targeting same artifact/app.
7. Unrelated strong open loop.
8. Promoted support branch from prior task turn.
9. Selected workstream/primary recap mismatch.
10. Frame fallback with a convincing human label.
11. Direct openable file/browser target.
12. Probe timeout/no change/privacy block.
13. Mixed-window/Mission Control capture.
14. Thin AX with OCR geometry.
15. Model output choosing a different task.
16. No clear task and no target.

### Broad labeled corpus

Grow to at least 100 privacy-safe, human-reviewed cases. Include balanced coverage across:

```text
chat and agent conversations
code editors
terminal commands/output/errors
browser research and search support
documents/notes/PDF reading and editing
messaging interruptions
Finder/file browsing detours
multi-pane windows
same app with several tasks
same project with successive tasks
cross-project switches
completed, active, blocked, waiting, superseded, idle, unclear states
direct target, app-focus-only, frame-preview-only, and no-target states
strong, medium, thin, privacy-blocked evidence
short and long interruption windows
```

Do not commit private source captures. Commit only bounded redacted fixtures that pass the fixture privacy linter.

Partition the corpus before final tuning:

```text
development
validation
locked holdout
```

Do not tune implementation or thresholds against the locked holdout. Require at least five cases in each major modality, execution-state class, target-state class, and interruption class. Use the frozen semantic slot rubric from P6.01 rather than subjective prose similarity. Record human reviewer sign-off for labels and privacy review; use two-reviewer adjudication where available, or a documented owner-plus-reviewer resolution process for ambiguous labels.

Codex must not self-label a case and claim it was human-reviewed. If required human sign-off is unavailable, the release gate is incomplete and the goal remains active.

### Counterfactual generation

For suitable base cases, generate deterministic variants adding one contaminant at a time:

```text
old completion cue
old accepted/corrected feedback
unrelated open loop
support branch
sidebar project names
terminal completion output
older memory cell
frame fallback target label
probe failure
model paraphrase
```

Expected current task identity must remain invariant unless the inserted evidence is explicitly labeled as a legitimate new task signal.

## Full-pipeline replay requirement

The release evaluator must use production transformations from privacy-safe capture/AX/OCR/content/event rows through final decision. It must not begin at already-scored candidates, expected role labels, or hand-authored recap facts. Explicit historical contaminants require declared injection boundaries. Any unavailable production checkpoint is missing, not replaceable by fixture-only logic, and each case must report production-path coverage.

For every case record:

```text
resolved ordered evidence
current task turn, execution state, current actor, and waiting-on state
task action/semantic delta
feedback applicability
branch role/promotion
selected workstream
eligible open loop/objective
cross-layer consistency
confidence vector
local recap
model recap/validation when enabled
public targets/evidence previews
main-card presentation state
island presentation state
strict-open result when applicable
first divergent checkpoint
latency and probe behavior
```

## Release metrics

Report numerator, denominator, excluded/unknown count, and rate for every metric.

Unknown/abstained predictions count as incorrect unless abstention is the labeled expected behavior. Report case-macro, surface-family macro, and worst-slice values; micro span totals cannot substitute for them.

### Critical zero-tolerance gates

All must be true:

```text
critical fixture pass rate = 100%
remaining P6 known-failure markers = 0
prior-completion override count = 0
stale-feedback false promotion count = 0
unrelated-open-loop primary count = 0
silent cross-layer contradiction count = 0
forbidden stale-term primary leakage = 0
wrong-confident output count = 0
unsupported public claim count = 0
frame-fallback public target count = 0
unsafe/open-policy bypass count = 0
main-card/island semantic disagreement count = 0
model-on/model-off task identity disagreement count = 0
privacy-lint violation count = 0
deterministic replay disagreement count = 0
```

### Broad-corpus minimum gates

Require at least:

```text
region-role macro-F1 >= 98%
conversational-role macro-F1 >= 98%
latest-user-span precision and recall >= 95%
current-agent-status precision and recall >= 95%
unknown/abstention correctness >= 95%
latest user-goal accuracy >= 95%
current agent/user state accuracy >= 95%
task-turn boundary accuracy >= 95%
task-action temporal accuracy >= 95%
semantic-delta temporal accuracy >= 95%
selected-workstream/current-task alignment >= 95%
primary-task summary accuracy >= 90%
task-summary coverage >= 95% on cases labeled summarizable
execution-state accuracy >= 90%
current-actor accuracy >= 95%
waiting-on accuracy >= 90%
supported-next-action precision >= 95%
supported-next-action coverage/recall >= 90% on cases with a labeled evidence-backed next action
no-clear accuracy >= 95% on genuinely unclear cases
direct-openability precision = 100% for URL/path or another explicitly supported typed direct locator
direct-target recall >= 95% on labeled safely openable cases
interruption/recovery task retention >= 90%
counterfactual contamination invariance >= 95% broad and 100% critical
```

`interruption/recovery task retention` uses only labeled sequences where the same task should survive an interruption; the numerator is sequences whose recovered current task matches the pre-interruption task after return. If a metric cannot be computed from the current corpus, the gate is missing, not passing. Every slice must also pass the frozen worst-slice floors from P6.01.

## Confidence calibration gate

For each claim dimension with enough labeled cases:

- report correctness by predicted label;
- report overconfident-wrong count;
- report Brier score where binary labels are meaningful;
- report expected calibration error with bin/sample counts;
- identify dimensions with insufficient sample size.

Release requires zero wrong-high-confidence critical claims. Enforce the P6.01 frozen label boundaries, sample minimums, and calibration thresholds. Do not choose or relax them after viewing validation or holdout results.

## Model parity and adversarial validation

Run every critical case with:

```text
activity recap model disabled
deterministic valid model response
wrong-task model response
prior-completion model response
unsupported-target model response
confidence-inflating model response
invalid structured output
timeout/error
```

Required:

- accepted model phrasing preserves local task identity, lifecycle, workstream, and target policy;
- invalid/wrong outputs fall back locally;
- model failure never removes a useful local recap;
- suppressed/rejected candidates and private raw history never enter the model pack;
- no network call is required for deterministic CI.

## Cache and longitudinal stability

Test repeated Continue decisions across:

1. Identical evidence and policy.
2. New user task turn.
3. Prior task completion.
4. Feedback event.
5. Branch promotion/demotion.
6. Open-loop lifecycle change.
7. Probe success/failure.
8. Model policy version change.
9. Target open event.
10. Surface snapshot update.

Required:

- identical evidence produces stable task identity and output policy;
- material semantic changes invalidate cache;
- non-semantic diagnostic changes do not cause unnecessary identity churn;
- cached recap and decision share the same task-turn/consistency/confidence policy hashes;
- stale decision cannot open.

## Performance and resource gate

P6 must not turn sparse local memory into continuous expensive analysis.

Measure and report:

```text
decision latency p50/p95 model-off
decision latency p50/p95 with deterministic model path
probe latency/success/timeout rates
task-turn/span rebuild time
fixture replay time
SQLite row growth for new P6 tables
snapshot/text retention impact
memory usage when measurable
cache hit/miss behavior
```

Verify:

- role/task extraction reuses persisted AX/OCR/content evidence;
- no new continuous screenshot loop exists;
- probe budgets/cooldowns are enforced;
- cleanup/retention covers new P6 tables;
- default Continue remains bounded for normal local use.

Enforce existing absolute budgets plus the P6.01 frozen baseline/regression policy. Do not introduce or relax a threshold after viewing validation/holdout performance.

## Privacy and security gate

Audit:

- new SQLite columns/tables;
- committed fixtures;
- audit exports;
- model packs;
- React/island payloads;
- logs and errors;
- retention/cleanup;
- strict-open inputs.

Required proof:

- no raw keylogged characters;
- no full clipboard text;
- no secrets/tokens/raw personal paths/raw query-bearing URLs in fixtures or model packs;
- no screenshots or local databases committed;
- sensitive/private surfaces fail closed;
- evidence preview cannot be used as an unsafe open locator;
- redaction/privacy linter covers new fields.

## Automated regression categories

Add or finalize:

1. Contract/schema round trips and migrations.
2. Role/region/order extraction.
3. Task-turn lifecycle.
4. Feedback scope/decay.
5. Workstream/branch/open-loop consistency.
6. Confidence/probe outcomes.
7. Local recap and model validation.
8. Target truth and strict open.
9. React presentation helpers/components.
10. Island mapping/parity/no-bypass.
11. Full-pipeline golden replay.
12. Counterfactual contamination invariance.
13. Cache/watermark stability.
14. Privacy lint and retention.
15. Performance budget/regression checks where deterministic.

## Manual macOS interruption-recovery QA

Run the real Tauri app and record results for at least these scenarios:

1. Complete one chat task, ask a new task in the same conversation, then open Continue.
2. Ask an agent to inspect code and interrupt while its working status is visible.
3. Edit code, run a command, see an error, search docs, and return.
4. Leave a terminal success message beside a new chat task.
5. Switch between two Smalltalk tasks in the same app.
6. Switch from Smalltalk to an unrelated project and back.
7. Open Finder/photos or messages as a brief interruption.
8. Use a weak Codex/editor surface with no direct locator.
9. Use an openable file or browser page.
10. Trigger a feedback-suppressed/support-only target.
11. Create a stale decision, then add a new user task before opening.
12. Compare main card and island for task, state, confidence, action, and open result.

For each record:

```text
expected task/state/target
observed main-card answer
observed island answer
open/inspect behavior
audit decision id
pass/fail and evidence
```

Do not commit screenshots or private audit bundles.

Persist privacy-safe proof rather than leaving it only in a final response:

```text
docs/phases/p6-task-turn-accuracy/p6-08-manual-qa-results.md
docs/phases/p6-task-turn-accuracy/p6-09-completion-audit.md
src-tauri/tests/fixtures/continue_accuracy/release-report.json
```

Redact or hash decision ids and any private locators in committed reports. The release report must include policy/fixture versions and holdout aggregate results without revealing holdout private text.

## Product documentation

Update `PRODUCT.md` and relevant engine/UI docs to current implementation truth:

- P6 task-turn evidence and lifecycle;
- feedback applicability;
- workstream/open-loop consistency;
- confidence vector and probe policy;
- recap validation/model parity;
- direct-target/evidence-preview distinction;
- eval schema/metrics/release gate;
- current limitations and known thin surfaces;
- exact verification commands.

Remove or correct stale baseline claims. Do not describe planned behavior as implemented unless the completion audit proves it.

## Acceptance criteria

P6.09 and the whole P6 program are complete only when:

- every prior prompt acceptance criterion is proven;
- the critical golden set passes at 100%;
- every zero-tolerance count is zero;
- broad-corpus thresholds pass on at least 100 privacy-safe labeled cases;
- development/validation/locked-holdout partitions, slice minimums, and genuine human sign-off are proven;
- model parity/adversarial validation passes;
- cache/longitudinal stability passes;
- performance and privacy gates pass;
- automated verification is green;
- manual macOS QA passes or every failure is fixed and rerun;
- main card and island agree;
- `PRODUCT.md` reflects current truth;
- privacy-safe manual-QA, completion-audit, and release-report artifacts are committed;
- no required work remains.

If any gate is weak, missing, below threshold, or based only on indirect evidence, keep the goal active and continue. Do not mark P6 complete.

## Verification commands

Run at minimum:

```bash
cd src-tauri && cargo fmt --check
cd src-tauri && cargo check
cd src-tauri && cargo test
npm run build
npm run tauri dev
git diff --check
git status --short
```

Also run the versioned P6 accuracy evaluator, privacy linter, model-parity suite, cache/replay suite, and any deterministic performance command added by prior phases. Report exact commands and results.

## Final response format

Report:

1. Completion-audit matrix summary.
2. Files and contracts changed in final hardening.
3. Corpus composition and privacy review.
4. Every critical metric and broad metric with numerator/denominator.
5. Confidence calibration and sample sizes.
6. Model parity/adversarial results.
7. Cache/longitudinal stability results.
8. Performance/resource results.
9. Privacy/security results.
10. Automated commands/results.
11. Manual macOS scenarios/results.
12. Documentation updates.
13. Remaining limitations, if any.

Mark the goal complete only when all release gates are genuinely proven. Do not call Smalltalk fully accurate merely because one golden fixture passes.
