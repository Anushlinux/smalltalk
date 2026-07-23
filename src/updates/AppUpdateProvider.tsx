import { getVersion } from "@tauri-apps/api/app";
import { relaunch } from "@tauri-apps/plugin-process";
import { check, type Update } from "@tauri-apps/plugin-updater";
import {
  createContext,
  type ReactNode,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import {
  type AppUpdatePhase,
  type UpdateErrorContext,
  updateProgressPercent,
} from "./updatePresentation";

const AUTOMATIC_CHECK_DELAY_MS = 4_000;
const AUTOMATIC_CHECK_INTERVAL_MS = 6 * 60 * 60 * 1_000;

type AppUpdateSnapshot = {
  phase: AppUpdatePhase;
  currentVersion: string | null;
  availableVersion: string | null;
  notes: string | null;
  progressPercent: number | null;
  errorContext: UpdateErrorContext;
  errorMessage: string | null;
  lastCheckedAt: number | null;
  dismissedVersion: string | null;
};

type AppUpdateContextValue = AppUpdateSnapshot & {
  checkForUpdates: (manual?: boolean) => Promise<void>;
  installUpdate: () => Promise<void>;
  dismissPrompt: () => void;
};

const initialSnapshot: AppUpdateSnapshot = {
  phase: "idle",
  currentVersion: null,
  availableVersion: null,
  notes: null,
  progressPercent: null,
  errorContext: null,
  errorMessage: null,
  lastCheckedAt: null,
  dismissedVersion: null,
};

const AppUpdateContext = createContext<AppUpdateContextValue | null>(null);

function isTauriRuntime(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

export function AppUpdateProvider({ children }: { children: ReactNode }) {
  const [snapshot, setSnapshot] = useState<AppUpdateSnapshot>(initialSnapshot);
  const pendingUpdateRef = useRef<Update | null>(null);
  const checkInFlightRef = useRef<Promise<void> | null>(null);

  const replacePendingUpdate = useCallback((next: Update | null) => {
    const previous = pendingUpdateRef.current;
    pendingUpdateRef.current = next;
    if (previous && previous !== next) {
      void previous.close().catch(() => undefined);
    }
  }, []);

  const checkForUpdates = useCallback(async (manual = true) => {
    if (checkInFlightRef.current) return checkInFlightRef.current;

    const task = (async () => {
      if (!isTauriRuntime() || import.meta.env.DEV) {
        setSnapshot((current) => ({
          ...current,
          phase: "unavailable",
          errorContext: null,
          errorMessage: null,
        }));
        return;
      }

      setSnapshot((current) => ({
        ...current,
        phase: "checking",
        errorContext: null,
        errorMessage: null,
      }));

      try {
        const currentVersion = await getVersion();
        const update = await check({ timeout: 15_000 });
        const checkedAt = Date.now();

        if (!update) {
          replacePendingUpdate(null);
          setSnapshot((current) => ({
            ...current,
            phase: "up-to-date",
            currentVersion,
            availableVersion: null,
            notes: null,
            progressPercent: null,
            errorContext: null,
            errorMessage: null,
            lastCheckedAt: checkedAt,
          }));
          return;
        }

        replacePendingUpdate(update);
        setSnapshot((current) => ({
          ...current,
          phase: "available",
          currentVersion: update.currentVersion || currentVersion,
          availableVersion: update.version,
          notes: update.body?.trim() || null,
          progressPercent: null,
          errorContext: null,
          errorMessage: null,
          lastCheckedAt: checkedAt,
          dismissedVersion:
            current.dismissedVersion === update.version ? current.dismissedVersion : null,
        }));
      } catch (error) {
        replacePendingUpdate(null);
        setSnapshot((current) => ({
          ...current,
          phase: "error",
          errorContext: "check",
          errorMessage: String(error),
          lastCheckedAt: manual ? Date.now() : current.lastCheckedAt,
        }));
      }
    })();

    checkInFlightRef.current = task;
    try {
      await task;
    } finally {
      checkInFlightRef.current = null;
    }
  }, [replacePendingUpdate]);

  const installUpdate = useCallback(async () => {
    const update = pendingUpdateRef.current;
    if (!update) {
      await checkForUpdates(true);
      return;
    }

    let downloaded = 0;
    let contentLength: number | null = null;
    setSnapshot((current) => ({
      ...current,
      phase: "downloading",
      progressPercent: null,
      errorContext: null,
      errorMessage: null,
    }));

    try {
      await update.downloadAndInstall((event) => {
        if (event.event === "Started") {
          downloaded = 0;
          contentLength = event.data.contentLength ?? null;
          setSnapshot((current) => ({ ...current, progressPercent: 0 }));
          return;
        }
        if (event.event === "Progress") {
          downloaded += event.data.chunkLength;
          setSnapshot((current) => ({
            ...current,
            phase: "downloading",
            progressPercent: updateProgressPercent(downloaded, contentLength),
          }));
          return;
        }
        setSnapshot((current) => ({
          ...current,
          phase: "installing",
          progressPercent: 100,
        }));
      });

      setSnapshot((current) => ({ ...current, phase: "restarting", progressPercent: 100 }));
      await relaunch();
    } catch (error) {
      replacePendingUpdate(null);
      setSnapshot((current) => ({
        ...current,
        phase: "error",
        progressPercent: null,
        errorContext: "install",
        errorMessage: String(error),
      }));
    }
  }, [checkForUpdates, replacePendingUpdate]);

  const dismissPrompt = useCallback(() => {
    setSnapshot((current) => ({
      ...current,
      dismissedVersion: current.availableVersion,
    }));
  }, []);

  useEffect(() => {
    const initialCheck = window.setTimeout(() => void checkForUpdates(false), AUTOMATIC_CHECK_DELAY_MS);
    const interval = window.setInterval(
      () => void checkForUpdates(false),
      AUTOMATIC_CHECK_INTERVAL_MS,
    );
    return () => {
      window.clearTimeout(initialCheck);
      window.clearInterval(interval);
      replacePendingUpdate(null);
    };
  }, [checkForUpdates, replacePendingUpdate]);

  const value = useMemo<AppUpdateContextValue>(() => ({
    ...snapshot,
    checkForUpdates,
    installUpdate,
    dismissPrompt,
  }), [checkForUpdates, dismissPrompt, installUpdate, snapshot]);

  return <AppUpdateContext.Provider value={value}>{children}</AppUpdateContext.Provider>;
}

export function useAppUpdate(): AppUpdateContextValue {
  const value = useContext(AppUpdateContext);
  if (!value) throw new Error("useAppUpdate must be used inside AppUpdateProvider");
  return value;
}
