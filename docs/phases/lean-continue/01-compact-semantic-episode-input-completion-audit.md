# Compact Semantic Episode Input Completion Audit

## Verdict summary

The compact evidence boundary is implemented and deterministically verified. The request now closes at the exact manual Continue frame, chooses causal boundaries instead of grouping by frame count, deduplicates before serialization, exposes only short neutral support slots, uses only `gpt-5.6-luna`, and validates each semantic field independently without local semantic repair.

The phase is not complete. The required twelve fresh Luna cases and four held-back cases remain user-run manual work. No React, native-island, or Computer Use test was run. The compact path remains feature-gated and is explicitly blocked from public authority until Prompt 03.

## July 14 user retest diagnosis

The provider input attached after the first user retest was not produced by this compact probe. It was a legacy Task Truth request.

Authoritative evidence:

- The attached text is 61,869 characters and starts with `evidence_reference_catalog`, followed by the complete `packet`, prior snapshot/thread context, and request policy. A compact request starts with `smalltalk.pftu_01.semantic_probe_request.v1` and contains only the cutoff, one or two boundaries, short `B...` slots, and missing-evidence notes.
- The attached packet id is `packet-0f8ccc1a89a4c63f`. Its live SQLite shadow-audit row reports 48,602 serialized packet bytes, 12,151 estimated packet tokens, and 50,361 ms latency.
- No `task_truth_v2_semantic_probe_runs` row exists for that session or timestamp. Therefore the compact probe did not make that provider call.
- Earlier real compact-probe rows in the same database contain 1,720 to 3,653 structured bytes, 430 to 914 estimated structured-text tokens, and 3,798 to 6,415 actual provider input tokens. Those rows prove that the compact request is materially different from the attached legacy request.

The cause was a fail-open development launcher: `scripts/tauri-dev-pftu.sh` disabled the probe when launched without a case id, which silently allowed the legacy request. The launcher is now compact-only:

- a case id is mandatory;
- the exact case must exist, be unconsumed, and have a fresh pre-output expectation in the selected live database;
- `SMALLTALK_PFTU_COMPACT_ONLY=1` forces the compact probe to own the attempt even if the ordinary probe flag is accidentally false;
- when the compact probe cannot run, the attempt fails closed instead of spending a legacy Task Truth request;
- the launch banner states `legacy_request=blocked`.

The one-image observation had a separate, valid privacy cause. Live frame 441 was the Terminal frame, but its `privacy_status` is `redacted`; `sensitive_regions` records an `api_key` match with `redacted_text`. That frame must not be sent to Luna. The later Visual Studio Code frame was normal and model-eligible, so it was the only transportable image in that evidence window. This phase does not weaken that privacy rule.

The reported white-island/capture instability is recorded as a separate runtime issue. It was not changed in this compact-input repair because the live evidence connects the Terminal transition to privacy redaction, and the user explicitly requested focus on input compaction.

## Dirty-checkout preservation

The checkout was already dirty before this phase. Existing modifications in `PRODUCT.md`, React, native island, capture, continuation, activity recap, Task Truth model, production projection, and semantic probe code were treated as user-owned. The supplied top-level prompts and `rsp1/` and `rsp2/` logs were not modified, copied, sanitized, or added to source code.

No reset, checkout, cleanup, generated-export rewrite, or destructive Git command was used.

## Supplied-log reproduction

The supplied full logs were inspected locally. Their measurements match the phase prompt.

| Measurement | `rsp1` | `rsp2` |
| --- | ---: | ---: |
| Provider input tokens | 53,301 | 58,549 |
| Provider output tokens | 3,455 | 2,691 |
| Main structured user input characters | 140,454 | 160,837 |
| Packet characters | 116,510 | 116,510 |
| Canonical-element characters | 88,666 | 88,666 |
| Causal-event characters | 22,111 | 22,111 |
| Evidence-catalogue characters | 19,016 | 19,016 |
| Reconciliation characters | 4 (`null`) | 20,387 |
| Strict output-schema characters | 36,757 | 36,757 |
| Maximum output tokens | 6,000 | 6,000 |

The first request contains 96 canonical elements, of which 27 are task-eligible. It has 47 unique non-null text-reference hashes, no focused elements, and no elements with causal evidence references. One element repeats `ax_ocr_text_disagreement` 167 times. The request contains 32 causal events. Eleven occur after the current-frame cutoff, and the latest is 5,411 ms after that cutoff.

## Before-and-after request shape

| Property | Legacy request | Compact request |
| --- | --- | --- |
| Temporal boundary | Included 11 post-current events | Exact current-frame cutoff; record and effective target times must both be at or before it |
| Boundary selection | Recent frame inventory | Current manual boundary first; at most one related causal boundary |
| Evidence serialization | Full packet plus repeated evidence catalogue | Short request-local slots only; full mapping remains local |
| Structured text | 140,454 characters in `rsp1` | 1,636 bytes and 409 estimated text tokens in the deterministic 11-late-event regression |
| Images | Two high-detail screenshots in supplied request | At most four images across at most two boundaries |
| Output contract | Large hypothesis, identity, locator, and forensic schema | Four semantic fields, support slots, missing evidence, confidence, and status |
| Output allowance | 6,000 tokens | 1,200 tokens |
| Model | Luna under the large legacy contract | Luna only; every other runtime model is rejected before transport |

The deterministic compact cutoff fixture is 98.8% smaller than the 140,454-character legacy structured input. It uses one sufficient current boundary and two images; the selector adds an earlier boundary only when the current boundary lacks a grounded material result. Live provider token medians and maxima remain pending until the manual corpus is run.

## Final boundary-selection rules

1. The final frame must have a non-empty identity, exactly match the packet observation time, be model-eligible, and not be private.
2. The final frame time is the authoritative cutoff.
3. Frames, observations, events, and deltas after the cutoff are excluded before slot construction.
4. An event with an earlier own timestamp is still excluded when its target frame is after the cutoff.
5. A delta is eligible only when its effective next frame is known and is at or before the cutoff.
6. The current boundary always exists and is recorded as `current_manual_boundary`.
7. Its before image is included only when a material delta or meaningful causal event directly connects it to the current frame.
8. One earlier boundary may be included only when the current boundary lacks a grounded material result, a user-grounded event has a material result, and the result belongs to the same owned surface as the current work.
9. An unrelated recent frame is never selected merely because it is recent.
10. Passive Accessibility notifications, capture bookkeeping, focus/window metadata, and no-effect scrolls do not become action slots.
11. Slots within each serialized boundary are ordered by observed time, then a stable neutral category order and slot id. They are not emitted in map-key order.

## Deduplication and compact-slot proof

Deduplication occurs before request JSON is created.

- Canonical observations use stable content, ownership, region, frame, and bounds identity. Exact duplicates collapse, while the same text in a different frame, owner, region, or position remains distinct.
- Repeated text hashes collapse only when they refer to the same owned region and state.
- Event duplicates use event kind, source/target state, target element, semantic delta, and committed state.
- A scroll with an explicit no-op delta cannot borrow an unrelated material delta from the same frame.
- Material deltas deduplicate by before/after identity, change kind, observable facts, summary hash, and changed regions.
- Conflict, rejection, and missing-evidence strings are deduplicated.
- Surface identity is used locally to establish boundary continuity. App names, bundle ids, paths, URLs, packet ids, and surface hashes are not serialized as support slots.

The persisted request audit now records the final frame id, cutoff, earliest admitted time, selection reasons, raw candidate counts, admitted counts, deduplication counts, late exclusions by record kind, non-semantic exclusions, image measurements, structured bytes, and estimated text tokens.

## Mechanical admission rules

- Every non-null semantic field must cite at least one slot from the exact request.
- The slot must retain the exact source fingerprint and content hash.
- The source and any event target or delta next frame must remain at or before the request cutoff.
- Private, foreign, stale, future, unsupported-category, and overlong values null only the affected field.
- Semantic text over 320 characters is rejected rather than truncated or rewritten.
- Generic primary tasks such as `editing`, `browsing`, `reviewing`, or `editing code` are rejected. Concrete purposes that begin with an activity word, such as `reviewing the agent output for future-event leakage`, are preserved.
- Evidence containing only passive navigation or scrolling cannot establish a primary task.
- Local code does not fill rejected fields from Task Truth, current focus, activity recap, app identity, or stored episode prose.
- Any field-level rejection remains a separate `support_slot_validation_failure`; a surviving field does not turn the response into an unqualified success.

## Non-public authority proof

The probe still requires an armed case id and an explicit proof-mode signal: either `SMALLTALK_PFTU_SEMANTIC_PROBE_ENABLED` or the stricter `SMALLTALK_PFTU_COMPACT_ONLY` launcher guard. `public_authority_enabled()` is deliberately hard-coded to `false`; there is no environment override in this phase. The probe does not attach a return target and does not become the React or island answer. Prompt 03 owns that cutover.

## Automated verification

Completed successfully:

- `cargo fmt --all -- --check`
- `cargo check`
- `cargo test task_truth_v2`: 149 passed, 3 intentionally ignored live-provider tests
- `cargo test semantic_probe`: 34 passed, 1 intentionally ignored live-provider test
- `cargo test --lib --quiet`: 759 passed, 3 intentionally ignored live-provider tests
- focused 11-late-event fixture: 1 passed; 1,636 structured bytes, 409 estimated text tokens, two images, one sufficient current boundary
- focused response-storage policy regression: 1 passed
- frontend `build` script through the bundled Node and pnpm runtime: passed
- `git diff --check`: passed before the final completion-artifact refresh; it must be rerun at handoff

The final strict audit added three regressions beyond the original implementation pass: sufficient current evidence now omits an unnecessary earlier boundary; request slots are proven chronological after JSON serialization; and concrete purposes beginning with an activity verb are no longer mistaken for forbidden generic labels.

After the July 14 retest diagnosis, the compact-only fallback regression also passes. Shell syntax validation passes, and the launcher was directly proven to reject both an already-consumed case and an armed-but-stale case before starting Tauri.

The first full Rust run found one stale test assertion from the existing development response-storage change. The test had hard-coded `store=false`, while the user-owned implementation intentionally makes storage configurable and defaults it on in development. The assertion now checks the configured policy. The focused regression and the full suite both pass after that repair.

## Manual Luna test procedure

This phase is non-public. Judge `admitted_output_json` in the private probe export, not the React card or native island.

For each case:

1. Perform the stated setup without pressing Continue.
2. Write the expectation JSON before any provider output. Set `expected_recorded_at_ms` to the current epoch time in milliseconds. It must be earlier than the Continue press and no more than 15 minutes old.
3. Arm that one case against the live capture database:

   ```bash
   cd src-tauri
   cargo run --features eval-binaries --bin pftu_01_probe -- \
     arm --database <DB_PATH> --input <CASE_JSON>
   cd ..
   SMALLTALK_PFTU_DATABASE="<DB_PATH>" ./scripts/tauri-dev-pftu.sh <CASE_ID>
   ```

   The launcher must print `legacy_request=blocked`. If it refuses the launch, fix the named case/database/freshness problem instead of falling back to `npm run tauri dev`.

4. Press Continue exactly once for that case. Do not judge the public card.
5. After the run, export the private review bundle outside the repository:

   ```bash
   cd src-tauri
   cargo run --features eval-binaries --bin pftu_01_probe -- \
     export-review --database <DB_PATH> --output /tmp/compact-luna-review.json
   ```

6. Confirm the model is exactly `gpt-5.6-luna`. Rate each recoverable field as `Correct`, `Partly right`, `Wrong`, or `Should be unresolved`.
7. Never commit the database, review export, screenshots, OCR/Accessibility text, paths, URLs, API keys, or provider payloads.

Use this case-file shape:

```json
{
  "case_id": "compact-live-01",
  "case_kind": "named_code_behavior",
  "held_back": false,
  "expected_recorded_at_ms": 0,
  "expected_primary_task": "Implement future-evidence rejection in the compact Continue boundary",
  "expected_current_step": "Add or review the final-frame cutoff regression",
  "expected_last_progress": "The cutoff selector and regression code are present",
  "expected_unfinished_state": "Run or inspect the focused cutoff test",
  "recoverable_by_field": {
    "primary_task": true,
    "current_step": true,
    "last_progress": true,
    "unfinished_state": true
  }
}
```

Replace the timestamp immediately before arming. Use `null` for a non-recoverable expected field and set its recoverability to `false`.

## Twelve manual inputs and expected outputs

Cases 9 through 12 are held back. Do not tune the prompt or validator after viewing those outputs.

| Case | Manual input | Expected admitted output |
| --- | --- | --- |
| `compact-live-01` | In a code editor, make a small change beside visible text: `Goal: implement future-evidence rejection in the compact Continue boundary`, `Current: add the final-frame cutoff regression`, `Done: selector code exists`, `Remaining: run the focused test`. | `primary_task`: implement future-evidence rejection. `current_step`: add/review cutoff regression. `last_progress`: selector/regression exists. `unfinished_state`: run or inspect focused test. All four recoverable. |
| `compact-live-02` | In a terminal run `cd src-tauri && cargo test supplied_log_style_cutoff_excludes_all_eleven_late_events --lib` and press Continue with the passing output visible. | `primary_task`: verify the compact cutoff. `current_step`: run the focused cutoff regression. `last_progress`: the 11-late-event test passed. `unfinished_state`: `null` unless the screen explicitly shows another required check. |
| `compact-live-03` | Ask an agent: `Review the compact semantic boundary for future-evidence leakage. Do not edit files.` Wait for its completed response, then press Continue while reviewing it. | `primary_task`: review compact-boundary leakage safety. `current_step`: review the agent result. `last_progress`: the agent completed its review. `unfinished_state`: act on named findings only if the result visibly contains them; otherwise `null`. |
| `compact-live-04` | In a browser search for `Rust serde default backward compatible audit fields`, open the relevant documentation, and press Continue without unrelated tabs in front. | `primary_task`: verify backward-compatible audit deserialization. `current_step`: research serde defaults. `last_progress`: relevant documentation was opened. `unfinished_state`: apply or confirm the audit-field compatibility rule. The browser page title alone must not become the task. |
| `compact-live-05` | Open the OpenAI response details for a compact probe run and keep token usage and the Luna model visible. | `primary_task`: inspect compact-probe request/response measurements. `current_step`: review the Luna response and token usage. `last_progress`: a provider response completed. `unfinished_state`: compare the measurement with the 24 KiB and 6,144-token caps. |
| `compact-live-06` | Open a neutral long article directly, do not search for a goal, scroll passively several times, then press Continue. | `primary_task`: `null`. `current_step`: may describe the concrete visible section or passive scroll. `last_progress`: `null` unless a real material navigation is visible. `unfinished_state`: `null`. Status must be unresolved or partly resolved, never confidently task-resolved. |
| `compact-live-07` | Show the same explicit code goal as case 1. Press Continue. Wait until the request has visibly started, then switch to Finder. | The selected final frame and all cited slots remain from the pre-switch task. Finder must not appear as support or change `primary_task`. Every admitted slot time is at or before the persisted cutoff. |
| `compact-live-08` | Use a surface known to produce both Accessibility and OCR evidence for the same owned editor region. Show `Goal: deduplicate compact evidence before serialization`, make one committed edit, then press Continue. If the audit shows no duplicate source candidates, discard and retry on a known dual-source surface. | `primary_task`: deduplicate compact evidence. The semantic fields remain correct. Audit raw observation/reason counts exceed admitted counts and at least one deduplication count is non-zero. No duplicate full record appears twice in the structured request. |
| `compact-live-09` | Held back. Run `sh -c 'echo "Goal: verify completed-state handling"; true; echo "COMPLETED_STATE_CASE_PASSED"'`. Wait for the shell prompt, then press Continue. | `primary_task`: verify completed-state handling. `current_step`: verification completed. `last_progress`: command completed successfully. `unfinished_state`: `null`. |
| `compact-live-10` | Held back. Run `sh -c 'echo "Goal: verify waiting-state handling"; sleep 30; echo "WAIT_CASE_DONE"'` and press Continue before `WAIT_CASE_DONE` appears. | `primary_task`: verify waiting-state handling. `current_step`: wait for command output. `last_progress`: command started. `unfinished_state`: wait for completion/result. |
| `compact-live-11` | Held back. In the editor show `Goal: make compact request ordering deterministic` and make a small change. Briefly research `Rust BTreeMap deterministic ordering`, return to the same editor, and press Continue. | `primary_task`: make compact request ordering deterministic. `current_step`: continue the editor change after the detour. `last_progress`: non-recoverable unless the admitted earlier boundary directly proves it. `unfinished_state`: run/inspect ordering verification if visibly stated. The detour must not replace the primary task. |
| `compact-live-12` | Held back. Use an application not present in the other eleven cases. In an editable surface show `Goal: validate compact Continue in an unfamiliar application`, `Current: draft the test note`, `Done: unfamiliar app opened`, `Remaining: press Continue`, then make one edit and press Continue. | `primary_task`: validate compact Continue in an unfamiliar app. `current_step`: draft the test note. `last_progress`: unfamiliar app opened/note started. `unfinished_state`: run Continue and review the result. App unfamiliarity must not create a generic app-name task. |

## Manual result rows

| Case | Partition | Luna round trip | Boundary reasons | Cutoff proof | Bytes/tokens | Admitted fields | Human rating | Correction |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `compact-live-01` | Tuning | Pending | Pending | Pending | Pending | Pending | Pending | Pending |
| `compact-live-02` | Tuning | Pending | Pending | Pending | Pending | Pending | Pending | Pending |
| `compact-live-03` | Tuning | Pending | Pending | Pending | Pending | Pending | Pending | Pending |
| `compact-live-04` | Tuning | Pending | Pending | Pending | Pending | Pending | Pending | Pending |
| `compact-live-05` | Tuning | Pending | Pending | Pending | Pending | Pending | Pending | Pending |
| `compact-live-06` | Tuning | Pending | Pending | Pending | Pending | Pending | Pending | Pending |
| `compact-live-07` | Tuning | Pending | Pending | Pending | Pending | Pending | Pending | Pending |
| `compact-live-08` | Tuning | Pending | Pending | Pending | Pending | Pending | Pending | Pending |
| `compact-live-09` | Held back | Pending | Pending | Pending | Pending | Pending | Pending | Pending |
| `compact-live-10` | Held back | Pending | Pending | Pending | Pending | Pending | Pending | Pending |
| `compact-live-11` | Held back | Pending | Pending | Pending | Pending | Pending | Pending | Pending |
| `compact-live-12` | Held back | Pending | Pending | Pending | Pending | Pending | Pending | Pending |

## Unsatisfied pass gates

- Zero of twelve fresh cases have been run under this implementation.
- Fewer than ten cases have a real Luna round trip and parsed response.
- No human field ratings exist.
- The zero-confident-wrong-primary-task rule is not yet measured.
- Recoverable-field and primary-task accuracy denominators are not yet measured.
- The four held-back cases are not yet run.
- Live provider input-token median, maximum, and outliers are not yet recorded.

## File-level phase changes

- `src-tauri/src/continuation/task_truth_v2/semantic_probe.rs`: authoritative cutoff, causal boundary selection, pre-serialization deduplication, compact neutral slots, expanded audit, Luna-only enforcement, chronological round-trip validation, field-local admission, and focused tests.
- `src-tauri/src/continuation/task_truth_v2.rs`: passes manual preflight failure into the probe so failed current-frame capture is persisted as a typed non-request rather than sending an old frame.
- `src-tauri/src/continuation/task_truth_v2/production.rs`: keeps Prompt 01 non-public and makes an older partial legacy diagnostic fail closed instead of crashing Continue.
- `src-tauri/src/capture.rs`: aligns the existing request-storage test with the checkout's configurable development storage policy.
- `docs/phases/lean-continue/01-compact-semantic-episode-input-completion-audit.md`: this audit and manual test contract.

INCOMPLETE — 02-continuation-useful-output-contract is blocked
