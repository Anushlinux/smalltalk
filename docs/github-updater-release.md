# Smalltalk GitHub update pipeline

Smalltalk checks the latest published GitHub Release after startup and every six hours while it remains open. A newer semantic version produces an in-app **Update and restart** prompt. The update is downloaded, verified with Smalltalk's Tauri public key, installed, and then the app relaunches.

## One-time GitHub setup

The permanent private updater key is stored only on the release owner's Mac:

```text
/Users/bhaskarpandit/.tauri/smalltalk.key
```

Back up that file in a secure password manager or encrypted backup. Losing it means existing installations cannot verify any future update key.

Add its contents to the `Anushlinux/smalltalk` repository as an Actions secret named `TAURI_SIGNING_PRIVATE_KEY`. With an authenticated GitHub CLI, run:

```bash
gh secret set TAURI_SIGNING_PRIVATE_KEY --repo Anushlinux/smalltalk < /Users/bhaskarpandit/.tauri/smalltalk.key
```

The `Anushlinux/smalltalk` repository and its Releases must be public before distributing the updater-enabled build. Installed apps read `latest.json` and the update archive without receiving a GitHub credential. A private repository would return an authorization error to customer installations.

## Publish a release

Keep the version identical in:

- `package.json`
- `src-tauri/Cargo.toml`
- `src-tauri/tauri.conf.json`

For example, after synchronizing the desktop app version and lockfiles to `0.1.3`, commit that change, then create and push the matching tag:

```bash
npm run release:check-version
git tag v0.1.3
git push origin v0.1.3
```

Pushing the tag starts `.github/workflows/publish-macos.yml`. The workflow runs deterministic webview tests and builds both Apple Silicon and Intel versions. It publishes the DMGs, updater archives, signatures, and `latest.json` in one non-draft GitHub Release.

Do not create a normal GitHub Release by uploading only a DMG. Installed copies require the Tauri updater archive, its `.sig` signature, and `latest.json`.

## First updater-enabled installation

An app that was installed before the updater code existed cannot discover this feed retroactively. Those users must install the first updater-enabled DMG manually once. Every later version can use the in-app update flow.

Because the technical alpha is not Apple-notarized, first-time installation still requires the documented quarantine-removal command. Update authenticity is protected by the separate Tauri signature, but stable Apple signing is still required to guarantee that macOS privacy permissions survive every update.
