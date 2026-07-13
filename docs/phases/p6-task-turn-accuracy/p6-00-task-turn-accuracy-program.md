# P6 — Task-Turn Accuracy And Interruption Recovery

## Codex task

Implement P6 for Smalltalk as a sequence of bounded, independently verifiable goals. P6 must make `Continue` accurately answer:

1. What was I actually doing?
2. Where was I doing it?
3. What had just finished, what is active now, and what state did I leave behind?
4. What should I do next?
5. Is there a real place Smalltalk can safely reopen?

This file is the phase map and execution contract. Run the implementation prompts in order. Do not treat this file as permission to implement all phases in one broad change.

## Product vision

Smalltalk is a local-first interruption-recovery product. It should help a user who was distracted, interrupted, or mentally lost recover their exact working state without replaying a session or inspecting a dashboard.

The ideal answer is compact and concrete:

```text
You were investigating what the island Capture button does in Smalltalk.
You were in a chat/agent conversation, and the agent had started tracing the Swift bridge and Rust handler to determine whether Capture starts continuous recording or saves one evidence point.
That investigation was still active: the agent was working and you were waiting for its result. The earlier Continue-card copy task had already finished.
Next: return to the investigation results or inspect the referenced implementation evidence.
I do not have a direct, safe conversation locator, so I will not pretend there is an openable target.
```

Smalltalk must not merely identify the dominant app, repeat an old open-loop label, or produce fluent prose from an internally inconsistent graph.

## Current verified failure

The critical local audit is:

```text
continue_outputs/session-012-session-17836451__continue-1783645410372__normal__continue-decision-
```

Two adjacent decisions from the same short session window are also diagnostic:

```text
continue_outputs/session-012-session-17836451__continue-1783645318852__normal__continue-decision-
continue_outputs/session-012-session-17836451__continue-1783645478796__normal__continue-decision-
```

The three outputs move between generic fallback and the stale Stremio/Helium story. P6 must treat this near-term instability as an accuracy problem, not only judge one selected bundle.

The evidence captured the latest task correctly:

- The previous task, updating Continue-card copy, was complete.
- The latest user question asked what the island Capture button does.
- The active agent state said it would trace the Swift bridge and Rust handler to determine whether Capture records continuously or captures one evidence point.
- The exact state axes were `execution_state = active`, `current_actor = assistant_or_agent`, and `waiting_on = agent`.

The pipeline then lost that truth:

```text
correct active-window evidence
  -> whole-window text flattened without speaker or turn order
  -> old completion text classified as current reviewing/completion
  -> stale inferred feedback promoted an unrelated Helium branch
  -> an old Stremio open loop supplied the objective
  -> P5 received the wrong semantic center
  -> the bounded model repeated the wrong local facts
  -> validation called the recap valid because it matched the pack
```

This is locally manufactured hallucination. The model did not need to invent Stremio or Helium because upstream layers had already made them look authoritative.

## Why P0-P5 did not solve this

P0-P4 strengthened target safety, stale-target suppression, feedback obedience, support-branch gating, weak-surface identity, and island parity. P5 added bounded activity-recap synthesis. Those phases largely answer whether a candidate is safe, eligible, explainable, or openable.

P6 addresses a different primitive:

```text
What is the latest meaningful human-agent task turn, and how does it supersede or continue earlier visible work?
```

P5 cannot repair a wrong task graph. P6 must give P5 a temporally correct, role-aware, lifecycle-aware task object and require every downstream layer to agree with it.

## Authoritative current-state rule

Every P6 session must inspect current state before editing:

```text
AGENTS.md
PRODUCT.md
git status --short
git log -5 --oneline
src-tauri/src/continuation.rs
src-tauri/src/continuation/
src-tauri/src/capture.rs
src-tauri/src/session_island.rs
src-tauri/src/session_island/
src-tauri/macos/SessionIslandPanel.swift
src/App.tsx
src/App.css
```

The live checkout and persisted contracts override line numbers and baseline claims in these prompts. Preserve unrelated user changes. Do not reset, overwrite, or reformat unrelated work.

At the time this program was written, the reviewed baseline was commit `d3f91aaf`, with a clean worktree. That hash is orientation only, not a reset target.

## P6 end-state contract

P6 is complete only when a full raw-evidence replay can produce and prove a first-class current task turn with these concepts kept separate:

| Concept | Required meaning |
| --- | --- |
| Surface identity | Which app/window/page/file/conversation was observed. |
| Evidence ownership | Which text belongs to the active window or artifact. |
| Region role | Chrome, navigation, prior result, user message, agent response/status, editor, terminal, composer, or unknown. |
| Conversational role | User, assistant/agent, system/status, tool output, non-conversation, or unknown. |
| Task turn | The latest meaningful goal plus current working state and its relation to the prior task. |
| Task state | Independent execution state, current actor, and waiting-on axes; for example active + assistant/agent + waiting on agent. |
| Workstream | The durable project/task cluster to which the task turn belongs. |
| Activity recap | A grounded explanation of the task turn, detours, state, and next step. |
| Return target | A directly openable URL/path-backed place, or another deliberately typed and strict-policy-validated direct locator if the product later supports one; otherwise null. |
| Evidence preview | A screenshot/frame/anchor that can be inspected but is not a return target. |

## Global invariants

### Temporal truth

- A completion cue can complete only the task turn to which that cue belongs.
- A newer user request after a completed result starts a new active turn unless evidence proves it is a clarification or child step of the same task.
- Old visible completion text must remain prior context, not become the current task state.
- A single frame may contain several turns. Whole-frame substring matches must not decide current task state.
- Recency alone is insufficient when speaker/region attribution is unknown; uncertainty must remain explicit.

### Semantic-center consistency

- `current_task_turn`, selected workstream, primary recap segment, objective, open loop, last state, and next action must share a task/workstream center or expose an explicit support/detour relationship.
- An unrelated open loop cannot provide the current objective.
- A stale feedback event cannot promote a branch into the current task without fresh, correctly scoped evidence.
- Model-on and model-off paths must agree on task identity for critical fixtures.

### Target honesty

- `return_target` and `resume_work_target` are null without direct openability and a real URL/document path or an explicitly supported typed direct locator.
- `frame_fallback` is evidence preview only.
- High task confidence may coexist with no target.
- UI copy must not say `Continue here`, `return point`, or equivalent target-shaped language when only an evidence preview exists.

### Confidence honesty

- Surface identity confidence is not task-intent confidence.
- Task identity, task state, region/speaker attribution, workstream alignment, and target openability have separate confidence dimensions.
- Public certainty is bounded by the weakest critical dimension for the claim being made.
- A failed or timed-out probe cannot increase confidence.

### Privacy

- Do not store raw keylogged characters.
- Do not store full clipboard text.
- Do not commit local SQLite databases, screenshots, captures, exported audits, raw URLs, raw paths, or personal conversation history.
- Golden fixtures must be redacted, bounded, and privacy-reviewed.
- Model-facing packs remain candidate-bounded and evidence-backed.

### Product boundary

- Continue remains the first-screen primitive.
- Sessions, frames, task turns, confidence vectors, workstream edges, audit checkpoints, and eval metrics remain evidence/diagnostic infrastructure.
- Do not revive the browser-extension MVP lane.
- Do not solve semantic errors with copy-only changes.

## Ordered implementation goals

| Goal | Prompt | Outcome |
| ---: | --- | --- |
| 01 | `p6-01-ground-truth-replay-eval.md` | Create a privacy-safe full-pipeline fixture format, the critical Capture-button golden case, first-divergence reporting, and baseline metrics. |
| 02 | `p6-02-role-region-turn-evidence.md` | Preserve ordered region/speaker evidence and build salient, tail-aware snapshots without flattening whole windows. |
| 03 | `p6-03-current-task-turn-lifecycle.md` | Add a first-class current task-turn object, lifecycle, supersession rules, and task-aware action/semantic-delta derivation. |
| 04 | `p6-04-feedback-scope-provenance-decay.md` | Scope feedback by provenance, time, task turn, workstream, and branch; prevent stale inferred promotion. |
| 05 | `p6-05-workstream-open-loop-consistency.md` | Rebuild branch/open-loop/objective behavior around the current task turn, enforce one semantic center, and establish the internal direct-target/evidence-preview policy. |
| 06 | `p6-06-confidence-and-observation-reliability.md` | Split confidence dimensions and make observe-before-decide failure, timeout, and missing evidence calibrate output honestly. |
| 07 | `p6-07-recap-truth-pack-and-validation.md` | Rebase P5 inputs and validation on current-task truth; reject locally grounded but temporally or cross-layer-wrong narratives. |
| 08 | `p6-08-target-truth-and-answer-composition.md` | Align backend, React, and island target contracts and render a useful interruption-recovery answer without fake openability. |
| 09 | `p6-09-longitudinal-release-gate.md` | Run full replay, counterfactual contamination, model parity, calibration, performance, privacy, and manual interruption QA before release. |

Do not skip Goal 01. Every later goal must add assertions to the same accuracy harness rather than creating a disconnected test system.

## Accuracy ladder

### L0 — Fixture and privacy integrity

- Fixture parse success: 100%.
- Every expected checkpoint is machine-readable.
- Committed fixture privacy violations: 0.

### L1 — Evidence and turn extraction

- Region-role macro-F1: at least 98% on labeled cases.
- Conversational-role macro-F1: at least 98% on labeled cases.
- Latest-user-span and current-agent-status precision/recall: at least 95%.
- Unknown/abstention correctness: at least 95%.
- Latest user-goal accuracy: at least 95%.
- Current agent-state accuracy: at least 95%.
- Task-turn boundary accuracy: at least 95%.
- Prior-completion override on critical fixtures: 0.

### L2 — Semantic graph

- Task-action and semantic-delta temporal accuracy: at least 95%.
- Selected-workstream/current-task alignment: at least 95%.
- Stale inferred-feedback false promotions: 0 on critical fixtures.
- Unrelated open-loop primary selections: 0 on critical fixtures.
- Silent cross-layer contradictions: 0 on critical fixtures.

### L3 — Narrative truth

- Primary-task summary accuracy: at least 90% on the broad labeled corpus.
- Task-summary coverage: at least 95% on labeled summarizable cases.
- Execution/current-actor/waiting-on accuracy: at least 90%, 95%, and 90% respectively.
- Evidence-backed public-claim precision: 100% on critical fixtures.
- Supported next-action precision: at least 95%, with at least 90% coverage/recall when an evidence-backed action is labeled.
- Forbidden stale-term leakage: 0 on critical fixtures.
- Wrong-confident output rate: 0.
- Model-on/model-off task-identity agreement: 100% on critical fixtures.

### L4 — Target truth

- Direct-openability precision: 100%.
- Direct-target recall: at least 95% on labeled safely openable cases.
- Frame fallback exposed as public return/resume target: 0.
- Unsafe opens: 0.
- Main-card/island target-policy disagreements: 0.

### L5 — Longitudinal recovery

- Identical evidence produces identical task identity: 100%.
- Counterfactual stale-contamination invariance: 100% on critical fixtures.
- Interruption/recovery retention: at least 90% on the broad corpus.
- All critical golden cases pass before any claim that P6 is complete.

Start with 20-30 labeled real audits. Grow the release corpus to at least 100 privacy-safe cases spanning chat/agent, editor, terminal, browser research, documents, messaging interruptions, mixed-window captures, thin evidence, and no-safe-target states. Codex cannot self-label cases and claim human review; missing human sign-off leaves the release gate incomplete.

## How to run each prompt in a fresh Codex session

1. Use the prompt file as the session's complete goal.
2. Read `AGENTS.md`, this phase map, `PRODUCT.md`, the current code, and the preceding phase's changes before editing.
3. Create or continue a goal and publish a concise implementation plan.
4. Inspect the current worktree; preserve unrelated changes.
5. Reproduce the relevant baseline failure before implementing.
6. Implement only the named goal and the minimal compatibility work it requires.
7. Add unit, integration, and full-pipeline checkpoint coverage.
8. Run the prompt's verification commands.
9. Audit every definition-of-done item against current evidence.
10. Report files changed, contracts added, tests, metrics, limitations, and what the next prompt may rely on.
11. Mark the goal complete only when every stated gate is proven.

## Whole-program definition of done

P6 is complete only when all of the following are true:

- The critical Capture-button fixture preserves the latest question and current agent state as the active task.
- The earlier completed Continue-card task is prior context.
- `Stremio`, `Helium`, and `Research Report Analysis` cannot become the primary task in that fixture.
- Old inferred feedback and unrelated open loops do not change the active task under counterfactual replay.
- Every final claim can be traced through ordered evidence spans, a task turn, and consistent workstream/recap state.
- P5's optional model may improve phrasing but cannot change task identity or target eligibility.
- No direct locator means public return/resume targets are null and the UI offers evidence inspection only.
- Main React card and native island present the same task, state, confidence, and target policy.
- Every P6.09 zero-tolerance, broad-corpus, calibration, performance, privacy, holdout, and manual-QA gate passes.
- `cargo fmt --check`, `cargo check`, `cargo test`, and `npm run build` pass.
- Manual Tauri interruption-recovery QA passes on macOS.
- `PRODUCT.md` describes the implemented P6 truth rather than the intended plan.

Do not claim that one extractor or one model prompt makes Smalltalk fully accurate. P6 is complete only when the full evidence-to-answer chain satisfies the release gate.
