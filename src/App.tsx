import { useCallback, useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";

type CaptureFrame = {
  id: number;
  captured_at: number;
  snapshot_path: string;
  app_name?: string | null;
  window_name?: string | null;
  browser_url?: string | null;
  document_path?: string | null;
  focused: boolean;
  capture_trigger: string;
  text_source?: string | null;
  accessibility_text?: string | null;
  accessibility_tree_json?: string | null;
  full_text?: string | null;
  content_hash?: string | null;
  image_hash?: string | null;
};

type CaptureStatus = {
  running: boolean;
  frame_count: number;
  started_at?: number | null;
  last_error?: string | null;
  latest_frame?: CaptureFrame | null;
  data_dir: string;
  database_path: string;
  screenshot_tool: boolean;
  accessibility_tool: boolean;
  ocr_tool: boolean;
};

type SearchResult = {
  frame: CaptureFrame;
  snippet: string;
  rank: number;
};

const initialStatus: CaptureStatus = {
  running: false,
  frame_count: 0,
  data_dir: "",
  database_path: "",
  screenshot_tool: false,
  accessibility_tool: false,
  ocr_tool: false,
};

function App() {
  const [status, setStatus] = useState<CaptureStatus>(initialStatus);
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<SearchResult[]>([]);
  const [selectedFrame, setSelectedFrame] = useState<CaptureFrame | null>(null);
  const [imageData, setImageData] = useState<string | null>(null);
  const [busyAction, setBusyAction] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const refreshStatus = useCallback(async () => {
    try {
      const nextStatus = await invoke<CaptureStatus>("capture_status");
      setStatus(nextStatus);
      setError(null);
      if (!selectedFrame && nextStatus.latest_frame) {
        setSelectedFrame(nextStatus.latest_frame);
      }
    } catch (err) {
      setError(String(err));
    }
  }, [selectedFrame]);

  const runSearch = useCallback(
    async (nextQuery = query) => {
      try {
        const rows = await invoke<SearchResult[]>("search_captures", {
          query: nextQuery,
          limit: 40,
        });
        setResults(rows);
        setError(null);
        if (!selectedFrame && rows[0]) {
          setSelectedFrame(rows[0].frame);
        }
      } catch (err) {
        setError(String(err));
      }
    },
    [query, selectedFrame],
  );

  const selectFrame = useCallback(async (frame: CaptureFrame) => {
    setSelectedFrame(frame);
    try {
      const freshFrame = await invoke<CaptureFrame | null>("get_frame", {
        frameId: frame.id,
      });
      if (freshFrame) {
        setSelectedFrame(freshFrame);
      }
    } catch (err) {
      setError(String(err));
    }
  }, []);

  const runAction = useCallback(
    async (action: "start_capture" | "stop_capture" | "capture_once") => {
      setBusyAction(action);
      setError(null);
      try {
        const response = await invoke<CaptureStatus | CaptureFrame>(action);
        if (action === "capture_once") {
          setSelectedFrame(response as CaptureFrame);
        } else {
          setStatus(response as CaptureStatus);
        }
        await refreshStatus();
        await runSearch();
      } catch (err) {
        setError(String(err));
      } finally {
        setBusyAction(null);
      }
    },
    [refreshStatus, runSearch],
  );

  useEffect(() => {
    void refreshStatus();
    void runSearch("");
  }, []);

  useEffect(() => {
    const id = window.setInterval(() => {
      void refreshStatus();
      if (status.running) {
        void runSearch();
      }
    }, status.running ? 3500 : 6000);

    return () => window.clearInterval(id);
  }, [refreshStatus, runSearch, status.running]);

  useEffect(() => {
    let cancelled = false;
    async function loadImage() {
      if (!selectedFrame) {
        setImageData(null);
        return;
      }

      setImageData(null);
      try {
        const dataUrl = await invoke<string | null>("get_frame_image", {
          frameId: selectedFrame.id,
        });
        if (!cancelled) {
          setImageData(dataUrl);
        }
      } catch (err) {
        if (!cancelled) {
          setError(String(err));
        }
      }
    }

    void loadImage();
    return () => {
      cancelled = true;
    };
  }, [selectedFrame?.id]);

  const selectedText = useMemo(() => {
    return (
      selectedFrame?.full_text ||
      selectedFrame?.accessibility_text ||
      ""
    ).trim();
  }, [selectedFrame]);

  return (
    <main className="app-shell">
      <aside className="sidebar">
        <div className="brand-block">
          <p className="label">smalltalk</p>
          <h1>Local capture</h1>
        </div>

        <div className="control-stack" aria-label="Capture controls">
          <button
            className="primary-button"
            disabled={status.running || busyAction !== null}
            onClick={() => void runAction("start_capture")}
          >
            Start
          </button>
          <button
            className="secondary-button"
            disabled={!status.running || busyAction !== null}
            onClick={() => void runAction("stop_capture")}
          >
            Stop
          </button>
          <button
            className="secondary-button warm"
            disabled={busyAction !== null}
            onClick={() => void runAction("capture_once")}
          >
            Capture now
          </button>
        </div>

        <section className="status-panel" aria-label="Capture status">
          <div className="status-row">
            <span>Status</span>
            <strong>{status.running ? "Running" : "Stopped"}</strong>
          </div>
          <div className="status-row">
            <span>Frames</span>
            <strong>{status.frame_count}</strong>
          </div>
          <div className="status-row">
            <span>Screen</span>
            <strong>{status.screenshot_tool ? "Ready" : "Missing"}</strong>
          </div>
          <div className="status-row">
            <span>A11y</span>
            <strong>{status.accessibility_tool ? "Ready" : "Missing"}</strong>
          </div>
          <div className="status-row">
            <span>OCR</span>
            <strong>{status.ocr_tool ? "Ready" : "Missing"}</strong>
          </div>
        </section>

        {error || status.last_error ? (
          <div className="error-box">{error || status.last_error}</div>
        ) : null}
      </aside>

      <section className="workspace">
        <header className="workspace-header">
          <div>
            <p className="label">Search</p>
            <h2>Screen memory</h2>
          </div>
          <form
            className="search-form"
            onSubmit={(event) => {
              event.preventDefault();
              void runSearch(query);
            }}
          >
            <input
              value={query}
              onChange={(event) => setQuery(event.currentTarget.value)}
              placeholder="Search captured text"
            />
            <button type="submit" disabled={busyAction !== null}>
              Search
            </button>
          </form>
        </header>

        <div className="content-grid">
          <section className="results-panel" aria-label="Search results">
            <div className="panel-heading">
              <h3>Frames</h3>
              <span>{results.length}</span>
            </div>
            <div className="result-list">
              {results.length === 0 ? (
                <div className="empty-state">No frames yet</div>
              ) : (
                results.map((result) => (
                  <button
                    key={result.frame.id}
                    className={
                      selectedFrame?.id === result.frame.id
                        ? "result-item active"
                        : "result-item"
                    }
                    onClick={() => void selectFrame(result.frame)}
                  >
                    <span>{formatTime(result.frame.captured_at)}</span>
                    <strong>{frameTitle(result.frame)}</strong>
                    <small>{cleanSnippet(result.snippet || result.frame.full_text)}</small>
                  </button>
                ))
              )}
            </div>
          </section>

          <section className="detail-panel" aria-label="Frame detail">
            {selectedFrame ? (
              <>
                <div className="preview-wrap">
                  {imageData ? (
                    <img src={imageData} alt={frameTitle(selectedFrame)} />
                  ) : (
                    <div className="preview-empty">Loading frame</div>
                  )}
                </div>

                <div className="detail-header">
                  <div>
                    <p className="label">{selectedFrame.capture_trigger}</p>
                    <h3>{frameTitle(selectedFrame)}</h3>
                  </div>
                  <span className="source-badge">
                    {selectedFrame.text_source || "visual"}
                  </span>
                </div>

                <dl className="metadata-grid">
                  <div>
                    <dt>Captured</dt>
                    <dd>{formatTime(selectedFrame.captured_at)}</dd>
                  </div>
                  <div>
                    <dt>App</dt>
                    <dd>{selectedFrame.app_name || "Unknown"}</dd>
                  </div>
                  <div>
                    <dt>Window</dt>
                    <dd>{selectedFrame.window_name || "Unknown"}</dd>
                  </div>
                  <div>
                    <dt>URL</dt>
                    <dd>{selectedFrame.browser_url || "None"}</dd>
                  </div>
                  <div>
                    <dt>Document</dt>
                    <dd>{selectedFrame.document_path || "None"}</dd>
                  </div>
                  <div>
                    <dt>Snapshot</dt>
                    <dd>{selectedFrame.snapshot_path}</dd>
                  </div>
                </dl>

                <div className="text-panel">
                  <div className="panel-heading">
                    <h3>Text</h3>
                    <span>{selectedText.length}</span>
                  </div>
                  <pre>{selectedText || "No text stored"}</pre>
                </div>
              </>
            ) : (
              <div className="detail-empty">No frame selected</div>
            )}
          </section>
        </div>
      </section>
    </main>
  );
}

function frameTitle(frame: CaptureFrame) {
  return frame.window_name || frame.app_name || `Frame ${frame.id}`;
}

function formatTime(value?: number | null) {
  if (!value) return "None";
  return new Intl.DateTimeFormat(undefined, {
    month: "short",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  }).format(new Date(value));
}

function cleanSnippet(value?: string | null) {
  if (!value) return "No text";
  return value.replace(/\[/g, "").replace(/\]/g, "").replace(/\s+/g, " ").trim();
}

export default App;
