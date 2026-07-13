# MFTI-02 — Make Cloud Multimodal Inference Understand The Activity

## Codex task

Turn the repaired observation stream from MFTI-01 into a working cloud multimodal inference path that understands what is visible, what the user did, what changed, and the likely meaning of the activity across screens.

This is an implementation and live-provider session. Do not stop after writing a prompt, mocking a response, or passing deterministic model fixtures. A successful privacy-approved live provider call over a newly captured multi-screen sequence is a hard dependency for completion.

The configured cloud multimodal provider is the sole semantic authority in this program. Local code may build requests, enforce privacy, validate evidence references, persist results, and reject unsupported claims. Local heuristics must not decide the task and must not replace a provider failure with labels such as `Browsing X`, `Editing Home / X`, `Reviewing output`, or `Searching`.

## Dependency gate

MFTI-01 must be complete in the current worktree, including its live model-input audit. Re-open the MFTI-01 completion evidence and independently verify:

- current foreground image availability;
- current-frame element/event capacity;
- source ownership;
- event-to-element grounding;
- before/after semantic deltas;
- explicit session scope.

If those properties are not proven, repair them before continuing. A model cannot infer truth from a temporally wrong or visually empty request.

Read before editing:

```text
AGENTS.md
docs/phases/model-first-task-inference/mfti-01-model-ready-observation-stream.md
docs/phases/task-truth-v2/tt2-04-multimodal-resolver-and-evidence-verifier.md
docs/phases/task-truth-v2/tt2-05-completion-audit.md
src-tauri/src/continuation/task_truth_v2/model.rs
src-tauri/src/continuation/task_truth_v2/verifier.rs
src-tauri/src/continuation/task_truth_v2/task_snapshot.rs
src-tauri/src/continuation/task_truth_v2.rs
src-tauri/src/continuation/activity_recap_model.rs
```

Reuse and repair the existing `TaskTruthResolver`, model client, schemas, and verifier where sound. Do not build a new parallel resolver.

## Current failures that must be removed

Verify the current state before editing:

- `SMALLTALK_TASK_TRUTH_MULTIMODAL_ENABLED` defaults the resolver to `multimodal_shadow_disabled`.
- Retained Continue outputs show `source = local_scorer`, `model = null`, `response_id = null`, and no selected current task turn.
- Existing completion evidence contains deterministic fixtures but no successful live provider smoke test.
- The activity recap model mainly phrases an already selected local fact pack. It cannot recover task meaning that local attribution lost.
- Provider/model failure can eventually lead the product back to a fluent local surface description, which looks like understanding even though no semantic inference happened.
- Model output is shadow-only and cannot affect the visible product.

This session fixes inference itself. MFTI-04 performs the final production authority cutover after task memory and answer composition are repaired.

## Required semantic separation

The model must reason about and return separate fields for:

```text
observed_surface
immediate_user_operation
semantic_effect_of_operation
current_subtask
likely_primary_task
task_object
relationship_to_prior_task
last_meaningful_progress
unfinished_state
possible_next_action
alternative_hypotheses
contradictions
confidence and evidence for each field
```

`observed_surface` is factual page/app content. It is not automatically the task.

`relationship_to_prior_task` must support at least:

```text
continuation
supporting_research
verification
temporary_detour
interruption
new_task
return_to_prior_task
unrelated_or_unknown
```

The model may say the relationship is ambiguous. It must not force an X page, browser tab, dialog, toolbar, or current window title to become the primary task.

## Goal 1 — Make provider configuration explicit and testable

Create one clear runtime configuration path for the cloud Task Truth provider.

Requirements:

- Use the existing secure provider/API-key configuration where possible.
- Never log, persist, export, or return API keys.
- Expose a diagnostic status that distinguishes disabled, credentials missing, model unavailable, privacy blocked, request invalid, timeout, provider error, invalid response, verification rejected, and success.
- Record provider name, model name, request id, response id, latency, image count, byte/token estimate, and estimated cost without storing private image bytes in durable audit.
- Manual Continue is allowed to make the call. Background image upload remains disabled unless the product is explicitly redesigned later.
- The developer UI must make it obvious whether a visible result came from a real cloud inference, a deterministic test fixture, a cache, or no inference.

Do not make an environment flag silently disable the only semantic engine while the UI continues producing a normal-looking answer.

## Goal 2 — Build a temporal multimodal request, not a text recap request

The request must provide the model with:

1. Chronologically ordered keyframe images.
2. The current screenshot and relevant earlier screenshots.
3. Window/app/document/page identity with ownership confidence.
4. Accessibility and OCR elements with regions and conflicts.
5. Grounded click, scroll, focus, typing-commit, submit, navigation, and app-switch records.
6. Before/after semantic deltas.
7. A prior task hypothesis only when it belongs to the same session/thread and is clearly labeled as a hypothesis, not truth.
8. Privacy and missing-evidence notes.

The model instruction must explicitly require it to reconstruct the sequence:

```text
what each selected screen is about
→ what object/control the user interacted with
→ what changed after the interaction
→ what immediate activity that supports
→ whether the activity continues, supports, interrupts, or replaces the earlier task
```

Do not send a locally chosen answer and ask the model to justify or rephrase it. Do not include legacy P6 output as ground truth. It may be included only in a separately labeled evaluation comparison after inference is complete.

## Goal 3 — Require bounded competing hypotheses

Use a strict versioned response schema. Require one to three task hypotheses when evidence is ambiguous.

Every hypothesis must include:

```text
hypothesis id
primary task summary
task object
current subtask
relationship to prior task
execution state
last meaningful progress
unfinished state
next action or null
supporting evidence references by field
contradicting evidence references by field
field-level confidence
overall confidence
```

Required resolver statuses:

```text
resolved
ambiguous
insufficient_evidence
privacy_blocked
model_unavailable
provider_failure
invalid_response
verification_rejected
```

The output must never represent `model_unavailable` as a resolved task.

## Goal 4 — Make interaction causality stronger than surface vocabulary

The model instruction and verifier must follow these evidence priorities:

1. User action grounded to an element plus the resulting state change.
2. Repeated interaction with a coherent work object across frames.
3. Focus, committed typing, submit, navigation, or tool-output boundaries.
4. Foreground page/app content.
5. Passive visible text.
6. Browser/app chrome and historical/background text.

Examples:

- A toolbar search label is not evidence that the user searched unless the user interacted with it or the URL/state transition confirms a search.
- A visible X post proves the user viewed that post. It does not prove whether the post was research, distraction, or a new task.
- Editing Task Truth code, opening AI-tool posts, and returning to the code may support a research relationship, but the model must preserve an unrelated-browsing alternative if the connection is weak.
- Agent output visible on screen is not user-authored work. The model must use roles and causal events to distinguish user input, app output, and third-party content.

## Goal 5 — Validate claims without performing local semantic inference

Retain a deterministic local verifier, but constrain its authority.

The verifier may:

- confirm cited frame/element/event/delta ids exist;
- confirm referenced sources are privacy-eligible and correctly owned;
- reject claims supported only by browser chrome, background windows, stale snapshots, or missing evidence;
- reject temporal contradictions;
- downgrade confidence when sources conflict;
- remove unsupported next actions or exact identities;
- prevent an unsafe open target.

The verifier may not:

- create a replacement task summary;
- choose a local workstream as the task;
- convert the window title into a task;
- turn `model_unavailable` into `Browsing`, `Editing`, `Reviewing`, or another semantic answer;
- use agreement with P6/local scorer as validation.

If verification removes the central task claim, the final status is `verification_rejected` or `insufficient_evidence`, not a locally reconstructed task.

## Goal 6 — Separate inference result from wording

Persist the verified structured inference before generating user-facing language.

Required provenance:

```text
semantic_source = cloud_multimodal_model | human_correction | unresolved
provider/model/request/response identity
observation packet identity
selected hypothesis id
field support and confidence
wording source
```

Wording may be deterministic or model-assisted, but it may not change task identity, task relationship, evidence, alternatives, or confidence.

## Goal 7 — Implement an honest no-inference result

When the provider cannot produce a verified result, return a typed result equivalent to:

> I captured your recent activity, but task inference is unavailable right now.

or:

> I could see the current screen, but there was not enough evidence to determine how it related to your earlier work.

Do not emit a polished local surface label on the primary card. The observed surface may be shown under evidence details, clearly labeled as observation rather than task understanding.

## Required deterministic tests

Add or update tests for:

1. Strict response parsing and evidence-id validation.
2. Multiple hypotheses with field-level evidence and contradictions.
3. Visible browser page is not automatically the primary task.
4. Browser chrome cannot establish the task.
5. Cross-app supporting-research inference from a grounded sequence.
6. Plausible casual-browsing alternative remains when relationship evidence is weak.
7. User-authored content is separated from agent/app/third-party output.
8. Provider disabled or unavailable returns typed unresolved status.
9. Provider failure cannot fall back to a local semantic task label.
10. Verifier rejects invented task objects and next actions.
11. Prior snapshot is treated as a hypothesis and cannot override newer contradictory evidence.
12. Wording cannot change the verified hypothesis identity.

Fixtures are necessary but not sufficient.

## Required live provider verification

Configure the provider safely and perform real manual Continue calls over at least three newly captured private sequences:

### Sequence A — Cross-app supporting work

- Work on Smalltalk Task Truth code or documentation.
- Open a relevant browser page.
- Click or scroll meaningful content.
- Return to the original work or press Continue.

Expected: the provider separates the visible browser activity from the likely ongoing task and reports the relationship with calibrated confidence.

### Sequence B — Genuine unrelated detour

- Work on a clear primary task.
- Switch to unrelated content and interact with it long enough to make the relationship ambiguous or unrelated.
- Press Continue.

Expected: the provider does not automatically claim the unrelated content supports the prior task.

### Sequence C — New task supersedes old task

- Work on one clear task.
- Start a distinct task with meaningful interaction and progress.
- Press Continue.

Expected: the provider identifies a likely new primary task and retains the old task only as prior context.

For every sequence, inspect:

- exact selected images and structured evidence;
- provider/model identity;
- request and response ids;
- raw structured hypotheses under private audit;
- verifier changes;
- final verified semantic result;
- latency and cost;
- whether the human reviewer agrees with observed surface, immediate operation, primary task, relationship, and uncertainty.

Do not commit personal screenshots or raw private model payloads.

## Verification commands

Run:

```bash
cd src-tauri && cargo fmt --check
cd src-tauri && cargo check
cd src-tauri && cargo test task_truth_v2
cd src-tauri && cargo test continuation
npm run build
```

Also run the explicit live-provider smoke command or manual app flow defined by the implementation. Record only safe metadata and review outcomes.

## Definition of done

This session is complete only when:

- the provider is genuinely invoked for manual Continue;
- at least three new private live sequences have successful cloud multimodal responses;
- the model receives chronological pixels plus structured causal evidence;
- the structured output separates surface, operation, subtask, primary task, and relationship;
- competing hypotheses and uncertainty are preserved;
- local validation checks support without creating semantic meaning;
- provider/model failure cannot produce a local semantic fallback answer;
- live audits prove request/response identity, latency, cost, and reviewed semantic quality;
- deterministic tests and builds pass;
- no credentials or personal captures are committed.

Do not mark completion based only on fixture clients, mocked JSON, schema tests, or code paths hidden behind a disabled environment flag.

## Handoff to MFTI-03

Report:

- semantic response schema and resolver status contract;
- provider configuration and safe diagnostic behavior;
- verifier authority boundaries;
- exact live sequences and human judgments;
- latency/cost measurements;
- failure behavior;
- the API by which MFTI-03 can maintain and compare task hypotheses over time.
