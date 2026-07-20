# MFTI-04 — Cut Production Over To Model-First Task Truth And Prove It Live

## Codex task

Make the cloud multimodal task-thread result from MFTI-01 through MFTI-03 the production semantic authority for Continue, remove local surface classification as a user-facing task fallback, verify React and the native island use the same atomic answer, and prove the result on a reviewed live corpus before declaring the problem solved.

This is the final and potentially longest implementation session. Continue across turns until the actual release evidence exists. Do not redefine completion around architecture, schemas, deterministic fixtures, a shadow report, or one impressive demo.

## End-state promise

After this session, pressing Continue must do this:

```text
capture a fresh boundary
→ construct the truthful chronological observation stream
→ invoke the configured cloud multimodal model
→ verify evidence references and safety locally
→ update/select one coherent task thread or bounded alternatives
→ render the task, current activity relationship, state, and supported next action
→ resolve/open a matching target only through strict local validation
```

It must not do this:

```text
latest window
→ heuristic verb
→ title copied into "what you were doing"
```

If the cloud inference is unavailable or insufficient, the primary answer must say that task understanding is unavailable or unresolved. It may show the observed current surface under evidence details, but it must not present a local surface label as inferred task truth.

## Dependency gate

MFTI-01, MFTI-02, and MFTI-03 must be complete and proven in the current worktree.

Before changing production authority, verify all of the following directly:

- a new manual Continue request has a session id;
- current model input contains readable current pixels and causal evidence;
- at least three successful live cloud inferences exist;
- at least five reviewed longitudinal task-thread scenarios exist;
- provider failure cannot manufacture a local semantic answer;
- one public answer is atomic to one thread/snapshot/hypothesis revision;
- stale cross-session snapshots cannot win without continuity evidence;
- direct return targets remain subordinate to task truth.

If any dependency is missing, repair it before cutover.

Read:

```text
AGENTS.md
docs/phases/model-first-task-inference/mfti-01-model-ready-observation-stream.md
docs/phases/model-first-task-inference/mfti-02-cloud-multimodal-task-inference.md
docs/phases/model-first-task-inference/mfti-03-task-thread-memory-and-answer.md
docs/phases/task-truth-v2/tt2-05-task-first-continue-and-final-release-gate.md
docs/phases/task-truth-v2/tt2-05-completion-audit.md
src/App.tsx
src-tauri/src/continuation.rs
src-tauri/src/continuation/task_truth_v2/production.rs
src-tauri/src/continuation/task_truth_v2.rs
src-tauri/src/session_island.rs
src-tauri/src/session_island/
src-tauri/tests/fixtures/continue_accuracy/task_truth_v2/
```

## Current production failures that must become impossible

Use the retained Continue outputs and current live database to reproduce the baseline:

- final source is `local_scorer`;
- model and response ids are null;
- `current_task_turn` is null;
- Task Truth is `shadow` and release gate is false;
- output is a verb plus visible title;
- observed activity can overwrite only part of an older snapshot;
- React ignores Task Truth unless authority and gate conditions are met;
- provider/model failure still leaves a fluent local answer on screen.

Create explicit regressions for these facts. At the end, no production semantic result may have this combination:

```text
resolved task
semantic_source = local_scorer or local_causal
model_response_id = null
```

Human-corrected task truth is allowed. Unresolved is allowed. Locally invented semantic task truth is not.

## Goal 1 — Define model-first semantic authority

Revise the authority contract so semantic sources are:

```text
cloud_multimodal_model
human_correction
unresolved
```

Local components may still provide:

```text
capture and privacy policy
evidence ordering and grounding
claim/evidence verification
task-thread state mechanics
strict target/open validation
cache and persistence
diagnostics and evaluation baseline
```

They may not provide the primary task summary, task object, semantic activity relationship, unfinished work, or next action.

Keep the old P6/local scorer available only under developer diagnostics for side-by-side evaluation during migration. It must not supply visible semantic fields in authoritative or provider-failure states.

## Goal 2 — Make manual Continue wait for fresh inference

The manual product action should prioritize semantic correctness over returning an instant local guess.

Required behavior:

1. Capture or identify a fresh post-action boundary.
2. Show an honest `Understanding your recent work…` state.
3. Build and send the model request.
4. Verify and persist the result.
5. Atomically adopt it if it is newer and stronger than the displayed revision.
6. Show typed unresolved/provider failure when the call fails.

Do not first display a confident local task and silently replace it later. If an older verified model result is displayed while refreshing, label it as the previous result and prevent it from absorbing new current-surface fields.

Background refresh may organize local evidence but must not upload images or create new semantic task truth unless a separate future product policy explicitly authorizes it. It may adopt an already completed verified inference without downgrading a stronger manual result.

## Goal 3 — Remove legacy semantic fallback and field mixing

Audit every path that builds or adopts a Continue result:

```text
manual desktop request
background refresh
startup restoration
cache hit
native island request/event
quiet refresh
provider failure
validation failure
privacy block
open-time revalidation
```

Requirements:

- No path falls through to `answer.what_you_were_doing` from local surface activity when model-first semantics are expected.
- No path copies a window/page title into task summary as fallback.
- No path partially attaches observed activity to an old task snapshot.
- No path mixes Task Truth and P6 fields on the first screen.
- Cache identity includes session, packet, provider/model, response schema, verifier, task thread/revision, selected hypothesis, feedback watermark, and authority policy.
- An old cached model result is never presented as understanding of newer uncaptured or uninferred activity.
- Rollback means a typed unresolved state plus diagnostics, not legacy semantic authority.

Delete or quarantine obsolete fallback code where safe. Do not leave two silently competing semantic systems.

## Goal 4 — Render one coherent answer in React and the native island

Both surfaces must consume the same versioned public contract.

The primary answer should display, when supported:

```text
You were
[primary task and work object]

Currently
[immediate activity/subtask and relationship]

State
[last progress and unfinished/waiting/blocked state]

Next
[one supported action]

Where
[supported app/document/thread/page identity]
```

Rules:

- Omit unsupported fields rather than fill them generically.
- Show two bounded alternatives when the top hypotheses are close.
- Show task-inference unavailable/insufficient states plainly.
- Put observed screenshots, current focus, confidence detail, provenance, and raw evidence under `Why this answer?`.
- `Continue here` appears only for a matching strict-open target.
- React and island must show the same task-thread revision and inference status.
- The user must be able to reject the task, choose an alternative, or correct the activity relationship.

## Goal 5 — Redesign the release evaluation around the actual promise

Reuse useful Task Truth v2 corpus, privacy, holdout, and audit infrastructure, but correct metrics that assume a local semantic fallback is desirable.

The core human labels for every decision boundary must be:

```text
visible surface/page meaning
immediate user operation
semantic effect of the operation
primary task
current subtask
relationship to prior task
last meaningful progress
unfinished state
supported next action
task switch/completion status
acceptable alternatives
whether the answer is immediately useful
```

Replace `model-on/off critical disagreement = 0` as a release goal. In this architecture, the model-off path is not a competing semantic authority. The correct model-off metric is:

```text
provider unavailable → 100% honest unresolved responses and 0 locally invented semantic tasks
```

Preserve or strengthen these release dimensions:

- wrong primary task;
- visible surface mistaken for task;
- incorrect relationship to prior task;
- wrong task switch versus detour;
- stale/cross-session task leakage;
- mixed-snapshot fields;
- unsupported specific claims;
- task-object accuracy;
- state and next-action accuracy;
- calibrated ambiguity;
- human immediate usefulness;
- unsafe opens;
- privacy violations;
- provider failure honesty;
- unseen/custom application performance;
- latency and cost.

Human ground truth must be written independently of Smalltalk's output. Codex or the product cannot self-label an output and call it reviewed truth.

## Goal 6 — Build a reviewed live corpus that tests causal understanding

Use privacy-safe review artifacts and the existing locked-holdout policy. Meet the existing stronger corpus minimum unless the current frozen policy legitimately requires more:

```text
at least 200 independently reviewed live decision boundaries
at least 50 locked application-level holdout cases
all required surface families
all required interruption, ambiguity, waiting, task-switch, and no-target slices
```

The corpus must include meaningful interaction sequences, not isolated screenshots only:

- editor → terminal → output/error → editor;
- editor/document → browser research → return;
- unrelated browsing detour;
- genuine new task;
- chat user input → assistant/agent output → waiting/review;
- document and spreadsheet work;
- custom-rendered or thin-Accessibility apps;
- browser chrome overlays and tab search;
- multiple displays and visible background windows;
- privacy-blocked frames;
- no active-window crop;
- provider timeout/failure;
- two plausible task threads;
- old confident task plus current unresolved evidence;
- understood task with no safe return target.

Do not weaken the gate because collecting and reviewing real evidence is time-consuming. That evidence is the proof the previous phases never obtained.

## Goal 7 — Freeze hard release gates before holdout tuning

Freeze thresholds before inspecting holdout outcomes. At minimum require:

| Metric | Required release behavior |
| --- | --- |
| Wrong primary task | At most 3% overall and 5% per required surface family |
| Visible surface substituted for task | At most 2% where a broader task is human-labeled |
| Wrong activity-to-task relationship | At most 5% |
| Wrong task switch/detour classification | At most 5% |
| Cross-session stale leakage | 0 |
| Mixed-snapshot semantic fields | 0 |
| Browser/app control as primary task | 0 |
| Unsupported specific claims | At most 1% |
| Provider failure with local semantic fallback | 0 |
| Provider failure honest unresolved | 100% |
| Useful non-generic task summary | At least 90% |
| Task-object accuracy | At least 88% |
| Execution/unfinished-state accuracy | At least 90% |
| Supported next-action precision | At least 90% |
| Human immediately useful | At least 85% |
| Unseen-application useful summary | At least 80% |
| Unsafe opens | 0 |
| Privacy violations | 0 |

Report denominators and confidence intervals. Zero failures over zero reviewed cases is missing evidence, not a pass.

## Goal 8 — Measure latency, cost, privacy, and failure behavior

Measure at least:

- capture-to-packet latency;
- request-build latency;
- provider latency percentiles;
- verification and persistence latency;
- total manual Continue latency;
- image, byte, and token distributions;
- cost per Continue and expected monthly cost under declared usage;
- provider timeout/error/invalid-output rates;
- second-pass frequency and cost;
- privacy exclusions;
- provider-failure user experience.

If inference is slower than desired, optimize frame selection, image size, structured packet size, caching of unchanged verified boundaries, and provider configuration. Do not hide latency by returning a local guess as equivalent truth.

## Goal 9 — Run manual end-to-end macOS QA

Run all existing required Task Truth manual scenarios plus these explicit acceptance scenarios:

1. The diagnosed Smalltalk case: code/docs work followed by AI-related X reading. Confirm the answer distinguishes visible X activity from the likely Smalltalk task and calibrates the relationship.
2. The same sequence with deliberately unrelated X content. Confirm it does not claim supporting research without evidence.
3. Google Drive wallpaper activity after engineering work. Confirm detour versus new-task behavior follows the actual interaction sequence, not the title.
4. Browser tab-search overlay on a non-search page. Confirm the page is not classified as searching unless the search control was used.
5. Provider disabled/timeout. Confirm no `Browsing`, `Editing`, `Reviewing output`, or other local task label appears as inferred truth.
6. Current unresolved evidence with an old selected snapshot. Confirm the old task cannot absorb new surface fields or win without continuity.
7. React/native-island parity.
8. Correct task but no openable locator.
9. Two close hypotheses and user correction.
10. Multiple displays and other-window OCR.

Record build/commit, evidence ids, expected result, actual result, provider/model, reviewer, and pass/fail without committing personal screenshots.

## Required verification commands

Run the relevant narrow tests while iterating, then run:

```bash
npm run build
cd src-tauri && cargo fmt --check
cd src-tauri && cargo check
cd src-tauri && cargo test
cd src-tauri && cargo run --bin task_truth_v2_release_gate
```

Also run any new model-first release generator and inspect the generated machine-readable report manually. Confirm commands ran the intended tests and did not silently select zero cases.

## Final completion audit

Create a requirement-by-requirement completion report. For every goal and release metric, include:

```text
requirement
authoritative source implementation
deterministic test evidence
live reviewed evidence
denominator
status: proven, contradicted, incomplete, missing
remaining work
```

The authoritative release report must bind to:

- exact corpus and holdout identity;
- model/provider and prompt/schema versions;
- observation packet and verifier versions;
- task-thread and answer contract versions;
- performance/cost/privacy policy;
- manual QA manifest;
- source commit/build identity.

## Definition of done

The complete four-session program is done only when:

- production manual Continue invokes the cloud multimodal semantic path;
- the model receives readable chronological screens plus grounded interactions and deltas;
- task threads maintain the user's likely task across support work, detours, interruptions, returns, and real task switches;
- the primary answer distinguishes visible surface, immediate operation, subtask, primary task, and relationship;
- no resolved semantic task can come from `local_scorer`, `local_causal`, a window title, or a heuristic fallback;
- provider failure produces an honest unresolved result;
- stale cross-session and partial-snapshot chimeras are impossible by invariant and test;
- React and island use the same atomic answer;
- strict local open safety remains intact;
- all deterministic builds/tests pass;
- reviewed live corpus, locked holdout, latency/cost/privacy evidence, and manual macOS scenarios meet the frozen gates;
- the final machine-readable release report says `passed = true` with non-zero required denominators and no zero-tolerance violation;
- `PRODUCT.md` describes the implemented model-first behavior and honest failure states;
- old P6/local semantic output remains diagnostic-only and cannot silently regain production authority.

Do not mark completion if the model is still shadow-only, no successful live provider calls exist, the release gate is false, human denominators are missing, or the visible answer can still become `verb + title` when inference fails.
