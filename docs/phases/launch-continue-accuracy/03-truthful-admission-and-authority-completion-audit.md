# LCA-03 Truthful Admission and Authority — Completion Audit

Date: 2026-07-19

Contract: `03-truthful-admission-and-authority.md`

## 1. Before and after

Before LCA-03, a structurally valid cloud answer could become public without a separate, field-level admission decision. Legacy paths could also turn weak or insufficient model output into a stronger public status. Target readiness and semantic truth were not fully separated, and the persisted decision did not contain the complete admitted public contract.

After LCA-03:

- The model remains the only semantic author. Local code validates and admits or rejects model fields; it does not synthesize replacement task meaning.
- Raw model status and admitted public status are separate and monotonic. Admission can preserve or weaken a status, never strengthen it.
- Task, resume point, action, completion, location, and return target receive independent admission verdicts.
- Semantic truth and target readiness are separate. A useful answer can remain public without a clickable target.
- Explicit user-goal support is accepted only when it is packet-attributed and submitted to the model.
- Public answers carry a versioned atomic identity and the exact admitted contract is persisted.
- Corrections invalidate only the affected field and any dependent target authority.

## 2. Field support matrix

| Field | Required support | Local admission behavior | Failure scope |
| --- | --- | --- | --- |
| `unfinished_task` | Attributed submitted user goal, or packet evidence with a task-evidence role and valid chronology | Accept exact model wording when supported; never infer a replacement task | Reject task field; preserve separately supported diagnostic fields; suppress direct target |
| `where_to_resume` | Eligible, non-private submitted evidence identifying a real resume surface | Accept when the cited surface supports the location | Reject location and direct target; task meaning may remain |
| `next_action` | Submitted evidence supporting the stated action and valid task state | Accept model-authored action only | Reject action and weaken admitted status to partly resolved |
| `completion_state` | Submitted evidence supporting terminal or non-terminal state | Accept only a state consistent with the task chronology | Reject completion field; completed same-task surfaces cannot become the primary continuation |
| `target_identity` | Eligible exact decision plus matching current frame and evidence identity | Attach only after semantic admission | Suppress target without rewriting admitted semantics |
| explicit user goal | P6 attribution proving the goal was actually submitted | Mark source `explicit` | Otherwise source is `inferred`; no explicit-authority shortcut |

Context-image identity alone is physical evidence identity. It is not semantic authority for an unfinished task.

## 3. Explicit versus inferred source proof

`semantic_probe` now records `explicit_goal_support_slots` from P6 attribution. Only slots whose submitted evidence is attributed as `LatestUserGoal` can establish an explicit source. Other admitted model answers are marked `inferred`.

This prevents a local artifact name, OCR fragment, context image, or legacy packet field from being relabeled as an explicit user instruction.

## 4. Semantic admission versus target readiness

The public answer now exposes both:

- `admitted_semantic_status`: whether the model-authored meaning is safely publishable.
- `target_status`: whether a direct return target is safely attachable now.

The target states include preview-only, ready, stale-decision, suppressed, unknown, and no-task outcomes. Target attachment is performed after semantic admission. A stale or missing target therefore removes the action affordance without erasing a supported continuation answer.

## 5. Four-case deterministic replay

The LCA replay fixture contract was updated so no fixture claims `resolved` when the raw model result is only `partly_resolved`.

| Case | Raw status | Admitted status | Source | Target | Result |
| --- | --- | --- | --- | --- | --- |
| explicit user goal | partly resolved | partly resolved | explicit | frame preview only | pass |
| inferred unfinished task | partly resolved | partly resolved | inferred | frame preview only | pass |
| supported current work | partly resolved | partly resolved | explicit | frame preview only | pass |
| supported return point | partly resolved | partly resolved | explicit | frame preview only | pass |

The focused `lca_` Rust filter passed all 11 tests, including all four replay cases.

## 6. False-positive and false-negative findings

False-positive protections proven:

- Raw `insufficient_evidence` cannot be upgraded locally to `resolved`.
- Passive context-image identity cannot create semantic task authority.
- Inline support tokens are rejected rather than exposed as user-facing meaning.
- Private, stale, wrong-surface, contradictory, generic, overlong, chronology-invalid, and state-invalid support receive typed rejection reasons.
- A completed terminal same-task surface cannot remain the eligible primary continuation.
- Invalid authority at the React or native island boundary clears all semantic and target fields.

False-negative protection proven:

- Rejecting one field does not automatically erase every other independently supported field.
- A target failure does not erase the admitted semantic answer.
- Refused and unresolved model outcomes remain truthful terminal outcomes rather than being treated as malformed transport.

## 7. Atomic identity, cache, correction, and target safety

The atomic answer identity contains:

- decision id and current frame id;
- packet policy, response schema, and admission versions;
- admitted result id;
- correction watermark;
- target identity.

The admitted result id is derived from the packet/evidence/output/admission contract and not from a newly allocated request or decision id. Reuse can therefore identify the same admitted result while target attachment still requires the current decision identity.

Persisted decision rows now include the complete admitted public answer JSON plus the identity and target columns. Review output exposes the same admission, source, reason, conflict, and identity facts.

LCA-05 completed the cache-retention proof without weakening fresh manual-decision identity. A new manual Continue still receives a fresh decision id and revalidates its target. The already admitted provider result is reused without another provider post only when the session, packet id, evidence watermark, current frame, model, deterministic request identity, schema versions, verifier/admission versions, and live correction watermark all match. The new row records the source decision id and `provider_post_count = 0`. Old cache rows without this complete identity are not reused.

The cache proof passed both directions:

- `repeated_manual_decision_reuses_unchanged_admitted_result_without_provider_post` — unchanged evidence preserved the admitted result and performed zero additional posts.
- `correction_change_invalidates_admitted_result_reuse` — changing the correction watermark prevented reuse.

Correction handling is field-scoped:

- task correction rejects task authority and suppresses its dependent target;
- action correction rejects the action and weakens the public status;
- location correction rejects the location and target, while separately supported task meaning remains.

## 8. Typed failure mapping

Public field admission uses typed outcomes:

- `accepted`
- `rejected_unsupported`
- `rejected_stale`
- `rejected_private`
- `rejected_wrong_surface`
- `rejected_chronology`
- `rejected_contradiction`
- `rejected_generic`
- `rejected_overlong`
- `rejected_invalid_state`

The public answer also exposes `unresolved_or_failure_reason` and `semantic_conflicts`. This keeps provider refusal, insufficient evidence, admission rejection, and target suppression inspectable without inventing a semantic fallback.

## 9. Verification record

Passed:

- `cargo fmt --all -- --check`
- `cargo check`
- `cargo test compact_admission --lib`
- `cargo test field_admission_reasons --lib`
- `cargo test lca_ --lib` — 11 passed, 0 failed
- `cargo test verifier --lib` — 20 passed, 0 failed
- `cargo test insufficient_model_status --lib`
- `cargo test semantic_consistency --lib` — 10 passed, 0 failed
- `cargo test session_island --lib` — 52 passed, 0 failed
- `cargo test island_sanitizes_resolved --lib`
- direct bundled TypeScript compiler check
- direct bundled Vite production build — 34 modules transformed
- direct Node webview tests — 32 passed, 0 failed
- exact production public-answer test
- exact legacy-packet non-promotion test
- review tests — 2 passed, 0 failed
- `cargo test repeated_manual_decision_reuses_unchanged_admitted_result_without_provider_post --lib` — 1 passed, 0 failed
- `cargo test correction_change_invalidates_admitted_result_reuse --lib` — 1 passed, 0 failed
- `git diff --check`

Bounded test limitations, following the user's two-attempt rule:

- The broad `production` filter was run twice. The second run had one stale string expectation after 28 passes. That expectation was fixed and the exact affected test then passed; the full filter was not run a third time.
- The broad `semantic_probe` filter was run twice. The second run exposed two tests that encoded the old context-image authority behavior. Both tests were rewritten to assert rejection. The full filter was not run a third time; the LCA-focused 11-test filter passed afterward.
- `npm run build` could not start because the shell did not expose `npm`; the `pnpm` retry likewise lacked `node`. The same bundled TypeScript and Vite tools were run directly and passed.
- The accuracy binary first required the `eval-binaries` feature. The feature-enabled retry compiled but received the cases subdirectory rather than the evaluator root. It was not run a third time. The deterministic Rust replay test passed all four cases.

## 10. LCA-04 limits

LCA-03 does not claim:

- live provider behavior under real credentials;
- visible macOS island approval;
- a soak test across capture churn, app switching, or restarts;
- final latency or release-gate policy;
- end-user calibration of the new status and rejection copy.

Those remain later-phase or manual acceptance work. LCA-03 establishes the truthful authority boundary that those checks can rely on.

## 11. User-owned manual verification

The following checks remain explicitly user-owned:

1. Run the normal `npm run tauri dev` application path with real provider access.
2. Exercise one explicit goal, one inferred task, one stale target, one correction, and one provider refusal.
3. Confirm the island shows the admitted model answer as-is, never exposes support tokens, and removes only the unsafe target/action.
4. Inspect the persisted decision and review artifact for matching result id, versions, field verdicts, target status, and correction watermark.

PASS — LCA-03 admission and authority are proven and LCA-04 may begin

## 2026-07-19 correction from LCA-06 live runtime recovery

Live use disproved two production-path assumptions behind the earlier pass. Target ownership mismatch could still become `stale_decision`, erasing separately supported semantics. The provider could also be called when no semantic field could pass the same validator used after the response.

LCA-06 makes target mismatch suppress only the target and open action. It computes and audits admission feasibility before transport; no useful admissible field produces a typed task-evidence acquisition failure with `provider_post_count = 0`. The original LCA-03 component proof remains useful, but its PASS line is not a current live-runtime or product-pass claim.
