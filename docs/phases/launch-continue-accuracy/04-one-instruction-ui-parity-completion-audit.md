# LCA-04 One-Instruction Product Answer and React/Island Parity — Completion Audit

Date: 2026-07-19

Contract: `04-one-instruction-ui-parity.md`

## 1. Dependency gate

The prerequisite audits end with the exact required verdicts:

- `PASS — LCA-01 evidence packet is proven and LCA-02 may begin`
- `PASS — LCA-02 actionable contract is proven and LCA-03 may begin`
- `PASS — LCA-03 admission and authority are proven and LCA-04 may begin`

The live backend public answer independently exposes admitted unfinished task, task state, resume point, supported action, completed context, semantic status, target status, and atomic identity.

## 2. Before and after presentation flow

Before LCA-04, React could construct a headline from `next_action || unfinished_state`, combine progress and unfinished state, and render recent trail or target diagnostics on the first screen. The island used `task_summary` as its title and expanded into raw task, activity, relationship, progress, unfinished-state, action, and location rows. The two surfaces could therefore disagree while reading the same answer.

After LCA-04, the backend owns one typed `smalltalk.continue_product_projection.v1` value. Both surfaces render its exact instruction, resume context, optional location, state, and action meaning. React does not reconstruct the answer from compatibility fields. Swift does not translate raw semantic fields into native-only prose.

The visible order is now:

1. Canonical primary instruction.
2. One canonical resume-context sentence.
3. Optional evidence-backed location context.
4. One typed primary action and, when available, `Inspect`.

Recent trail, field admission, confidence, identities, support details, raw semantic fields, and evidence diagnostics are not part of the canonical first-screen branch.

## 3. Canonical projection contract

| Field | Meaning |
| --- | --- |
| `answer_identity` | Atomic admitted answer identity saved with the presentation. |
| `presentation_state` | `action_known`, `task_known_action_unknown`, `task_unknown`, typed provider/parser/validation failure, or `stale_decision`. |
| `primary_instruction` | Admitted `next_supported_action` only. It never falls back to `unfinished_state`. |
| `resume_context` | One bounded resume-point sentence, with completed context only when needed to prevent temporal confusion. |
| `location_context` | Optional evidence-backed orientation, without implying openability. |
| `semantic_status` | Admitted semantic status. |
| `task_state` | Admitted lifecycle state. |
| `target_status` | Direct, preview-only, unknown, suppressed, or stale target state. |
| `primary_action` | `open_direct_target`, `inspect_evidence`, `refresh_continue`, or `none`, with one canonical label. |
| `inspect_available` | Whether aligned evidence or diagnostics can be inspected. |
| `unresolved_reason` | Typed unresolved or failure reason when applicable. |

Instruction and resume-context limits are deterministic Unicode character limits of 160 and 180 characters. Truncation includes a final ellipsis inside the limit.

The projection is recomputed after admission, correction, forced unresolved sanitization, direct-target attachment, and stale-decision transitions. Old serialized answers deserialize through conservative defaults.

## 4. React/native field and enum map

| Canonical meaning | React | Native island |
| --- | --- | --- |
| Instruction | `product_projection.primary_instruction` | `productProjection.primaryInstruction` |
| Resume state | `resume_context` | `resumeContext` |
| Location | `location_context` | `locationContext` |
| Semantic/task/target state | Projection fields, diagnostic only outside the hero | Same decoded fields; no native reinterpretation |
| Direct action | Existing strict decision-bound open path | Enabled `OpenContinueTarget` action with decision ID |
| Evidence preview | `View last screen` / Inspect | Typed `InspectEvidence` action; visual cue remains answer-linked |
| Stale | Refresh only; direct open disabled | Refresh only; remembered nested projection is recomputed |
| No action | No manufactured primary button | No manufactured primary button |

Current and legacy visit roles share one explicit compatibility map:

- `primary_work`, `continuation`, `return_to_prior_task` → Primary work
- `supporting_work`, `supporting_research`, `verification` → Supporting work
- `detour_or_unrelated`, `temporary_detour`, `interruption`, `new_task` → Detour or unrelated
- `unclear`, `unrelated_or_unknown`, and unknown values → Relationship unclear

Known `primary_work` is covered by deterministic tests and cannot fall through to unclear.

## 5. Four critical fixture presentation snapshots

### 05cd

```text
Continue reviewing the answer about whether the product solves a real need.

The answer has begun and continues beyond the visible section.

[View last screen] [Inspect]
```

### 0d1c

```text
Return to the Codex visual-cue task and inspect its implementation result.

The backend connection was already complete; the newer visual-cue request was still active.

[View last screen] [Inspect]
```

### 0056

```text
Test the new answer-linked visual cue in Smalltalk.

Implementation passed its checks; user verification remains.

[View last screen] [Inspect]
```

The first-screen test excludes the PFTU release gate.

### 0e34

```text
Return to the drafted regression report and continue the investigation.

The latest Continue result was rejected as insufficient evidence; the cause is not yet proven.

[View last screen] [Inspect]
```

The first-screen test does not assert that the visual-cue change caused the regression.

## 6. Direct, inspect, stale, and failure policy

| State | Instruction | Primary action | Safety rule |
| --- | --- | --- | --- |
| Action known, direct target ready | Admitted supported action | `Continue here` | Requires existing strict target eligibility and decision ID. |
| Action known, target unavailable, frame aligned | Same admitted supported action | `View last screen` | Frame remains evidence, never a return target. |
| Task known, action unknown | `I found the task, but not a safe next step.` | `Inspect` or none | No action is synthesized from task or unfinished state. |
| Task unknown | Honest task abstention | `Try Continue again` or Inspect | Observed activity cannot become task truth. |
| Provider/parser/validation failure | Failure-specific copy | Refresh, Inspect, or none | Failure is not mislabeled as insufficient evidence. |
| Stale decision | `The saved answer is older than the latest work.` | `Refresh Continue` | All direct-open affordances are removed. |

## 7. Inspect boundary and preserved behavior

React's canonical hero source test proves that first-screen copy does not reference support slots, confidence, snapshot IDs, frame IDs, recent trail, `unfinished_state`, or legacy `next_action`. Those details remain in `ContinueEvidencePanel`.

The native island now renders only canonical resume and location rows around the instruction. Its source-contract test rejects the old ten-row diagnostic answer shape.

The following behavior remains intact and is covered by deterministic tests or unchanged routing:

- compact/resting dimensions and content-driven width;
- hover, expansion, and existing motion behavior;
- answer-linked visual cue with same-session, normal-privacy, capture-root validation;
- visual-cue load nonce and decision/path stale guards;
- read-only, one-panel output history and request-ID latching;
- capture status and Continue request routing;
- strict decision-ID target opening;
- privacy redaction and no legacy open bypass.

History stores immutable canonical instruction, resume context, location, action label, and answer identity. Existing saved rows remain readable; history selection cannot open a target or mutate the live answer.

## 8. Automated verification record

Passed:

- `cargo fmt --all -- --check`
- `cargo check --message-format short`
- `cargo test product_projection --lib` — 4 passed, 0 failed
- `cargo test production --lib` — 33 passed, 0 failed
- `cargo test session_island --lib` — 53 passed, 0 failed
- `cargo test history --lib` — 19 passed, 0 failed
- `cargo test continuation --lib` — 618 passed, 0 failed, 3 ignored live/provider tests
- `cargo test lca_ --lib` — 11 passed, 0 failed
- exact remembered-answer stale regression — 1 passed, 0 failed
- `npm run build` — TypeScript and Vite production build passed; 34 modules transformed
- `npm run test:webview` — 36 passed, 0 failed
- `swiftc -typecheck src-tauri/macos/SessionIslandPanel.swift` — passed with existing macOS 14 `onChange` deprecation warnings
- focused Swift source-contract test — 1 passed, 0 failed
- `git diff --check`

Bounded corrections under the user's two-attempt rule:

- The first final formatting check found only line wrapping in the new stale-state test. `cargo fmt` fixed it; the final check passed.
- The first broad island run passed 51 of 53 tests and exposed two old copy expectations. They were updated to the canonical validation and role labels. The second run passed all 53 tests.
- The plain npm wrapper initially lacked `node` on the shell path. The bundled Node runtime was placed on `PATH`; the actual build and webview commands then passed.
- The final legacy-role compatibility expansion exposed one four-value React lookup during TypeScript compilation. React was switched to the same shared role-label helper, and the second build passed.

No Computer Use, browser automation, screenshots, GUI launch, mouse input, keyboard automation, or live visual acceptance was performed.

## 9. User-owned manual script

Run these only after LCA-05. Every scenario remains intentionally unclaimed here.

| Scenario | What to verify | Status |
| --- | --- | --- |
| Compact first line | The island leads with the canonical actionable instruction. | Not run — user-owned |
| Show more hierarchy | Instruction remains first; resume context follows; diagnostics do not replace it. | Not run — user-owned |
| Visual cue alignment | Visual cue belongs to the exact saved answer and does not change after later capture. | Not run — user-owned |
| View versus Continue | Frame preview says `View last screen`; only a strict direct target says `Continue here`. | Not run — user-owned |
| React/native parity | Same answer identity shows the same instruction, context, states, and action meaning. | Not run — user-owned |
| History preservation | Saved canonical copy and identity remain immutable and read-only. | Not run — user-owned |
| Stale and failure copy | New evidence produces refresh-only stale copy; provider/parser/validation failures remain distinct. | Not run — user-owned |
| Keyboard/accessibility | Buttons, labels, focus, Show more/less, Inspect, history, and visual cue remain accurate. | Not run — user-owned |

## 10. Limits reserved for LCA-05

LCA-04 proves deterministic presentation and parity. It does not claim:

- live replay of the four supplied logs through the normal application path;
- real provider behavior, latency, or cost;
- visible macOS approval of compact and expanded geometry;
- final launch-gate corpus, holdout, calibration, or manual evidence;
- long-running capture churn, restart, and stale-history soak behavior.

LCA-05 must replay the four logs end to end, confirm the canonical projection is produced from the real admitted outputs, complete the required launch-gate measurements, and leave all larger release claims truthful.

PASS — LCA-04 product answer and parity are proven and LCA-05 may begin

## 2026-07-19 correction from LCA-06 live runtime recovery

The earlier stale-state proof used constructed stale inputs. Live runtime behavior later showed that target failure could manufacture that stale input for a fresh answer. React also derived freshness from ambient counters while the native island had its own timestamp/count comparison.

LCA-06 makes the backend material evidence watermark the shared freshness identity. Raw event, signal, frame-count, and timestamp churn do not by themselves mark an answer stale. React stores the decision's exact material watermark, the native island consumes backend `decision_stale`, and a true material advance disables open and shows Refresh. The original LCA-04 tests remain useful, but its PASS line is not live parity approval.
