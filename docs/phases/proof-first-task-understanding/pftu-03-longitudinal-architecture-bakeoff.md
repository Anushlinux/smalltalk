# PFTU-03 — Select a Longitudinal Understanding Architecture by Real Bake-Off

## Codex implementation prompt

Execute this phase as a measured architecture experiment. Do not implement the production longitudinal engine yet.

This phase begins root problem 2: Smalltalk must understand the person’s actual focus across movement between applications, pages, research, coding, verification, waiting, detours, returns, and task switches.

## Hard dependency gate

Read `pftu-02-completion-audit.md` first.

Stop unless its final verdict is exactly:

```text
PASS — root problem 1 is proven and PFTU-03 may begin
```

Verify that production truth is already limited to verified cloud meaning, scoped human correction, or unresolved. If local activity labels still provide public task meaning, fix PFTU-02 instead. Longitudinal reasoning built on mixed authority will be impossible to evaluate honestly.

## Core idea

The current system compresses a long workflow into a small number of selected screens and asks one model call to infer both local actions and the larger task. This loses causal order and encourages visible-surface substitution: the latest app or most verbose page becomes the supposed task.

Research suggests that long-horizon computer reasoning benefits from bounded working memory and hierarchical organization. However, Smalltalk must prove which concrete design works on its passive observation data. Do not assume that “episodes” are correct merely because earlier phase documents named them.

Build a replayable bake-off over the same real workflows. Compare the current baseline against at least two materially different temporal inputs. Select a winner from measured semantic quality, truthfulness, latency, and cost.

## Required reading

- PFTU-01 and PFTU-02 prompts and completion audits;
- `docs/phases/task-truth-v2/tt2-03-observation-packets-task-snapshots-and-checkpoints.md`;
- `docs/phases/task-truth-v2/tt2-04-multimodal-resolver-and-evidence-verifier.md`;
- `docs/phases/model-first-task-inference/mfti-01-model-ready-observation-stream.md`;
- `docs/phases/model-first-task-inference/mfti-03-task-thread-memory-and-answer.md`;
- existing observation packet, semantic checkpoint, episode, task snapshot, and task-thread code;
- session-038 capture and output artifacts;
- the current live SQLite schema and capture status path.

Read the implementation, not just the documents. Identify which temporal structures contain real live data and which exist only for fixtures or shadow evaluation.

Reuse the existing replay, observation-packet, provider, redaction, and review utilities wherever they are real and correct. The experimental candidates should differ only in temporal selection and reasoning shape. Do not duplicate capture, persistence, privacy, or scoring infrastructure under new names.

## Research basis

Use these findings to shape the experiment, not to predetermine its result:

- OSWorld shows that real computer tasks span applications and require recovery of hidden or changing state: <https://arxiv.org/abs/2404.07972>.
- ScreenAI shows that extracting interface content is a distinct lower-level capability: <https://arxiv.org/abs/2402.04615>.
- HiAgent reports that organizing long histories into bounded subgoal memory reduces redundant context in long-horizon tasks: <https://arxiv.org/abs/2408.09559>.
- Recent GUI-memory research reports that raw-history replay can overwhelm the model and text-only summaries can lose decisive visual evidence. Treat this as a hypothesis to test on Smalltalk, not as release proof.

## Experimental boundary

This phase may add a replay/evaluation harness, compact experimental request builders, redacted result schemas, and temporary feature-gated provider calls inside Task Truth v2.

It must not:

- change production public answers;
- add production tables or migrations;
- update task-thread heads;
- change opening behavior;
- run cloud calls in the background;
- build a new capture subsystem;
- create another release gate.

## Build the frozen longitudinal corpus first

Capture or reconstruct at least twelve real workflows. Each workflow must be long enough to contain multiple meaningful boundaries, not four isolated screenshots.

Before running any architecture, write a blind human label containing:

- the primary task;
- the current subtask at Continue time;
- the role of each major application segment: main work, supporting research, verification, waiting, temporary detour, interruption, return, or new task;
- the last meaningful progress;
- the unfinished state;
- the point where a real task switch occurred, if any;
- what evidence is insufficient or ambiguous.

Freeze at least four workflows as held-back cases before prompt tuning.

Required workflows:

1. Same task across editor, terminal, browser research, and return.
2. Same task across Codex, VS Code, browser, and an API dashboard.
3. Browser-heavy research with many tabs but one main goal.
4. Terminal verification after an implementation change.
5. Waiting for an agent, then reviewing its output.
6. Short unrelated detour and return.
7. Long unrelated detour and return.
8. Genuine task switch with no return.
9. Completed task followed by a new task.
10. Two plausible current tasks.
11. Unseen application inside an otherwise familiar workflow.
12. Current evidence too weak to recover the task.

The session-038 reconstruction must preserve the actual order and duration of application travel as closely as privacy permits. Do not label the expected answer by reading Smalltalk’s prior output.

## Compare three architectures

All candidates must use the same frozen workflows, privacy-approved evidence, model pair, and scoring rubric.

### Candidate A — Current baseline

Run the current production or shadow model-first request as it exists at the start of this phase. Preserve its current image limit, packet selection, schema, and verifier. This is the control.

### Candidate B — Salience-selected storyboard

Select a larger but bounded chronological storyboard from semantic-neutral boundaries. Include the current state plus earlier material changes. Use one cloud synthesis call and a response contract no larger than the proven PFTU-01 semantic contract plus relationship fields.

This tests whether better temporal selection alone is enough.

### Candidate C — Hierarchical boundary interpretation

Use two cloud stages after explicit authorization:

1. Interpret each meaningful boundary or small adjacent group using a compact screen/event schema.
2. Synthesize the task journey from those interpretations plus a bounded current visual state.

Each boundary interpretation should separate:

- observed work object or surface;
- grounded user operation;
- semantic effect;
- progress or completion signal;
- possible relationship cues;
- uncertainty and missing evidence.

The synthesis should decide:

- primary task;
- current subtask;
- role of each meaningful segment;
- progress;
- unfinished state;
- continuation, return, or task switch.

Do not feed legacy local semantic labels as model truth. Do not use the human answer as model input.

## Temporal selection rules to test

Use existing capture and observation infrastructure. Candidate B and C may select boundaries from locally observed events such as:

- application or window switch;
- navigation or document identity change;
- committed typing without storing raw keystrokes;
- submit or send;
- command start and output completion;
- material visual or owned-text change;
- inactivity followed by return;
- manual Continue.

Local boundary detection is semantic-neutral. It may say that a change happened, not why it happened or what task it served.

Prevent two failure modes:

- **recency collapse:** only the latest screen survives;
- **history flooding:** old or verbose evidence consumes the request and hides the current causal chain.

Record exactly which boundaries each candidate selected and why under deterministic selection rules.

## Model comparison

For Candidates B and C, compare:

- the selected cost-efficient image-capable model from PFTU-01; and
- the stronger image-capable reasoning model from PFTU-01, refreshed from current official docs if availability changed.

Do not change models between architectures in a way that makes the comparison unfair. If a provider does not support the required ordered images or strict output, record that limitation rather than silently changing the case.

## Scoring

Every workflow and candidate receives human scores without seeing the candidate identity:

- primary task: `Correct`, `Partly right`, `Wrong`, or `Should be unresolved`;
- current subtask: same scale;
- segment relationships: per-segment accuracy;
- last progress: same scale;
- unfinished state: same scale;
- task switch/return boundary: correct or incorrect;
- useful without app-name or generic-verb guessing: yes or no;
- unsupported specificity: yes or no;
- visible-surface substitution: yes or no.

Also record:

- input images, bytes, and tokens;
- output and reasoning tokens where exposed;
- latency per stage and total;
- estimated cost per Continue;
- parse and validation outcome;
- number of selected versus available boundaries.

The evaluator must preserve the raw human labels separately from model output. Product feedback is not automatically a gold label.

## Winner gate

An architecture may win only if it satisfies all of these on the complete corpus and does not regress materially on the held-back cases:

- zero confident wrong primary tasks;
- wrong primary task or visible-surface substitution at most one of twelve cases;
- useful, non-generic primary-task summary in at least eleven of twelve recoverable cases;
- correct primary task in at least ten of twelve recoverable cases;
- correct or partly-right segment relationship in at least 90% of labeled segments;
- correct task-switch or return boundary in at least 90% of applicable cases;
- correct or partly-right progress and unfinished state in at least 90% of recoverable fields;
- unsupported specificity in zero public-eligible answers;
- unresolved chosen when current evidence is insufficient;
- background uploads and privacy violations equal zero.

If more than one architecture passes, select the lowest-cost candidate whose semantic quality is not worse on any zero-tolerance measure and is within one case on primary-task correctness. Explain the practical trade-off.

If no architecture passes, do not choose the least-bad option. Keep PFTU-04 blocked. Diagnose whether the failure comes from capture coverage, boundary selection, lower-level screen interpretation, task synthesis, or model capability, then run a focused second experiment.

## Required verification

Add tests for:

- deterministic corpus freezing and holdout separation;
- boundary selection order and budgets;
- no raw keystroke or clipboard inclusion;
- no background provider call;
- identical case identity across candidates;
- blinded result rows;
- denominator and metric calculations;
- refusal, timeout, invalid output, and partial-stage failure;
- proof that experimental results cannot be adopted by production.

Run:

```bash
cd src-tauri
cargo fmt --all -- --check
cargo check
cargo test task_truth_v2 --lib
cargo test continuation --lib
cd ..
npm run build
npm run test:webview
git diff --check
```

## Required completion audit

Create `pftu-03-completion-audit.md` beside this file. It must contain:

- live versus fixture status of existing temporal structures;
- the frozen corpus and held-back split;
- architecture definitions and exact request budgets;
- selected boundaries for every workflow;
- blind labels and per-candidate results;
- model, latency, token, and cost comparison;
- all pass-gate numerators and denominators;
- failure analysis;
- the selected architecture and why;
- an explicit statement that production was not changed.

End with exactly one verdict:

- `PASS — architecture selected and PFTU-04 may begin`, or
- `INCOMPLETE — no longitudinal architecture is proven and PFTU-04 is blocked`.
