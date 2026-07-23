import type { SupabaseClient } from "@supabase/supabase-js";

export function normalizeEmailAddress(value: string) {
  return value.trim();
}

export function isValidEmailAddress(value: string) {
  const normalized = normalizeEmailAddress(value);
  if (!normalized || normalized.length > 320 || /\s/.test(normalized)) return false;

  const separator = normalized.indexOf("@");
  return (
    separator > 0 &&
    separator === normalized.lastIndexOf("@") &&
    separator < normalized.length - 1
  );
}

export async function requestPasswordlessEmail(
  client: SupabaseClient,
  value: string,
  redirectTo: string,
) {
  const email = normalizeEmailAddress(value);
  if (!isValidEmailAddress(email)) {
    throw new Error("Enter a valid email address.");
  }

  const { error } = await client.auth.signInWithOtp({
    email,
    options: {
      emailRedirectTo: redirectTo,
      shouldCreateUser: true,
    },
  });

  if (error) throw error;
  return email;
}
