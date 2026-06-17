# Smalltalk Resume Bookmark

A local-first WXT browser extension prototype for restoring research context. It captures an explicit research session, builds a tab and attention graph, sends a sanitized dossier to a localhost proxy, and highlights the exact paragraph to continue from.

## Run

```bash
/Users/bhaskarpandit/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/bin/node /opt/homebrew/lib/node_modules/npm/bin/npm-cli.js install
/Users/bhaskarpandit/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/bin/node /opt/homebrew/lib/node_modules/npm/bin/npm-cli.js run proxy
/Users/bhaskarpandit/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/bin/node /opt/homebrew/lib/node_modules/npm/bin/npm-cli.js run dev
```

Set `OPENAI_API_KEY` before starting the proxy. If the proxy or key is unavailable, the extension still returns a deterministic local resume card so the capture and highlighting flow can be tested.

## Prototype Contract

- Capture starts only after `Start research`.
- Raw page and attention evidence remains in extension storage.
- OpenAI receives only a compact sanitized dossier through `http://localhost:8787/api/resume`.
- The API key never lives in the browser extension.
