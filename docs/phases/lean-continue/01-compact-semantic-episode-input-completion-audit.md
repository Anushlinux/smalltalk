# Compact Semantic Episode Input Completion Audit

## Verdict summary

The compact evidence boundary is implemented and deterministically verified. The request now closes at the exact manual Continue frame, chooses causal boundaries instead of grouping by frame count, deduplicates before serialization, exposes only short neutral support slots, uses only `gpt-5.6-luna`, and validates each semantic field independently without local semantic repair.

The phase is not complete. The required twelve fresh Luna cases and four held-back cases remain user-run manual work. No Computer Use test was run. After the original Prompt 01 implementation, the user explicitly requested an early normal-app cutover with rollback accepted. The compact result is therefore now routed through React and the native island for manual testing, even though the live proof gate below remains incomplete.

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

## July 14 compact-input quality repair

The next real compact run proved that request size alone was not the remaining problem. The request was small, but it selected the wrong evidence.

The exact run used decision `continue-decision-5ede8d74e10a6dbf` and packet `packet-d16e8c71a6b3f286`. The packet retained four external keyframes: WeMakeDevs, Devfolio, X Home before the interaction interval, and X Home at Continue. The compact request sent only the final two X images. Its short action list reduced the interval to scroll, click, scroll. Luna therefore had enough evidence to describe the visible feed movement, but no transported evidence from which to recover the earlier hackathon-application work.

The live database exposed three distinct defects:

1. A grounded scroll on the current surface suppressed all earlier context, even though a scroll proves only that the screen changed. It does not prove that the current surface contains the primary task.
2. Frame 470 loaded the outgoing `470 -> 471` ChatGPT transition while its frame diff described the incoming `469 -> 470` edge. Merging those two different edges created the false `transition:switched_app` fact inside an X-only boundary.
3. The 46-second interval between the two X frames contained many recorded title changes and clicks through a reply, profiles, and repeated returns to Home. The capture loop treated all same-app changes as low-value traffic, waited on the 45-second interval, and stored only the final Home state. Request deduplication then collapsed the retained actions without preserving the navigation path.

The repair changes the evidence flow rather than asking Luna to guess around those losses:

- A frame now owns only events, triggers, and transitions on its incoming edge. Events after the frame cutoff and outgoing transitions cannot contaminate it.
- A transition is merged into a frame delta only when its pre-frame and post-frame exactly match that delta.
- A current click or scroll no longer suppresses a recent transition into the current surface when continued activity shows a detour, return, or supporting visit.
- Same-app page, tab, document, and window-title changes receive a bounded `surface_change` capture opportunity every four seconds instead of waiting behind the 45-second low-value interval. These captures still pass through privacy checks and the rolling screenshot budget; `surface_change` does not bypass that budget.
- Under four-keyframe pressure, the packet reserves the most recent different browser origin and the entry frame into the current origin. Newer pages from a detour therefore cannot crowd the earlier task-bearing origin out before request selection.
- Shared images and source records are emitted once across boundaries. The structured input labels earlier versus current chronology and uses plain statements such as `The user scrolled on this surface` instead of `change_kind`, `committed=None`, and internal transition classifier strings.

The deterministic live-shaped regression now produces two chronological boundaries and three unique images for a Devfolio-to-X scroll case. It proves that the Devfolio image remains available as earlier task context, the X images remain current context, duplicate source records are not serialized, and an image-grounded earlier task can survive field-local admission. This is deterministic evidence-shaping proof only; a fresh privacy-approved Luna run is still required to measure semantic improvement.

## July 14 lost-session-context repair

A later live run exposed a second selection failure. Capture had retained ChatGPT, Devfolio, Google research, the OpenAI Logs list, and the final blank response page. Request construction still sent only the final Logs-list to blank-page boundary. Luna accurately described those two screenshots, but it could not recover session context that was never transported.

The repair now separates factual session chronology from semantic task inference:

- Manual Continue loads meaningful foreground visits from the current session through the exact persisted cutoff. Consecutive duplicates collapse, but departures and returns remain separate. The public/request chronology is capped at eight visits.
- Browser facts are limited to application label plus normalized hostname. URL paths, queries, document paths, and page titles are never added to the compact request. A private visit is retained locally only as `Private activity`, with no hostname or image.
- ChatGPT and Codex are ordinary work surfaces. Only Smalltalk's own interface is categorically excluded as self-evidence.
- Request schema `smalltalk.pftu_01.semantic_probe_request.v3` adds `recent_surface_timeline`, neutral `context_image` slots, and exact request-local visit ids. The four-image cap remains. The current image is reserved first; earlier images are ranked by observed dwell, grounded interactions, distinct surface, and recency. A current-before image is included only when it is visually distinct and capacity remains.
- Timeline text is context, not proof of a task. Only a cited image or other admitted support slot can establish `primary_task` or earlier progress.
- Public answer schema `smalltalk.task_truth_public_answer.v4` carries factual `recent_context` plus any locally admitted provider role, confidence, relationship explanation, and evidence references. React shows up to eight visits for resolved, partial, and unresolved answers. The native island shows the latest four and points to Smalltalk when more exist.
- The request audit retains every selected timeline visit, per-visit image slot or omission reason, representative-frame reasons, budget decisions, and typed exclusion counts for private, missing-crop, ownership-rejected, and budget-omitted images.

The exact deterministic regression is shaped as ChatGPT to Devfolio to Google to OpenAI Logs list to blank response page. It produces three earlier context images plus the reserved current image. The blank final page cannot consume the full image budget. A separate regression proves that an unresolved provider result still exposes the factual Recent context list without inventing a primary task.

No live provider request was made for this repair. The required privacy-approved reproduction remains a user-controlled manual gate.

## July 14 Codex-workspace and semantic-role repair

The next user reproduction proved that the session timeline was still mechanically wrong before Luna saw it. The actual order was Google, OpenAI documentation, roughly 111 seconds of Codex work, then the same OpenAI documentation again. The compact request omitted Codex, merged the two documentation visits, credited Codex clicks and scrolls to the final Helium frame, and rejected the cited context image because two metadata copies of the same physical frame had different selection reasons.

The repair separates factual validation from semantic judgment:

- Codex is no longer excluded by application identity. Smalltalk's own interface remains hidden.
- Hidden self frames remain chronology barriers. They are not emitted as visits, but a visible surface on either side cannot merge across them.
- UI events with an explicit application or window mismatch are excluded before visit engagement scoring, representative-image ranking, causal-event creation, and request phrasing. The packet records `action_surface_ownership_mismatch_excluded:<count>`.
- Keyframe citations now use a physical-frame fingerprint. It includes capture, surface, image, privacy, and ownership facts, but excludes collection-specific `partition`, `selection_reasons`, and the ephemeral image path. All packet copies of one frame id must agree on that physical fingerprint.
- Response schema `smalltalk.pftu_01.semantic_probe_response.v2` requires Luna to classify every imaged visit as `primary_work`, `supporting_work`, `detour_or_unrelated`, or `unclear`, with confidence, the visit's own image slot, and a short relationship explanation.
- Local admission validates the exact visit set, citations, privacy, ownership, chronology, and physical fingerprint. It can downgrade an invalid role to `unclear`, but it never invents a replacement role.
- The current visit's admitted provider role becomes `current_activity.relationship_to_primary`; the factual `Current` and `Returned later` labels remain separate.

Deterministic regressions cover Codex as real work, visible-surface returns across Codex, hidden-Smalltalk chronology barriers, cross-app event exclusion, physical-frame fingerprint stability, conflicting physical variants, exact visit-role schemas, field-local role rejection, old-row compatibility, and public React/native projection. The original Google to OpenAI docs to Codex to OpenAI docs workflow still requires one user-run live Luna test.

## July 14 structured-output truncation repair

The first live run after adding per-visit roles reached the provider but returned no admitted answer. The persisted run proves this was not an evidence abstention: it ended with `diagnostic_status=provider_no_usable_output`, `failure_reason=provider_response_incomplete`, and exactly 1,200 output tokens, matching the request's old 1,200-token ceiling. The response text ended inside `primary_task`, so it was not valid JSON.

The compact request now allows up to 6,000 output tokens. This is an upper bound, not a requirement to generate 6,000 tokens. It gives Luna room for model reasoning plus the strict JSON contract, which now contains four semantic fields and as many as four per-visit role objects. The chosen ceiling matches the already-proven legacy Luna allowance while preserving the compact input, four-image cap, one-provider-post rule, and zero automatic retries.

The request audit now records `max_output_tokens`. A deterministic regression checks that the largest four-image/eight-field contract receives the 6,000-token allowance. A separate regression proves that a provider envelope marked `incomplete` is still rejected rather than attempting to salvage partial JSON.

React now presents compact-probe transport and validation states separately. `provider_no_usable_output` is a retryable no-usable-answer failure; `structured_parse_failure` is an invalid model response; `support_slot_validation_failure` is a local evidence-verifier rejection; and provider rejection or unavailability is no longer mislabeled as insufficient evidence. The copy stays deliberately general because that coarse status can represent an incomplete response, a refusal, or empty output. The native island treats the same states as inference failures. The phrase “There is not enough evidence” is therefore reserved for a genuine unresolved semantic answer.

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
| Output allowance | 6,000 tokens | 6,000-token ceiling for reasoning plus strict JSON |
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
8. One earlier boundary may be included when it is a grounded transition into the current surface followed by continued activity. A current scroll or click does not suppress that context merely because it changed the visible screen. The older same-surface fallback still requires the current boundary to lack a grounded material result.
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

## Post-phase normal-app integration

The user explicitly authorized the compact path to become the manual Continue semantic path before the live corpus was complete. Normal `npm run tauri dev` no longer needs an armed case id or a PFTU launcher. Evaluation runs can still use an armed case, while ordinary app runs create a private internal `production_runtime` record tied to the exact decision id.

For explicit manual Continue, the compact Luna result now becomes the public semantic answer. The legacy full-packet resolver, reconciliation path, micro-inference call, and model-assisted activity recap call are not executed. A failed or rejected compact attempt becomes typed unresolved rather than exposing legacy semantic prose. This is an implementation cutover for user testing, not a claim that the Prompt 01 or Prompt 03 live release gates have passed.

## Automated verification

Completed successfully:

- `cargo fmt --all -- --check`
- `cargo check`
- focused Task Truth v2 module tests: passed; live-provider tests remained intentionally ignored
- focused semantic-probe tests: passed; the live-provider transport test remained intentionally ignored
- `cargo test`: 783 passed, 3 intentionally ignored live-provider tests
- focused 11-late-event fixture: 1 passed; 1,636 structured bytes, 409 estimated text tokens, two images, one sufficient current boundary
- focused response-storage policy regression: 1 passed
- focused incoming-edge ownership regression: 1 passed
- focused same-app surface-change capture regression: 1 passed
- focused browser-detour keyframe-pressure regression: 1 passed
- focused live-shaped Devfolio-to-X compact-request regression: 1 passed
- focused ChatGPT-to-Devfolio-to-Google-to-OpenAI session chronology and four-image allocation regressions: passed
- focused private redaction, typed image omission, duplicate-before-image, old-row deserialization, and unresolved Recent context regressions: passed
- frontend Continue presentation suite: 24 passed
- frontend `build` script through the bundled Node and pnpm runtime: passed
- Swift native-island type-check: passed with one pre-existing macOS 14 deprecation warning
- `git diff --check`: passed at handoff

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

- `src-tauri/src/continuation/task_truth_v2/observation_packet.rs`: exact transition/delta edge alignment and reserved browser-origin context under detour keyframe pressure.
- `src-tauri/src/continuation/task_truth_v2/semantic_probe.rs`: authoritative cutoff, causal boundary selection, pre-serialization deduplication, compact neutral slots, readable chronology, expanded audit, Luna-only enforcement, chronological round-trip validation, field-local admission, and focused tests.
- `src-tauri/src/continuation/task_truth_v2.rs`: makes the compact probe the sole explicit-manual semantic path and leaves the legacy full-packet resolver unreachable from that path.
- `src-tauri/src/continuation/task_truth_v2/production.rs`: projects the exact decision-bound compact result and blocks legacy model-answer or snapshot prose from filling a manual answer.
- `src-tauri/src/continuation.rs` and `src/App.tsx`: disable the older micro-inference and model-assisted recap requests so normal Continue does not create extra provider posts.
- `scripts/tauri-dev-mfti.sh`: documents that normal model-first development uses compact Luna without an armed case.
- `src-tauri/src/capture.rs`: aligns the existing request-storage test with the checkout's configurable development storage policy and promotes bounded same-app surface changes without bypassing privacy or rolling capture budgets.
- `docs/phases/lean-continue/01-compact-semantic-episode-input-completion-audit.md`: this audit and manual test contract.

INCOMPLETE — 02-continuation-useful-output-contract is blocked
