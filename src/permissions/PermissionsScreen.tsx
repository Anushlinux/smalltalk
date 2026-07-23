import { useCallback, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  ArrowSquareOut,
  Check,
  CheckCircle,
  CursorClick,
  Eye,
  Keyboard,
  SignOut,
} from "@phosphor-icons/react";
import smalltalkLogo from "../assets/smalltalk-logo.png";
import "./PermissionsScreen.css";

type PermissionKey = "screen_recording" | "accessibility" | "input_monitoring";

type RequiredPermissionStatus = {
  key: PermissionKey;
  granted: boolean;
  can_request: boolean;
  restart_required: boolean;
};

type AppPermissionsStatus = {
  permissions: RequiredPermissionStatus[];
  all_granted: boolean;
};

type PermissionDefinition = {
  key: PermissionKey;
  title: string;
  description: string;
  privacyNote: string;
  icon: ReactNode;
};

const permissionDefinitions: PermissionDefinition[] = [
  {
    key: "screen_recording",
    title: "Screen Recording",
    description: "Lets Smalltalk see the apps and windows you work in.",
    privacyNote: "Screen evidence stays on this Mac.",
    icon: <Eye weight="regular" aria-hidden="true" />,
  },
  {
    key: "accessibility",
    title: "Accessibility",
    description: "Lets Smalltalk read visible interface text and understand the task in front of you.",
    privacyNote: "This avoids relying on screenshots alone.",
    icon: <CursorClick weight="regular" aria-hidden="true" />,
  },
  {
    key: "input_monitoring",
    title: "Input Monitoring",
    description: "Lets Smalltalk notice clicks, scrolling, and broad key activity.",
    privacyNote: "Smalltalk never stores what you type.",
    icon: <Keyboard weight="regular" aria-hidden="true" />,
  },
];

const EMPTY_STATUS: AppPermissionsStatus = {
  permissions: [],
  all_granted: false,
};

function permissionByKey(status: AppPermissionsStatus, key: PermissionKey) {
  return status.permissions.find((permission) => permission.key === key);
}

export function PermissionsGate({
  accountEmail,
  onSignOut,
  children,
}: {
  accountEmail: string;
  onSignOut: () => void;
  children: ReactNode;
}) {
  const [status, setStatus] = useState<AppPermissionsStatus | null>(null);
  const [setupComplete, setSetupComplete] = useState(false);
  const [busyPermission, setBusyPermission] = useState<PermissionKey | "refresh" | null>(null);
  const [requestedPermissions, setRequestedPermissions] = useState<Set<PermissionKey>>(new Set());
  const [error, setError] = useState<string | null>(null);
  const mountedRef = useRef(true);
  const setupWasRequiredRef = useRef(false);

  const refresh = useCallback(async (quiet = false) => {
    if (!quiet) setBusyPermission("refresh");
    try {
      const nextStatus = await invoke<AppPermissionsStatus>("get_app_permissions_status");
      if (!mountedRef.current) return;
      if (!nextStatus.all_granted) setupWasRequiredRef.current = true;
      if (nextStatus.all_granted && !setupWasRequiredRef.current) setSetupComplete(true);
      setStatus(nextStatus);
      setError(null);
    } catch (nextError) {
      if (!mountedRef.current) return;
      setError(`Smalltalk could not check macOS permissions. ${String(nextError)}`);
      setStatus((current) => current || EMPTY_STATUS);
    } finally {
      if (mountedRef.current && !quiet) setBusyPermission(null);
    }
  }, []);

  useEffect(() => {
    mountedRef.current = true;
    void refresh();
    return () => {
      mountedRef.current = false;
    };
  }, [refresh]);

  useEffect(() => {
    if (!status || status.all_granted) return;
    const refreshWhenVisible = () => {
      if (document.visibilityState === "visible") void refresh(true);
    };
    const interval = window.setInterval(refreshWhenVisible, 1600);
    window.addEventListener("focus", refreshWhenVisible);
    document.addEventListener("visibilitychange", refreshWhenVisible);
    return () => {
      window.clearInterval(interval);
      window.removeEventListener("focus", refreshWhenVisible);
      document.removeEventListener("visibilitychange", refreshWhenVisible);
    };
  }, [refresh, status]);

  useEffect(() => {
    if (!status?.all_granted || setupComplete || !setupWasRequiredRef.current) return;
    const completionDelay = window.setTimeout(() => setSetupComplete(true), 900);
    return () => window.clearTimeout(completionDelay);
  }, [setupComplete, status]);

  const allowPermission = useCallback(async (permission: PermissionKey) => {
    setBusyPermission(permission);
    setError(null);
    try {
      const nextStatus = await invoke<AppPermissionsStatus>("request_app_permission", {
        permission,
      });
      setRequestedPermissions((current) => new Set(current).add(permission));
      setStatus(nextStatus);
    } catch (nextError) {
      setError(`macOS could not open the ${permission.replace("_", " ")} request. ${String(nextError)}`);
    } finally {
      setBusyPermission(null);
    }
  }, []);

  const openSettings = useCallback(async (permission: PermissionKey) => {
    setBusyPermission(permission);
    setError(null);
    try {
      await invoke("open_app_permission_settings", { permission });
      setRequestedPermissions((current) => new Set(current).add(permission));
    } catch (nextError) {
      setError(`System Settings could not be opened. ${String(nextError)}`);
    } finally {
      setBusyPermission(null);
    }
  }, []);

  const readyCount = useMemo(
    () => status?.permissions.filter((permission) => permission.granted).length || 0,
    [status],
  );

  if (setupComplete) return children;

  if (!status && !error) {
    return (
      <main className="permissions-screen permissions-checking" aria-label="Checking macOS permissions">
        <div>
          <img src={smalltalkLogo} alt="" />
          <span className="permissions-pulse" aria-hidden="true" />
          <p>Checking permissions…</p>
        </div>
      </main>
    );
  }

  return (
    <main className="permissions-screen">
      <section className="permissions-shell" aria-labelledby="permissions-title">
        <header className="permissions-topbar">
          <div className="permissions-brand">
            <img src={smalltalkLogo} alt="" />
            <span>Smalltalk</span>
          </div>
          <button className="permissions-account" type="button" onClick={onSignOut}>
            <span>{accountEmail}</span>
            <SignOut aria-hidden="true" />
            <span className="sr-only">Sign out</span>
          </button>
        </header>

        <div className="permissions-intro">
          <div>
            <h1 id="permissions-title">Give Smalltalk the context it needs</h1>
            <p>
              These three macOS permissions let local memory understand your work and help you
              continue later. Setup usually takes less than a minute.
            </p>
          </div>
          <div className="permissions-progress" aria-label={`${readyCount} of 3 permissions ready`}>
            <strong>{readyCount} of 3 ready</strong>
            <span aria-hidden="true">
              <i style={{ width: `${(readyCount / 3) * 100}%` }} />
            </span>
          </div>
        </div>

        <div className="permissions-list">
          {permissionDefinitions.map((definition) => {
            const permission = status ? permissionByKey(status, definition.key) : undefined;
            const granted = permission?.granted === true;
            const waitingForSettings = requestedPermissions.has(definition.key);
            const needsSettings = permission?.restart_required || waitingForSettings;
            const busy = busyPermission === definition.key;
            return (
              <article
                className={`permission-row ${granted ? "is-granted" : "is-needed"}`}
                key={definition.key}
              >
                <div className="permission-icon">{granted ? <Check aria-hidden="true" /> : definition.icon}</div>
                <div className="permission-copy">
                  <div className="permission-title-line">
                    <h2>{definition.title}</h2>
                    <span className={`permission-state ${granted ? "is-granted" : "is-needed"}`}>
                      {granted ? "Allowed" : "Required"}
                    </span>
                  </div>
                  <p>{definition.description}</p>
                  <small>{definition.privacyNote}</small>
                  {permission?.restart_required ? (
                    <small className="permission-restart-note">
                      After enabling it, quit and reopen Smalltalk once so macOS applies the change.
                    </small>
                  ) : null}
                </div>
                <div className="permission-actions">
                  {granted ? (
                    <span className="permission-confirmation">
                      <CheckCircle weight="fill" aria-hidden="true" /> Ready
                    </span>
                  ) : (
                    <>
                      <button
                        className="permission-primary-action"
                        type="button"
                        disabled={busyPermission !== null}
                        aria-busy={busy}
                        onClick={() => {
                          if (needsSettings) {
                            void openSettings(definition.key);
                          } else {
                            void allowPermission(definition.key);
                          }
                        }}
                      >
                        {busy ? "Opening…" : needsSettings ? "Open Settings" : "Allow"}
                      </button>
                      {!needsSettings ? (
                        <button
                          className="permission-settings-link"
                          type="button"
                          disabled={busyPermission !== null}
                          onClick={() => void openSettings(definition.key)}
                        >
                          Settings <ArrowSquareOut aria-hidden="true" />
                        </button>
                      ) : null}
                    </>
                  )}
                </div>
              </article>
            );
          })}
        </div>

        <footer className={`permissions-footer ${status?.all_granted ? "is-complete" : ""}`}>
          <div className="permissions-auto-check" role={status?.all_granted ? "status" : undefined}>
            {status?.all_granted ? (
              <CheckCircle className="permissions-ready-icon" weight="fill" aria-hidden="true" />
            ) : (
              <span className="permissions-pulse" aria-hidden="true" />
            )}
            <p>
              <strong>{status?.all_granted ? "All permissions ready" : "Checking automatically"}</strong>
              <span>
                {status?.all_granted
                  ? "Opening Smalltalk…"
                  : "Return here after each macOS prompt. Green means the permission is active."}
              </span>
            </p>
          </div>
          {!status?.all_granted ? (
            <button
              className="permissions-refresh"
              type="button"
              disabled={busyPermission !== null}
              aria-busy={busyPermission === "refresh"}
              onClick={() => void refresh()}
            >
              {busyPermission === "refresh" ? "Checking…" : "Check again"}
            </button>
          ) : null}
        </footer>

        {error ? <p className="permissions-error" role="alert">{error}</p> : null}
      </section>
    </main>
  );
}
