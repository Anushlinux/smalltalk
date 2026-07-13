# Task Truth v2.03 Completion Audit

Date: 2026-07-11  
Scope: observation packets, task snapshots, semantic checkpoints, task-first shadow selection, and explicit-Continue audit

## Outcome

Task Truth v2.03 is implemented as a shadow-only path. It does not replace `ContinueDecisionResult` or change the first screen.

The second-layer rebuild now constructs a deterministic `smalltalk.observation_packet.v2` before workstream and candidate ranking. The packet references existing capture evidence, keeps current/prior/background/support partitions separate, chooses two to four semantic keyframes when enough frames exist, applies privacy filtering before model eligibility, preserves AX/OCR provenance and conflicts, carries causal events and frame changes, hashes local return-anchor facts, and records byte/token estimates.

`smalltalk.task_snapshot.v2` is a versioned projection over the existing P6 `CurrentTaskTurn`; it is not a competing task table. P6 remains the compatibility authority. Shared lifecycle enums are reused, claim-level evidence and per-field confidence remain inspectable, and return-anchor state is subordinate. Missing current task evidence creates an unresolved snapshot instead of copying stale precise truth forward.

Semantic checkpoints persist packets and snapshots at bounded rebuild/manual-Continue boundaries. Semantically unchanged checkpoints deduplicate. Revision lineage, supersession, uncertainty decay, retention limits, and final pre-switch records remain local and auditable. The selector uses only task evidence and explicitly excludes URL/path existence, openability, artifact richness, and legacy candidate score.

Every manual Continue attempts a bounded shadow audit containing packet summary, keyframe reasons, canonical conflicts, causal edges, snapshot hypotheses, task-only selection, legacy comparison, first divergence, latency, bytes, and token estimates. Audit data stores evidence handles and hashes rather than image bytes or raw return paths.

## Evaluator proof

The Task Truth v2.02 evaluator now executes path C for all five session-013 decision boundaries. Every case reports:

- path C status `implemented`;
- deterministic packet and snapshot checkpoints;
- the same source fingerprint as paths A and B;
- control/navigation excluded as task truth;
- multimodal resolution explicitly `not_implemented` for the future v2.04 provider/verifier.

The release gate remains false. The committed corpus has five pending live-redacted cases and no independently reviewed release denominator or locked holdout. TT2-03 does not claim that the task resolver is release-ready.

## Verification

- `cargo test continuation::task_truth_v2`: 17 passed.
- `cargo test continuation::accuracy_eval`: 6 passed.
- `cargo test continuation::task_turn`: 34 passed.
- Full `cargo test`: 576 passed.
- `cargo check`: passed.
- `cargo fmt --check`: passed.
- `npm run build`: passed.
- `git diff --check`: passed.
- The v2 evaluator generated `/tmp/task-truth-v2-03-report.json`; all five path C results were `implemented`, deterministic replay was true, and the release gate remained false for the documented corpus/holdout gaps.

## Remaining Task Truth v2 work

Task Truth v2.04 must add the bounded multimodal resolver and evidence verifier over these packet/snapshot contracts. It must keep provider/network code outside the domain model, validate claims locally, and continue to abstain when evidence is insufficient. Task Truth v2.05 must decide product authority only after the independently reviewed corpus, locked holdout, calibration, performance, privacy, and manual macOS gates pass.

Task Truth v2 as a whole is not complete.
