# Continue Architecture

Smalltalk Continue is the native desktop continuation engine. It does not require Stop Session and it does not use the browser extension.

## Pipeline

1. Local evidence is captured into the SQLite store.
2. `rebuild_continue_second_layer` resolves stable artifacts and task actions.
3. `rebuild_continue_third_layer` builds episodes, workstreams, durable artifact roles, and unresolved state.
4. `get_continue_decision` generates local continuation candidates and scores them.
5. Optional bounded OpenAI micro-inference may choose among those local candidate ids and phrase a cautious next action.
6. Local validation accepts the model result only if ids, workstream, target, evidence quality, and confidence remain compatible with the local candidate pack.

`get_native_resume_card` is legacy local resume-card behavior. `run_cloud_resume` is the Stop/resume-query bundle path. Neither is the core Continue engine.

## Invoke Local Continue

```ts
await invoke("get_continue_decision", {
  input: {
    lookback_ms: 2700000,
    rebuild_layers: true
  }
});
```

## Default Bounded OpenAI Micro-Inference

Normal Continue requests use bounded OpenAI micro-inference by default. Callers can still pass `micro_inference_enabled: false` for local-only diagnostics.

```ts
await invoke("get_continue_decision", {
  input: {
    lookback_ms: 2700000,
    rebuild_layers: true,
    micro_inference_enabled: true,
    max_candidates_for_model: 5
  }
});
```

The backend reads `OPENAI_API_KEY` from the process environment or project `.env`. The model defaults to `gpt-4.1-mini`, with `SMALLTALK_CONTINUE_OPENAI_MODEL`, `SMALLTALK_OPENAI_MODEL`, or `OPENAI_MODEL` as overrides.

The OpenAI request uses the Responses API with Structured Outputs. The model receives a compact candidate pack only: current focus facts, top workstreams, top candidate ids, local score components, artifact roles, short local reasons, missing evidence notes, and manual breadcrumbs. It does not receive raw screenshots, raw timelines, raw typed characters, full clipboard text, raw paths, or raw URLs.

## Validation

The model output is rejected and persisted as `local_fallback` when:

- the selected candidate id was not supplied;
- the selected workstream id does not match the selected candidate;
- the output mentions unsupported URLs or paths;
- `next_action` is empty, too long, or incompatible with an evidence-only candidate;
- high confidence is returned for thin local evidence;
- a branch/support target is promoted without a strong local candidate.

## Feedback And Breadcrumbs

`infer_continue_feedback` infers `accepted`, `rejected`, `ignored`, `corrected`, or `auto_resumed` from post-decision local observations. `get_continue_decision` also checks pending prior decisions before making a new one.

`add_continue_breadcrumb` stores a short local-only note on a workstream. Breadcrumbs can be included in later candidate packs, but there is no product UI for them yet.

## Eval

Run the default fixture set:

```ts
await invoke("run_continue_eval", { evalFilePath: null });
```

Run a custom fixture:

```ts
await invoke("run_continue_eval", {
  evalFilePath: "/absolute/path/to/continue-eval.json"
});
```

The eval report includes target artifact correctness, Recall@k, MRR, current-focus false-positive rate, hallucinated artifact count, and model validation fallback rate.
