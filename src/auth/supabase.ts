import { createClient, type SupabaseClient } from "@supabase/supabase-js";
import { browserAuthStorage } from "./authStorage";

const DEFAULT_REDIRECT_URL = "smalltalk://auth/callback";

let sharedSupabaseClient: SupabaseClient | null = null;

export class AuthConfigurationError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "AuthConfigurationError";
  }
}

function requiredEnvironmentValue(name: string, value: string | undefined) {
  const normalized = value?.trim();
  if (!normalized) {
    throw new AuthConfigurationError(
      `Smalltalk authentication is not configured. Add ${name} to the local .env file and restart the app.`,
    );
  }
  return normalized;
}

export function getSupabaseRedirectUrl() {
  return import.meta.env.VITE_SUPABASE_REDIRECT_URL?.trim() || DEFAULT_REDIRECT_URL;
}

export function getSupabaseClient() {
  if (sharedSupabaseClient) return sharedSupabaseClient;

  const url = requiredEnvironmentValue("VITE_SUPABASE_URL", import.meta.env.VITE_SUPABASE_URL);
  const key = requiredEnvironmentValue(
    "VITE_SUPABASE_PUBLISHABLE_KEY",
    import.meta.env.VITE_SUPABASE_PUBLISHABLE_KEY || import.meta.env.VITE_SUPABASE_ANON_KEY,
  );

  sharedSupabaseClient = createClient(url, key, {
    auth: {
      flowType: "pkce",
      persistSession: true,
      autoRefreshToken: true,
      detectSessionInUrl: false,
      storage: browserAuthStorage,
    },
  });

  return sharedSupabaseClient;
}

export function isTrustedSupabaseOAuthUrl(rawUrl: string) {
  try {
    const authorizationUrl = new URL(rawUrl);
    const configuredUrl = new URL(
      requiredEnvironmentValue("VITE_SUPABASE_URL", import.meta.env.VITE_SUPABASE_URL),
    );

    return (
      authorizationUrl.protocol === "https:" &&
      authorizationUrl.origin === configuredUrl.origin &&
      authorizationUrl.pathname.endsWith("/auth/v1/authorize")
    );
  } catch {
    return false;
  }
}
