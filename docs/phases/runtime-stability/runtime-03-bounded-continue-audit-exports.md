# Runtime Stability 03 — Keep Continue Audits Useful Without Re-Exporting The World

## Codex task

Redesign standard `continue_outputs` generation so one manual Continue produces a compact, decision-scoped, inspectable audit. Prevent concurrent export storms, keep a deliberately explicit full-forensic mode, and add retention controls that do not destroy reviewed evidence unexpectedly.

Complete Runtime Stability 01 and 02 first. Preserve the user's ability to understand why Smalltalk produced an answer.

## Read before editing

```text
AGENTS.md
PRODUCT.md
docs/phases/runtime-stability/runtime-00-always-on-stability-program.md
docs/phases/runtime-stability/runtime-01-screen-capture-crash-containment.md
docs/phases/runtime-stability/runtime-02-bounded-event-and-storage-runtime.md
docs/full-engine-flow.md
src/App.tsx
src-tauri/src/capture.rs
src-tauri/src/continuation.rs
src-tauri/src/continuation/activity_recap_integration.rs
src-tauri/src/continuation/task_truth_v2/audit.rs
```

Inspect recent audit folders and identify which files are genuinely necessary to explain one decision. Do not copy personal contents into fixtures.

## Verified failure

Seven diagnosed audit folders occupied about 227 MB. A standard manual request produced roughly 26–43 MB because it exported cumulative tables such as all `continue_decisions`, all ordered evidence spans, and all task actions.

Each audit is launched with an independent detached thread. Repeated manual clicks can therefore make several exporters read the same live database and write large overlapping folders concurrently.

The audit feature is needed. The failure is its scope and workload control, not its existence.

## Required implementation

### 1. Define standard and forensic audit modes

Standard manual audit must be decision-scoped. It should include:

```text
normalized request
final public decision
decision trace
selected task, workstream, and candidate
alternatives considered for this decision
activity recap and validation
task-turn evidence used by this decision
selected evidence closure
model request and response metadata when applicable
fallback and validation reasons
cache identity
openability and target checks
warnings and missing evidence
safe runtime and storage diagnostics
manifest with sizes and hashes
```

Do not export every row from every Continue table in standard mode.

Keep full database and table export behind an explicit developer-only forensic mode. It must never be enabled by an ordinary Continue click or background refresh.

### 2. Export the transitive evidence closure

Build a bounded closure starting from the current decision id. Follow only the identifiers required to explain that decision, with explicit depth and row limits.

If required evidence is missing because it was already retained or compacted, record that fact as a typed audit warning. Do not silently substitute unrelated historical rows.

### 3. Add single-flight export control

Use one export coordinator with a bounded queue. Required behavior:

- at most one active standard export;
- duplicate requests for the same decision coalesce;
- a newer request may queue once, not spawn unlimited threads;
- application shutdown cancels or drains safely;
- incomplete folders are clearly marked and never look complete;
- finalization uses atomic rename or an equivalent commit boundary;
- UI receives queued, running, complete, or failed status.

Continue computation must return without waiting for the complete disk export, but the exporter must not be an untracked detached workload.

### 4. Add audit retention and pinning

Implement a configurable developer-output budget using both count and bytes. Preserve:

- explicitly pinned or reviewed audits;
- audits referenced by active manual QA records;
- the newest completed audits needed for testing.

Prune only completed, unpinned audit directories. Never prune the currently running export. Report what was removed and why.

Do not include `continue_outputs` in capture evidence or task inference.

### 5. Avoid unnecessary duplication

Do not write the same large JSON object under several paths unless compatibility requires it. If compatibility aliases remain, make them small references or document their cost.

Avoid duplicate image formats and variants unless each has a proven consumer. Preserve privacy-safe evidence previews required for manual review.

### 6. Add deterministic tests and budgets

Tests must prove:

- standard audit exports only decision-linked rows;
- old unrelated decisions do not enlarge a new standard audit;
- repeated requests coalesce;
- only one exporter runs;
- interruption leaves a typed incomplete state;
- pinning protects an audit;
- byte and count retention remove only eligible folders;
- forensic mode remains explicit;
- background Continue cannot enable audit export;
- audit output paths are excluded from capture and task evidence.

Set and justify a standard-audit size budget. A reasonable initial gate is no more than 10 MB for the diagnosed live-shaped fixture, with a target below 5 MB. If the evidence closure legitimately exceeds that, document the exact required files and establish a tested alternative budget rather than quietly removing evidence.

## Acceptance criteria

- Standard manual audits are decision-scoped.
- Unrelated historical rows do not appear in a new standard audit.
- Standard audit size meets the established gate on the live-shaped fixture.
- Only one exporter can run at a time.
- Repeated clicks cannot create an unbounded export queue.
- Full forensic export is explicit and developer-only.
- Pinned or reviewed outputs are protected by retention.
- A completed audit still explains every user-visible claim and uncertainty.
- Background Continue writes no audit folder.
- Automated checks and a live repeated-click test pass.

## Verification commands

Run at minimum:

```bash
cd src-tauri && cargo fmt --check
cd src-tauri && cargo check
cd src-tauri && cargo test
npm run build
npm run tauri dev
git diff --check
git status --short
```

Also report:

```text
standard audit byte size and file count
decision-linked row counts
forensic audit byte size when explicitly requested
peak active exporter count during repeated-click testing
queue depth and coalesced request count
```

## Final response format

Report:

1. Standard audit contract.
2. Evidence-closure rules and limits.
3. Export coordinator behavior.
4. Retention and pinning policy.
5. Before and after size comparison.
6. Repeated-click concurrency result.
7. Tests and commands.
8. Any information intentionally restricted to forensic mode.

