import { ArrowClockwise, CheckCircle, DownloadSimple, X } from "@phosphor-icons/react";
import { useAppUpdate } from "./AppUpdateProvider";
import { appUpdateStatusCopy } from "./updatePresentation";
import "./AppUpdatePrompt.css";

export function AppUpdatePrompt() {
  const update = useAppUpdate();
  const visible =
    (update.phase === "available" && update.dismissedVersion !== update.availableVersion) ||
    update.phase === "downloading" ||
    update.phase === "installing" ||
    update.phase === "restarting" ||
    (update.phase === "error" && update.errorContext === "install");

  if (!visible) return null;

  const copy = appUpdateStatusCopy(update);
  const busy = ["downloading", "installing", "restarting"].includes(update.phase);

  return (
    <aside className="app-update-prompt" aria-live="polite" aria-label="Smalltalk software update">
      <div className="app-update-prompt-icon" aria-hidden="true">
        {update.phase === "restarting" ? <CheckCircle weight="fill" /> : <DownloadSimple />}
      </div>
      <div className="app-update-prompt-copy">
        <span>Software update</span>
        <strong>{copy.title}</strong>
        <p>{update.phase === "available" && update.notes ? update.notes : copy.detail}</p>
        {update.phase === "downloading" ? (
          <div
            className={`app-update-progress ${update.progressPercent === null ? "is-indeterminate" : ""}`}
            role="progressbar"
            aria-label="Downloading Smalltalk update"
            aria-valuemin={0}
            aria-valuemax={100}
            aria-valuenow={update.progressPercent ?? undefined}
          >
            <i style={update.progressPercent === null ? undefined : { width: `${update.progressPercent}%` }} />
          </div>
        ) : null}
        <div className="app-update-prompt-actions">
          {update.phase === "available" ? (
            <>
              <button type="button" className="app-update-later" onClick={update.dismissPrompt}>Later</button>
              <button type="button" className="app-update-install" onClick={() => void update.installUpdate()}>
                Update and restart
              </button>
            </>
          ) : null}
          {update.phase === "error" ? (
            <button type="button" className="app-update-install" onClick={() => void update.checkForUpdates(true)}>
              <ArrowClockwise aria-hidden="true" /> Try again
            </button>
          ) : null}
          {busy ? <span className="app-update-busy-label">Keep Smalltalk open</span> : null}
        </div>
      </div>
      {!busy ? (
        <button type="button" className="app-update-close" aria-label="Dismiss update" onClick={update.dismissPrompt}>
          <X aria-hidden="true" />
        </button>
      ) : null}
    </aside>
  );
}
