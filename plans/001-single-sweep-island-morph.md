# 001 — Replace the cut with one continuous island morph

- **Status**: DONE
- **Commit**: 3c11ebe7
- **Severity**: HIGH
- **Category**: Physicality and origin; interruptibility; cohesion
- **Estimated scope**: 2 files, approximately 120–180 changed lines

## Problem

The micro-to-medium hover transition is not one continuous motion. SwiftUI
removes a 58×10 micro capsule and inserts a separate 168×34 medium capsule.
The first shape only fades out, while the second shape begins at 92% scale and
fades in. There are no interpolated frames connecting the two silhouettes, so
the viewer sees a small block grow, disappear, and a second larger block grow.

`src-tauri/macos/SessionIslandPanel.swift:1196` currently switches between two
independent view identities:

```swift
// src-tauri/macos/SessionIslandPanel.swift:1196 — current
var body: some View {
    ZStack(alignment: .top) {
        switch model.presentation {
        case .micro:
            microView
                .transition(microTransition)
        case .ambientMemory:
            ambientMemoryView
                .transition(ambientMemoryTransition)
        case .answerSummary:
            answerSummaryView
                .transition(.opacity)
        case .answerExpanded:
            answerExpandedView
                .transition(stateTransition(scale: 0.97))
        }
    }
```

The micro silhouette is a standalone capsule:

```swift
// src-tauri/macos/SessionIslandPanel.swift:1219 — current
private var microView: some View {
    Button(action: onRevealAmbientMemory) {
        Capsule()
            .fill(WhisperFlowPreview.surface)
            .overlay(
                Capsule()
                    .stroke(
                        WhisperFlowPreview.outline.opacity(microOutlineOpacity),
                        lineWidth: s(1)
                    )
            )
            .frame(width: s(kBaseMicroVisualW), height: s(kBaseMicroVisualH))
            .scaleEffect(microScale, anchor: .top)
```

The medium silhouette is another standalone capsule around different content:

```swift
// src-tauri/macos/SessionIslandPanel.swift:1254 — current
private var ambientMemoryView: some View {
    HStack(spacing: s(5)) {
        ambientStatusRegion
        // Arrow action omitted here only for brevity; preserve it verbatim.
    }
    .padding(.leading, s(8))
    .padding(.trailing, s(4))
    .frame(
        width: s(ambientCapsuleWidth),
        height: s(ambientCapsuleHeight)
    )
    .background(WhisperFlowPreview.surface)
    .overlay(
        Capsule()
            .stroke(WhisperFlowPreview.outline, lineWidth: s(1))
    )
    .clipShape(Capsule())
```

The transitions confirm the handoff instead of a morph:

```swift
// src-tauri/macos/SessionIslandPanel.swift:1705 — current
private var microTransition: AnyTransition {
    guard !reduceMotion else { return .opacity }
    return .asymmetric(
        insertion: AnyTransition.opacity
            .animation(IslandMotion.microAmbientScaleDownTop()),
        removal: AnyTransition.opacity
            .animation(IslandMotion.microAmbientScaleUpTop())
    )
}

private var ambientMemoryTransition: AnyTransition {
    guard !reduceMotion else { return .opacity }
    return .asymmetric(
        insertion: AnyTransition.modifier(
            active: IslandMorphModifier(
                opacity: 0,
                scale: kWhisperFlowMicroAmbientTransitionScale
            ),
            identity: IslandMorphModifier(opacity: 1, scale: 1)
        )
        .animation(IslandMotion.microAmbientScaleUpTop()),
        removal: AnyTransition.modifier(
            active: IslandMorphModifier(
                opacity: 0,
                scale: kWhisperFlowMicroAmbientTransitionScale
            ),
            identity: IslandMorphModifier(opacity: 1, scale: 1)
        )
        .animation(IslandMotion.microAmbientScaleDownTop())
    )
}
```

## Target

Implement a **continuity transition** and **morph**: one persistent capsule path
must interpolate its width and height from micro to medium and back. The top
edge must remain fixed. Only the inner medium content should fade; the black
capsule silhouette must never crossfade, disappear, or be replaced during the
micro ↔ medium interaction.

Use a custom animatable shape so the silhouette changes without animating the
layout frame itself:

```swift
// target — add near IslandMorphModifier
private struct TopAnchoredCapsuleShape: Shape {
    var width: CGFloat
    var height: CGFloat

    var animatableData: AnimatablePair<CGFloat, CGFloat> {
        get { AnimatablePair(width, height) }
        set {
            width = newValue.first
            height = newValue.second
        }
    }

    func path(in rect: CGRect) -> Path {
        let clampedWidth = min(max(0, width), rect.width)
        let clampedHeight = min(max(0, height), rect.height)
        let capsuleRect = CGRect(
            x: rect.midX - clampedWidth / 2,
            y: rect.minY,
            width: clampedWidth,
            height: clampedHeight
        )
        return Path(
            roundedRect: capsuleRect,
            cornerRadius: clampedHeight / 2,
            style: .continuous
        )
    }
}
```

The exact standard geometry remains:

- Micro: `58 × 10 pt`.
- Medium capture state: `168 × 34 pt`.
- Timed notification: `236 × 46 pt`, with a 14 pt status label and 2 pt timer line.
- Standard native panel: `187 × 49 pt`.
- Timed-notification native panel: `255 × 61 pt`.
- Anchor: top-center. The path must always use `y: rect.minY`.

Use one on-screen morph curve because this is an element already on screen
moving between sizes:

```swift
// target
static func memoryContinuityMorph(_ reduceMotion: Bool) -> Animation? {
    guard !reduceMotion else { return nil }
    return .timingCurve(
        0.77,
        0,
        0.175,
        1,
        duration: 0.18
    )
}
```

This is `cubic-bezier(0.77, 0, 0.175, 1)` over 180 ms. Do not keep the
direction-specific enter/exit curves for micro ↔ medium, because there is no
longer an entering or exiting silhouette. There is one shape morphing on
screen.

Create one persistent `memoryContinuityView` for both `.micro` and
`.ambientMemory`. Its fill and stroke must use the same animated width and
height values:

```swift
// target structure; keep existing controls and callbacks intact
private var memoryContinuityView: some View {
    let expanded = model.presentation == .ambientMemory
    let visualWidth = expanded ? ambientCapsuleWidth : kBaseMicroVisualW * microScale
    let visualHeight = expanded ? ambientCapsuleHeight : kBaseMicroVisualH * microScale

    return ZStack(alignment: .top) {
        TopAnchoredCapsuleShape(
            width: s(visualWidth),
            height: s(visualHeight)
        )
        .fill(WhisperFlowPreview.surface)

        TopAnchoredCapsuleShape(
            width: s(visualWidth),
            height: s(visualHeight)
        )
        .stroke(
            WhisperFlowPreview.outline.opacity(expanded ? 1 : microOutlineOpacity),
            lineWidth: s(1)
        )

        ambientMemoryContent
            .opacity(expanded ? 1 : 0)
            .allowsHitTesting(expanded)

        microHitTarget
            .opacity(expanded ? 0 : 1)
            .allowsHitTesting(!expanded)
    }
    .frame(
        width: s(ambientPanelWidth),
        height: s(ambientPanelHeight),
        alignment: .top
    )
    .animation(
        IslandMotion.memoryContinuityMorph(shouldReduceMotion),
        value: model.presentation
    )
}
```

`ambientMemoryContent` must contain the existing dot matrix, status copy,
arrow button, countdown line, accessibility labels, hover handlers, and memory
start action, but it must not draw another black background capsule or another
capsule outline. The shared `TopAnchoredCapsuleShape` owns the only visible
silhouette.

Orchestrate content separately from the silhouette:

- Expand: medium content opacity `0 → 1` over `120 ms ease-out`, delayed `40 ms`.
- Collapse: medium content opacity `1 → 0` over `100 ms ease-out`, with no delay.
- The shared silhouette continues its uninterrupted 180 ms interpolation
  during both directions.
- Disable hit testing for whichever control layer has opacity zero.
- Do not remove `memoryContinuityView` when switching only between `.micro`
  and `.ambientMemory`.

For Reduce Motion:

- Do not interpolate width or height.
- Snap the silhouette to its target geometry.
- Keep a `120 ms ease-out` opacity change for the medium content.

## Repo conventions to follow

- Native island motion lives in `IslandMotion` inside
  `src-tauri/macos/SessionIslandPanel.swift:537`.
- The native panel already preserves the top-center anchor in
  `resolvedPanelFrame(preserveCurrentAnchor:)`; do not change that method.
- The micro and standard medium states already share the same native panel
  size in `targetPanelSize` at
  `src-tauri/macos/SessionIslandPanel.swift:2190`. Preserve that invariant.
- Preserve `NSWorkspace.shared.accessibilityDisplayShouldReduceMotion` through
  the existing `shouldReduceMotion` property.
- Preserve the persistent `WhisperFlowIslandModel`; do not recreate the
  `NSHostingView` or SwiftUI root during state updates.

## Steps

1. In `src-tauri/macos/SessionIslandPanel.swift`, add
   `TopAnchoredCapsuleShape` with `AnimatablePair<CGFloat, CGFloat>` exactly as
   specified above. Its rectangle must remain horizontally centered and use
   `rect.minY` so the top line never moves.
2. Add `IslandMotion.memoryContinuityMorph(_:)` using
   `cubic-bezier(0.77, 0, 0.175, 1)` and `0.18` seconds.
3. Refactor the `.micro` and `.ambientMemory` branches into one stable
   `memoryContinuityView` branch. Group the switch cases or move them behind a
   single branch whose view identity does not change when the enum toggles
   between these two values.
4. Split `microView` into a transparent `microHitTarget`; it must keep the
   existing action, accessibility label, 86×24 hit region, hover callback,
   pulse lifecycle, and top alignment, but must not draw its own capsule.
5. Split `ambientMemoryView` into `ambientMemoryContent`; preserve all current
   text, buttons, hover logic, countdown behavior, memory lifecycle cleanup,
   accessibility, and start-memory routing, but remove its background capsule,
   outline capsule, and capsule clipping.
6. Draw the black fill and outline exactly once, using the persistent
   `TopAnchoredCapsuleShape`. Drive both instances from the same
   `visualWidth`/`visualHeight` targets.
7. Apply the 180 ms continuity morph to the silhouette. Apply the 120 ms/40 ms
   delayed expand fade and 100 ms immediate collapse fade only to medium
   content. Ensure transparent layers cannot receive pointer input.
8. Remove `microTransition`, `ambientMemoryTransition`, and
   `kWhisperFlowMicroAmbientTransitionScale` if they become unused. Do not
   remove `stateTransition(scale:)`, which still serves the expanded answer.
9. In `src-tauri/src/session_island.rs`, replace the source-contract assertions
   that require two separate transitions with assertions that require:
   `TopAnchoredCapsuleShape`, animatable width and height, `y: rect.minY`, one
   stable micro/ambient branch, one shared silhouette, the exact 180 ms curve,
   content-only opacity orchestration, top alignment, and the Reduced Motion
   fallback.

## Boundaries

- Do NOT change the 168×34 standard capture pill, 236×46 timed notification,
  152×30 answer summary, or 520×152 expanded answer geometry.
- Do NOT change the one-second medium-to-micro hover hold.
- Do NOT change `Start memory` versus `Memory paused` state semantics.
- Do NOT change the three-second memory feedback countdown.
- Do NOT change capture controls, `start_capture`, Continue preview behavior,
  answer timing, privacy/error precedence, Rust data types, Tauri commands,
  backend action routing, or public APIs.
- Do NOT add dependencies.
- Do NOT animate the native panel width during ordinary micro ↔ medium hover;
  those two states must continue sharing the 187×49 panel.
- If the cited structure has drifted since commit `3c11ebe7`, STOP and report
  the drift instead of improvising.

## Verification

- **Mechanical**:
  - `swiftc -typecheck src-tauri/macos/SessionIslandPanel.swift` succeeds; the
    existing macOS 14 `onChange(of:perform:)` deprecation warnings are allowed.
  - `cd src-tauri && cargo fmt --all --check` succeeds.
  - `cd src-tauri && cargo check` succeeds.
  - `cd src-tauri && cargo test session_island --lib` reports 47 passed and 0
    failed, or the current higher test count with 0 failed.
  - `npm run build` succeeds.
  - `git diff --check` succeeds.
- **Feel check**: run `npm run tauri dev`, then:
  - Hover micro and confirm the same black silhouette continuously interpolates
    from 58×10 to 168×34. It must never disappear and no second block may pop
    in.
  - Leave medium and confirm the same silhouette reverses continuously after
    the one-second hold.
  - Re-enter during collapse and confirm the current motion redirects smoothly
    instead of restarting from micro or flashing.
  - Record at 60 fps and inspect frame by frame. Every frame must contain one
    black capsule only; its top edge must stay on the same pixel row.
  - Check the 3-second starting/pausing notification expands to 236×46
    without affecting normal medium geometry.
  - Enable Reduce Motion and confirm geometry snaps while the content uses only
    the short opacity transition.
- **Done when**: micro ↔ medium reads as one uninterrupted Dynamic-Island-style
  morph in both directions, with no silhouette crossfade, gap, double exposure,
  midpoint size jump, or top-edge movement.
