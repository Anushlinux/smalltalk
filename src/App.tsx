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

type StopCaptureOutput = {
  status: CaptureStatus;
  session?: CaptureSession | null;
  export?: SessionExportSummary | null;
};

type RuntimeDiagnostics = {
  heavy_captures_stored: number;
  heavy_captures_skipped: number;
  heavy_captures_skipped_budget: number;
  heavy_captures_skipped_dedupe: number;
  heavy_captures_skipped_privacy: number;
  heavy_captures_skipped_cancellation: number;
  heavy_captures_skipped_smalltalk_self: number;
  events_aggregated: number;
  ocr_runs: number;
  ax_snapshots: number;
  continue_normal_calls: number;
  continue_rebuild_calls: number;
  decision_cache_hits: number;
};

type CaptureStatus = {
  running: boolean;
  frame_count: number;
  recent_app_labels: string[];
  signal_count: number;
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
  runtime_diagnostics: RuntimeDiagnostics;
};

type LocalMemoryDiagnostics = {
  database_path: string;
  captured_root: string;
  database_bytes: number;
  snapshot_bytes: number;
  safe_export_bytes: number;
  frame_count: number;
  event_count: number;
  heavy_evidence_rows: {
    content_units: number;
    ax_nodes: number;
    ocr_text_rows: number;
    ocr_spans: number;
    app_contexts: number;
    window_snapshots: number;
    windows: number;
  };
  continue_object_counts: {
    artifacts: number;
    artifact_observations: number;
    task_actions: number;
    episodes: number;
    workstreams: number;
    candidates: number;
    decisions: number;
    feedback_events: number;
    breadcrumbs: number;
  };
  low_value_duplicate_frames: number;
  self_capture_frames: number;
  decision_linked_frames: number;
  estimated_cleanup_potential_bytes: number;
  oldest_retained_frame_ms?: number | null;
  latest_frame_ms?: number | null;
  cleanup_last_run_ms?: number | null;
  cleanup_last_result?: string | null;
  budgets: {
    min_important_capture_interval_ms: number;
    min_low_value_capture_interval_ms: number;
    idle_capture_interval_ms: number;
    rolling_window_ms: number;
    max_screenshots_per_10_minutes: number;
    max_screenshots_per_surface_without_change: number;
    max_snapshot_dir_bytes: number;
    max_retained_low_value_duplicate_frames: number;
    max_diagnostic_rows_per_cleanup: number;
  };
  runtime_diagnostics: RuntimeDiagnostics;
};

type CleanupLocalMemoryResult = {
  diagnostics: LocalMemoryDiagnostics;
  dry_run: boolean;
  candidate_frames: number;
  protected_frames: number;
  deleted_frames: number;
  deleted_snapshot_files: number;
  reclaimed_bytes: number;
  summary: string;
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

type OpenResumePointResult = {
  strategy: string;
  frame_id?: string | null;
  opened_url?: string | null;
  anchor_text?: string | null;
  confidence: number;
  warnings: string[];
};

type ContinueMemoryStatus = {
  schema: string;
  schema_version?: number | null;
  has_schema: boolean;
  counts: {
    artifacts: number;
    artifact_observations: number;
    task_actions: number;
    task_action_events: number;
    episodes: number;
    episode_actions: number;
    episode_artifacts: number;
    workstreams: number;
    workstream_episodes: number;
    workstream_artifacts: number;
    candidates: number;
    decisions: number;
    feedback_events: number;
    breadcrumbs: number;
  };
  latest_artifact_timestamp?: number | null;
  latest_workstream_timestamp?: number | null;
};

type ContinueFocusSummary = {
  frame_id: string;
  artifact_id?: string | null;
  artifact_kind?: string | null;
  app_name?: string | null;
  window_title?: string | null;
  title?: string | null;
  browser_url?: string | null;
  document_path?: string | null;
  captured_at_ms: number;
};

type ContinueSelectedWorkstream = {
  workstream_id: string;
  state: string;
  title_candidate?: string | null;
  primary_artifact_id?: string | null;
  last_active_timestamp_ms: number;
  unresolved_signal?: string | null;
};

type ContinueReturnTarget = {
  artifact_id?: string | null;
  artifact_kind?: string | null;
  title?: string | null;
  browser_url?: string | null;
  document_path?: string | null;
  openability: string;
  fallback_frame_id?: string | null;
};

type ContinueActionSummary = {
  action_id: string;
  action_kind: string;
  action_role: string;
  timestamp_ms: number;
  evidence_frame_id: string;
  artifact_id?: string | null;
  collapse_count: number;
  first_frame_id?: string | null;
  last_frame_id?: string | null;
  strongest_frame_id?: string | null;
};

type ContinueEvidenceAnchors = {
  frame_ids: string[];
  action_ids: string[];
  episode_ids: string[];
  artifact_ids: string[];
};

type ContinueCandidateSummary = {
  candidate_id: string;
  workstream_id: string;
  target_artifact_id?: string | null;
  candidate_kind: string;
  score: number;
  confidence_label: string;
  reason?: string | null;
  missing_evidence: string[];
  evidence_frame_id?: string | null;
  supporting_episode_id?: string | null;
  last_meaningful_action_id?: string | null;
};

type ContinueScoreComponents = {
  actionability: number;
  primary_target: number;
  unresolved_state: number;
  branch_origin: number;
  evidence_quality: number;
  recency: number;
  openability: number;
  privacy_safety: number;
};

type ContinueDecisionResult = {
  decision_id: string;
  mode: string;
  cache_hit: boolean;
  source: string;
  model?: string | null;
  response_id?: string | null;
  current_focus?: ContinueFocusSummary | null;
  current_activity?: string | null;
  selected_workstream?: ContinueSelectedWorkstream | null;
  return_target?: ContinueReturnTarget | null;
  resume_work_target?: ContinueReturnTarget | null;
  candidate_kind?: string | null;
  last_meaningful_action?: ContinueActionSummary | null;
  unresolved_state?: string | null;
  next_action?: string | null;
  confidence: number;
  confidence_label: string;
  evidence_anchors: ContinueEvidenceAnchors;
  missing_evidence: string[];
  warnings: string[];
  validation_failures: string[];
  alternatives: ContinueCandidateSummary[];
  generated_candidates: number;
  validation_status: string;
};

type RecentContinueWorkstream = {
  id: string;
  state: string;
  title_candidate?: string | null;
  primary_artifact_id?: string | null;
  primary_artifact_title?: string | null;
  created_at_ms: number;
  last_active_timestamp_ms: number;
  suspended_timestamp_ms?: number | null;
  confidence: number;
  unresolved_signal?: string | null;
  source: string;
  episodes: Array<{
    episode_id: string;
    membership_score: number;
    membership_reason?: string | null;
    order_index: number;
  }>;
  artifacts: Array<{
    artifact_id: string;
    durable_role: string;
    display_title?: string | null;
    stable_key?: string | null;
    importance_score: number;
    first_seen_frame_id?: string | null;
    last_seen_frame_id?: string | null;
    reason?: string | null;
  }>;
};

type ContinueBreadcrumbResult = {
  id: string;
  workstream_id: string;
  text: string;
  source: string;
  created_at_ms: number;
};

type ContinueFeedbackEventResult = {
  id: string;
  decision_id?: string | null;
  selected_candidate_id?: string | null;
  workstream_id?: string | null;
  event_kind: string;
  observed_frame_id?: string | null;
  target_artifact_id?: string | null;
  chosen_artifact_id?: string | null;
  timestamp_ms: number;
  confidence: number;
  reason?: string | null;
  note?: string | null;
  source?: string | null;
};

type ContinueEvalReport = {
  schema: string;
  evaluated_at_ms: number;
  case_count: number;
  target_artifact_correct: number;
  target_artifact_accuracy: number;
  recall_at_k: number;
  mrr: number;
  current_focus_false_positive_rate: number;
  hallucinated_artifact_count: number;
  model_validation_fallback_rate: number;
  cases: Array<{
    name: string;
    scenario: string;
    selected_candidate_id?: string | null;
    selected_target_artifact_id?: string | null;
    target_artifact_correct: boolean;
    validation_status: string;
    validation_failures: string[];
  }>;
};

type ContinueWorkstreamArtifactDetail = {
  artifact_id: string;
  durable_role: string;
  artifact_kind: string;
  display_title?: string | null;
  stable_key?: string | null;
  app_name?: string | null;
  window_title?: string | null;
  browser_url?: string | null;
  document_path?: string | null;
  openability: string;
  evidence_quality: string;
  privacy_status?: string | null;
  importance_score: number;
  first_seen_frame_id?: string | null;
  last_seen_frame_id?: string | null;
  reason?: string | null;
};

type ContinueWorkstreamActionDetail = {
  action_id: string;
  frame_id: string;
  previous_frame_id?: string | null;
  artifact_id?: string | null;
  artifact_title?: string | null;
  secondary_artifact_id?: string | null;
  secondary_artifact_title?: string | null;
  action_kind: string;
  action_role: string;
  role_in_episode: string;
  order_index: number;
  trigger_type?: string | null;
  transition_label?: string | null;
  evidence_event_ids: string[];
  confidence: number;
  reason?: string | null;
  created_at_ms: number;
};

type ContinueWorkstreamEpisodeDetail = {
  id: string;
  state: string;
  start_frame_id?: string | null;
  end_frame_id?: string | null;
  start_timestamp_ms: number;
  end_timestamp_ms?: number | null;
  primary_artifact_id?: string | null;
  primary_artifact_title?: string | null;
  dominant_action_kind?: string | null;
  boundary_start_reason?: string | null;
  boundary_end_reason?: string | null;
  evidence_quality: string;
  confidence: number;
  summary_label?: string | null;
  membership_score: number;
  membership_reason?: string | null;
  actions: ContinueWorkstreamActionDetail[];
  artifacts: RecentContinueWorkstream["artifacts"];
};

type ContinueWorkstreamCandidateDetail = {
  candidate_id: string;
  workstream_id: string;
  target_artifact_id?: string | null;
  target_title?: string | null;
  target_kind?: string | null;
  target_openability?: string | null;
  candidate_kind: string;
  last_meaningful_action_id?: string | null;
  evidence_frame_id?: string | null;
  supporting_episode_id?: string | null;
  score: number;
  confidence_label: string;
  reason?: string | null;
  missing_evidence: string[];
  components: ContinueScoreComponents;
  created_at_ms: number;
};

type ContinueDecisionSummary = {
  decision_id: string;
  requested_at_ms: number;
  source: string;
  selected_candidate_id?: string | null;
  return_target_artifact_id?: string | null;
  confidence: number;
  decision_reason?: string | null;
  next_action?: string | null;
  warnings: string[];
  validation_status: string;
};

type ContinueBreadcrumbSummary = ContinueBreadcrumbResult;

type ContinueWorkstreamDetailResult = {
  workstream: RecentContinueWorkstream;
  artifacts: ContinueWorkstreamArtifactDetail[];
  episodes: ContinueWorkstreamEpisodeDetail[];
  candidates: ContinueWorkstreamCandidateDetail[];
  latest_decision?: ContinueDecisionSummary | null;
  feedback_events: ContinueFeedbackEventResult[];
  breadcrumbs: ContinueBreadcrumbSummary[];
  evidence_anchors: ContinueEvidenceAnchors;
};

type OverlayMode = "units" | "ocr" | "ax" | "privacy";
type EvidenceTab = "text" | "events" | "context" | "paths";

const emptyRuntimeDiagnostics: RuntimeDiagnostics = {
  heavy_captures_stored: 0,
  heavy_captures_skipped: 0,
  heavy_captures_skipped_budget: 0,
  heavy_captures_skipped_dedupe: 0,
  heavy_captures_skipped_privacy: 0,
  heavy_captures_skipped_cancellation: 0,
  heavy_captures_skipped_smalltalk_self: 0,
  events_aggregated: 0,
  ocr_runs: 0,
  ax_snapshots: 0,
  continue_normal_calls: 0,
  continue_rebuild_calls: 0,
  decision_cache_hits: 0,
};

const initialStatus: CaptureStatus = {
  running: false,
  frame_count: 0,
  recent_app_labels: [],
  signal_count: 0,
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
  runtime_diagnostics: emptyRuntimeDiagnostics,
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
  const [continueMemory, setContinueMemory] = useState<ContinueMemoryStatus | null>(null);
  const [continueDecision, setContinueDecision] = useState<ContinueDecisionResult | null>(null);
  const [continueDecisionFrameCount, setContinueDecisionFrameCount] = useState<number | null>(null);
  const [continueDecisionUpdatedAt, setContinueDecisionUpdatedAt] = useState<number | null>(null);
  const [continueError, setContinueError] = useState<string | null>(null);
  const [continueOpenResult, setContinueOpenResult] = useState<OpenResumePointResult | null>(null);
  const [workstreams, setWorkstreams] = useState<RecentContinueWorkstream[]>([]);
  const [selectedWorkstreamId, setSelectedWorkstreamId] = useState<string | null>(null);
  const [workstreamDetail, setWorkstreamDetail] = useState<ContinueWorkstreamDetailResult | null>(null);
  const [workstreamDetailError, setWorkstreamDetailError] = useState<string | null>(null);
  const [feedbackStatus, setFeedbackStatus] = useState<string | null>(null);
  const [evalReport, setEvalReport] = useState<ContinueEvalReport | null>(null);
  const [evalError, setEvalError] = useState<string | null>(null);
  const [breadcrumbText, setBreadcrumbText] = useState("");
  const [breadcrumbStatus, setBreadcrumbStatus] = useState<string | null>(null);
  const [memoryDiagnostics, setMemoryDiagnostics] = useState<LocalMemoryDiagnostics | null>(null);
  const [cleanupResult, setCleanupResult] = useState<CleanupLocalMemoryResult | null>(null);
  const [evidenceOpen, setEvidenceOpen] = useState(false);
  const [diagnosticsOpen, setDiagnosticsOpen] = useState(false);
  const [memoryMenuOpen, setMemoryMenuOpen] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const storeGenerationRef = useRef(0);
  const autoContinueRef = useRef(false);
  const captureMenuRef = useRef<HTMLDetailsElement | null>(null);
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

  const refreshContinueMemory = useCallback(async () => {
    try {
      const nextMemory = await invoke<ContinueMemoryStatus>("get_continue_memory_status");
      setContinueMemory(nextMemory);
    } catch (err) {
      setContinueError(`Continue memory status failed: ${String(err)}`);
    }
  }, []);

  const refreshMemoryDiagnostics = useCallback(async () => {
    try {
      const diagnostics = await invoke<LocalMemoryDiagnostics>("get_local_memory_diagnostics");
      setMemoryDiagnostics(diagnostics);
    } catch (err) {
      setContinueError(`Memory diagnostics failed: ${String(err)}`);
    }
  }, []);

  const refreshWorkstreams = useCallback(async () => {
    try {
      const rows = await invoke<RecentContinueWorkstream[]>(
        "get_recent_continue_workstreams",
        { limit: 8 },
      );
      setWorkstreams(rows);
    } catch (err) {
      setContinueError(`Workstreams failed: ${String(err)}`);
    }
  }, []);

  const loadWorkstreamDetail = useCallback(async (workstreamId: string | null) => {
    if (!workstreamId) {
      setWorkstreamDetail(null);
      return;
    }
    try {
      const detail = await invoke<ContinueWorkstreamDetailResult>(
        "get_continue_workstream_detail",
        {
          input: {
            workstream_id: workstreamId,
            decision_id: continueDecision?.decision_id || null,
          },
        },
      );
      setWorkstreamDetail(detail);
      setWorkstreamDetailError(null);
    } catch (err) {
      setWorkstreamDetail(null);
      setWorkstreamDetailError(`Workstream detail failed: ${String(err)}`);
    }
  }, [continueDecision?.decision_id]);

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

  const revealContinueFrame = useCallback(
    async (frameId?: string | null) => {
      if (!frameId) {
        setEvidenceOpen(true);
        return;
      }

      const parsedFrameId = Number(frameId);
      if (!Number.isFinite(parsedFrameId)) {
        setEvidenceOpen(true);
        return;
      }

      setEvidenceOpen(true);
      try {
        const frame = await invoke<CaptureFrame | null>("get_frame", {
          frameId: parsedFrameId,
        });
        if (frame) {
          await selectFrame(frame);
        }
      } catch (err) {
        setError(String(err));
      }
    },
    [selectFrame],
  );

  const runContinueDecision = useCallback(async (options: { forceRebuild?: boolean } = {}) => {
    setBusyAction("get_continue_decision");
    setContinueError(null);
    setContinueOpenResult(null);
    setBreadcrumbStatus(null);
    try {
      const decision = await invoke<ContinueDecisionResult>("get_continue_decision", {
        input: {
          mode: options.forceRebuild === true ? "rebuild" : "normal",
          rebuild_layers: options.forceRebuild === true,
          micro_inference_enabled: false,
        },
      });
      setContinueDecision(decision);
      setSelectedWorkstreamId(decision.selected_workstream?.workstream_id || null);
      setContinueDecisionFrameCount(status.frame_count);
      setContinueDecisionUpdatedAt(Date.now());
      await refreshContinueMemory();
      if (diagnosticsOpen) {
        await refreshWorkstreams();
      }
      const firstEvidenceFrame = decision.evidence_anchors.frame_ids[0];
      if (firstEvidenceFrame && !selectedFrame) {
        await revealContinueFrame(firstEvidenceFrame);
        setEvidenceOpen(false);
      }
    } catch (err) {
      setContinueError(`Continue failed: ${String(err)}`);
    } finally {
      setBusyAction(null);
    }
  }, [diagnosticsOpen, refreshContinueMemory, refreshWorkstreams, revealContinueFrame, selectedFrame, status.frame_count]);

  const openContinueTarget = useCallback(async () => {
    if (!continueDecision) return;
    setBusyAction("open_continue_target");
    setContinueOpenResult(null);
    setContinueError(null);
    try {
      const result = await invoke<OpenResumePointResult>("open_resume_point", {
        input: {
          continue_decision_id: continueDecision.decision_id,
        },
      });
      setContinueOpenResult(result);
      if (result.warnings.length > 0) {
        setContinueError(result.warnings.join(" "));
      }
    } catch (err) {
      setContinueError(`Open target failed: ${String(err)}`);
    } finally {
      setBusyAction(null);
    }
  }, [continueDecision]);

  const saveBreadcrumb = useCallback(async () => {
    const text = breadcrumbText.trim();
    const workstreamId = selectedWorkstreamId || continueDecision?.selected_workstream?.workstream_id;
    if (!text || !workstreamId) return;
    setBusyAction("add_continue_breadcrumb");
    setBreadcrumbStatus(null);
    setContinueError(null);
    try {
      const saved = await invoke<ContinueBreadcrumbResult>("add_continue_breadcrumb", {
        input: {
          workstream_id: workstreamId,
          text: text.slice(0, 240),
          source: "desktop_ui",
        },
      });
      setBreadcrumbText("");
      setBreadcrumbStatus(`Saved for this workstream at ${formatTime(saved.created_at_ms)}`);
      setFeedbackStatus("Next-step note saved.");
      await refreshContinueMemory();
      await invoke<ContinueFeedbackEventResult>("record_continue_feedback", {
        input: {
          decision_id: continueDecision?.decision_id || null,
          selected_candidate_id: workstreamDetail?.latest_decision?.selected_candidate_id || null,
          workstream_id: workstreamId,
          target_artifact_id: continueDecision?.resume_work_target?.artifact_id || continueDecision?.return_target?.artifact_id || null,
          corrected_artifact_id: null,
          feedback_kind: "user_next_step_note",
          note: text.slice(0, 240),
          source: "desktop_ui",
        },
      }).catch(() => null);
      await loadWorkstreamDetail(workstreamId);
    } catch (err) {
      setContinueError(`Save note failed: ${String(err)}`);
    } finally {
      setBusyAction(null);
    }
  }, [
    breadcrumbText,
    continueDecision,
    loadWorkstreamDetail,
    refreshContinueMemory,
    selectedWorkstreamId,
    workstreamDetail?.latest_decision?.selected_candidate_id,
  ]);

  const recordContinueFeedback = useCallback(
    async (
      feedbackKind: string,
      options: {
        targetArtifactId?: string | null;
        correctedArtifactId?: string | null;
        selectedCandidateId?: string | null;
        workstreamId?: string | null;
        note?: string | null;
      } = {},
    ) => {
      const workstreamId =
        options.workstreamId ||
        selectedWorkstreamId ||
        continueDecision?.selected_workstream?.workstream_id ||
        null;
      setBusyAction("record_continue_feedback");
      setContinueError(null);
      setFeedbackStatus(null);
      try {
        const feedback = await invoke<ContinueFeedbackEventResult>(
          "record_continue_feedback",
          {
            input: {
              decision_id: continueDecision?.decision_id || workstreamDetail?.latest_decision?.decision_id || null,
              selected_candidate_id:
                options.selectedCandidateId ||
                workstreamDetail?.latest_decision?.selected_candidate_id ||
                null,
              workstream_id: workstreamId,
              target_artifact_id:
                options.targetArtifactId ||
                continueDecision?.resume_work_target?.artifact_id ||
                continueDecision?.return_target?.artifact_id ||
                workstreamDetail?.latest_decision?.return_target_artifact_id ||
                null,
              corrected_artifact_id: options.correctedArtifactId || null,
              feedback_kind: feedbackKind,
              note: options.note || null,
              source: "desktop_ui",
            },
          },
        );
        setFeedbackStatus(`${sentenceCase(feedback.event_kind)} feedback saved.`);
        await loadWorkstreamDetail(workstreamId);
      } catch (err) {
        setContinueError(`Feedback failed: ${String(err)}`);
      } finally {
        setBusyAction(null);
      }
    },
    [continueDecision, loadWorkstreamDetail, selectedWorkstreamId, workstreamDetail],
  );

  const continueFromAlternative = useCallback(
    async (candidate: ContinueWorkstreamCandidateDetail | ContinueCandidateSummary) => {
      await recordContinueFeedback("corrected", {
        selectedCandidateId: candidate.candidate_id,
        workstreamId: candidate.workstream_id,
        targetArtifactId:
          "target_artifact_id" in candidate
            ? candidate.target_artifact_id
            : null,
        correctedArtifactId:
          "target_artifact_id" in candidate
            ? candidate.target_artifact_id
            : null,
        note: candidate.reason || "Selected alternative continuation target.",
      });
      setSelectedWorkstreamId(candidate.workstream_id);
      const frameId = Number(candidate.evidence_frame_id);
      if (Number.isFinite(frameId)) {
        try {
          const result = await invoke<OpenResumePointResult>("open_resume_point", {
            input: { target_frame_id: frameId },
          });
          setContinueOpenResult(result);
        } catch (err) {
          setContinueError(`Open alternative failed: ${String(err)}`);
        }
      }
    },
    [recordContinueFeedback],
  );

  const runContinueEval = useCallback(async () => {
    setBusyAction("run_continue_eval");
    setEvalError(null);
    try {
      const report = await invoke<ContinueEvalReport>("run_continue_eval", {
        evalFilePath: null,
      });
      setEvalReport(report);
    } catch (err) {
      setEvalError(`Continue eval failed: ${String(err)}`);
    } finally {
      setBusyAction(null);
    }
  }, []);

  const runMemoryCleanup = useCallback(async (dryRun = true) => {
    setBusyAction("cleanup_local_memory");
    setContinueError(null);
    try {
      const result = await invoke<CleanupLocalMemoryResult>("cleanup_local_memory", {
        input: {
          include_debug_exports: false,
          vacuum: false,
          dry_run: dryRun,
        },
      });
      setCleanupResult(result);
      setMemoryDiagnostics(result.diagnostics);
      if (!dryRun) {
        await refreshStatus();
        await refreshContinueMemory();
      }
    } catch (err) {
      setContinueError(`Cleanup failed: ${String(err)}`);
    } finally {
      setBusyAction(null);
    }
  }, [refreshContinueMemory, refreshStatus]);

  const runDevReset = useCallback(async () => {
    const confirmed = window.confirm(
      "Reset local memory for developer testing? This clears frames, events, derived Continue rows, snapshots, and generated debug exports.",
    );
    if (!confirmed) return;
    setBusyAction("dev_reset_local_memory");
    setError(null);
    storeGenerationRef.current += 1;
    try {
      const nextStatus = await invoke<CaptureStatus>("dev_reset_local_memory", {
        input: { include_debug_exports: true },
      });
      storeGenerationRef.current += 1;
      setResults([]);
      setSelectedFrame(null);
      setImageData(null);
      setFrameDetail(null);
      setTimeline(emptyTimeline);
      setContinueDecision(null);
      setContinueDecisionFrameCount(null);
      setContinueDecisionUpdatedAt(null);
      setWorkstreams([]);
      setSelectedWorkstreamId(null);
      setWorkstreamDetail(null);
      setFeedbackStatus(null);
      setEvalReport(null);
      setCleanupResult(null);
      setQuery("");
      setStatus(nextStatus);
      await refreshContinueMemory();
      await refreshMemoryDiagnostics();
    } catch (err) {
      setError(`Developer reset failed: ${String(err)}`);
    } finally {
      setBusyAction(null);
    }
  }, [refreshContinueMemory, refreshMemoryDiagnostics]);

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
          setSelectedFrame(null);
          setFrameDetail(null);
          setImageData(null);
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
      setContinueDecision(null);
      setContinueDecisionFrameCount(null);
      setContinueDecisionUpdatedAt(null);
      setWorkstreams([]);
      setSelectedWorkstreamId(null);
      setWorkstreamDetail(null);
      setFeedbackStatus(null);
      setEvalReport(null);
      setQuery("");
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
    void refreshContinueMemory();
  }, []);

  useEffect(() => {
    if (!memoryMenuOpen) return;

    const handlePointerDown = (event: PointerEvent) => {
      if (!captureMenuRef.current?.contains(event.target as Node)) {
        setMemoryMenuOpen(false);
      }
    };
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        setMemoryMenuOpen(false);
      }
    };

    document.addEventListener("pointerdown", handlePointerDown);
    document.addEventListener("keydown", handleKeyDown);
    return () => {
      document.removeEventListener("pointerdown", handlePointerDown);
      document.removeEventListener("keydown", handleKeyDown);
    };
  }, [memoryMenuOpen]);

  useEffect(() => {
    const id = window.setInterval(() => {
      if (isDeleting) return;
      void refreshStatus();
      if (status.running) {
        void refreshContinueMemory();
        if (diagnosticsOpen) {
          void runSearch();
          void refreshTimeline();
          void refreshWorkstreams();
        }
      }
    }, status.running ? 1500 : 6000);

    return () => window.clearInterval(id);
  }, [diagnosticsOpen, isDeleting, refreshContinueMemory, refreshStatus, refreshTimeline, refreshWorkstreams, runSearch, status.running]);

  useEffect(() => {
    if (
      autoContinueRef.current ||
      busyAction !== null ||
      continueDecision ||
      status.frame_count === 0
    ) {
      return;
    }
    autoContinueRef.current = true;
    void runContinueDecision();
  }, [busyAction, continueDecision, runContinueDecision, status.frame_count]);

  useEffect(() => {
    if (diagnosticsOpen) {
      void loadWorkstreamDetail(selectedWorkstreamId);
    } else {
      setWorkstreamDetail(null);
    }
  }, [diagnosticsOpen, loadWorkstreamDetail, selectedWorkstreamId]);

  useEffect(() => {
    if (!selectedWorkstreamId && workstreams[0]) {
      setSelectedWorkstreamId(workstreams[0].id);
    }
  }, [selectedWorkstreamId, workstreams]);

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
  const continueResumeTarget = continueDecision?.resume_work_target || continueDecision?.return_target || null;
  const continueWorkstreamTitle =
    continueDecision?.selected_workstream?.title_candidate ||
    continueTargetLabel(continueResumeTarget) ||
    "Possible continuation";
  const continueSourceLabel = continueDecision
    ? continueDecision.source === "cloud_micro_inference"
      ? "Model-ranked, locally validated"
      : continueDecision.source === "local_fallback"
        ? "Local fallback"
        : "Local decision"
    : "No decision yet";
  const continueHasEvidence =
    status.frame_count > 0 ||
    Boolean(continueMemory && continueMemory.counts.artifacts > 0);
  const continueIsStale =
    Boolean(continueDecision) &&
    continueDecisionFrameCount !== null &&
    status.frame_count > continueDecisionFrameCount;
  const continueFreshnessLabel = busyAction === "get_continue_decision"
    ? "Updating"
    : continueIsStale
      ? "New evidence"
      : continueDecision
        ? "Current"
        : "Ready";
  const continueStatusLabel = isDeleting
    ? "Deleting local memory"
    : status.last_error
      ? "Permission issue"
      : status.running
        ? "Active"
        : continueHasEvidence
          ? "Paused"
          : "No evidence";
  const continuePrimaryMessage = !continueHasEvidence
    ? "Start local memory to make Continue useful."
    : status.running && !continueDecision
      ? "Smalltalk is watching locally. Continue when there is enough evidence."
      : continueDecision
        ? continueWorkstreamTitle
        : "Ready to find where to continue.";
  const captureStateLabel = isDeleting
    ? "Deleting"
    : status.running
      ? "Local memory active"
      : "Ready";
  const hasFrames = status.frame_count > 0;
  const hasQuery = query.trim().length > 0;
  const latestFrameLabel = status.latest_frame
    ? formatTime(status.latest_frame.captured_at)
    : "None yet";
  const latestEvidenceAgeLabel = status.latest_frame
    ? formatRelativeAge(status.latest_frame.captured_at)
    : "No evidence yet";
  const sessionLabel = currentSession
    ? `${currentSession.status} capture-${String(currentSession.sequence).padStart(3, "0")}`
    : "No capture";
  const activeContext = frameDetail?.app_contexts[0];
  const activeTransition = frameDetail?.transitions[0];
  const selectedTitle = selectedFrame ? frameTitle(selectedFrame) : "No frame selected";
  useEffect(() => {
    let disposed = false;
    const unlisteners: Array<() => void> = [];

    listen<CaptureFrame>("capture-frame", (event) => {
      void refreshStatus();
      if (!selectedFrame) {
        setSelectedFrame(event.payload);
      }
      void refreshContinueMemory();
      if (diagnosticsOpen) {
        void refreshWorkstreams();
      }
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
  }, [
    refreshContinueMemory,
    refreshWorkstreams,
    refreshStatus,
    diagnosticsOpen,
    selectedFrame,
  ]);

  return (
    <main className="capture-shell">
      <header className="capture-topbar">
        <div className="identity-block">
          <div className="brand-mark" aria-hidden="true">S</div>
          <div>
            <p className="product-kicker">Smalltalk</p>
            <h1>Smalltalk Continue</h1>
          </div>
        </div>

        <div className="continue-status-strip" aria-label="Local memory status">
          <StatusPill label="Local memory" value={continueStatusLabel} tone={status.running ? "good" : status.last_error ? "bad" : "quiet"} />
          <StatusPill label="Evidence age" value={latestEvidenceAgeLabel} />
          <StatusPill label="Continue" value={continueFreshnessLabel} tone={continueIsStale ? "good" : "quiet"} />
        </div>

        <div className="control-strip" aria-label="Capture controls">
          <button
            className="primary-button"
            disabled={busyAction !== null}
            aria-busy={busyAction === "get_continue_decision"}
            onClick={() => void runContinueDecision()}
          >
            {busyAction === "get_continue_decision" ? "Finding" : "Continue"}
          </button>
          <details
            className="capture-menu"
            open={memoryMenuOpen}
            ref={captureMenuRef}
            onToggle={(event) => setMemoryMenuOpen(event.currentTarget.open)}
          >
            <summary>Memory</summary>
            <div>
              <button
                className="secondary-button"
                disabled={status.running || busyAction !== null}
                aria-busy={busyAction === "start_capture"}
                onClick={() => {
                  setMemoryMenuOpen(false);
                  void runAction("start_capture");
                }}
                type="button"
              >
                {busyAction === "start_capture" ? "Starting" : "Start local memory"}
              </button>
              <button
                className="secondary-button"
                disabled={!status.running || busyAction !== null}
                aria-busy={busyAction === "stop_capture"}
                onClick={() => {
                  setMemoryMenuOpen(false);
                  void runAction("stop_capture");
                }}
                type="button"
              >
                {busyAction === "stop_capture" ? "Pausing" : "Pause local memory"}
              </button>
              <button
                className="secondary-button"
                disabled={!status.running || busyAction !== null}
                aria-busy={busyAction === "capture_once"}
                onClick={() => {
                  setMemoryMenuOpen(false);
                  void runAction("capture_once");
                }}
                type="button"
              >
                {busyAction === "capture_once" ? "Capturing" : "Capture evidence now"}
              </button>
              <button
                className="danger-button"
                disabled={busyAction !== null}
                aria-busy={isDeleting}
                onClick={() => {
                  setMemoryMenuOpen(false);
                  void deleteAllFrames();
                }}
                type="button"
              >
                {isDeleting ? "Deleting" : "Delete local memory"}
              </button>
            </div>
          </details>
        </div>
      </header>

      <div
        className="app-scroll"
        onScroll={() => {
          if (memoryMenuOpen) setMemoryMenuOpen(false);
        }}
      >
      <section className="continue-home" aria-label="Continue">
        <ContinueDecisionCard
          decision={continueDecision}
          primaryMessage={continuePrimaryMessage}
          hasEvidence={continueHasEvidence}
          running={status.running}
          busyAction={busyAction}
          sourceLabel={continueSourceLabel}
          openResult={continueOpenResult}
          liveFrameCount={status.frame_count}
          liveSignalCount={status.signal_count}
          decisionFrameCount={continueDecisionFrameCount}
          decisionUpdatedAt={continueDecisionUpdatedAt}
          stale={continueIsStale}
          onStartMemory={() => void runAction("start_capture")}
          onContinue={() => void runContinueDecision()}
          onOpenTarget={() => void openContinueTarget()}
          onShowEvidence={() => void revealContinueFrame(continueDecision?.evidence_anchors.frame_ids[0])}
          onRecordFeedback={(kind) => void recordContinueFeedback(kind)}
          onUseAlternative={(candidate) => void continueFromAlternative(candidate)}
        />
      </section>

      {continueError ? (
        <div className="error-box" role="alert">{continueError}</div>
      ) : null}

      {error || status.last_error ? (
        <div className="error-box" role="alert">{error || status.last_error}</div>
      ) : null}

      {evidenceOpen ? (
        <ContinueEvidencePanel
          decision={continueDecision}
          selectedFrame={selectedFrame}
          imageData={imageData}
          onClose={() => setEvidenceOpen(false)}
        />
      ) : null}

      <details
        className="developer-panel diagnostics-panel"
        onToggle={(event) => {
          const open = event.currentTarget.open;
          setDiagnosticsOpen(open);
          if (open) {
            void refreshWorkstreams();
            void runSearch("");
            void refreshTimeline();
            void refreshMemoryDiagnostics();
            void loadWorkstreamDetail(selectedWorkstreamId);
          }
        }}
      >
        <summary>
          <span>Developer diagnostics</span>
          <strong>Frame inspector, search, raw events, and local evidence substrate</strong>
        </summary>

        <section className="diagnostics-workspace" aria-label="Continue diagnostics">
          <WorkstreamList
            workstreams={workstreams}
            selectedWorkstreamId={
              selectedWorkstreamId ||
              continueDecision?.selected_workstream?.workstream_id
            }
            onRefresh={() => void refreshWorkstreams()}
            onSelect={(workstreamId) => setSelectedWorkstreamId(workstreamId)}
          />

          <section className="breadcrumb-card" aria-label="Next-step note">
            <div>
              <h2>Leave a next-step note for later</h2>
              <p>Attach a short local cue to the selected workstream.</p>
            </div>
            <textarea
              value={breadcrumbText}
              maxLength={240}
              disabled={
                !(selectedWorkstreamId || continueDecision?.selected_workstream) ||
                busyAction !== null
              }
              onChange={(event) => setBreadcrumbText(event.currentTarget.value)}
              placeholder="e.g. check the failing test, then update the parser"
              aria-label="Next-step note"
            />
            <div className="breadcrumb-actions">
              <span>{breadcrumbStatus || `${breadcrumbText.length}/240`}</span>
              <button
                className="secondary-button"
                type="button"
                disabled={
                  !breadcrumbText.trim() ||
                  !(selectedWorkstreamId || continueDecision?.selected_workstream) ||
                  busyAction !== null
                }
                aria-busy={busyAction === "add_continue_breadcrumb"}
                onClick={() => void saveBreadcrumb()}
              >
                {busyAction === "add_continue_breadcrumb" ? "Saving" : "Save note"}
              </button>
            </div>
          </section>
        </section>

        <WorkstreamDetailPanel
          detail={workstreamDetail}
          decision={continueDecision}
          feedbackStatus={feedbackStatus}
          busyAction={busyAction}
          error={workstreamDetailError}
          onRefresh={() => void loadWorkstreamDetail(selectedWorkstreamId)}
          onShowEvidence={(frameId) => void revealContinueFrame(frameId)}
          onRecordFeedback={(kind, options) => void recordContinueFeedback(kind, options)}
          onContinueFromCandidate={(candidate) => void continueFromAlternative(candidate)}
        />

        <section className="memory-diagnostics-panel" aria-label="Local memory diagnostics">
          <div className="detail-section-head">
            <div>
              <h3>Local memory storage</h3>
              <span>Developer-only retention, cleanup, and budget readout</span>
            </div>
            <div className="diagnostic-actions">
              <button
                className="secondary-button"
                type="button"
                disabled={busyAction !== null}
                aria-busy={busyAction === "get_continue_decision"}
                onClick={() => void runContinueDecision({ forceRebuild: true })}
              >
                {busyAction === "get_continue_decision" ? "Rebuilding" : "Rebuild Continue"}
              </button>
              <button
                className="secondary-button"
                type="button"
                disabled={busyAction !== null}
                onClick={() => void refreshMemoryDiagnostics()}
              >
                Refresh diagnostics
              </button>
              <button
                className="secondary-button"
                type="button"
                disabled={busyAction !== null}
                aria-busy={busyAction === "cleanup_local_memory"}
                onClick={() => void runMemoryCleanup(true)}
              >
                {busyAction === "cleanup_local_memory" ? "Checking" : "Preview cleanup"}
              </button>
              <button
                className="secondary-button"
                type="button"
                disabled={busyAction !== null}
                aria-busy={busyAction === "cleanup_local_memory"}
                onClick={() => void runMemoryCleanup(false)}
              >
                {busyAction === "cleanup_local_memory" ? "Cleaning" : "Apply cleanup"}
              </button>
              <button
                className="danger-button"
                type="button"
                disabled={busyAction !== null}
                aria-busy={busyAction === "dev_reset_local_memory"}
                onClick={() => void runDevReset()}
              >
                {busyAction === "dev_reset_local_memory" ? "Resetting" : "Dev reset"}
              </button>
            </div>
          </div>
          {memoryDiagnostics ? (
            <>
              <div className="eval-grid">
                <MetricBlock label="Database" value={formatBytes(memoryDiagnostics.database_bytes)} />
                <MetricBlock label="Snapshots" value={formatBytes(memoryDiagnostics.snapshot_bytes)} />
                <MetricBlock label="Safe exports" value={formatBytes(memoryDiagnostics.safe_export_bytes)} />
                <MetricBlock label="Cleanup potential" value={formatBytes(memoryDiagnostics.estimated_cleanup_potential_bytes)} />
                <MetricBlock label="Frames" value={String(memoryDiagnostics.frame_count)} />
                <MetricBlock label="Events" value={String(memoryDiagnostics.event_count)} />
                <MetricBlock label="Protected frames" value={String(memoryDiagnostics.decision_linked_frames)} />
                <MetricBlock label="Low-value duplicates" value={String(memoryDiagnostics.low_value_duplicate_frames)} />
                <MetricBlock label="Self-capture" value={String(memoryDiagnostics.self_capture_frames)} />
                <MetricBlock label="Heavy stored" value={String(memoryDiagnostics.runtime_diagnostics.heavy_captures_stored)} />
                <MetricBlock label="Heavy skipped" value={String(memoryDiagnostics.runtime_diagnostics.heavy_captures_skipped)} />
                <MetricBlock label="Event signals" value={String(memoryDiagnostics.runtime_diagnostics.events_aggregated)} />
                <MetricBlock label="Cache hits" value={String(memoryDiagnostics.runtime_diagnostics.decision_cache_hits)} />
                <MetricBlock label="Oldest frame" value={memoryDiagnostics.oldest_retained_frame_ms ? formatTime(memoryDiagnostics.oldest_retained_frame_ms) : "None"} />
                <MetricBlock label="Last cleanup" value={memoryDiagnostics.cleanup_last_run_ms ? formatTime(memoryDiagnostics.cleanup_last_run_ms) : "Never"} />
              </div>
              <dl className="diagnostic-facts">
                <div>
                  <dt>Captured root</dt>
                  <dd>{memoryDiagnostics.captured_root}</dd>
                </div>
                <div>
                  <dt>Database path</dt>
                  <dd>{memoryDiagnostics.database_path}</dd>
                </div>
                <div>
                  <dt>Heavy capture budget</dt>
                  <dd>
                    {memoryDiagnostics.budgets.max_screenshots_per_10_minutes} screenshots per 10 minutes; low-value interval {Math.round(memoryDiagnostics.budgets.min_low_value_capture_interval_ms / 1000)}s
                  </dd>
                </div>
                <div>
                  <dt>Heavy evidence rows</dt>
                  <dd>
                    {memoryDiagnostics.heavy_evidence_rows.content_units} content units; {memoryDiagnostics.heavy_evidence_rows.ax_nodes} AX nodes; {memoryDiagnostics.heavy_evidence_rows.ocr_text_rows} OCR rows; {memoryDiagnostics.heavy_evidence_rows.ocr_spans} OCR spans
                  </dd>
                </div>
                <div>
                  <dt>Continue objects</dt>
                  <dd>
                    {memoryDiagnostics.continue_object_counts.artifacts} artifacts; {memoryDiagnostics.continue_object_counts.task_actions} actions; {memoryDiagnostics.continue_object_counts.episodes} episodes; {memoryDiagnostics.continue_object_counts.workstreams} workstreams; {memoryDiagnostics.continue_object_counts.decisions} decisions
                  </dd>
                </div>
                <div>
                  <dt>Runtime diet counters</dt>
                  <dd>
                    {memoryDiagnostics.runtime_diagnostics.heavy_captures_skipped_budget} budget skips; {memoryDiagnostics.runtime_diagnostics.heavy_captures_skipped_dedupe} dedupe skips; {memoryDiagnostics.runtime_diagnostics.heavy_captures_skipped_smalltalk_self} Smalltalk self skips; {memoryDiagnostics.runtime_diagnostics.ocr_runs} OCR runs; {memoryDiagnostics.runtime_diagnostics.ax_snapshots} AX snapshots
                  </dd>
                </div>
                <div>
                  <dt>Continue calls</dt>
                  <dd>
                    {memoryDiagnostics.runtime_diagnostics.continue_normal_calls} normal; {memoryDiagnostics.runtime_diagnostics.continue_rebuild_calls} rebuild; {memoryDiagnostics.runtime_diagnostics.decision_cache_hits} cache hits
                  </dd>
                </div>
                <div>
                  <dt>Cleanup result</dt>
                  <dd>
                    {cleanupResult?.summary || memoryDiagnostics.cleanup_last_result || "No cleanup has run yet."}
                    {cleanupResult ? ` (${cleanupResult.dry_run ? "preview" : "applied"}; ${cleanupResult.protected_frames} protected)` : ""}
                  </dd>
                </div>
              </dl>
            </>
          ) : (
            <p className="feed-empty">Open diagnostics or refresh to inspect local memory storage.</p>
          )}
        </section>

        <form
          className="search-form developer-search"
          onSubmit={(event) => {
            event.preventDefault();
            void runSearch(query);
          }}
        >
          <input
            value={query}
            onChange={(event) => setQuery(event.currentTarget.value)}
            placeholder="Search captured evidence"
            aria-label="Search captured evidence"
          />
          <button type="submit" disabled={busyAction !== null}>Search evidence</button>
        </form>

      <section className="health-strip" aria-label="Capture health">
        <StatusPill label="State" value={captureStateLabel} tone={status.running ? "good" : "quiet"} />
        <StatusPill label="Session" value={sessionLabel} tone={status.running ? "good" : "quiet"} />
        <StatusPill label="Signals" value={status.signal_count} />
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

      <section className="continue-eval-panel" aria-label="Continue eval diagnostics">
        <div className="detail-section-head">
          <div>
            <h3>Continue eval</h3>
            <span>Developer-only scoring and validation metrics</span>
          </div>
          <button
            className="secondary-button"
            type="button"
            disabled={busyAction !== null}
            aria-busy={busyAction === "run_continue_eval"}
            onClick={() => void runContinueEval()}
          >
            {busyAction === "run_continue_eval" ? "Running" : "Run eval"}
          </button>
        </div>
        {evalError ? <div className="error-box" role="alert">{evalError}</div> : null}
        {evalReport ? (
          <div className="eval-grid">
            <MetricBlock label="Cases" value={String(evalReport.case_count)} />
            <MetricBlock label="Target correct" value={`${evalReport.target_artifact_correct}/${evalReport.case_count}`} />
            <MetricBlock label="Recall@k" value={confidenceLabel(evalReport.recall_at_k)} />
            <MetricBlock label="MRR" value={confidenceLabel(evalReport.mrr)} />
            <MetricBlock label="Focus false positive" value={confidenceLabel(evalReport.current_focus_false_positive_rate)} tone={evalReport.current_focus_false_positive_rate > 0 ? "warn" : "quiet"} />
            <MetricBlock label="Hallucinated artifacts" value={String(evalReport.hallucinated_artifact_count)} tone={evalReport.hallucinated_artifact_count > 0 ? "warn" : "quiet"} />
            <MetricBlock label="Validation fallback" value={confidenceLabel(evalReport.model_validation_fallback_rate)} tone={evalReport.model_validation_fallback_rate > 0 ? "warn" : "quiet"} />
          </div>
        ) : (
          <p className="feed-empty">Run the built-in Continue fixture set to inspect product-quality metrics.</p>
        )}
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
                <h2>Selected evidence signal</h2>
                <p>Diagnostic context only; Continue decides from workstreams.</p>
              </div>
            </div>
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
                <dt>Strongest text</dt>
                <dd>{topContentUnit(frameDetail)?.text || selectedText || "Capture more evidence to inspect text."}</dd>
              </div>
            </dl>
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
      </details>
      </div>
    </main>
  );
}

function ContinueDecisionCard({
  decision,
  primaryMessage,
  hasEvidence,
  running,
  busyAction,
  sourceLabel,
  openResult,
  liveFrameCount,
  liveSignalCount,
  decisionFrameCount,
  decisionUpdatedAt,
  stale,
  onStartMemory,
  onContinue,
  onOpenTarget,
  onShowEvidence,
  onRecordFeedback,
  onUseAlternative,
}: {
  decision: ContinueDecisionResult | null;
  primaryMessage: string;
  hasEvidence: boolean;
  running: boolean;
  busyAction: string | null;
  sourceLabel: string;
  openResult: OpenResumePointResult | null;
  liveFrameCount: number;
  liveSignalCount: number;
  decisionFrameCount: number | null;
  decisionUpdatedAt: number | null;
  stale: boolean;
  onStartMemory: () => void;
  onContinue: () => void;
  onOpenTarget: () => void;
  onShowEvidence: () => void;
  onRecordFeedback: (feedbackKind: string) => void;
  onUseAlternative: (candidate: ContinueCandidateSummary) => void;
}) {
  const resumeTarget = decision?.resume_work_target || decision?.return_target || null;
  const target = resumeTarget;
  const lowConfidence = decision ? decision.confidence < 0.55 : false;
  const presentation = decision ? presentContinueDecision(decision) : null;
  const [correctionOpen, setCorrectionOpen] = useState(false);
  const [alternativesOpen, setAlternativesOpen] = useState(false);
  const refreshLabel = busyAction === "get_continue_decision"
    ? "Refreshing"
    : stale
      ? "Catching up"
      : decision
        ? "Live"
        : "Idle";
  const alternatives = decision?.alternatives || [];
  const visibleAlternatives = alternativesOpen ? alternatives.slice(0, 4) : [];

  useEffect(() => {
    setCorrectionOpen(false);
    setAlternativesOpen(false);
  }, [decision?.decision_id]);

  const recordAndClose = (feedbackKind: string) => {
    onRecordFeedback(feedbackKind);
    setCorrectionOpen(false);
  };

  if (!hasEvidence && !decision) {
    return (
      <section className="continue-card empty" aria-label="Continue decision">
        <div className="continue-card-head">
          <div>
            <p className="product-kicker">Continue</p>
            <h2>{primaryMessage}</h2>
          </div>
          <span className="trust-badge partial">No evidence</span>
        </div>
        <p className="continue-lede">
          Smalltalk needs local evidence before it can identify a workstream and return target.
        </p>
        <div className="continue-actions">
          <button
            className="primary-button"
            type="button"
            disabled={busyAction !== null}
            aria-busy={busyAction === "get_continue_decision"}
            onClick={onContinue}
          >
            {busyAction === "get_continue_decision" ? "Checking local memory" : "Continue"}
          </button>
          <button
            className="secondary-button"
            type="button"
            disabled={running || busyAction !== null}
            aria-busy={busyAction === "start_capture"}
            onClick={onStartMemory}
          >
            {busyAction === "start_capture" ? "Starting" : "Start local memory"}
          </button>
          <button className="secondary-button" type="button" onClick={onShowEvidence}>
            Inspect evidence
          </button>
        </div>
      </section>
    );
  }

  if (!decision) {
    return (
      <section className="continue-card empty" aria-label="Continue decision">
        <div className="continue-card-head">
          <div>
            <p className="product-kicker">Continue</p>
            <h2>{primaryMessage}</h2>
          </div>
          <span className="trust-badge partial">{running ? "Local memory active" : "Ready"}</span>
        </div>
        <p className="continue-lede">
          Continue runs against the local memory layer. Stopping capture is not required.
        </p>
        <div className="continue-actions">
          <button
            className="primary-button"
            type="button"
            disabled={busyAction !== null}
            aria-busy={busyAction === "get_continue_decision"}
            onClick={onContinue}
          >
            {busyAction === "get_continue_decision" ? "Finding where to continue" : "Continue"}
          </button>
          <button className="secondary-button" type="button" onClick={onShowEvidence}>
            Inspect evidence
          </button>
        </div>
      </section>
    );
  }

  return (
    <section className={`continue-card ${lowConfidence ? "low-confidence" : ""}`} aria-label="Continue decision">
      <div className="continue-card-head">
        <div>
          <p className="product-kicker">{sourceLabel}</p>
          <h2>{presentation?.workstreamTitle || "Recent workstream"}</h2>
        </div>
        <div className="continue-card-badges">
          <span className={`trust-badge ${stale ? "partial" : "complete"}`}>{refreshLabel}</span>
          <span className={`trust-badge ${lowConfidence ? "thin" : "complete"}`}>
            {presentation?.confidenceLabel || confidenceLabel(decision.confidence)}
          </span>
        </div>
      </div>

      <div className="continue-live-row" aria-label="Continue freshness">
        <span>{decisionUpdatedAt ? `Updated ${formatTime(decisionUpdatedAt)}` : "Not refreshed yet"}</span>
        <span>{`${liveSignalCount} ${liveSignalCount === 1 ? "signal" : "signals"}`}</span>
        <span>
          {decisionFrameCount === null
            ? `${liveFrameCount} evidence frames`
            : `${decisionFrameCount}/${liveFrameCount} evidence frames`}
        </span>
      </div>

      <div className="continue-state-grid">
        <div className="target-block current-focus-target">
          <span>Current focus</span>
          <strong>{presentation?.currentFocus || "No current screen returned."}</strong>
          <small>{presentation?.currentActivity || "Current activity is still thin."}</small>
        </div>
        <div className="target-block primary-target">
          <span>{lowConfidence ? "Best available return point" : "Return target"}</span>
          <strong>{presentation?.returnTarget || "No target returned"}</strong>
          <small>{presentation?.targetMeta || "Target metadata unavailable."}</small>
        </div>
      </div>

      <div className="continue-state-grid product-state-grid">
        <div className="next-action-block">
          <span>Last meaningful state</span>
          <strong>{presentation?.lastState || "No last state returned."}</strong>
        </div>
        <div className="next-action-block">
          <span>Next action</span>
          <strong>
            {presentation?.nextAction || "Open the target and continue from the last meaningful state."}
          </strong>
        </div>
      </div>

      <div className="continue-evidence-summary">
        <div>
          <span>Confidence</span>
          <strong>{presentation?.confidenceSummary || "Evidence quality unavailable."}</strong>
        </div>
        <p>{presentation?.missingEvidenceSummary || "No missing evidence called out."}</p>
      </div>

      <div className="continue-primary-action">
        <button
          className="primary-button"
          type="button"
          disabled={busyAction !== null || !target}
          aria-busy={busyAction === "open_continue_target"}
          onClick={onOpenTarget}
        >
          {busyAction === "open_continue_target" ? "Opening" : "Continue here"}
        </button>
        <button
          className="secondary-button"
          type="button"
          disabled={busyAction !== null}
          onClick={onShowEvidence}
        >
          Inspect evidence
        </button>
      </div>

      {alternatives.length > 0 ? (
        <div className="continue-alternative-summary">
          <span>Other possible continuations available.</span>
          <button
            className="text-button"
            type="button"
            onClick={() => setAlternativesOpen((open) => !open)}
          >
            {alternativesOpen ? "Hide alternatives" : "Show alternatives"}
          </button>
        </div>
      ) : null}

      {visibleAlternatives.length > 0 ? (
        <div className="alternative-list" aria-label="Alternative continuations">
          {visibleAlternatives.map((candidate) => (
            <div className="alternative-row" key={candidate.candidate_id}>
              <div>
                <strong>{presentAlternativeCandidate(candidate)}</strong>
                <span>{productizeInternalLabel(candidate.reason) || candidate.confidence_label || "Possible continuation"}</span>
              </div>
              <button
                className="secondary-button"
                type="button"
                disabled={busyAction !== null}
                onClick={() => onUseAlternative(candidate)}
              >
                Use this instead
              </button>
            </div>
          ))}
        </div>
      ) : null}

      <div className="continue-correction">
        <button
          className="text-button"
          type="button"
          disabled={busyAction !== null}
          onClick={() => setCorrectionOpen((open) => !open)}
        >
          Wrong target?
        </button>
        {correctionOpen ? (
          <div className="continue-correction-panel" aria-label="Correction controls">
            <button
              className="secondary-button"
              type="button"
              disabled={busyAction !== null}
              onClick={() => recordAndClose("rejected")}
            >
              Mark wrong target
            </button>
            <button
              className="secondary-button"
              type="button"
              disabled={busyAction !== null || alternatives.length === 0}
              onClick={() => {
                setAlternativesOpen(true);
                setCorrectionOpen(false);
              }}
            >
              Use another target
            </button>
            <button
              className="secondary-button"
              type="button"
              disabled={busyAction !== null}
              onClick={() => recordAndClose("artifact_only_evidence")}
            >
              This was only evidence
            </button>
            <button
              className="secondary-button"
              type="button"
              disabled={busyAction !== null}
              onClick={() => recordAndClose("ignored_workstream")}
            >
              Ignore this workstream
            </button>
          </div>
        ) : null}
      </div>

      {openResult ? (
        <div className="continue-open-result">
          <strong>Open target</strong>
          <span>{presentOpenResult(openResult)}</span>
        </div>
      ) : null}
    </section>
  );
}

function ContinueEvidencePanel({
  decision,
  selectedFrame,
  imageData,
  onClose,
}: {
  decision: ContinueDecisionResult | null;
  selectedFrame: CaptureFrame | null;
  imageData: string | null;
  onClose: () => void;
}) {
  const target = decision?.resume_work_target || decision?.return_target || null;
  const warnings = [
    ...(decision?.missing_evidence || []),
    ...(decision?.warnings || []),
    ...(decision?.validation_failures || []),
  ].map(productizeInternalLabel);
  const presentation = decision ? presentContinueDecision(decision) : null;

  if (!decision) {
    return (
      <section className="continue-evidence-panel empty" aria-label="Continue evidence">
        <div className="continue-evidence-head">
          <div>
            <p className="product-kicker">Continue evidence</p>
            <h2>Run Continue after local memory has evidence.</h2>
          </div>
          <button className="secondary-button" type="button" onClick={onClose}>
            Close
          </button>
        </div>
      </section>
    );
  }

  return (
    <section className="continue-evidence-panel" aria-label="Continue evidence">
      <div className="continue-evidence-head">
        <div>
          <p className="product-kicker">Continue evidence</p>
          <h2>{presentation?.workstreamTitle || continueTargetLabel(target) || "Selected workstream"}</h2>
        </div>
        <button className="secondary-button" type="button" onClick={onClose}>
          Close
        </button>
      </div>

      <div className="continue-evidence-grid">
        <dl className="continue-evidence-facts">
          <div>
            <dt>Why this workstream</dt>
            <dd>{presentation?.decisionReason || "Selected by local continuation scoring."}</dd>
          </div>
          <div>
            <dt>Return target</dt>
            <dd>{presentation?.returnTarget || "No return target returned."}</dd>
          </div>
          <div>
            <dt>Current screen</dt>
            <dd>{presentation?.currentFocus || "No current screen returned."}</dd>
          </div>
          <div>
            <dt>Last meaningful action</dt>
            <dd>{presentation?.lastState || "No action returned."}</dd>
          </div>
          <div>
            <dt>Unresolved state</dt>
            <dd>{presentation?.unresolvedState || "No unresolved state returned."}</dd>
          </div>
          <div>
            <dt>Evidence</dt>
            <dd>{presentation?.missingEvidenceSummary || "No missing evidence called out."}</dd>
          </div>
        </dl>

        <div className="anchor-preview">
          <div className="anchor-preview-head">
            <strong>Evidence anchor</strong>
            <span>{selectedFrame ? frameTitle(selectedFrame) : "No evidence selected"}</span>
          </div>
          {selectedFrame && imageData ? (
            <div className="anchor-image" style={stageStyle(selectedFrame)}>
              <img src={imageData} alt="Evidence preview" />
            </div>
          ) : (
            <div className="anchor-empty">
              <strong>No preview loaded</strong>
              <span>{selectedFrame ? frameTitle(selectedFrame) : "No evidence preview is selected."}</span>
            </div>
          )}
        </div>
      </div>

      {warnings.length ? (
        <div className="continue-warning-grid evidence-warnings">
          <WarningGroup title="Evidence notes" items={warnings} />
        </div>
      ) : null}

    </section>
  );
}

function AnchorIdGroup({ title, ids }: { title: string; ids: string[] }) {
  return (
    <div className="anchor-id-group">
      <strong>{title}</strong>
      <span>{ids.length ? ids.slice(0, 8).join(" / ") : "None"}</span>
    </div>
  );
}

function WorkstreamDetailPanel({
  detail,
  decision,
  feedbackStatus,
  busyAction,
  error,
  onRefresh,
  onShowEvidence,
  onRecordFeedback,
  onContinueFromCandidate,
}: {
  detail: ContinueWorkstreamDetailResult | null;
  decision: ContinueDecisionResult | null;
  feedbackStatus: string | null;
  busyAction: string | null;
  error: string | null;
  onRefresh: () => void;
  onShowEvidence: (frameId?: string | null) => void;
  onRecordFeedback: (
    feedbackKind: string,
    options?: {
      targetArtifactId?: string | null;
      correctedArtifactId?: string | null;
      selectedCandidateId?: string | null;
      workstreamId?: string | null;
      note?: string | null;
    },
  ) => void;
  onContinueFromCandidate: (candidate: ContinueWorkstreamCandidateDetail) => void;
}) {
  if (error) {
    return <div className="error-box" role="alert">{error}</div>;
  }

  if (!detail) {
    return (
      <section className="workstream-detail empty" aria-label="Workstream detail">
        <div>
          <p className="product-kicker">Workstream detail</p>
          <h2>Select a workstream to inspect where Continue would return.</h2>
        </div>
        <p className="continue-lede">
          Workstream detail appears after local memory has enough evidence to build episodes, artifact roles, and candidates.
        </p>
      </section>
    );
  }

  const primaryCandidate =
    detail.candidates.find(
      (candidate) =>
        candidate.candidate_id === detail.latest_decision?.selected_candidate_id,
    ) || detail.candidates[0] || null;
  const artifactGroups = groupArtifactsByRole(detail.artifacts);
  const latestFeedback = detail.feedback_events[0];

  return (
    <section className="workstream-detail" aria-label="Workstream detail">
      <div className="workstream-detail-head">
        <div>
          <p className="product-kicker">Selected workstream</p>
          <h2>
            {detail.workstream.title_candidate ||
              detail.workstream.primary_artifact_title ||
              "Recent workstream"}
          </h2>
        </div>
        <div className="workstream-detail-actions">
          <span className={`trust-badge ${detail.workstream.unresolved_signal ? "partial" : "complete"}`}>
            {sentenceCase(detail.workstream.state)}
          </span>
          <button className="secondary-button" type="button" onClick={onRefresh}>
            Refresh detail
          </button>
        </div>
      </div>

      <div className="workstream-summary-grid">
        <MetricBlock label="Confidence" value={confidenceLabel(detail.workstream.confidence)} />
        <MetricBlock label="Last active" value={formatTime(detail.workstream.last_active_timestamp_ms)} />
        <MetricBlock label="Primary artifact" value={detail.workstream.primary_artifact_title || detail.workstream.primary_artifact_id || "Unknown"} />
        <MetricBlock label="Unresolved" value={detail.workstream.unresolved_signal || "No unresolved signal"} tone={detail.workstream.unresolved_signal ? "warn" : "quiet"} />
      </div>

      <div className="workstream-target-grid">
        <section className="detail-block target-block primary-target">
          <span>Continue target</span>
          <strong>
            {primaryCandidate?.target_title ||
              primaryCandidate?.target_artifact_id ||
              continueTargetLabel(decision?.resume_work_target || decision?.return_target) ||
              "No candidate target"}
          </strong>
          <small>
            {[
              primaryCandidate ? sentenceCase(primaryCandidate.candidate_kind) : null,
              primaryCandidate?.target_openability,
              primaryCandidate ? confidenceLabel(primaryCandidate.score) : null,
            ].filter(Boolean).join(" / ") || "Target metadata unavailable."}
          </small>
        </section>
        <section className="detail-block">
          <span>Last meaningful state</span>
          <strong>{decision?.next_action || detail.latest_decision?.next_action || "No next action returned."}</strong>
          <small>
            {[
              decision?.unresolved_state || detail.workstream.unresolved_signal,
              detail.latest_decision?.validation_status,
            ].filter(Boolean).join(" / ") || "No unresolved state returned."}
          </small>
        </section>
        <section className="detail-block current-focus-target">
          <span>Current focus relationship</span>
          <strong>{continueFocusLabel(decision?.current_focus)}</strong>
          <small>
            {decision?.current_focus?.artifact_id &&
            primaryCandidate?.target_artifact_id &&
            decision.current_focus.artifact_id !== primaryCandidate.target_artifact_id
              ? "Current focus is not the return target."
              : "Current focus may be the same as the return target."}
          </small>
        </section>
      </div>

      <div className="feedback-bar" aria-label="Correction controls">
        <button
          className="secondary-button"
          type="button"
          disabled={busyAction !== null}
          onClick={() => onRecordFeedback("accepted", {
            selectedCandidateId: primaryCandidate?.candidate_id,
            targetArtifactId: primaryCandidate?.target_artifact_id,
            workstreamId: detail.workstream.id,
          })}
        >
          Correct target
        </button>
        <button
          className="secondary-button"
          type="button"
          disabled={busyAction !== null}
          onClick={() => onRecordFeedback("rejected", {
            selectedCandidateId: primaryCandidate?.candidate_id,
            targetArtifactId: primaryCandidate?.target_artifact_id,
            workstreamId: detail.workstream.id,
          })}
        >
          Wrong target
        </button>
        <button
          className="secondary-button"
          type="button"
          disabled={busyAction !== null}
          onClick={() => onRecordFeedback("artifact_only_evidence", {
            selectedCandidateId: primaryCandidate?.candidate_id,
            targetArtifactId: primaryCandidate?.target_artifact_id,
            workstreamId: detail.workstream.id,
          })}
        >
          Only evidence
        </button>
        <button
          className="secondary-button"
          type="button"
          disabled={busyAction !== null}
          onClick={() => onRecordFeedback("ignored_workstream", {
            workstreamId: detail.workstream.id,
            note: "Ignored from workstream detail.",
          })}
        >
          Ignore workstream
        </button>
        <span>{feedbackStatus || latestFeedback ? feedbackStatus || `${sentenceCase(latestFeedback.event_kind)} feedback` : "Pending feedback"}</span>
      </div>

      <div className="detail-section-grid">
        <section className="detail-section">
          <div className="detail-section-head">
            <h3>Artifact roles</h3>
            <span>{detail.artifacts.length}</span>
          </div>
          {Object.entries(artifactGroups).map(([role, artifacts]) => (
            <div className="artifact-role-group" key={role}>
              <strong>{sentenceCase(role)}</strong>
              {artifacts.map((artifact) => (
                <div className="artifact-role-row" key={`${artifact.artifact_id}-${artifact.durable_role}`}>
                  <span>{detailArtifactLabel(artifact)}</span>
                  <small>
                    {[
                      sentenceCase(artifact.artifact_kind),
                      artifact.openability,
                      artifact.privacy_status,
                      artifact.reason,
                    ].filter(Boolean).join(" / ")}
                  </small>
                  <button
                    className="text-button"
                    type="button"
                    onClick={() => onShowEvidence(artifact.last_seen_frame_id || artifact.first_seen_frame_id)}
                  >
                    Inspect evidence
                  </button>
                </div>
              ))}
            </div>
          ))}
          {detail.artifacts.length === 0 ? (
            <div className="workstream-empty">
              <strong>No artifact roles yet</strong>
              <span>Run Continue after local memory collects more evidence.</span>
            </div>
          ) : null}
        </section>

        <section className="detail-section">
          <div className="detail-section-head">
            <h3>Candidate targets</h3>
            <span>{detail.candidates.length}</span>
          </div>
          {detail.candidates.slice(0, 6).map((candidate) => (
            <div className="candidate-row" key={candidate.candidate_id}>
              <div>
                <strong>{candidate.target_title || candidate.target_artifact_id || sentenceCase(candidate.candidate_kind)}</strong>
                <span>{candidate.reason || "No local reason returned."}</span>
                <small>
                  {[
                    sentenceCase(candidate.candidate_kind),
                    candidate.confidence_label || confidenceLabel(candidate.score),
                    candidate.target_openability,
                  ].filter(Boolean).join(" / ")}
                </small>
              </div>
              <button
                className="secondary-button"
                type="button"
                disabled={busyAction !== null}
                onClick={() => onContinueFromCandidate(candidate)}
              >
                Continue from this
              </button>
            </div>
          ))}
          {detail.candidates.length === 0 ? (
            <div className="workstream-empty">
              <strong>No candidates yet</strong>
              <span>Refresh Continue to generate continuation candidates.</span>
            </div>
          ) : null}
        </section>
      </div>

      <section className="detail-section full">
        <div className="detail-section-head">
          <h3>Episodes and actions</h3>
          <span>{detail.episodes.length}</span>
        </div>
        <div className="episode-stack">
          {detail.episodes.map((episode) => (
            <article className="episode-card" key={episode.id}>
              <div className="episode-head">
                <div>
                  <strong>{episode.summary_label || sentenceCase(episode.dominant_action_kind) || "Episode"}</strong>
                  <span>
                    {[
                      sentenceCase(episode.state),
                      episode.primary_artifact_title || episode.primary_artifact_id,
                      formatTime(episode.start_timestamp_ms),
                    ].filter(Boolean).join(" / ")}
                  </span>
                </div>
                <button
                  className="text-button"
                  type="button"
                  onClick={() => onShowEvidence(episode.end_frame_id || episode.start_frame_id)}
                >
                  Inspect frame
                </button>
              </div>
              <div className="action-list">
                {episode.actions.slice(0, 8).map((action) => (
                  <div className="action-row" key={action.action_id}>
                    <strong>{sentenceCase(action.action_kind)}</strong>
                    <span>{action.reason || action.artifact_title || action.artifact_id || "No local reason returned."}</span>
                    <small>
                      {[
                        action.role_in_episode,
                        `frame ${action.frame_id}`,
                        confidenceLabel(action.confidence),
                      ].join(" / ")}
                    </small>
                  </div>
                ))}
              </div>
            </article>
          ))}
        </div>
      </section>

      <section className="detail-section full">
        <div className="detail-section-head">
          <h3>Evidence anchors and feedback</h3>
          <span>{continueAnchorSummary(detail.evidence_anchors)}</span>
        </div>
        <div className="anchor-id-grid">
          <AnchorIdGroup title="Frames" ids={detail.evidence_anchors.frame_ids} />
          <AnchorIdGroup title="Actions" ids={detail.evidence_anchors.action_ids} />
          <AnchorIdGroup title="Episodes" ids={detail.evidence_anchors.episode_ids} />
          <AnchorIdGroup title="Artifacts" ids={detail.evidence_anchors.artifact_ids} />
        </div>
        <div className="feedback-list">
          {detail.feedback_events.slice(0, 4).map((event) => (
            <div className="feedback-row" key={event.id}>
              <strong>{sentenceCase(event.event_kind)}</strong>
              <span>{event.note || event.reason || "Feedback recorded."}</span>
              <small>{formatTime(event.timestamp_ms)} / {event.source || "local"}</small>
            </div>
          ))}
          {detail.breadcrumbs.slice(0, 4).map((breadcrumb) => (
            <div className="feedback-row" key={breadcrumb.id}>
              <strong>Next-step note</strong>
              <span>{breadcrumb.text}</span>
              <small>{formatTime(breadcrumb.created_at_ms)} / {breadcrumb.source}</small>
            </div>
          ))}
        </div>
      </section>
    </section>
  );
}

function WarningGroup({ title, items }: { title: string; items: string[] }) {
  return (
    <div className="warning-group">
      <strong>{title}</strong>
      {items.slice(0, 4).map((item) => (
        <span key={item}>{item}</span>
      ))}
    </div>
  );
}

function WorkstreamList({
  workstreams,
  selectedWorkstreamId,
  onRefresh,
  onSelect,
}: {
  workstreams: RecentContinueWorkstream[];
  selectedWorkstreamId?: string | null;
  onRefresh: () => void;
  onSelect: (workstreamId: string) => void;
}) {
  const grouped = groupWorkstreams(workstreams);

  return (
    <section className="workstream-card" aria-label="Workstreams">
      <div className="workstream-head">
        <div>
          <h2>Workstreams</h2>
          <p>Recent continuation candidates, not sessions.</p>
        </div>
        <button className="secondary-button" type="button" onClick={onRefresh}>
          Refresh
        </button>
      </div>
      {workstreams.length === 0 ? (
        <div className="workstream-empty">
          <strong>No workstreams yet</strong>
          <span>Run Continue after local memory has evidence.</span>
        </div>
      ) : (
        Object.entries(grouped).map(([state, rows]) => (
          <div className="workstream-group" key={state}>
            <div className="workstream-group-label">
              <strong>{sentenceCase(state)}</strong>
              <span>{rows.length}</span>
            </div>
            {rows.map((workstream) => (
              <button
                className={
                  selectedWorkstreamId === workstream.id
                    ? "workstream-row active"
                    : "workstream-row"
                }
                key={workstream.id}
                type="button"
                onClick={() => onSelect(workstream.id)}
              >
                <strong>{workstream.title_candidate || workstream.primary_artifact_title || "Recent workstream"}</strong>
                <span>
                  {[
                    workstream.primary_artifact_title || workstream.primary_artifact_id,
                    workstream.unresolved_signal ? "unresolved" : null,
                  ].filter(Boolean).join(" / ") || workstream.source}
                </span>
                <small>
                  {[
                    sentenceCase(workstream.state),
                    confidenceLabel(workstream.confidence),
                    formatTime(workstream.last_active_timestamp_ms),
                  ].join(" / ")}
                </small>
              </button>
            ))}
          </div>
        ))
      )}
    </section>
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

type ContinuePresentation = {
  workstreamTitle: string;
  currentFocus: string;
  currentActivity: string;
  returnTarget: string;
  targetMeta: string;
  lastState: string;
  unresolvedState: string;
  nextAction: string;
  confidenceLabel: string;
  confidenceSummary: string;
  missingEvidenceSummary: string;
  decisionReason: string;
};

function presentContinueDecision(decision: ContinueDecisionResult): ContinuePresentation {
  const target = decision.resume_work_target || decision.return_target || null;
  const unresolvedState = productizeUnresolvedState(
    decision.unresolved_state || decision.selected_workstream?.unresolved_signal,
  );
  const lastAction = productizeAction(decision.last_meaningful_action);
  const missingEvidence = [
    ...decision.missing_evidence,
    ...decision.warnings,
    ...decision.validation_failures,
  ]
    .map(productizeInternalLabel)
    .filter(Boolean);
  const targetLabel = humanTargetLabel(target);
  const workstreamTitle = cleanHumanText(decision.selected_workstream?.title_candidate)
    || targetLabel
    || "Recent workstream";
  const confidence = decision.confidence_label
    ? sentenceCase(decision.confidence_label)
    : confidenceLabel(decision.confidence);
  const confidenceSummary = missingEvidence.length
    ? `${confidence}; ${missingEvidence[0]}`
    : `${confidence}; evidence is enough for a local recommendation.`;

  return {
    workstreamTitle,
    currentFocus: humanFocusLabel(decision.current_focus),
    currentActivity: productizeInternalLabel(decision.current_activity || ""),
    returnTarget: targetLabel || "No stable return target yet",
    targetMeta: humanTargetMeta(target),
    lastState: lastAction || unresolvedState || "No meaningful prior state is clear yet.",
    unresolvedState,
    nextAction: productizeInternalLabel(
      decision.next_action || "Open the target and continue from the last meaningful state.",
    ),
    confidenceLabel: confidence,
    confidenceSummary,
    missingEvidenceSummary: summarizeProductEvidence(missingEvidence),
    decisionReason: productizeCandidateKind(decision.candidate_kind)
      || unresolvedState
      || "Selected from local workstream evidence.",
  };
}

function humanTargetLabel(target?: ContinueReturnTarget | null) {
  if (!target) return "";
  return cleanHumanText(target.title)
    || pathBasename(target.document_path || target.browser_url)
    || productizeArtifactKind(target.artifact_kind)
    || "";
}

function humanTargetMeta(target?: ContinueReturnTarget | null) {
  if (!target) return "No stable target metadata yet.";
  const parts = [
    productizeArtifactKind(target.artifact_kind),
    productizeOpenability(target.openability),
  ].filter(Boolean);
  return parts.join(" / ") || "Target metadata is thin.";
}

function humanFocusLabel(focus?: ContinueFocusSummary | null) {
  if (!focus) return "No current screen returned.";
  return cleanHumanText(focus.title || focus.window_title || focus.app_name)
    || productizeArtifactKind(focus.artifact_kind)
    || "Current screen";
}

function productizeAction(action?: ContinueActionSummary | null) {
  if (!action) return "";
  const label = productizeActionKind(action.action_kind);
  return label ? `${label} ${formatRelativeAge(action.timestamp_ms)}` : "";
}

function productizeActionKind(value?: string | null) {
  const key = normalizeToken(value);
  const labels: Record<string, string> = {
    reading: "Reading was the last clear activity.",
    editing: "Editing was in progress.",
    composing: "A draft or composer was active.",
    searching: "The user had branched into search.",
    copying_evidence: "Evidence was copied for use elsewhere.",
    reviewing_output: "Output was being reviewed.",
    running_command: "A command had just been run.",
    observing_command_output: "Command output was being checked.",
    encountering_error: "Error still visible.",
    navigating: "The user was navigating within the current surface.",
    switching_context: "The user switched context.",
    branching_away: "The user branched away from the target.",
    returning_to_origin: "The user returned to the original target.",
    idle_after_progress: "Work paused after meaningful progress.",
    messaging_interrupt: "Messaging interrupted the workstream.",
    verification_branch: "A verification branch was open.",
    possible_distraction: "The current screen looks like a possible distraction.",
  };
  return labels[key] || (key ? sentenceCase(key) : "");
}

function productizeCandidateKind(value?: string | null) {
  const key = normalizeToken(value);
  const labels: Record<string, string> = {
    continue_edit: "Continue the edit in the primary target.",
    return_to_primary_artifact: "Return to the primary work target.",
    resolve_error: "Resolve the visible blocker.",
    verify_output: "Verify the output before moving on.",
    continue_reply: "Continue the draft or reply.",
    read_next_source: "Continue reading the next source.",
    finish_search: "Finish the search branch and apply it back to the target.",
    rerun_command: "Rerun or inspect the command result.",
    resume_chat_reasoning: "Resume reasoning in the conversation.",
    evidence_only: "This looks like evidence, not the return target.",
  };
  return labels[key] || "";
}

function productizeUnresolvedState(value?: string | null) {
  if (!value) return "";
  const kind = unresolvedKind(value);
  const labels: Record<string, string> = {
    idle_after_progress: "Work paused after meaningful progress.",
    visible_error_or_failure: "Visible error still unresolved.",
    draft_or_composer_active: "Draft or composer still active.",
    verification_without_return: "Verification branch has not been applied back to the target.",
    branch_without_return: "Search branch has not been applied back to the target.",
  };
  return labels[kind] || sentenceCase(kind) || "Unresolved state still present.";
}

function unresolvedKind(value: string) {
  const trimmed = value.trim();
  if (trimmed.startsWith("{")) {
    try {
      const parsed = JSON.parse(trimmed) as { kind?: unknown };
      if (typeof parsed.kind === "string") return normalizeToken(parsed.kind);
    } catch {
      return normalizeToken(trimmed);
    }
  }
  return normalizeToken(trimmed);
}

function productizeInternalLabel(value?: string | null) {
  const raw = cleanHumanText(value);
  if (!raw) return "";
  const key = normalizeToken(raw);
  const labels: Record<string, string> = {
    error_signal: "An error or failure was visible.",
    unresolved_error_signal: "There appears to be an unresolved error.",
    typing_in_composer: "A draft or composer was active.",
    frame_fallback: "This target is based on visible screen evidence.",
    primary_artifact_fallback: "This looks like the main place to continue.",
    last_meaningful_error: "The last meaningful state was an error/blocker.",
    secondary_artifact_for_searching: "Search was treated as supporting evidence.",
    current_focus_differs_from_return_target: "Current screen is not the return target.",
    thin_evidence: "Evidence is thin.",
    no_last_meaningful_action: "No clear last action was captured.",
    no_openable_target: "No directly openable target was found.",
    no_candidate_generated: "No continuation candidate could be generated yet.",
    micro_inference_missing_openai_api_key: "Model ranking is unavailable; using local scoring.",
    smalltalk_self_observation_downranked: "Smalltalk's own UI was treated as low-value diagnostic evidence.",
    branch_surface_is_evidence_not_default_return_target: "Branch surface is evidence, not the default return target.",
    privacy_sensitive_or_redacted_target_local_only: "Target contains sensitive or redacted evidence and stays local.",
    current_focus_mismatch: "Current screen is not the return target.",
  };
  if (labels[key]) return labels[key];
  if (raw.includes(":")) {
    const prefix = normalizeToken(raw.split(":")[0]);
    if (labels[prefix]) return labels[prefix];
  }
  if (raw.startsWith("{")) {
    return productizeUnresolvedState(raw);
  }
  return sentenceCase(raw).replace(/\b(id|json)\b/gi, "").replace(/\s+/g, " ").trim();
}

function summarizeProductEvidence(items: string[]) {
  const unique = [...new Set(items.filter(Boolean))];
  if (unique.length === 0) {
    return "No major missing evidence called out.";
  }
  if (unique.length === 1) return unique[0];
  return `${unique[0]} ${unique.length - 1} more evidence note${unique.length > 2 ? "s" : ""}.`;
}

function productizeArtifactKind(value?: string | null) {
  const key = normalizeToken(value);
  const labels: Record<string, string> = {
    browser_tab: "Browser tab",
    chat_conversation: "Conversation",
    code_editor: "Code editor",
    terminal: "Terminal",
    pdf: "PDF",
    finder: "Finder",
    messaging: "Messaging",
    notes_doc: "Document",
    unknown: "",
  };
  return labels[key] ?? sentenceCase(key);
}

function productizeOpenability(value?: string | null) {
  const key = normalizeToken(value);
  const labels: Record<string, string> = {
    openable: "Openable",
    frame_fallback: "Needs evidence preview",
    blocked: "Opening may be blocked",
    unknown: "Openability unknown",
  };
  return labels[key] || "";
}

function presentOpenResult(result: OpenResumePointResult) {
  if (result.warnings.length > 0) {
    return productizeInternalLabel(result.warnings[0]);
  }
  if (result.opened_url) return "Opened the selected target.";
  if (result.strategy.startsWith("smalltalk_")) {
    return "Could not open directly; focused Smalltalk instead.";
  }
  return "Attempted to open the selected target.";
}

function presentAlternativeCandidate(candidate: ContinueCandidateSummary) {
  return productizeCandidateKind(candidate.candidate_kind)
    || productizeInternalLabel(candidate.reason)
    || "Alternative continuation";
}

function normalizeToken(value?: string | null) {
  return cleanHumanText(value).toLowerCase().replace(/[^a-z0-9]+/g, "_").replace(/^_+|_+$/g, "");
}

function cleanHumanText(value?: string | null) {
  if (!value) return "";
  if (looksLikeInternalId(value)) return "";
  return value.split(/\s+/).join(" ").trim();
}

function looksLikeInternalId(value: string) {
  const trimmed = value.trim();
  return /^(frame|action|artifact|episode|workstream|continue-decision|task-action)-?[a-z0-9_-]+$/i.test(trimmed)
    || /^-?\d+$/.test(trimmed);
}

function continueTargetLabel(target?: ContinueReturnTarget | null) {
  if (!target) return "";
  return (
    target.title ||
    target.artifact_kind ||
    pathBasename(target.document_path || target.browser_url) ||
    target.artifact_id ||
    ""
  );
}

function continueFocusLabel(focus?: ContinueFocusSummary | null) {
  if (!focus) return "No current screen returned.";
  return [
    focus.title || focus.window_title || focus.app_name || "Current screen",
    focus.artifact_kind,
    `frame ${focus.frame_id}`,
  ]
    .filter(Boolean)
    .join(" / ");
}

function continueAnchorSummary(anchors: ContinueEvidenceAnchors) {
  const parts = [
    anchors.frame_ids.length ? `${anchors.frame_ids.length} frames` : null,
    anchors.action_ids.length ? `${anchors.action_ids.length} actions` : null,
    anchors.episode_ids.length ? `${anchors.episode_ids.length} episodes` : null,
    anchors.artifact_ids.length ? `${anchors.artifact_ids.length} artifacts` : null,
  ].filter(Boolean);
  return parts.join(" / ") || "No anchors returned.";
}

function groupWorkstreams(workstreams: RecentContinueWorkstream[]) {
  return workstreams.reduce<Record<string, RecentContinueWorkstream[]>>((groups, workstream) => {
    const key = workstream.state || "unknown";
    groups[key] = groups[key] || [];
    groups[key].push(workstream);
    return groups;
  }, {});
}

function groupArtifactsByRole(artifacts: ContinueWorkstreamArtifactDetail[]) {
  return artifacts.reduce<Record<string, ContinueWorkstreamArtifactDetail[]>>((groups, artifact) => {
    const key = artifact.durable_role || "unknown";
    groups[key] = groups[key] || [];
    groups[key].push(artifact);
    return groups;
  }, {});
}

function detailArtifactLabel(artifact: ContinueWorkstreamArtifactDetail) {
  return (
    artifact.display_title ||
    pathBasename(artifact.document_path || artifact.browser_url) ||
    artifact.window_title ||
    artifact.app_name ||
    artifact.artifact_id
  );
}

function MetricBlock({
  label,
  value,
  tone = "quiet",
}: {
  label: string;
  value: string;
  tone?: "quiet" | "warn";
}) {
  return (
    <div className={`metric-block ${tone}`}>
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}

function sentenceCase(value?: string | null) {
  if (!value) return "";
  const text = value.replace(/[_-]+/g, " ").trim();
  return text ? text.charAt(0).toUpperCase() + text.slice(1) : "";
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

function formatRelativeAge(value?: number | null) {
  if (!value) return "No evidence yet";
  const elapsedMs = Math.max(0, Date.now() - value);
  const seconds = Math.floor(elapsedMs / 1000);
  if (seconds < 10) return "just now";
  if (seconds < 60) return `${seconds}s ago`;
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}h ago`;
  const days = Math.floor(hours / 24);
  return `${days}d ago`;
}

function formatBytes(value: number) {
  if (!Number.isFinite(value) || value <= 0) return "0 B";
  const units = ["B", "KB", "MB", "GB"];
  let size = value;
  let unitIndex = 0;
  while (size >= 1024 && unitIndex < units.length - 1) {
    size /= 1024;
    unitIndex += 1;
  }
  return `${size >= 10 || unitIndex === 0 ? Math.round(size) : size.toFixed(1)} ${units[unitIndex]}`;
}

function pathBasename(path?: string | null) {
  if (!path) return "";
  return path.split(/[\\/]/).filter(Boolean).pop() || path;
}

function cleanSnippet(value?: string | null) {
  if (!value) return "No text";
  return value.replace(/\[/g, "").replace(/\]/g, "").replace(/\s+/g, " ").trim();
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

function topContentUnit(detail: FrameDetail | null) {
  if (!detail) return null;
  return detail.content_units.find((unit) => unit.text && unit.text.length > 24) || detail.content_units[0] || null;
}

export default App;
