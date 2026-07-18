# Animation plans

| Plan | Title | Severity | Status |
| --- | --- | --- | --- |
| [001](001-single-sweep-island-morph.md) | Replace the cut with one continuous island morph | HIGH | DONE |

## Recommended execution order

1. Execute plan 001 first. It replaces the feel-breaking micro/medium view
   handoff with one persistent animatable capsule.

## Dependencies

- Plan 001 has no dependency on another animation plan.
- It assumes the current uncommitted native-island implementation remains the
  baseline and must be preserved.
