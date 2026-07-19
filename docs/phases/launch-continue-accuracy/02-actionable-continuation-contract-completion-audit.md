# LCA-02 Actionable Continuation Contract Completion Audit

Date: 2026-07-19

## Verdict summary

The live compact provider and public answer contracts now represent one atomic continuation: the newest unfinished task, its lifecycle state, the exact resume point, one evidence-supported next action, relevant completed context, and a factual non-openable location summary. The prompt, strict response schema, local field admission, compatibility projection, stored-row behavior, and public mapping were changed together.

The implementation and deterministic proof are complete. The final four-case replay passes through the real task-relevant packet, strict compact admission, shared production mapper, public compatibility projection, and target-honesty checkpoints without provider transport. The broad continuation suite also passes with 610 tests successful, zero failures, and three explicitly ignored live/private tests. LCA-02 is proven and LCA-03 may begin.

## Old versus new provider and public contracts

Old compact provider response:

```text
primary_task
current_step
last_progress
unfinished_state
visit_roles
```

New compact provider response:

```text
smalltalk.pftu_01.semantic_probe_response.v3

unfinished_task
task_state
resume_point
next_supported_action
completed_context
where_summary
visit_roles
support_slots_by_field
missing_evidence
missing_evidence_by_field
confidence_by_field
verifier_result_by_field
status
```

The compact request is `smalltalk.pftu_01.semantic_probe_request.v7`. Its system instruction now asks which task is newest and unfinished, what is already complete, what exact state was left behind, what one next action is directly supported, and where that state was observed without inventing a locator. It explicitly distinguishes workstreams from tasks, implementation from verification, drafts from submissions, supporting panes from primary work, and hypotheses from facts.

The public answer is now `smalltalk.task_truth_public_answer.v5`. It adds the same canonical actionable fields plus `model_resolution_status`, which preserves the provider's raw `resolved`, `partly_resolved`, `unresolved`, or `refused` status. Local validation may downgrade the public compatibility status, but it does not upgrade or rewrite the raw model status.

## Field semantics, size limits, and support rules

| Field | Meaning | Maximum characters | Admission rule |
| --- | --- | ---: | --- |
| `unfinished_task` | Newest concrete incomplete objective | 220 | Must cite request-local task evidence; broad activities and passive screen guesses are rejected. A transported image has task authority only when P6 explicitly tags it `LatestUserGoal` or `CurrentUnsentDraft`. |
| `task_state` | `active`, `waiting_for_result`, `needs_user_verification`, `blocked`, `superseded`, `completed`, or `unclear` | enum | Every value except `unclear` must be evidence-cited |
| `resume_point` | Exact meaningful state where work stopped | 260 | Must cite request-local evidence and pass chronology, privacy, ownership, and fingerprint checks |
| `next_supported_action` | One concrete evidence-supported action | 180 | Generic, destructive, submission-inventing, locator-like, and state-contradicting actions are rejected |
| `completed_context` | Immediately relevant work already complete | 180 | Must remain context and cannot override the unfinished task |
| `where_summary` | Factual descriptive location | 220 | Raw paths, raw or query-bearing URLs, and invented locators are rejected; it creates no target authority |

Every field retains request-local support slots, a confidence value, field-local missing evidence, and a local verifier result of `admitted`, `rejected`, or `not_proposed`. Overlong values are nulled rather than truncated or semantically rewritten. One rejected field does not erase independently admitted fields.

## Compatibility mapping

Existing consumers remain functional through a one-way derivation from the new atomic meanings:

| Canonical LCA-02 field | Compatibility field |
| --- | --- |
| `unfinished_task` | `task_summary` |
| `task_state` | `execution_state` |
| `resume_point` | `current_subtask` and `unfinished_state` |
| `next_supported_action` | `next_action` |
| `completed_context` | `last_meaningful_progress` |

Compatibility fields cannot feed values back into the canonical fields. The compact mapper does not read local activity recap prose, stale task-thread actions, app labels, workstream labels, or legacy semantic fields. Inferred compact wording uses `task_basis = model_inferred`; it is not promoted to `explicit_goal`.

Stored pre-LCA compact rows fail the new strict parse, are detected as legacy rows, and project an explicitly unresolved answer with `legacy_compact_contract_downgraded`. Their old semantic wording is not exposed through the new public fields or compatibility fields.

## Atomic identity mapping

Production and deterministic replay use the same `map_compact_probe_output_to_public_answer` implementation. Its identity seed includes the exact decision, packet, request/provider-response identity, verifier version, and admitted compact output. The resulting public answer carries the session, task-thread, snapshot, selected-hypothesis, model request/response, observation-packet, evidence-watermark, and correction identities.

The compact mapper always starts with `direct_return_target = null`. `where_summary` never creates openability. The existing strict target path may attach a direct target only when the independently validated target has the same task-thread id and revision and passes the existing target policy and feedback checks.

## Four critical fixture inputs and semantic results

The fixtures contain synthetic, privacy-safe provider-shaped output. They do not contain the supplied private logs, screenshots, raw URLs, paths, typed characters, or clipboard text.

| Case | Unfinished task | State | Resume point | Next supported action | Completed context |
| --- | --- | --- | --- | --- | --- |
| `05cd` | Assess whether the proposed screen-aware context-reconstruction product solves a real need by reviewing the conversational answer. | `active` | The answer has begun, affirms the need, and continues beyond the visible section. | Continue reviewing from the “Does your product solve a real need?” section. | null |
| `0d1c` | Add and inspect the answer-linked visual cue in the real island output. | `waiting_for_result` | The agent is implementing the cue while preserving the island output contract. | Return to the Codex visual-cue task and inspect its implementation result. | Backend connection and output flow were already implemented and verified. |
| `0056` | Verify the completed answer-linked visual cue in Smalltalk. | `needs_user_verification` | Implementation completed, focused checks passed, and user verification remains. | Open the latest Continue answer and verify that Show more reveals the linked cue. | Visual-cue implementation is complete and focused checks passed. |
| `0e34` | Investigate why the latest Continue result was rejected as insufficient evidence. | `active` | An unsent regression report is being drafted after the failed result. | Return to the draft and continue the regression report. | null |

The expected `compact_semantic_output` and `product_answer` checkpoints assert all six canonical fields, raw status, compatibility equality, null direct target, and target-honesty behavior.

## Generic and invented-action rejection proof

The compact admission path rejects generic instructions such as `Continue working`, `Keep browsing`, and `Review it`. It also rejects destructive actions, invented submissions, raw paths or URLs, and actions that contradict `completed`, `superseded`, `unclear`, `waiting_for_result`, or `needs_user_verification` state.

Focused proof:

- the LCA filter passed `lca_02_generic_destructive_and_locator_actions_are_field_local_rejections`;
- `cargo test verifier --lib` passed 20 tests, including generic-action, destructive/locator-action, invented identity, and field-local rejection cases;
- production tests passed the atomic projection and stored-legacy-row downgrade cases.

## One-request and no-legacy-fill proof

- `MANUAL_PROVIDER_RETRIES` remains `0`.
- The compact request audit ceiling remains one provider post, zero reconciliation posts, and zero HTTP retries.
- The deterministic four-case path does not execute transport.
- Exact decision idempotence prevents a second post for the same decision.
- Production maps only the admitted compact object. No legacy recap, task-thread action, local workstream, app name, title, or target candidate fills missing compact fields.
- Provider schema refusal, parse failure, privacy failure, and missing credentials remain typed unresolved outcomes rather than fallback answer-engine triggers.

## Automated verification

| Command or equivalent | Exact result |
| --- | --- |
| `cargo fmt --all -- --check` | Passed. |
| `cargo check` | Passed in 27.42 seconds. |
| `cargo test production --lib` | Final lane run passed: 26 passed, zero failed, 864 filtered out. |
| `cargo test verifier --lib` | Passed: 20 passed, zero failed, 871 filtered out. |
| `cargo test task_turn --lib` | Passed: 37 passed, zero failed, 854 filtered out. |
| `cargo test proof_gate_ --lib` | Passed: 2 passed, zero failed, 889 filtered out. |
| `cargo test slot_round_trip_rejects_chronology_drift --lib` | Passed: 1 passed, zero failed, 890 filtered out. |
| `cargo test lca_cases_exercise_real_packet_and_request_without_transport --lib` | Final run passed: 1 passed, zero failed, 890 filtered out. All four fixture checkpoints matched. The final fix recognizes a current image as task authority only when P6 explicitly identifies the frame as the latest user goal or current unsent draft. |
| `cargo test continuation --lib -- --format terse` | Passed: 610 passed, zero failed, three ignored, 278 filtered out, in 78.64 seconds. This is the broad current superset of semantic-probe, production, verifier, task-turn, accuracy, continuation, and four-case replay coverage. |
| root TypeScript stage through bundled Node | Passed with no diagnostics. |
| root Vite build through bundled Node | Passed: 34 modules transformed; build completed in 663 ms. |
| webview tests through bundled Node | Passed: 32 passed, zero failed. |
| `git diff --check` | Passed after the final fixture edit. |

The plain `npm run build` and `npm run test:webview` wrappers initially failed with `env: node: No such file or directory`. Their exact TypeScript, Vite, and Node test stages then passed through the bundled workspace runtime. The broad continuation suite was run once after the focused replay became green.

## Known limitations reserved for LCA-03

- LCA-03 still owns the final local admission thresholds and policy across provider wording, confidence, P6 lifecycle bounds, and field combinations.
- LCA-02 does not make `where_summary` openable and does not loosen direct-target ownership or open-time validation.
- The deterministic fixture output proves the contract seam, not live provider quality or launch accuracy.
- The broader P6 release corpus, calibration, performance, holdout, and manual macOS gates remain outside this phase and remain unproven.

## Manual testing ownership

No Computer Use, browser automation, screenshots, live provider call, native-island visual pass, or manual app test was run or claimed. Live app and visual testing remain user-owned after the later launch-accuracy phases.

PASS — LCA-02 actionable contract is proven and LCA-03 may begin
