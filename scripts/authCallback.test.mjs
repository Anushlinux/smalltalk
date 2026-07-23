import assert from "node:assert/strict";
import test from "node:test";

import { parseAuthCallback } from "../src/auth/authCallback.ts";

test("accepts the exact Smalltalk auth callback and extracts its code", () => {
  assert.deepEqual(
    parseAuthCallback("smalltalk://auth/callback?code=authorization-code"),
    { kind: "code", code: "authorization-code" },
  );
});

test("rejects callbacks with the wrong scheme, host, or path", () => {
  assert.deepEqual(parseAuthCallback("other://auth/callback?code=secret"), { kind: "ignored" });
  assert.deepEqual(parseAuthCallback("smalltalk://other/callback?code=secret"), { kind: "ignored" });
  assert.deepEqual(parseAuthCallback("smalltalk://auth/other?code=secret"), { kind: "ignored" });
});

test("returns a concise cancellation error without exposing the callback URL", () => {
  assert.deepEqual(
    parseAuthCallback("smalltalk://auth/callback?error=access_denied&error_description=cancelled"),
    { kind: "oauth_error", message: "Google authorization was cancelled." },
  );
});

test("rejects a valid callback that has no authorization code", () => {
  assert.deepEqual(parseAuthCallback("smalltalk://auth/callback"), { kind: "missing_code" });
});

test("ignores malformed URLs", () => {
  assert.deepEqual(parseAuthCallback("not a url"), { kind: "ignored" });
});
