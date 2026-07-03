# Repository Guidelines

## Project Structure & Module Organization

This repository is centered on a Tauri desktop app. `src/` contains the React/Vite UI. `src-tauri/` contains the Rust backend, Tauri commands, SQLite capture logic, macOS native island code, and helper scripts under `src-tauri/scripts/`. Product notes live in `docs/` and `product.md`. The older WXT browser-extension prototype is preserved in `browser-extension/`, with shared logic in `browser-extension/src/shared/` and tests in `browser-extension/tests/`. Treat `resume_query_exports/`, `cloud_resume_exports/`, `output/`, `target/`, and local snapshot folders as generated artifacts unless a task explicitly asks for fixtures.

## Continue Product Doctrine

Smalltalk is continuation-first, not session-recorder-first. `Continue` is the primary product primitive; sessions, timelines, screenshots, and resume bundles are evidence, debugging, and export infrastructure rather than the core product model. The native desktop app is the MVP lane, so do not build or revive the browser extension for this MVP unless a task explicitly reopens that older prototype.

Smalltalk is now Continue-first at the product-surface level. The primary app screen must be a single continuation answer. Sessions, frames, screenshots, search, timelines, raw events, bundles, native resume cards, cloud resume paths, candidate scoring internals, artifact-role tables, episode/action lists, and evals are diagnostics or evidence inspection only, and must not appear on the first screen by default.

Do not make `Stop Session` a prerequisite for `Continue`. Continue should observe local evidence, resolve stable artifacts, extract task actions, segment episodes, cluster workstreams, score continuation candidates, and return the user to the next actionable point with inspectable evidence.

Do not send broad raw history to a model and ask it to invent intent. Any model call added later must be candidate-bounded, evidence-backed, and locally validated. Every continuation answer must separate factual current focus from actionable return target. Branch and support surfaces are evidence, not default return targets.

Do not store raw typed characters or full clipboard text; preserve the existing privacy boundaries. Do not invent artifacts, URLs, file paths, user intent, or next actions. If evidence is thin, say it is thin. The backend must be able to explain every `Continue` result through frame ids, event ids, artifact ids, action ids, and evidence-quality notes.

## Build, Test, and Development Commands

- `npm install`: install root React/Tauri dependencies.
- `npm run dev`: run the Vite frontend only.
- `npm run tauri dev`: run the desktop app through Tauri; this is the normal local app path.
- `npm run build`: type-check and build the root frontend.
- `cd src-tauri && cargo check`: compile-check the Rust backend.
- `cd src-tauri && cargo test`: run Rust tests when present.
- `cd browser-extension && npm install`: install extension dependencies.
- `cd browser-extension && npm run test`: run Vitest tests for the extension.
- `cd browser-extension && npm run compile`: run WXT preparation plus TypeScript checking.

## Coding Style & Naming Conventions

Use strict TypeScript and React function components. Follow the existing 2-space TypeScript indentation, double quotes, semicolons, `PascalCase` for types/components, and `camelCase` for functions and props. Rust code should be `rustfmt`-formatted, with `snake_case` modules, functions, fields, and Tauri command names. Keep native macOS changes in `src-tauri/macos/` or `src-tauri/src/session_island.rs` unless the React surface is intentionally involved.

## Testing Guidelines

Root desktop changes should at least pass `npm run build` and `cargo check`; use `cargo test` for Rust behavior. Extension logic uses Vitest with `*.test.ts` files under `browser-extension/tests/`. Add tests for parsing, resume-card selection, redaction, export shaping, or other deterministic logic. There is no repository-wide coverage gate currently.

## Commit & Pull Request Guidelines

Recent commits use short imperative subjects such as `Refactor session island capture and export flow`. Keep commits focused and avoid mixing generated exports with source changes. Pull requests should describe behavior, list verification commands, link related issues or notes, and include screenshots or recordings for UI, native island, or capture-flow changes.

## Security & Configuration Tips

Never commit `.env` files, API keys, personal captures, local SQLite databases, screenshots, or resume-query exports. When debugging cloud resume behavior, record whether output came from a real cloud response, cached data, or local fallback.
