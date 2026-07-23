import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { getCurrent, onOpenUrl } from "@tauri-apps/plugin-deep-link";
import { openUrl } from "@tauri-apps/plugin-opener";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { invoke } from "@tauri-apps/api/core";
import { getVersion } from "@tauri-apps/api/app";
import { listen } from "@tauri-apps/api/event";
import type { Session, User } from "@supabase/supabase-js";
import { parseAuthCallback } from "./authCallback";
import {
  AuthConfigurationError,
  getSupabaseClient,
  isTrustedSupabaseOAuthUrl,
} from "./supabase";
import { requestPasswordlessEmail } from "./emailAuth";

export type Profile = {
  id: string;
  email: string | null;
  full_name: string | null;
  avatar_url: string | null;
  provider: string | null;
  created_at: string;
  updated_at: string;
  last_sign_in_at: string | null;
};

export type AuthState = {
  initialized: boolean;
  loading: boolean;
  session: Session | null;
  user: User | null;
  profile: Profile | null;
  error: string | null;
  signInWithGoogle: () => Promise<void>;
  signInWithEmail: (email: string) => Promise<boolean>;
  signOut: () => Promise<void>;
};

const AuthContext = createContext<AuthState | null>(null);
const INSTALLATION_ID_KEY = "smalltalk.installation_id.v1";

function getInstallationId() {
  const existing = window.localStorage.getItem(INSTALLATION_ID_KEY);
  if (existing && /^[0-9a-f-]{16,64}$/i.test(existing)) return existing;
  const created = crypto.randomUUID();
  window.localStorage.setItem(INSTALLATION_ID_KEY, created);
  return created;
}

function errorMessage(error: unknown, fallback: string) {
  if (error instanceof AuthConfigurationError) return error.message;
  if (error instanceof Error && error.message.trim()) return `${fallback}: ${error.message}`;
  return fallback;
}

async function revealMainWindow() {
  try {
    const mainWindow = getCurrentWindow();
    await mainWindow.show();
    await mainWindow.unminimize();
    await mainWindow.setFocus();
  } catch {
    // Authentication can still finish if macOS has already revealed the app.
  }
}

export function AuthProvider({ children }: { children: ReactNode }) {
  const [initialized, setInitialized] = useState(false);
  const [loading, setLoading] = useState(true);
  const [session, setSession] = useState<Session | null>(null);
  const [profile, setProfile] = useState<Profile | null>(null);
  const [error, setError] = useState<string | null>(null);
  const mountedRef = useRef(true);
  const loginInFlightRef = useRef(false);
  const processedCodesRef = useRef(new Set<string>());

  const loadProfile = useCallback(async (user: User) => {
    const client = getSupabaseClient();

    for (let attempt = 0; attempt < 3; attempt += 1) {
      const { data, error: profileError } = await client
        .from("profiles")
        .select(
          "id,email,full_name,avatar_url,provider,created_at,updated_at,last_sign_in_at",
        )
        .eq("id", user.id)
        .maybeSingle<Profile>();

      if (!mountedRef.current) return;
      if (data) {
        setProfile(data);
        return;
      }

      if (profileError || attempt === 2) {
        setProfile(null);
        setError(
          profileError
            ? `You are signed in, but your profile could not be loaded: ${profileError.message}`
            : "You are signed in, but your profile is not available yet.",
        );
        return;
      }

      await new Promise((resolve) => window.setTimeout(resolve, 250 * (attempt + 1)));
    }
  }, []);

  const processIncomingUrls = useCallback(
    async (urls: string[]) => {
      for (const rawUrl of urls) {
        const callback = parseAuthCallback(rawUrl);
        if (callback.kind === "ignored") continue;

        await revealMainWindow();

        if (callback.kind === "auth_error") {
          if (mountedRef.current) setError(callback.message);
          return;
        }

        if (callback.kind === "missing_code") {
          if (mountedRef.current) {
            setError("Authentication returned without an authorization code.");
          }
          return;
        }

        if (processedCodesRef.current.has(callback.code)) return;
        processedCodesRef.current.add(callback.code);

        if (mountedRef.current) {
          setLoading(true);
          setError(null);
        }

        try {
          const client = getSupabaseClient();
          const { data, error: exchangeError } = await client.auth.exchangeCodeForSession(
            callback.code,
          );
          if (exchangeError) throw exchangeError;
          if (!data.session) throw new Error("Supabase did not return a session.");

          if (mountedRef.current) {
            setSession(data.session);
            void loadProfile(data.session.user);
          }
        } catch (exchangeError) {
          if (mountedRef.current) {
            setError(errorMessage(exchangeError, "The authorization code could not be exchanged"));
          }
        } finally {
          if (mountedRef.current) setLoading(false);
        }

        return;
      }
    },
    [loadProfile],
  );

  useEffect(() => {
    mountedRef.current = true;
    let cancelled = false;
    let unlisten: (() => void) | undefined;
    let unlistenLoopback: (() => void) | undefined;
    let unsubscribeAuth: (() => void) | undefined;

    const initialize = async () => {
      let client;
      try {
        client = getSupabaseClient();
      } catch (configurationError) {
        if (!cancelled) {
          setError(errorMessage(configurationError, "Authentication configuration is invalid"));
          setLoading(false);
          setInitialized(true);
        }
        return;
      }

      const { data } = client.auth.onAuthStateChange((_event, nextSession) => {
        if (cancelled) return;
        setSession(nextSession);
        if (nextSession) {
          window.setTimeout(() => void loadProfile(nextSession.user), 0);
        } else {
          setProfile(null);
        }
      });
      unsubscribeAuth = () => data.subscription.unsubscribe();

      try {
        unlistenLoopback = await listen<string[]>("auth-callback", (event) => {
          void processIncomingUrls(event.payload);
        });
      } catch {
        if (!cancelled) {
          setError("Smalltalk could not start its secure sign-in callback listener.");
        }
      }

      try {
        unlisten = await onOpenUrl((urls) => {
          void processIncomingUrls(urls);
        });
      } catch {
        // Keep the custom-URL listener as a compatibility fallback. The normal
        // browser flow returns through the loopback listener above.
      }

      try {
        const startUrls = await getCurrent();
        if (startUrls?.length) await processIncomingUrls(startUrls);
      } catch {
        // There may be no launch URL, and the loopback callback remains active.
      }

      try {
        const { data: sessionData, error: sessionError } = await client.auth.getSession();
        if (sessionError) throw sessionError;
        if (!cancelled) {
          setSession(sessionData.session);
          if (sessionData.session) void loadProfile(sessionData.session.user);
        }
      } catch (sessionError) {
        if (!cancelled) {
          setSession(null);
          setProfile(null);
          setError(errorMessage(sessionError, "Your saved session could not be restored"));
        }
      } finally {
        if (!cancelled) {
          setLoading(false);
          setInitialized(true);
        }
      }
    };

    void initialize();

    return () => {
      cancelled = true;
      mountedRef.current = false;
      unlisten?.();
      unlistenLoopback?.();
      unsubscribeAuth?.();
    };
  }, [loadProfile, processIncomingUrls]);

  useEffect(() => {
    let cancelled = false;
    const synchronizeCloudSession = async () => {
      try {
        if (!session) {
          await invoke("set_cloud_auth_session", {
            accessToken: null,
            installationId: null,
            appVersion: null,
          });
          return;
        }
        const appVersion = await getVersion();
        if (cancelled) return;
        await invoke("set_cloud_auth_session", {
          accessToken: session.access_token,
          installationId: getInstallationId(),
          appVersion,
        });
      } catch {
        if (!cancelled) {
          setError("You are signed in, but secure cloud inference could not be initialized.");
        }
      }
    };
    void synchronizeCloudSession();
    return () => {
      cancelled = true;
    };
  }, [session]);

  const signInWithGoogle = useCallback(async () => {
    if (loginInFlightRef.current) return;
    loginInFlightRef.current = true;
    setLoading(true);
    setError(null);

    try {
      const client = getSupabaseClient();
      const redirectTo = await invoke<string>("get_auth_redirect_url");
      const { data, error: signInError } = await client.auth.signInWithOAuth({
        provider: "google",
        options: {
          redirectTo,
          skipBrowserRedirect: true,
          queryParams: {
            prompt: "select_account",
          },
        },
      });

      if (signInError) throw signInError;
      if (!data.url || !isTrustedSupabaseOAuthUrl(data.url)) {
        throw new Error("Supabase returned an invalid authorization URL.");
      }

      try {
        await openUrl(data.url);
      } catch (openError) {
        throw new Error(errorMessage(openError, "The OAuth window could not be opened"));
      }
    } catch (signInError) {
      setError(errorMessage(signInError, "Google authentication could not be started"));
    } finally {
      loginInFlightRef.current = false;
      setLoading(false);
    }
  }, []);

  const signInWithEmail = useCallback(async (email: string) => {
    if (loginInFlightRef.current) return false;
    loginInFlightRef.current = true;
    setLoading(true);
    setError(null);

    try {
      const client = getSupabaseClient();
      const redirectTo = await invoke<string>("get_auth_redirect_url");
      await requestPasswordlessEmail(client, email, redirectTo);
      return true;
    } catch (signInError) {
      setError(errorMessage(signInError, "Email authentication could not be started"));
      return false;
    } finally {
      loginInFlightRef.current = false;
      setLoading(false);
    }
  }, []);

  const signOut = useCallback(async () => {
    setLoading(true);
    setError(null);

    try {
      const { error: signOutError } = await getSupabaseClient().auth.signOut();
      if (signOutError) throw signOutError;
      setSession(null);
      setProfile(null);
    } catch (signOutError) {
      setError(errorMessage(signOutError, "You could not be signed out"));
    } finally {
      setLoading(false);
    }
  }, []);

  const value = useMemo<AuthState>(
    () => ({
      initialized,
      loading,
      session,
      user: session?.user || null,
      profile,
      error,
      signInWithGoogle,
      signInWithEmail,
      signOut,
    }),
    [
      error,
      initialized,
      loading,
      profile,
      session,
      signInWithEmail,
      signInWithGoogle,
      signOut,
    ],
  );

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>;
}

export function useAuth() {
  const value = useContext(AuthContext);
  if (!value) throw new Error("useAuth must be used inside AuthProvider.");
  return value;
}
