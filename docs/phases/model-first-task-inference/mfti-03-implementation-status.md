# MFTI-03 implementation status

## Verdict

The MFTI-03 task-thread implementation is complete and its deterministic verification is green. The phase is **not live-complete** because the five private longitudinal sequences required by the phase document were not run. The earlier Screen Recording and signing-identity problem remains outside this implementation, and no live result is claimed from fixture data.

## Implemented contract

- Task threads are persisted separately from applications, windows, artifacts, workstreams, and return targets.
- Thread heads, immutable revisions, evidence-linked relationship edges, and one-to-three boundary hypotheses are stored in additive Task Truth v2 tables.
- Model continuity requires an exact thread id, revision, identity token, the exact prior-thread-revision evidence record, and current evidence.
- Session-local context is the default. An unscoped request cannot load global prior task memory. Cross-session reuse requires the exact continuity proof or a scoped human correction.
- Supporting research, verification, detours, interruptions, returns, new tasks, completion, and supersession have different deterministic lifecycle behavior.
- Completion requires current completion-state evidence or explicit human confirmation.
- Supersession writes an old-thread status revision and an evidence-linked old-to-new edge atomically with the replacement boundary.
- Current unresolved evidence defeats unsupported older certainty.
- Public answers are atomic to one thread revision, snapshot revision, selected hypothesis, model response, observation packet, and evidence watermark.
- Observed activity and strict return targets cannot rewrite semantic task fields.
- A target attaches only when its exact model-selected return anchor proves the locator, frame, thread id, and thread revision. Open time revalidates the current thread head and the complete atomic identity before existing freshness, privacy, suppression, and locator checks.
- Human hypothesis selection, relationship correction, completion, reactivation, and selected-task rejection create scoped durable revisions. Selected-task rejection demotes only that exact thread revision to unresolved and does not suppress an application, domain, or unrelated session.
- React and the native island consume the same semantic answer. Both expose bounded alternatives and scoped correction actions.

## Verification

- `cargo fmt --check`: passed.
- `cargo check`: passed.
- `cargo test task_truth_v2 --lib`: 100 passed, 0 failed, 2 opt-in live tests ignored.
- `cargo test continuation --lib`: 492 passed, 0 failed, 2 opt-in live tests ignored.
- `cargo test session_island --lib`: 40 passed, 0 failed.
- TypeScript compilation: passed.
- Continue presentation tests: 19 passed, 0 failed.
- Swift native-island type-check: passed; only the existing macOS 14 `onChange` deprecation warning remains.
- `git diff --check`: passed.

The opt-in synthetic provider transport smoke also passed against the configured real provider using only the public repository icon and synthetic structured evidence:

- provider/model: `openai` / `gpt-5.4-mini`
- private capture data sent: false
- diagnostic: `success`
- response schema: `smalltalk.task_truth_model_output.v2`
- latency: 5,897 ms
- usage: 2,596 input, 806 output, 3,402 total tokens

This smoke found and verified one final contract repair: when task-grade causal or user-authored evidence is unavailable, both the corresponding semantic fields and their claim-evidence entries are schema-constrained to `null`.

## Remaining live gate

The following private, newly captured sequences remain required and unclaimed:

1. Same task across editor, terminal, browser research, and return.
2. Brief unrelated detour followed by return.
3. A new task superseding the old foreground task.
4. Two plausible tasks with explicit user choice.
5. A current unresolved boundary with an older confident snapshot present.

Each sequence still needs human labels for surface, operation, primary task, subtask, relationship, progress, unfinished state, next action, and selected thread, plus a privacy-safe audit showing the winning revision and proving that no semantic field came from another revision.

No credentials, screenshots, raw model payloads, capture database, or personal history are included in this report.
