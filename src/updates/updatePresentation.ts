export type AppUpdatePhase =
  | "idle"
  | "checking"
  | "available"
  | "downloading"
  | "installing"
  | "restarting"
  | "up-to-date"
  | "unavailable"
  | "error";

export type UpdateErrorContext = "check" | "install" | null;

export function updateProgressPercent(downloaded: number, total: number | null): number | null {
  if (!total || total <= 0) return null;
  return Math.min(100, Math.max(0, Math.round((downloaded / total) * 100)));
}

export function appUpdateStatusCopy({
  phase,
  currentVersion,
  availableVersion,
  progressPercent,
  errorContext,
}: {
  phase: AppUpdatePhase;
  currentVersion: string | null;
  availableVersion: string | null;
  progressPercent: number | null;
  errorContext: UpdateErrorContext;
}): { title: string; detail: string } {
  const current = currentVersion ? `Smalltalk ${currentVersion}` : "This version of Smalltalk";

  if (phase === "checking") {
    return { title: "Checking for updates", detail: `${current} is checking the release feed.` };
  }
  if (phase === "available") {
    return {
      title: `Smalltalk ${availableVersion || "update"} is available`,
      detail: `You are currently using ${currentVersion || "an older version"}.`,
    };
  }
  if (phase === "downloading") {
    return {
      title: `Downloading Smalltalk ${availableVersion || "update"}`,
      detail: progressPercent === null ? "Downloading the verified update…" : `${progressPercent}% downloaded.`,
    };
  }
  if (phase === "installing") {
    return { title: "Installing update", detail: "Smalltalk is verifying and installing the downloaded update." };
  }
  if (phase === "restarting") {
    return { title: "Restarting Smalltalk", detail: "The update is installed and Smalltalk is reopening." };
  }
  if (phase === "up-to-date") {
    return { title: "Smalltalk is up to date", detail: `${current} is the newest published version.` };
  }
  if (phase === "error") {
    return errorContext === "install"
      ? { title: "Update could not be installed", detail: "Nothing was changed. Check your connection and try again." }
      : { title: "Could not check for updates", detail: "Check your connection and try again." };
  }
  if (phase === "unavailable") {
    return { title: "Updates are available in installed builds", detail: "Development previews do not install published releases." };
  }
  return { title: "Software updates", detail: `${current} checks GitHub Releases automatically.` };
}
