import assert from "node:assert/strict";
import test from "node:test";

import {
  isValidEmailAddress,
  normalizeEmailAddress,
  requestPasswordlessEmail,
} from "../src/auth/emailAuth.ts";

test("normalizes surrounding whitespace without rewriting the address", () => {
  assert.equal(normalizeEmailAddress("  Person+Smalltalk@example.com  "), "Person+Smalltalk@example.com");
});

test("accepts a normal email address and rejects incomplete values", () => {
  assert.equal(isValidEmailAddress("person@example.com"), true);
  assert.equal(isValidEmailAddress("person+smalltalk@example.com"), true);
  assert.equal(isValidEmailAddress("person"), false);
  assert.equal(isValidEmailAddress("@example.com"), false);
  assert.equal(isValidEmailAddress("person@"), false);
  assert.equal(isValidEmailAddress("person@@example.com"), false);
  assert.equal(isValidEmailAddress("person @example.com"), false);
});

test("requests a Supabase passwordless email that can create a new user", async () => {
  const calls = [];
  const client = {
    auth: {
      async signInWithOtp(credentials) {
        calls.push(credentials);
        return { data: { user: null, session: null }, error: null };
      },
    },
  };

  const email = await requestPasswordlessEmail(
    client,
    "  person@example.com ",
    "http://127.0.0.1:45453/auth/callback",
  );

  assert.equal(email, "person@example.com");
  assert.deepEqual(calls, [
    {
      email: "person@example.com",
      options: {
        emailRedirectTo: "http://127.0.0.1:45453/auth/callback",
        shouldCreateUser: true,
      },
    },
  ]);
});

test("surfaces Supabase delivery failures", async () => {
  const deliveryError = new Error("Email rate limit exceeded");
  const client = {
    auth: {
      async signInWithOtp() {
        return { data: { user: null, session: null }, error: deliveryError };
      },
    },
  };

  await assert.rejects(
    requestPasswordlessEmail(
      client,
      "person@example.com",
      "http://127.0.0.1:45453/auth/callback",
    ),
    deliveryError,
  );
});
