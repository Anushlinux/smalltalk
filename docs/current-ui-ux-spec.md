# Smalltalk Current UI/UX Specification

Last verified: 2026-07-20

This is the product contract for the main Tauri app. It covers the public information architecture, visual system, interaction hierarchy, and the boundary between product UI and developer diagnostics. It does not change Continue engine truth, privacy rules, target-opening safety, or the native macOS island.

Primary implementation files:

- `src/App.tsx` — product navigation, Continue presentation and continuation field, History, Settings, Privacy, and developer Inspect.
- `src/App.css` — the warm application shell, dark Continue canvas, responsive layouts, motion, and accessibility preferences.
- `src-tauri/src/continuation/history.rs` — the bounded read-only history model for explicit Continue answers.
- `src-tauri/src/capture.rs` and `src-tauri/src/lib.rs` — Tauri commands that expose that history to the main app.
- `src-tauri/macos/SessionIslandPanel.swift` — the native island, which remains a separate surface.

## 1. Product promise

Smalltalk exists to answer two questions:

1. What was I doing?
2. Where should I continue?

The default screen must answer those questions before showing controls, evidence, history, or system status. The app is not a session recorder, activity dashboard, analytics product, or evidence browser.

Every Continue answer must remain truthful:

- Current focus and return target are separate facts.
- Supporting work and detours are context, not automatic destinations.
- A visible frame is not automatically safe to open.
- Thin evidence produces an honest unresolved answer.
- The backend owns decision identity, evidence identity, answer text, and opening eligibility.

## 2. Information architecture

The React app has four view modes:

```ts
type ViewMode = "continue" | "history" | "settings" | "developer";
```

The public navigation contains:

- **Continue** — the current continuation answer and primary return action.
- **History** — a read-only record of explicit Continue answers saved locally.
- **Settings** — memory, privacy, permissions, and stored-data controls.

`developer` is labelled **Inspect**. It is visible only in development builds. It is not part of the launch product navigation.

Onboarding and the macOS permission flow are states rather than permanent sidebar destinations. A new user should be guided from Continue to the permission they need, then returned to Continue.

## 3. Application shell

The shell is warm and light. It should feel like a quiet macOS utility, not a dark dashboard.

### 3.1 Sidebar and application frame

The app uses a quiet outer frame with a collapsible sidebar and one rounded main-content surface. The sidebar toggle sits in the outer top strip, outside both the navigation and content card, and stays in the same location whether the sidebar is open or closed.

The sidebar is approximately 66 px when collapsed and 212 px when expanded. It contains:

1. Smalltalk identity.
2. Continue and History.
3. Settings.
4. Inspect only in development builds.

The sidebar starts collapsed to preserve focus on Continue. The existing toggle expands it to reveal the same navigation labels; it does not add destinations. At narrower widths, the expanded width reduces to preserve the main content. Every icon-only button retains an accessible name and a hover title.

All product destinations live inside one bordered, rounded, independently scrollable content card. The sidebar remains on the outer background rather than sharing the content card. This frame relationship should remain consistent across Continue, History, Settings, and Inspect.

The sidebar does not contain a memory control. It must also not contain Privacy, manual evidence updates, Continue refresh, deletion, sessions, search, frames, workstreams, activity statistics, or upgrade-oriented dashboard cards.

### 3.2 Page toolbar

The content card has a compact sticky title bar. Continue uses the personal greeting `Hey Anushrut, get back into work with ⌥`; the Option symbol is visual copy, not a registered shortcut. History, Settings, and Inspect retain compact destination titles. The top-right toolbar contains the single public `Memory on` / `Memory off` control and does not expose a Continue refresh action. The memory control directly calls the existing start or stop action, shows busy feedback, and exposes its inverse action through its accessible label.

Changing destinations always resets the page scroll to the top. Content must never begin underneath the sticky title bar.

## 4. Continue screen

The Continue screen is a warm page containing one dark answer canvas. The contrast gives the continuation moment focus without forcing the entire application into a dark theme.

The dark canvas restores recognition with one dominant, untruncated answer headline. It does not repeat progress, location, or the next step. A compact overflow action remains when correction options are available.

The full Continue screen then answers, in order:

1. **What to continue** — the dark hero headline.
2. **What was completed** — the last checkpoint.
3. **What to do next** — the first supported unresolved step.
4. **Where to return** — a grounded artifact, app, document, or page.
5. **Primary action** — beside the location, and only when the return target is validated.

### 4.1 States

#### Memory not ready

Show one plain explanation. The persistent sidebar memory control and existing permission recovery surface provide the actions.

#### Waiting for evidence

Explain that the user can keep working. Do not fabricate a task or show successful-looking destination language.

#### Generating

Replace the answer content with a polite status, one supporting line, and the bounded dot-matrix animation. Background refreshes remain quiet when a useful answer is already visible.

#### Resolved

Show the exact supported headline and correction overflow in the hero, followed by the checkpoint, continuation point, and safe return action when available. Do not truncate generated answer text.

#### Unresolved

Lead with the honest abstention. Supporting details may explain what is missing, but the public card must not show retry, evidence-only, or fake-target actions.

## 5. Continuation field

The July 21 Continue-view taste direction supersedes the older always-visible supporting-details layout. The area below the dark answer canvas is a borderless continuation field on the page background, not a second raised card or report.

Its default reading order is:

- **Last checkpoint** — the last meaningful completed outcome, paired with the observed app or page when that identity is supported.
- **Continue from here** — the earliest concrete unresolved step, or an honest statement that no exact step was captured. When the destination is supported, the sentence itself names the app, product, or page; a nearby badge is reinforcement, not a substitute for clear copy.
- **Location** — the most precise grounded work surface available.
- **Return action** — shown only when the existing direct-target policy says the target is safely openable.
- **Context trail** — one horizontal icon rail of up to four meaningful surfaces. App surfaces are labeled `App`; browser surfaces are labeled `Page`.

The hero remains the only task explanation. The continuation field must not repeat it as `What you were doing`, combine completed and unfinished work into one paragraph, or expose the recent trail as a large default timeline.

Do not use placeholder destinations such as `the running app`, `the current app`, `the app`, `the browser`, or `the page` when a supported name is available. Prefer direct wording such as `Open Smalltalk and run one real Continue interaction` or `In Codex, run the final check`.

The public context trail includes grounded primary work, useful supporting work, and returns to that work. When those relationships are unavailable, it may show the single current observed non-Smalltalk surface so the user can recognize where the answer belongs; this fallback does not make that surface a return target. Detours, unrelated surfaces, duplicate visits, and diagnostic evidence remain available in Inspect. The trail stays horizontal at every supported width and scrolls sideways when it cannot fit. Each stop shows a known app icon or a quiet generic fallback plus one short relationship sentence. The optional visual-cue disclosure lazily loads the existing evidence-preview frame after the rail. Screenshot evidence never creates a primary action.

When a task is understood but its target is unavailable, show a quiet inline `Exact place not captured` status. Do not replace the useful task answer or turn ordinary uncertainty into a warning card. When no clear task exists, lead with the abstention and do not render a confident continuation field or detailed public trail.

The continuation field must not expose:

- Decision, request, frame, event, artifact, or response identifiers.
- Candidate scores, confidence arithmetic, provider diagnostics, or model prompts.
- Database terminology, schema names, or engine-stage labels.
- Raw capture events or broad activity history.

Correction choices live in the answer overflow rather than as a permanent panel:

- This isn't right.
- This was supporting work.
- This was unrelated.
- Mark task complete.
- Show or hide other possibilities when they exist.

## 6. History

History is needed because users may want to revisit a continuation they explicitly requested. It is not a timeline of everything Smalltalk observed.

The History list:

- Is read only.
- Groups answers by Today, Yesterday, and calendar date.
- Shows answer title, origin (`Island` or `Main app`), and local time.
- Uses pagination rather than loading the entire archive.
- Removes internal evaluation markers from visible copy.

Opening an item shows a calm read-only sheet with human labels. History never exposes raw JSON, IDs, scores, or evidence tables.

The backend bounds history to explicit manual Continue requests and retains at most 100 entries. It exposes two read-only Tauri commands:

- `list_continue_history`
- `get_continue_history_output`

## 7. Settings

Settings uses stacked, scannable rows rather than a dashboard.

Required groups:

- **Local memory** — read-only status and latest-evidence age; the persistent top-right toolbar control owns start and pause.
- **Privacy** — local-first explanation and entry to exclusions.
- **Permissions** — current screen-recording readiness and a recovery action when needed.
- **Stored data** — delete local memory through the existing confirmation flow.
- **Advanced** — Inspect, only in development builds.

Raw typed characters and full clipboard contents remain excluded. Public errors use plain product language. Technical error strings stay in Inspect and logs.

## 8. Inspect

Inspect remains the existing developer workspace for:

- Frames, screenshots, and OCR.
- Search, timeline, raw events, and identifiers.
- Workstreams and engine diagnostics.
- Storage, capture, evaluation, audit, and provider information.

Inspect can be dense because it serves verification. That density must not leak into Continue, its continuation field, History, Settings, or Privacy.

## 9. Visual system

### 9.1 Palette

| Role | Value | Use |
| --- | --- | --- |
| App background | `#F3F2EE` | Main canvas and toolbar |
| Sidebar | `#EBEAE5` with translucency | Navigation rail |
| Raised panel | `#FFFEFA` | History and Settings surfaces |
| Primary ink | `#1B1917` | Headings and main controls |
| Secondary ink | `#68635D` | Supporting copy |
| Quiet ink | `#918B83` | Labels and metadata |
| Divider | `#C9C5BC` / `#DEDBD3` | Hairlines and boundaries |
| Accent | `#8D397D` | Active states and restrained emphasis |
| Accent soft | `#F3DCEF` | Icon wells and subtle selection |
| Continue canvas | `#11110F` | Primary answer object |
| Canvas text | `#F8F6F2` | Continue answer |
| Canvas secondary | `#AAA69E` | Supporting answer copy |
| Canvas accent | `#F2BDEB` | Identity and active generation |

Pink and plum are accents, not page backgrounds. There are no gradients in ordinary controls or cards. The only ambient canvas decoration is the very subtle, low-contrast ring treatment inside the Continue object.

### 9.2 Typography and density

Use the locally bundled Geist variable font, with Geist Mono reserved for keycap treatment. Use tree-shaken Phosphor React icons at consistent 16–18 px regular weight. The answer headline is the largest type in the product. Page titles are compact. Supporting text stays between 12 and 14 px with generous line height.

The resolved dark answer canvas is content-sized rather than hero-height, with a calm 270–320 px minimum depending on viewport height. It uses 24–32 px padding and 18–22 px internal gaps. Long truthful text may grow naturally and must not be truncated.

The UI should feel editorial and calm. Avoid hero-page marketing composition, statistic cards, large decorative banners, pill overuse, and dense tables on public screens.

## 10. Motion

Motion explains interaction and state change.

- Buttons transition exact visual properties over roughly 120–160 ms.
- Pressed controls scale to `0.97` or `0.992`, depending on size.
- Menus and disclosures open from their trigger origin using opacity and `scale(0.97)` with `cubic-bezier(0.23, 1, 0.32, 1)`.
- The dot matrix animates only while an explicit Continue request is generating.
- A newly adopted answer may use one restrained GSAP opacity-and-vertical-settle transition lasting 180–220 ms; attached details may follow by roughly 40 ms.
- Hover styles are gated behind `(hover: hover) and (pointer: fine)`.
- Normal memory status remains static; there is no idle breathing animation.

With Reduce Motion enabled, GSAP positional movement and control scaling are removed. Short opacity and color feedback remains.

## 11. Responsive and accessibility contract

The review widths are 1440, 900, 800, 620, and 360 px.

- Above 860 px, use the full sidebar.
- At 860 px and below, use the 74 px icon rail.
- At 620 px and below, use a 60 px rail and tighter page insets.
- Supporting-detail rows collapse from label/value columns to a single column at narrow widths.
- Answer actions stack only when the available width requires it.

Every icon-only control has an accessible label. Focus remains visible. Continue status uses polite live regions. Generated answer text remains present in the accessibility tree. Destructive actions retain the existing confirmation flow.

## 12. Explicit non-goals for launch

Do not add these to the public main app:

- Activity, session, or productivity analytics.
- Word counts, streaks, insights, or usage gamification.
- A visual timeline of everything observed.
- Candidate-ranking or confidence dashboards.
- General search across raw evidence.
- Team, billing, invite, or upgrade surfaces.
- Separate `See more` and `Why this?` actions.

Those additions would dilute the launch promise. Continue with its compact continuation field, History, Settings, and onboarding/permission recovery are sufficient for the first product surface.
