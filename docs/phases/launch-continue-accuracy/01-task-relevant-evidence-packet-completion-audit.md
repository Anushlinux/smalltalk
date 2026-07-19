# LCA-01 Task-Relevant Evidence Packet Completion Audit

Date: 2026-07-19

## Verdict summary

The normal manual `Continue` path now builds a task-relevance layer from P6 evidence before it selects model-facing boundaries and images. The implementation contains the required role ordering, pane separation, completion-boundary handling, near-duplicate collapse, image preparation, privacy audit, and one-request limits.

The post-fix four-case replay passes both the task-relevant packet and compact semantic request checkpoints for every critical case without provider transport. The broader continuation suite also passes with 602 tests successful, zero failures, and three explicitly ignored live/private tests. LCA-01 is proven and LCA-02 may begin.

## Before and after evidence-selection flow

Before:

```text
manual Continue cutoff
  -> persist current frame
  -> build ObservationPacketV2
  -> group a factual surface timeline
  -> choose representative frames using model eligibility, activity, and recency
  -> rank cross-surface context images using engagement, duration, distinct surface, and recency
  -> add the current causal boundary
  -> send up to four original or existing active-window images
  -> ask Luna to infer task relevance and visit roles
```

This allowed interaction count and dwell to affect which cross-surface pixels reached the model before task relation had been established. The compact request did not contain structured P6 user, agent, prior-boundary, pane, and task-turn evidence.

After:

```text
manual Continue cutoff
  -> persist current frame
  -> load P6 salient spans, ordered evidence spans, and selected task turn through the cutoff
  -> build ObservationPacketV2 with smalltalk.task_relevance_evidence.v1
  -> preserve pane ownership, reading order, authorship, task relation, and completion boundaries
  -> select task-state transitions in task-authority order
  -> use engagement only as a tie-breaker inside an already-related task turn
  -> collapse physical or semantic near-duplicates unless the task fact changed
  -> prepare a reliable owned crop, or reject the crop and use a bounded fallback
  -> send at most two chronological boundaries and four unique prepared images
  -> ask Luna to resolve only the bounded, role-separated evidence
```

The factual current frame is retained for state awareness, but it does not automatically become the task authority.

## Production files and contracts changed

- `src-tauri/src/continuation/task_truth_v2.rs`: the two normal manual compact-request entry points load the P6 task-relevance evidence and pass it into packet construction.
- `src-tauri/src/continuation/task_truth_v2/observation_packet.rs`: adds `smalltalk.task_relevance_evidence.v1`, typed span roles, task-turn metadata, pane ambiguity, same-task relation, neutral hostname mention, near-duplicate groups, and selected/rejected image-candidate audits.
- `src-tauri/src/continuation/task_truth_v2/semantic_probe.rs`: advances the request to `smalltalk.pftu_01.semantic_probe_request.v6`; applies task-first selection; removes engagement fields from model-facing visits; adds bounded image preparation and preparation audits; preserves one provider post with zero retries.
- `src-tauri/src/continuation/accuracy_fixture.rs` and `accuracy_eval.rs`: add four privacy-safe launch-accuracy scenarios and real packet/request checkpoints without transport.
- `src-tauri/tests/fixtures/continue_accuracy/cases/lca_*.json`: add the four synthetic causal-shape fixtures. They contain no supplied screenshots or private conversation transcripts.
- `src-tauri/tests/fixtures/continue_accuracy/README.md`: records the expanded 12-case corpus and the launch-accuracy fixture boundary.
- `model.rs`, `production.rs`, `task_snapshot.rs`, `task_thread.rs`, and `verifier.rs`: update deterministic packet constructors for the new typed fields. They do not change public answer policy.

No React surface, native-island presentation, public answer wording, target-opening policy, browser extension, schema migration, or background uploader was added.

## P6 evidence reuse

The packet reads these existing P6 contracts rather than inventing a parallel role parser:

- `continue_ordered_evidence_spans` supplies pane, region, conversational role, reading order, ownership, focus, selection, submission, geometry, confidence, privacy, and hashed text references.
- `continue_salient_turn_evidence` supplies the latest user, current agent, prior-boundary, and salient span selections.
- the selected `smalltalk.current_task_turn.v2` row supplies task-turn identity, execution state, actor, waiting state, relation to prior work, and separate confidence dimensions.

The resulting authority order is current unsent draft or latest user goal, directly related current agent/result state, immediately prior task boundary, current-task context, supporting context, flattened fallback, then unknown. The draft is explicitly `submitted=false`. Flattened fallback remains available for missing P6 data but is marked non-authoritative and cannot justify confident speaker, pane, or task-turn claims.

## Four critical deterministic checkpoints

| Case | Expected result | Evidence obtained | Final status |
| --- | --- | --- | --- |
| `05cd` | Product-need review is primary; the related answer state remains attached; earlier discussion is support | The dedicated semantic test and real packet/request replay both retain the attributed user goal and related agent state. Earlier context has no primary authority. | Packet and request checkpoints match |
| `0d1c` | New visual-cue request is primary; completed backend connection is prior context; launch checklist is a detour | The replay exposed that visit grouping could hide a prior completion on the same conversation surface. Production selection now reserves attributed task-state transition frames before general context. The final replay retains the user goal, agent state, and prior completion while excluding the checklist. | Packet and request checkpoints match |
| `0056` | Real-island visual-cue verification is primary; PFTU side pane has no authority | The replay keeps the owned verification pane primary and preserves the adjacent PFTU pane as non-authoritative. | Packet and request checkpoints match |
| `0e34` | Unsent regression draft is current, not submitted, and does not prove causality | The replay retains the composer draft as `submitted=false`, preserves prior completion context, and keeps the stated cause as a hypothesis rather than a fact. | Packet and request checkpoints match |

The four dedicated semantic tests passed inside the focused semantic suite. The final combined replay then completed all four fixtures and both checkpoints: 1 replay test passed, zero failed, zero ignored, and 882 unrelated tests were filtered out.

## Selected and rejected image reasons

The supplied provider logs were used only to audit the previous request shape. Their model answers were not treated as labels and their private contents were not copied into fixtures.

- `05cd`: select the latest attributed user request and directly related answer state. Retain earlier product discussion only as bounded support. Reject any older same-fact image as a near-duplicate or budget omission.
- `0d1c`: select the short newer visual-cue request and its current agent state. Preserve backend connection as a prior completion boundary. Reject the launch checklist because it is an unrelated detour, regardless of its higher interaction count.
- `0056`: select the owned visual-cue verification pane. The adjacent PFTU pane remains separate and has no primary-task authority. If ownership is ambiguous, do not guess a crop and cap request confidence at 0.55.
- `0e34`: select the current composer draft and enough prior completion context to explain the regression complaint. Mark the draft unsubmitted. Reject language that upgrades its causal theory into a fact.

Every image candidate now records whether it was selected, its role, task-turn relation, pane ambiguity, duplicate group, original and prepared dimensions, crop policy, redaction status, and a typed selection or rejection reason. Rejection reasons include privacy blocking, missing readable image, unrelated task evidence, same-task near-duplicate, and bounded-budget omission.

## Image preparation and size proof

The request limits are two boundaries, four images, 12 task-relevant spans, 24 KiB of structured text, and an estimated 6,144 input text tokens. The long edge of a transported image is capped at 1,600 pixels.

The deterministic dimension test proves that a 3,104 by 1,962 image becomes 1,600 by 1,011. It also proves that a crop is accepted only when geometry is valid, the scope is owned, ownership confidence is at least 0.75, and no cross-pane ambiguity exists. An unreliable crop is removed rather than guessed; the full image is then bounded when available, and semantic confidence is capped at 0.55.

The old provider-log measurements were:

| Case | Images | Structured text characters | Provider input tokens |
| --- | ---: | ---: | ---: |
| `05cd` | 3 | 7,083 | 6,758 |
| `0d1c` | 3 | 9,063 | 7,377 |
| `0056` | 2 | 6,826 | 7,796 |
| `0e34` | 3 | 6,705 | 10,583 |

Those logs redact image payloads and do not retain original or sent dimensions. Dimension proof therefore comes from production code and deterministic tests, not a reconstructed claim about historical uploads.

## One-request, privacy, cutoff, and background-upload proof

- The normal manual path invokes the compact semantic probe once after the manual cutoff.
- `MANUAL_PROVIDER_RETRIES` remains `0`.
- The request checkpoint reports a provider-post ceiling of `1`, reconciliation posts `0`, HTTP retries `0`, and `transport_executed=false` for deterministic replay.
- No raw typed characters, clipboard text, private screenshot bytes, or supplied provider-log transcripts were added to fixtures or SQLite audit rows.
- Private or model-ineligible images are rejected before transport.
- Hostname occurrence is transported only as `hostname_mentioned_in_current_surface`; the former carry inference is false and hostname mention has no semantic authority.
- No background upload path was added or enabled.

## Automated verification

| Command or broader current equivalent | Final authoritative result |
| --- | --- |
| `cargo fmt --all -- --check` | Succeeded after the final production and fixture changes. |
| `cargo check --tests` | Succeeded after the multi-agent implementation merged. |
| `cargo check` | Succeeded after the final task-state image reservation change in 17.94 seconds. |
| `cargo test semantic_probe --lib` | Focused run: 61 passed, zero failed, one ignored. The post-fix broad suite included these tests and remained green. |
| `cargo test observation_packet --lib` | The post-fix `cargo test continuation --lib` superset included all observation-packet tests and reported zero failures. |
| `cargo test task_turn_evidence --lib` | Focused run: 14 passed, zero failed, zero ignored. The post-fix broad suite included these tests and remained green. |
| `cargo test accuracy --lib` | The post-fix broad suite included the accuracy module and the four-case replay. The exact replay checkpoint also passed independently: 1 passed, zero failed, 882 filtered out. |
| `cargo test continuation --lib -- --format terse` | 602 passed, zero failed, three ignored, 278 filtered out, in 82.91 seconds. |
| `npm run build` current equivalent | The shell wrapper lacked `npm`/`node` path resolution, so the same build stages were invoked directly with bundled Node. TypeScript succeeded and Vite built 34 modules in 1.06 seconds. |
| `git diff --check` | Succeeded after the final production change. |

No live provider request, GUI test, Computer Use test, browser automation, or native-island visual pass was run or claimed.

## Remaining limitations reserved for LCA-02

LCA-02 may rely on the typed role ordering, pane boundary, same-conversation prior-completion preservation, neutral hostname handling, duplicate collapse, bounded image preparation audit, and one-request limits documented here.

LCA-01 intentionally does not redesign the public actionable-continuation answer, loosen semantic admission, add reconciliation, change target eligibility, or alter first-screen presentation. Those remain LCA-02 or later responsibilities.

## Manual testing ownership

Live app behavior, real provider output, native-island rendering, mixed-pane screenshot quality, crop readability, and the final five-phase visual acceptance remain user-owned manual tests.

PASS — LCA-01 evidence packet is proven and LCA-02 may begin
