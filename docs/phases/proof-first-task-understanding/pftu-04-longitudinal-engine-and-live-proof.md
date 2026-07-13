# PFTU-04 — Implement the Winning Longitudinal Engine and Prove It Live

## Codex implementation prompt

Implement the architecture selected by PFTU-03 inside the existing Task Truth v2 and model-first path. Then prove that the production Continue answer understands real cross-application behavior.

This phase completes root problem 2. It must not merely add episode tables, task-thread revisions, or green fixtures. The visible answer must improve on fresh workflows.

## Hard dependency gate

Read `pftu-03-completion-audit.md` first.

Stop unless its final verdict is exactly:

```text
PASS — architecture selected and PFTU-04 may begin
```

Verify that:

- the corpus and held-back cases exist;
- the winning architecture passed every semantic gate;
- its provider/model/request contracts are recorded;
- production remained unchanged during the bake-off;
- root problem 1 remains passing under PFTU-02.

If the audit does not prove a winner, do not implement a hierarchical engine by preference. Return to PFTU-03.

## Core idea

Implement only the measured winner. The likely winner may be hierarchical boundary interpretation followed by task synthesis, but this prompt does not assume that result.

The final production flow must preserve three separate responsibilities:

1. Local evidence code decides what was observed, when it happened, whether it is private, and whether a target is safe to open.
2. Cloud semantic reasoning decides what the observed sequence means for the person’s task.
3. Public-answer code admits one atomic, verified task interpretation or an honest unresolved state.

No local scorer, activity classifier, application name, duration heuristic, or openable target may independently decide the main task.

## Required reading

- all PFTU prompts and completion audits;
- the selected PFTU-03 request builders and results;
- existing Task Truth v2 observation packets, snapshots, model client, verifier, production authority, review, task-thread, cache, and audit code;
- existing capture, semantic checkpoint, and episode infrastructure;
- current SQLite migrations and live rows;
- React and native-island presentation and correction paths;
- current target selection and open-time validation;
- the existing MFTI final release gate and audit.

Trace the current explicit Continue path end to end before editing.

The existing repository already has observation packets, semantic checkpoints, task snapshots, task-thread revisions, atomic public answers, feedback scope, cache identity, and release audits. Treat them as reusable implementation, not a checklist to rebuild. Produce a requirement-to-existing-code map first. Modify an existing structure only when the PFTU-03 winner needs behavior it cannot express or when live evidence proves the current behavior wrong.

## Implementation requirements

### 1. Extend existing temporal evidence instead of creating another capture system

Reuse the current observation packet, semantic checkpoint, capture, and episode infrastructure where it contains real data. Repair or simplify structures that are fixture-only.

If the selected architecture needs a durable evidence-bound episode packet, add an additive Task Truth v2 contract containing:

- packet id and version;
- ordered start/end timestamps;
- boundary reason;
- before/after frame references;
- exact evidence hashes;
- owned Accessibility/OCR observations;
- grounded operations;
- semantic-neutral deltas;
- app/window/page/document identity where observed;
- privacy decisions;
- missing-evidence notes.

Do not put local task names, intent labels, relationship labels, or next actions in the local episode packet.

### 2. Run semantic processing only after explicit Continue

Capture and semantic-neutral boundary construction may happen locally before Continue under existing privacy rules. Images or private semantic evidence may be sent to a provider only after an explicit Continue action.

The explicit Continue flow should:

1. persist a fresh current external frame or return a typed capture failure;
2. close the current boundary;
3. select the current causal chain plus meaningful earlier boundaries under the winning policy;
4. run the winning cloud interpretation stages;
5. validate and persist results;
6. update the task-thread revision if required by the winning design;
7. compose one atomic public answer;
8. attach a return target only after local safety checks.

No background path may perform steps 4 through 7.

### 3. Persist verified semantic episodes only when the winner requires them

If the winning architecture includes a lower-level cloud boundary stage, add an additive verified semantic-episode contract. It must bind:

- semantic episode id and schema;
- exact evidence-packet id and content hash;
- provider, model, request, and response ids;
- prompt and response-schema versions;
- admitted semantic fields and support slots;
- field-level confidence and rejection reasons;
- privacy and verification status;
- creation time.

Reuse a semantic episode only when its entire evidence identity and semantic policy identity are unchanged. A similar-looking window or application is not the same evidence.

Do not write these semantics into legacy `continue_episodes` fields that local ranking can reinterpret.

### 4. Synthesize the task journey in the cloud

The task-level call receives only:

- selected verified boundary interpretations or the winning equivalent;
- a bounded current visual state if required;
- one to three relevant prior Task Truth thread heads;
- explicit missing-evidence and contradiction notes.

It must decide, when supported:

- primary task;
- current subtask;
- last meaningful progress;
- unfinished state;
- whether each material segment is main work, supporting research, verification, waiting, temporary detour, interruption, return, completion, supersession, or unrelated;
- whether the current task continues a prior thread or starts a new one.

Maintain one to three alternatives only when the PFTU-03 winner proved that alternatives improve truthfulness. Do not restore multi-hypothesis complexity by default.

Continuity requires both current and prior evidence. Recency, duration, application identity, repeated window title, candidate score, or target openability cannot prove continuity by itself.

### 5. Update task threads atomically

Use the existing task-thread tables and revisions. Do not create a second memory system.

One revision must bind:

- selected temporal input identity;
- all cloud stage request/response identities;
- selected interpretation or hypothesis;
- field-level support;
- prior revision when continuity is supported;
- correction watermark;
- public answer identity.

Never merge a task from one response, progress from another response, and unfinished state from an older revision.

New unresolved evidence must defeat stale precision unless the current synthesis explicitly proves continuity. Completion and supersession require current evidence or a scoped human correction.

### 6. Compose a task-first public answer

The public answer should contain only supported fields:

- primary task;
- current activity as a concrete step within or relative to that task;
- last meaningful progress;
- unfinished state;
- grounded next action only when supported by unfinished state or an explicit plan;
- supported location or target;
- one bounded alternative only when genuinely useful.

Never use internal activity classes as the headline. “Editing,” “viewing,” “browsing,” “reviewing,” “typing,” or “filling a form” is not a useful task answer without the concrete task object and purpose.

Every public sentence must trace to one selected task-thread revision and the exact semantic claims that support it.

### 7. Keep target selection subordinate

The selected task-thread revision constrains eligible targets. Local open safety still has final authority over whether the target can be opened.

The system must support these honest combinations:

- task understood and safe target available;
- task understood but no safe target available;
- task ambiguous with two bounded alternatives;
- task unresolved with observable evidence only.

Never substitute a recent or easy-to-open surface for the selected task.

### 8. Preserve PFTU-02 truth guarantees

All PFTU-02 invariants still apply:

- verified cloud, scoped human correction, or unresolved are the only semantic sources;
- React and island share one atomic answer;
- local fallback is forbidden;
- cache and adoption require the complete identity;
- explicit negative feedback is strong and narrow;
- inferred timeout, dismissal, or empty-target events cannot suppress unrelated tasks;
- unsafe opens and background uploads remain zero.

## Live workflow proof

Run at least fifteen fresh, privacy-approved macOS workflows. Write expected labels before pressing Continue. Do not rely only on the PFTU-03 corpus.

Include:

1. Coding in VS Code, verification in terminal, return to code.
2. Asking Codex for a change, inspecting code, reviewing its output.
3. Browser research supporting a coding task.
4. API dashboard work supporting the same task.
5. Browser-heavy research across many tabs.
6. Waiting for an agent.
7. Waiting for a command or build.
8. Short unrelated detour and return.
9. Long unrelated detour and return.
10. Genuine task switch.
11. Completed task followed by a new task.
12. Two plausible tasks.
13. Unseen application.
14. Weak current evidence that should remain unresolved.
15. A full session-038 reconstruction across Codex, VS Code, browser research, and an API dashboard.

For every Continue boundary, record:

- expected primary task and current subtask;
- expected segment relationships;
- expected progress and unfinished state;
- actual public answer and source kind;
- selected temporal boundaries;
- semantic episode ids if used;
- selected task-thread id and revision;
- provider request/response ids;
- field support and rejection reasons;
- target/open outcome;
- React/island parity;
- human verdict and correction;
- latency, tokens, and estimated cost.

Commit only redacted labels and aggregate evidence. Never commit screenshots, raw OCR/Accessibility content, provider payloads, private URLs or paths, databases, credentials, raw typed characters, or clipboard contents.

## Acceptance gates

Use the existing MFTI thresholds where denominators are large enough, and report exact fractions for this fifteen-workflow development proof.

All of these are mandatory:

- confident wrong primary task: zero;
- visible-surface substitution: at most one of fifteen;
- useful non-generic task summary: at least 90% of recoverable cases;
- correct primary task: at least 88% of recoverable cases;
- correct or partly-right progress and unfinished state: at least 90% of recoverable fields;
- correct or partly-right segment relationship: at least 90% of labeled segments;
- correct return or task-switch boundary: at least 90% of applicable workflows;
- human usefulness: at least 85%;
- unsupported public claims: zero;
- mixed atomic identities: zero;
- local-semantic fallback: zero;
- stale-current claims: zero;
- unsafe or task-inconsistent opens: zero;
- background uploads: zero;
- privacy violations: zero.

The fifteen-workflow proof is a development gate, not a substitute for the larger locked MFTI release corpus. Keep the formal release report honest. If it still lacks 200 independently reviewed boundaries, 50 holdout cases, or its manual manifest, state that clearly.

## Automated verification

Add tests for the winning architecture, including:

- semantic-neutral boundary creation;
- long workflows not reduced to unrelated newest frames;
- old evidence not starving the current causal chain;
- semantic-episode identity and reuse, when applicable;
- partial-stage provider failure;
- task synthesis across support, detour, return, completion, supersession, and ambiguity;
- current unresolved evidence defeating stale precision;
- atomic task-thread revision updates;
- no cross-response field mixing;
- public sentence provenance;
- cache and correction invalidation;
- target ownership and open safety;
- React/island parity;
- no background provider call;
- privacy exclusions.

Run:

```bash
cd src-tauri
cargo fmt --all -- --check
cargo check
cargo test task_truth_v2 --lib
cargo test continuation --lib
cargo test session_island --lib
cd ..
npm run build
npm run test:webview
git diff --check
```

Run the selected architecture’s replay and live-evaluation commands and include their exact output counts.

## Forbidden work

Do not:

- implement an architecture that did not win PFTU-03;
- add a third semantic engine;
- use local activity classes to name tasks or relationships;
- send broad raw history to the model;
- upload without explicit Continue;
- capture raw typed characters or clipboard contents;
- weaken truth, privacy, feedback, freshness, cache, or open-safety gates;
- lower evaluation thresholds after seeing results;
- claim release readiness from fifteen workflows;
- claim success because tables, schemas, migrations, or fixtures pass.

## Required completion audit

Create `pftu-04-completion-audit.md` beside this file. It must include:

- the PFTU-03 winner and any implementation deviation;
- complete data flow and component responsibilities in plain English;
- migrations and compatibility impact;
- every live workflow result;
- exact gate numerators and denominators;
- latency, token, and cost results;
- automated verification;
- evidence chain and task-thread revision behind every public sentence in at least the session-038 reconstruction;
- regressions found and fixed;
- engineering behavior proven live;
- formal release evidence still missing.

End with exactly one verdict:

- `PASS — root problem 2 is proven in the development build`, or
- `INCOMPLETE — longitudinal understanding remains unproven`.

Do not use the word “solved” unless every live acceptance gate passes.
