# Animation plans

| Plan | Title | Severity | Status |
| --- | --- | --- | --- |
| [001](001-single-sweep-island-morph.md) | Replace the cut with one continuous island morph | HIGH | DONE |
| [002](002-simplify-sidebar-toggle-motion.md) | Simplify the sidebar toggle motion | HIGH | DONE |

## Recommended execution order

1. Plan 001 is complete.
2. Plan 002 is complete. The whole-surface sidebar transition is removed and
   only a short label fade remains.

## Dependencies

- Plan 001 has no dependency on another animation plan.
- It assumes the current uncommitted native-island implementation remains the
  baseline and must be preserved.
- Plan 002 has no dependency on plan 001. It is limited to the React sidebar
  toggle in `src/App.tsx` and `src/App.css`.
