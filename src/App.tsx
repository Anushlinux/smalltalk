import {
  type CSSProperties,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { openPath } from "@tauri-apps/plugin-opener";
import "./App.css";

type CaptureFrame = {
  id: number;
  session_id?: string | null;
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
  capture_provider?: string | null;
  scope?: string | null;
  display_id?: string | null;
  window_id?: number | null;
  app_pid?: number | null;
  app_bundle_id?: string | null;
  screen_scale?: number | null;
  pixel_width?: number | null;
  pixel_height?: number | null;
  full_screenshot_path?: string | null;
  active_window_crop_path?: string | null;
  active_element_crop_path?: string | null;
  phash?: string | null;
  privacy_status?: string | null;
  capture_trigger_id?: string | null;
  previous_frame_id?: string | null;
  sck_display_id?: string | null;
  sck_window_id?: number | null;
  sck_owning_bundle_id?: string | null;
  sck_filter_summary_json?: string | null;
  sck_configuration_summary_json?: string | null;
  sck_frame_metadata_json?: string | null;
  sck_capture_mode?: string | null;
  sck_audio_policy?: string | null;
};

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

type SessionExportSummary = {
  session_id: string;
  session_sequence: number;
  generated_at: number;
  kind: string;
  folder_name: string;
  path: string;
  byte_size: number;
  file_count: number;
  warning_count: number;
  counts: SessionCounts;
};

type ResumeLineAnchor = {
  frame_id?: string | null;
  quote?: string | null;
  previous_line?: string | null;
  next_line?: string | null;
  line_index?: number | null;
  source_unit_id?: string | null;
  bounds?: {
    x: number;
    y: number;
    w: number;
    h: number;
  } | null;
  source?: string | null;
  semantic_role?: string | null;
  section_anchor?: string | null;
  confidence?: number | null;
  reason?: string | null;
};

type ResumeQueryBundleResult = {
  output_dir: string;
  bundle_path: string;
  byte_size: number;
  image_count: number;
  json_char_count: number;
  bundle: {
    schema: string;
    candidate_episodes: unknown[];
    resume_candidate?: {
      frame_id: string;
      app: string;
      window_title: string;
      confidence: number;
      exact_words?: string;
      line_anchor?: ResumeLineAnchor | null;
      quality_flags?: string[];
    };
    missing_evidence?: string[];
  };
};

type StopCaptureOutput = {
  status: CaptureStatus;
  session?: CaptureSession | null;
  export?: SessionExportSummary | null;
  resume_query?: ResumeQueryBundleResult | null;
  preview?: unknown;
};

type CaptureStatus = {
  running: boolean;
  frame_count: number;
  recent_app_labels: string[];
  event_count: number;
  transition_count: number;
  content_unit_count: number;
  session_count: number;
  active_session?: CaptureSession | null;
  latest_session?: CaptureSession | null;
  last_export?: SessionExportSummary | null;
  started_at?: number | null;
  last_error?: string | null;
  latest_frame?: CaptureFrame | null;
  skipped_samples: number;
  last_skipped_at?: number | null;
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

type TimelineEvent = {
  id: string;
  ts_ms: number;
  event_type: string;
  app_name?: string | null;
  window_title?: string | null;
  key_category?: string | null;
  payload_json?: string | null;
};

type CaptureTriggerSummary = {
  id: string;
  ts_ms: number;
  trigger_type: string;
  caused_by_event_ids: string;
  pre_frame_id?: string | null;
  post_frame_id?: string | null;
  status: string;
};

type TransitionSummary = {
  id: string;
  trigger_id: string;
  primary_event_id?: string | null;
  pre_frame_id?: string | null;
  post_frame_id?: string | null;
  ts_start_ms: number;
  ts_end_ms: number;
  transition_type?: string | null;
  confidence?: number | null;
  summary?: string | null;
};

type Timeline = {
  events: TimelineEvent[];
  triggers: CaptureTriggerSummary[];
  transitions: TransitionSummary[];
  frames: CaptureFrame[];
};

type BoxLike = {
  id: string;
  text?: string | null;
  bounds_x?: number | null;
  bounds_y?: number | null;
  bounds_w?: number | null;
  bounds_h?: number | null;
  source?: string | null;
  unit_type?: string | null;
  semantic_role?: string | null;
  role?: string | null;
  region_type?: string | null;
  confidence?: number | null;
};

type AppContextSummary = {
  id: string;
  adapter_id: string;
  object_type: string;
  title?: string | null;
  url?: string | null;
  file_path?: string | null;
  selected_text?: string | null;
  focused_object?: string | null;
  confidence?: number | null;
};

type WindowSummary = {
  cg_window_id?: number | null;
  owner_name?: string | null;
  bundle_id?: string | null;
  window_title?: string | null;
  is_active: boolean;
};

type VerificationSignals = {
  screenshot_present: boolean;
  has_ax: boolean;
  has_ocr: boolean;
  has_content_units: boolean;
  has_app_context: boolean;
  has_window_graph: boolean;
  has_transition: boolean;
  has_event_provenance: boolean;
  has_sensitive_regions: boolean;
  ax_node_count: number;
  ocr_span_count: number;
  content_unit_count: number;
  app_context_count: number;
  window_count: number;
  transition_count: number;
  event_count: number;
  missing_signals: string[];
  trust_label: string;
  trust_score: number;
};

type FrameDetail = {
  frame: CaptureFrame;
  verification: VerificationSignals;
  events: TimelineEvent[];
  ax_nodes: BoxLike[];
  ocr_spans: BoxLike[];
  content_units: BoxLike[];
  app_contexts: AppContextSummary[];
  sensitive_regions: BoxLike[];
  windows: WindowSummary[];
  transitions: TransitionSummary[];
};

type NativeResumeCard = {
  generated_at_ms: number;
  lookback_minutes: number;
  what_was_i_doing: string;
  what_was_i_reading?: string | null;
  focus_now: string;
  why_this_focus: string;
  continue_from: {
    frame_id?: string | null;
    app_name?: string | null;
    window_name?: string | null;
    title?: string | null;
    url?: string | null;
    document_path?: string | null;
    quote?: string | null;
    line_anchor?: ResumeLineAnchor | null;
    reason: string;
  };
  what_changed: string[];
  useful_evidence: string[];
  likely_distractions: string[];
  behavior_read: {
    mode: string;
    confidence: number;
    notes: string[];
  };
  next_action: string;
  confidence: number;
  evidence_frame_ids: string[];
  evidence_transition_ids: string[];
  warnings: string[];
};

type CloudResumeTarget = {
  frame_id?: string | null;
  app?: string | null;
  title?: string | null;
  reason?: string;
  exact_words?: string | null;
  exact_visible_words?: string | null;
  anchor_id?: string | null;
  section_anchor?: string | null;
  line_anchor?: ResumeLineAnchor | null;
};

type CloudResumeResult = {
  schema: string;
  request?: {
    session_id: string;
    current_frame_id?: number | null;
  };
  cached?: boolean;
  source: "cloud" | string;
  decision: "current_focus" | "resume_target_found" | "ambiguous_current_focus_vs_prior_task" | "need_more_evidence" | "insufficient_evidence" | string;
  resume_target?: CloudResumeTarget;
  current_activity?: {
    frame_id?: string | null;
    app?: string | null;
    title?: string | null;
    activity_type?: string | null;
    reason?: string;
    exact_visible_words?: string | null;
    anchor_id?: string | null;
    line_anchor?: ResumeLineAnchor | null;
  };
  current_focus?: CloudResumeTarget;
  resume_work_target?: CloudResumeTarget;
  resume_target_if_returning?: CloudResumeTarget;
  return_target?: CloudResumeTarget;
  session_theme?: {
    summary?: string;
    evidence_frames?: string[];
    confidence?: number;
  };
  rejected_input_target?: {
    frame_id?: string | null;
    reason?: string;
  } | null;
  answer?: {
    focus_now?: string;
    what_was_i_doing?: string;
    why_this_focus?: string;
    next_action?: string;
  };
  confidence: number;
  warnings: string[];
  evidence_limits?: string[];
  evidence_handles: string[];
  local_card: NativeResumeCard;
  model?: string | null;
  response_id?: string | null;
  output_path?: string | null;
};

type OpenResumePointResult = {
  strategy: string;
  frame_id?: string | null;
  opened_url?: string | null;
  anchor_text?: string | null;
  confidence: number;
  warnings: string[];
};

type CloudResumeStatus = {
  has_key: boolean;
  key_source: "process_env" | "project_env" | "missing" | string;
  model: string;
};

type OverlayMode = "units" | "ocr" | "ax" | "privacy";
type EvidenceTab = "text" | "events" | "context" | "paths";

const initialStatus: CaptureStatus = {
  running: false,
  frame_count: 0,
  recent_app_labels: [],
  event_count: 0,
  transition_count: 0,
  content_unit_count: 0,
  session_count: 0,
  active_session: null,
  latest_session: null,
  last_export: null,
  skipped_samples: 0,
  data_dir: "",
  database_path: "",
  screenshot_tool: false,
  accessibility_tool: false,
  ocr_tool: false,
};

const emptyTimeline: Timeline = {
  events: [],
  triggers: [],
  transitions: [],
  frames: [],
};

function App() {
  const [status, setStatus] = useState<CaptureStatus>(initialStatus);
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<SearchResult[]>([]);
  const [selectedFrame, setSelectedFrame] = useState<CaptureFrame | null>(null);
  const [frameDetail, setFrameDetail] = useState<FrameDetail | null>(null);
  const [timeline, setTimeline] = useState<Timeline>(emptyTimeline);
  const [overlayMode, setOverlayMode] = useState<OverlayMode>("units");
  const [evidenceTab, setEvidenceTab] = useState<EvidenceTab>("text");
  const [highlightedBoxId, setHighlightedBoxId] = useState<string | null>(null);
  const [imageData, setImageData] = useState<string | null>(null);
  const [busyAction, setBusyAction] = useState<string | null>(null);
  const [lastStopOutput, setLastStopOutput] = useState<StopCaptureOutput | null>(null);
  const [resumeCard, setResumeCard] = useState<NativeResumeCard | null>(null);
  const [cloudResume, setCloudResume] = useState<CloudResumeResult | null>(null);
  const [cloudStatus, setCloudStatus] = useState<CloudResumeStatus | null>(null);
  const [cloudError, setCloudError] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const storeGenerationRef = useRef(0);
  const isDeleting = busyAction === "delete_all_frames";
  const currentSession = status.active_session || status.latest_session || null;
  const currentSessionId = currentSession?.id || null;

  const refreshStatus = useCallback(async () => {
    const requestGeneration = storeGenerationRef.current;
    try {
      const nextStatus = await invoke<CaptureStatus>("capture_status");
      if (requestGeneration !== storeGenerationRef.current) return;
      setStatus(nextStatus);
      setError(null);
      if (!selectedFrame && nextStatus.latest_frame) {
        setSelectedFrame(nextStatus.latest_frame);
      }
    } catch (err) {
      setError(String(err));
    }
  }, [selectedFrame]);

  const refreshCloudStatus = useCallback(async () => {
    try {
      const nextStatus = await invoke<CloudResumeStatus>("get_cloud_resume_status");
      setCloudStatus(nextStatus);
    } catch (err) {
      setCloudStatus({
        has_key: false,
        key_source: "missing",
        model: "gpt-4.1-mini",
      });
      setError(String(err));
    }
  }, []);

  const runSearch = useCallback(
    async (nextQuery = query, sessionId = currentSessionId) => {
      const requestGeneration = storeGenerationRef.current;
      try {
        const rows = await invoke<SearchResult[]>("search_captures", {
          query: nextQuery,
          limit: 48,
          sessionId,
        });
        if (requestGeneration !== storeGenerationRef.current) return;
        setResults(rows);
        setError(null);
        if (!selectedFrame && rows[0]) {
          setSelectedFrame(rows[0].frame);
        }
      } catch (err) {
        setError(String(err));
      }
    },
    [currentSessionId, query, selectedFrame],
  );

  const refreshTimeline = useCallback(async (sessionId = currentSessionId) => {
    const requestGeneration = storeGenerationRef.current;
    try {
      const nextTimeline = await invoke<Timeline>("get_recent_timeline", {
        rangeMs: 10 * 60 * 1000,
        sessionId,
      });
      if (requestGeneration !== storeGenerationRef.current) return;
      setTimeline(nextTimeline);
    } catch (err) {
      setError(String(err));
    }
  }, [currentSessionId]);

  const selectFrame = useCallback(async (frame: CaptureFrame) => {
    setSelectedFrame(frame);
    setFrameDetail(null);
    setHighlightedBoxId(null);
    try {
      const freshFrame = await invoke<CaptureFrame | null>("get_frame", {
        frameId: frame.id,
      });
      const frameForDetail = freshFrame || frame;
      setSelectedFrame(frameForDetail);

      const detail = await invoke<FrameDetail | null>("get_frame_detail", {
        frameId: frameForDetail.id,
      });
      setFrameDetail(detail);
      setError(null);
    } catch (err) {
      setError(String(err));
    }
  }, []);

  const runAction = useCallback(
    async (action: "start_capture" | "stop_capture" | "capture_once") => {
      setBusyAction(action);
      setError(null);
      try {
        if (action === "stop_capture") {
          const response = await invoke<StopCaptureOutput>(action);
          const stoppedSessionId =
            response.session?.id ||
            response.status.latest_session?.id ||
            currentSessionId;
          setLastStopOutput(response);
          setStatus(response.status);
          await refreshStatus();
          await runSearch(query, stoppedSessionId);
          await refreshTimeline(stoppedSessionId);
          return;
        }

        const response = await invoke<CaptureStatus | CaptureFrame>(action);
        if (action === "capture_once") {
          await selectFrame(response as CaptureFrame);
        } else {
          const nextStatus = response as CaptureStatus;
          setLastStopOutput(null);
          setSelectedFrame(null);
          setFrameDetail(null);
          setImageData(null);
          setResumeCard(null);
          setCloudResume(null);
          setCloudError(null);
          setTimeline(emptyTimeline);
          setStatus(nextStatus);
          const nextSessionId =
            nextStatus.active_session?.id ||
            nextStatus.latest_session?.id ||
            currentSessionId;
          await refreshStatus();
          await runSearch(query, nextSessionId);
          await refreshTimeline(nextSessionId);
          return;
        }
        await refreshStatus();
        await runSearch(query, currentSessionId);
        await refreshTimeline(currentSessionId);
      } catch (err) {
        setError(String(err));
      } finally {
        setBusyAction(null);
      }
    },
    [currentSessionId, query, refreshStatus, refreshTimeline, runSearch, selectFrame],
  );

  const deleteAllFrames = useCallback(async () => {
    const confirmed = window.confirm(
      "Delete all stored frames and screenshots? This creates a clean slate.",
    );
    if (!confirmed) return;

    setBusyAction("delete_all_frames");
    setError(null);
    storeGenerationRef.current += 1;
    try {
      const nextStatus = await invoke<CaptureStatus>("delete_all_frames");
      storeGenerationRef.current += 1;
      setResults([]);
      setSelectedFrame(null);
      setImageData(null);
      setFrameDetail(null);
      setTimeline(emptyTimeline);
      setQuery("");
      setLastStopOutput(null);
      setResumeCard(null);
      setCloudResume(null);
      setCloudError(null);
      setStatus({
        ...nextStatus,
        running: false,
        frame_count: 0,
        event_count: 0,
        transition_count: 0,
        content_unit_count: 0,
        latest_frame: null,
        active_session: null,
        latest_session: null,
        last_export: null,
        session_count: 0,
        skipped_samples: 0,
        last_skipped_at: null,
        last_error: null,
      });
    } catch (err) {
      setError(`Delete all failed: ${String(err)}`);
    } finally {
      setBusyAction(null);
    }
  }, []);

  useEffect(() => {
    void refreshStatus();
    void refreshCloudStatus();
    void runSearch("");
    void refreshTimeline();
  }, []);

  useEffect(() => {
    const id = window.setInterval(() => {
      if (isDeleting) return;
      void refreshStatus();
      if (status.running) {
        void runSearch();
        void refreshTimeline();
      }
    }, status.running ? 1500 : 6000);

    return () => window.clearInterval(id);
  }, [isDeleting, refreshStatus, refreshTimeline, runSearch, status.running]);

  useEffect(() => {
    let cancelled = false;
    async function loadImage() {
      if (!selectedFrame) {
        setImageData(null);
        return;
      }

      setImageData(null);
      try {
        const dataUrl = await invoke<string | null>("get_frame_image_variant", {
          frameId: selectedFrame.id,
          variant: "full",
        });
        if (!cancelled) setImageData(dataUrl);
      } catch (err) {
        if (!cancelled) setError(String(err));
      }
    }

    void loadImage();
    return () => {
      cancelled = true;
    };
  }, [selectedFrame?.id]);

  useEffect(() => {
    let cancelled = false;
    async function loadDetail() {
      if (!selectedFrame) {
        setFrameDetail(null);
        return;
      }
      try {
        const detail = await invoke<FrameDetail | null>("get_frame_detail", {
          frameId: selectedFrame.id,
        });
        if (!cancelled) setFrameDetail(detail);
      } catch (err) {
        if (!cancelled) setError(String(err));
      }
    }

    void loadDetail();
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

  const overlayItems = useMemo(() => {
    if (!frameDetail) return [];
    if (overlayMode === "ocr") return frameDetail.ocr_spans;
    if (overlayMode === "ax") return frameDetail.ax_nodes;
    if (overlayMode === "privacy") return frameDetail.sensitive_regions;
    return frameDetail.content_units;
  }, [frameDetail, overlayMode]);

  const selectedVerification = frameDetail?.verification;
  const captureStateLabel = isDeleting
    ? "Deleting"
    : status.running
      ? "Session active"
      : "Ready";
  const hasFrames = status.frame_count > 0;
  const hasQuery = query.trim().length > 0;
  const latestFrameLabel = status.latest_frame
    ? formatTime(status.latest_frame.captured_at)
    : "None yet";
  const sessionLabel = currentSession
    ? `${currentSession.status} session-${String(currentSession.sequence).padStart(3, "0")}`
    : "No session";
  const stopBundle = lastStopOutput?.resume_query || null;
  const cloudBundleLabel = stopBundle ? pathBasename(stopBundle.output_dir) : "None";
  const activeContext = frameDetail?.app_contexts[0];
  const activeTransition = frameDetail?.transitions[0];
  const selectedTitle = selectedFrame ? frameTitle(selectedFrame) : "No frame selected";
  const cloudStatusTitle = cloudResume?.source === "cloud"
    ? "OpenAI answered"
    : cloudError
      ? "OpenAI request failed"
      : cloudStatus?.has_key
    ? "OpenAI key connected"
    : "OpenAI key missing";
  const cloudStatusDetail = cloudError
    ? cloudError
    : cloudStatus?.has_key
    ? `Using ${cloudStatus.model} from ${cloudStatus.key_source === "process_env" ? "process env" : "project .env"}`
    : "Ask OpenAI will fail clearly until OPENAI_API_KEY is available.";
  const cloudResultSummary = cloudResume
    ? cloudResume.source === "cloud"
      ? cloudResume.cached
        ? "Cached cloud answer"
        : cloudResume.answer?.focus_now || "Cloud answer saved"
      : "Rejected non-cloud result"
    : cloudError
      ? "No OpenAI answer generated"
      : "No OpenAI answer yet";
  const cloudCurrentFocus = cloudResume?.current_focus || null;
  const cloudReturnTarget = cloudResume?.resume_target_if_returning || null;
  const cloudTarget = cloudReturnTarget || cloudResume?.resume_target || null;
  const cloudLineAnchor = cloudTarget?.line_anchor || null;
  const cloudExactLine =
    presentLine(cloudLineAnchor?.quote) ||
    presentLine(cloudTarget?.exact_visible_words) ||
    presentLine(cloudTarget?.exact_words);
  const cloudCurrentLine =
    presentLine(cloudCurrentFocus?.line_anchor?.quote) ||
    presentLine(cloudCurrentFocus?.exact_visible_words) ||
    presentLine(cloudCurrentFocus?.exact_words);
  const cloudReturnLine =
    presentLine(cloudReturnTarget?.line_anchor?.quote) ||
    presentLine(cloudReturnTarget?.exact_visible_words) ||
    presentLine(cloudReturnTarget?.exact_words);
  const cloudEvidenceLine = lineAnchorEvidenceLabel(
    cloudLineAnchor,
    cloudTarget?.frame_id,
  );
  const cloudHasSplitTargets =
    Boolean(cloudCurrentFocus?.frame_id && cloudReturnTarget?.frame_id) &&
    cloudCurrentFocus?.frame_id !== cloudReturnTarget?.frame_id;
  const resumeLineAnchor = resumeCard?.continue_from.line_anchor || null;
  const resumeExactLine =
    presentLine(resumeLineAnchor?.quote) ||
    presentLine(resumeCard?.continue_from.quote);
  const resumeEvidenceLine = lineAnchorEvidenceLabel(
    resumeLineAnchor,
    resumeCard?.continue_from.frame_id,
  );
  const openStopBundle = useCallback(async () => {
    const path = lastStopOutput?.resume_query?.bundle_path;
    if (!path) return;
    try {
      await openPath(path);
    } catch (err) {
      setError(String(err));
    }
  }, [lastStopOutput?.resume_query?.bundle_path]);

  const acceptCloudResumeResult = useCallback((result: CloudResumeResult) => {
    if (result.source !== "cloud" || !result.response_id) {
      setCloudResume(null);
      setCloudError("OpenAI did not return a cloud response id; no OpenAI answer was shown.");
      return;
    }
    setCloudResume(result);
    setCloudError(null);
    setError(null);
  }, []);

  const generateCloudResume = useCallback(async () => {
    setBusyAction("run_cloud_resume");
    setCloudResume(null);
    setCloudError(null);
    setError(null);
    try {
      const result = await invoke<CloudResumeResult>("run_cloud_resume", {
        input: {
          current_frame_id: selectedFrame?.id ?? null,
          allow_followup: true,
        },
      });
      acceptCloudResumeResult(result);
    } catch (err) {
      setCloudError(readableOpenAIError(String(err)));
    } finally {
      setBusyAction(null);
    }
  }, [acceptCloudResumeResult, selectedFrame?.id]);

  const generateNativeResumeCard = useCallback(async () => {
    setBusyAction("get_native_resume_card");
    setError(null);
    try {
      const card = await invoke<NativeResumeCard>("get_native_resume_card", {
        input: {
          lookback_minutes: 20,
          current_frame_id: selectedFrame?.id ?? null,
          max_keyframes: 10,
        },
      });
      setResumeCard(card);
    } catch (err) {
      setError(String(err));
    } finally {
      setBusyAction(null);
    }
  }, [selectedFrame?.id]);

  const openCloudResumePoint = useCallback(async (targetFrameId?: string | null) => {
    if (!cloudResume) return;
    setBusyAction("open_resume_point");
    setError(null);
    const parsedTargetFrameId = targetFrameId ? Number(targetFrameId) : null;
    const safeTargetFrameId =
      parsedTargetFrameId !== null && Number.isFinite(parsedTargetFrameId)
        ? parsedTargetFrameId
        : null;
    try {
      const result = await invoke<OpenResumePointResult>("open_resume_point", {
        input: {
          output_path: cloudResume.output_path ?? null,
          session_id: cloudResume.request?.session_id ?? null,
          current_frame_id: selectedFrame?.id ?? cloudResume.request?.current_frame_id ?? null,
          target_frame_id: safeTargetFrameId,
        },
      });
      if (result.warnings.length > 0) {
        setError(result.warnings.join(" "));
      }
    } catch (err) {
      setError(String(err));
    } finally {
      setBusyAction(null);
    }
  }, [cloudResume, selectedFrame?.id]);

  useEffect(() => {
    let disposed = false;
    const unlisteners: Array<() => void> = [];

    listen("session-island-resume-requested", () => {
      void generateCloudResume();
    })
      .then((nextUnlisten) => {
        if (disposed) {
          nextUnlisten();
        } else {
          unlisteners.push(nextUnlisten);
        }
      })
      .catch((err) => setError(String(err)));

    listen<CloudResumeResult>("session-island-cloud-resume-ready", (event) => {
      acceptCloudResumeResult(event.payload);
    })
      .then((nextUnlisten) => {
        if (disposed) {
          nextUnlisten();
        } else {
          unlisteners.push(nextUnlisten);
        }
      })
      .catch((err) => setError(String(err)));

    return () => {
      disposed = true;
      unlisteners.forEach((unlisten) => unlisten());
    };
  }, [acceptCloudResumeResult, generateCloudResume]);

  return (
    <main className="capture-shell">
      <header className="capture-topbar">
        <div className="identity-block">
          <div className="brand-mark" aria-hidden="true">S</div>
          <div>
            <p className="product-kicker">Smalltalk</p>
            <h1>Session Capture</h1>
          </div>
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
            placeholder="Search captured text, app names, URLs"
            aria-label="Search captured evidence"
          />
          <button type="submit" disabled={busyAction !== null}>Search</button>
        </form>

        <div className="control-strip" aria-label="Capture controls">
          <button
            className="primary-button"
            disabled={status.running || busyAction !== null}
            aria-busy={busyAction === "start_capture"}
            onClick={() => void runAction("start_capture")}
          >
            {busyAction === "start_capture" ? "Starting" : "Start session"}
          </button>
          <button
            className="secondary-button"
            disabled={!status.running || busyAction !== null}
            aria-busy={busyAction === "stop_capture"}
            onClick={() => void runAction("stop_capture")}
          >
            {busyAction === "stop_capture" ? "Preparing" : "Stop session"}
          </button>
          <button
            className="secondary-button"
            disabled={!status.running || busyAction !== null}
            aria-busy={busyAction === "capture_once"}
            onClick={() => void runAction("capture_once")}
          >
            {busyAction === "capture_once" ? "Capturing" : "Capture now"}
          </button>
          <button
            className="danger-button"
            disabled={busyAction !== null}
            aria-busy={isDeleting}
            onClick={() => void deleteAllFrames()}
          >
            {isDeleting ? "Deleting" : "Delete all"}
          </button>
        </div>
      </header>

      <section
        className={`cloud-resume-bar ${cloudResume?.source === "cloud" ? "ready" : cloudError || !cloudStatus?.has_key ? "missing" : "ready"}`}
        aria-label="OpenAI resume output"
      >
        <div className="cloud-resume-status">
          <p className="product-kicker">OpenAI resume</p>
          <h2>{cloudStatusTitle}</h2>
          <span>{cloudStatusDetail}</span>
        </div>
        <div className="cloud-resume-result">
          <span>{cloudResultSummary}</span>
          {cloudResume ? (
            <strong>{cloudResumeProvenanceLabel(cloudResume)}</strong>
          ) : (
            <strong>Waiting for a resume request</strong>
          )}
        </div>
        <div className="cloud-resume-actions">
          <button
            className="primary-button"
            type="button"
            disabled={busyAction !== null || !hasFrames}
            aria-busy={busyAction === "run_cloud_resume"}
            onClick={() => void generateCloudResume()}
          >
            {busyAction === "run_cloud_resume" ? "Asking OpenAI" : "Ask OpenAI"}
          </button>
          <button
            className="secondary-button"
            type="button"
            disabled={busyAction !== null || !cloudResume}
            aria-busy={busyAction === "open_resume_point"}
            onClick={() => void openCloudResumePoint(cloudTarget?.frame_id ?? null)}
          >
            {busyAction === "open_resume_point" ? "Opening" : "Open return target"}
          </button>
          {cloudHasSplitTargets ? (
            <button
              className="secondary-button"
              type="button"
              disabled={busyAction !== null || !cloudResume}
              aria-busy={busyAction === "open_resume_point"}
              onClick={() => void openCloudResumePoint(cloudCurrentFocus?.frame_id ?? null)}
            >
              Open current focus
            </button>
          ) : null}
        </div>
        {cloudResume ? (
          <div className="cloud-resume-details">
            <dl className="resume-facts">
              {cloudCurrentFocus ? (
                <div>
                  <dt>Current focus</dt>
                  <dd>
                    {[
                      cloudCurrentFocus.frame_id ? `Frame ${cloudCurrentFocus.frame_id}` : null,
                      cloudCurrentFocus.title || cloudCurrentFocus.app || null,
                      cloudCurrentLine,
                    ]
                      .filter(Boolean)
                      .join(" / ") || "No current focus returned"}
                  </dd>
                </div>
              ) : null}
              {cloudReturnTarget ? (
                <div>
                  <dt>Return target</dt>
                  <dd>
                    {[
                      cloudReturnTarget.frame_id ? `Frame ${cloudReturnTarget.frame_id}` : null,
                      cloudReturnTarget.title || cloudReturnTarget.app || null,
                      cloudReturnLine,
                    ]
                      .filter(Boolean)
                      .join(" / ") || "No return target returned"}
                  </dd>
                </div>
              ) : null}
              <div>
                <dt>Exact line</dt>
                <dd>{cloudExactLine || "No exact line returned"}</dd>
              </div>
              {presentLine(cloudLineAnchor?.previous_line) ? (
                <div>
                  <dt>Before</dt>
                  <dd>{presentLine(cloudLineAnchor?.previous_line)}</dd>
                </div>
              ) : null}
              {presentLine(cloudLineAnchor?.next_line) ? (
                <div>
                  <dt>After</dt>
                  <dd>{presentLine(cloudLineAnchor?.next_line)}</dd>
                </div>
              ) : null}
              <div>
                <dt>Why</dt>
                <dd>
                  {[
                    cloudResume.decision === "ambiguous_current_focus_vs_prior_task"
                      ? "Current focus and return target are different."
                      : null,
                    cloudTarget?.reason || cloudResume.answer?.why_this_focus || null,
                  ]
                    .filter(Boolean)
                    .join(" ") || "No reason returned"}
                </dd>
              </div>
              {cloudResume.evidence_limits?.length ? (
                <div>
                  <dt>Limits</dt>
                  <dd>{cloudResume.evidence_limits.slice(0, 3).join(" / ")}</dd>
                </div>
              ) : null}
              <div>
                <dt>Evidence</dt>
                <dd>
                  {[
                    cloudEvidenceLine,
                    cloudResume.evidence_handles.slice(0, 4).join(" / "),
                  ]
                    .filter(Boolean)
                    .join(" / ") || "No evidence handles"}
                </dd>
              </div>
            </dl>
          </div>
        ) : null}
      </section>

      {error || status.last_error ? (
        <div className="error-box" role="alert">{error || status.last_error}</div>
      ) : null}

      {lastStopOutput?.resume_query ? (
        <section className="session-output" aria-label="Stopped session output">
          <div className="session-output-main">
            <div>
              <p className="product-kicker">Stop output</p>
              <h2>Cloud-ready bundle ready</h2>
            </div>
            <dl>
              <div>
                <dt>Session</dt>
                <dd>
                  {lastStopOutput.session
                    ? `session-${String(lastStopOutput.session.sequence).padStart(3, "0")}`
                    : pathBasename(lastStopOutput.resume_query.output_dir)}
                </dd>
              </div>
              <div>
                <dt>Frames</dt>
                <dd>{lastStopOutput.session?.counts.frames ?? 0}</dd>
              </div>
              <div>
                <dt>Images</dt>
                <dd>{lastStopOutput.resume_query.image_count}</dd>
              </div>
              <div>
                <dt>Episodes</dt>
                <dd>{lastStopOutput.resume_query.bundle.candidate_episodes.length}</dd>
              </div>
            </dl>
          </div>
          <div className="artifact-path">
            <span>{lastStopOutput.resume_query.bundle_path}</span>
          </div>
          <button
            className="secondary-button open-artifact-button"
            type="button"
            onClick={() => void openStopBundle()}
          >
            Open bundle JSON
          </button>
          <pre>{formatStopPreview(lastStopOutput)}</pre>
        </section>
      ) : null}

      <section className="health-strip" aria-label="Capture health">
        <StatusPill label="State" value={captureStateLabel} tone={status.running ? "good" : "quiet"} />
        <StatusPill label="Session" value={sessionLabel} tone={status.running ? "good" : "quiet"} />
        <StatusPill label="Cloud bundle" value={cloudBundleLabel} />
        <StatusPill label="Frames" value={status.frame_count} />
        <StatusPill label="Events" value={status.event_count} />
        <StatusPill label="Transitions" value={status.transition_count} />
        <StatusPill label="Units" value={status.content_unit_count} />
        <StatusPill label="Total sessions" value={status.session_count} />
        <StatusPill label="Latest" value={latestFrameLabel} />
        <StatusPill label="Screen" value={status.screenshot_tool ? "ready" : "missing"} tone={status.screenshot_tool ? "good" : "bad"} />
        <StatusPill label="A11y" value={status.accessibility_tool ? "ready" : "missing"} tone={status.accessibility_tool ? "good" : "bad"} />
        <StatusPill label="OCR" value={status.ocr_tool ? "ready" : "missing"} tone={status.ocr_tool ? "good" : "bad"} />
      </section>

      <section className="inspector-grid">
        <aside className="timeline-pane" aria-label="Captured frames">
          <div className="pane-heading">
            <div>
              <h2>Evidence timeline</h2>
              <p>{hasQuery ? "Filtered frames" : "Most recent local captures"}</p>
            </div>
            <span>{results.length}</span>
          </div>

          <div className="frame-list">
            {results.length === 0 ? (
              <EmptyCaptureState hasFrames={hasFrames} hasQuery={hasQuery} />
            ) : (
              results.map((result) => (
                <FrameRow
                  key={result.frame.id}
                  frame={result.frame}
                  active={selectedFrame?.id === result.frame.id}
                  snippet={result.snippet || result.frame.full_text}
                  onSelect={() => void selectFrame(result.frame)}
                />
              ))
            )}
          </div>

          <div className="event-feed">
            <div className="feed-heading">
              <h3>Raw event stream</h3>
              <span>{timeline.events.length}</span>
            </div>
            {timeline.events.slice(0, 8).map((event) => (
              <div className="event-row" key={event.id}>
                <time>{formatTime(event.ts_ms)}</time>
                <strong>{event.event_type}</strong>
                <span>{event.app_name || event.window_title || event.key_category || "event"}</span>
              </div>
            ))}
            {timeline.events.length === 0 ? (
              <p className="feed-empty">No raw events in the last 10 minutes.</p>
            ) : null}
          </div>
        </aside>

        <section className="viewer-pane" aria-label="Frame inspector">
          <div className="viewer-toolbar">
            <div>
              <p className="product-kicker">{selectedFrame?.capture_trigger || "waiting"}</p>
              <h2>{selectedTitle}</h2>
            </div>
            <span className={`trust-badge ${selectedVerification?.trust_label || "unknown"}`}>
              {selectedVerification
                ? `${selectedVerification.trust_label} ${Math.round(selectedVerification.trust_score * 100)}%`
                : "unverified"}
            </span>
          </div>

          <div className="viewer-stage">
            {selectedFrame && imageData ? (
              <div className="screenshot-stage" style={stageStyle(selectedFrame)}>
                <img src={imageData} alt={frameTitle(selectedFrame)} />
                <div className={`overlay-layer ${overlayMode}`} aria-hidden="true">
                  {overlayItems
                    .filter(hasBounds)
                    .slice(0, 140)
                    .map((item) => (
                      <span
                        className={
                          highlightedBoxId === item.id
                            ? "overlay-box active"
                            : "overlay-box"
                        }
                        key={item.id}
                        style={boxStyle(item, selectedFrame)}
                        title={overlayLabel(item)}
                      />
                    ))}
                </div>
              </div>
            ) : selectedFrame ? (
              <div className="viewer-empty">
                <strong>Loading screenshot</strong>
                <span>{selectedTitle}</span>
              </div>
            ) : (
              <div className="viewer-empty">
                <strong>No frame selected</strong>
                <span>Choose a frame or capture now to inspect the screenshot, sources, and transitions.</span>
              </div>
            )}
          </div>

          <div className="overlay-toolbar" aria-label="Overlay controls">
            {(["units", "ocr", "ax", "privacy"] as const).map((mode) => (
              <button
                key={mode}
                className={overlayMode === mode ? "active" : ""}
                type="button"
                onClick={() => {
                  setOverlayMode(mode);
                  setHighlightedBoxId(null);
                }}
              >
                <span>{overlayLabelForMode(mode)}</span>
                <strong>{overlayCount(frameDetail, mode)}</strong>
              </button>
            ))}
          </div>

          <div className="legend-row">
            <span><i className="legend-dot units" /> content units</span>
            <span><i className="legend-dot ocr" /> OCR spans</span>
            <span><i className="legend-dot ax" /> AX nodes</span>
            <span><i className="legend-dot privacy" /> privacy regions</span>
          </div>
        </section>

        <aside className="evidence-pane" aria-label="Verification drawer">
          <section className="verification-card">
            <div className="pane-heading compact">
              <div>
                <h2>Last capture</h2>
                <p>{selectedFrame ? formatTime(selectedFrame.captured_at) : "Nothing selected"}</p>
              </div>
              <span>{selectedFrame?.text_source || "visual"}</span>
            </div>

            <div className="signal-grid">
              <Signal label="Screenshot" ok={selectedVerification?.screenshot_present} />
              <Signal label="AX" ok={selectedVerification?.has_ax} count={selectedVerification?.ax_node_count} />
              <Signal label="OCR" ok={selectedVerification?.has_ocr} count={selectedVerification?.ocr_span_count} />
              <Signal label="Units" ok={selectedVerification?.has_content_units} count={selectedVerification?.content_unit_count} />
              <Signal label="Window graph" ok={selectedVerification?.has_window_graph} count={selectedVerification?.window_count} />
              <Signal label="Transition" ok={selectedVerification?.has_transition} count={selectedVerification?.transition_count} />
            </div>

            {selectedVerification?.missing_signals.length ? (
              <div className="missing-box">
                <strong>Missing signals</strong>
                {selectedVerification.missing_signals.slice(0, 5).map((signal) => (
                  <span key={signal}>{signal}</span>
                ))}
              </div>
            ) : (
              <div className="complete-box">All core verification signals are present for this frame.</div>
            )}
          </section>

          <section className="resume-card">
            <div className="pane-heading compact">
              <div>
                <h2>Resume cue</h2>
                <p>
                  {resumeCard
                    ? `Local / last ${resumeCard.lookback_minutes} minutes`
                    : "Safe native resume card"}
                </p>
              </div>
              <button
                className="secondary-button"
                type="button"
                disabled={busyAction !== null || !hasFrames}
                aria-busy={busyAction === "get_native_resume_card"}
                onClick={() => void generateNativeResumeCard()}
              >
                {busyAction === "get_native_resume_card" ? "Reading" : "Resume me"}
              </button>
            </div>
            {resumeCard ? (
              <div className="native-resume-card">
                <strong>{resumeCard.focus_now}</strong>
                <p>{resumeCard.what_was_i_doing}</p>
                {resumeCard.what_was_i_reading ? (
                  <p>{resumeCard.what_was_i_reading}</p>
                ) : null}
                <dl className="resume-facts">
                  <div>
                    <dt>Continue from</dt>
                    <dd>
                      {resumeCard.continue_from.title ||
                        resumeCard.continue_from.window_name ||
                        resumeCard.continue_from.app_name ||
                        "No safe frame"}
                    </dd>
                  </div>
                  <div>
                    <dt>Exact line</dt>
                    <dd>{resumeExactLine || "No exact line available"}</dd>
                  </div>
                  {presentLine(resumeLineAnchor?.previous_line) ? (
                    <div>
                      <dt>Before</dt>
                      <dd>{presentLine(resumeLineAnchor?.previous_line)}</dd>
                    </div>
                  ) : null}
                  {presentLine(resumeLineAnchor?.next_line) ? (
                    <div>
                      <dt>After</dt>
                      <dd>{presentLine(resumeLineAnchor?.next_line)}</dd>
                    </div>
                  ) : null}
                  <div>
                    <dt>Why</dt>
                    <dd>{resumeCard.why_this_focus}</dd>
                  </div>
                  <div>
                    <dt>Target reason</dt>
                    <dd>{resumeLineAnchor?.reason || resumeCard.continue_from.reason}</dd>
                  </div>
                  <div>
                    <dt>Next action</dt>
                    <dd>{resumeCard.next_action}</dd>
                  </div>
                  <div>
                    <dt>Evidence</dt>
                    <dd>
                      {[
                        resumeEvidenceLine,
                        resumeCard.evidence_frame_ids.slice(0, 4).map((id) => `frame ${id}`).join(" / "),
                      ]
                        .filter(Boolean)
                        .join(" / ") || "No evidence handles"}
                    </dd>
                  </div>
                  <div>
                    <dt>Confidence</dt>
                    <dd>{confidenceLabel(resumeCard.confidence)}</dd>
                  </div>
                </dl>
                {resumeExactLine ? (
                  <blockquote>{resumeExactLine}</blockquote>
                ) : null}
                {resumeCard.warnings.length ? (
                  <div className="resume-warning">
                    {resumeCard.warnings.slice(0, 2).map((warning) => (
                      <span key={warning}>{warning}</span>
                    ))}
                  </div>
                ) : null}
              </div>
            ) : (
              <dl className="resume-facts">
                <div>
                  <dt>Current surface</dt>
                  <dd>{activeContext?.title || selectedTitle}</dd>
                </div>
                <div>
                  <dt>Likely object</dt>
                  <dd>{activeContext?.object_type || "unknown"}</dd>
                </div>
                <div>
                  <dt>Transition</dt>
                  <dd>{activeTransition?.transition_type || selectedFrame?.capture_trigger || "none"}</dd>
                </div>
                <div>
                  <dt>Focus now</dt>
                  <dd>{topContentUnit(frameDetail)?.text || selectedText || "Capture more evidence to infer focus."}</dd>
                </div>
              </dl>
            )}
          </section>

          <section className="detail-drawer">
            <div className="drawer-tabs" role="tablist" aria-label="Evidence tabs">
              {(["text", "events", "context", "paths"] as const).map((tab) => (
                <button
                  key={tab}
                  className={evidenceTab === tab ? "active" : ""}
                  type="button"
                  role="tab"
                  aria-selected={evidenceTab === tab}
                  onClick={() => setEvidenceTab(tab)}
                >
                  {tab}
                </button>
              ))}
            </div>

            <EvidencePanel
              tab={evidenceTab}
              frame={selectedFrame}
              detail={frameDetail}
              selectedText={selectedText}
              onHighlight={(item, mode) => {
                setOverlayMode(mode);
                setHighlightedBoxId(item.id);
              }}
            />
          </section>
        </aside>
      </section>
    </main>
  );
}

function StatusPill({
  label,
  value,
  tone = "quiet",
}: {
  label: string;
  value: string | number;
  tone?: "quiet" | "good" | "bad";
}) {
  return (
    <div className={`status-pill ${tone}`}>
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}

function FrameRow({
  frame,
  active,
  snippet,
  onSelect,
}: {
  frame: CaptureFrame;
  active: boolean;
  snippet?: string | null;
  onSelect: () => void;
}) {
  return (
    <button
      aria-pressed={active}
      className={active ? "frame-row active" : "frame-row"}
      onClick={onSelect}
      type="button"
    >
      <FrameThumbnail frame={frame} />
      <span className="frame-row-main">
        <span className="row-meta">
          <time>{formatTime(frame.captured_at)}</time>
          <b>{frame.capture_trigger}</b>
        </span>
        <strong>{frameTitle(frame)}</strong>
        <small>{cleanSnippet(snippet || frame.full_text)}</small>
        <span className="badge-row">
          <EvidenceBadge label="screen" ok={Boolean(frame.snapshot_path)} />
          <EvidenceBadge label={frame.capture_provider || "capture"} ok={frame.capture_provider === "screen_capture_kit"} />
          <EvidenceBadge label={frame.text_source || "visual"} ok={Boolean(frame.text_source)} />
          <EvidenceBadge label={frame.privacy_status || "normal"} ok={frame.privacy_status !== "skipped_sensitive"} />
          <EvidenceBadge label="transition" ok={Boolean(frame.capture_trigger_id || frame.previous_frame_id)} />
        </span>
      </span>
    </button>
  );
}

function FrameThumbnail({ frame }: { frame: CaptureFrame }) {
  const [src, setSrc] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    async function load() {
      try {
        const dataUrl = await invoke<string | null>("get_frame_image_variant", {
          frameId: frame.id,
          variant: "preview",
        });
        if (!cancelled) setSrc(dataUrl);
      } catch {
        if (!cancelled) setSrc(null);
      }
    }
    void load();
    return () => {
      cancelled = true;
    };
  }, [frame.id]);

  return (
    <span className="frame-thumb" aria-hidden="true">
      {src ? <img src={src} alt="" /> : <span />}
    </span>
  );
}

function EvidenceBadge({ label, ok }: { label: string; ok: boolean }) {
  return <span className={ok ? "evidence-badge ok" : "evidence-badge"}>{label}</span>;
}

function Signal({
  label,
  ok,
  count,
}: {
  label: string;
  ok?: boolean;
  count?: number;
}) {
  return (
    <div className={ok ? "signal ok" : "signal"}>
      <span>{label}</span>
      <strong>{typeof count === "number" ? count : ok ? "yes" : "no"}</strong>
    </div>
  );
}

function EmptyCaptureState({
  hasFrames,
  hasQuery,
}: {
  hasFrames: boolean;
  hasQuery: boolean;
}) {
  return (
    <div className="empty-state">
      <strong>{hasFrames && hasQuery ? "No matching evidence" : "No captured frames yet"}</strong>
      <span>
        {hasFrames && hasQuery
          ? "Clear the search or use a broader term to inspect existing captures."
          : "Start a session to collect screenshots, events, text sources, and missing-signal checks."}
      </span>
    </div>
  );
}

function EvidencePanel({
  tab,
  frame,
  detail,
  selectedText,
  onHighlight,
}: {
  tab: EvidenceTab;
  frame: CaptureFrame | null;
  detail: FrameDetail | null;
  selectedText: string;
  onHighlight: (item: BoxLike, mode: OverlayMode) => void;
}) {
  if (!frame) {
    return (
      <div className="drawer-empty">
        <strong>No frame selected</strong>
        <span>Select a frame to inspect stored evidence.</span>
      </div>
    );
  }

  if (tab === "events") {
    return (
      <div className="drawer-list">
        {detail?.events.length ? (
          detail.events.map((event) => (
            <div className="drawer-row" key={event.id}>
              <strong>{event.event_type}</strong>
              <span>{formatTime(event.ts_ms)}</span>
              <small>{event.app_name || event.window_title || event.key_category || event.id}</small>
            </div>
          ))
        ) : (
          <div className="drawer-empty">
            <strong>No raw event linked</strong>
            <span>Manual captures may not have event provenance.</span>
          </div>
        )}
        {detail?.transitions.map((transition) => (
          <div className="drawer-row transition" key={transition.id}>
            <strong>{transition.transition_type || "transition"}</strong>
            <span>{transition.summary || transition.trigger_id}</span>
            <small>
              {transition.pre_frame_id || "none"}{" -> "}{transition.post_frame_id || "none"}
            </small>
          </div>
        ))}
      </div>
    );
  }

  if (tab === "context") {
    return (
      <div className="drawer-list">
        {detail?.app_contexts.map((context) => (
          <div className="drawer-row" key={context.id}>
            <strong>{context.object_type}</strong>
            <span>{context.title || context.url || context.file_path || context.adapter_id}</span>
            <small>{confidenceLabel(context.confidence)} via {context.adapter_id}</small>
          </div>
        ))}
        {detail?.content_units.slice(0, 8).map((unit) => (
          <button
            className="drawer-row selectable"
            key={unit.id}
            type="button"
            onClick={() => onHighlight(unit, "units")}
          >
            <strong>{unit.semantic_role || unit.unit_type || unit.source}</strong>
            <span>{cleanSnippet(unit.text)}</span>
            <small>{confidenceLabel(unit.confidence)} content unit</small>
          </button>
        ))}
      </div>
    );
  }

  if (tab === "paths") {
    return (
      <dl className="path-list">
        <div>
          <dt>Snapshot</dt>
          <dd>{frame.snapshot_path}</dd>
        </div>
        <div>
          <dt>Window crop</dt>
          <dd>{frame.active_window_crop_path || "None"}</dd>
        </div>
        <div>
          <dt>Database</dt>
          <dd>{frame.capture_trigger_id || "No trigger id"}</dd>
        </div>
        <div>
          <dt>Session</dt>
          <dd>{frame.session_id || "No session id"}</dd>
        </div>
        <div>
          <dt>App bundle</dt>
          <dd>{frame.app_bundle_id || "Unknown"}</dd>
        </div>
        <div>
          <dt>Capture provider</dt>
          <dd>{frame.capture_provider || "Unknown"}</dd>
        </div>
        <div>
          <dt>SCK scope</dt>
          <dd>{[
            frame.sck_capture_mode,
            frame.sck_display_id ? `display ${frame.sck_display_id}` : null,
            frame.sck_window_id ? `window ${frame.sck_window_id}` : null,
            frame.sck_audio_policy ? `audio ${frame.sck_audio_policy}` : null,
          ].filter(Boolean).join(" / ") || "None"}</dd>
        </div>
        <div>
          <dt>URL / document</dt>
          <dd>{frame.browser_url || frame.document_path || "None"}</dd>
        </div>
      </dl>
    );
  }

  return (
    <div className="text-reader">
      <div className="source-stack">
        {detail?.content_units.slice(0, 6).map((unit) => (
          <button
            key={unit.id}
            type="button"
            onClick={() => onHighlight(unit, "units")}
          >
            <strong>{unit.semantic_role || unit.unit_type || unit.source}</strong>
            <span>{cleanSnippet(unit.text)}</span>
          </button>
        ))}
      </div>
      <pre>{selectedText || "No text stored for this frame."}</pre>
    </div>
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

function formatStopPreview(output: StopCaptureOutput) {
  return JSON.stringify(
    output.preview || {
      session: output.session,
      resumeQuery: output.resume_query,
    },
    null,
    2,
  );
}

function pathBasename(path?: string | null) {
  if (!path) return "";
  return path.split(/[\\/]/).filter(Boolean).pop() || path;
}

function cleanSnippet(value?: string | null) {
  if (!value) return "No text";
  return value.replace(/\[/g, "").replace(/\]/g, "").replace(/\s+/g, " ").trim();
}

function presentLine(value?: string | null) {
  if (!value) return null;
  const text = value.replace(/\[/g, "").replace(/\]/g, "").replace(/\s+/g, " ").trim();
  return text || null;
}

function lineAnchorEvidenceLabel(anchor?: ResumeLineAnchor | null, frameId?: string | null) {
  const resolvedFrameId = frameId || anchor?.frame_id || null;
  const parts = [
    resolvedFrameId ? `frame ${resolvedFrameId}` : null,
    anchor?.source || null,
    anchor?.semantic_role || null,
    typeof anchor?.confidence === "number" ? confidenceLabel(anchor.confidence) : null,
  ].filter(Boolean);
  return parts.join(" / ");
}

function hasBounds(item: BoxLike) {
  return (
    typeof item.bounds_x === "number" &&
    typeof item.bounds_y === "number" &&
    typeof item.bounds_w === "number" &&
    typeof item.bounds_h === "number" &&
    item.bounds_w > 1 &&
    item.bounds_h > 1
  );
}

function stageStyle(frame: CaptureFrame): CSSProperties {
  const width = frame.pixel_width || 16;
  const height = frame.pixel_height || 9;
  return {
    aspectRatio: `${width} / ${height}`,
    "--frame-aspect": `${width / height}`,
  } as CSSProperties;
}

function boxStyle(item: BoxLike, frame: CaptureFrame): CSSProperties {
  const width = frame.pixel_width || 1;
  const height = frame.pixel_height || 1;
  return {
    left: `${((item.bounds_x || 0) / width) * 100}%`,
    top: `${((item.bounds_y || 0) / height) * 100}%`,
    width: `${((item.bounds_w || 0) / width) * 100}%`,
    height: `${((item.bounds_h || 0) / height) * 100}%`,
  };
}

function overlayLabel(item: BoxLike) {
  return (
    item.semantic_role ||
    item.unit_type ||
    item.role ||
    item.region_type ||
    item.source ||
    cleanSnippet(item.text)
  );
}

function overlayLabelForMode(mode: OverlayMode) {
  if (mode === "ocr") return "OCR";
  if (mode === "ax") return "AX";
  if (mode === "privacy") return "Privacy";
  return "Units";
}

function overlayCount(detail: FrameDetail | null, mode: OverlayMode) {
  if (!detail) return 0;
  if (mode === "ocr") return detail.ocr_spans.length;
  if (mode === "ax") return detail.ax_nodes.length;
  if (mode === "privacy") return detail.sensitive_regions.length;
  return detail.content_units.length;
}

function confidenceLabel(value?: number | null) {
  if (typeof value !== "number") return "unscored";
  return `${Math.round(value * 100)}%`;
}

function cloudResumeProvenanceLabel(result: CloudResumeResult) {
  if (result.source === "cloud") {
    return ["Cloud", result.model, result.response_id].filter(Boolean).join(" / ");
  }
  return "Rejected non-cloud result";
}

function readableOpenAIError(error: string) {
  const lower = error.toLowerCase();
  if (lower.includes("openai_api_key")) {
    return "OPENAI_API_KEY is not set; Ask OpenAI did not call OpenAI.";
  }
  if (
    lower.includes("could not resolve host") ||
    lower.includes("couldn't resolve host") ||
    lower.includes("exit status: 6")
  ) {
    return "OpenAI network unavailable: DNS could not resolve api.openai.com; no OpenAI answer was generated.";
  }
  if (lower.includes("authentication")) {
    return "OpenAI authentication failed: check OPENAI_API_KEY; no OpenAI answer was generated.";
  }
  if (lower.includes("rate limit")) {
    return "OpenAI rate limit reached; no OpenAI answer was generated.";
  }
  return error.replace(/^Error:\s*/, "");
}

function topContentUnit(detail: FrameDetail | null) {
  if (!detail) return null;
  return detail.content_units.find((unit) => unit.text && unit.text.length > 24) || detail.content_units[0] || null;
}

export default App;
