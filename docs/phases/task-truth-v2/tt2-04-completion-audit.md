# Task Truth v2.04 Completion Audit

Date: 2026-07-11  
Scope: provider-neutral multimodal resolver, bounded image request, deterministic evidence verifier, conflict-only second pass, shadow evaluation, and deterministic wording

## Outcome

TT2-04 is implemented as a shadow-only path. It does not replace the existing Continue decision, open a target, apply feedback, or change the first screen. TT2-05 remains responsible for authority and release gates.

An explicit manual Continue can now construct a provider-neutral multimodal request from the privacy-approved `ObservationPacketV2`. The request contains the current active-window image plus at most three relevant current/prior/support keyframes, canonical elements, causal events, frame changes, identities, the previous valid snapshot, and missing-evidence notes. Active-window crops are accepted only when they are readable PNG, JPEG, or WebP files under 4 MiB each and 12 MiB total. Background and private frames are excluded. The packet carries an ephemeral local path only in memory; serde serialization omits it, so checkpoints and audits retain the existing image-handle hash instead.

The provider boundary is the `TaskTruthModelClient` trait. The Task Truth domain has no provider-specific types. The OpenAI adapter uses strict structured output and actual `input_image` data URLs. Fixture clients provide deterministic responses without network. Unavailable credentials, disabled configuration, timeouts, policy refusal, provider errors, and invalid structured output are typed failures and fall back to the existing local/unresolved shadow snapshot.

The model returns up to two strict hypotheses. Material fields have field-specific evidence references and confidence. The local verifier independently checks whether references exist, rejects controls and navigation as goals, requires temporal continuity for historical content, checks causal or cross-modal user authorship, rejects invented app/document/surface identities, validates lifecycle values, removes generic or unsupported next actions, keeps return anchors subordinate, limits contradictions to affected fields, and removes privacy-blocked support. It records accepted, downgraded, removed, ambiguous, unsupported, or contradicted verdicts per field.

A second pass runs only for a bounded conflict trigger: close hypotheses, near-threshold critical fields, AX/OCR/visual conflict, control-region claims, task/anchor disagreement, or contradictory temporal ordering. It receives the conflict, hypotheses, and verifier decisions. It never receives legacy P6 as truth. Audit records whether the pass ran, its latency, estimated cost, and whether it changed fields.

Each durable shadow audit stores total latency and the resolver/request audit. The returned audit summary also computes rolling p50 and p95 latency over the latest 200 shadow audits. Request image count, image bytes, structured bytes, estimated tokens, hashed image handles, privacy exclusions, skipped-image reasons, model/config identity, resolution status, and estimated request cost are recorded without persisting image bytes or local image paths.

Verified first-screen wording is deterministic and separate from task resolution. It preserves snapshot identity, nullability, lifecycle, evidence-backed next action, and target policy. The production UI still uses the existing authority.

## Adversarial proof

The deterministic suite covers:

- button/control text selected as a goal, including the Session-013 `Approve for me` failure;
- historical completed text without a continuity edge;
- invented file/thread identities;
- generic unsupported next actions;
- a return target belonging to another task;
- nonexistent evidence ids;
- close hypotheses and ambiguity;
- AX/visual disagreement;
- private current images;
- typed timeout and invalid JSON failures;
- deterministic wording identity preservation;
- correct tasks without direct open targets;
- unfamiliar applications with pixels, events, and thin AX.

The frozen evaluator now reports path C as implemented. Manual boundaries record deterministic multimodal fixture resolution before verification. Background boundaries explicitly record `not_requested_background`. Session-013 chooses the causally supported user submission and rejects the approval control without using geometry or the legacy answer as ground truth. The release gate remains false because independent labels, the broad corpus, holdout, calibration, performance, privacy review, and manual macOS QA are still incomplete.

## Provider smoke-test status

No live provider call was made during deterministic verification. A live smoke test is intentionally opt-in through `SMALLTALK_TASK_TRUTH_MULTIMODAL_ENABLED=true` and safe local credentials. Keys and private image contents are never printed.

## Verification

- `cargo test continuation::task_truth_v2`: 33 passed.
- `cargo test continuation::activity_recap_validation`: 15 passed.
- `cargo test continuation::accuracy_eval`: 6 passed.
- Full `cargo test`: 592 passed.
- `cargo check`: passed.
- `cargo fmt --all -- --check`: passed.
- Frontend TypeScript and Vite production build: passed.
- Frozen Task Truth v2 evaluator: five deterministic Path C implementations; release gate false for the documented evidence gaps.
- `git diff --check`: passed.

## Program status

Task Truth v2 is not complete. TT2-05 remains.
