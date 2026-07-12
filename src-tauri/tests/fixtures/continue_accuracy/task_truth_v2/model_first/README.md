# MFTI-04 release evidence

This directory is a new model-first release namespace. The frozen Task Truth v2 policy and reports in the parent directory remain historical inputs and are not redefined.

`eval-policy.v1.json` was frozen before any locked holdout exists or was inspected. It replaces model-on/off semantic parity with the model-first failure contract: provider unavailability must produce 100% honest unresolved answers and zero locally invented semantic tasks.

Generate an evaluator report without editing the historical TT2 report:

```bash
cargo run --features eval-binaries --bin task_truth_v2_eval -- \
  --fixtures tests/fixtures/continue_accuracy/task_truth_v2 \
  --output tests/fixtures/continue_accuracy/task_truth_v2/model_first/release-evaluator-report.v1.json
```

Then generate the MFTI verdict:

```bash
cargo run --features eval-binaries --bin mfti_04_release_gate
```

The gate also requires `performance-cost-privacy.v1.json`, matching the adjacent schema, with at least 30 measured manual Continue runs. It records the capture, request-build, provider, verification/persistence, and total latency distributions; image, byte, token, and cost distributions; provider failures; second-pass behavior; privacy exclusions; and the reviewed provider-failure experience.

Each completed manual Continue writes one row to `task_truth_v2_performance_samples`. That table is deliberately limited to numeric measurements and bounded outcome labels. It cannot store raw OCR, Accessibility text, model hypotheses, image paths, URLs, or provider responses. Generate the aggregate from the live capture database with:

```bash
cargo run --features eval-binaries --bin mfti_04_performance_report -- \
  --database /path/to/capture.sqlite \
  --output tests/fixtures/continue_accuracy/task_truth_v2/model_first/performance-cost-privacy.v1.json \
  --monthly-continues 600 \
  --privacy-violations 0 \
  --unsafe-opens 0 \
  --provider-failure-experience-reviewed
```

`--monthly-continues` is the declared usage assumption behind expected monthly cost. The two reviewed counts and the provider-failure flag must come from the human release review; the generator does not infer or silently claim them. Input tokens use provider usage when available and the bounded request estimate otherwise. Output tokens use provider usage, or zero when no provider response exists. Cost uses the frozen audit estimate of $0.50 per million input tokens and $2.00 per million output tokens. A report with fewer than 30 complete rows is provisional and cannot satisfy the release schema or gate.

The verdict must remain `passed: false` while reviewed corpus, holdout, release identity, or manual QA evidence is absent. Never create those manifests by copying product output into the human labels. Human ground truth must be written independently before the reviewer sees Smalltalk's answer.
