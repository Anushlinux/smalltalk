# 002 — Simplify the sidebar toggle motion

- **Status**: DONE
- **Commit**: 963b7fa2
- **Severity**: HIGH
- **Category**: Purpose and frequency; performance; cohesion
- **Estimated scope**: 2 files, approximately 45–70 changed lines

## Problem

The sidebar toggle currently uses the Document View Transition API to snapshot
and crossfade the whole sidebar and main card. That includes the animated
MOSAIC canvas inside the card. The transition therefore freezes, rescales, and
crossfades a large rasterized surface for a frequent navigation action. It
feels heavier than the simple sidebar state change and can show double-exposed
or stretched content.

`src/App.tsx:2981` currently adds synchronous React state flushing, browser
feature detection, cancellation, and promise cleanup for one boolean toggle:

```tsx
// src/App.tsx:2981 — current
const toggleSidebar = useCallback(() => {
  const updateSidebar = () => {
    flushSync(() => {
      setSidebarOpen((open) => !open);
    });
  };
  const reducedMotion = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  const viewTransitionDocument = document as Document & {
    startViewTransition?: (update: () => void) => {
      finished: Promise<void>;
      skipTransition: () => void;
    };
  };

  if (reducedMotion || !viewTransitionDocument.startViewTransition) {
    updateSidebar();
    return;
  }

  sidebarViewTransitionRef.current?.skipTransition();
  const transition = viewTransitionDocument.startViewTransition(updateSidebar);
  sidebarViewTransitionRef.current = transition;
  void transition.finished.finally(() => {
    if (sidebarViewTransitionRef.current === transition) {
      sidebarViewTransitionRef.current = null;
    }
  });
}, []);
```

`src/App.css:6595`, `src/App.css:6610`, and `src/App.css:6670` opt the two
largest surfaces into a 240 ms drawer transition:

```css
/* src/App.css:6595 — current */
view-transition-name: smalltalk-sidebar;

/* src/App.css:6610 — current */
view-transition-name: smalltalk-main-card;

/* src/App.css:6670 — current */
::view-transition-group(smalltalk-sidebar),
::view-transition-group(smalltalk-main-card),
::view-transition-old(smalltalk-sidebar),
::view-transition-new(smalltalk-sidebar),
::view-transition-old(smalltalk-main-card),
::view-transition-new(smalltalk-main-card) {
  animation-duration: 240ms;
  animation-timing-function: var(--ease-drawer);
}
```

The labels also combine opacity, horizontal translation, and delayed
visibility. That is more choreography than this frequent control needs:

```css
/* src/App.css:6639 — current */
.product-sidebar .identity-block > div:last-child,
.primary-nav button span,
.secondary-nav button span {
  display: block;
  overflow: hidden;
  opacity: 1;
  transform: translateX(0);
  transition:
    opacity 140ms var(--ease-out),
    transform 180ms var(--ease-out),
    visibility 0s linear;
  white-space: nowrap;
}
```

## Target

Make the sidebar toggle deliberately simple:

- Update the open/closed layout immediately. Do not animate
  `grid-template-columns`, width, padding, margin, or the main card.
- Keep the existing stable `20px` navigation icon column, button padding, row
  height, and sidebar padding in both states. Icons must not translate, scale,
  crossfade, or change grid alignment.
- Animate only the text labels from opacity `0` to `1` over `120ms` using the
  existing `--ease-out: cubic-bezier(0.23, 1, 0.32, 1)` token.
- Do not translate labels. The opacity change is enough to soften the state
  change without making the navigation feel animated for its own sake.
- Hide collapsed labels from accessibility and pointer interaction using the
  existing sidebar state and CSS visibility, but delay `visibility: hidden`
  until the 120 ms opacity transition finishes.
- Under `prefers-reduced-motion: reduce`, retain a 100 ms opacity-only change.
- Do not snapshot or crossfade the animated MOSAIC canvas.

The React toggle should return to a direct state update:

```tsx
// target
onClick={() => setSidebarOpen((open) => !open)}
```

The label transition should be:

```css
/* target */
.product-sidebar .identity-block > div:last-child,
.primary-nav button span,
.secondary-nav button span {
  display: block;
  overflow: hidden;
  opacity: 1;
  transition:
    opacity 120ms var(--ease-out),
    visibility 0s linear;
  white-space: nowrap;
}

.sidebar-collapsed .product-sidebar .identity-block > div:last-child,
.sidebar-collapsed .primary-nav button span,
.sidebar-collapsed .secondary-nav button span {
  visibility: hidden;
  opacity: 0;
  transition:
    opacity 120ms var(--ease-out),
    visibility 0s linear 120ms;
}
```

## Repo conventions to follow

- Motion tokens live in `src/App.css:45-46`. Reuse `--ease-out`; do not add a
  new curve.
- Button press feedback already uses a small compositor-only scale in
  `src/App.css:3811-3813`. Preserve it; it is separate from sidebar layout.
- Reduced-motion overrides already live at the end of the final shell layer in
  `src/App.css:6737`. Extend that existing block instead of creating another
  media query.

## Steps

1. In `src/App.tsx`, remove `flushSync` from the `react-dom` imports.
2. Remove `sidebarViewTransitionRef` near `src/App.tsx:1585`.
3. Remove the `toggleSidebar` callback near `src/App.tsx:2981`.
4. Change the sidebar button back to the direct functional state update shown
   in the Target section.
5. In `src/App.css`, remove both `view-transition-name` declarations and the
   complete `::view-transition-*` rule.
6. Preserve the stable sidebar, identity, navigation-button, and icon-grid
   geometry at `src/App.css:6617-6656`.
7. Replace the label transition rules with the exact opacity-only target above.
   Remove both `translateX` declarations.
8. In the existing reduced-motion block, give these labels a 100 ms opacity
   transition and no positional movement.

## Boundaries

- Do NOT change sidebar width, colors, borders, typography, icons, navigation
  destinations, card geometry, or responsive breakpoints.
- Do NOT change `src/MosaicLeafBackground.tsx` or the Canvas2D renderer.
- Do NOT add GSAP, Motion, springs, keyframes, or another animation library.
- Do NOT animate layout properties.
- Do NOT launch or restart the Tauri app. The user will inspect the existing
  `npm run tauri dev` app.
- If the cited code has materially drifted from commit `963b7fa2`, stop and
  report the mismatch instead of improvising.

## Verification

- **Mechanical**:
  - Run `npm run build`; TypeScript and Vite must succeed.
  - Run `npm run test:continue-presentation`; all existing tests must pass.
  - Run `git diff --check -- src/App.tsx src/App.css`.
  - Confirm `rg -n "startViewTransition|view-transition|sidebarViewTransition|flushSync" src/App.tsx src/App.css` returns no matches.
- **Feel check**: in the already-running Tauri app, toggle the sidebar slowly,
  then click it repeatedly and confirm:
  - The main card changes size once without freezing, stretching, or
    crossfading the MOSAIC animation.
  - Every icon remains in exactly the same x/y position.
  - Only the text labels fade; nothing slides or bounces.
  - Rapid toggles always reflect the latest state immediately.
  - With reduced motion enabled, the state remains clear and only the brief
    opacity feedback remains.
- **Done when**: the sidebar feels immediate and quiet, icons do not move, the
  MOSAIC canvas never freezes or double-exposes, and no whole-surface animation
  remains.
