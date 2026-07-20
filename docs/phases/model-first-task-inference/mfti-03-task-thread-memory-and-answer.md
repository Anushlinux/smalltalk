# MFTI-03 — Track The User's Task Across Screens And Produce The Right Answer

## Codex task

Build the stateful task-thread layer on top of the working cloud multimodal resolver from MFTI-02. Smalltalk must maintain competing task hypotheses across applications, distinguish support work and detours from task switches, prevent stale or cross-session truth from winning, and generate one honest Continue answer from one coherent inferred task state.

This is an implementation session. Do not add another snapshot type beside existing Task Truth structures unless migration is unavoidable and explicitly proven. Repair the current `TaskSnapshot`, checkpoint, selection, production, and answer-composition path in place.

Local code still must not infer semantic task meaning. The cloud model produces semantic hypotheses and relationship judgments. Local code may maintain hypothesis identity, confidence history, evidence lineage, recency, supersession, user corrections, and deterministic safety rules.

## Dependency gate

MFTI-01 and MFTI-02 must be complete in the current worktree.

Before editing, verify:

- recent model requests contain readable current and earlier keyframes;
- interactions and before/after deltas are present;
- at least three real cloud multimodal calls succeeded;
- provider failure does not fall back to local semantic labels;
- model output separates observed surface, operation, primary task, subtask, and relationship.

If no successful live provider evidence exists, do not build task memory on mocked semantics. Finish MFTI-02 first.

Read:

```text
AGENTS.md
docs/phases/model-first-task-inference/mfti-01-model-ready-observation-stream.md
docs/phases/model-first-task-inference/mfti-02-cloud-multimodal-task-inference.md
docs/phases/task-truth-v2/tt2-03-observation-packets-task-snapshots-and-checkpoints.md
docs/phases/task-truth-v2/tt2-05-task-first-continue-and-final-release-gate.md
docs/phases/task-truth-v2/tt2-05-completion-audit.md
src-tauri/src/continuation/task_truth_v2/task_snapshot.rs
src-tauri/src/continuation/task_truth_v2/checkpoint.rs
src-tauri/src/continuation/task_truth_v2/selection.rs
src-tauri/src/continuation/task_truth_v2/production.rs
src-tauri/src/continuation.rs
src/App.tsx
src-tauri/src/session_island.rs
```

## Current failures that must be removed

Reproduce and verify these failures before editing:

- Continue requests may lack `session_id`.
- Snapshot selection can search globally when session scope is absent.
- Current unresolved snapshots are filtered out, allowing an older selected snapshot to win.
- Age contributes only a soft score; there is no strong session/task-continuity requirement for stale snapshots.
- `attach_observed_activity` can overwrite task summary/object/location while leaving old `last_meaningful_progress` and `unfinished_state`, creating a mixed-task answer.
- Current visible activity can replace task identity instead of being represented as a subtask, detour, support action, or unrelated activity.
- All workstream `inferred_intent` values were null in the diagnosed live state.
- The final product can say `Browsing X` even when the user had an earlier unresolved engineering task.

The solution is not to make stale snapshots win more often. It is to maintain a small, evidence-linked set of task threads and let the cloud model update their semantic relationships.

## Required task-thread model

Maintain a bounded set of active or recently active task threads. Each thread must have:

```text
task_thread_id
semantic identity and task object
session lineage
current verified model hypothesis
observation packet and response provenance
first/last supported time
execution state
last meaningful progress
unfinished state
current subtask or surface activity
relationship history
supporting and contradicting evidence
confidence history
status: active, background, interrupted, completed, superseded, unresolved
revision
human corrections
```

A task thread is not an app, window, browser tab, artifact, or workstream. Those are evidence attached to a thread.

## Goal 1 — Establish task identity and update rules

Define deterministic identity rules around model output and evidence lineage.

Requirements:

- A model hypothesis may update an existing thread only when it declares a supported continuity relationship and cites current evidence plus the prior thread revision.
- A new-task relationship creates a new thread instead of mutating the old task.
- Supporting research, verification, and temporary detours update the current subtask/activity while preserving the primary task.
- An interruption may move a thread to background without completing it.
- Returning to prior artifacts can reactivate a thread only when the model and causal evidence support continuity.
- Completion requires model-supported completion evidence or explicit human confirmation; inactivity alone is insufficient.
- Contradictory new evidence can supersede a thread.
- Two close hypotheses remain separate choices instead of being merged into vague prose.

Thread identity must never be inferred from openability, candidate score, URL richness, window title length, or the availability of a return target.

## Goal 2 — Make recency and unresolved truth safe

Fix snapshot/thread selection so uncertainty in the present defeats unsupported certainty from the past.

Required invariants:

- A current unresolved inference cannot be discarded merely because an older snapshot is `selected`.
- An old task cannot become current without explicit continuity evidence.
- Session-local evidence is the default scope.
- Cross-session continuation requires an evidence-backed model relationship or explicit user correction.
- Stale confidence decays, but decay must not invent a replacement task.
- A completed or superseded task cannot silently reactivate.
- A new foreground surface is not automatically a task switch.
- If no thread is sufficiently supported, return unresolved with bounded alternatives.

Add hard rejection reason codes for session mismatch, missing continuity edge, superseded thread, stale unsupported snapshot, task-object mismatch, and conflicting current evidence.

## Goal 3 — Make every task state atomic

One public answer must derive from exactly one task-thread revision and one selected model hypothesis.

Prohibit partial semantic overwrites. In particular:

- Do not overwrite `task_summary` and `task_object` from current focus while retaining old progress/unfinished fields.
- Do not combine task identity from one snapshot, state from another, and next action from a local workstream.
- Do not let a return target rewrite the task.
- Do not let observed activity rewrite the primary task.
- Do not retain stale fields when a new hypothesis intentionally replaces a task.

Represent observed surface and immediate operation as separate nested evidence/state, not as replacements for task identity.

Persist an atomic identity equivalent to:

```text
task_thread_id
task_snapshot_id
snapshot_revision
selected_hypothesis_id
model_response_id
observation_packet_id
evidence watermark
```

Every semantic field in the public answer must trace to that atomic identity or an explicit human correction on that identity.

## Goal 4 — Keep multiple task hypotheses when needed

Maintain at most a small bounded number of live task threads and one to three hypotheses per unresolved boundary.

For each candidate hypothesis, preserve:

- model confidence;
- supporting/contradicting evidence;
- continuity relationship;
- thread identity;
- last supported time;
- reason it was retained, demoted, superseded, or selected.

Do not average incompatible tasks into a generic summary. If two interpretations remain close, the answer should present two concise choices and allow the user to select or reject one.

## Goal 5 — Design the public answer around task, state, relationship, and uncertainty

Create or revise the public answer contract so it can express:

```text
You were — likely primary task and work object
Currently — immediate activity/subtask and its relationship to the primary task
State — last meaningful progress and unfinished/waiting/blocked state
Next — one model-supported next action, or omitted
Where — current or return location at supported precision
Alternative — shown only when genuinely close
Action — open only through strict local target validation
```

Examples of required behavior:

### Strong continuity

> You were debugging why Smalltalk's Continue result loses the user's real task. You opened AI-related posts on X as supporting research. The unfinished work is repairing the evidence-to-inference path.

### Probable but uncertain relationship

> You had been working on Smalltalk's task-understanding problem. Your latest activity was reading AI-related posts on X, which may have been supporting research, but that connection is not certain.

### Unrelated or new task

> You started a separate task: comparing wallpaper files in Google Drive. Your earlier Smalltalk debugging task remains unfinished in the background.

### Insufficient evidence

> I can see that you were reading a Dwarkesh Patel post on X, but I cannot determine whether it belonged to your earlier Smalltalk work.

### Provider unavailable

> I captured your recent activity, but task inference is unavailable right now.

The last two states must not be converted into `Browsing X` as the entire task answer.

## Goal 6 — Integrate user correction at the right semantic level

`Not right` feedback must attach to:

```text
task thread
snapshot revision
hypothesis id
affected field or relationship
evidence watermark
```

Allow the user to:

- reject the selected task;
- choose an alternative hypothesis;
- say the current surface was unrelated;
- say it was supporting work;
- mark a task complete;
- reactivate a prior task.

Corrections may become `semantic_source = human_correction` for that scope. They must not globally poison unrelated artifacts, domains, apps, or future sessions.

## Goal 7 — Keep opening separate from understanding

The cloud model may infer the task and suggest a next step. It may not authorize opening a URL, file, tab, or application.

Strict local target validation remains responsible for:

- locator existence;
- freshness;
- task-thread identity match;
- privacy and suppression policy;
- open-time revalidation.

High task confidence may coexist with no direct return target. No return target may increase semantic task confidence.

## Required tests

Add deterministic and integration tests for:

1. Supporting browser research preserves an engineering primary task.
2. A genuine new task creates a new task thread.
3. A temporary detour does not overwrite the primary task.
4. Current unresolved evidence defeats an unsupported stale selected snapshot.
5. Cross-session task reuse requires explicit continuity evidence.
6. Completed and superseded threads do not reactivate silently.
7. Atomic snapshot composition prevents mixed-task fields.
8. Target mismatch nulls the target without changing the task.
9. Two close model hypotheses remain separate.
10. Human selection promotes only the chosen scoped hypothesis.
11. Relationship correction does not suppress the entire app/domain.
12. Provider failure produces unresolved state and preserves prior thread only as prior context.
13. Cache identity changes with session, thread revision, model response, correction, or evidence watermark.
14. React and island serialize the same semantic answer contract.

## Required live longitudinal verification

Run at least five private, newly captured multi-boundary sequences with the real provider:

1. Same task across editor, terminal, browser research, and return.
2. Brief unrelated detour followed by return.
3. A new task that supersedes the old foreground task.
4. Two plausible tasks with explicit user choice.
5. A current unresolved boundary with an older confident snapshot present.

For each boundary, record human labels for:

```text
visible surface
immediate operation
primary task
current subtask
relationship
last progress
unfinished state
next action, if supported
task thread selected
```

The audit must show thread revisions and why the selected thread won. Explicitly test that no field came from another task revision.

Do not commit private screenshots, raw provider payloads, or personal history.

## Verification commands

Run:

```bash
cd src-tauri && cargo fmt --check
cd src-tauri && cargo check
cd src-tauri && cargo test task_truth_v2
cd src-tauri && cargo test continuation
cd src-tauri && cargo test session_island
npm run build
```

Run the five live longitudinal scenarios in the Tauri app and retain only privacy-safe review metadata.

## Definition of done

This session is complete only when:

- task threads are distinct from apps, artifacts, workstreams, and visible surfaces;
- cloud model relationships update thread state without local semantic inference;
- current unresolved evidence cannot be replaced by stale confidence;
- cross-session continuity requires proof;
- public semantic fields are atomic to one thread/snapshot/hypothesis revision;
- support work, detours, interruptions, new tasks, and returns behave differently;
- the answer can express current activity and its relationship to the primary task;
- provider failure remains honest and non-semantic;
- all required tests pass;
- five new live longitudinal sequences are reviewed and the completion report audits every invariant.

Do not mark this goal complete because thread tables and selection tests exist. The live provider must maintain the correct task through real app switches and task boundaries.

## Handoff to MFTI-04

Report:

- task-thread schema and migration;
- identity, update, decay, supersession, and selection rules;
- atomic answer provenance;
- feedback semantics;
- exact tests and live longitudinal results;
- remaining wrong-task examples;
- the production contract MFTI-04 will make authoritative.
