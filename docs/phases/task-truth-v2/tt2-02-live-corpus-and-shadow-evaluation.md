# Task Truth v2.02 — Build The Live Corpus And Shadow Evaluation

## Codex task

Replace synthetic confidence with a privacy-safe, human-adjudicated corpus and a production-path shadow evaluator that can measure whether Smalltalk understood the user's real task.

This goal exists because the seven P6 fixtures contain clean semantic role identifiers that were absent from the live Codex accessibility tree. Passing them did not predict live behavior. Do not change task-resolution policy to improve scores in this goal. Build the measurement authority first.

## Terminal success condition

This goal is complete only when:

1. Session-013 is reproducible from live-shaped redacted evidence without injected role labels.
2. The evaluator compares three paths on identical evidence: legacy P6, causally repaired local truth, and Task Truth v2 shadow output when available.
3. Human labels cover task, object, lifecycle, next action, where, ambiguity, and target independently.
4. Application-level holdouts prevent tuning on near-duplicate frames from the same surface.
5. Release reports distinguish missing human evidence from passing behavior.
6. The corpus has a documented path to the final minimum and cannot report release success below it.

## Dependency gate

Task Truth v2.01 must be complete and verified. Read its completion report and inspect the implementation rather than assuming this prompt's symbol names still match.

Read:

```text
AGENTS.md
PRODUCT.md
docs/phases/task-truth-v2/tt2-01-causal-evidence-and-containment.md
docs/phases/p6-task-turn-accuracy/p6-01-ground-truth-replay-eval.md
docs/phases/p6-task-turn-accuracy/p6-09-longitudinal-release-gate.md
docs/phases/p6-task-turn-accuracy/p6-09-completion-audit.md
src-tauri/src/continuation/accuracy_fixture.rs
src-tauri/src/continuation/accuracy_eval.rs
src-tauri/tests/fixtures/continue_accuracy/
continue_outputs/session-013-*/
```

Do not commit private captures, screenshots, SQLite files, full URLs/paths, conversation ids, or raw user text.

## Measurement doctrine

The primary metric is whether the current task is correct and useful. Target safety, confidence calibration, and field population are secondary and cannot compensate for a wrong task.

The evaluator must distinguish:

```text
wrong specific task
correct task
acceptable alternative interpretation
precise abstention
generic non-answer
unsupported confident answer
```

A precise abstention is safer than a wrong task, but it is not counted as useful task recovery. A generic sentence is neither correctness nor abstention.

## Goal 1 — Version a real Task Truth fixture contract

Extend or supersede `smalltalk.continue_accuracy_fixture.v1` with a versioned contract that preserves production-shaped evidence. Do not fork a fake evaluator.

Required source records:

```text
frame metadata and privacy status
local screenshot/keyframe references for private replay
redacted screenshot-derived regions for committed fixtures
AX nodes with native roles, subroles, hierarchy, bounds, actions, focus, editability
OCR spans with bounds and ownership
content units exactly as capture produced them
window/app/surface metadata
ui events without raw typed characters
typing-burst counts, commit signals, pre/post links, and association provenance
capture triggers and event transitions
frame diffs and change regions
prior valid task snapshot when the case explicitly supplies one
return-anchor facts without making them task labels
```

Never inject expected authorship, task relevance, task identity, current goal, or semantic region role into source input. Expected labels live only under the adjudication section.

Every case must declare:

```text
source_kind = live_redacted | synthetic_counterfactual
capture_pipeline_version
injection_boundary
surface_family
application_identity_bucket
privacy_review_status
label_review_status
partition
```

Synthetic counterfactuals remain useful for invariant tests, but cannot be the primary release denominator.

## Goal 2 — Add a privacy-safe local corpus builder

Create an explicit local-only workflow that can turn selected private evidence into a review candidate. It must default-deny fields and require human approval before a redacted fixture is committed.

Required protections:

- keep screenshots in a gitignored private corpus unless a separately generated fully synthetic image is approved;
- strip home paths, URL query strings, account names, tokens, conversation ids, and stable personal identifiers;
- cap every retained text field;
- label retained text `derived_redacted`, `human_paraphrase`, or `synthetic`;
- preserve geometry/tree/event relationships while changing sensitive strings;
- fail on likely secrets, emails, phone numbers, long opaque tokens, raw home paths, and oversized fields;
- generate a privacy manifest and content hashes;
- never call copied private conversation text synthetic.

Provide a dry-run mode that reports what would be retained without writing a fixture.

## Goal 3 — Make session-013 the first authoritative live-shaped family

Create a redacted replay family from the five latest session-013 decisions. It must preserve:

- the actual sparse/ambiguous Codex AX roles;
- the user interaction and committed Enter metadata;
- null `post_frame_id` in the legacy input case and the repaired causal interpretation as a derived checkpoint;
- visible controls including an approval control;
- current and prior content order;
- manual versus background decision timing;
- optional model request/validation outcome;
- lack of a safe direct thread locator when that is the truth.

It must not contain convenience ids such as `conversation-user-message` unless the live source actually contained them.

Label each decision boundary separately. Do not assume adjacent decisions share identical evidence.

## Goal 4 — Define human adjudication independently of product output

For every live case, collect:

```text
primary task summary
task object
user goal
last meaningful progress
unfinished step
execution state
current actor
waiting on
next supported action
where/app/surface identity
relation to prior task
support/detour surfaces
direct return anchor or none
acceptable alternative hypotheses
required abstention fields
forbidden claims
immediately useful: yes/no
reviewer notes
```

The reviewer must not see the product's selected answer before forming the initial label. Store reviewer identity as a pseudonymous local id. Ambiguous cases require resolution by at least three reviewers or must remain explicitly ambiguous.

Codex may build the tooling and prefill evidence handles. Codex must not label its own output as independently human-reviewed.

## Goal 5 — Use application-level partitions

Random frame splits are forbidden. Split by application/layout/workflow identity so near-duplicate frames cannot leak across development and holdout.

Required final corpus:

```text
at least 200 independently reviewed live decision boundaries
at least 50 locked holdout cases
at least 10 surface families
at least 15 reviewed cases in every required surface family
at least 30 interruption/resumption sequences
at least 20 ambiguous or privacy-blocked cases
at least 20 waiting-on-agent/application cases
at least 20 completed-versus-new-task boundary cases
```

Required families include:

```text
agent/chat
editor/IDE
terminal
browser research/search
documents
spreadsheets
email/messaging
PDF/file manager
custom-rendered or canvas UI
mixed-window/thin/unknown surface
```

The totals may overlap by slice, but the release report must publish every denominator.

The initial implementation session does not need to fabricate 200 cases. It must make progress honestly, record the real count, and keep the release gate false until the minimum is independently satisfied.

## Goal 6 — Build a three-path shadow evaluator

On identical source evidence, report:

```text
path_a = legacy P6 production semantics
path_b = causally repaired local task truth
path_c = Task Truth v2 multimodal shadow resolver, when implemented
```

Missing path C is `not_implemented`, never a pass.

For each path, compare field-by-field and record the first divergence:

```text
observation construction
causal interaction association
control/region eligibility
authorship
task selection
task object
lifecycle state
last progress
unfinished step
next action
where
target resolution
answer composition
```

Report both macro results and worst surface-family slice. Do not let abundant easy browser cases hide failures in native or unknown applications.

## Goal 7 — Freeze metrics before model tuning

Freeze the rubric and thresholds in a versioned policy file before inspecting locked-holdout output.

At minimum, define:

```text
wrong-primary-task
useful-non-generic-summary
task-object accuracy
execution-state accuracy
supported-next-action accuracy
unsupported-specific-claim
control/navigation-as-task
return-target precision
stronger-manual-result-downgraded
unseen-application useful summary
human immediately-useful rating
precise abstention
generic non-answer
```

The release gate in Task Truth v2.05 will use these definitions. Any threshold change after holdout access requires a policy version bump and written justification.

## Required tooling and outputs

Prefer extending:

```text
src-tauri/src/continuation/accuracy_fixture.rs
src-tauri/src/continuation/accuracy_eval.rs
src-tauri/tests/fixtures/continue_accuracy/
```

Add a separate `task_truth_v2` policy/report namespace if that prevents old P6 metrics from silently changing meaning.

Required report fields:

```text
corpus counts and partitions
human review counts
privacy review counts
surface/slice denominators
per-path field metrics
wrong-task examples by case id
first-divergence histogram
manual/background downgrade count
unsupported claim count
locked-holdout access status
release gate status and explicit violations
```

## Verification

```text
cargo test continuation::accuracy_fixture
cargo test continuation::accuracy_eval
cargo check
run privacy lint against every committed fixture
run deterministic replay twice and compare task identity/checkpoints
generate the Task Truth v2 baseline report
```

Inspect the generated report. A command exiting zero is not proof if it silently skips live cases or human labels.

## Definition of done

- Session-013 is represented faithfully enough to reproduce the original semantic failure.
- Source input contains no expected role/task labels.
- Human labels and product outputs are separate.
- Partitions are application-level.
- The evaluator compares all available paths on identical evidence.
- Missing cases, labels, paths, and holdouts keep the release gate false.
- Privacy lint and deterministic replay pass.
- The report gives Task Truth v2.03 a stable measurement contract.

Do not tune the multimodal model in this goal. Do not mark the overall program complete.
