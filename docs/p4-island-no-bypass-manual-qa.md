# P4 Island No-Bypass Manual QA

Use this checklist after the deterministic tests pass. The goal is to verify that the native island never displays or opens a target that the main Continue card would suppress.

## Setup

Run the desktop app:

```sh
npm run tauri dev
```

Use the main Continue card and the native island in the same run. Keep Inspect open only when checking evidence; the default surface should stay Continue-first.

## Checks

1. Start local memory.
   Expected: island shows local memory warming or checking Continue. It must not show legacy resume-ready copy.

2. Open the island before enough evidence exists.
   Expected: local memory warming or no clear continuation. No open Continue target action is available.

3. Create valid work in an openable target.
   Expected: main card and island share the fresh `continue_decision_id`; island shows Continue and opens through `open_resume_point` with source `island_primary`.

4. Reject or ignore a stale target.
   Expected: island does not offer Continue to that target. Audit shows `open_allowed: false` and a feedback/suppression blocked reason if an open is attempted.

5. Open support surfaces after primary work, such as search, docs, diagnostics, or messages.
   Expected: island treats the support surface as evidence only unless promoted by local evidence. It must not become the return target from recency or openability alone.

6. Switch to weak current work with thin evidence.
   Expected: island says recent work seen or no safe return target yet and offers Inspect evidence, not open Continue target.

7. Create newer event-only evidence after a decision.
   Expected: island refreshes or asks for refresh before opening. A stale cached decision must not open.

8. Pause local memory if the stop path builds a resume-query bundle.
   Expected: island must not show legacy resume-ready state from that bundle.

## Audit Evidence

For explicit island-triggered Continue decisions, inspect the latest `continue_outputs/.../decision/island_continue_audit.json`.

Expected fields:

```json
{
  "island": {
    "state_schema": "smalltalk.island_continue_state.v1",
    "trigger_reason": "user_pressed_continue",
    "decision_id": "...",
    "display_state": "continue_ready",
    "available_actions": ["open_continue_target"],
    "open_attempted": true,
    "open_allowed": true,
    "open_blocked_reason": null,
    "source": "island_primary"
  }
}
```

The file must not contain raw screenshots, raw URLs, raw paths, or raw typed text.
