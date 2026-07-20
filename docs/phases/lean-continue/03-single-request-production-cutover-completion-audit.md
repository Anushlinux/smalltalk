# Single-Request Production Cutover Completion Audit

## Verdict summary

The normal explicit Continue path now uses the compact `gpt-5.6-luna` boundary without an armed case or special launcher. The implementation cutover is complete enough for the user-owned live test, but the phase release gate is not proven. Prompt 01 still lacks the required reviewed live corpus, the Prompt 02 completion audit does not exist, unchanged-evidence zero-request reuse is not implemented for separate manual decision ids, and the twelve required live cutover rows remain unrun.

## Dependency truth

- Prompt 01 ends incomplete because the twelve fresh Luna cases and four held-back cases have not been human-rated.
- `docs/phases/lean-continue/02-continuation-useful-output-contract-completion-audit.md` is absent.
- The user explicitly accepted rollback risk and requested early normal-app integration so the behavior can be tested through `npm run tauri dev`.

This audit therefore distinguishes implemented routing from proven release readiness.

## Provider-call graph

Before:

```text
manual Continue
  -> legacy micro inference when eligible
  -> legacy full ObservationPacketV2 Task Truth request
  -> optional conflict reconciliation request
  -> legacy/public fallback projection
```

After:

```text
manual Continue
  -> current-frame preflight and compact boundary
  -> zero posts when preflight/privacy/credentials fail
  -> otherwise one compact gpt-5.6-luna POST with zero HTTP retries
  -> deterministic semantic-field and per-visit-role admission
  -> exact decision-bound public answer or typed unresolved
```

Startup and background requests have both legacy model switches forced off. React also sends those switches as false. They may still construct local evidence and presentation data, but they do not execute the old micro-inference or model-assisted recap calls.

## Unreachable fallback proof

- `record_manual_continue_shadow_with_lookback` calls `run_manual_probe` unconditionally for manual Continue.
- The same function no longer calls `run_multimodal_shadow`; therefore `build_multimodal_request`, second-pass reconciliation, and stronger legacy selection are not reachable from that production edge.
- `production_decision_for_attempt` uses the compact result for a decision-bound manual attempt. It does not use `visible_legacy_model_answer` or a legacy Task Truth snapshot when the compact result is missing or rejected.
- A missing or rejected compact answer produces `typed_unresolved_model_first_answer`.
- Local activity recap, work truth, current focus, and candidate prose remain diagnostics and cannot fill the decision-bound semantic fields.

Legacy builders and decoders remain in source for historical tests and offline diagnostics. They have no call edge from normal manual Continue.

## Transport accounting

- Manual compact transport uses `MANUAL_PROVIDER_RETRIES = 0`.
- Each persisted compact run stores `provider_post_count` separately from request construction and `parsed_response`.
- Preflight failure, privacy rejection, unsupported model, request-build failure, and missing credentials record zero posts.
- Once transport is invoked, success, timeout, rejection, parse failure, and validation-limited output record one post.
- An exact decision id is idempotent: if its compact run already exists, it cannot submit again.
- Reconciliation count is structurally zero on the manual compact path.

## Armed-case removal

Normal app execution creates an internal `production_runtime` case from the exact decision id. It does not read `/tmp/*.json`, require `SMALLTALK_PFTU_CASE_ID`, or ask the launcher to verify a database. Internal production cases are excluded from the private human-review export so they cannot pollute the Prompt 01 evaluation corpus.

The special `pftu_01_probe arm` and `scripts/tauri-dev-pftu.sh` flow remains only for controlled evaluation cases.

## Automated verification

Passed on July 14, 2026:

- `cargo fmt --all -- --check`
- `cargo check`
- `cargo test task_truth_v2::observation_packet --lib`: 27 passed
- `cargo test task_truth_v2::semantic_probe --lib`: 47 passed, 1 ignored live-provider test
- `cargo test task_truth_v2::production --lib`: 23 passed
- `cargo test`: 783 passed, 3 ignored live-provider tests
- frontend TypeScript/Vite build through the bundled Node runtime
- `test:continue-presentation`: 24 passed
- Swift native-island type-check passed with one pre-existing macOS 14 deprecation warning
- `git diff --check`

No live provider call, Computer Use test, React click test, or native-island visual test was performed by Codex. The user owns the live app test.

## Remaining gates

- Twelve fresh production-like Luna scenarios are pending.
- React/island parity is covered deterministically but not live-tested in this build.
- One real normal-app boundary must prove `provider_post_count = 1`, one compact OpenAI log, and no legacy log.
- The Google to OpenAI docs to Codex to OpenAI docs reproduction must prove that Codex is `primary_work`, the final docs remain factual `Current`, and their role is provider-classified rather than inferred from recency.
- Provider and validation failure scenarios need live proof that no fallback appears.
- The raised 6,000-token compact output ceiling needs one live proof that the expanded semantic fields plus per-visit roles finish as valid JSON instead of ending with `provider_response_incomplete`.
- Exact target usefulness and open safety need live proof.
- Unchanged evidence across a newly requested manual decision does not yet reuse the prior semantic response at zero posts. Exact same decision id replay is protected, but the broader atomic-evidence cache gate remains unfinished.
- The Prompt 02 atomic answer dependency has not been audited to its required pass gate.
- The `rsp1`/`rsp2` workflow reconstruction remains pending.

## File-level changes

- `src-tauri/src/continuation/task_truth_v2/semantic_probe.rs`: normal production records, exact-decision idempotence, zero retries, actual post accounting, and public compact authority.
- `src-tauri/src/continuation/task_truth_v2.rs`: compact-only manual routing and no legacy reconciliation call edge.
- `src-tauri/src/continuation/task_truth_v2/production.rs`: exact compact projection and typed-unresolved behavior without legacy semantic fallback.
- `src-tauri/src/continuation.rs`: legacy cloud semantic switches forced off.
- `src/App.tsx`: normal Continue requests the compact-only backend behavior.
- `scripts/tauri-dev-mfti.sh`: production-like development no longer claims that PFTU is a separate disabled path.

INCOMPLETE — lean Luna Continue cutover is not proven
