# Task Truth v2.05 — Make Task Truth Authoritative And Pass The Final Release Gate

## Codex task

Integrate verified TaskSnapshots into the production Continue path, make target selection subordinate to task truth, simplify the first-screen answer, and prove on a locked live corpus that vague or wrong outputs have been solved to the declared release standard.

This is the only Task Truth v2 goal allowed to change production semantic authority. It may do so only after completing the requirement-by-requirement release audit below.

Architecture completion, green unit tests, or a good session-013 demo are not enough.

## Dependency gate

Task Truth v2.01 through v2.04 must be complete and verified. Read every completion report and inspect the current worktree.

Required inputs:

```text
v2.02 frozen evaluation policy
current corpus and review manifests
locked holdout access policy
three-path shadow report
v2 TaskSnapshot/ObservationPacket schemas
multimodal resolver and verifier metrics
latency/cost/privacy results
session-013 regression results
```

If the corpus or independent human labels are below the frozen minimum, continue building evidence and keep v2 shadow-only. Do not weaken the gate to finish the phase.

## Product contract

Continue must answer, in this order:

```text
You were — precise task and work object
State — last meaningful progress and unfinished/waiting/blocked state
Next — one supported next action, or omitted
Where — app plus document/thread/page identity at the supported precision
Action — Continue here only when a strict direct target exists
```

If task understanding is strong but opening is unavailable, say so. If two tasks are genuinely close, present two bounded choices. If task truth is insufficient, show a precise ambiguity state with inspectable evidence.

Never output a fluent generic headline as a substitute for task understanding.

## Goal 1 — Add an explicit authority and rollout policy

Use a versioned feature/authority state:

```text
off
shadow
eligible
authoritative
rollback
```

Production authority must be decided by policy, not by whether a model happened to return JSON.

Requirements:

- keep old P6 available for diagnostic comparison and emergency rollback;
- never silently mix fields from different task identities;
- if v2 fails, use the causally repaired local result or a typed ambiguity state, not unsafe legacy historical fallback;
- persist which path supplied task truth, wording, and target selection;
- invalidate caches when resolver/schema/policy/evidence watermark changes;
- preserve deterministic decision-id and island handoff rules;
- record every authority switch in audit.

## Goal 2 — Require a selected TaskSnapshot before task-shaped candidates

Reorder production composition:

```text
verified TaskSnapshot selection
→ associate matching workstream/open loop/support evidence
→ resolve anchors belonging to that snapshot
→ strict-open validation
→ answer composition
```

Candidate/workstream/open-loop data may enrich a selected snapshot. They may not manufacture or replace its task.

Enforce:

- unrelated open loops cannot supply the objective;
- an older openable browser tab cannot beat a fresh native task;
- support/detour surfaces stay subordinate;
- return-target score cannot raise task identity confidence;
- target mismatch nulls the target rather than rewriting the task;
- high task confidence may coexist with `return_target = null`.

## Goal 3 — Map TaskSnapshot into a clean public answer contract

Create/version a public contract derived from exactly one verified snapshot revision. Include:

```text
task_resolution_status
task_summary
task_object
last_meaningful_progress
unfinished_state
next_action
where_summary
alternative_hypotheses
direct_return_target or null
evidence_preview
field confidence/support status
task_understanding_source
wording_source
target_selection_source
snapshot id/revision/evidence watermark
```

Do not expose internal candidate terminology on the primary screen.

Remove generic production strings equivalent to:

```text
The agent is working on the current task
Continue working on the current item
Review the latest activity
```

unless they appear only in diagnostics or migration tests as forbidden examples.

## Goal 4 — Redesign the React card and native island around the answer

The first screen must remain one continuation answer. Render:

```text
You were
[task + object]

State
[last progress + unfinished/waiting/blocked state]

Next
[supported action, when present]

Where
[supported identity]

[Continue here] or [Return location unavailable]
```

For two close hypotheses, show two concise choices with `Not right` feedback. For unresolved truth, state that exact task recovery is unavailable and offer evidence inspection.

Do not lead with current focus, detours, provenance badges, confidence jargon, or evidence strength. Put diagnostics under `Why this answer?`.

React and native island must consume the same semantic contract. Island refresh cannot bypass the authority policy or strict-open check.

## Goal 5 — Finish quality-dominant adoption and provenance

Use the v2.01 adoption policy everywhere decisions arrive:

- manual desktop invocation;
- background refresh;
- startup refresh;
- island refresh/event;
- cached result restoration.

No stronger manual result may be downgraded by a weaker background/local result. Add telemetry for adoption accepted/rejected and reason codes.

Show user-facing provenance only when it helps. Internally preserve:

```text
task_understanding_source = local_causal | multimodal_model | human_correction | unresolved
wording_source = deterministic | model_assisted | fallback
target_selection_source = strict_local_policy
```

Remove the ambiguous primary-card `AI-assisted` badge if it conflates these sources.

## Goal 6 — Close the feedback loop without poisoning task truth

`Not right` feedback must attach to the exact snapshot revision and affected field/hypothesis. It may:

- suppress that hypothesis for the scoped evidence/task;
- record a user-selected alternative;
- request fresh evidence;
- improve later evaluation labels after explicit review.

It must not globally promote an unrelated branch, URL, workstream, or old open loop. Inferred navigation remains weaker than explicit correction.

User feedback data is not automatically an independently reviewed gold label.

## Goal 7 — Run the locked release evaluation

Use the frozen v2.02 definitions. The following are hard gates:

| Metric | Release requirement |
| --- | ---: |
| Wrong primary-task rate | at most 3% overall and at most 5% in every required surface family |
| Control/navigation text used as task | 0 cases |
| Useful, non-generic task summary | at least 90% |
| Task-object accuracy | at least 88% |
| Execution-state accuracy | at least 90% |
| Supported next-action precision | at least 90% |
| Supported next-action coverage where labeled | at least 85% |
| Return-target precision | at least 98% |
| Unsupported specific claim rate | at most 1% |
| Stronger manual result downgraded by background | 0 cases |
| Unseen-application useful-summary rate | at least 80% |
| Human `immediately useful` rating | at least 85% |
| Model-on/off task disagreement on critical local-solvable cases | 0 unexplained cases |
| Privacy violations | 0 cases |
| Unsafe opens | 0 cases |

Corpus minimums remain:

```text
at least 200 independently reviewed live decision boundaries
at least 50 locked application-level holdout cases
at least 10 surface families
all required slice minimums from v2.02
```

Report macro, worst-slice, and confidence intervals where meaningful. Every zero-tolerance failure blocks release.

Do not count generic abstentions as useful summaries. Report precise abstention separately.

## Goal 8 — Meet performance, cost, and privacy gates

Freeze and verify:

- p50/p95 manual Continue latency;
- local packet/checkpoint overhead;
- model request image/byte/token bounds;
- maximum semantic checkpoint write rate and retention;
- provider failure rate and safe fallback rate;
- no background multimodal requests;
- privacy-blocked frames excluded before transport;
- no secrets/private artifacts in committed fixtures or audit bundles.

Choose explicit budgets based on the v2.02 baseline before holdout access. If multimodal p95 is too slow, optimize keyframe selection/request size or expose honest progress; do not silently return a vague local answer and label it equivalent.

## Goal 9 — Complete manual interruption-recovery QA

Run real macOS QA, not only component tests:

1. Ask a new question after an older completed chat task; interrupt and Continue.
2. Type and submit in a chat surface with weak AX roles.
3. Edit code, run a command, observe an error, switch away, and Continue.
4. Edit a document/spreadsheet, switch windows, and Continue.
5. Search/research across tabs with an older openable distraction.
6. Wait for agent/application output, interrupt, and Continue.
7. Use a custom-rendered or thin-accessibility application.
8. Trigger privacy blocking.
9. Produce two genuinely close tasks and verify choice UI.
10. Verify task understood but no target available.
11. Verify direct target opens only the selected task.
12. Verify a weaker background refresh cannot replace the manual answer.
13. Verify main card and island agree on task, state, next, where, and opening.
14. Verify `Not right` stays scoped.

Record evidence ids, expected result, actual result, app build/commit, and reviewer. Do not commit personal screenshots.

## Required verification commands

Run narrow tests during implementation, then at minimum:

```text
cargo test continuation::task_truth_v2
cargo test continuation::accuracy_eval
cargo test continuation::task_turn
cargo test continuation::activity_recap_validation
cargo test session_island
cargo test
cargo check
cargo fmt --check
npm run test:webview
npm run build
```

Generate the final machine-readable Task Truth v2 release report and a human-readable completion audit. Inspect them manually. A green command does not override missing labels, denominators, holdout, or native QA.

## Final completion audit

Before declaring completion, create a requirement matrix covering every numbered goal and hard gate across v2.01-v2.05. For each item list:

```text
requirement
authoritative implementation evidence
test or live proof
metric denominator
status: proven | contradicted | incomplete | missing
remaining remediation
```

Task Truth v2 is complete only if every required item is `proven`, the locked release gate is true, all zero-tolerance counts are zero, and no required manual scenario is missing.

If any item is incomplete, keep the feature in shadow/eligible mode and report the exact remaining work. Do not redefine success around session-013, unit tests, architecture, or the cases already available.

## Definition of done

- The visible answer is derived from one verified TaskSnapshot revision.
- Task truth precedes workstream/candidate/target selection.
- The card precisely explains task, state, next step, and where at supported precision.
- No generic fallback masquerades as understanding.
- Ambiguity and two-hypothesis states are useful and explicit.
- React and island share the same contract and strict-open behavior.
- Manual/background adoption is quality-dominant.
- Provenance is split correctly.
- The independently reviewed live corpus and locked holdout satisfy every hard gate.
- Manual macOS interruption QA passes.
- Privacy, latency, cost, rollback, and audit requirements pass.
- The final release report says `passed = true` for the real v2 release gate, not merely a milestone gate.

Only then may the overall Task Truth v2 program be marked complete.
