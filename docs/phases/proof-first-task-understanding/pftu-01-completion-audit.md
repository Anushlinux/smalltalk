# PFTU-01 Completion Audit

## Verdict summary

The small semantic kernel is implemented and feature-gated. Its focused tests, the broader Rust suites, the frontend build, and the webview presentation tests pass. A real provider transport smoke test also succeeded with both comparison models.

The phase is not complete. The required twelve fresh Tauri-development workflows have not completed, and no human has rated the scored outputs. The first human-driven Continue produced two real probe responses, but the workflow was paired with the wrong queued expectation and is therefore preserved as unscored diagnostic evidence. Provider transport success and deterministic tests do not satisfy the semantic proof gate.

## Confirmed root cause of the old request failures

The old Task Truth request combines too many responsibilities in one model response. It sends up to four images plus a large structured packet, asks for up to three hypotheses and sixteen semantic fields, and requires the model to reproduce opaque evidence keys and continuity identities while also understanding the task.

The checked-in session-038 evidence and live SQLite rows confirm the practical result:

- the largest observed old structured request was 131,580 bytes and 36,195 estimated text tokens, with three images and a 6,000-token output allowance;
- provider attempts included `request_invalid` and evidence-policy rejection states;
- the exported final decision `continue-decision-589031e7417c31cd` used `local_scorer` copy while Task Truth was unresolved;
- the live request path also recorded `manual_continue_current_external_frame_not_persisted` for the final manual attempt;
- deterministic fixtures proved contract behavior, but did not establish that a model could produce useful task meaning on fresh work.

This matches the phase hypothesis: the model was simultaneously doing semantic reasoning and exact bookkeeping under an overloaded schema. Later MFTI machinery therefore existed without the live accepted central-task proof required by its own dependency chain.

## Code paths changed

### Development semantic probe

- `src-tauri/src/continuation/task_truth_v2/semantic_probe.rs`
  - selects at most two chronological causal boundaries and four images;
  - builds a bounded request with a 24 KiB structured-text cap and a 6,144 estimated-token cap;
  - creates request-local support slots while retaining exact hashes, ownership, privacy state, timestamps, and source identities locally;
  - uses only four semantic answer fields: primary task, current step, last progress, and unfinished state;
  - validates every cited slot and nulls only the unsupported field;
  - never creates local replacement semantics;
  - separates request, privacy, provider, timeout, output, parse, support-validation, human-wrong, and success diagnostics;
  - compares two models, persists safe attempt metadata, exports a private review bundle, and evaluates a redacted human-reviewed corpus.
- `src-tauri/src/continuation/task_truth_v2.rs`
  - invokes the probe only inside the existing explicit manual Continue shadow path and only when `SMALLTALK_PFTU_SEMANTIC_PROBE_ENABLED=true`;
  - treats probe errors as non-authoritative diagnostics, so production Continue still completes.
- `src-tauri/src/continuation/task_truth_v2/model.rs`
  - exposes the existing image loading, base64, and provider-metadata helpers to the sibling probe module. The provider transport itself was reused.
- `src-tauri/src/bin/pftu_01_probe.rs`, `src-tauri/src/lib.rs`, and `src-tauri/Cargo.toml`
  - add local-only commands to arm pre-output expectations, export a private review bundle, and evaluate a redacted proof corpus.

### Mechanical verification repair

- `src/App.tsx`
  - removes one obsolete second argument from a one-argument copy helper. This was an existing TypeScript compile error discovered by `npm run build`; it does not change PFTU authority or UI design.
- `src-tauri/src/continuation/task_truth_v2/production.rs` and `review.rs`
  - contain formatting-only changes produced by the phase-required `cargo fmt --all` command.

## Production-authority and privacy audit

- The probe defaults off.
- Only the explicit manual Continue shadow path calls it.
- Startup and background Continue requests cannot invoke it.
- It does not change React copy, native-island copy, candidate ranking, open behavior, caches, task-thread state, or release authority.
- It reuses the current provider transport with `store=false`.
- Private or ineligible current frames stop request construction before transport.
- The model receives short slots and bounded summaries, not local paths, URLs, raw typed characters, clipboard text, legacy candidates, workstreams, feedback history, or prior hypotheses.
- SQLite stores safe request measurements, provider identities, token and latency data, local support mappings, and admitted semantic output. It does not store image bytes or raw provider payloads.
- The private review export warns that it must remain outside version control.

## Old versus new request shape

| Measurement | Session-038 old request | PFTU-01 probe |
| --- | ---: | ---: |
| Structured text bytes | 131,580 | 3,777 in the two-model transport smoke; hard cap 24,576 |
| Estimated text tokens | 36,195 | 945 in the transport smoke; hard cap 6,144 |
| Images | 3 in the largest measured old request | 4 in the transport smoke; hard cap 4 |
| Semantic fields | 16, plus identity and hypothesis machinery | 4 |
| Hypotheses | Up to 3 | 0 |
| Output allowance | 6,000 tokens | 1,200 tokens; smoke outputs were 845 bytes for Luna and 991 bytes for Sol |
| Evidence references | Opaque persisted keys and hashes copied by the model | Short request-local slots expanded and verified locally |

The smoke request was 97.1% smaller by structured bytes and 97.4% smaller by estimated text tokens than the measured session-038 request. Those values demonstrate request-size reduction only. They are not live semantic evidence.

## Model comparison

Official model guidance was rechecked on 2026-07-13. The development comparison uses:

- cost-efficient image-capable model: `gpt-5.6-luna`;
- stronger image-capable reasoning model: `gpt-5.6-sol`.

The repository's prior `SMALLTALK_TASK_TRUTH_MODEL` value was `gpt-5.5-mini`, which the live model list did not expose. The probe therefore supports an explicit `SMALLTALK_PFTU_COST_MODEL` override without changing production configuration.

The ignored transport smoke test was run explicitly with the two models above. Both returned a response identity and strict parseable output for the same synthetic packet. The measured structured request was 3,777 bytes and 945 estimated text tokens.

| Model | Admission diagnostic | Latency | Provider input tokens | Provider output tokens | Estimated cost | Output bytes |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| `gpt-5.6-luna` | `support_slot_validation_failure` | 5,720 ms | 2,046 | 705 | $0.006276 | 845 |
| `gpt-5.6-sol` | `success` | 7,098 ms | 2,046 | 493 | $0.025020 | 991 |

Luna's valid JSON was not counted as admitted success because its cited support did not pass local validation. Sol passed admission on this synthetic packet. This proves transport, schema parsing, and one validator path only. It is excluded from the semantic pass denominator because the image was a repository icon rather than fresh human work.

Estimated cost uses environment overrides when supplied. Otherwise the probe uses the official rates rechecked on 2026-07-13: Luna at $1.00 per million input tokens and $6.00 per million output tokens, and Sol at $5.00 per million input tokens and $30.00 per million output tokens.

## Twelve-case proof status

The original expectations below were written to the live database at timestamp `1783924838000`, before any provider output. Cases 9 through 12 are held back. The first user-run workflow showed a completed terminal test, but the queue consumed it as the named-code-change case. Its Luna and Sol responses remain in the database for diagnosis, but the run is excluded from every proof denominator because its prewritten expectation did not match the actual workflow. A replacement named-code-change expectation was armed at `1783927130000`, before its output, using the real manual-Continue crash fix as the named product behavior.

| Case | Required workflow | Partition | Output | Human judgment |
| --- | --- | --- | --- | --- |
| `pftu-live-01` | Named product code change | Tuning | Invalid workflow/expectation pairing; excluded | Not scored |
| `pftu-live-02` | Command verification | Tuning | Not run | Pending |
| `pftu-live-03` | Agent-response review | Tuning | Not run | Pending |
| `pftu-live-04` | Supporting browser research | Tuning | Not run | Pending |
| `pftu-live-05` | Supporting API dashboard | Tuning | Not run | Pending |
| `pftu-live-06` | Task-indeterminable application | Tuning | Not run | Pending |
| `pftu-live-07` | Completed action with no unfinished state | Tuning | Not run | Pending |
| `pftu-live-08` | Waiting for agent or command output | Tuning | Not run | Pending |
| `pftu-live-09` | Form with visible business purpose | Held back | Not run | Pending |
| `pftu-live-10` | Form with only form activity visible | Held back | Not run | Pending |
| `pftu-live-11` | Session-038 cross-application reconstruction | Held back | Not run | Pending |
| `pftu-live-12` | Previously unseen application | Held back | Not run | Pending |
| `pftu-live-01-rerun` | Named product code change: prevent manual Continue crash | Tuning replacement | Armed before output | Pending |

Tuning denominator: 0 completed scored cases of 8 required. Held-back denominator: 0 completed of 4 armed cases. Human-reviewed denominator: 0 of 12. The excluded mismatch is not counted as a thirteenth proof case.

## Automated verification

Completed successfully:

- `cargo fmt --all -- --check`
- `cargo check`
- `cargo check --features eval-binaries`
- `cargo test semantic_probe --lib`: 13 passed, 1 intentionally ignored live-provider test
- explicit two-model ignored transport smoke: 1 passed; Luna parsed with support validation failure, Sol passed admission
- `cargo test task_truth_v2 --lib`: 125 passed, 3 intentionally ignored live-provider tests
- `cargo test continuation --lib`: 518 passed, 3 intentionally ignored live-provider tests
- `cargo test --lib`: 731 passed, 3 intentionally ignored live-provider tests
- `cargo test manual_continue --lib`: 7 passed after the live empty-window crash fix
- `npm run build`
- `npm run test:webview`: 23 passed
- `git diff --check`

## Failures encountered and resulting changes

1. The configured `gpt-5.5-mini` provider model was unavailable. The probe gained an explicit, development-only cost-model override, and current official model guidance was used to select Luna and Sol.
2. The first frontend build found an unrelated one-argument helper being called with two arguments. The obsolete empty argument was removed, after which the build passed.
3. The initial focused coverage did not explicitly exercise resolved, partly resolved, and unresolved admission in one parsing test. Coverage was expanded, along with valid short-slot round trip, privacy-blocked slot, and timeout classification tests.
4. The installed Smalltalk `.app` has known problems outside this phase. Per user instruction, it was not launched or tested. `npm run tauri dev` successfully started Vite and `target/debug/smalltalk`, and rebuilt cleanly after changes. The development process did not expose its window to the available macOS automation interface, so no UI-only Continue proof was fabricated.
5. A real Tauri-development Continue from a non-Code surface crashed at `capture.rs:3882`. The code used `bool::then_some(matches[0])`; `then_some` eagerly evaluated the index even when the list was empty. The resolver now matches the slice safely and returns a window id only for exactly one match. Regression coverage proves zero, one, and multiple matches without indexing an empty list.
6. The first terminal workflow consumed the first queued expectation even though that expectation described a code-change workflow. The provider outputs are retained as diagnostic evidence but excluded from scoring. A replacement code-change expectation was written before output; no result is being relabeled after it was seen.

## Unsatisfied pass gates

- Twelve cases do not yet have typed provider outcomes.
- Fewer than ten cases have real provider round trips and parsed semantic output.
- No human review exists, so wrong-task, recoverable-field, primary-task, held-back, and understandability gates cannot be calculated.
- Field-level support admission has been proven in tests, but not across the twelve live cases.
- The request-size gate passes, but size reduction alone cannot advance the phase.

## Remaining work and risks

Run the twelve armed cases using the already-running Tauri development workflow, pressing the real Continue action once per case. Then export the private bundle with `pftu_01_probe export-review`, have a human rate every field for both models, create the redacted corpus, and run `pftu_01_probe evaluate`. Only a zero-violation report permits changing this audit to PASS.

The main semantic risk remains unknown until that review: the small packet may omit necessary business context, or either model may infer a plausible but wrong primary task. The validator can remove unsupported claims, but it intentionally cannot repair meaning.

INCOMPLETE — PFTU-02 is blocked
