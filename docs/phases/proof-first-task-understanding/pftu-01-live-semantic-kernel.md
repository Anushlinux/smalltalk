# PFTU-01 — Prove a Small Semantic Kernel on Real Work

## Codex implementation prompt

Implement this phase completely. Do not stop after changing types, adding fixtures, or making tests pass. This phase is complete only when a deliberately small cloud inference contract produces useful, human-approved task meaning on fresh real workflows.

This is the first of four successive sessions. It solves the upstream part of root problem 1: Smalltalk must say what the person was actually doing, not merely describe the visible application activity.

## Core idea

The current model call asks for too many things at once. It asks one response to understand up to four screens, infer a task, infer continuity, create up to three hypotheses, fill sixteen semantic fields, copy exact opaque identities, and cite exact evidence keys under a large strict schema. Session 038 showed that this produces expensive requests and still returns invalid or rejected answers.

Do not improve that request by adding more fields or more prompt text. First prove that the model can recover four pieces of meaning:

1. The primary task.
2. The current step within that task.
3. The last meaningful progress.
4. What remains unfinished.

The response must be bound to a small, current evidence packet. Grounding must be easy enough that a model can do it reliably and strict enough that local code can reject unsupported claims. Local code must not invent replacement semantics.

## Why the previous approach failed

Read these files before changing code:

- `docs/phases/model-first-task-inference/mfti-02-cloud-multimodal-task-inference.md`
- `docs/phases/model-first-task-inference/mfti-02-implementation-status.md`
- `docs/phases/model-first-task-inference/mfti-03-task-thread-memory-and-answer.md`
- `docs/phases/model-first-task-inference/mfti-03-implementation-status.md`
- `docs/phases/model-first-task-inference/mfti-04-completion-audit.md`
- `src-tauri/src/continuation/task_truth_v2/model.rs`
- `src-tauri/src/continuation/task_truth_v2/verifier.rs`
- the current production path from the explicit Continue command through React and the native island
- all available `continue_outputs/session-038-*` artifacts

Confirm these facts against the current checkout rather than copying them blindly:

- MFTI-02 never produced an accepted live central task.
- MFTI-03 and MFTI-04 machinery was built even though their stated live dependencies had not passed.
- the current request is limited to four images but contains a large structured packet and a strict multi-hypothesis response;
- session 038 contains model failures and locally generated public copy even though Task Truth did not understand the task;
- deterministic tests proved contracts, not semantic usefulness.

The current checked-in session-038 exports also show `local_scorer` as the final copy source, `request_invalid` or an unconfigured provider in the Task Truth diagnostic, and repeated `no_eligible_candidates_after_feedback_gate` fallback. Treat those as concrete reproduction targets. Query the live SQLite rows as well because the export directory contains only part of the session.

Record any material difference in the completion audit.

## Research basis

This design follows three well-supported observations:

- Screen understanding and task understanding are different problems. ScreenAI demonstrates that identifying interface content is its own visual-language task: <https://research.google/pubs/screenai-a-vision-language-model-for-ui-and-infographics-understanding/>.
- Real computer work is long, cross-application, and partially observed. OSWorld demonstrates that realistic desktop tasks need reasoning across applications and state, not a single-screen label: <https://arxiv.org/abs/2404.07972>.
- Long-horizon agent research finds that replaying raw history creates redundancy, while bounded or hierarchical memory can preserve the useful parts. This phase does not assume a final memory design; it first proves a small semantic unit.

The OpenAI Responses API supports ordered multiple image inputs and structured output. Use the existing provider client. Recheck current official model guidance when executing this phase because model names and availability change.

## Hard dependency gate

There is no earlier PFTU dependency. However, do not start by editing production authority, React copy, island copy, cache adoption, task-thread tables, or release gates.

This phase is an upstream semantic proof. If the proof fails, keep production unchanged and report why.

## Required architecture

### Reuse rule

This phase is not permission to rewrite Task Truth v2. Reuse the existing provider transport, privacy checks, observation records, persistence identities, diagnostics, and verifier utilities when they already satisfy this prompt. Add the smallest feature-gated probe needed for the live experiment. Before replacing any existing component, demonstrate the concrete behavior that makes replacement necessary.

### 1. Add a semantic-probe path inside Task Truth v2

Extend the existing `task_truth_v2` module and provider client. Do not create another inference service, another capture system, or a competing public-answer engine.

The probe must run only after an explicit Continue action or an explicitly authorized replay command. It must never upload images in the background.

The probe is a development path until this phase passes. It must not become public semantic authority in this session.

### 2. Build a deliberately small input packet

For each probe, select one current causal boundary and at most one immediately relevant earlier boundary. A boundary is a small before/after unit around a user-grounded event such as:

- committed typing;
- submit or send;
- navigation;
- command execution and resulting output;
- a material page or document change;
- application switch followed by continued work on the same object;
- manual Continue.

Each boundary may contain:

- chronologically ordered before/after images;
- a small set of owned Accessibility or OCR observations;
- grounded operations;
- semantic-neutral deltas such as content appeared, disappeared, or changed;
- app, window, page, or document identity when locally observed;
- timestamps and explicit missing-evidence notes.

Do not include the whole session, the legacy candidate list, activity recap prose, local semantic labels, stale workstreams, stale feedback, or a dump of prior hypotheses.

Cap the textual request by measured bytes and estimated tokens. Log those measurements. A request that silently grows back toward the session-038 size fails this phase.

### 3. Replace opaque evidence-key burden with short request-local support slots

The current model must copy exact opaque evidence catalog keys, hashes, thread identities, and return anchors while also reasoning about the task. Remove that burden from the semantic probe.

Give each admitted evidence item a short request-local slot such as `B1_IMAGE_AFTER`, `B1_USER_ACTION`, or `B1_DELTA`. The mapping from a slot to the real evidence record, frame, hash, ownership, and privacy state stays local.

The model may cite only these short slots. Local validation expands them back to exact records and checks:

- the slot existed in this exact request;
- its underlying evidence hash is unchanged;
- ownership and privacy permit its use;
- chronology is valid;
- the cited evidence category is allowed for that claim.

Local validation may reject or null a claim. It may not write a more specific task, step, progress statement, or unfinished state.

### 4. Use a minimal response contract

Create a probe-specific structured response with no more than these semantic fields:

- `primary_task`;
- `current_step`;
- `last_progress`;
- `unfinished_state`;
- `support_slots_by_field`;
- `missing_evidence`;
- `confidence_by_field`;
- one overall status: `resolved`, `partly_resolved`, or `unresolved`.

Do not include task threads, continuity tokens, supersession, return targets, open locators, application identity hashes, public wording, next action, or multiple hypotheses in this first contract.

A field may be null. Null is preferable to a generic verb or invented detail.

The model must not use `editing`, `viewing`, `browsing`, `reviewing`, `reviewing_output`, `typing`, `filling_form`, or similar activity classes as the primary task unless the evidence truly shows that the activity itself is the task. These words may appear in `current_step` only when they add concrete meaning.

### 5. Separate semantic generation from deterministic admission

The cloud model supplies semantic meaning. Local code supplies:

- evidence selection;
- privacy filtering;
- chronological ordering;
- support-slot mapping;
- response parsing;
- field-level support checks;
- persistence of request and response identities;
- rejection and typed failure states.

Local code must not create semantic fallback text from application names, titles, local classifications, candidate scores, or `verb + object` templates.

### 6. Diagnose transport, parsing, grounding, and meaning separately

Persist privacy-safe diagnostics that distinguish:

- request not built;
- privacy blocked;
- provider unavailable;
- timeout;
- provider returned no usable output;
- structured parse failure;
- support-slot validation failure;
- semantic answer parsed but was human-rated wrong;
- success.

Do not treat a successful HTTP response or valid JSON as semantic success.

## Affordable-model proof

Run every frozen probe case through the configured image-capable production candidate, `gpt-5.6-luna`. The accuracy benchmark is the expected meaning written by a human before model output, not a comparison with a more expensive model.

Use the same input packet, response schema, and validation policy for every case. Record latency, tokens, estimated cost, parse result, admitted fields, and human judgment.

Luna must meet the semantic gate without help from local fallback wording. Do not lower the gate to preserve the affordable model. If it fails, keep this phase incomplete and report whether the dominant failure is evidence quality, request design, model capability, or validation design. A stronger model may be used later to diagnose a specific failure, but it is not part of the required proof corpus and must not run automatically.

## Real proof corpus

Create twelve fresh, privacy-approved probe cases. Write the expected meaning before seeing model output. At least four cases must be held back from prompt tuning.

The twelve cases must include:

1. Writing or changing code for a named product behavior.
2. Running a command to verify that code change.
3. Reviewing an agent response about that same work.
4. Browser research supporting the same work.
5. An API dashboard used to inspect or configure the same work.
6. A visible application whose task cannot be determined.
7. A completed action with no supported unfinished state.
8. Waiting for an agent or command output.
9. A form where the business purpose is visible.
10. A form where only the act of filling it is visible.
11. A session-038 reconstruction around Codex, VS Code, browser research, and an API dashboard.
12. One previously unseen application.

For each case, record:

- case id and timestamp;
- exact session and decision ids;
- expected primary task, current step, progress, and unfinished state;
- whether each field is actually recoverable from the admitted evidence;
- model and request/response ids;
- output status;
- field-level `Correct`, `Partly right`, `Wrong`, or `Should be unresolved`;
- one-line correction;
- cited support slots and admission result;
- latency and token measurements.

Do not commit screenshots, databases, raw provider payloads, full OCR text, paths, URLs, credentials, or personal content. Commit only redacted evaluation rows and aggregate results.

## Required pass gate

All of these must pass:

- All twelve cases produce either a parsed response or a precise typed provider failure.
- At least ten of twelve cases complete a real provider round trip and parse successfully.
- No case contains a confident wrong primary task.
- No unresolved case is filled with a local semantic label.
- At least 90% of fields marked recoverable are `Correct` or `Partly right`.
- At least 80% of recoverable primary-task fields are `Correct`, not merely `Partly right`.
- Every admitted non-null field cites at least one valid support slot.
- Unsupported fields are null or rejected; local code never repairs their meaning.
- The four held-back cases meet the same no-confident-wrong-task rule and at least 75% correct recoverable primary tasks.
- The chosen request is materially smaller than the existing session-038 request. Report exact bytes, tokens, images, and output size for both.
- A human reviewer can read at least ten answers and understand the concrete work without relying on an application name or generic activity verb.

If any gate fails, continue iterating only on evidence selection, the probe prompt/schema, model choice, or validator until the gate passes or a genuine external blocker is proven. Do not proceed to PFTU-02 on a partial result.

## Automated verification

Add focused tests for:

- boundary and evidence selection;
- request size limits;
- chronological image order;
- short support-slot mapping and round trip;
- foreign, missing, stale, or privacy-blocked slots;
- field-level nulling without local semantic replacement;
- forbidden generic primary-task labels;
- provider diagnostic distinctions;
- response parsing for resolved, partial, unresolved, refusal, empty output, and malformed output;
- proof-corpus result parsing and denominator integrity.

Then run:

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

Run any narrower new test commands explicitly and report them.

## Forbidden work

Do not:

- switch production semantic authority;
- redesign the first screen;
- build new task-thread persistence;
- add a release gate or large evaluation framework;
- revive the browser extension;
- upload in the background;
- store raw typed characters or clipboard text;
- send broad raw history to a provider;
- weaken existing open-safety, privacy, feedback, or stale-target protections;
- count fixtures, a synthetic icon request, or valid JSON as live semantic proof.

## Required completion audit

Create `pftu-01-completion-audit.md` beside this file. It must include:

- the confirmed root cause of the old request failures;
- exact code paths changed;
- old versus new request shape and measured size;
- all twelve redacted case results;
- tuning versus held-back denominators;
- model comparison;
- automated command results;
- failures encountered and what changed because of them;
- the exact evidence that every pass gate is satisfied;
- remaining risks.

End with exactly one verdict:

- `PASS — PFTU-02 may begin`, or
- `INCOMPLETE — PFTU-02 is blocked`.

Do not write `PASS` unless real provider calls and human semantic judgments satisfy every gate.
