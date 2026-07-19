# LCA-01 — Task-Relevant Evidence Packet And Pane Isolation

## Codex implementation task

Repair the evidence packet used by the normal manual `Continue` path so the model receives the smallest set of evidence that explains the newest unfinished task. The packet must stop rewarding busy or text-heavy surfaces, stop mixing unrelated panes and adjacent chats into the primary task, and preserve the latest user request, the latest agent/result state, and the boundary between completed work and current work.

This is the first phase of the Launch Continue Accuracy repair. Implement it completely before beginning LCA-02.

This phase changes evidence selection and model-input shaping only. It must not redesign the public answer, loosen semantic validation, change first-screen UI copy, create a second model request, or weaken target safety.

## Why this phase exists

The four supplied provider logs prove that the compact request is small enough to transport but is not consistently task-relevant:

```text
resp_05cd15c928f2b22f006a5bf31d506481a0987b22d9e16500b9-full-log.json
resp_0d1ccaabe671fd37006a5bf51ade34819ebe5fa0f01a250b8e-full-log.json
resp_0056a4be4033902f006a5bfb214a50819ca241ce0c2a3ec577-full-log.json
resp_0e34cbea9d0858af006a5bfb615f7c81929e12bbc2bf4b5197-full-log.json
```

The recurring failures are:

1. The request can include a prior completed task and a newer unfinished task without clearly separating them.
2. A dense adjacent ChatGPT or browser pane can influence the inferred task even when it is only supporting or unrelated material.
3. Representative-frame ranking uses interaction volume and dwell as major signals. A busy surface can outrank a quieter surface containing the actual user request.
4. Assistant-generated prose is longer and visually louder than the user's request, so it can become the semantic center.
5. Nearly identical high-resolution images consume attention and provider input without adding a new task fact.
6. `carried_into_current_surface` is described to the model more strongly than its local hostname-text check justifies.

The 0056 case is the critical regression. The correct state was that implementation had completed and user verification remained. The request also exposed an adjacent PFTU discussion, allowing the output to mix the release-gate topic into the visual-cue task.

## Hard operating rules

- Read `AGENTS.md` and obey it.
- Inspect `git status --short` before editing. Preserve all unrelated user changes.
- Do not reset, restore, clean, or broadly reformat the checkout.
- Do not use Computer Use, AppleScript UI automation, Chrome control, the in-app browser, screenshots of the running app, mouse clicks, or keyboard automation.
- Do not perform live visual QA. The user will test the finished five-phase program manually.
- Do not claim a GUI, native-island, or live-provider pass.
- Deterministic unit tests, replay tests, builds, Rust checks, and Swift type-checking are allowed.
- Do not upload private captures as part of automated testing.
- Do not commit raw provider logs, screenshots, SQLite databases, raw URLs, raw file paths, typed characters, clipboard text, or credentials.
- Keep manual `Continue` at one provider POST with zero automatic model reconciliation requests and zero HTTP retries.
- Keep background and startup semantic uploads disabled.
- Keep Smalltalk's own UI excluded as task evidence.
- Keep privacy-ineligible images blocked before transport.
- Do not revive the browser-extension MVP lane.

## Required reading

Read the current versions of:

```text
AGENTS.md
PRODUCT.md
docs/phases/lean-continue/01-compact-semantic-episode-input-completion-audit.md
docs/phases/lean-continue/03-single-request-production-cutover-completion-audit.md
docs/phases/p6-task-turn-accuracy/p6-00-task-turn-accuracy-program.md
docs/phases/p6-task-turn-accuracy/p6-01-ground-truth-replay-eval.md
docs/phases/p6-task-turn-accuracy/p6-02-role-region-turn-evidence.md
docs/phases/p6-task-turn-accuracy/p6-03-current-task-turn-lifecycle.md
docs/phases/p6-task-turn-accuracy/p6-09-completion-audit.md
docs/phases/proof-first-task-understanding/pftu-01-completion-audit.md
```

Inspect the live implementations and tests for:

```text
src-tauri/src/continuation/task_truth_v2/observation_packet.rs
src-tauri/src/continuation/task_truth_v2/semantic_probe.rs
src-tauri/src/continuation/task_turn_evidence.rs
src-tauri/src/continuation/task_turn_regions.rs
src-tauri/src/continuation/task_turn.rs
src-tauri/src/continuation/accuracy_eval.rs
src-tauri/src/capture.rs
src-tauri/tests/fixtures/continue_accuracy/
```

The live checkout overrides prompt-era line numbers and assumptions. Reuse correct P6 region, speaker, ownership, task-turn, privacy, and replay contracts instead of creating a parallel evidence architecture.

## Required baseline audit

Before changing code, produce a concise private working map of the normal manual path:

```text
manual Continue cutoff
  -> current frame persistence
  -> observation packet
  -> surface timeline
  -> boundary selection
  -> context-image ranking
  -> image preparation
  -> compact semantic request
```

For every selected image and structured evidence item in each supplied log, identify:

- why it was selected;
- whether it contains the latest user request;
- whether it contains the latest agent/result state;
- whether it contains completed prior work;
- whether it contains a separate pane or conversation;
- whether its selection depended on interaction count, dwell, recency, or task relation;
- whether it added a new semantic fact beyond an already selected image;
- whether the image was sent at its original dimensions;
- whether its role could be established before the model call.

Do not treat the model's answer as ground truth while performing this audit. The expected task states are defined below.

## Ground truth for the four critical logs

### 05cd — product-need answer review

```text
Newest unfinished task:
Review the conversational answer assessing whether the proposed screen-aware context-reconstruction product solves a real need.

Latest meaningful state:
The answer has begun and affirms that the product addresses attention residue and context reconstruction. More of the response remains to be reviewed.

Prior/supporting context:
The surrounding discussion that motivated the product-need question.
```

### 0d1c — newer visual-cue request after completed backend work

```text
Newest unfinished task:
Add an answer-linked visual cue to the real island output and expose it as a separate block under Show more.

Completed prior context:
The backend connection and arrow-output flow were already implemented and verified.

Detour/supporting context:
The adjacent product-launch checklist is not the current implementation task.
```

### 0056 — implementation complete, user verification remains

```text
Newest unfinished task:
Verify the completed answer-linked visual cue in the real Smalltalk island.

Completed prior context:
The visual-cue implementation is complete and focused checks passed.

Unrelated or supporting pane:
The adjacent PFTU release-gate explanation must not become part of the primary task unless explicit evidence links the verification to that discussion.
```

### 0e34 — regression investigation from an unsent draft

```text
Newest unfinished task:
Investigate why the latest Continue attempt was reduced to an insufficient-evidence answer after the visual-cue work.

Latest meaningful state:
A complaint/regression report is being drafted but has not been submitted.

Unproven claim:
The visual-cue implementation caused the semantic rejection. Preserve it as a hypothesis, not a fact.
```

## Required evidence-selection contract

### 1. Task relevance outranks surface activity

Replace cross-surface semantic ranking based primarily on engagement with an explicit task-relevance policy.

The preferred evidence order is:

1. Latest high-confidence user-authored goal, request, correction, or unsent draft associated with the current task turn.
2. Latest agent, tool, editor, terminal, or result state that directly answers or advances that same task turn.
3. The completion boundary of the immediately prior task when it is needed to prove that the newer task supersedes it or begins after it.
4. One pre-detour task-bearing frame when the current surface is a temporary detour or return.
5. The current frame as factual current-state evidence, without automatically making it the primary task.

`interaction_count`, `engagement_score`, dwell, frame count, and recency may break ties among evidence already proven to belong to the same task turn. They must not establish cross-surface task relevance or allow a busy unrelated surface to displace the newest user goal.

Keep engagement values in audit output if useful. Do not describe them to the model as task authority.

### 2. Reuse P6 role, region, and task-turn evidence

When current P6 evidence exists, the compact packet must consume:

- surface/window ownership;
- pane or region identity;
- conversational role;
- latest user-goal spans;
- current agent/status spans;
- prior task boundary spans;
- current task-turn identity and lifecycle;
- confidence and missing-evidence flags.

Do not flatten an entire window when structured spans can distinguish the user request, agent result, navigation, sidebar, old completion, and adjacent pane.

When structured evidence is unavailable, use a visibly downgraded fallback. A flattened window fallback must never receive the same task-authority label as attributed user/agent spans.

### 3. Isolate multiple panes and conversations

If a screenshot contains multiple panes, tabs, columns, editors, terminals, or chats:

- preserve region boundaries in the request;
- identify which region owns the current user/agent task evidence;
- mark other regions as supporting, unrelated, or unknown before final semantic inference when local evidence can do so;
- never merge text across panes into one chronological conversation;
- never use vertical position alone as reading order across panes;
- never let a long adjacent assistant response dominate a short current user request.

When pane ownership cannot be established, keep the ambiguity explicit and prevent high-confidence task resolution.

### 4. Prefer task-state transitions over repeated screenshots

Select an image only when it adds at least one of:

- a new user goal;
- a new agent or tool state;
- completion of the prior task;
- a task-turn boundary;
- a meaningful detour or return;
- a material artifact/result change;
- a current-state fact not present in another selected image.

Do not send near-identical images merely because their physical hashes differ. Add a deterministic near-duplicate policy based on existing diff, perceptual, region, or stable content signals. Preserve the newer image when it contains the same facts. Preserve both only when the before/after distinction changes task meaning.

### 5. Prepare images for semantic legibility

Do not send a full 3104×1962 mixed-pane screenshot at high detail when a privacy-safe owned task region is available.

Use this order:

1. Privacy-safe crop of the owned task region or active window when geometry and ownership are reliable.
2. Separate privacy-safe crops for two necessary panes with explicit region roles.
3. A bounded full-window variant only when cropping would remove necessary relationship evidence.

Use one deterministic image-preparation path. Record original dimensions, sent dimensions, crop ownership, redaction status, and preparation reason in the request audit. Do not guess a crop when geometry is unreliable.

Preserve enough resolution for model-readable text. The implementation must add deterministic tests for dimension caps and crop ownership rather than relying on a visual claim.

### 6. Downgrade hostname carry

`carried_into_current_surface` must not imply verified semantic continuity merely because an earlier hostname appears in visible chat text.

Either:

- require an attributed, visible source reference connected to the current user/agent task turn; or
- rename/downgrade the fact to a neutral observation such as `hostname_mentioned_in_current_surface`.

The model instruction must say that this fact cannot establish task continuity by itself.

### 7. Preserve compactness and one-request behavior

Keep these hard ceilings unless deterministic evidence proves a smaller safe ceiling:

```text
at most two chronological boundaries
at most four unique prepared images
bounded structured text
one explicit manual provider POST
zero reconciliation requests
zero HTTP retries
```

Compactness is a constraint, not the selection objective. A smaller wrong packet is still wrong.

## Required audit additions

For every selected and rejected candidate image, persist safe diagnostic fields for:

```text
surface and region ownership
task_turn_id when available
candidate evidence role
selection or rejection reasons
latest-user-goal support
current-agent-state support
prior-completion support
same-task relation evidence
cross-pane ambiguity
near-duplicate group
engagement used only as same-task tie-breaker
original and sent image dimensions
crop/redaction/preparation policy
```

Do not add raw user text or image bytes to SQLite audit rows.

## Required deterministic tests

Add focused tests covering:

1. A newer user request outranks older completed work in the same conversation.
2. Implementation-complete plus user-verification-remains selects the verification state.
3. A high-engagement unrelated pane cannot displace a low-engagement current task pane.
4. Assistant prose cannot outrank a short attributed user request merely because it is longer.
5. A PFTU side discussion remains supporting or unrelated in the 0056-shaped fixture.
6. A launch-checklist detour remains non-primary in the 0d1c-shaped fixture.
7. An unsent current draft survives as current task evidence without being treated as submitted.
8. A prior Grok answer plus current continuation of that answer remains one review task.
9. Near-identical images collapse unless task state changed.
10. A mixed-pane image is cropped or represented with explicit pane roles when ownership is reliable.
11. Unreliable pane geometry prevents a guessed crop and caps confidence.
12. Hostname mention alone cannot establish cross-surface task continuity.
13. Image-count, boundary-count, privacy, and text-size limits remain fail-closed.
14. Manual Continue still has one provider post and zero reconciliation posts.
15. Existing P6 Capture-button and contamination fixtures remain passing.

Use the existing replay/eval infrastructure. Do not create a disconnected launch-only test runner when the production replay harness can represent the checkpoint.

The raw supplied logs are diagnostic inputs, not fixtures to copy into source. Create privacy-safe bounded fixture facts that reproduce their causal shapes without embedding private screenshots or full conversation text.

## Phase acceptance criteria

LCA-01 is complete only when:

- The normal manual compact request consumes the task-relevant selection policy.
- Cross-surface engagement cannot establish task relevance.
- P6 role/region/task-turn evidence is reused when available.
- Mixed panes remain separated or explicitly ambiguous.
- Completed work and the newest unfinished request are structurally distinguishable.
- 0056-shaped input keeps visual-cue verification primary and PFTU secondary/unrelated.
- 0d1c-shaped input keeps the visual-cue request primary and backend connection as completed context.
- 0e34-shaped input keeps the unsent regression draft current without asserting causality.
- 05cd-shaped input retains the product-need answer review.
- Near-duplicate images no longer consume semantic budget without a changed task fact.
- Image preparation is bounded, privacy-safe, and auditable.
- Hostname mention alone has no semantic authority.
- One-request, privacy, cutoff, and no-background-upload invariants remain passing.
- Existing P6 accuracy fixtures do not regress.
- All automated verification commands pass.
- No live, GUI, CUA, or user-owned manual result is claimed.

## Verification commands

Run the current equivalents of:

```bash
cd src-tauri
cargo fmt --all -- --check
cargo check
cargo test semantic_probe --lib
cargo test observation_packet --lib
cargo test task_turn_evidence --lib
cargo test accuracy --lib
cargo test continuation --lib
cd ..
npm run build
git diff --check
git status --short
```

If a named test filter matches zero tests, find and run the current exact suite instead of reporting a false pass. Report pass counts and ignored tests.

## Required completion audit

Create:

```text
docs/phases/launch-continue-accuracy/01-task-relevant-evidence-packet-completion-audit.md
```

It must contain:

1. Before/after evidence-selection flow.
2. Exact production files and contracts changed.
3. How P6 evidence was reused.
4. The four supplied-log fixture expectations and actual deterministic checkpoint results.
5. Selected/rejected image reasons for each critical case.
6. Image preparation and token/size measurements available without a live call.
7. One-request, privacy, and background-upload proof.
8. Automated commands and exact results.
9. Remaining limitations reserved for LCA-02.
10. A statement that manual app testing remains user-owned.

End with exactly one verdict:

```text
PASS — LCA-01 evidence packet is proven and LCA-02 may begin
```

or:

```text
INCOMPLETE — LCA-01 evidence packet remains open and LCA-02 is blocked
```

Do not write `PASS` while any acceptance item is unproven.

## Final response format

Report:

1. Core evidence-selection change.
2. Files changed.
3. Four critical-case checkpoint outcomes.
4. Test commands and results.
5. Completion-audit verdict.
6. What LCA-02 may rely on.
7. Exact manual testing intentionally deferred to the user.

