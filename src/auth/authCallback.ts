export type AuthCallbackResult =
  | { kind: "ignored" }
  | { kind: "auth_error"; message: string }
  | { kind: "missing_code" }
  | { kind: "code"; code: string };

export function parseAuthCallback(rawUrl: string): AuthCallbackResult {
  let url: URL;

  try {
    url = new URL(rawUrl);
  } catch {
    return { kind: "ignored" };
  }

  if (
    url.protocol !== "smalltalk:" ||
    url.hostname !== "auth" ||
    url.pathname !== "/callback"
  ) {
    return { kind: "ignored" };
  }

  const authError = url.searchParams.get("error_description") || url.searchParams.get("error");
  if (authError) {
    const normalizedError = authError.replace(/\+/g, " ").trim();
    const wasCancelled =
      normalizedError.toLowerCase().includes("access_denied") ||
      normalizedError.toLowerCase().includes("cancel");

    return {
      kind: "auth_error",
      message: wasCancelled
        ? "Sign-in was cancelled."
        : `Authentication failed: ${normalizedError}`,
    };
  }

  const code = url.searchParams.get("code")?.trim();
  if (!code) {
    return { kind: "missing_code" };
  }

  return { kind: "code", code };
}
