# Task Truth v2.02 corpus

This namespace is additive. It does not change the P6 fixture, metric, or release-report meanings.

`session-013-family.v2.json` contains five separately timed, privacy-safe, live-redacted decision boundaries. The source section contains production-shaped evidence only. Recorded product output is separate, and independently blinded human adjudication remains pending. The corpus therefore reports missing human evidence and cannot pass the release gate.

The private builder reads only inputs under the gitignored `private_task_truth_corpus/` root. Dry-run is the default workflow. It retains an allowlisted structural subset, including geometry and record relationships, rejects sensitive text and unknown fields, and emits the retained review candidate beside a content-hashed privacy manifest. It never calls copied private text synthetic. The candidate remains `pending`; generating it is not approval to commit a fixture.

Application, layout, and workflow buckets are the partition unit. A bucket cannot cross development, validation, and locked holdout partitions.

Locked holdouts live only in `locked-holdout.v2.json`. Normal evaluation never opens or deserializes that file; explicit holdout access is required. Release metrics use only live-redacted cases with approved privacy review, independently blinded labels, and complete adjudication. Synthetic counterfactual metrics are reported separately.

The frozen policy stores a definition, direction, and numeric threshold for every TT2-02 metric. Missing labels or an unavailable path produce no denominator and fail the threshold gate; they never become a pass.

Path meanings:

- `path_a_legacy_p6`: privacy-safe snapshot of recorded historical production output. It is missing when no authentic snapshot exists.
- `path_b_causally_repaired`: a fresh replay through the current production pipeline on the same redacted source.
- `path_c_task_truth_shadow`: Task Truth v2 multimodal-shadow semantics followed by the local evidence verifier. Manual cases use deterministic fixture responses; background boundaries record `not_requested_background`, because live image inference is allowed only for explicit Continue. This path remains non-authoritative until TT2-05.

Live provider smoke tests are opt-in. Set `SMALLTALK_TASK_TRUTH_MULTIMODAL_ENABLED=true` only in a private local environment with `OPENAI_API_KEY` configured safely, then trigger Continue manually. The request uses active-window image crops, never background/private frames, and persists only image-handle hashes and size/count metadata.

Generate and inspect the baseline report:

```bash
cd src-tauri
cargo run --bin task_truth_v2_eval -- \
  --output tests/fixtures/continue_accuracy/task_truth_v2/baseline-report.v1.json
```

Keep that baseline file frozen before anyone opens the locked holdout. Generate the release evaluator as a separate artifact; this is the only evaluator input allowed to include the explicitly unlocked holdout:

```bash
cd src-tauri
cargo run --bin task_truth_v2_eval -- \
  --allow-locked-holdout \
  --output tests/fixtures/continue_accuracy/task_truth_v2/release-evaluator-report.v1.json
```

Generate the final release verdict from the separate release evaluator and frozen baseline:

```bash
cd src-tauri
cargo run --bin task_truth_v2_release_gate
```

The final gate also requires `manual-macos-qa.v1.json` with all 14 reviewed scenarios, a separately frozen `release-budgets.v1.json`, and `performance-cost-privacy.v1.json` with measured values, at least 30 performance samples, and zero privacy violations, unsafe opens, secret findings, and background multimodal requests. Their committed shapes are defined by `manual-macos-qa.schema.v1.json`, `release-budgets.schema.v1.json`, and `performance-cost-privacy.schema.v1.json`. The budget policy must contain the exact `sha256:` fingerprint printed for the frozen pre-holdout baseline; a policy bound to different baseline bytes is rejected. It also cannot exceed the architectural four-image, 12 MiB request, or 500-checkpoint retention caps. Missing, partial, duplicated, or self-declared-passed manifests are explicit violations.

The v2 evaluator calculates all TT2-05 hard metrics separately from the older frozen metric map. This includes next-action precision and coverage as distinct denominators, per-required-surface wrong-task rates, critical model-on/off unexplained disagreement, and Wilson 95% confidence intervals. The final generator revalidates every metric, interval, corpus minimum, slice, surface, and manifest instead of trusting an upstream `passed` boolean.

The generated `final-release-report.v1.json` is the only Task Truth v2 authority input. Runtime validation independently checks its schema, policy, corpus and holdout counts, all 13 hard semantic metrics, ten surface-family gates, required slices, manual scenarios, performance/privacy results, confidence intervals, and zero-tolerance counts. A green unit test, baseline replay, or hand-authored `passed: true` cannot replace those proofs.
