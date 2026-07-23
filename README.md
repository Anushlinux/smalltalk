<p align="center">
  <img src="./docs/assets/smalltalk-wordmark.svg" alt="Smalltalk" width="720" />
</p>

<p align="center">
  <strong>Leave any task. Come back to the right next move.</strong>
</p>

<p align="center">
  A local-first macOS app that helps you recover what you were doing, where you left it, and what to do next.
</p>

<p align="center">
  <a href="https://github.com/Anushlinux/smalltalk/releases/latest"><strong>Download the latest macOS build</strong></a>
  ·
  <a href="#run-smalltalk-locally">Run from source</a>
  ·
  <a href="#how-smalltalk-protects-your-data">Privacy</a>
</p>

---

## Pick up the work, not just the last window

Modern work rarely lives in one place. A task can move through a browser tab, a document, a terminal, a chat, and several supporting pages before it is interrupted.

Most activity tools can show what happened. Smalltalk is built to answer a harder question:

> **What was I actually trying to do, and where should I continue?**

Press **Continue** and Smalltalk gives you one concise, evidence-backed answer. It separates the screen you happen to be looking at now from the work that is genuinely unfinished. When it can verify a safe return target, it can take you back there. When the evidence is too thin, it says so instead of inventing an answer.

## What Smalltalk gives you

- **One clear continuation answer.** See the task, the point you reached, what remains unfinished, and the next useful action.
- **Context across apps and windows.** Smalltalk can connect work that moved through browsers, editors, terminals, documents, chats, and supporting tools.
- **A truthful return target.** The latest screen and the best place to resume are treated as separate facts.
- **Inspectable evidence.** You can open the supporting context behind an answer instead of trusting a black box.
- **A native macOS experience.** Continue is available in the main app and from a compact floating island.
- **Honest uncertainty.** If Smalltalk cannot support a task or target with evidence, it abstains.

## How Continue works

1. **Observe locally.** Smalltalk uses macOS screen, window, Accessibility, and interaction signals to build a sparse record of meaningful changes.
2. **Recover the task.** It groups related evidence into workstreams and distinguishes primary work from references, detours, and interruptions.
3. **Validate the answer.** Any model-assisted interpretation is bounded to selected evidence and checked against local facts.
4. **Return safely.** Smalltalk opens a target only when that exact destination is supported and safe to open.

Stopping capture is never a prerequisite for Continue. The product is designed around interruption recovery, not around recording and ending sessions.

## How Smalltalk protects your data

Smalltalk is local-first. Its evidence store and task memory live on your Mac in SQLite.

- Raw typed characters are not stored.
- Full clipboard text is not stored.
- Capture is sparse and bounded rather than a permanent high-frequency recording.
- Smalltalk does not send broad raw history to a model.
- Cloud inference, when used, receives a compact privacy-filtered evidence packet selected for the explicit Continue request.
- Model output cannot create a file, URL, task, or return target that local evidence does not support.

Local-first does not mean fully offline: the current Continue path can use a cloud model for semantic interpretation. The local evidence and validation layers remain responsible for deciding what the product is allowed to claim or open.

## Project status

Smalltalk is a **technical alpha for macOS**. The native desktop app is the active product; the older browser-extension prototype remains in the repository for historical reference and is not the current MVP.

Current releases are built for Apple Silicon and Intel Macs. They are ad-hoc signed and not yet Apple-notarized, so macOS may show an additional warning during first installation. Expect product behavior, storage contracts, and setup details to change while the release gates are being completed.

## Run Smalltalk locally

### 1. Install the prerequisites

You need:

- macOS
- [Node.js 22](https://nodejs.org/)
- [Rust stable](https://www.rust-lang.org/tools/install)
- Xcode Command Line Tools
- The [Tauri 2 macOS prerequisites](https://v2.tauri.app/start/prerequisites/)
- A Supabase project for sign-in and profile storage

Install the Xcode tools with:

```bash
xcode-select --install
```

### 2. Clone and install

```bash
git clone https://github.com/Anushlinux/smalltalk.git
cd smalltalk
npm install
```

### 3. Configure Supabase

Link the repository to your Supabase project and apply the included database migrations:

```bash
npx supabase login
npx supabase link --project-ref YOUR_PROJECT_REF
npx supabase db push
```

In your Supabase Auth URL configuration, add this redirect URL:

```text
http://127.0.0.1:45453/auth/callback
```

Enable email sign-in, Google OAuth, or both. Google sign-in also requires the normal Google provider credentials in Supabase.

Create the local environment file:

```bash
cp .env.example .env
```

Then replace the placeholders in `.env`:

```dotenv
VITE_SUPABASE_URL=https://YOUR_PROJECT_REF.supabase.co
VITE_SUPABASE_PUBLISHABLE_KEY=your-public-publishable-key

# Optional local developer fallback. Keep this only on your machine.
OPENAI_API_KEY=your-openai-api-key
OPENAI_MODEL=gpt-5.6-luna
```

Never use a Supabase service-role key in the desktop app. The publishable key is designed to be embedded in a client; the service-role key is private and bypasses Row Level Security.

### 4. Start the desktop app

```bash
npm run auth:check-config
npm run tauri dev
```

On first launch, Smalltalk guides you through the macOS permissions it needs:

- **Screen Recording** to observe visible work
- **Accessibility** to understand app and interface context
- **Input Monitoring** to detect privacy-safe interaction signals without storing typed text

After granting a permission, macOS may ask you to restart the app.

## Use your own inference service

The desktop app can call the included Cloudflare Worker instead of putting an OpenAI key in the distributed app. This is the recommended path for an independent deployment.

1. Update `smalltalk-api/wrangler.jsonc` with your Supabase URL and desired Worker settings.
2. Apply the Supabase migrations described above.
3. Add the Worker secrets:

```bash
cd smalltalk-api
npm install
npx wrangler secret put OPENAI_API_KEY
npx wrangler secret put USER_HASH_SECRET
npx wrangler secret put SUPABASE_PUBLISHABLE_KEY
npm run deploy
```

4. Start Smalltalk with the deployed endpoint:

```bash
cd ..
SMALLTALK_API_URL=https://YOUR_WORKER.workers.dev/v1/continue npm run tauri dev
```

The Worker verifies the signed-in Supabase user, enforces request limits, calls the model, and returns the structured answer. It does not store screenshots, prompts, model output, URLs, paths, window titles, or captured text.

## Useful development commands

```bash
# Frontend-only development
npm run dev

# Type-check and build the React app
npm run build

# Deterministic webview, auth, and updater tests
npm run test:webview

# Check the Rust backend
cd src-tauri && cargo check

# Run the Rust test suite
cd src-tauri && cargo test
```

## Repository guide

| Path | What lives here |
| --- | --- |
| `src/` | React and Vite interface for the desktop app |
| `src-tauri/` | Rust backend, SQLite evidence store, Tauri commands, native helpers, and updater |
| `src-tauri/macos/` | Native Swift floating-island interface |
| `smalltalk-api/` | Optional Cloudflare Worker inference gateway |
| `supabase/` | Authentication, profile, and inference-quota migrations |
| `docs/` | Product contracts, architecture notes, release guidance, and verification plans |
| `browser-extension/` | Preserved prototype; not the active product path |

For a deeper technical tour, read:

- [Product and technical specification](./PRODUCT.md)
- [Full engine flow](./docs/full-engine-flow.md)
- [Continue architecture](./docs/continue-architecture.md)
- [Current UI and UX contract](./docs/current-ui-ux-spec.md)
- [macOS release and updater guide](./docs/github-updater-release.md)

## Contributing

Issues and focused pull requests are welcome.

Before changing the product, keep its central rule intact: **Continue must recover an evidence-backed unfinished task or honestly abstain.** Screenshots, sessions, timelines, scores, and raw events are diagnostic evidence; they are not the default product experience.

For changes to the main app, run at least:

```bash
npm run build
cd src-tauri && cargo check
```

UI, native island, and capture-flow changes should also include screenshots or recordings and a clear description of the manual macOS checks performed.
