import { useCallback, useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { openPath } from "@tauri-apps/plugin-opener";
import "./App.css";

type SessionCounts = {
  frames: number;
  events: number;
  triggers: number;
  transitions: number;
  content_units: number;
  ax_nodes: number;
  ocr_text_rows: number;
  ocr_spans: number;
  app_contexts: number;
  window_snapshots: number;
  windows: number;
  frame_diffs: number;
  clipboard_events: number;
  typing_bursts: number;
  presence_samples: number;
  sensitive_regions: number;
};

type CaptureSession = {
  id: string;
  sequence: number;
  started_at: number;
  stopped_at?: number | null;
  status: string;
  export_path?: string | null;
  counts: SessionCounts;
};

type MemoryCounts = {
  saved_states: number;
  visual_proofs: number;
  skipped_screenshots: number;
  work_items: number;
  log_entries: number;
};

type WorkStateSummary = {
  id: number;
  session_id?: string | null;
  ts_ms: number;
  app_name?: string | null;
  window_name?: string | null;
  browser_url?: string | null;
  document_path?: string | null;
  work_key: string;
  work_type: string;
  activity: string;
  privacy: string;
  confidence: number;
  screenshot_decision: string;
  screenshot_reason: string;
  visual_proof_frame_id?: number | null;
};

type ResumeEvidenceScores = {
  location: number;
  content: number;
  action: number;
  progress: number;
  unresolved: number;
  reopenability: number;
  visual_proof: number;
};

type MemoryResumeCard = {
  headline: string;
  detail: string;
  continue_label: string;
  confidence: number;
  evidence: string[];
  missing: string[];
  work_state_id?: number | null;
  frame_id?: number | null;
  evidence_scores?: ResumeEvidenceScores | null;
  missing_signals: string[];
};

type MemoryStatus = {
  running: boolean;
  mode: string;
  active_session?: CaptureSession | null;
  latest_work?: WorkStateSummary | null;
  resume_card: MemoryResumeCard;
  counts: MemoryCounts;
  recent_work: WorkStateSummary[];
  last_error?: string | null;
  data_dir: string;
  database_path: string;
};

type WorkTrail = {
  items: WorkStateSummary[];
  counts: MemoryCounts;
};

type TestLogEntry = {
  id: number;
  ts_ms: number;
  session_id?: string | null;
  level: string;
  event: string;
  app_name?: string | null;
  work_key?: string | null;
  message: string;
  detail_json?: string | null;
};

type BusyAction =
  | "start"
  | "pause"
  | "screenshot"
  | "resume"
  | "export"
  | null;

const emptyCounts: MemoryCounts = {
  saved_states: 0,
  visual_proofs: 0,
  skipped_screenshots: 0,
  work_items: 0,
  log_entries: 0,
};

const emptyResume: MemoryResumeCard = {
  headline: "Not enough evidence yet",
  detail: "Turn Memory On and keep working. Smalltalk will save lightweight work states before it saves screenshots.",
  continue_label: "No return target yet",
  confidence: 0,
  evidence: [],
  missing: ["No saved work state yet"],
  work_state_id: null,
  frame_id: null,
  evidence_scores: null,
  missing_signals: [],
};

const initialStatus: MemoryStatus = {
  running: false,
  mode: "Paused",
  active_session: null,
  latest_work: null,
  resume_card: emptyResume,
  counts: emptyCounts,
  recent_work: [],
  last_error: null,
  data_dir: "",
  database_path: "",
};

export default function App() {
  const [status, setStatus] = useState<MemoryStatus>(initialStatus);
  const [trail, setTrail] = useState<WorkStateSummary[]>([]);
  const [logs, setLogs] = useState<TestLogEntry[]>([]);
  const [busy, setBusy] = useState<BusyAction>(null);
  const [error, setError] = useState<string | null>(null);
  const [showLogs, setShowLogs] = useState(false);
  const [logFilter, setLogFilter] = useState("");
  const [lastExport, setLastExport] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    const [nextStatus, nextTrail, nextLogs] = await Promise.all([
      invoke<MemoryStatus>("get_memory_status"),
      invoke<WorkTrail>("get_work_trail", { limit: 40 }),
      invoke<TestLogEntry[]>("get_test_logs", { limit: 120 }),
    ]);
    setStatus(nextStatus);
    setTrail(nextTrail.items);
    setLogs(nextLogs);
  }, []);

  useEffect(() => {
    void refresh().catch((reason: unknown) => {
      setError(errorMessage(reason));
    });
    const id = window.setInterval(() => {
      void refresh().catch(() => undefined);
    }, 2500);
    return () => window.clearInterval(id);
  }, [refresh]);

  const run = useCallback(
    async (action: BusyAction, task: () => Promise<void>) => {
      setBusy(action);
      setError(null);
      try {
        await task();
        await refresh();
      } catch (reason) {
        setError(errorMessage(reason));
      } finally {
        setBusy(null);
      }
    },
    [refresh],
  );

  const startMemory = () =>
    run("start", async () => {
      setStatus(await invoke<MemoryStatus>("start_memory"));
    });

  const pauseMemory = () =>
    run("pause", async () => {
      setStatus(await invoke<MemoryStatus>("pause_memory"));
    });

  const saveScreenshot = () =>
    run("screenshot", async () => {
      setStatus(await invoke<MemoryStatus>("save_screenshot"));
    });

  const resumeMe = () =>
    run("resume", async () => {
      const resume = await invoke<MemoryResumeCard>("get_resume_card");
      setStatus((current) => ({ ...current, resume_card: resume }));
    });

  const exportLogs = () =>
    run("export", async () => {
      const path = await invoke<string>("export_test_bundle");
      setLastExport(path);
      await openPath(path);
    });

  const mode = status.latest_work?.privacy && status.latest_work.privacy !== "normal"
    ? "Private"
    : status.running
      ? "Memory On"
      : "Paused";
  const resume = status.resume_card || emptyResume;
  const latestWork = status.latest_work;

  const filteredLogs = useMemo(() => {
    const query = logFilter.trim().toLowerCase();
    if (!query) {
      return logs;
    }
    return logs.filter((entry) =>
      [
        entry.level,
        entry.event,
        entry.app_name,
        entry.work_key,
        entry.message,
        entry.detail_json,
      ]
        .filter(Boolean)
        .join(" ")
        .toLowerCase()
        .includes(query),
    );
  }, [logFilter, logs]);

  return (
    <main className="memory-shell">
      <header className="memory-topbar">
        <div className="brand-lockup">
          <div className="brand-mark" aria-hidden="true">S</div>
          <div>
            <span className="muted-label">Smalltalk</span>
            <h1>Smalltalk Memory</h1>
          </div>
        </div>

        <div className={`mode-pill ${toneForMode(mode)}`}>
          <span aria-hidden="true" />
          {mode}
        </div>

        <div className="top-actions">
          <button
            className="primary-button"
            type="button"
            disabled={status.running || busy !== null}
            aria-busy={busy === "start"}
            onClick={() => void startMemory()}
          >
            {busy === "start" ? "Starting" : "Memory On"}
          </button>
          <button
            className="secondary-button"
            type="button"
            disabled={!status.running || busy !== null}
            aria-busy={busy === "pause"}
            onClick={() => void pauseMemory()}
          >
            {busy === "pause" ? "Pausing" : "Pause"}
          </button>
          <button
            className="secondary-button"
            type="button"
            disabled={busy !== null}
            aria-busy={busy === "screenshot"}
            onClick={() => void saveScreenshot()}
          >
            {busy === "screenshot" ? "Saving" : "Save screenshot"}
          </button>
          <button
            className="ghost-button"
            type="button"
            onClick={() => setShowLogs((value) => !value)}
          >
            Test Log
          </button>
        </div>
      </header>

      {error || status.last_error ? (
        <div className="notice error" role="alert">{error || status.last_error}</div>
      ) : null}

      <section className="resume-panel" aria-label="Resume card">
        <div className="resume-copy">
          <span className="muted-label">What you were doing</span>
          <h2>{resume.headline}</h2>
          <p>{resume.detail}</p>
        </div>
        <div className="continue-panel">
          <span className="muted-label">Continue here</span>
          <strong>{resume.continue_label}</strong>
          <div className="confidence-line">
            <span>{confidenceLabel(resume.confidence)}</span>
            <meter min={0} max={1} value={resume.confidence} />
          </div>
          <button
            className="primary-button wide"
            type="button"
            disabled={busy !== null}
            aria-busy={busy === "resume"}
            onClick={() => void resumeMe()}
          >
            {busy === "resume" ? "Reading memory" : "Resume me"}
          </button>
        </div>
      </section>

      <section className="current-strip" aria-label="Current work">
        <WorkFact label="App" value={latestWork?.app_name || "No app yet"} />
        <WorkFact label="Page or file" value={workLocation(latestWork)} />
        <WorkFact label="Activity" value={simpleActivity(latestWork)} />
        <WorkFact label="Confidence" value={confidenceLabel(latestWork?.confidence || 0)} />
      </section>

      <section className="proof-strip" aria-label="Memory counts">
        <Metric label="Saved states" value={status.counts.saved_states} />
        <Metric label="Visual proof" value={status.counts.visual_proofs} />
        <Metric label="Skipped screenshots" value={status.counts.skipped_screenshots} />
        <Metric label="Work items" value={status.counts.work_items} />
      </section>

      <div className="content-grid">
        <section className="work-trail" aria-label="Work trail">
          <div className="section-heading">
            <div>
              <span className="muted-label">Work trail</span>
              <h2>Recent work changes</h2>
            </div>
            <span>{trail.length} states</span>
          </div>

          {trail.length ? (
            <div className="trail-list">
              {trail.map((item) => (
                <article className="trail-row" key={item.id}>
                  <div className="trail-time">
                    <strong>{formatTime(item.ts_ms)}</strong>
                    <span>{item.activity}</span>
                  </div>
                  <div className="trail-main">
                    <h3>{item.app_name || "Unknown app"}</h3>
                    <p>{workLocation(item)}</p>
                    <small>{item.screenshot_decision} · {item.screenshot_reason}</small>
                  </div>
                  <span className={`privacy-tag ${item.privacy === "normal" ? "ok" : "private"}`}>
                    {item.privacy === "normal" ? "Saved state" : "Private"}
                  </span>
                </article>
              ))}
            </div>
          ) : (
            <div className="empty-state">
              <strong>No work states yet</strong>
              <p>Turn Memory On. Clicks, scrolls, typing pauses, app switches, and idle moments will be saved as lightweight work states first.</p>
            </div>
          )}
        </section>

        <aside className="evidence-panel" aria-label="Evidence">
          <div className="section-heading compact">
            <div>
              <span className="muted-label">Evidence</span>
              <h2>What Smalltalk knows</h2>
            </div>
          </div>
          {resume.evidence_scores ? (
            <div className="evidence-scores">
              <span className="muted-label">Evidence sufficiency</span>
              <ScoreBar label="Location" value={resume.evidence_scores.location} />
              <ScoreBar label="Content" value={resume.evidence_scores.content} />
              <ScoreBar label="Action" value={resume.evidence_scores.action} />
              <ScoreBar label="Progress" value={resume.evidence_scores.progress} />
              <ScoreBar label="Unresolved" value={resume.evidence_scores.unresolved} />
              <ScoreBar label="Reopenability" value={resume.evidence_scores.reopenability} />
              <ScoreBar label="Visual proof" value={resume.evidence_scores.visual_proof} />
            </div>
          ) : null}
          {latestWork ? (
            <div className="path-box">
              <span>Latest capture decision</span>
              <code>{latestWork.screenshot_decision} · {latestWork.screenshot_reason}</code>
            </div>
          ) : null}
          <EvidenceList title="Evidence" items={resume.evidence} empty="No evidence yet" />
          <EvidenceList title="Missing" items={resume.missing} empty="Nothing missing" />
          {resume.missing_signals.length ? (
            <EvidenceList
              title="Missing signals"
              items={resume.missing_signals}
              empty="None"
            />
          ) : null}
          <div className="path-box">
            <span>Data</span>
            <code>{status.data_dir || "Not loaded"}</code>
          </div>
        </aside>
      </div>

      {showLogs ? (
        <section className="test-log" aria-label="Test Log">
          <div className="section-heading">
            <div>
              <span className="muted-label">Test Log</span>
              <h2>Why screenshots were saved or skipped</h2>
            </div>
            <div className="log-actions">
              <input
                value={logFilter}
                onChange={(event) => setLogFilter(event.currentTarget.value)}
                placeholder="Filter app, event, privacy, reason"
                aria-label="Filter test log"
              />
              <button
                className="secondary-button"
                type="button"
                disabled={busy !== null}
                aria-busy={busy === "export"}
                onClick={() => void exportLogs()}
              >
                {busy === "export" ? "Exporting" : "Export test bundle"}
              </button>
            </div>
          </div>

          {lastExport ? (
            <div className="notice">Last export: <code>{lastExport}</code></div>
          ) : null}

          <div className="log-table">
            {filteredLogs.length ? (
              filteredLogs.map((entry) => (
                <article className="log-row" key={entry.id}>
                  <time>{formatTime(entry.ts_ms)}</time>
                  <strong className={entry.level}>{entry.event}</strong>
                  <span>{entry.app_name || "system"}</span>
                  <p>{entry.message}</p>
                  <code>{compactJson(entry.detail_json)}</code>
                </article>
              ))
            ) : (
              <div className="empty-state small">
                <strong>No matching log entries</strong>
                <p>Try a broader filter or keep Memory On while testing.</p>
              </div>
            )}
          </div>
        </section>
      ) : null}
    </main>
  );
}

function Metric({ label, value }: { label: string; value: number }) {
  return (
    <div className="metric">
      <span>{label}</span>
      <strong>{value.toLocaleString()}</strong>
    </div>
  );
}

function ScoreBar({ label, value }: { label: string; value: number }) {
  return (
    <div className="score-bar">
      <span>{label}</span>
      <meter min={0} max={1} value={value} />
      <strong>{value.toFixed(2)}</strong>
    </div>
  );
}

function WorkFact({ label, value }: { label: string; value: string }) {
  return (
    <div className="work-fact">
      <span>{label}</span>
      <strong title={value}>{value}</strong>
    </div>
  );
}

function EvidenceList({
  title,
  items,
  empty,
}: {
  title: string;
  items: string[];
  empty: string;
}) {
  return (
    <div className="evidence-list">
      <span>{title}</span>
      {items.length ? (
        items.map((item) => <p key={item}>{item}</p>)
      ) : (
        <p className="quiet">{empty}</p>
      )}
    </div>
  );
}

function toneForMode(mode: string) {
  if (mode === "Memory On") return "on";
  if (mode === "Private") return "private";
  return "paused";
}

function simpleActivity(item?: WorkStateSummary | null) {
  if (!item) return "Waiting";
  return `${item.activity} · ${item.work_type}`;
}

function workLocation(item?: WorkStateSummary | null) {
  if (!item) return "No page or file yet";
  return (
    item.document_path ||
    item.browser_url ||
    item.window_name ||
    item.work_key ||
    "Unknown location"
  );
}

function confidenceLabel(value: number) {
  if (value >= 0.75) return "High confidence";
  if (value >= 0.45) return "Medium confidence";
  if (value > 0) return "Low confidence";
  return "Not enough evidence";
}

function formatTime(value?: number | null) {
  if (!value) return "now";
  return new Date(value).toLocaleTimeString([], {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  });
}

function compactJson(value?: string | null) {
  if (!value) return "";
  try {
    const parsed = JSON.parse(value) as unknown;
    return JSON.stringify(parsed);
  } catch {
    return value;
  }
}

function errorMessage(reason: unknown) {
  if (reason instanceof Error) return reason.message;
  if (typeof reason === "string") return reason;
  return "Something went wrong";
}
