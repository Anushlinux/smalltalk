import React, { useEffect, useMemo, useState } from "react";
import { createRoot } from "react-dom/client";
import { BookOpenCheck, ExternalLink, Highlighter, Loader2, Play, Square, Sparkles } from "lucide-react";
import { browser } from "wxt/browser";
import type { ExtensionMessage, ExtensionResponse, ResumeCard, SessionState } from "../../src/shared/types";
import "./style.css";

type OkResponse = Extract<ExtensionResponse, { ok: true }>;

function App() {
  const [state, setState] = useState<SessionState>({
    visitCount: 0,
    eventCount: 0,
    chunkCount: 0,
    proxyReachable: false,
    proxyHasKey: false
  });
  const [busy, setBusy] = useState<"start" | "stop" | "resume" | "open" | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    void refresh();
    const id = window.setInterval(() => void refresh(false), 1800);
    return () => window.clearInterval(id);
  }, []);

  const isActive = Boolean(state.activeSession);
  const latestCard = state.latestCard;
  const savedSession = state.savedSession;
  const savedCard = state.savedCard;
  const statusText = useMemo(() => {
    if (!isActive) return "Ready";
    return "Researching";
  }, [isActive]);
  const aiStatus = useMemo(() => {
    if (!state.proxyReachable) return "AI offline";
    if (!state.proxyHasKey) return "API key missing";
    return `AI ready${state.proxyModel ? `: ${state.proxyModel}` : ""}`;
  }, [state.proxyHasKey, state.proxyModel, state.proxyReachable]);

  async function send(message: ExtensionMessage): Promise<OkResponse> {
    const response = (await browser.runtime.sendMessage(message)) as ExtensionResponse;
    if (!response.ok) throw new Error(response.error);
    return response;
  }

  async function refresh(showError = true) {
    try {
      const response = await send({ type: "GET_SESSION_STATE" });
      if (response.state) setState(response.state);
      if (showError) setError(null);
    } catch (caught) {
      if (showError) setError(caught instanceof Error ? caught.message : String(caught));
    }
  }

  async function start() {
    await run("start", async () => {
      const response = await send({ type: "START_SESSION" });
      if (response.state) setState(response.state);
      await refresh();
    });
  }

  async function stop() {
    await run("stop", async () => {
      setState((current) => ({
        ...current,
        activeSession: undefined,
        visitCount: 0,
        eventCount: 0,
        chunkCount: 0
      }));
      const response = await send({ type: "STOP_SESSION" });
      if (response.state) setState(response.state);
      await refresh();
    });
  }

  async function resume() {
    await run("resume", async () => {
      const response = await send({ type: "ANALYZE_RESUME" });
      setState((current) => ({
        ...current,
        latestCard: response.card ?? current.latestCard
      }));
      await refresh(false);
    });
  }

  async function openLatestTarget() {
    const target = latestCard?.resumeTarget;
    if (!target) return;
    await run("open", async () => {
      await send({ type: "OPEN_RESUME_TARGET", target });
    });
  }

  async function openSavedTarget() {
    const target = savedCard?.resumeTarget;
    if (!target) return;
    await run("open", async () => {
      await send({ type: "OPEN_RESUME_TARGET", target });
    });
  }

  async function run(kind: NonNullable<typeof busy>, action: () => Promise<void>) {
    setBusy(kind);
    setError(null);
    try {
      await action();
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : String(caught));
    } finally {
      setBusy(null);
    }
  }

  return (
    <main className="shell">
      <header className="topbar">
        <div className="brand">
          <BookOpenCheck size={18} strokeWidth={2.4} />
          <span>Smalltalk</span>
        </div>
        <span className={isActive ? "pill active" : "pill"}>{statusText}</span>
      </header>

      <div className={state.proxyReachable && state.proxyHasKey ? "aiStatus ready" : "aiStatus error"}>{aiStatus}</div>

      <section className="controls" aria-label="Session controls">
        {isActive ? (
          <button className="button ghost" onClick={stop} disabled={Boolean(busy)} title="Stop research">
            {busy === "stop" ? <Loader2 className="spin" size={17} /> : <Square size={17} />}
            <span>Stop</span>
          </button>
        ) : (
          <button className="button primary" onClick={start} disabled={Boolean(busy)} title="Start research">
            {busy === "start" ? <Loader2 className="spin" size={17} /> : <Play size={17} />}
            <span>Start research</span>
          </button>
        )}
        <button className="button accent" onClick={resume} disabled={Boolean(busy)} title="Resume me">
          {busy === "resume" ? <Loader2 className="spin" size={17} /> : <Sparkles size={17} />}
          <span>Resume me</span>
        </button>
      </section>

      <section className="metrics" aria-label="Capture counters">
        <Metric label="Pages" value={state.visitCount} />
        <Metric label="Signals" value={state.eventCount} />
        <Metric label="Chunks" value={state.chunkCount} />
      </section>

      {error ? <div className="notice error">{error}</div> : null}

      <ResumePanel card={latestCard} busy={busy === "open"} onOpen={openLatestTarget} />
      <SavedSessionPanel session={savedSession} card={savedCard} busy={busy === "open"} onOpen={openSavedTarget} />
    </main>
  );
}

function Metric({ label, value }: { label: string; value: number }) {
  return (
    <div className="metric">
      <strong>{value}</strong>
      <span>{label}</span>
    </div>
  );
}

function ResumePanel({ card, busy, onOpen }: { card?: ResumeCard; busy: boolean; onOpen: () => void }) {
  if (!card) {
    return (
      <section className="empty">
        <Highlighter size={24} />
        <h1>No resume card yet</h1>
        <p>Start a session, follow the research trail, then ask Smalltalk where to continue.</p>
      </section>
    );
  }

  const branchFindings = card.branchFindings?.length ? card.branchFindings : card.newKnowledge ? [card.newKnowledge] : [];
  const warnings = card.instrumentationWarnings ?? [];
  const suggestedNextMessage =
    card.suggestedNextMessage || card.summary || "Help me continue the original task from where I left off.";

  return (
    <section className="card">
      <div className="cardTitle">
        <Highlighter size={18} />
        <h1>Bring back research</h1>
      </div>
      <div className="nextPrompt">
        <span>Suggested next message</span>
        <p>{suggestedNextMessage}</p>
      </div>
      <p className="summary">{card.summary}</p>
      {branchFindings.length > 0 ? (
        <div className="findings">
          <span>Useful branch evidence</span>
          <ul>
            {branchFindings.slice(0, 4).map((finding, index) => (
              <li key={`${index}-${finding.slice(0, 12)}`}>{finding}</li>
            ))}
          </ul>
        </div>
      ) : null}
      <dl>
        <div>
          <dt>Intent</dt>
          <dd>{card.originalIntent}</dd>
        </div>
        <div>
          <dt>Detour</dt>
          <dd>{card.journeySummary}</dd>
        </div>
      </dl>
      {warnings.length > 0 ? (
        <div className="warnings">
          {warnings.slice(0, 3).map((warning) => (
            <span key={warning}>{warning}</span>
          ))}
        </div>
      ) : null}
      <button
        className="button openTarget"
        onClick={onOpen}
        disabled={busy || !card.resumeTarget}
        title={card.resumeTarget ? "Open and highlight the origin resume point" : "No origin anchor was captured"}
      >
        {busy ? <Loader2 className="spin" size={17} /> : <ExternalLink size={17} />}
        <span>{card.resumeTarget ? "Open origin anchor" : "No origin anchor"}</span>
      </button>
      <footer>
        <span>{Math.round(card.confidence * 100)}% confidence</span>
        <span>{card.resumeTarget ? safeHostname(card.resumeTarget.url) : "anchor missing"}</span>
      </footer>
    </section>
  );
}

function SavedSessionPanel({
  session,
  card,
  busy,
  onOpen
}: {
  session?: SessionState["savedSession"];
  card?: ResumeCard;
  busy: boolean;
  onOpen: () => void;
}) {
  if (!session) return null;

  return (
    <section className="savedSession">
      <div>
        <span>Last saved</span>
        <strong>{session.originTitle || session.originUrl || "Saved research session"}</strong>
        <small>
          {session.visitCount} pages · {session.eventCount} signals · {formatTime(session.stoppedAt ?? session.startedAt)}
        </small>
      </div>
      {card ? (
        <button
          className="iconButton"
          onClick={onOpen}
          disabled={busy || !card.resumeTarget}
          title={card.resumeTarget ? "Open saved resume point" : "Saved card has no origin anchor"}
        >
          {busy ? <Loader2 className="spin" size={16} /> : <ExternalLink size={16} />}
        </button>
      ) : null}
    </section>
  );
}

function safeHostname(url: string): string {
  try {
    return new URL(url).hostname;
  } catch {
    return url || "unknown target";
  }
}

function formatTime(timestamp: number): string {
  return new Intl.DateTimeFormat(undefined, {
    hour: "numeric",
    minute: "2-digit"
  }).format(new Date(timestamp));
}

createRoot(document.getElementById("root")!).render(<App />);
