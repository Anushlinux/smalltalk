# P6.01 — Ground-Truth Corpus And Full-Pipeline Replay

## Codex task

Build the measurement foundation for P6 before changing production semantics. Add a privacy-safe accuracy fixture contract and replay harness that can identify the first pipeline checkpoint where captured truth diverges from the expected current task.

This goal must make the known Capture-button failure reproducible and measurable. It must not try to repair all extraction, lifecycle, feedback, workstream, recap, or UI behavior yet.

## Dependency gate

Read:

```text
AGENTS.md
PRODUCT.md
docs/phases/p6-task-turn-accuracy/p6-00-task-turn-accuracy-program.md
```

Inspect the live checkout and existing eval implementation. Do not assume symbols or line numbers are unchanged.

## Product context

Smalltalk should recover the user's exact work after an interruption: what they were trying to do, where, what state was left, and what to do next. Existing evals are heavily target-selection-oriented. They often start with already-derived candidates, so they cannot catch a failure that happened during text-role extraction, action derivation, branch promotion, open-loop selection, or recap construction.

P6 needs a replay path covering:

```text
redacted captured evidence
  -> text/region resolution
  -> latest task-turn extraction
  -> task action and semantic delta
  -> branch/feedback eligibility
  -> workstream and open loop
  -> current surface and candidate selection
  -> local recap and optional model recap
  -> public target and product answer
```

## Verified critical case

Source bundle:

```text
continue_outputs/session-012-session-17836451__continue-1783645410372__normal__continue-decision-
```

Adjacent observed decisions:

```text
continue_outputs/session-012-session-17836451__continue-1783645318852__normal__continue-decision-
continue_outputs/session-012-session-17836451__continue-1783645478796__normal__continue-decision-
```

Use the adjacent decisions as evidence of near-term decision instability. Do not assume their evidence watermarks are identical. Compare their source checkpoints and label which changes are legitimate evidence changes versus semantic churn.

Evidence to inspect:

```text
evidence/frames/frame-000070/
evidence/frames/frame-000071/frame_text_resolution.json
evidence/frames/frame-000071/linked_task_actions.json
decision/surface-snapshots.json
continue/layers/continue_feedback_events.ndjson
continue/layers/continue_branch_contexts.ndjson
continue/layers/continue_open_loops.ndjson
activity_recap/stitched_timeline.json
activity_recap/work_labels.json
activity_recap/model_pack.json
activity_recap/model_validation.json
final/continue_decision_result.json
```

Lock this ground truth:

| Field | Expected truth |
| --- | --- |
| Prior task | Update Continue-card copy. |
| Prior task state | Completed. |
| Latest user goal | Understand what the island Capture button does. |
| Current agent state | Tracing the Swift bridge and Rust handler to determine one-shot evidence capture versus continuous capture/recording. |
| Execution state | Active. |
| Current actor | Assistant/agent. |
| Waiting on | Agent. |
| Turn relation | New task after a completed task. |
| Project/workstream | Smalltalk. |
| Direct return target | None unless the replay contains a real, validated URL/path locator. |
| Forbidden primary terms | Stremio, Helium, Research Report Analysis. |
| Old completion text | Allowed only as prior-task evidence. |

The fixture must permit bounded wording variation while making task identity, state, relationship, target policy, and forbidden contamination machine-checkable.

## Files and symbols to inspect first

```text
src-tauri/src/continuation.rs
src-tauri/src/continuation/
src-tauri/src/capture.rs
src-tauri/tests/
continue_outputs/session-012-session-17836451__continue-1783645410372__normal__continue-decision-/
```

Search for:

```text
ContinueEvalFixture
ContinueEvalReport
run_continue_eval
run_continue_replay_eval
continue_eval_fixtures
evaluate_continue_fixture_case
summarize_continue_eval_fixture
ActivityRecapInputs
ContinueDecisionResult
activity_recap
audit_output
```

The current candidate-level eval is useful and must remain compatible. Extend or version it; do not silently change old metric semantics.

## Non-goals

- Do not change production task classification to make the golden pass.
- Do not tune scorer weights.
- Do not alter feedback promotion, open-loop precedence, P5 synthesis, or UI copy.
- Do not commit the source audit folder, screenshots, SQLite data, raw paths, URLs, or personal conversation text.
- Do not mark P6 accurate because the harness exists.

## Required fixture contract

Add a versioned fixture schema such as `smalltalk.continue_accuracy_fixture.v1`. Use names consistent with the repository, but preserve these semantics:

```text
schema
case_id
description
privacy_review
fixture_partition
injection_boundary
redacted_source_records
  frames
  ax_nodes with source role/tree/bounds/order
  ocr_spans with bounds/source ownership
  content_units as captured before P6 role assignment
  frame_text_resolution
  app/window context
  ui events, transitions, and typing metadata
injected_historical_state
  feedback_events
  branch_contexts
  workstreams
  open_loops
  memory_cells
expected_checkpoints
  resolved_text
  region_roles
  conversational_roles
  ordered_turn_spans
  latest_task_turn
  task_action
  semantic_delta
  eligible_feedback
  selected_workstream
  eligible_open_loop
  primary_recap_segment
  local_recap
  validated_recap
  public_target
  product_answer
forbidden_claims
allowed_uncertainty
expected_model_parity
```

Use optional fields where an early phase does not produce a checkpoint yet. Missing expected checkpoints must be reported as `not_implemented` or `missing`, not counted as correct.

Never put expected `surface_role`, `conversational_role`, turn order, task action, workstream, or recap labels into the source records consumed by the transformation that is meant to infer them. Text hashes may support equality/privacy checks, but a hash-only source is not valid for a fixture expected to test semantic text extraction.

Every fixture must declare its injection boundary. The canonical Capture-button path inserts redacted capture/AX/OCR/content/event rows and runs every current semantic transformation through production functions. Historical stale-state contaminants may be inserted only when the case explicitly tests already-persisted prior state; the report must distinguish that coverage from facts derived during the replay.

## Privacy-safe import strategy

Implement a local-only importer or fixture builder that reads a private audit and emits a bounded, redacted candidate fixture. The human-reviewed fixture is the committed artifact.

Required safeguards:

- Default-deny unknown text fields.
- Cap every retained text field with a documented per-field maximum.
- Strip or hash raw URLs, paths, stable keys, conversation ids, frame image paths, user names, tokens, and identifiers.
- Preserve semantic terms required by the case only after explicit review.
- Preserve source AX/OCR roles, hierarchy, geometry, order, timestamp, ownership, and confidence metadata without copying the expected P6 region/speaker labels into inputs.
- Never copy screenshots or database files into fixtures.
- Add a fixture privacy linter that fails on likely secrets, home-directory paths, URL query strings, long opaque tokens, and oversized text.
- Mark retained fixture text as `synthetic` or `derived_redacted`. Do not label copied private text as synthetic.
- Do not durably persist verbatim user messages or typed sequences by default. Prefer public-safe semantic summaries, hashes, and source references.

If an exact question is needed, write a genuinely synthetic equivalent. If a reviewed redacted excerpt is retained, label it `derived_redacted` and record human privacy approval.

## Full-pipeline replay design

Create a focused module rather than expanding the main `continuation.rs` policy block. A likely shape is:

```text
src-tauri/src/continuation/accuracy_eval.rs
src-tauri/src/continuation/accuracy_fixture.rs
src-tauri/tests/fixtures/continue_accuracy/
```

Use the repository's actual module conventions if a better location exists.

The replay must:

1. Build a fresh in-memory or temporary SQLite database.
2. Install the real capture/Continue schemas required by the fixture.
3. Insert redacted capture/AX/OCR/content/event rows at the declared input boundary and explicitly labeled historical contaminants at their declared boundary.
4. Run every available semantic transformation through the production functions used by `get_continue_decision`.
5. Prevent network dependence by using model-off mode and deterministic fixture responses for model-on mode.
6. Record every available checkpoint.
7. Compare checkpoints with typed expectations.
8. Report the first divergent checkpoint and all later mismatches.
9. Repeat the same evidence to detect nondeterminism.

Do not build a parallel fake pipeline that reimplements production logic in the evaluator. An unavailable production checkpoint is `missing`; a fixture-only substitute cannot count as full-pipeline coverage. Report production-path coverage per checkpoint and classify component/counterfactual injection cases separately from capture-to-answer replay.

## Required counterfactual cases

Create these five core Capture-button fixtures. The `All contaminants` case is the canonical critical bundle:

| Case | Difference | Required invariant |
| --- | --- | --- |
| Fresh task only | No old completion, feedback, or open loop. | Capture-button task is active. |
| Old completion visible | Add prior completed Continue-card result. | Current task identity does not change. |
| Stale inferred feedback | Add old inferred Helium correction/acceptance. | Current task identity does not change. |
| Unrelated open loop | Add strong Stremio open loop. | Current task identity does not change. |
| All contaminants | Add every stale contaminant. | Current task identity does not change. |

Run the identical `All contaminants` fixture repeatedly as a determinism metric, not as a sixth fixture.

Add two more longitudinal fixtures for the adjacent `5318852` and `5478796` decisions. The main `5410372` bundle reuses the canonical `All contaminants` fixture. This yields exactly seven distinct initial case ids: five core cases plus two adjacent-window cases. Task identity may change across adjacent cases only when a labeled material task-turn/evidence change occurred.

Early in P6 these cases are expected to expose failures. The normal test suite may assert that baseline mismatches are reported accurately without requiring production behavior to pass the future release thresholds yet. Add explicit milestone flags so a later phase can convert known failures into mandatory passes without hiding regressions.

Store known failures in a checked-in milestone manifest with `case_id`, `expected_status`, `expected_first_divergence`, `must_pass_by_phase`, and `owner_checkpoint`. Do not use ignored tests or unrestricted allow-failure. The owning goal must flip its checkpoints to mandatory pass. Unexpected improvement and regression both require manifest review. P6.09 must fail if any P6 known-failure marker remains.

## Frozen eval policy

Before P6.02 changes production behavior, add a versioned eval-policy file under the committed accuracy fixture directory. Freeze:

```text
semantic slot scoring rubric
materially-wrong definition
confidence label score boundaries
wrong-confident threshold
minimum sample sizes
development/validation/locked-holdout partitions
macro and worst-slice aggregation
calibration threshold
baseline latency/resource measurements
allowed regression budget
```

Reuse documented current product thresholds when they exist. Record any new boundary before looking at locked-holdout results. Require ECE at or below 0.10 for a dimension only when it has at least 100 labeled predictions and enough cases per bin; otherwise report insufficient sample size. Unless a stricter existing budget applies, freeze p95 model-off decision latency to no more than 1.25 times the P6.01 baseline and within any existing absolute budget.

## Required checkpoint result

Each case result must include at least:

```text
case_id
status
first_divergent_checkpoint
checkpoint_results
forbidden_claim_matches
wrong_confident
public_target_honest
model_on_off_task_identity_match
deterministic_replay_match
privacy_lint_passed
notes
```

`wrong_confident` is true whenever a materially wrong task, state, workstream, or target is presented at or above the frozen wrong-confident boundary. Unknown and abstained fields are not counted as correct unless the fixture explicitly labels abstention as the expected result.

## Metrics to add

Add explicit numerators, denominators, and rates for:

- fixture parse/privacy success;
- latest user-goal accuracy;
- current agent-state accuracy;
- execution-state accuracy;
- current-actor accuracy;
- waiting-on accuracy;
- region-role macro-F1;
- conversational-role macro-F1;
- latest-user-span precision and recall;
- current-agent-status precision and recall;
- unknown/abstention correctness;
- task-turn boundary accuracy;
- prior-completion override rate;
- task-action accuracy;
- semantic-delta temporal accuracy;
- selected-workstream/task alignment;
- stale-feedback false promotion count;
- unrelated-open-loop primary count;
- recap/current-task contradiction count;
- forbidden stale-term leakage;
- current-state accuracy;
- task-summary precision and coverage;
- supported-next-action precision and coverage/recall;
- no-clear accuracy on genuinely unclear cases;
- wrong-confident rate;
- direct-openability precision and labeled-openable target recall;
- frame-fallback public-target count;
- model-on/model-off task-identity agreement;
- deterministic replay agreement.

Do not derive semantic metrics from target correctness proxies. Compare actual typed checkpoint slots for task action, object, project/workstream, actor, execution state, waiting-on state, and target policy. Report macro averages across cases and surface families plus worst-slice performance; do not let abundant chrome spans inflate attribution accuracy.

## Tests

Add deterministic tests for:

1. Fixture version parsing and rejection of unknown required fields.
2. Privacy linter rejection of a home path, secret-like token, raw URL query, oversized text, and screenshot path.
3. Ordered insertion and replay determinism.
4. First-divergence reporting.
5. Forbidden-claim matching with normalization that does not create false positives from audit metadata.
6. Wrong-confident classification.
7. Target-honesty classification.
8. The critical Capture-button fixture and all counterfactual variants.
9. Model-off replay and deterministic model-response replay.
10. Backward compatibility for existing `run_continue_eval` behavior.
11. Milestone manifest enforcement and expiration.
12. Frozen eval-policy parsing and holdout access controls.
13. Production-path coverage reporting.

## Audit output

Add a privacy-safe summary file for explicit accuracy-eval runs, not background Continue actions. It should contain the fixture schema, case results, aggregate metrics, policy/version hashes, and first-divergence data. It must not contain source screenshots, raw databases, unredacted paths, or broad captured history.

## Acceptance criteria

This goal is complete when:

- A versioned, privacy-safe full-pipeline fixture schema exists.
- A fixture importer/builder and privacy linter exist.
- Exactly seven initial Capture-button case ids are represented: five core cases and two additional adjacent-window cases; repeatability reuses the canonical all-contaminants case.
- Replay calls production transformation code rather than a separate mock decision engine.
- Every case declares its injection boundary and production-path coverage.
- The report identifies the earliest incorrect checkpoint in the current baseline.
- Semantic accuracy metrics have real typed denominators.
- A frozen eval policy and milestone manifest exist before P6.02.
- Existing candidate-level eval behavior remains available and documented.
- Normal tests stay green while known P6 baseline failures are explicitly visible as milestone failures.
- No private generated artifact is committed.
- The next phase can add role/region extraction and observe its checkpoint improvements in this harness.

## Verification commands

Run at minimum:

```bash
cd src-tauri && cargo fmt --check
cd src-tauri && cargo check
cd src-tauri && cargo test
npm run build
git diff --check
git status --short
```

Run the new accuracy evaluator against the committed fixtures and include the command and baseline report in the final response. If a private audit importer is run, confirm that only redacted fixture data was written inside the repository.

## Final response format

Report:

1. Files changed.
2. Fixture and checkpoint schemas.
3. Privacy safeguards.
4. Critical/counterfactual cases added.
5. Current first-divergence result.
6. Metrics with numerators and denominators.
7. Tests and verification results.
8. Known baseline semantic failures that remain.
9. Exact contracts P6.02 may rely on.

Do not claim output accuracy is fixed. P6.01 creates ground truth and makes failure measurable.
