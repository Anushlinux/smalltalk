import React, { useEffect, useMemo, useState } from "react";
import { createRoot } from "react-dom/client";
import { BookOpenCheck, Highlighter, Loader2, Play, Square, Sparkles } from "lucide-react";
import { browser } from "wxt/browser";
import type { ExtensionMessage, ExtensionResponse, ResumeCard, SessionState } from "../../src/shared/types";
import "./style.css";

type OkResponse = Extract<ExtensionResponse, { ok: true }>;

function App() {
  const [state, setState] = useState<SessionState>({ visitCount: 0, eventCount: 0, chunkCount: 0 });
  const [busy, setBusy] = useState<"start" | "stop" | "resume" | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    void refresh();
    const id = window.setInterval(() => void refresh(false), 1800);
    return () => window.clearInterval(id);
  }, []);

  const isActive = Boolean(state.activeSession);
  const latestCard = state.latestCard;
  const statusText = useMemo(() => {
    if (!isActive) return latestCard ? "Session paused" : "Ready";
    return "Researching";
  }, [isActive, latestCard]);

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

      <ResumePanel card={latestCard} />
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

function ResumePanel({ card }: { card?: ResumeCard }) {
  if (!card) {
    return (
      <section className="empty">
        <Highlighter size={24} />
        <h1>No resume card yet</h1>
        <p>Start a session, follow the research trail, then ask Smalltalk where to continue.</p>
      </section>
    );
  }

  return (
    <section className="card">
      <div className="cardTitle">
        <Highlighter size={18} />
        <h1>Continue here</h1>
      </div>
      <blockquote>{card.resumeTarget.textQuote}</blockquote>
      <p className="summary">{card.summary}</p>
      <dl>
        <div>
          <dt>Intent</dt>
          <dd>{card.originalIntent}</dd>
        </div>
        <div>
          <dt>Detour</dt>
          <dd>{card.journeySummary}</dd>
        </div>
        <div>
          <dt>Now known</dt>
          <dd>{card.newKnowledge}</dd>
        </div>
      </dl>
      <footer>
        <span>{Math.round(card.confidence * 100)}% confidence</span>
        <span>{new URL(card.resumeTarget.url).hostname}</span>
      </footer>
    </section>
  );
}

createRoot(document.getElementById("root")!).render(<App />);
