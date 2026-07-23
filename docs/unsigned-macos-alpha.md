# Unsigned macOS technical alpha

This build is intentionally ad-hoc signed because the project does not yet
have an Apple Developer membership. It is not notarized by Apple. Only share
it with testers who understand and accept that limitation.

## Install

1. Open `smalltalk_0.1.3_aarch64.dmg` and copy `smalltalk.app` to Applications.
2. Open Terminal and run:

   ```bash
   xattr -dr com.apple.quarantine /Applications/smalltalk.app
   ```

3. Launch Smalltalk from Applications.
4. Grant Screen Recording and Accessibility permission when macOS requests
   them. If capture does not begin after permission is granted, quit and reopen
   Smalltalk once.

The `xattr` command removes Gatekeeper's downloaded-file quarantine marker. It
does not provide Apple notarization or a stable Developer ID. Testers should
only run a build received directly from the Smalltalk team.

## Build

```bash
npm run tauri:build:macos:unsigned-alpha
```

The build command creates optimized release artifacts, applies an explicit
ad-hoc signature, and verifies the bundle identifier, hardened runtime, DMG,
and exact executable allowlist. The normal signed QA and release build lanes
remain strict and still require Apple certificates.
