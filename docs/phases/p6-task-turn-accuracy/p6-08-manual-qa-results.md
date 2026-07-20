# P6.08 Manual QA Results

Date: 2026-07-11  
Platform: macOS, local debug application bundle  
Privacy: No screenshots, raw decision ids, URLs, paths, or private conversation text are recorded here.

## Run status

The Tauri development command reached the native executable successfully after adding the package's explicit default binary. A debug `.app` bundle was also built and launched successfully. The DMG packaging step failed after the `.app` bundle had already been produced; DMG packaging is not required for this product-behavior QA.

The native controller later gained enough access to confirm the launched application's empty first screen, but the full scenario run was stopped at user direction before scenario evidence could be injected and observed. The scenarios below are therefore recorded as not run, not passed. Deterministic backend and island contract tests cover the expected state/action tuples, but they are not represented as manual observations.

The React surface was also opened through the local Vite server for a read-only browser inspection. That pass confirmed one `Continue decision` card on the first screen, no visible diagnostics shell, semantic heading structure, and a native button in the card with `tabIndex=0`, `type=button`, and an accessible text label. Because a browser page cannot exercise Tauri IPC or the native island, this is partial React-only evidence and does not clear the blocked native scenarios.

Six pure presentation-policy tests now cover direct `Continue here`, frame-preview inspection, task-known/target-null copy, current-focus separation, split task/target confidence, and stale/support-only blocking.

## Scenarios

| Scenario | Expected task-state tuple | Expected target/action | Main-card/island parity | Observed | Result | Redacted evidence reference |
| --- | --- | --- | --- | --- | --- | --- |
| Clear chat/agent task without a conversation locator | task known; actor/state/waiting-on retained | `task_known_target_unknown`; Inspect evidence | Same task summary and no direct open | Scenario execution stopped at user direction | Not run | P6.08 task-known/target-null contract test |
| Direct openable file or browser page | task known; target eligible | `direct_continue_ready`; Continue here through decision id | Same direct state and action | Scenario execution stopped at user direction | Not run | Direct-target policy and island direct-action tests |
| Frame-preview-only weak surface | task known or thin; frame remains evidence | `frame_preview_only`; Inspect evidence | No public return target in either surface | Scenario execution stopped at user direction | Not run | P6.08 frame-fallback public-boundary test |
| Support branch | primary task retained; support relation explicit | `target_support_only`; no direct open | Both surfaces block support-only open | Scenario execution stopped at user direction | Not run | Existing support-branch island parity tests |
| Feedback-suppressed target | task recap retained where supported | `target_feedback_suppressed`; no direct open | Both surfaces suppress target | Scenario execution stopped at user direction | Not run | Feedback suppression and strict-open tests |
| Stale decision after newer task evidence | prior decision no longer actionable | `stale_decision`; Refresh before action | Both surfaces block open and request refresh | Scenario execution stopped at user direction | Not run | Evidence-watermark strict-open and island freshness tests |
| Main card/island parity across states | same decision and task identity | direct/inspect/blocked action parity | Required | Scenario execution stopped at user direction | Not run | Island contract mapping suite |
| Keyboard and accessibility behavior | focusable controls with accurate accessible names | Continue here only for direct; Inspect evidence otherwise | Matching action labels | React empty-state card exposed one enabled semantic button with `tabIndex=0`, `type=button`, and accessible label; native direct/inspect scenario execution stopped at user direction | Not run natively | React browser inspection plus six presentation-policy tests |

## React-only observations

| Check | Observed | Result |
| --- | --- | --- |
| First-screen composition | One Continue card and no visible developer diagnostics shell | Pass for React empty state |
| Semantic structure | `Continue` heading, `Continue` region, and `Continue decision` region exposed | Pass for React empty state |
| Keyboard-ready primary control | Card button used native button semantics, `tabIndex=0`, `type=button`, and an accessible label | Pass for React empty state |
| Direct versus inspect scenario copy | Six pure presentation-policy tests passed | Pass automated; native observation still blocked |

## Deferred native release check

No further Computer Use or manual scenario work is included in this P6.08 implementation run, per user direction. If native manual verification is required as a release gate, P6.09 or the release checklist must exercise all eight scenarios and replace each not-run row with the actual observed tuple and pass/fail result. This document does not claim a manual QA pass.
