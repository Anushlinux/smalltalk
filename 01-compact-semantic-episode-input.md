# Prompt 01 — Build the Compact, Truthful Luna Evidence Boundary

## Codex implementation prompt

Implement this phase completely in the current `smalltalk` repository. This is the first of three ordered implementation sessions. Its job is to replace the large, duplicated Task Truth request with a small, temporally correct evidence boundary that `gpt-5.6-luna` can understand reliably.

Do not perform the public-answer cutover in this session. Do not redesign React or the native island. Do not remove the legacy production path yet. This phase proves the upstream input before later sessions depend on it.

## Product outcome

Smalltalk is a lean continuation layer. When the user explicitly presses Continue, it should understand the concrete work immediately around the interruption and eventually tell the user the exact useful point from which to resume.

This phase supplies the evidence for that answer. It does not send the full session, a large UI-element inventory, or locally invented activity prose to Luna. It selects one current causal boundary and, only when needed, one earlier related boundary.

The central rule is:

```text
full local evidence history
        -> deterministic boundary selection and privacy filtering
        -> small request-local evidence slots
        -> one Luna semantic interpretation
```

Local code may select, order, hash, validate, reject, and persist evidence. Local code may not decide the semantic task and then feed that decision to Luna as if it were evidence.

## Start by preserving and auditing the current checkout

The checkout is intentionally dirty and already contains substantial uncommitted PFTU/MFTI work. Treat every existing modification as user-owned.

Before editing:

1. Run `git status --short`.
2. Read the complete diffs for:
   - `src-tauri/src/continuation/task_truth_v2.rs`
   - `src-tauri/src/continuation/task_truth_v2/semantic_probe.rs`
   - `src-tauri/src/continuation/task_truth_v2/model.rs`
   - `src-tauri/src/continuation/task_truth_v2/verifier.rs`
   - `src-tauri/src/continuation/task_truth_v2/production.rs`
   - `src-tauri/src/continuation.rs`
   - `scripts/tauri-dev-pftu.sh`
   - `scripts/tauri-dev-mfti.sh`
3. Read completely:
   - `docs/phases/proof-first-task-understanding/pftu-01-live-semantic-kernel.md`
   - `docs/phases/proof-first-task-understanding/pftu-01-completion-audit.md`
   - `docs/phases/proof-first-task-understanding/pftu-02-truthful-production-answer.md`
4. Inspect the supplied `rsp1/` and `rsp2/` full logs locally. Do not commit, copy, or sanitize those generated logs into source control.
5. Map each requirement below to existing code. Keep code that already satisfies the requirement. Do not create a parallel inference service or a second semantic-probe implementation.

Never use `git reset`, `git checkout --`, destructive cleanup, or broad rewrites that erase unrelated changes. If current uncommitted code partly implements this prompt, finish and test it in place.

## Proven reproduction facts

Verify these numbers against the supplied logs before relying on them. Record any differences in the completion audit.

The first legacy request contains approximately:

- 53,301 provider input tokens;
- 140,454 characters in its main structured user input;
- 116,510 characters in `packet`;
- 88,666 characters for 96 canonical elements;
- 22,111 characters for 32 causal events;
- 19,016 characters in an evidence catalogue;
- a 36,757-character strict output schema;
- two high-detail screenshots;
- a 6,000-token maximum output allowance.

The packet is not merely large. It is structurally noisy:

- only 27 of 96 canonical elements are task-eligible;
- all 96 have no `visual_description`;
- none is focused;
- none contains causal evidence references;
- the 96 elements contain only 47 unique text-reference hashes;
- the evidence catalogue repeats references for every canonical element and causal event;
- one element repeats the same `ax_ocr_text_disagreement` marker 167 times;
- 11 of 32 events occur after the timestamp of the claimed current screenshot, reaching roughly 5.4 seconds into the future and including an app switch.

The second request sends the same packet again and adds about 20,387 characters of reconciliation material. That second-request behavior is eliminated in Prompt 03. This phase must first make the single intended request correct.

The existing compact PFTU path is the correct starting point. Its current limits are approximately:

- no more than two boundaries;
- no more than four images;
- no more than 24 KiB of structured text;
- no more than 6,144 estimated text tokens;
- short request-local support slots;
- four semantic fields;
- Luna as the configured model.

Do not relax those limits. Reduce actual inputs further where evidence permits.

## Clarification: what “semantic episode” means here

Do not send stored local episode labels such as `reviewing output`, `unknown`, `draft_active`, `composing`, or similar classifier output to Luna as task meaning.

Existing `continue_semantic_moments` and `continue_episodes` may be used as time indexes or boundary candidates only if their timestamps and evidence membership are reliable. Their prose is not semantic authority.

For this phase, a semantic episode is a request-time evidence boundary:

- a small before/after sequence;
- centered on one user-grounded change;
- chronologically closed at the selected current frame;
- containing only evidence useful for task interpretation;
- free of local conclusions about what the user’s task means.

Do not create another episode database merely to satisfy the word “episode.” Add storage only if the current packet, frame, transition, and event records cannot represent the required deterministic boundary, and prove that limitation first.

## Required implementation

### 1. Define one authoritative cutoff for the Continue boundary

Every request must have an explicit final evidence time derived from the newly captured current frame used for that Continue attempt.

Enforce all of the following:

- No event with `observed_at_ms` later than the final frame may enter any request slot.
- No canonical observation with `changed_at_ms` later than the final frame may enter the request.
- No delta whose effective next frame is later than the final frame may enter the request.
- A target-frame reference must not pull a future event backward into an earlier boundary.
- The request audit must persist the selected final frame id, cutoff time, earliest included time, and counts of excluded late records by kind.
- If the current frame is missing, stale, private, or not model-eligible, return a typed request-not-built result. Do not silently choose an older frame and call it current.

Add a deterministic regression fixture reproducing the supplied packet’s 11 late events. The built compact request must exclude all 11.

### 2. Replace frame-count grouping with causal boundary selection

The current `boundary_assignments(frame_count)` grouping is not a semantic episode selector. Replace or subordinate it to deterministic boundary selection based on observed causality.

Prefer boundaries centered on evidence such as:

- committed typing or submit/send;
- command execution and resulting output;
- material navigation or page/document change;
- a meaningful click with an observable result;
- a real app switch followed by continued activity on the new surface;
- the explicit Continue capture boundary.

Do not treat these as meaningful user actions by themselves:

- passive Accessibility notifications;
- repeated focus/window metadata;
- capture bookkeeping;
- duplicate scroll samples with no material visual change;
- classifier labels;
- an app switch that occurs after the final frame;
- browser chrome or toolbar observations unless they establish a necessary surface identity.

Choose the current boundary first. Add one earlier boundary only when it is required to explain progress, interruption, or continuity. The earlier boundary must be causally or identity-related to the current work; “it was one of the last frames” is insufficient.

Persist neutral selection reasons such as `current_manual_boundary`, `committed_action_with_result`, or `same_surface_material_change`. Do not persist semantic task labels as selection reasons.

### 3. Deduplicate before serialization

Deduplication must occur before request construction, not merely through token truncation after the dump is built.

At minimum:

- collapse identical canonical records by stable content identity within a boundary;
- collapse repeated text-reference hashes when they do not represent distinct owned regions;
- deduplicate repeated conflict and rejection reason strings;
- collapse repeated scroll/notification events that have no distinct observable effect;
- avoid serializing the same record once in the packet and again in a full evidence catalogue;
- preserve distinct before/after facts when the same text moved, changed ownership, or has a materially different state;
- keep enough local mapping to validate every request-local slot back to its exact source record.

The completion audit must report raw candidate counts, admitted counts, deduplication counts, late-record exclusions, and final serialized bytes.

### 4. Send only short, semantically neutral support slots

Reuse and harden the existing support-slot design. A slot may contain:

- request-local slot id;
- category;
- boundary number;
- observed time;
- short neutral summary;
- the corresponding image when applicable.

The local slot map retains real ids, hashes, frame ids, privacy state, ownership state, and fingerprints. Do not expose long internal identifiers to Luna unless the provider requires them for a non-semantic transport reason.

Allowed semantic-support categories are:

- before/after image;
- user-grounded action;
- material semantic-neutral delta;
- owned observation with useful visible content.

Surface identity may help order and distinguish boundaries, but it must not by itself support a primary-task claim.

### 5. Keep the Luna contract deliberately small

Use `gpt-5.6-luna`. Do not add Sol comparison, automatic fallback, model arbitration, or a stronger-model path.

For this phase, Luna returns only:

- `primary_task`;
- `current_step`;
- `last_progress`;
- `unfinished_state`;
- `support_slots_by_field`;
- `missing_evidence`;
- `confidence_by_field`;
- `status` as `resolved`, `partly_resolved`, or `unresolved`.

Do not ask Luna for multiple hypotheses, opaque hashes, task-thread ids, return locators, public copy, application identity, forensic claim objects, or a next action in this phase.

The prompt must make these distinctions explicit:

- Visible page content is evidence, not automatically the user’s purpose.
- Passive navigation or scrolling without an explicit objective cannot establish `primary_task`.
- `reviewing`, `browsing`, `editing`, `typing`, and similar activity labels are not acceptable primary tasks without a concrete evidenced purpose.
- A concrete current step may still be recoverable when the primary task is not.
- Null is correct when evidence is insufficient.

### 6. Make admission mechanical and field-local

Local validation may:

- reject foreign, stale, future, private, or ineligible slots;
- reject slot categories that cannot support a field;
- null one unsupported field while retaining other supported fields;
- enforce bounded strings and confidence ranges;
- reject forbidden generic primary-task labels;
- persist precise validation issues.

Local validation must not:

- rewrite Luna’s semantic text;
- infer a task from app/window names;
- fill a missing field from legacy Task Truth, activity recap, work truth, current focus, candidate labels, or stored local episode prose;
- run a second semantic interpretation.

A partially admitted response may be retained for evaluation, but must not be called fully successful merely because one field survived. Preserve `support_slot_validation_failure` separately from semantic correctness.

### 7. Keep this phase non-public

The compact path remains feature-gated and non-authoritative throughout this session.

Do not:

- make it the default production Continue path;
- route it as the only React or island answer;
- remove the legacy model path;
- change public release gates;
- attach an open target;
- create compatibility fallback wording.

Prompts 02 and 03 own those changes after this phase passes.

## Automated verification

Add or update focused Rust tests covering at least:

1. Late events, observations, and deltas are excluded by the final-frame cutoff.
2. A future `target_frame_id` cannot pull an event into the request.
3. Passive Accessibility notifications do not become user-action slots.
4. Duplicate elements, hashes, conflicts, and no-op scrolls are collapsed.
5. Distinct material before/after changes are preserved.
6. Boundary selection is causal and deterministic, not based only on frame count.
7. An unrelated recent frame is not selected as prior context.
8. Privacy-blocked and stale current frames produce typed failures.
9. Structured byte, estimated-token, image, boundary, and per-category caps fail closed.
10. Slot round-trip validation checks exact source fingerprint and chronology.
11. Unsupported fields are nulled without semantic replacement.
12. Passive scrolling with no visible objective admits no primary task.
13. The supplied-log-style packet excludes all late events and remains far below the legacy size.

Run at minimum:

```bash
cd src-tauri
cargo fmt -- --check
cargo check
cargo test task_truth_v2
cargo test semantic_probe
```

Use the actual test filters supported by the crate. If a suggested filter matches no tests, run the relevant test module or the full Rust suite and report the adjustment.

## Fresh Luna proof

Do not declare semantic success from fixtures alone. Run twelve fresh, privacy-approved manual or authorized replay cases with expected meaning written before viewing Luna’s output.

Include at least:

1. Coding a named behavior.
2. Running a command to verify it.
3. Reviewing an agent result about that work.
4. Browser research supporting that same work.
5. An API dashboard supporting that work.
6. Passive scrolling with no explicit objective.
7. An app switch after the selected final frame, proving it is excluded.
8. Duplicate Accessibility/OCR evidence.
9. Completed work with no unfinished state.
10. Waiting for command or agent output.
11. A true interruption or detour.
12. One previously unseen application.

For each case record only privacy-safe measurements and review fields:

- case id;
- expected recoverable fields;
- selected boundary reasons;
- frame and cutoff identities;
- included and excluded evidence counts;
- structured bytes and estimated text tokens;
- provider input/output/total tokens;
- request and response ids;
- admitted fields and cited slots;
- validation issues;
- human rating per field: `Correct`, `Partly right`, `Wrong`, or `Should be unresolved`;
- one-line correction.

Do not commit screenshots, raw OCR/Accessibility text, API keys, SQLite databases, paths, URLs, or raw provider payloads.

## Required pass gate

Every item must pass:

- All twelve cases use Luna.
- At least ten complete a real provider round trip and parse.
- Zero cases contain a confidently wrong primary task.
- Passive pages without an evidenced objective do not receive an invented primary task.
- All included evidence is at or before the final-frame cutoff.
- Every non-null field cites at least one valid request-local support slot.
- No local semantic fallback fills a rejected or null field.
- At least 90% of recoverable fields are `Correct` or `Partly right`.
- At least 80% of recoverable primary tasks are `Correct`.
- Four held-back cases satisfy the same zero-confident-wrong-primary-task rule.
- Structured text never exceeds 24 KiB or 6,144 estimated tokens.
- Actual provider input is materially below the supplied 53,301-token legacy request; report median, maximum, and each outlier.
- The selected request never includes the same full record inventory plus a duplicate evidence catalogue.

If the gate fails, keep the phase incomplete. Diagnose whether the failure comes from capture, cutoff, boundary selection, deduplication, prompt, Luna capability, or admission. Do not hide it by moving to public authority.

## Completion artifact

Create or update:

```text
docs/phases/lean-continue/01-compact-semantic-episode-input-completion-audit.md
```

The audit must contain:

- dirty-checkout preservation notes;
- before/after request composition and token measurements;
- the final boundary-selection rules;
- late-evidence and deduplication proof;
- automated command results;
- all twelve redacted human-review rows;
- every failed or skipped gate;
- a file-level change summary;
- the exact final verdict.

Use exactly one of these final lines:

```text
PASS — 02-continuation-useful-output-contract may begin
```

or:

```text
INCOMPLETE — 02-continuation-useful-output-contract is blocked
```

Do not write `PASS` when live Luna review, held-back cases, or cutoff proof is missing.

