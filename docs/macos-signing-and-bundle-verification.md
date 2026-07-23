# macOS Signing And Bundle Verification

## Why this exists

macOS privacy permissions belong to a code-signing identity, not merely to the
name shown in System Settings. An ad-hoc build has a designated requirement
based only on its code hash. That hash changes when the executable changes, so
Screen Recording or Accessibility permission granted to one build may not
apply to the next build.

`npm run tauri dev` is a separate development executable. Its permission state
must not be used as proof that a packaged `smalltalk.app` has permission.

## Configure a stable QA identity

The repository does not contain certificate files, passwords, Apple account
credentials, or a developer name. Tauri reads the signing identity from the
standard `APPLE_SIGNING_IDENTITY` environment variable.

List identities already installed in the login keychain:

```bash
security find-identity -v -p codesigning
```

For local packaged QA, select an `Apple Development` identity and build with:

```bash
export APPLE_SIGNING_IDENTITY="Apple Development: Your Name (TEAMID)"
npm run tauri:build:macos:qa
```

Non-debug macOS bundling is intentionally rejected when it is invoked outside
this wrapper. The wrapper removes only the previous generated release app,
sets the verified-build marker for the Tauri bundle hook, builds the app, and
then runs the signature and executable allowlist checks. This prevents a
direct `tauri build` from retaining a stale helper or evaluator binary inside
an otherwise newly signed app.

The explicitly named `npm run tauri:build:macos:unsigned-alpha` command is the
only exception. It creates an ad-hoc-signed technical-alpha package for teams
without Apple Developer membership. It is not signed QA or release evidence,
and testers must follow [the unsigned alpha installation guide](unsigned-macos-alpha.md).

The build command removes only the generated release `smalltalk.app` before
bundling. This prevents stale development binaries from surviving inside a new
bundle. It then verifies the signature and bundle contents.

The macOS bundle hook also rejects a normal non-debug `tauri build` when
`APPLE_SIGNING_IDENTITY` is absent or set to the ad-hoc pseudo-identity `-`.
This makes it impossible to accidentally produce a release `.app` whose
privacy permission identity changes on the next rebuild. Debug bundles remain
available for local structural checks, but they are not permission or release
evidence.

Do not put `APPLE_SIGNING_IDENTITY` in a committed `.env` file. The identity
name is not a private key, but keeping machine-specific signing configuration
outside the repository avoids accidental coupling to one developer account.

## Release verification

Distribution builds must use a `Developer ID Application` identity and should
be notarized separately. Build the release-profile app with:

```bash
export APPLE_SIGNING_IDENTITY="Developer ID Application: Your Company (TEAMID)"
# Configure one Tauri-supported notarization credential set:
# APPLE_API_ISSUER + APPLE_API_KEY + APPLE_API_KEY_PATH
# or APPLE_ID + APPLE_PASSWORD + APPLE_TEAM_ID
npm run tauri:build:macos:release
```

The release command rejects Apple Development and ad-hoc identities before it
starts the build. It also refuses to build without one complete notarization
credential set. Secrets remain in the environment and must never be committed.
Verify an existing release bundle with:

```bash
export SMALLTALK_SIGNING_PROFILE=release
export APPLE_SIGNING_IDENTITY="Developer ID Application: Your Company (TEAMID)"
./scripts/verify-macos-bundle.sh \
  src-tauri/target/release/bundle/macos/smalltalk.app
```

The verifier rejects:

- an ad-hoc signature;
- a missing Apple team identifier;
- a code-hash-only designated requirement;
- disabled hardened runtime;
- a bundle identifier other than `com.smalltalk.app`;
- release builds not signed with Developer ID Application;
- a release app rejected by Gatekeeper after signing and notarization;
- a `Contents/MacOS` executable set other than the main `smalltalk` binary and
  the six approved native capture sidecars. Development tools such as
  `task_truth_v2_eval` are rejected;
- a main executable or capture sidecar with an invalid, ad-hoc, CDHash-only, or
  different-team signature. Every permission-sensitive helper must be signed
  by the same Apple team as the outer app.

The Tauri bundle contains only the explicitly configured native capture
sidecars. Cargo evaluator programs are gated behind the `eval-binaries`
feature, so a normal GUI build does not compile or bundle them.

Run evaluator commands by enabling the feature explicitly:

```bash
cd src-tauri
cargo run --features eval-binaries --bin continue_accuracy_eval -- --help
cargo run --features eval-binaries --bin task_truth_v2_eval -- --help
cargo run --features eval-binaries --bin task_truth_v2_release_gate -- --help
```

The same rule applies to focused compile checks:

```bash
cd src-tauri
cargo check --features eval-binaries --bin task_truth_v2_eval
```
