export const CONTINUE_REQUEST_TIMEOUT_MS = 120_000;

export class ContinueRequestTimeoutError extends Error {
  constructor(timeoutMs: number) {
    super(`Continue exceeded its ${Math.round(timeoutMs / 1_000)} second safety limit.`);
    this.name = "ContinueRequestTimeoutError";
  }
}

export async function withContinueRequestTimeout<T>(
  request: Promise<T>,
  timeoutMs = CONTINUE_REQUEST_TIMEOUT_MS,
): Promise<T> {
  let timeoutId: ReturnType<typeof setTimeout> | null = null;
  const timeout = new Promise<never>((_, reject) => {
    timeoutId = setTimeout(() => {
      reject(new ContinueRequestTimeoutError(timeoutMs));
    }, timeoutMs);
  });

  try {
    return await Promise.race([request, timeout]);
  } finally {
    if (timeoutId !== null) {
      clearTimeout(timeoutId);
    }
  }
}

export function isContinueRequestTimeout(error: unknown): boolean {
  return error instanceof ContinueRequestTimeoutError;
}

export function continueRequestErrorCopy(error: unknown): string {
  if (isContinueRequestTimeout(error)) {
    return "Continue stopped because capture or inference exceeded the two-minute safety limit. The previous answer is still available; you can try again.";
  }
  if (String(error).includes("workload governor timed out")) {
    return "Continue could not start because an earlier capture or refresh was still finishing. The previous answer is still available; please try again.";
  }
  return `Continue failed: ${String(error)}`;
}
