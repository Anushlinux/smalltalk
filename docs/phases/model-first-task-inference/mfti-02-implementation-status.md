# MFTI-02 implementation status

## Verdict

The code path is implemented and its deterministic verification is green. The phase is **not complete** because the required three successful live provider sequences have not been produced.

The current debug application bundle was rebuilt during verification. macOS treated that rebuilt capture client as different from the application that already had Screen Recording permission. The live audit recorded a ScreenCaptureKit TCC denial even though System Settings visibly showed the `smalltalk` toggle enabled.

The immediate product bug was repaired: screenshots had been delegated to a separately compiled Swift command-line helper, so the capture request did not run under the permission-bearing application process. macOS capture now runs inside `smalltalk` through Core Graphics. Audio remains disabled.

The rebuild-level cause is also repaired in source and packaging. Release bundles now require a certificate-backed `APPLE_SIGNING_IDENTITY`; ad-hoc release builds are rejected. The first deliberate **Turn on local memory** action performs the one allowed permission request before capture is marked running. Request state is scoped to the running code's designated requirement, so a stable signed update keeps its permission identity while a changed ad-hoc CDHash does not inherit stale request state. Six native Swift helpers are compiled during the developer build and shipped as signed sidecars; a customer Mac never invokes `swiftc` or the Swift interpreter.

This machine currently has zero installed code-signing identities. The packaged layout is verified with a debug bundle, but the required two-build stable-signature and TCC-retention proof remains externally blocked until an Apple Development or Developer ID Application certificate is installed.

## Implemented goals

### Observation-stream dependency repair

- Scroll events retain pointer coordinates.
- A scroll without coordinates can still be grounded to an owned content region.
- A `same_screen_idle` or other no-change diff cannot claim that content appeared or disappeared.
- Keyframes carry per-frame app, window, document/page, ownership, privacy, and evidence-window identity.

### Provider configuration and diagnostics

- Manual Continue uses one explicit OpenAI multimodal provider path.
- Cloud inference is on by default unless it is explicitly disabled.
- Background image upload remains disabled.
- Diagnostics distinguish disabled, missing credentials, unavailable model, privacy block, invalid request, timeout, provider failure, invalid response, verification rejection, and success.
- Audits store provider/model identity, local and provider request identity, response identity, latency, selected image metadata, token usage, and estimated cost without storing image bytes in the audit.

### Temporal request and structured response

- Images are ordered chronologically and interleaved with their frame identity and ownership metadata.
- The request contains grounded elements, causal events, semantic deltas, transitions, privacy notes, and a same-scope prior hypothesis when available.
- The strict response supports one to three competing hypotheses.
- Surface, immediate operation, semantic effect, subtask, primary task, task object, relationship, lifecycle, progress, unfinished state, and possible next action remain separate fields.

### Verifier authority boundary

- The verifier checks cited records, frame ownership, hashes, privacy, source kind, chronology, and contradictions.
- Passive page text, browser chrome, app output, and third-party text cannot establish a user task.
- A task or subtask requires user-authored or causal support.
- A semantic effect requires a real semantic-delta reference.
- A next action requires an evidence-backed unfinished state and user-authored plan evidence.
- Removing the central task claim produces `verification_rejected`; the verifier never constructs a replacement task.

### Persistence, presentation, and failure behavior

- Verified inference is persisted before wording.
- Snapshots retain semantic source, provider/model/request/response identity, packet identity, selected hypothesis, field support, confidence, and wording source.
- Local compatibility projections are excluded from semantic-authority selection.
- Provider failure persists an unresolved result and cannot fall back to a local label such as Browsing, Editing, Reviewing, or Searching.
- The primary UI uses honest unavailable or insufficient-evidence language and exposes safe provider provenance in the developer explanation.
- An explicit native-island Continue press uses the manual inference path; background island activity does not upload images.

### Packaged macOS permission durability

- `start_capture` checks Screen Recording before it creates a session or reports capture as running.
- App launch only reads permission state. It never requests permission or opens System Settings.
- **Turn on local memory** is the deliberate one-time request action and refreshes permission state on every later attempt.
- The request claim is an atomic per-identity SQLite insert, so concurrent UI or IPC calls cannot invoke the macOS request twice. Delete-all and developer reset preserve only these one-time markers while still removing captured evidence and ordinary diagnostics.
- Diagnostics expose the actual executable path, bundle identifier, signing identifier, Team ID, designated requirement, CDHash, and signature kind instead of hard-coding `com.smalltalk.app`.
- Normal release bundling refuses a missing or ad-hoc signing identity and refuses to bypass the clean verified wrapper. The signed QA workflow verifies the bundle identifier, hardened runtime, stable designated requirements, and matching Team ID across the app and every native helper. The release profile additionally requires a Developer ID Application identity, complete notarization credentials, and Gatekeeper acceptance.
- Cargo evaluator binaries require the explicit `eval-binaries` feature and cannot enter a normal GUI bundle.
- Packaged helper resolution requires the six signed `Contents/MacOS` sidecars and never falls back to runtime Swift compilation.

## Verification completed

- `cargo fmt --all -- --check`: passed.
- `cargo check`: passed.
- `cargo test task_truth_v2`: 73 passed, 0 failed, 2 opt-in live tests ignored.
- `cargo test continuation`: 465 passed, 0 failed, 2 opt-in live tests ignored.
- `swiftc -module-cache-path /tmp/smalltalk-swift-module-cache -typecheck scripts/capture_events.swift`: passed.
- `npm run build`: passed.
- `npm run test:webview`: 17 passed, 0 failed.
- `cargo test --lib`: passed.
- `cargo test screen_capture_permission --lib`: passed.
- `cargo test swift_helpers --lib`: 3 passed, 0 failed.
- `npm run tauri -- build --debug --bundles app`: passed without launching the app; the bundle contains only `smalltalk` and the six approved native helpers.
- Runtime-source audit for `/usr/bin/swiftc`, `/usr/bin/swift`, and equivalent `Command::new` calls: no matches.
- `cargo check --features eval-binaries --bin task_truth_v2_eval`: passed.
- `git diff --check`: passed.

## Live verification evidence

### Non-private provider transport smoke

The opt-in synthetic smoke test successfully called the configured provider with only the repository's public 32x32 application icon and a synthetic packet:

- provider/model: `openai` / `gpt-5.4-mini`
- diagnostic: `success`
- resolver status: `insufficient_evidence` (expected for the intentionally content-free packet)
- request id: `task-truth-request-e485d227af385131`
- response id: `resp_0fe17b4a1e6b086a016a5386e8b7bc81a3ace085f038120d6e`
- latency: 10,278 ms
- image count/bytes: 1 / 974
- actual token usage: 3,450 input, 1,425 output, 4,875 total
- private capture data sent: false

This smoke exposed and then verified a response-contract repair: field evidence is now a strict object keyed by all semantic fields. Every non-null semantic field must have a grounded evidence object; a null semantic field must have a null evidence slot.

Run this non-private transport check explicitly with:

```bash
cd src-tauri
cargo test live_provider_transport_smoke_uses_only_synthetic_evidence -- --ignored --nocapture
```

### Private manual-Continue evidence

One manual Continue call reached the current v2 runtime and produced safe audit metadata:

- audit schema: `smalltalk.task_truth_multimodal_inference_audit.v2`
- provider enabled: true
- inference origin: `live_cloud`
- provider: `openai`
- configured model: `gpt-5.4-mini`
- result: `insufficient_evidence`
- diagnostic: `request_invalid`
- safe failure reason: `no_readable_image_asset`

The provider was correctly not called because no privacy-approved readable current image was available. No local semantic fallback label was shown.

The approved private Sequence A replay produced two successful OpenAI responses while the request/verifier contract was being repaired:

- first response: `resolved`, 4 images, 19,837 ms, 50,541 total tokens; verification rejected all central task fields because evidence hashes were attached to the wrong record types;
- second response: `ambiguous`, 4 images, 23,045 ms, 52,137 total tokens; immediate operation, document/thread identity, and current actor survived verification, but the central task was still rejected;
- both calls recorded exact request/response identity and ran a bounded second pass;
- no raw hypotheses or screenshots are included here.

Those responses led to two concrete fixes:

- the request now gives exact per-source reference rules, including null hashes for events/deltas and exact text hashes for canonical elements;
- the verifier now treats an exact record id and frame as authoritative, normalizes only redundant hash metadata to the packet's exact value, records `evidence_hash_normalized_to_request`, and downgrades confidence without creating semantic meaning. Missing record ids remain hard failures.

Further private provider calls were denied by the execution environment after explicit user authorization. No workaround was attempted, and every temporary database copy was deleted. Therefore:

- Sequence A produced successful cloud responses but not a verified semantic result.
- Sequence B was not attempted.
- Sequence C was not attempted.
- No human semantic-quality verdict is claimed.

An opt-in private-session smoke harness is implemented as the ignored test `live_provider_smoke_against_private_session`. It requires a writable database copy, an exact reviewed session id, and explicit authorization to send the selected private screenshots and activity evidence to the configured provider. Normal test commands never run it.

No credentials, screenshots, or raw private model payloads are included in this report.

## Remaining completion gate

Install an Apple Development or Developer ID Application certificate, run `npm run tauri:build:macos:qa`, and prove that two differently built versions keep the same certificate-backed designated requirement and Screen Recording grant from `/Applications/smalltalk.app`. Then run Sequences A, B, and C from the phase document. Each sequence must produce a successful response with request/response identity, inspected hypotheses, verifier changes, final verified semantics, latency/cost, and a human judgment. Only then can this phase be marked complete and handed to MFTI-03.
