import {
  type CSSProperties,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { useGSAP } from "@gsap/react";
import {
  ArrowBendDownRight,
  ArrowsClockwise,
  Brain,
  Browser,
  CaretDown,
  CaretLeft,
  CaretRight,
  CaretUp,
  ClockCounterClockwise,
  Code,
  Compass,
  Database,
  DiscordLogo,
  DotsThree,
  Eye,
  FigmaLogo,
  GithubLogo,
  GlobeSimple,
  GoogleChromeLogo,
  ChatCircleText,
  Monitor,
  NotionLogo,
  OpenAiLogo,
  ShieldCheck,
  SidebarSimple,
  SignOut,
  SlackLogo,
  SlidersHorizontal,
  TerminalWindow,
} from "@phosphor-icons/react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { gsap } from "gsap";
import {
  authoritativeTaskTruthActionState,
  authoritativeTaskTruthAnswer,
  authoritativeTaskTruthTarget,
  buildContinueContinuationFieldProjection,
  buildContinuePublicProjection,
  compareContinueDecisionAdoption,
  getContinuePresentationActionState,
  hasVisibleTaskTruthContinuationDetails,
  hasVisibleTaskTruthSemantics,
  inspectTargetCopy,
  isDirectPresentationTargetOpenable,
  recentContextSurfaceLabel,
  recentContextSurfaceKind,
  taskInferenceFailurePresentation,
  NO_CLEAR_CURRENT_TASK_COPY,
  NO_CLEAR_CURRENT_TASK_HEADLINE,
  normalizeTaskResolutionStatus,
  selectPrimaryTaskHeadline,
  type ContinueAlternativeHypothesis,
  type ContinueCurrentTaskTurnSummary,
  type ContinueDecisionRequestTrigger,
  type ContinueDecisionSupportedSurface,
  type ContinueEvidenceFreshnessSummary,
  type ContinuePresentationActionState,
  type ContinueTaskResolutionStatus,
  type ContinueTaskTruthAnswer,
  type ContinueTaskTruthRecentContext,
  type ContinueSurfaceProjection,
} from "./continuePresentation";
import {
  continueRequestErrorCopy,
  isTransientScreenshotCaptureContention,
  withContinueRequestTimeout,
} from "./continueRequest";
import { MosaicLeafBackground } from "./MosaicLeafBackground";
import { useAuth } from "./auth/AuthProvider";
import { useAppUpdate } from "./updates/AppUpdateProvider";
import { appUpdateStatusCopy } from "./updates/updatePresentation";
import smalltalkLogo from "./assets/smalltalk-logo.png";
import "@fontsource/instrument-serif/400.css";
import "./App.css";

gsap.registerPlugin(useGSAP);

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
  active_window_capture_provider?: string | null;
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
  capture_runtime_state: string;
  worker_generation: number;
  current_operation_class?: string | null;
  current_operation_started_at_ms?: number | null;
  last_operation_class?: string | null;
  last_operation_duration_ms?: number | null;
  helper_launches: number;
  helper_successes: number;
  helper_timeouts: number;
  helper_timeouts_reaped: number;
  helper_cancellations: number;
  helper_abnormal_exits: number;
  helper_output_limit_failures: number;
  helper_launch_failures: number;
  active_child_processes: number;
  last_safe_error_category?: string | null;
  stop_latency_ms?: number | null;
  worker_panic_count: number;
  provider_health: Record<string, string>;
  provider_by_operation: Record<string, string>;
  fallback_counts_by_operation: Record<string, number>;
  provider_circuit_breaker_opens: number;
  provider_recovery_probes: number;
  event_pipeline: {
    queue_depth: number;
    queue_capacity: number;
    high_queue_depth: number;
    normal_queue_depth: number;
    pressure_queue_depth: number;
    high_water_mark: number;
    coalesced_count: number;
    dropped_count: number;
    dropped_pressure_count: number;
    dropped_normal_count: number;
    dropped_high_count: number;
    shutdown: boolean;
  };
  schema_initialization_count: number;
  migration_execution_count: number;
  database_busy_retry_count: number;
  database_busy_time_ms: number;
  database_generation: number;
  audit_executor: {
    active: boolean;
    queued: number;
    queue_capacity: number;
    submitted: number;
    completed: number;
    failed: number;
    coalesced: number;
    superseded: number;
    shutting_down: boolean;
  };
  status_metrics: {
    sample_count: number;
    p50_latency_us: number;
    p95_latency_us: number;
    last_response_bytes: number;
    max_response_bytes: number;
  };
  workload: {
    active_operations: string[];
    queued_operation_count: number;
    queue_capacity: number;
    queue_high_water_mark: number;
    queued_by_class: Record<string, number>;
    queue_capacity_by_class: Record<string, number>;
    rejected_by_class: Record<string, number>;
    coalesced_requests: number;
    cancelled_or_superseded_requests: number;
    background_decisions_avoided: number;
    shutting_down: boolean;
    duration_percentiles_ms: Record<string, { p50: number; p95: number }>;
  };
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
  continue_output_audit_failures: number;
  weak_surface_enrichment_attempts: number;
  weak_surface_enrichment_success_strong: number;
  weak_surface_enrichment_success_medium: number;
  weak_surface_enrichment_success_thin: number;
  weak_surface_enrichment_skipped_privacy: number;
  weak_surface_enrichment_skipped_budget: number;
  weak_surface_enrichment_failed: number;
  latest_weak_surface_attempt?: string | null;
  latest_weak_surface_snapshot_id?: string | null;
  sck_display_successes: number;
  sck_active_window_successes: number;
  sck_active_window_abnormal_exits: number;
  sck_timeouts: number;
  sck_circuit_breaker_opens: number;
  screencapture_fallbacks: number;
  latest_sck_capture_mode?: string | null;
  latest_sck_provider?: string | null;
  latest_sck_duration_ms?: number | null;
  latest_sck_exit_category?: string | null;
  latest_sck_fallback_used?: boolean | null;
  sck_active_window_circuit_breaker_state: string;
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

type ScreenCapturePermissionStatus = {
  state: "granted" | "request_required" | "restart_required" | "unsupported";
  granted: boolean;
  can_request: boolean;
  request_attempted: boolean;
  restart_required: boolean;
  message: string;
  settings_hint?: string | null;
  identity: {
    executable_path: string;
    executable_name?: string | null;
    bundle_identifier?: string | null;
    bundle_path?: string | null;
    signing_identifier?: string | null;
    team_identifier?: string | null;
    designated_requirement?: string | null;
    cdhash?: string | null;
    signature_kind: string;
    request_scope_key: string;
  };
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
    semantic_moments: number;
    task_actions: number;
    episodes: number;
    workstreams: number;
    open_loops: number;
    candidates: number;
    decisions: number;
    open_events: number;
    feedback_events: number;
    breadcrumbs: number;
  };
  low_value_duplicate_frames: number;
  excess_low_value_events: number;
  self_capture_frames: number;
  self_capture_events: number;
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
    max_retained_low_value_ui_events: number;
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
  deleted_event_rows: number;
  deleted_snapshot_files: number;
  reclaimed_bytes: number;
  summary: string;
};

type ExclusionRule = {
  id: string;
  rule_type: string;
  pattern: string;
  action: string;
  enabled: boolean;
  origin: "system_default" | "user";
  created_at_ms: number;
};

type ExclusionRuleInput = {
  rule_type: string;
  pattern: string;
  action: string;
  enabled?: boolean;
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
  weak_surface_classification?: WeakSurfaceClassification | null;
  events: TimelineEvent[];
  ax_nodes: BoxLike[];
  ocr_spans: BoxLike[];
  content_units: BoxLike[];
  app_contexts: AppContextSummary[];
  sensitive_regions: BoxLike[];
  windows: WindowSummary[];
  transitions: TransitionSummary[];
};

type WeakSurfaceClassification = {
  domain: string;
  enrichment_need: string;
  confidence: number;
  reasons: string[];
  adapter_key?: string | null;
  privacy_tier: string;
  observed_app_name?: string | null;
  observed_bundle_id?: string | null;
  observed_window_title?: string | null;
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
    open_loops: number;
    open_loop_artifacts: number;
    open_loop_evidence: number;
    candidates: number;
    decisions: number;
    open_events: number;
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
  domain?: string | null;
  app_name?: string | null;
  window_title?: string | null;
  title?: string | null;
  display_title?: string | null;
  browser_url?: string | null;
  document_path?: string | null;
  activity_state?: string | null;
  task_state?: string | null;
  evidence_quality?: string | null;
  identity_confidence?: number | null;
  snapshot_id?: string | null;
  missing_fields?: string[];
  openability?: string | null;
  captured_at_ms: number;
};

type WeakSurfaceEnrichmentAttempt = {
  attempt_id: string;
  observed_at_ms: number;
  scheduled_at_ms: number;
  completed_at_ms?: number | null;
  surface_key: string;
  weak_domain: string;
  app_name?: string | null;
  bundle_id?: string | null;
  window_title_hash?: string | null;
  window_title_capped?: string | null;
  window_id?: number | null;
  trigger_event_ids: string[];
  trigger_type: string;
  attempt_index: number;
  status: string;
  reason?: string | null;
  snapshot_id?: string | null;
  missing_fields: string[];
  adapter_key?: string | null;
};

type WeakSurfaceEnrichmentDiagnostics = {
  weak_surface_enrichment_attempts: number;
  weak_surface_enrichment_success_strong: number;
  weak_surface_enrichment_success_medium: number;
  weak_surface_enrichment_success_thin: number;
  weak_surface_enrichment_skipped_privacy: number;
  weak_surface_enrichment_skipped_budget: number;
  weak_surface_enrichment_failed: number;
  latest_weak_surface_attempt?: WeakSurfaceEnrichmentAttempt | null;
  latest_weak_surface_snapshot_id?: string | null;
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

type ContinueTargetTruth = {
  schema: string;
  state:
    | "direct_continue_ready"
    | "task_known_target_unknown"
    | "thin_task_seen"
    | "no_clear_task"
    | "stale_decision"
    | "target_suppressed"
    | "support_only"
    | string;
  reason_codes: string[];
};

type ContinueEvidencePreview = {
  schema: string;
  preview_kind: "frame" | string;
  frame_id: string;
};

type ContinueInterruptionRecoveryAnswer = {
  schema: string;
  what_you_were_doing?: string | null;
  where_label?: string | null;
  where_you_left_off?: string | null;
  next: string;
  action: "continue_here" | "inspect_evidence" | "view_summary" | string;
  target_note?: string | null;
  task_confidence_label: string;
  target_confidence_label: string;
};

type ClaimConfidenceSummary = {
  score: number;
  label: "none" | "low" | "medium" | "high" | string;
  missing_evidence: string[];
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
  risk_flags?: string[];
  score_caps_applied?: string[];
  eligible_for_primary_selection?: boolean;
  public_alternative_eligible_after_feedback?: boolean;
  feedback_suppression_state?: string;
  feedback_negative_weight?: number;
  feedback_positive_weight_after_last_negative?: number;
  feedback_last_negative_ms?: number | null;
  feedback_last_reconfirming_evidence_ms?: number | null;
  feedback_score_cap?: number | null;
  feedback_reason_codes?: string[];
  branch_promotion_state?: string | null;
  branch_public_return_eligible?: boolean | null;
  branch_promotion_reason?: string | null;
  branch_kind?: string | null;
  branch_eligibility_state?: string | null;
  public_return_eligible?: boolean;
  blocked_reason?: string | null;
  evidence_frame_id?: string | null;
  supporting_episode_id?: string | null;
  last_meaningful_action_id?: string | null;
  app_family?: string | null;
  surface_type?: string | null;
  activity_intent?: string | null;
  task_phase?: string | null;
  continuation_role?: string | null;
  work_value_reason?: string | null;
  why_not_primary?: string | null;
  candidate_score_components?: ContinueCandidateScoreTrace;
};

type ContinueScoreComponents = {
  actionability: number;
  primary_target: number;
  unresolved_state: number;
  branch_origin: number;
  evidence_quality: number;
  recency: number;
  fresh_current_work?: number;
  openability: number;
  privacy_safety: number;
  memory_support?: number;
  memory_contradiction?: number;
  feedback_prior?: number;
  retrieval_confidence?: number;
  work_value?: number;
  resume_likelihood?: number;
  divergence?: number;
  diagnostic?: number;
  objective_relation?: number;
  interaction_depth?: number;
  evidence_sufficiency?: number;
};

type ContinueCandidateScoreTrace = {
  fresh_current_work_score: number;
  stale_mismatch_cap_applied: boolean;
  openability_score: number;
  recency_score: number;
  evidence_quality_score: number;
  feedback_suppression_state?: string;
  feedback_negative_weight?: number;
  feedback_last_negative_ms?: number | null;
  feedback_last_reconfirming_evidence_ms?: number | null;
  feedback_reason_codes?: string[];
};

type ContinueActivitySummary = {
  main_work?: string | null;
  support_context: string[];
  recent_divergence: string[];
  diagnostic_surfaces: string[];
  missing_for_current_focus: string[];
};

type ActivityConfidence = "none" | "low" | "medium" | "high";

type ActivityEvidenceConfidence = "low" | "medium" | "high";

type ActivityDetourSummary = {
  surface_title?: string | null;
  app_name?: string | null;
  role:
    | "support"
    | "detour"
    | "interrupt"
    | "current_focus_only"
    | "promoted_primary"
    | "unclear";
  activity_label?: string | null;
  reason: string;
  start_ms?: number | null;
  end_ms?: number | null;
  confidence: ActivityEvidenceConfidence;
  evidence_anchor_ids: string[];
};

type ActivitySupportSummary = {
  summary: string;
  role:
    | "source_evidence"
    | "branch_support"
    | "output_verification"
    | "blocker"
    | "message_interrupt"
    | "diagnostic"
    | "unknown";
  confidence: ActivityEvidenceConfidence;
  evidence_anchor_ids: string[];
};

type ActivityEvidenceSpan = {
  claim_key: string;
  claim_text: string;
  anchor_type:
    | "frame"
    | "event"
    | "action"
    | "episode"
    | "workstream"
    | "open_loop"
    | "branch"
    | "surface_snapshot"
    | "memory_cell";
  anchor_ids: string[];
  confidence: ActivityEvidenceConfidence;
  source: "local" | "model_validated";
};

type ContinueActivityRecap = {
  schema: "smalltalk.activity_recap.v1";
  primary_work_summary?: string | null;
  primary_work_label?: string | null;
  primary_where_summary?: string | null;
  activity_confidence: ActivityConfidence;
  target_confidence: ActivityConfidence;
  current_state:
    | "actively_working"
    | "recently_detoured"
    | "paused_after_progress"
    | "blocked"
    | "complete_or_idle"
    | "unclear";
  last_meaningful_state?: string | null;
  unfinished_state?: string | null;
  next_action_summary?: string | null;
  recent_detours: ActivityDetourSummary[];
  supporting_context: ActivitySupportSummary[];
  why_this_target?: string | null;
  why_no_safe_target?: string | null;
  missing_evidence: string[];
  warnings: string[];
  evidence_spans: ActivityEvidenceSpan[];
  generated_by: "local" | "model_assisted" | "fallback";
  validation_status: "valid" | "thin" | "rejected" | "fallback";
};

type ActiveCurrentWorkUnresolved = {
  id: string;
  observed_at_ms: number;
  app_name?: string | null;
  bundle_id?: string | null;
  window_title?: string | null;
  artifact_id?: string | null;
  observation_id?: string | null;
  frame_id?: string | null;
  event_ids: string[];
  task_action_ids?: string[];
  event_backed: boolean;
  has_fresh_heavy_frame: boolean;
  has_human_readable_title: boolean;
  has_openable_target: boolean;
  evidence_quality: string;
  identity_confidence: number;
  activity_hint?: string | null;
  unresolved_reason: string;
  missing_evidence: string[];
  warnings: string[];
};

type ContinueWorkTruth = {
  schema: string;
  policy_version: string;
  resolution_status: "task_supported" | "activity_supported" | "recent_activity_only" | "unresolved" | string;
  activity_kind: string;
  activity_summary?: string | null;
  work_object?: string | null;
  where_summary?: string | null;
  app_name?: string | null;
  artifact_id?: string | null;
  observed_at_ms: number;
  confidence: number;
  evidence_ids: string[];
  source: string;
  broader_goal_known: boolean;
  primary_relation: "primary" | "support" | "detour" | "recent-only" | string;
  reason_codes: string[];
};

type ContinueDecisionResult = {
  decision_id: string;
  mode: string;
  cache_hit: boolean;
  source: string;
  request_trigger?: ContinueDecisionRequestTrigger | string | null;
  task_understanding_source?: string | null;
  wording_source?: string | null;
  target_selection_source?: string | null;
  task_truth_v2?: TaskTruthProductionDecision | null;
  model?: string | null;
  response_id?: string | null;
  task_resolution_status?: ContinueTaskResolutionStatus | string | null;
  task_resolution_reason_codes?: string[];
  supported_surface?: ContinueDecisionSupportedSurface | null;
  alternative_hypotheses?: ContinueAlternativeHypothesis[];
  current_focus?: ContinueFocusSummary | null;
  active_current_work_unresolved?: ActiveCurrentWorkUnresolved | null;
  work_truth?: ContinueWorkTruth | null;
  current_activity?: string | null;
  current_task_turn?: ContinueCurrentTaskTurnSummary | null;
  selected_workstream?: ContinueSelectedWorkstream | null;
  selected_candidate_id?: string | null;
  target_truth?: ContinueTargetTruth;
  evidence_preview?: ContinueEvidencePreview | null;
  answer?: ContinueInterruptionRecoveryAnswer;
  direct_target_policy?: {
    direct_target_allowed: boolean;
    validated_direct_locator_present: boolean;
    evidence_preview_available: boolean;
    reason_codes: string[];
  };
  confidence_summary?: {
    task: ClaimConfidenceSummary;
    state: ClaimConfidenceSummary;
    recap: ClaimConfidenceSummary;
    target: ClaimConfidenceSummary;
  };
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
  handoff: ContinueHandoff;
  support_evidence?: ContinueSupportEvidenceItem[];
  alternatives: ContinueCandidateSummary[];
  generated_candidates: number;
  validation_status: string;
  feedback_policy_version?: string;
  feedback_watermark_ms?: number | null;
  open_watermark_ms?: number | null;
  feedback_suppressed_candidate_count?: number;
  feedback_score_capped_candidate_count?: number;
  eligible_candidate_count_after_feedback_gate?: number;
  model_candidate_count_before_feedback_filter?: number;
  model_candidate_count_after_feedback_filter?: number;
  selectable_candidate_count_before_branch_filter?: number;
  selectable_candidate_count_after_branch_filter?: number;
  excluded_branch_candidate_ids?: string[];
  support_evidence_count?: number;
  branch_validation_failures?: string[];
  continue_output_mode?: "strong_continue" | "thin_continue" | "no_clear_continuation";
  evidence_watermark_hash?: string;
  latest_boundary_revision?: number | null;
  current_surface_resolution?: {
    selected?: {
      evidence_ids?: string[];
      evidence_kinds?: string[];
      reason?: string;
      weak_surface_classification?: {
        adapter_key?: string | null;
      } | null;
    } | null;
  } | null;
  evidence_freshness_ledger?: ContinueEvidenceFreshnessSummary | null;
  weak_surface_enrichment?: WeakSurfaceEnrichmentDiagnostics | null;
  observe_before_decide?: unknown | null;
  app_activity?: unknown | null;
  activity_summary?: ContinueActivitySummary | null;
  activity_recap?: ContinueActivityRecap | null;
  activity_recap_watermark_hash?: string;
};

type TaskTruthPublicAnswer = ContinueTaskTruthAnswer;

type TaskTruthInferenceDiagnostic = {
  schema: string;
  status: string;
  origin: "live_cloud" | "cache" | "fixture" | "none" | string;
  provider: string;
  model: string;
  request_id?: string | null;
  provider_request_id?: string | null;
  response_id?: string | null;
  provider_attempt_count: number;
  latency_ms: number;
  image_count: number;
  image_bytes: number;
  estimated_tokens: number;
  input_tokens?: number | null;
  output_tokens?: number | null;
  total_tokens?: number | null;
  estimated_cost_usd?: number | null;
  verification_status: string;
  selected_hypothesis_id?: string | null;
};

type TaskTruthProductionDecision = {
  requested_state: "off" | "shadow" | "eligible" | "authoritative" | "rollback";
  effective_state: "off" | "shadow" | "eligible" | "authoritative" | "rollback";
  release_gate_passed: boolean;
  reason_codes: string[];
  cache_fingerprint: string;
  answer?: TaskTruthPublicAnswer | null;
  inference_diagnostic?: TaskTruthInferenceDiagnostic | null;
};

type ContinueSupportEvidenceItem = {
  artifact_id?: string | null;
  artifact_kind?: string | null;
  title?: string | null;
  branch_kind: string;
  origin_artifact_id?: string | null;
  role: string;
  public_return_eligible: boolean;
  reason: string;
  evidence_anchor_ids?: string[];
};

type ContinueHandoff = {
  headline: string;
  return_line: string;
  current_focus_line: string;
  last_state_line: string;
  next_action: string;
  why_this: string[];
  missing_evidence_line?: string | null;
  confidence_label: string;
  user_visible_uncertainty?: string | null;
};

type ContinueCardActionState = ContinuePresentationActionState;

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
  support_branch_false_positive_rate?: number;
  unpromoted_branch_selected_count?: number;
  branch_origin_recall_rate?: number;
  promoted_branch_selection_precision?: number;
  message_interrupt_false_positive_rate?: number;
  diagnostic_self_selected_count?: number;
  model_branch_policy_violation_count?: number;
  cases: Array<{
    name: string;
    scenario: string;
    selected_candidate_id?: string | null;
    selected_target_artifact_id?: string | null;
    target_artifact_correct: boolean;
    validation_status: string;
    validation_failures: string[];
    support_branch_false_positive?: boolean;
    unpromoted_branch_selected?: boolean;
    branch_origin_recalled?: boolean;
    promoted_branch_selection_correct?: boolean;
    message_interrupt_false_positive?: boolean;
    diagnostic_self_selected?: boolean;
    model_branch_policy_violation?: boolean;
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
  app_family?: string | null;
  surface_type?: string | null;
  activity_intent?: string | null;
  task_phase?: string | null;
  continuation_role?: string | null;
  work_value_reason?: string | null;
  why_not_primary?: string | null;
  feedback_suppression_state?: string;
  feedback_reason_codes?: string[];
  risk_flags?: string[];
  score_caps_applied?: string[];
  eligible_for_primary_selection?: boolean;
  public_alternative_eligible_after_feedback?: boolean;
  branch_promotion_state?: string | null;
  branch_public_return_eligible?: boolean | null;
  branch_promotion_reason?: string | null;
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

type ContinueHistoryCursor = {
  created_at_ms: number;
  decision_id: string;
};

type ContinueHistorySummary = {
  decision_id: string;
  created_at_ms: number;
  origin: "island" | "main_app";
  title: string;
};

type ContinueHistoryPage = {
  schema: string;
  items: ContinueHistorySummary[];
  next_cursor?: ContinueHistoryCursor | null;
};

type ContinueHistoryAnswerRow = {
  label: string;
  value: string;
};

type ContinueHistoryOutput = {
  schema: string;
  decision_id: string;
  created_at_ms: number;
  origin: "island" | "main_app";
  title: string;
  rows: ContinueHistoryAnswerRow[];
};

type ViewMode = "continue" | "history" | "settings" | "developer";
type OverlayMode = "units" | "ocr" | "ax" | "privacy";
type EvidenceTab = "text" | "events" | "context" | "paths";
type MemoryProductStatus =
  | "off"
  | "starting"
  | "on"
  | "paused_with_evidence"
  | "private_or_excluded"
  | "needs_permission"
  | "needs_attention"
  | "deleting";
type DangerousAction = "delete_all" | "delete_recent" | "dev_reset";
type ContinueRequestTrigger = ContinueDecisionRequestTrigger;

type ContinueAdoptionDiagnostic = {
  recordedAtMs: number;
  incumbentDecisionId: string;
  challengerDecisionId: string;
  challengerTrigger: ContinueRequestTrigger;
  reasonCodes: string[];
};

type ContinueEvidenceSnapshot = {
  frameCount: number;
  eventCount: number;
  signalCount: number;
  contentUnitCount: number;
  artifactCount: number;
  workstreamCount: number;
  latestFrameAtMs?: number | null;
  latestArtifactAtMs?: number | null;
  latestWorkstreamAtMs?: number | null;
};

const RECENT_MEMORY_DELETE_RANGE_MS = 60 * 60 * 1000;
const BACKGROUND_CONTINUE_VISIBLE_DEBOUNCE_MS = 5000;
const BACKGROUND_CONTINUE_IDLE_DEBOUNCE_MS = 30000;
const BACKGROUND_CONTINUE_MIN_INTERVAL_MS = 60000;
// Capture and session changes arrive through Tauri events. This slow heartbeat
// is recovery insurance for a missed event, not the main work scheduler.
const STATUS_HEARTBEAT_RUNNING_MS = 60000;
const STATUS_HEARTBEAT_IDLE_MS = 120000;

const memoryProductCopy: Record<MemoryProductStatus, { label: string; detail: string }> = {
  off: {
    label: "Memory off",
    detail: "Turn it on once, keep working, then ask Continue when needed.",
  },
  starting: {
    label: "Starting memory",
    detail: "Preparing local memory.",
  },
  on: {
    label: "Memory on",
    detail: "Smalltalk is maintaining context quietly in the background.",
  },
  paused_with_evidence: {
    label: "Memory paused",
    detail: "Continue can still answer from evidence already stored locally.",
  },
  private_or_excluded: {
    label: "Excluded by your privacy settings",
    detail: "This app or website matches an exclusion you control.",
  },
  needs_permission: {
    label: "Permission needed",
    detail: "Smalltalk needs permission before local memory can work.",
  },
  needs_attention: {
    label: "Memory needs attention",
    detail: "Open Details for the technical cause.",
  },
  deleting: {
    label: "Deleting local memory",
    detail: "Clearing local evidence.",
  },
};

const emptyRuntimeDiagnostics: RuntimeDiagnostics = {
  capture_runtime_state: "stopped",
  worker_generation: 0,
  current_operation_class: null,
  current_operation_started_at_ms: null,
  last_operation_class: null,
  last_operation_duration_ms: null,
  helper_launches: 0,
  helper_successes: 0,
  helper_timeouts: 0,
  helper_timeouts_reaped: 0,
  helper_cancellations: 0,
  helper_abnormal_exits: 0,
  helper_output_limit_failures: 0,
  helper_launch_failures: 0,
  active_child_processes: 0,
  last_safe_error_category: null,
  stop_latency_ms: null,
  worker_panic_count: 0,
  provider_health: {},
  provider_by_operation: {},
  fallback_counts_by_operation: {},
  provider_circuit_breaker_opens: 0,
  provider_recovery_probes: 0,
  event_pipeline: {
    queue_depth: 0,
    queue_capacity: 320,
    high_queue_depth: 0,
    normal_queue_depth: 0,
    pressure_queue_depth: 0,
    high_water_mark: 0,
    coalesced_count: 0,
    dropped_count: 0,
    dropped_pressure_count: 0,
    dropped_normal_count: 0,
    dropped_high_count: 0,
    shutdown: false,
  },
  schema_initialization_count: 0,
  migration_execution_count: 0,
  database_busy_retry_count: 0,
  database_busy_time_ms: 0,
  database_generation: 0,
  audit_executor: {
    active: false,
    queued: 0,
    queue_capacity: 1,
    submitted: 0,
    completed: 0,
    failed: 0,
    coalesced: 0,
    superseded: 0,
    shutting_down: false,
  },
  status_metrics: {
    sample_count: 0,
    p50_latency_us: 0,
    p95_latency_us: 0,
    last_response_bytes: 0,
    max_response_bytes: 0,
  },
  workload: {
    active_operations: [],
    queued_operation_count: 0,
    queue_capacity: 48,
    queue_high_water_mark: 0,
    queued_by_class: {},
    queue_capacity_by_class: {},
    rejected_by_class: {},
    coalesced_requests: 0,
    cancelled_or_superseded_requests: 0,
    background_decisions_avoided: 0,
    shutting_down: false,
    duration_percentiles_ms: {},
  },
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
  continue_output_audit_failures: 0,
  weak_surface_enrichment_attempts: 0,
  weak_surface_enrichment_success_strong: 0,
  weak_surface_enrichment_success_medium: 0,
  weak_surface_enrichment_success_thin: 0,
  weak_surface_enrichment_skipped_privacy: 0,
  weak_surface_enrichment_skipped_budget: 0,
  weak_surface_enrichment_failed: 0,
  latest_weak_surface_attempt: null,
  latest_weak_surface_snapshot_id: null,
  sck_display_successes: 0,
  sck_active_window_successes: 0,
  sck_active_window_abnormal_exits: 0,
  sck_timeouts: 0,
  sck_circuit_breaker_opens: 0,
  screencapture_fallbacks: 0,
  latest_sck_capture_mode: null,
  latest_sck_provider: null,
  latest_sck_duration_ms: null,
  latest_sck_exit_category: null,
  latest_sck_fallback_used: null,
  sck_active_window_circuit_breaker_state: "closed",
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
  const {
    profile,
    user,
    loading: authLoading,
    error: authError,
    signOut,
  } = useAuth();
  const [status, setStatus] = useState<CaptureStatus>(initialStatus);
  const [screenCapturePermission, setScreenCapturePermission] =
    useState<ScreenCapturePermissionStatus | null>(null);
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
  const [islandDecisionHydrationComplete, setIslandDecisionHydrationComplete] = useState(false);
  const [continueDecisionFrameCount, setContinueDecisionFrameCount] = useState<number | null>(null);
  const [continueDecisionEvidenceSnapshot, setContinueDecisionEvidenceSnapshot] =
    useState<ContinueEvidenceSnapshot | null>(null);
  const [continueError, setContinueError] = useState<string | null>(null);
  const [, setBackgroundContinueError] = useState<string | null>(null);
  const [continueAdoptionDiagnostics, setContinueAdoptionDiagnostics] = useState<
    ContinueAdoptionDiagnostic[]
  >([]);
  const [continueOpenResult, setContinueOpenResult] = useState<OpenResumePointResult | null>(null);
  const [quietContinueRefreshing, setQuietContinueRefreshing] = useState(false);
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
  const [viewMode, setViewMode] = useState<ViewMode>("continue");
  const [sidebarOpen, setSidebarOpen] = useState(false);
  const [historyPage, setHistoryPage] = useState<ContinueHistoryPage | null>(null);
  const [historyOutput, setHistoryOutput] = useState<ContinueHistoryOutput | null>(null);
  const [historyBusy, setHistoryBusy] = useState(false);
  const [historyError, setHistoryError] = useState<string | null>(null);
  const [memoryMenuOpen, setMemoryMenuOpen] = useState(false);
  const [privacyPanelOpen, setPrivacyPanelOpen] = useState(false);
  const [exclusionRules, setExclusionRules] = useState<ExclusionRule[]>([]);
  const [privacyActionStatus, setPrivacyActionStatus] = useState<string | null>(null);
  const [pendingDangerAction, setPendingDangerAction] = useState<DangerousAction | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [appVisible, setAppVisible] = useState(() => document.visibilityState === "visible");
  const storeGenerationRef = useRef(0);
  const autoContinueRef = useRef(false);
  const continueRequestInFlightRef = useRef<ContinueRequestTrigger | null>(null);
  const continueDecisionRef = useRef<ContinueDecisionResult | null>(null);
  const continueDecisionTriggerRef = useRef<ContinueRequestTrigger | null>(null);
  const statusRef = useRef(status);
  const continueMemoryRef = useRef(continueMemory);
  const selectedFrameRef = useRef(selectedFrame);
  const lastBackgroundContinueAttemptRef = useRef(0);
  const failedBackgroundContinueSignatureRef = useRef<string | null>(null);
  const captureMenuRef = useRef<HTMLDetailsElement | null>(null);
  const appScrollRef = useRef<HTMLDivElement | null>(null);
  const isDeleting = busyAction === "delete_all_frames" || busyAction === "delete_recent_captures";
  const diagnosticsOpen = viewMode === "developer";
  const diagnosticsOpenRef = useRef(diagnosticsOpen);
  const currentSession = status.active_session || status.latest_session || null;
  const currentSessionId = currentSession?.id || null;

  // Tauri event listeners must stay registered across ordinary React state
  // updates. The refs let their stable handlers read current state without
  // making the subscription effect depend on frequently changing objects.
  statusRef.current = status;
  continueMemoryRef.current = continueMemory;
  selectedFrameRef.current = selectedFrame;
  diagnosticsOpenRef.current = diagnosticsOpen;

  const refreshStatus = useCallback(async (): Promise<CaptureStatus | null> => {
    const requestGeneration = storeGenerationRef.current;
    try {
      const nextStatus = await invoke<CaptureStatus>("capture_status");
      if (requestGeneration !== storeGenerationRef.current) return null;
      statusRef.current = nextStatus;
      setStatus(nextStatus);
      setError(null);
      if (!selectedFrameRef.current && nextStatus.latest_frame) {
        selectedFrameRef.current = nextStatus.latest_frame;
        setSelectedFrame(nextStatus.latest_frame);
      }
      return nextStatus;
    } catch (err) {
      setError(String(err));
      return null;
    }
  }, []);

  const refreshScreenCapturePermission = useCallback(async () => {
    try {
      const permission = await invoke<ScreenCapturePermissionStatus>(
        "get_screen_capture_permission_status",
      );
      setScreenCapturePermission(permission);
      return permission;
    } catch (err) {
      setError(`Screen access status failed: ${String(err)}`);
      return null;
    }
  }, []);

  const refreshContinueMemory = useCallback(async (): Promise<ContinueMemoryStatus | null> => {
    try {
      const nextMemory = await invoke<ContinueMemoryStatus>("get_continue_memory_status");
      continueMemoryRef.current = nextMemory;
      setContinueMemory(nextMemory);
      return nextMemory;
    } catch (err) {
      setContinueError(`Continue memory status failed: ${String(err)}`);
      return null;
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

  const refreshExclusionRules = useCallback(async () => {
    try {
      const rules = await invoke<ExclusionRule[]>("list_exclusion_rules");
      setExclusionRules(rules);
    } catch (err) {
      setPrivacyActionStatus(`Could not load privacy exclusions: ${String(err)}`);
    }
  }, []);

  const loadContinueHistory = useCallback(async (
    cursor: ContinueHistoryCursor | null = null,
    append = false,
  ) => {
    setHistoryBusy(true);
    setHistoryError(null);
    try {
      const page = await invoke<ContinueHistoryPage>("list_continue_history", {
        cursor,
        limit: 25,
      });
      setHistoryPage((current) => append && current
        ? {
            ...page,
            items: [...current.items, ...page.items],
          }
        : page);
    } catch (err) {
      if (!append) {
        setHistoryPage({
          schema: "smalltalk.continue_history_page.v1",
          items: [],
          next_cursor: null,
        });
      }
      setHistoryError(`Previous continuations are unavailable: ${String(err)}`);
    } finally {
      setHistoryBusy(false);
    }
  }, []);

  const loadContinueHistoryOutput = useCallback(async (decisionId: string) => {
    setHistoryBusy(true);
    setHistoryError(null);
    try {
      const output = await invoke<ContinueHistoryOutput | null>(
        "get_continue_history_output",
        { decisionId },
      );
      setHistoryOutput(output);
      if (!output) {
        setHistoryError("That saved continuation is no longer available.");
      }
    } catch (err) {
      setHistoryError(`That continuation could not be opened: ${String(err)}`);
    } finally {
      setHistoryBusy(false);
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

  const applyContinueDecision = useCallback(
    async (
      decision: ContinueDecisionResult,
      receivedTrigger: ContinueRequestTrigger,
    ): Promise<boolean> => {
      // The receiving path is authoritative for adoption priority. A cache hit may
      // preserve the request trigger that originally produced the decision.
      const challengerTrigger = receivedTrigger;
      const incumbent = continueDecisionRef.current;
      const comparison = compareContinueDecisionAdoption({
        incumbent,
        challenger: decision,
        incumbentTrigger: continueDecisionTriggerRef.current,
        challengerTrigger,
      });
      if (!comparison.adopt && incumbent) {
        setContinueAdoptionDiagnostics((current) => [
          {
            recordedAtMs: Date.now(),
            incumbentDecisionId: incumbent.decision_id,
            challengerDecisionId: decision.decision_id,
            challengerTrigger,
            reasonCodes: comparison.reasonCodes,
          },
          ...current,
        ].slice(0, 8));
        if (challengerTrigger === "background") {
          failedBackgroundContinueSignatureRef.current = continueEvidenceSignature(
            buildContinueEvidenceSnapshot(statusRef.current, continueMemoryRef.current),
          );
          setBackgroundContinueError(
            "A quiet refresh returned a weaker answer. Keeping the stronger Continue answer.",
          );
        } else if (
          comparison.reasonCodes.includes(
            "rejected:explicit_refresh_failed_without_semantics",
          )
        ) {
          setContinueError(
            "Couldn’t refresh Continue. Keeping the previous answer instead.",
          );
        }
        return false;
      }

      continueDecisionRef.current = decision;
      continueDecisionTriggerRef.current = challengerTrigger;
      setContinueDecision(decision);
      setSelectedWorkstreamId(decision.selected_workstream?.workstream_id || null);
      setBackgroundContinueError(null);
      failedBackgroundContinueSignatureRef.current = null;

      const [nextStatus, nextMemory] = await Promise.all([
        refreshStatus(),
        refreshContinueMemory(),
      ]);
      const evidenceStatus = nextStatus || statusRef.current;
      const evidenceMemory = nextMemory || continueMemoryRef.current;
      setContinueDecisionFrameCount(evidenceStatus.frame_count);
      setContinueDecisionEvidenceSnapshot(
        buildContinueEvidenceSnapshot(evidenceStatus, evidenceMemory),
      );
      return true;
    },
    [refreshContinueMemory, refreshStatus],
  );

  const runContinueDecision = useCallback(async (options: {
    forceRebuild?: boolean;
    writeAudit?: boolean;
    auditMode?: "none" | "mfti_review" | "full";
    trigger?: ContinueRequestTrigger;
  } = {}) => {
    const trigger = options.trigger || (options.writeAudit === true ? "manual" : "startup");
    const auditMode = options.auditMode || (trigger === "manual" ? "mfti_review" : "none");
    const background = trigger === "background";
    const activeTrigger = continueRequestInFlightRef.current;
    if (activeTrigger && !(trigger === "manual" && activeTrigger === "background")) return;
    continueRequestInFlightRef.current = trigger;
    if (background) {
      setQuietContinueRefreshing(true);
      setBackgroundContinueError(null);
      lastBackgroundContinueAttemptRef.current = Date.now();
    } else {
      setBusyAction("get_continue_decision");
      setContinueError(null);
      setContinueOpenResult(null);
      setBreadcrumbStatus(null);
    }
    try {
      const decision = await withContinueRequestTimeout(
        invoke<ContinueDecisionResult>("get_continue_decision", {
          input: {
            mode: options.forceRebuild === true ? "rebuild" : "normal",
            session_id: status.active_session?.id || status.latest_session?.id || null,
            rebuild_layers: options.forceRebuild === true,
            micro_inference_enabled: false,
            activity_recap_model_enabled: false,
            max_candidates_for_model: 5,
            audit_output_enabled: auditMode === "full",
            audit_mode: auditMode,
            request_trigger: trigger,
          },
        }),
      );
      const adopted = await applyContinueDecision(decision, trigger);
      if (!adopted) return;
      await invoke("get_island_continue_state", {
        input: {
          reason: "main_card_decision_updated",
          existing_decision_id: decision.decision_id,
          allow_refresh: false,
          force_refresh: false,
          source: "desktop_continue_card",
        },
      }).catch(() => null);
      if (diagnosticsOpen) {
        await refreshWorkstreams();
      }
      const firstEvidenceFrame = continueEvidencePreviewFrameId(decision);
      if (diagnosticsOpen && firstEvidenceFrame && !selectedFrame) {
        await revealContinueFrame(firstEvidenceFrame);
        setEvidenceOpen(false);
      }
    } catch (err) {
      if (background) {
        failedBackgroundContinueSignatureRef.current = continueEvidenceSignature(
          buildContinueEvidenceSnapshot(status, continueMemory),
        );
        setBackgroundContinueError("Could not refresh Continue quietly. Keeping the previous answer.");
      } else {
        setContinueError(continueRequestErrorCopy(err));
      }
    } finally {
      if (continueRequestInFlightRef.current === trigger) {
        continueRequestInFlightRef.current = null;
      }
      if (background) {
        setQuietContinueRefreshing(false);
      } else {
        setBusyAction(null);
      }
    }
  }, [
    applyContinueDecision,
    continueMemory,
    diagnosticsOpen,
    refreshWorkstreams,
    revealContinueFrame,
    selectedFrame,
    status,
  ]);

  const openContinueTarget = useCallback(async () => {
    if (!continueDecision) return;
    if (getContinueCardActionState(continueDecision).kind !== "openable_return_target") {
      setContinueOpenResult(null);
      setContinueError("This surface is supporting evidence, not a safe continuation target.");
      return;
    }
    const taskTruthAnswer = authoritativeTaskTruthAnswer(continueDecision);
    const resumeTarget = taskTruthAnswer
      ? authoritativeTaskTruthTarget(continueDecision)
      : continueDecision.resume_work_target || continueDecision.return_target || null;
    setBusyAction("open_continue_target");
    setContinueOpenResult(null);
    setContinueError(null);
    try {
      const result = await invoke<OpenResumePointResult>("open_resume_point", {
        input: {
          continue_decision_id: continueDecision.decision_id,
          target_artifact_id: resumeTarget?.artifact_id || null,
          source: "desktop_continue_card",
          strict_continue_target: true,
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
        taskSnapshotId?: string | null;
        taskSnapshotRevision?: number | null;
        affectedTaskField?: string | null;
        taskHypothesisId?: string | null;
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
        await invoke<ContinueFeedbackEventResult>(
          "record_continue_feedback",
          {
            input: {
              decision_id: continueDecision?.decision_id || workstreamDetail?.latest_decision?.decision_id || null,
              selected_candidate_id:
                options.selectedCandidateId ||
                continueDecision?.selected_candidate_id ||
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
              task_snapshot_id: options.taskSnapshotId || null,
              task_snapshot_revision: options.taskSnapshotRevision || null,
              affected_task_field: options.affectedTaskField || null,
              task_hypothesis_id: options.taskHypothesisId || null,
            },
          },
        );
        setFeedbackStatus("Got it. Smalltalk will use that correction next time.");
        await loadWorkstreamDetail(workstreamId);
        if (
          feedbackKind === "rejected" ||
          feedbackKind === "ignored" ||
          feedbackKind === "corrected" ||
          feedbackKind === "supporting_work" ||
          feedbackKind === "unrelated_activity" ||
          feedbackKind === "completed" ||
          feedbackKind === "reactivated" ||
          feedbackKind === "artifact_only_evidence" ||
          feedbackKind === "ignored_workstream"
        ) {
          continueDecisionRef.current = null;
          continueDecisionTriggerRef.current = null;
          setContinueDecision(null);
          await runContinueDecision({ forceRebuild: true, trigger: "manual" });
        }
      } catch (err) {
        setContinueError(`Feedback failed: ${String(err)}`);
      } finally {
        setBusyAction(null);
      }
    },
    [
      continueDecision,
      loadWorkstreamDetail,
      runContinueDecision,
      selectedWorkstreamId,
      workstreamDetail,
    ],
  );

  const continueFromAlternative = useCallback(
    async (candidate: ContinueWorkstreamCandidateDetail | ContinueCandidateSummary) => {
      const originalTarget =
        continueDecision?.resume_work_target?.artifact_id ||
        continueDecision?.return_target?.artifact_id ||
        workstreamDetail?.latest_decision?.return_target_artifact_id ||
        null;
      await recordContinueFeedback("corrected", {
        selectedCandidateId:
          continueDecision?.selected_candidate_id ||
          workstreamDetail?.latest_decision?.selected_candidate_id ||
          null,
        workstreamId:
          continueDecision?.selected_workstream?.workstream_id ||
          workstreamDetail?.workstream.id ||
          selectedWorkstreamId ||
          candidate.workstream_id ||
          null,
        targetArtifactId: originalTarget,
        correctedArtifactId:
          "target_artifact_id" in candidate
            ? candidate.target_artifact_id
            : null,
        note: candidate.reason || "Selected alternative continuation target.",
      });
      setSelectedWorkstreamId(candidate.workstream_id);
      if (!diagnosticsOpen) {
        setContinueOpenResult(null);
        setFeedbackStatus("Alternative noted. Refreshing Continue with that correction.");
        await runContinueDecision({ forceRebuild: true, trigger: "manual" });
        return;
      }
      const frameId = Number(candidate.evidence_frame_id);
      if (Number.isFinite(frameId)) {
        try {
          const result = await invoke<OpenResumePointResult>("open_resume_point", {
            input: {
              target_frame_id: frameId,
              source: "diagnostics",
              diagnostic_allowed: true,
            },
          });
          setContinueOpenResult(result);
        } catch (err) {
          setContinueError(`Open alternative failed: ${String(err)}`);
        }
      }
    },
    [
      continueDecision,
      diagnosticsOpen,
      recordContinueFeedback,
      runContinueDecision,
      selectedWorkstreamId,
      workstreamDetail,
    ],
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

  const performDevReset = useCallback(async () => {
    setBusyAction("dev_reset_local_memory");
    setError(null);
    setPrivacyActionStatus(null);
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
      continueDecisionRef.current = null;
      continueDecisionTriggerRef.current = null;
      setContinueDecision(null);
      setContinueDecisionFrameCount(null);
      setContinueDecisionEvidenceSnapshot(null);
      setBackgroundContinueError(null);
      setContinueAdoptionDiagnostics([]);
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
      await refreshExclusionRules();
      setPrivacyActionStatus("Developer reset completed.");
    } catch (err) {
      setError(`Developer reset failed: ${String(err)}`);
    } finally {
      setBusyAction(null);
    }
  }, [refreshContinueMemory, refreshExclusionRules, refreshMemoryDiagnostics]);

  const requestDevReset = useCallback(() => {
    setPendingDangerAction("dev_reset");
  }, []);

  const runAction = useCallback(
    async (action: "start_capture" | "stop_capture" | "capture_once") => {
      setBusyAction(action);
      setError(null);
      try {
        if (action === "start_capture") {
          let permission = await refreshScreenCapturePermission();
          if (!permission) {
            return;
          }
          if (!permission.granted && permission.can_request) {
            permission = await invoke<ScreenCapturePermissionStatus>(
              "request_screen_capture_permission",
            );
            setScreenCapturePermission(permission);
          }
          if (!permission.granted) {
            setError(permission.message);
            return;
          }
        }
        if (action === "stop_capture") {
          const response = await invoke<StopCaptureOutput>(action);
          const stoppedSessionId =
            response.session?.id ||
            response.status.latest_session?.id ||
            currentSessionId;
          setStatus(response.status);
          await refreshStatus();
          if (diagnosticsOpen) {
            await runSearch(query, stoppedSessionId);
            await refreshTimeline(stoppedSessionId);
          }
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
          if (diagnosticsOpen) {
            await runSearch(query, nextSessionId);
            await refreshTimeline(nextSessionId);
          }
          return;
        }
        await refreshStatus();
        if (diagnosticsOpen) {
          await runSearch(query, currentSessionId);
          await refreshTimeline(currentSessionId);
        }
      } catch (err) {
        setError(String(err));
        if (action === "start_capture") {
          await refreshScreenCapturePermission();
        }
      } finally {
        setBusyAction(null);
      }
    },
    [
      currentSessionId,
      diagnosticsOpen,
      query,
      refreshScreenCapturePermission,
      refreshStatus,
      refreshTimeline,
      runSearch,
      selectFrame,
    ],
  );

  const requestScreenCapturePermission = useCallback(async () => {
    setBusyAction("request_screen_capture_permission");
    setError(null);
    try {
      const permission = await invoke<ScreenCapturePermissionStatus>(
        "request_screen_capture_permission",
      );
      setScreenCapturePermission(permission);
      setPrivacyActionStatus(permission.message);
    } catch (err) {
      setError(`Screen access request failed: ${String(err)}`);
      await refreshScreenCapturePermission();
    } finally {
      setBusyAction(null);
    }
  }, [refreshScreenCapturePermission]);

  const performDeleteAllMemory = useCallback(async () => {
    setBusyAction("delete_all_frames");
    setError(null);
    setPrivacyActionStatus(null);
    storeGenerationRef.current += 1;
    try {
      const nextStatus = await invoke<CaptureStatus>("delete_all_frames");
      storeGenerationRef.current += 1;
      setResults([]);
      setSelectedFrame(null);
      setImageData(null);
      setFrameDetail(null);
      setTimeline(emptyTimeline);
      continueDecisionRef.current = null;
      continueDecisionTriggerRef.current = null;
      setContinueDecision(null);
      setContinueDecisionFrameCount(null);
      setContinueDecisionEvidenceSnapshot(null);
      setBackgroundContinueError(null);
      setContinueAdoptionDiagnostics([]);
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
      await refreshContinueMemory();
      await refreshMemoryDiagnostics();
      await refreshExclusionRules();
      setPrivacyActionStatus("Local memory deleted.");
    } catch (err) {
      setError(`Delete all failed: ${String(err)}`);
    } finally {
      setBusyAction(null);
    }
  }, [refreshContinueMemory, refreshExclusionRules, refreshMemoryDiagnostics]);

  const deleteAllFrames = useCallback(() => {
    setPendingDangerAction("delete_all");
  }, []);

  const deleteRecentMemory = useCallback(() => {
    setPendingDangerAction("delete_recent");
  }, []);

  const performDeleteRecentMemory = useCallback(async () => {
    setBusyAction("delete_recent_captures");
    setError(null);
    setPrivacyActionStatus(null);
    storeGenerationRef.current += 1;
    try {
      const deletedCount = await invoke<number>("delete_recent_captures", {
        rangeMs: RECENT_MEMORY_DELETE_RANGE_MS,
      });
      storeGenerationRef.current += 1;
      setResults([]);
      setSelectedFrame(null);
      setImageData(null);
      setFrameDetail(null);
      setTimeline(emptyTimeline);
      continueDecisionRef.current = null;
      continueDecisionTriggerRef.current = null;
      setContinueDecision(null);
      setContinueDecisionFrameCount(null);
      setContinueDecisionEvidenceSnapshot(null);
      setBackgroundContinueError(null);
      setContinueAdoptionDiagnostics([]);
      setWorkstreams([]);
      setSelectedWorkstreamId(null);
      setWorkstreamDetail(null);
      setFeedbackStatus(null);
      setEvalReport(null);
      const nextStatus = await invoke<CaptureStatus>("capture_status");
      setStatus(nextStatus);
      await refreshContinueMemory();
      await refreshMemoryDiagnostics();
      setPrivacyActionStatus(
        deletedCount > 0
          ? `Deleted ${deletedCount} recent evidence ${deletedCount === 1 ? "item" : "items"}.`
          : "No recent local evidence needed deletion.",
      );
    } catch (err) {
      setError(`Delete recent memory failed: ${String(err)}`);
    } finally {
      setBusyAction(null);
    }
  }, [refreshContinueMemory, refreshMemoryDiagnostics]);

  const confirmDangerAction = useCallback(async () => {
    const action = pendingDangerAction;
    if (!action) return;
    setPendingDangerAction(null);
    if (action === "delete_all") {
      await performDeleteAllMemory();
    } else if (action === "delete_recent") {
      await performDeleteRecentMemory();
    } else {
      await performDevReset();
    }
  }, [pendingDangerAction, performDeleteAllMemory, performDeleteRecentMemory, performDevReset]);

  useEffect(() => {
    void refreshStatus();
    void refreshScreenCapturePermission();
    void refreshContinueMemory();
    void refreshExclusionRules();
  }, [
    refreshContinueMemory,
    refreshExclusionRules,
    refreshScreenCapturePermission,
    refreshStatus,
  ]);

  useEffect(() => {
    if (privacyPanelOpen) {
      void refreshExclusionRules();
    }
  }, [privacyPanelOpen, refreshExclusionRules]);

  useEffect(() => {
    if (viewMode === "history" && !historyPage && !historyBusy) {
      void loadContinueHistory();
    }
  }, [historyBusy, historyPage, loadContinueHistory, viewMode]);

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
      if (status.running && diagnosticsOpen) {
        void refreshContinueMemory();
        void runSearch();
        void refreshTimeline();
        void refreshWorkstreams();
      }
    }, status.running ? STATUS_HEARTBEAT_RUNNING_MS : STATUS_HEARTBEAT_IDLE_MS);

    return () => window.clearInterval(id);
  }, [diagnosticsOpen, isDeleting, refreshContinueMemory, refreshStatus, refreshTimeline, refreshWorkstreams, runSearch, status.running]);

  useEffect(() => {
    let disposed = false;

    const hydrateLatestIslandDecision = async () => {
      if (continueDecisionRef.current) {
        if (!disposed) setIslandDecisionHydrationComplete(true);
        return;
      }
      try {
        const decision = await invoke<ContinueDecisionResult | null>(
          "get_latest_island_continue_decision",
        );
        if (!disposed && decision) {
          await applyContinueDecision(decision, "island");
        }
      } catch {
        // The remembered decision is an optimization for a missed native
        // event. If none is available, the normal startup request remains the
        // source of truth.
      } finally {
        if (!disposed) setIslandDecisionHydrationComplete(true);
      }
    };

    void hydrateLatestIslandDecision();
    return () => {
      disposed = true;
    };
  }, [applyContinueDecision]);

  useEffect(() => {
    if (
      !islandDecisionHydrationComplete ||
      autoContinueRef.current ||
      busyAction !== null ||
      continueDecision ||
      status.frame_count === 0
    ) {
      return;
    }
    autoContinueRef.current = true;
    void runContinueDecision({ writeAudit: false, trigger: "startup" });
  }, [
    busyAction,
    continueDecision,
    islandDecisionHydrationComplete,
    runContinueDecision,
    status.frame_count,
  ]);

  useEffect(() => {
    const handleVisibilityChange = () => {
      setAppVisible(document.visibilityState === "visible");
    };

    document.addEventListener("visibilitychange", handleVisibilityChange);
    return () => {
      document.removeEventListener("visibilitychange", handleVisibilityChange);
    };
  }, []);

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
    if (viewMode !== "continue" || !continueDecision) {
      setEvidenceOpen(false);
      return;
    }

    const previewFrameId = continueEvidencePreviewFrameId(continueDecision);
    if (!previewFrameId) {
      setEvidenceOpen(false);
      return;
    }

    void revealContinueFrame(previewFrameId);
  }, [continueDecision?.decision_id, revealContinueFrame, viewMode]);

  useEffect(() => {
    let cancelled = false;
    async function loadImage() {
      if (!selectedFrame || (!diagnosticsOpen && !evidenceOpen)) {
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
  }, [diagnosticsOpen, evidenceOpen, selectedFrame?.id]);

  useEffect(() => {
    let cancelled = false;
    async function loadDetail() {
      if (!selectedFrame || !diagnosticsOpen) {
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
  }, [diagnosticsOpen, selectedFrame?.id]);

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
  const continueHasEvidence =
    status.frame_count > 0 ||
    Boolean(continueMemory && continueMemory.counts.artifacts > 0);
  const currentContinueEvidenceSnapshot = useMemo(
    () => buildContinueEvidenceSnapshot(status, continueMemory),
    [
      continueMemory,
      status.content_unit_count,
      status.event_count,
      status.frame_count,
      status.latest_frame?.captured_at,
      status.signal_count,
    ],
  );
  const continueIsStale =
    Boolean(continueDecision) &&
    (
      (continueDecisionEvidenceSnapshot
        ? continueEvidenceChanged(continueDecisionEvidenceSnapshot, currentContinueEvidenceSnapshot)
        : continueDecisionFrameCount !== null && status.frame_count > continueDecisionFrameCount)
    );
  const continueRefreshBusy = busyAction === "get_continue_decision" || quietContinueRefreshing;
  const currentEvidenceSignature = continueEvidenceSignature(currentContinueEvidenceSnapshot);

  useEffect(() => {
    if (
      !appVisible ||
      !status.running ||
      !continueHasEvidence ||
      !continueDecision ||
      !continueIsStale ||
      busyAction !== null ||
      quietContinueRefreshing ||
      failedBackgroundContinueSignatureRef.current === currentEvidenceSignature
    ) {
      return;
    }

    const requestedDelay = viewMode === "continue"
      ? BACKGROUND_CONTINUE_VISIBLE_DEBOUNCE_MS
      : BACKGROUND_CONTINUE_IDLE_DEBOUNCE_MS;
    const sinceLastAttempt = Date.now() - lastBackgroundContinueAttemptRef.current;
    const delay = Math.max(
      requestedDelay,
      BACKGROUND_CONTINUE_MIN_INTERVAL_MS - sinceLastAttempt,
      0,
    );
    const id = window.setTimeout(() => {
      void runContinueDecision({
        writeAudit: false,
        trigger: "background",
      });
    }, delay);

    return () => window.clearTimeout(id);
  }, [
    appVisible,
    busyAction,
    continueDecision,
    continueHasEvidence,
    continueIsStale,
    currentEvidenceSignature,
    quietContinueRefreshing,
    runContinueDecision,
    status.running,
    viewMode,
  ]);

  const latestEvidenceFrame = status.latest_frame;
  const memorySurfacePrivate = isPrivateMemorySurface(status);
  const persistentCaptureError = status.last_error
    && !isTransientScreenshotCaptureContention(status.last_error)
    ? status.last_error
    : null;
  const visibleRuntimeError = [error, persistentCaptureError]
    .find((message) => message && !isTransientScreenshotCaptureContention(message))
    || null;
  const memoryProductStatus = deriveMemoryProductStatus(
    { ...status, last_error: persistentCaptureError },
    continueHasEvidence,
    busyAction,
    memorySurfacePrivate,
  );
  const memoryProduct = getMemoryProductCopy(memoryProductStatus, persistentCaptureError);
  const continuePrimaryMessage = status.running && !continueDecision && !continueHasEvidence
    ? "Local memory is on."
    : !continueHasEvidence
      ? "Turn on local memory once."
      : status.running && !continueDecision
        ? "Local memory is on."
        : continueDecision
          ? continueWorkstreamTitle
          : "Ready to find your continuation.";
  const captureStateLabel = isDeleting
    ? "Deleting"
    : status.running
      ? "Local memory active"
      : "Ready";
  const hasFrames = status.frame_count > 0;
  const hasQuery = query.trim().length > 0;
  const latestFrameLabel = latestEvidenceFrame
    ? formatTime(latestEvidenceFrame.captured_at)
    : "None yet";
  const latestEvidenceAgeLabel = latestEvidenceFrame
    ? formatRelativeAge(latestEvidenceFrame.captured_at)
    : "No evidence yet";
  const memoryWindowLabel = currentSession
    ? `${sentenceCase(currentSession.status)} memory-${String(currentSession.sequence).padStart(3, "0")}`
    : "No memory window";
  const currentAppPattern = latestEvidenceFrame?.app_bundle_id || latestEvidenceFrame?.app_name || "";
  const currentAppLabel = latestEvidenceFrame?.app_name || latestEvidenceFrame?.app_bundle_id || "";
  const currentWebsitePattern = sitePatternFromUrl(latestEvidenceFrame?.browser_url);
  const currentWebsiteLabel = siteLabelFromUrl(latestEvidenceFrame?.browser_url);
  const currentAppExcluded = currentAppPattern
    ? hasEnabledExclusion(exclusionRules, "app_bundle", currentAppPattern)
    : false;
  const currentWebsiteExcluded = currentWebsitePattern
    ? hasEnabledExclusion(exclusionRules, "url_regex", currentWebsitePattern)
    : false;
  const activeContext = frameDetail?.app_contexts[0];
  const activeTransition = frameDetail?.transitions[0];
  const selectedTitle = selectedFrame ? frameTitle(selectedFrame) : "No evidence selected";
  const showInspectEntry = import.meta.env.DEV;
  const googleGivenName =
    typeof user?.user_metadata?.given_name === "string"
      ? user.user_metadata.given_name.trim()
      : "";
  const profileFirstName = profile?.full_name?.trim().split(/\s+/)[0] || "";
  const googleFullName =
    typeof user?.user_metadata?.full_name === "string"
      ? user.user_metadata.full_name.trim()
      : typeof user?.user_metadata?.name === "string"
        ? user.user_metadata.name.trim()
        : "";
  const googleFirstName = googleFullName.split(/\s+/)[0] || "";
  const greetingName =
    googleGivenName || profileFirstName || googleFirstName || "there";
  const openProductView = useCallback((nextView: Exclude<ViewMode, "developer">) => {
    setMemoryMenuOpen(false);
    setEvidenceOpen(false);
    setHistoryOutput(null);
    appScrollRef.current?.scrollTo({ top: 0 });
    setViewMode(nextView);
    if (nextView === "history") {
      void loadContinueHistory();
    }
  }, [loadContinueHistory]);
  const openDeveloperMode = useCallback(() => {
    setMemoryMenuOpen(false);
    appScrollRef.current?.scrollTo({ top: 0 });
    setViewMode("developer");
    void refreshWorkstreams();
    void runSearch("");
    void refreshTimeline();
    void refreshMemoryDiagnostics();
    void loadWorkstreamDetail(selectedWorkstreamId);
  }, [
    loadWorkstreamDetail,
    refreshMemoryDiagnostics,
    refreshTimeline,
    refreshWorkstreams,
    runSearch,
    selectedWorkstreamId,
  ]);

  const openPrivacyPanel = useCallback(() => {
    setMemoryMenuOpen(false);
    setPrivacyPanelOpen(true);
    setPrivacyActionStatus(null);
    void refreshExclusionRules();
  }, [refreshExclusionRules]);

  const addPrivacyExclusion = useCallback(
    async (rule: ExclusionRuleInput, successMessage: string) => {
      setBusyAction("add_exclusion_rule");
      setError(null);
      setPrivacyActionStatus(null);
      try {
        await invoke<ExclusionRule>("add_exclusion_rule", { rule });
        await refreshExclusionRules();
        await refreshStatus();
        setPrivacyActionStatus(successMessage);
      } catch (err) {
        setPrivacyActionStatus(`Could not add privacy exclusion: ${String(err)}`);
      } finally {
        setBusyAction(null);
      }
    },
    [refreshExclusionRules, refreshStatus],
  );

  const excludeCurrentApp = useCallback(async () => {
    if (!currentAppPattern) {
      setPrivacyActionStatus("No current app is available to exclude yet.");
      return;
    }
    if (currentAppExcluded) {
      setPrivacyActionStatus(`${currentAppLabel || "This app"} is already excluded.`);
      return;
    }
    await addPrivacyExclusion(
      {
        rule_type: "app_bundle",
        pattern: currentAppPattern,
        action: "skip_capture",
        enabled: true,
      },
      `${currentAppLabel || "This app"} will be skipped by local memory.`,
    );
  }, [addPrivacyExclusion, currentAppExcluded, currentAppLabel, currentAppPattern]);

  const excludeCurrentWebsite = useCallback(async () => {
    if (!currentWebsitePattern) {
      setPrivacyActionStatus("No current website is available to exclude yet.");
      return;
    }
    if (currentWebsiteExcluded) {
      setPrivacyActionStatus(`${currentWebsiteLabel || "This website"} is already excluded.`);
      return;
    }
    await addPrivacyExclusion(
      {
        rule_type: "url_regex",
        pattern: currentWebsitePattern,
        action: "skip_capture",
        enabled: true,
      },
      `${currentWebsiteLabel || "This website"} will be skipped by local memory.`,
    );
  }, [addPrivacyExclusion, currentWebsiteExcluded, currentWebsiteLabel, currentWebsitePattern]);

  const removeExclusionRule = useCallback(
    async (ruleId: string) => {
      setBusyAction("remove_exclusion_rule");
      setError(null);
      setPrivacyActionStatus(null);
      try {
        await invoke<boolean>("remove_exclusion_rule", { ruleId });
        await refreshExclusionRules();
        setPrivacyActionStatus("Privacy exclusion removed.");
      } catch (err) {
        setPrivacyActionStatus(`Could not remove privacy exclusion: ${String(err)}`);
      } finally {
        setBusyAction(null);
      }
    },
    [refreshExclusionRules],
  );

  useEffect(() => {
    let disposed = false;
    const unlisteners: Array<() => void> = [];

    listen<CaptureFrame>("capture-frame", (event) => {
      void refreshStatus();
      if (!selectedFrameRef.current) {
        selectedFrameRef.current = event.payload;
        setSelectedFrame(event.payload);
      }
      void refreshContinueMemory();
      if (diagnosticsOpenRef.current) {
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

    listen<CaptureStatus>("capture-status", (event) => {
      statusRef.current = event.payload;
      setStatus(event.payload);
      if (diagnosticsOpenRef.current) {
        void refreshContinueMemory();
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

    listen<ContinueDecisionResult>("smalltalk-continue-updated", (event) => {
      void applyContinueDecision(event.payload, "island");
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
    applyContinueDecision,
    refreshContinueMemory,
    refreshWorkstreams,
    refreshStatus,
  ]);

  return (
    <main className={`capture-shell app-mode-${viewMode} ${viewMode === "developer" ? "developer-mode" : "continue-mode"} ${sidebarOpen ? "sidebar-open" : "sidebar-collapsed"}`}>
      <button
        className="sidebar-shell-toggle"
        type="button"
        aria-controls="smalltalk-sidebar"
        aria-expanded={sidebarOpen}
        aria-label={sidebarOpen ? "Collapse sidebar" : "Expand sidebar"}
        title={sidebarOpen ? "Collapse sidebar" : "Expand sidebar"}
        onClick={() => setSidebarOpen((open) => !open)}
      >
        <ProductIcon name="sidebar" />
      </button>

      <aside
        className="capture-topbar product-sidebar"
        id="smalltalk-sidebar"
        aria-label="Smalltalk navigation"
      >
        <div className="identity-block">
          <div className="brand-mark" aria-hidden="true">
            <img src={smalltalkLogo} alt="" />
          </div>
          <div>
            <p className="product-kicker">Smalltalk</p>
          </div>
        </div>

        <nav className="primary-nav" aria-label="Main screens">
          <button
            className={viewMode === "continue" ? "active" : ""}
            type="button"
            aria-label="Continue"
            title="Continue"
            aria-current={viewMode === "continue" ? "page" : undefined}
            onClick={() => openProductView("continue")}
          >
            <ProductIcon name="continue" />
            <span>Continue</span>
          </button>
          <button
            className={viewMode === "history" ? "active" : ""}
            type="button"
            aria-label="History"
            title="History"
            aria-current={viewMode === "history" ? "page" : undefined}
            onClick={() => openProductView("history")}
          >
            <ProductIcon name="history" />
            <span>History</span>
          </button>
        </nav>

        <div className="topbar-status" aria-label="Local memory status">
          {viewMode === "developer" ? (
            <button
              className="secondary-button"
              type="button"
              onClick={() => {
                setViewMode("continue");
                setEvidenceOpen(false);
              }}
            >
              Back to Continue
            </button>
          ) : null}
        </div>

        <nav className="secondary-nav" aria-label="Settings and advanced screens">
          <button
            className={viewMode === "settings" ? "active" : ""}
            type="button"
            aria-label="Settings"
            title="Settings"
            aria-current={viewMode === "settings" ? "page" : undefined}
            onClick={() => openProductView("settings")}
          >
            <ProductIcon name="settings" />
            <span>Settings</span>
          </button>
          {showInspectEntry ? (
            <button
              className={viewMode === "developer" ? "active" : ""}
              type="button"
              aria-label="Inspect"
              title="Inspect"
              aria-current={viewMode === "developer" ? "page" : undefined}
              onClick={openDeveloperMode}
            >
              <ProductIcon name="inspect" />
              <span>Inspect</span>
            </button>
          ) : null}
          <button
            className="logout-nav-button"
            type="button"
            disabled={authLoading}
            aria-busy={authLoading}
            aria-label="Log out"
            title="Log out"
            onClick={() => void signOut()}
          >
            <ProductIcon name="logout" />
            <span>{authLoading ? "Logging out" : "Log out"}</span>
          </button>
        </nav>
      </aside>

      <div
        className="app-scroll"
        ref={appScrollRef}
        onScroll={() => {
          if (memoryMenuOpen) setMemoryMenuOpen(false);
        }}
      >
      <header className={`product-toolbar ${viewMode === "continue" ? "continue-toolbar" : ""}`}>
        {viewMode === "continue" ? (
          <h1 className="continue-greeting">
            Hey {greetingName}, pick up where you left off
          </h1>
        ) : (
          <div>
            <p>{viewMode === "developer" ? "Advanced" : "Smalltalk"}</p>
            <h1>{viewMode === "history"
              ? "History"
              : viewMode === "settings"
                ? "Settings"
                : "Evidence inspection"}</h1>
          </div>
        )}
        {viewMode !== "developer" ? (
          <button
            className={`toolbar-memory-toggle ${memoryProductStatus}`}
            type="button"
            disabled={busyAction !== null}
            aria-busy={busyAction === "start_capture" || busyAction === "stop_capture"}
            aria-label={status.running ? "Turn memory off" : "Turn memory on"}
            title={status.running ? "Turn memory off" : "Turn memory on"}
            onClick={() => void runAction(status.running ? "stop_capture" : "start_capture")}
          >
            <ProductIcon name="memory" />
            <span>
              {busyAction === "start_capture"
                ? "Turning memory on"
                : busyAction === "stop_capture"
                  ? "Turning memory off"
                  : status.running
                    ? "Memory on"
                    : "Memory off"}
            </span>
            <i aria-hidden="true" />
          </button>
        ) : null}
      </header>
      {viewMode === "continue" ? (
      <section className="continue-home" aria-label="Continue">
        <div className="continue-stage">
          <ContinuationAnswer
            decision={continueDecision}
            primaryMessage={continuePrimaryMessage}
            hasEvidence={continueHasEvidence}
            running={status.running}
            busyAction={busyAction}
            visualCueVisible={evidenceOpen}
            selectedFrame={selectedFrame}
            imageData={imageData}
            openResult={continueOpenResult}
            onStartMemory={() => void runAction("start_capture")}
            onOpenTarget={() => void openContinueTarget()}
            feedbackStatus={feedbackStatus}
            onRecordFeedback={(kind, options) => void recordContinueFeedback(kind, options)}
            onUseAlternative={(candidate) => void continueFromAlternative(candidate)}
            onRevealVisualCue={(frameId) => void revealContinueFrame(frameId)}
          />
	        </div>
	      </section>
      ) : null}

	      {privacyPanelOpen ? (
	        <div
	          className="privacy-backdrop"
	          role="presentation"
	          onMouseDown={(event) => {
	            if (event.target === event.currentTarget) setPrivacyPanelOpen(false);
	          }}
	        >
	          <PrivacyPanel
	            status={status}
	            memoryProductStatus={memoryProductStatus}
	            memoryProduct={memoryProduct}
	            exclusionRules={exclusionRules}
	            currentAppLabel={currentAppLabel}
	            currentWebsiteLabel={currentWebsiteLabel}
	            currentAppExcluded={currentAppExcluded}
	            currentWebsiteExcluded={currentWebsiteExcluded}
	            hasCurrentApp={Boolean(currentAppPattern)}
	            hasCurrentWebsite={Boolean(currentWebsitePattern)}
	            busyAction={busyAction}
	            privacyActionStatus={privacyActionStatus}
	            onClose={() => setPrivacyPanelOpen(false)}
	            onStartMemory={() => void runAction("start_capture")}
	            onPauseMemory={() => void runAction("stop_capture")}
	            onExcludeCurrentApp={() => void excludeCurrentApp()}
	            onExcludeCurrentWebsite={() => void excludeCurrentWebsite()}
	            onRemoveExclusion={(ruleId) => void removeExclusionRule(ruleId)}
	            onDeleteRecentMemory={deleteRecentMemory}
	            onDeleteAllMemory={deleteAllFrames}
	          />
	        </div>
	      ) : null}

		      {viewMode === "continue" && continueError ? (
		        <MemoryErrorBox message={continueError} />
		      ) : null}

              {viewMode === "continue" && screenCapturePermission && !screenCapturePermission.granted ? (
                <ScreenCapturePermissionBox
                  permission={screenCapturePermission}
                  busy={busyAction !== null}
                  onRequest={() => void requestScreenCapturePermission()}
                />
              ) : null}

		      {viewMode === "continue" && visibleRuntimeError ? (
	        <MemoryErrorBox message={visibleRuntimeError} />
	      ) : null}

      {viewMode === "history" ? (
        <ContinueHistoryScreen
          page={historyPage}
          output={historyOutput}
          busy={historyBusy}
          error={historyError}
          onRefresh={() => void loadContinueHistory()}
          onLoadOlder={() => {
            if (historyPage?.next_cursor) {
              void loadContinueHistory(historyPage.next_cursor, true);
            }
          }}
          onSelect={(decisionId) => void loadContinueHistoryOutput(decisionId)}
          onBack={() => {
            setHistoryOutput(null);
            appScrollRef.current?.scrollTo({ top: 0 });
          }}
        />
      ) : null}

      {viewMode === "settings" ? (
        <SettingsHome
          memoryProductStatus={memoryProductStatus}
          memoryProduct={memoryProduct}
          permission={screenCapturePermission}
          exclusionCount={exclusionRules.filter((rule) => rule.enabled).length}
          latestEvidenceAgeLabel={latestEvidenceAgeLabel}
          busyAction={busyAction}
          hasEvidence={continueHasEvidence}
          showInspectEntry={showInspectEntry}
          accountEmail={user?.email || profile?.email || "Signed in"}
          authBusy={authLoading}
          authError={authError}
          onOpenPrivacy={openPrivacyPanel}
          onRequestPermission={() => void requestScreenCapturePermission()}
          onDeleteMemory={deleteAllFrames}
          onInspect={openDeveloperMode}
          onSignOut={() => void signOut()}
        />
      ) : null}

      {viewMode === "developer" ? (
              <section className="developer-panel diagnostics-panel" aria-label="Inspect local evidence">
        <div className="developer-panel-head">
          <div>
            <span>Inspect local evidence</span>
            <strong>Evidence inspector, search, raw event rows, and local memory substrate</strong>
          </div>
	          <div className="control-strip" aria-label="Local memory controls">
            <button
              className="primary-button"
              disabled={busyAction !== null}
              aria-busy={continueRefreshBusy}
              onClick={() => void runContinueDecision({ writeAudit: true })}
            >
              {continueRefreshBusy ? "Understanding your recent work…" : "Continue"}
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
                  {busyAction === "start_capture" ? "Starting" : "Turn on local memory"}
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
	                  {busyAction === "stop_capture" ? "Pausing" : "Pause memory"}
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
	                  {busyAction === "capture_once" ? "Adding" : "Add evidence"}
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
        </div>

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
                onClick={() => void runContinueDecision({
                  forceRebuild: true,
                  writeAudit: true,
                  auditMode: "full",
                })}
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
                onClick={requestDevReset}
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
                <MetricBlock label="Work queued" value={String(memoryDiagnostics.runtime_diagnostics.workload.queued_operation_count)} />
                <MetricBlock label="Work coalesced" value={String(memoryDiagnostics.runtime_diagnostics.workload.coalesced_requests)} />
                <MetricBlock label="Background avoided" value={String(memoryDiagnostics.runtime_diagnostics.workload.background_decisions_avoided)} />
                <MetricBlock label="SCK display" value={String(memoryDiagnostics.runtime_diagnostics.sck_display_successes)} />
                <MetricBlock label="SCK active window" value={String(memoryDiagnostics.runtime_diagnostics.sck_active_window_successes)} />
                <MetricBlock label="Capture fallbacks" value={String(memoryDiagnostics.runtime_diagnostics.screencapture_fallbacks)} />
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
                    {memoryDiagnostics.budgets.max_screenshots_per_10_minutes} screenshots per 10 minutes; low-value interval {Math.round(memoryDiagnostics.budgets.min_low_value_capture_interval_ms / 1000)}s; {memoryDiagnostics.budgets.max_retained_low_value_ui_events} retained low-value events
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
                  <dt>Event pressure</dt>
                  <dd>
                    {memoryDiagnostics.event_count} stored events; {memoryDiagnostics.excess_low_value_events} excess scroll/AX rows; {memoryDiagnostics.self_capture_events} Smalltalk self rows
                  </dd>
                </div>
                <div>
                  <dt>Screen capture providers</dt>
                  <dd>
                    {memoryDiagnostics.runtime_diagnostics.sck_active_window_abnormal_exits} abnormal active-window exits; {memoryDiagnostics.runtime_diagnostics.sck_timeouts} timeouts; {memoryDiagnostics.runtime_diagnostics.sck_circuit_breaker_opens} breaker opens; breaker {memoryDiagnostics.runtime_diagnostics.sck_active_window_circuit_breaker_state}
                  </dd>
                </div>
                <div>
                  <dt>Latest screen capture operation</dt>
                  <dd>
                    {memoryDiagnostics.runtime_diagnostics.latest_sck_capture_mode || "none"} via {memoryDiagnostics.runtime_diagnostics.latest_sck_provider || "none"}; {memoryDiagnostics.runtime_diagnostics.latest_sck_exit_category || "no result"}; {memoryDiagnostics.runtime_diagnostics.latest_sck_duration_ms ?? 0}ms; fallback {memoryDiagnostics.runtime_diagnostics.latest_sck_fallback_used ? "used" : "not used"}
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
            placeholder="Search evidence"
            aria-label="Search evidence"
          />
          <button type="submit" disabled={busyAction !== null}>Search evidence</button>
        </form>

      <section className="health-strip" aria-label="Local memory health">
        <StatusPill label="State" value={captureStateLabel} tone={status.running ? "good" : "quiet"} />
        <StatusPill label="Memory window" value={memoryWindowLabel} tone={status.running ? "good" : "quiet"} />
        <StatusPill label="Signals" value={status.signal_count} />
        <StatusPill label="Anchors" value={status.frame_count} />
        <StatusPill label="Events" value={status.event_count} />
        <StatusPill label="Transitions" value={status.transition_count} />
        <StatusPill label="Units" value={status.content_unit_count} />
        <StatusPill label="Memory windows" value={status.session_count} />
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

      {continueAdoptionDiagnostics.length > 0 ? (
        <section className="continue-eval-panel" aria-label="Continue result adoption diagnostics">
          <div className="detail-section-head">
            <div>
              <h3>Result adoption</h3>
              <span>Bounded record of quieter results that did not replace a stronger answer</span>
            </div>
            <span>{continueAdoptionDiagnostics.length}</span>
          </div>
          <div className="event-feed">
            {continueAdoptionDiagnostics.map((diagnostic) => (
              <div
                className="event-row"
                key={`${diagnostic.recordedAtMs}-${diagnostic.challengerDecisionId}`}
              >
                <time>{formatTime(diagnostic.recordedAtMs)}</time>
                <strong>{sentenceCase(diagnostic.challengerTrigger)}</strong>
                <span>{diagnostic.reasonCodes.join(" / ")}</span>
              </div>
            ))}
          </div>
        </section>
      ) : null}

      <section className="inspector-grid">
        <aside className="timeline-pane" aria-label="Evidence anchors">
          <div className="pane-heading">
            <div>
              <h2>Evidence history</h2>
              <p>{hasQuery ? "Filtered evidence" : "Most recent local evidence"}</p>
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
              <h3>Raw event rows</h3>
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
              <p className="feed-empty">No raw event rows in the last 10 minutes.</p>
            ) : null}
          </div>
        </aside>

        <section className="viewer-pane" aria-label="Evidence inspector">
          <div className="viewer-toolbar">
            <div>
              <p className="product-kicker">{productizeEvidenceTrigger(selectedFrame?.capture_trigger) || "waiting"}</p>
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
                <strong>No evidence anchor selected</strong>
                <span>Choose an evidence anchor or add evidence to inspect the screenshot, sources, and transitions.</span>
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
                <h2>Last evidence update</h2>
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
              <div className="complete-box">All core verification signals are present for this evidence anchor.</div>
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
                <dt>Weak surface</dt>
                <dd>
                  {frameDetail?.weak_surface_classification
                    ? `${sentenceCase(frameDetail.weak_surface_classification.domain)} / ${sentenceCase(frameDetail.weak_surface_classification.enrichment_need)}`
                    : "not classified"}
                </dd>
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
      </section>
      ) : null}
	      </div>
	      {pendingDangerAction ? (
	        <DangerConfirmDialog
	          action={pendingDangerAction}
	          busyAction={busyAction}
	          onCancel={() => setPendingDangerAction(null)}
	          onConfirm={() => void confirmDangerAction()}
	        />
	      ) : null}
	    </main>
	  );
	}

type ProductIconName =
  | "sidebar"
  | "continue"
  | "history"
  | "settings"
  | "logout"
  | "inspect"
  | "refresh"
  | "chevron"
  | "chevronDown"
  | "chevronUp"
  | "more"
  | "back"
  | "memory"
  | "privacy"
  | "permission"
  | "data";

function ProductIcon({ name }: { name: ProductIconName }) {
  const common = { size: 18, weight: "regular" as const, "aria-hidden": true };
  if (name === "sidebar") return <SidebarSimple {...common} />;
  if (name === "continue") return <ArrowBendDownRight {...common} />;
  if (name === "history") return <ClockCounterClockwise {...common} />;
  if (name === "settings") return <SlidersHorizontal {...common} />;
  if (name === "logout") return <SignOut {...common} />;
  if (name === "inspect") return <Eye {...common} />;
  if (name === "refresh") return <ArrowsClockwise {...common} />;
  if (name === "chevron") return <CaretRight {...common} />;
  if (name === "chevronDown") return <CaretDown {...common} />;
  if (name === "chevronUp") return <CaretUp {...common} />;
  if (name === "more") return <DotsThree {...common} weight="bold" />;
  if (name === "back") return <CaretLeft {...common} />;
  if (name === "memory") return <Brain {...common} />;
  if (name === "privacy") return <ShieldCheck {...common} />;
  if (name === "permission") return <Monitor {...common} />;
  return <Database {...common} />;
}

function ContinueHistoryScreen({
  page,
  output,
  busy,
  error,
  onRefresh,
  onLoadOlder,
  onSelect,
  onBack,
}: {
  page: ContinueHistoryPage | null;
  output: ContinueHistoryOutput | null;
  busy: boolean;
  error: string | null;
  onRefresh: () => void;
  onLoadOlder: () => void;
  onSelect: (decisionId: string) => void;
  onBack: () => void;
}) {
  if (output) {
    const rows = productHistoryRows(output.rows);
    return (
      <section className="history-screen history-detail" aria-label="Saved continuation">
        <button className="history-back" type="button" onClick={onBack}>
          <ProductIcon name="back" />
          Previous continuations
        </button>
        <article className="history-detail-sheet">
          <div className="history-detail-heading">
            <div>
              <span>{formatHistoryDate(output.created_at_ms)} · {output.origin === "island" ? "Island" : "Main app"}</span>
              <h2>{sanitizeHistoryCopy(output.title)}</h2>
            </div>
            <span className="history-read-only">Read only</span>
          </div>
          {rows.length > 0 ? (
            <dl className="history-detail-rows">
              {rows.map((row) => (
                <div key={`${row.label}:${row.value}`}>
                  <dt>{row.label}</dt>
                  <dd>{row.value}</dd>
                </div>
              ))}
            </dl>
          ) : (
            <p className="history-detail-empty">This saved answer contains no additional context.</p>
          )}
        </article>
      </section>
    );
  }

  return (
    <section className="history-screen" aria-label="Previous continuations">
      <div className="screen-intro">
        <div>
          <span>Previous continuations</span>
          <h2>Return to an earlier answer</h2>
          <p>Answers you explicitly requested are saved locally and stay read only here.</p>
        </div>
        <button className="secondary-button" type="button" disabled={busy} onClick={onRefresh}>
          <ProductIcon name="refresh" />
          Refresh
        </button>
      </div>

      {busy && !page ? (
        <div className="history-loading" aria-label="Loading previous continuations">
          {[0, 1, 2, 3].map((index) => <span key={index} />)}
        </div>
      ) : null}

      {error ? <p className="history-error" role="status">{productizeMemoryError(error)}</p> : null}

      {page && page.items.length === 0 ? (
        <div className="history-empty-state">
          <div><ProductIcon name="history" /></div>
          <h3>No previous answers yet</h3>
          <p>Use Continue when you want Smalltalk to save a return point for later.</p>
        </div>
      ) : null}

      {page && page.items.length > 0 ? (
        <div className="history-list">
          {groupHistoryItems(page.items).map((group) => (
            <section key={group.label} className="history-day-group">
              <h3>{group.label}</h3>
              <div>
                {group.items.map((item) => (
                  <button
                    key={item.decision_id}
                    className="history-row"
                    type="button"
                    disabled={busy}
                    onClick={() => onSelect(item.decision_id)}
                  >
                    <span className="history-row-mark" aria-hidden="true"><ProductIcon name="continue" /></span>
                    <span className="history-row-copy">
                      <strong>{sanitizeHistoryCopy(item.title)}</strong>
                      <small>{item.origin === "island" ? "Island" : "Main app"} · {formatHistoryTime(item.created_at_ms)}</small>
                    </span>
                    <ProductIcon name="chevron" />
                  </button>
                ))}
              </div>
            </section>
          ))}
          {page.next_cursor ? (
            <button className="history-load-more" type="button" disabled={busy} onClick={onLoadOlder}>
              {busy ? "Loading" : "Load older answers"}
            </button>
          ) : null}
        </div>
      ) : null}
    </section>
  );
}

function SettingsHome({
  memoryProductStatus,
  memoryProduct,
  permission,
  exclusionCount,
  latestEvidenceAgeLabel,
  busyAction,
  hasEvidence,
  showInspectEntry,
  accountEmail,
  authBusy,
  authError,
  onOpenPrivacy,
  onRequestPermission,
  onDeleteMemory,
  onInspect,
  onSignOut,
}: {
  memoryProductStatus: MemoryProductStatus;
  memoryProduct: { label: string; detail: string };
  permission: ScreenCapturePermissionStatus | null;
  exclusionCount: number;
  latestEvidenceAgeLabel: string;
  busyAction: string | null;
  hasEvidence: boolean;
  showInspectEntry: boolean;
  accountEmail: string;
  authBusy: boolean;
  authError: string | null;
  onOpenPrivacy: () => void;
  onRequestPermission: () => void;
  onDeleteMemory: () => void;
  onInspect: () => void;
  onSignOut: () => void;
}) {
  const update = useAppUpdate();
  const updateCopy = appUpdateStatusCopy(update);
  const updateBusy = ["checking", "downloading", "installing", "restarting"].includes(update.phase);

  return (
    <section className="settings-screen" aria-label="Settings">
      <div className="screen-intro">
        <div>
          <span>Smalltalk on this Mac</span>
          <h2>Memory and privacy</h2>
          <p>Control what Smalltalk remembers, what it ignores, and when it can help you continue.</p>
        </div>
      </div>

      <div className="settings-stack">
        <section className="settings-section">
          <div className="settings-icon"><ProductIcon name="settings" /></div>
          <div className="settings-section-copy">
            <span>Account</span>
            <h3>{accountEmail}</h3>
            <p>
              Your Smalltalk account is signed in. Signing out does not delete local memory or captures.
              {authError ? ` ${authError}` : ""}
            </p>
          </div>
          <div className="settings-actions">
            <button
              className="secondary-button"
              type="button"
              disabled={authBusy}
              aria-busy={authBusy}
              onClick={onSignOut}
            >
              {authBusy ? "Signing out" : "Sign out"}
            </button>
          </div>
        </section>

        <section className="settings-section">
          <div className="settings-icon"><ProductIcon name="memory" /></div>
          <div className="settings-section-copy">
            <span>Local memory</span>
            <h3>{memoryProduct.label}</h3>
            <p>{memoryProduct.detail} Latest evidence: {latestEvidenceAgeLabel}.</p>
          </div>
          <span className={`settings-status-dot ${memoryProductStatus}`} aria-hidden="true" />
        </section>

        <section className="settings-section">
          <div className="settings-icon"><ProductIcon name="privacy" /></div>
          <div className="settings-section-copy">
            <span>Privacy</span>
            <h3>Local by default</h3>
            <p>Raw typed characters and full clipboard contents are never stored. {exclusionCount ? `${exclusionCount} exclusion${exclusionCount === 1 ? " is" : "s are"} active.` : "No app or website exclusions are active."}</p>
          </div>
          <div className="settings-actions">
            <button className="secondary-button" type="button" onClick={onOpenPrivacy}>Manage privacy</button>
          </div>
        </section>

        <section className={`settings-section ${permission && !permission.granted ? "needs-attention" : ""}`}>
          <div className="settings-icon"><ProductIcon name="permission" /></div>
          <div className="settings-section-copy">
            <span>Permissions</span>
            <h3>{permission?.granted ? "Screen recording is ready" : "Screen recording needs attention"}</h3>
            <p>{permission?.message || "Smalltalk checks macOS permission before collecting visual memory."}</p>
          </div>
          {permission && !permission.granted ? (
            <div className="settings-actions">
              <button className="secondary-button" type="button" disabled={busyAction !== null} onClick={onRequestPermission}>Review permission</button>
            </div>
          ) : null}
        </section>

        <section className={`settings-section ${update.phase === "available" || update.phase === "error" ? "needs-attention" : ""}`}>
          <div className="settings-icon"><ProductIcon name="refresh" /></div>
          <div className="settings-section-copy">
            <span>Software update</span>
            <h3>{updateCopy.title}</h3>
            <p>{updateCopy.detail}</p>
          </div>
          <div className="settings-actions">
            {update.phase === "available" ? (
              <button
                className="primary-button"
                type="button"
                onClick={() => void update.installUpdate()}
              >
                Update and restart
              </button>
            ) : (
              <button
                className="secondary-button"
                type="button"
                disabled={updateBusy || update.phase === "unavailable"}
                aria-busy={update.phase === "checking"}
                onClick={() => void update.checkForUpdates(true)}
              >
                {update.phase === "checking" ? "Checking" : "Check for updates"}
              </button>
            )}
          </div>
        </section>

        <section className="settings-section danger-zone">
          <div className="settings-icon"><ProductIcon name="data" /></div>
          <div className="settings-section-copy">
            <span>Stored data</span>
            <h3>Delete local memory</h3>
            <p>Remove captured evidence, derived Continue data, and previous answers from this Mac.</p>
          </div>
          <div className="settings-actions">
            <button className="danger-button" type="button" disabled={!hasEvidence || busyAction !== null} onClick={onDeleteMemory}>Delete local memory</button>
          </div>
        </section>

        {showInspectEntry ? (
          <section className="settings-section advanced-settings">
            <div className="settings-icon"><ProductIcon name="inspect" /></div>
            <div className="settings-section-copy">
              <span>Advanced</span>
              <h3>Inspect local evidence</h3>
              <p>Open the developer workspace for frames, events, diagnostics, and audit information.</p>
            </div>
            <div className="settings-actions">
              <button className="secondary-button" type="button" onClick={onInspect}>Open Inspect</button>
            </div>
          </section>
        ) : null}
      </div>
    </section>
  );
}

function productHistoryRows(rows: ContinueHistoryAnswerRow[]) {
  const labels: Record<string, string> = {
    "Task object": "What you were doing",
    "Current activity — observed surface": "Where you were working",
    "Current activity — immediate operation": "What was happening",
    "Current activity — operation effect": "What changed",
    "Current activity — current subtask": "Current step",
    "Last meaningful progress": "Progress",
    "Unfinished state": "Still open",
    "Next action": "Next step",
    "Where summary": "Return to",
  };
  return rows
    .filter((row) => row.label !== "Current activity — relationship to primary")
    .map((row) => ({
      label: labels[row.label] || row.label,
      value: sanitizeHistoryCopy(row.value),
    }));
}

function sanitizeHistoryCopy(value: string) {
  const cleaned = value
    .replace(/\[(?:T|B|P)\d+_[A-Z0-9_]+\]/g, " ")
    .replace(/\s+/g, " ")
    .trim();
  return cleaned || "Couldn’t recover the task";
}

function groupHistoryItems(items: ContinueHistorySummary[]) {
  const groups = new Map<string, ContinueHistorySummary[]>();
  items.forEach((item) => {
    const label = formatHistoryDay(item.created_at_ms);
    groups.set(label, [...(groups.get(label) || []), item]);
  });
  return Array.from(groups, ([label, groupedItems]) => ({ label, items: groupedItems }));
}

function formatHistoryDay(timestampMs: number) {
  const date = new Date(timestampMs);
  const today = new Date();
  const startToday = new Date(today.getFullYear(), today.getMonth(), today.getDate()).getTime();
  const startDate = new Date(date.getFullYear(), date.getMonth(), date.getDate()).getTime();
  const dayDifference = Math.round((startToday - startDate) / 86_400_000);
  if (dayDifference === 0) return "Today";
  if (dayDifference === 1) return "Yesterday";
  return date.toLocaleDateString(undefined, { month: "long", day: "numeric", year: date.getFullYear() === today.getFullYear() ? undefined : "numeric" });
}

function formatHistoryDate(timestampMs: number) {
  return new Date(timestampMs).toLocaleString(undefined, {
    month: "short",
    day: "numeric",
    year: "numeric",
    hour: "numeric",
    minute: "2-digit",
  });
}

function formatHistoryTime(timestampMs: number) {
  return new Date(timestampMs).toLocaleTimeString(undefined, {
    hour: "numeric",
    minute: "2-digit",
  });
}

function PrivacyPanel({
  status,
  memoryProductStatus,
  memoryProduct,
  exclusionRules,
  currentAppLabel,
  currentWebsiteLabel,
  currentAppExcluded,
  currentWebsiteExcluded,
  hasCurrentApp,
  hasCurrentWebsite,
  busyAction,
  privacyActionStatus,
  onClose,
  onStartMemory,
  onPauseMemory,
  onExcludeCurrentApp,
  onExcludeCurrentWebsite,
  onRemoveExclusion,
  onDeleteRecentMemory,
  onDeleteAllMemory,
}: {
  status: CaptureStatus;
  memoryProductStatus: MemoryProductStatus;
  memoryProduct: { label: string; detail: string };
  exclusionRules: ExclusionRule[];
  currentAppLabel: string;
  currentWebsiteLabel: string;
  currentAppExcluded: boolean;
  currentWebsiteExcluded: boolean;
  hasCurrentApp: boolean;
  hasCurrentWebsite: boolean;
  busyAction: string | null;
  privacyActionStatus: string | null;
  onClose: () => void;
  onStartMemory: () => void;
  onPauseMemory: () => void;
  onExcludeCurrentApp: () => void;
  onExcludeCurrentWebsite: () => void;
  onRemoveExclusion: (ruleId: string) => void;
  onDeleteRecentMemory: () => void;
  onDeleteAllMemory: () => void;
}) {
  const activeRules = exclusionRules.filter((rule) => rule.enabled);
  const memoryBusy = busyAction === "start_capture" || busyAction === "stop_capture";
  const exclusionBusy = busyAction === "add_exclusion_rule" || busyAction === "remove_exclusion_rule";
  const deleting = busyAction === "delete_all_frames" || busyAction === "delete_recent_captures";
  const startMemoryLabel = "Turn on local memory";
  const panelRef = useRef<HTMLElement | null>(null);
  const closeRef = useRef(onClose);

  useEffect(() => {
    closeRef.current = onClose;
  }, [onClose]);

  useEffect(() => {
    const previousFocus = document.activeElement instanceof HTMLElement
      ? document.activeElement
      : null;
    const restoreFocus = previousFocus?.closest("details")
      ?.querySelector<HTMLElement>("summary") || previousFocus;
    const panel = panelRef.current;
    panel?.focus();

    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        closeRef.current();
        return;
      }
      if (event.key !== "Tab" || !panel) return;

      const focusable = Array.from(
        panel.querySelectorAll<HTMLElement>(
          'button:not(:disabled), summary, [href], [tabindex]:not([tabindex="-1"])',
        ),
      ).filter((element) => !element.hasAttribute("hidden"));
      if (focusable.length === 0) {
        event.preventDefault();
        panel.focus();
        return;
      }

      const first = focusable[0];
      const last = focusable[focusable.length - 1];
      if (event.shiftKey && document.activeElement === first) {
        event.preventDefault();
        last.focus();
      } else if (!event.shiftKey && document.activeElement === last) {
        event.preventDefault();
        first.focus();
      }
    };

    document.addEventListener("keydown", handleKeyDown);
    return () => {
      document.removeEventListener("keydown", handleKeyDown);
      restoreFocus?.focus();
    };
  }, []);

  return (
    <section
      className="privacy-panel"
      ref={panelRef}
      role="dialog"
      aria-modal="true"
      aria-labelledby="privacy-dialog-title"
      tabIndex={-1}
    >
      <div className="privacy-panel-head">
        <div>
          <p className="product-kicker">Privacy</p>
          <h2 id="privacy-dialog-title">Local memory boundaries</h2>
        </div>
        <button className="secondary-button" type="button" onClick={onClose}>
          Close
        </button>
      </div>

      <div className={`privacy-status-card ${memoryProductStatus}`}>
        <span>Local memory</span>
        <strong>{memoryProduct.label}</strong>
        <p>{memoryProduct.detail}</p>
      </div>

      <div className="privacy-grid">
        <section aria-label="What Smalltalk may use">
          <h3>What Smalltalk may use</h3>
          <ul>
            <li>App and window context</li>
            <li>Visible text when available</li>
            <li>Browser pages unless you exclude the app or website</li>
            <li>Lightweight activity signals</li>
            <li>Derived workstream metadata</li>
          </ul>
        </section>

        <section aria-label="What Smalltalk excludes">
          <h3>What Smalltalk excludes</h3>
          <ul>
            <li>Raw typed characters</li>
            <li>Full clipboard contents</li>
            <li>Apps and websites you exclude</li>
            <li>Evidence marked local-only</li>
          </ul>
        </section>
      </div>

      <div className="privacy-expanded-detail">
        Smalltalk includes browser screenshots, visible page text, URLs, and activity by default so browser work remains part of Continue. Add an app or website exclusion when you want that browser evidence omitted. Raw typed characters and full clipboard contents are never stored.
      </div>

      <div className="privacy-controls-grid" aria-label="Privacy controls">
        {status.running ? (
          <button
            className="secondary-button"
            type="button"
            disabled={busyAction !== null}
            aria-busy={busyAction === "stop_capture"}
            onClick={onPauseMemory}
          >
            {busyAction === "stop_capture" ? "Pausing" : "Pause memory"}
          </button>
        ) : (
          <button
            className="secondary-button"
            type="button"
            disabled={busyAction !== null}
            aria-busy={busyAction === "start_capture"}
            onClick={onStartMemory}
          >
            {busyAction === "start_capture" ? "Starting" : startMemoryLabel}
          </button>
        )}
        <button
          className="secondary-button"
          type="button"
          disabled={!hasCurrentApp || currentAppExcluded || exclusionBusy || memoryBusy || deleting}
          aria-busy={busyAction === "add_exclusion_rule"}
          onClick={onExcludeCurrentApp}
        >
          {currentAppExcluded ? "Current app excluded" : "Exclude this app"}
        </button>
        {hasCurrentWebsite ? (
          <button
            className="secondary-button"
            type="button"
            disabled={currentWebsiteExcluded || exclusionBusy || memoryBusy || deleting}
            aria-busy={busyAction === "add_exclusion_rule"}
            onClick={onExcludeCurrentWebsite}
          >
            {currentWebsiteExcluded ? "Current website excluded" : "Exclude this website"}
          </button>
        ) : null}
        <button
          className="danger-button"
          type="button"
          disabled={busyAction !== null}
          aria-busy={busyAction === "delete_recent_captures"}
          onClick={onDeleteRecentMemory}
        >
          Delete recent memory
        </button>
        <button
          className="danger-button"
          type="button"
          disabled={busyAction !== null}
          aria-busy={busyAction === "delete_all_frames"}
          onClick={onDeleteAllMemory}
        >
          Delete local memory
        </button>
      </div>

      <div className="current-surface-note">
        <span>Current surface</span>
        <p>
          {hasCurrentApp
            ? `${currentAppLabel || "This app"} is included in future local memory unless you exclude it.`
            : "Smalltalk has not observed an app that can be excluded yet."}
          {hasCurrentWebsite ? ` ${currentWebsiteLabel || "This website"} can also be excluded.` : ""}
        </p>
      </div>

      <section className="exclusion-list" aria-label="Current exclusions">
        <div className="exclusion-list-head">
          <h3>Current exclusions</h3>
          <span>{activeRules.length}</span>
        </div>
        {activeRules.length > 0 ? (
          <ul>
            {activeRules.map((rule) => (
              <li key={rule.id}>
                <div>
                  <strong>{formatExclusionRule(rule)}</strong>
                  <span>
                    {rule.origin === "system_default" ? "Built-in" : "Added by you"}
                    {` · ${formatExclusionAction(rule.action)}`}
                  </span>
                </div>
                <button
                  className="text-button"
                  type="button"
                  disabled={busyAction !== null}
                  onClick={() => onRemoveExclusion(rule.id)}
                >
                  Remove
                </button>
              </li>
            ))}
          </ul>
        ) : (
          <p>No user-visible exclusions are configured yet.</p>
        )}
      </section>

      {privacyActionStatus ? (
        <p className="privacy-action-status" role="status">{privacyActionStatus}</p>
      ) : null}
    </section>
  );
}

function MemoryErrorBox({ message }: { message: string }) {
  const productCopy = productizeMemoryError(message);
  return (
    <div className="error-box" role="alert">
      <strong>{productCopy}</strong>
    </div>
  );
}

function ScreenCapturePermissionBox({
  permission,
  busy,
  onRequest,
}: {
  permission: ScreenCapturePermissionStatus;
  busy: boolean;
  onRequest: () => void;
}) {
  return (
    <div className="error-box" role="status">
      <strong>{permission.message}</strong>
      {permission.can_request ? (
        <button
          className="primary-button"
          type="button"
          disabled={busy}
          aria-busy={busy}
          onClick={onRequest}
        >
          {busy ? "Waiting for macOS" : "Allow screen access"}
        </button>
      ) : null}
      <details>
        <summary>Running app identity</summary>
        <span>
          {permission.identity.bundle_identifier || "No macOS bundle identifier"}
          {" · "}
          {permission.identity.executable_path}
          {" · "}
          {permission.identity.team_identifier
            ? `Team ${permission.identity.team_identifier}`
            : permission.identity.signature_kind}
        </span>
      </details>
    </div>
  );
}

function DangerConfirmDialog({
  action,
  busyAction,
  onCancel,
  onConfirm,
}: {
  action: DangerousAction;
  busyAction: string | null;
  onCancel: () => void;
  onConfirm: () => void;
}) {
  const copy = dangerousActionCopy(action);
  const busy = busyAction === "delete_all_frames" ||
    busyAction === "delete_recent_captures" ||
    busyAction === "dev_reset_local_memory";
  return (
    <div className="confirm-backdrop" role="presentation">
      <section className="confirm-dialog" role="dialog" aria-modal="true" aria-label={copy.title}>
        <div>
          <h2>{copy.title}</h2>
          <p>{copy.body}</p>
        </div>
        <div className="confirm-actions">
          <button
            className="danger-button"
            type="button"
            disabled={busy || busyAction !== null}
            aria-busy={busy}
            onClick={onConfirm}
          >
            {copy.confirmLabel}
          </button>
          <button
            className="secondary-button"
            type="button"
            disabled={busy}
            onClick={onCancel}
          >
            Cancel
          </button>
        </div>
      </section>
    </div>
  );
}

function buildContinueEvidenceSnapshot(
  status: CaptureStatus,
  memory: ContinueMemoryStatus | null,
): ContinueEvidenceSnapshot {
  return {
    frameCount: Math.max(0, status.frame_count),
    eventCount: Math.max(0, status.event_count),
    signalCount: Math.max(0, status.signal_count),
    contentUnitCount: Math.max(0, status.content_unit_count),
    artifactCount: Math.max(0, memory?.counts.artifacts || 0),
    workstreamCount: Math.max(0, memory?.counts.workstreams || 0),
    latestFrameAtMs: status.latest_frame?.captured_at || null,
    latestArtifactAtMs: memory?.latest_artifact_timestamp || null,
    latestWorkstreamAtMs: memory?.latest_workstream_timestamp || null,
  };
}

function continueEvidenceChanged(
  decisionSnapshot: ContinueEvidenceSnapshot,
  currentSnapshot: ContinueEvidenceSnapshot,
) {
  return currentSnapshot.frameCount > decisionSnapshot.frameCount ||
    currentSnapshot.artifactCount > decisionSnapshot.artifactCount ||
    currentSnapshot.workstreamCount > decisionSnapshot.workstreamCount ||
    latestTimestamp(currentSnapshot) > latestTimestamp(decisionSnapshot);
}

function continueEvidenceSignature(snapshot: ContinueEvidenceSnapshot) {
  return [
    snapshot.frameCount,
    snapshot.artifactCount,
    snapshot.workstreamCount,
    latestTimestamp(snapshot),
  ].join(":");
}

function latestTimestamp(snapshot: ContinueEvidenceSnapshot) {
  return Math.max(
    snapshot.latestFrameAtMs || 0,
    snapshot.latestArtifactAtMs || 0,
    snapshot.latestWorkstreamAtMs || 0,
  );
}

function RecentContextVisit({ visit }: { visit: ContinueTaskTruthRecentContext }) {
  const surfaceLabel = recentContextSurfaceLabel(visit);
  const surfaceKind = recentContextSurfaceKind(visit);
  const relationship = safeProductLine(visit.relationship_to_primary_task || "", "");
  const activityLabel = relationship || {
    primary_work: "Worked on the task",
    supporting_work: "Supported the task",
    detour_or_unrelated: "Brief detour",
    unclear: visit.is_current ? "Current work surface" : "Observed work surface",
  }[visit.semantic_role || "unclear"];
  const marker = visit.is_current ? "Current" : visit.revisited ? "Returned" : null;

  return (
    <li
      className={[
        visit.is_current ? "current" : "",
        visit.semantic_role ? `role-${visit.semantic_role}` : "",
      ].filter(Boolean).join(" ")}
    >
      <div className="continuation-trail-stop-heading">
        <SurfaceGlyph
          label={surfaceLabel}
          appLabel={visit.app_label}
          siteHostname={visit.site_hostname}
        />
        <div className="answer-context-surface">
          <strong>{surfaceLabel}</strong>
          <span>{[surfaceKind, marker].filter(Boolean).join(" · ")}</span>
        </div>
      </div>
      <small>{activityLabel}</small>
    </li>
  );
}

function SurfaceGlyph({
  label,
  appLabel,
  siteHostname,
}: {
  label: string;
  appLabel?: string | null;
  siteHostname?: string | null;
}) {
  const normalized = `${label} ${appLabel || ""} ${siteHostname || ""}`.toLowerCase();
  if (normalized.includes("smalltalk")) {
    return (
      <span className="surface-glyph surface-glyph-smalltalk" aria-hidden="true" title={label}>
        <img src={smalltalkLogo} alt="" />
      </span>
    );
  }
  const isBrandIcon = [
    "codex",
    "chatgpt",
    "openai",
    "google chrome",
    "chrome",
    "github",
    "figma",
    "notion",
    "slack",
    "discord",
  ].some((brand) => normalized.includes(brand));
  const Icon = normalized.includes("codex")
    || normalized.includes("chatgpt")
    || normalized.includes("openai")
    ? OpenAiLogo
    : normalized.includes("chrome")
      ? GoogleChromeLogo
      : normalized.includes("github")
        ? GithubLogo
        : normalized.includes("figma")
          ? FigmaLogo
          : normalized.includes("notion")
            ? NotionLogo
            : normalized.includes("slack")
              ? SlackLogo
              : normalized.includes("discord")
                ? DiscordLogo
                : normalized.includes("safari")
                  ? Compass
                  : normalized.includes("terminal")
    ? TerminalWindow
    : normalized.includes("vs code") || normalized.includes("visual studio code") || normalized.includes("code")
      ? Code
      : siteHostname
        ? GlobeSimple
        : normalized.includes("chat")
        ? ChatCircleText
        : normalized.includes("http") || normalized.includes("www")
          ? Browser
          : Monitor;
  const tone = normalized.includes("vs code") || normalized.includes("visual studio code")
    ? "code"
    : normalized.includes("openai") || normalized.includes("codex") || normalized.includes("chatgpt")
      ? "openai"
      : normalized.includes("thinking machines")
        ? "research"
        : "neutral";

  return (
    <span className="surface-glyph" data-tone={tone} aria-hidden="true" title={label}>
      <Icon size={18} weight={isBrandIcon ? "fill" : "regular"} />
    </span>
  );
}

function ContinuationSurfaceBadge({
  surface,
}: {
  surface: ContinueSurfaceProjection;
}) {
  return (
    <div
      className="continuation-surface-badge"
      aria-label={`${surface.label}, ${surface.kind}`}
    >
      <SurfaceGlyph
        label={surface.label}
        appLabel={surface.appLabel}
        siteHostname={surface.siteHostname}
      />
      <span>
        <strong>{surface.label}</strong>
        <small>{surface.kind}</small>
      </span>
    </div>
  );
}

const continueMatrixPerimeterOrder = new Map(
  [0, 1, 2, 3, 4, 9, 14, 19, 24, 23, 22, 21, 20, 15, 10, 5].map(
    (cell, order) => [cell, order],
  ),
);

function ContinueGeneratingIndicator() {
  return (
    <div className="continue-generating" role="status" aria-live="polite">
      <div className="continue-dot-matrix" aria-hidden="true">
        {Array.from({ length: 25 }, (_, index) => {
          const perimeterOrder = continueMatrixPerimeterOrder.get(index);
          return (
            <span
              className={perimeterOrder === undefined ? "" : "perimeter"}
              key={index}
              style={{ "--dot-index": perimeterOrder ?? 0 } as CSSProperties}
            />
          );
        })}
      </div>
      <div>
        <strong>Generating answer…</strong>
        <span>Reconstructing the last meaningful place to continue.</span>
      </div>
    </div>
  );
}

function ContinuationDetailsPlaceholder({
  generating = false,
  hasEvidence,
  running,
  checkpointCopy: checkpointCopyOverride,
  continuationCopy: continuationCopyOverride,
}: {
  generating?: boolean;
  hasEvidence: boolean;
  running: boolean;
  checkpointCopy?: string | null;
  continuationCopy?: string | null;
}) {
  const checkpointCopy = checkpointCopyOverride || (generating
    ? "Finding the last clear point you reached…"
    : hasEvidence
      ? "Your last clear checkpoint will appear here when an answer is ready."
      : running
        ? "Keep working. Your last clear checkpoint will appear here once Smalltalk has enough context."
        : "Turn on local memory so Smalltalk can remember the last clear point you reached.");
  const continuationCopy = continuationCopyOverride || (generating
    ? "Working out the most useful next step…"
    : hasEvidence
      ? "Your next supported step will appear here when an answer is ready."
      : running
        ? "Smalltalk will suggest where to continue once it understands enough of your work."
        : "Once local memory is on, Smalltalk can help you return to unfinished work.");

  return (
    <section className="continuation-field continuation-field-placeholder" aria-label="Continuation preview">
      <div className="continuation-core">
        <section className="continuation-copy-block">
          <div className="continuation-copy-heading">
            <h3>Last checkpoint</h3>
          </div>
          <p>{checkpointCopy}</p>
        </section>
        <section className="continuation-copy-block continuation-next">
          <div className="continuation-copy-heading">
            <h3>Continue from here</h3>
          </div>
          <p>{continuationCopy}</p>
        </section>
      </div>
    </section>
  );
}

function ContinuationAnswer({
  decision,
  primaryMessage,
  hasEvidence,
  running,
  busyAction,
  visualCueVisible,
  selectedFrame,
  imageData,
  openResult,
  feedbackStatus,
  onStartMemory,
  onOpenTarget,
  onRecordFeedback,
  onUseAlternative,
  onRevealVisualCue,
}: {
  decision: ContinueDecisionResult | null;
  primaryMessage: string;
  hasEvidence: boolean;
  running: boolean;
  busyAction: string | null;
  visualCueVisible: boolean;
  selectedFrame: CaptureFrame | null;
  imageData: string | null;
  openResult: OpenResumePointResult | null;
  feedbackStatus: string | null;
  onStartMemory: () => void;
  onOpenTarget: () => void;
  onUseAlternative: (candidate: ContinueCandidateSummary) => void;
  onRevealVisualCue: (frameId?: string | null) => void;
  onRecordFeedback: (
    feedbackKind: string,
    options?: {
      taskSnapshotId?: string | null;
      taskSnapshotRevision?: number | null;
      affectedTaskField?: string | null;
      taskHypothesisId?: string | null;
    },
  ) => void;
}) {
  const taskTruthAnswer = authoritativeTaskTruthAnswer(decision);
  const taskTruthDiagnostic = decision?.task_truth_v2?.inference_diagnostic || null;
  const rawUnresolvedTaskTruthAnswer =
    decision?.task_truth_v2?.answer?.task_resolution_status === "unresolved"
      ? decision.task_truth_v2.answer
      : null;
  const cardTaskTruthAnswer = taskTruthAnswer || rawUnresolvedTaskTruthAnswer;
  const hasVisibleSemanticAnswer = hasVisibleTaskTruthSemantics(cardTaskTruthAnswer);
  const taskInferenceFailureStatus = normalizeToken(taskTruthDiagnostic?.status) === "success"
    ? cardTaskTruthAnswer?.inference_status
    : taskTruthDiagnostic?.status || cardTaskTruthAnswer?.inference_status;
  const taskInferenceFailed = !hasVisibleSemanticAnswer && (
    cardTaskTruthAnswer?.task_resolution_status === "unresolved"
    || Boolean(
      taskTruthDiagnostic
      && normalizeToken(taskTruthDiagnostic.status) !== "success",
    )
  );
  const taskInferenceFailure = taskInferenceFailed
    ? taskInferenceFailurePresentation(
        taskInferenceFailureStatus,
        taskTruthDiagnostic?.verification_status,
        taskTruthDiagnostic?.origin,
        taskTruthDiagnostic?.image_count,
        taskTruthDiagnostic?.provider_attempt_count,
      )
    : null;
  const resumeTarget = cardTaskTruthAnswer
    ? cardTaskTruthAnswer.direct_return_target || null
    : decision?.resume_work_target || decision?.return_target || null;
  const actionState = decision ? getContinueCardActionState(decision) : null;
  const isThinCurrentWork = actionState?.kind === "thin_current_work";
  const noClearCurrentTask = cardTaskTruthAnswer
    ? cardTaskTruthAnswer.task_resolution_status === "unresolved"
    : Boolean(
        decision
        && getContinueTaskResolutionStatus(decision) === "no_clear_current_task"
        && !hasSupportedWorkTruth(decision),
      );
  const canOpenResumeTarget = actionState?.kind === "openable_return_target";
  const lowConfidence = cardTaskTruthAnswer
    ? noClearCurrentTask
    : decision ? decision.confidence < 0.55 || noClearCurrentTask : false;
  const handoff = decision?.handoff || null;
  const activityRecap = noClearCurrentTask || cardTaskTruthAnswer
    ? null
    : usableActivityRecap(decision?.activity_recap);
  const presentation = decision ? presentContinueDecision(decision) : null;
  const [alternativesOpen, setAlternativesOpen] = useState(false);
  const [contextOpen, setContextOpen] = useState(false);
  const answerRef = useRef<HTMLElement | null>(null);
  const alternatives = noClearCurrentTask || cardTaskTruthAnswer
    ? []
    : (decision?.alternatives || []).filter(isPublicAlternativeCandidate);
  const visibleAlternatives = alternativesOpen ? alternatives.slice(0, 4) : [];
  const taskTruthAlternatives = cardTaskTruthAnswer?.alternative_hypotheses || [];
  const taskTruthActionState = actionState || {
    kind: "no_clear_continuation",
    label: "Inspect evidence",
  } satisfies ContinueCardActionState;
  const productState = decision && cardTaskTruthAnswer
    ? buildTaskTruthProductStateCopy(
        cardTaskTruthAnswer as TaskTruthPublicAnswer,
        taskTruthActionState,
        taskTruthDiagnostic,
      )
    : !taskInferenceFailure && decision && actionState && presentation
      ? buildContinueProductStateCopy(decision, actionState, presentation, primaryMessage)
      : null;
  const rawTargetLine = handoff?.return_line || presentation?.returnTarget || "No stable place to continue yet.";
  const workstreamLine = productState?.headline || taskInferenceFailure?.headline || safeProductLine(
    handoff?.headline || presentation?.workstreamTitle || primaryMessage,
    "Recent work",
  );
  const targetLine = productState?.targetLine || safeProductLine(rawTargetLine, "No stable place to continue yet.");
  const targetLooksInternal = isInternalFacingText(targetLine);
  const lastStateLine = cardTaskTruthAnswer
    ? productState?.lastStateLine || taskInferenceFailure?.detail || ""
    : productState?.lastStateLine
      || taskInferenceFailure?.detail
      || "No last meaningful state is clear yet.";
  const currentFocusLine = safeProductLine(
    productState?.currentFocusLine || (cardTaskTruthAnswer ? "" : stripCurrentFocusPrefix(
      safeProductLine(handoff?.current_focus_line || presentation?.currentFocus || "", ""),
    ) || humanFocusLabel(decision?.current_focus)),
    "",
  );
  const activityWhereLine = cardTaskTruthAnswer
    ? safeProductLine(cardTaskTruthAnswer.where_summary || "", "")
    : safeProductLine(
        noClearCurrentTask
          ? ""
          : decision?.answer?.where_label || activityRecap?.primary_where_summary || currentFocusLine,
        "",
      );
  const uncertaintyLine = productState?.uncertaintyLine || (
    targetLooksInternal
      ? "I saw the current focus, but I don't have a reliable return target yet."
      : safeProductLine(
          handoff?.user_visible_uncertainty ||
            handoff?.missing_evidence_line ||
            presentation?.missingEvidenceSummary ||
            "",
          "",
        )
  );
  const publicProjection = cardTaskTruthAnswer && hasVisibleSemanticAnswer
    ? buildContinuePublicProjection(cardTaskTruthAnswer, canOpenResumeTarget)
    : null;
  const publicHeadline = publicProjection?.headline || workstreamLine;
  const publicMemoryLine = publicProjection
    ? publicProjection.memoryLine
    : !noClearCurrentTask && lastStateLine
      ? sentenceCase(lastStateLine)
      : null;
  const defaultOpenActionLabel = publicProjection?.openActionLabel
    || actionState?.label
    || "Continue here";
  const continuationField = cardTaskTruthAnswer
    ? buildContinueContinuationFieldProjection(
        cardTaskTruthAnswer,
        canOpenResumeTarget,
        humanTargetLabel(resumeTarget) || null,
      )
    : {
        checkpoint: safeProductLine(
          activityRecap?.last_meaningful_state || decision?.answer?.where_you_left_off || lastStateLine,
          "",
        ) || null,
        continuation: safeProductLine(
          activityRecap?.unfinished_state
            || productState?.nextActionLine
            || decision?.answer?.next
            || "",
          "No unfinished step was clearly captured.",
        ),
        checkpointSurface: null,
        continuationSurface: null,
        locationLabel: safeProductLine(
          humanTargetLabel(resumeTarget) || activityWhereLine || currentFocusLine,
          "",
        ) || null,
        targetStatus: canOpenResumeTarget
          ? null
          : activityWhereLine || currentFocusLine
            ? "Exact place not captured"
            : "Return location not captured",
        openActionLabel: canOpenResumeTarget ? defaultOpenActionLabel : null,
        recentContext: [],
      };
  const publicActionLabel = busyAction === "open_continue_target"
    ? "Opening"
    : continuationField.openActionLabel || defaultOpenActionLabel;
  const recentContext = continuationField.recentContext;
  const evidencePreviewFrameId = cardTaskTruthAnswer?.evidence_preview?.frame_id || null;
  const visualCueMatchesAnswer = Boolean(
    contextOpen
    && evidencePreviewFrameId
    && visualCueVisible
    && selectedFrame
    && String(selectedFrame.id) === String(evidencePreviewFrameId)
    && imageData,
  );
  const hasSupportedContinuationDetails = hasVisibleTaskTruthContinuationDetails(
    cardTaskTruthAnswer,
  );
  const showCoreContinuation = !noClearCurrentTask || hasSupportedContinuationDetails;
  const hasContextDisclosure = showCoreContinuation && (
    recentContext.length > 0 || Boolean(evidencePreviewFrameId)
  );
  const hasDetails = Boolean(
    showCoreContinuation
    || visualCueMatchesAnswer
    || hasContextDisclosure
    || taskTruthAlternatives.length
    || feedbackStatus
    || visibleAlternatives.length,
  );

  useEffect(() => {
    setAlternativesOpen(false);
    setContextOpen(false);
  }, [decision?.decision_id]);

  useGSAP(() => {
    if (!decision || window.matchMedia("(prefers-reduced-motion: reduce)").matches) return;
    const elements = answerRef.current?.querySelectorAll(
      ":scope > .answer-shell, :scope > .continuation-field",
    );
    if (!elements?.length) return;
    gsap.fromTo(
      elements,
      { opacity: 0, transform: "translateY(7px)" },
      {
        opacity: 1,
        transform: "translateY(0px)",
        duration: 0.2,
        stagger: 0.04,
        ease: "power3.out",
        clearProps: "opacity,transform",
      },
    );
  }, { scope: answerRef, dependencies: [decision?.decision_id] });

  const recordAndClose = (
    feedbackKind: string,
    scope?: {
      taskSnapshotId?: string | null;
      taskSnapshotRevision?: number | null;
      affectedTaskField?: string | null;
      taskHypothesisId?: string | null;
    },
  ) => {
    onRecordFeedback(feedbackKind, scope);
  };
  const toggleContext = () => {
    const nextOpen = !contextOpen;
    setContextOpen(nextOpen);
    if (nextOpen && evidencePreviewFrameId) {
      onRevealVisualCue(evidencePreviewFrameId);
    }
  };
  const emptySubcopy = !hasEvidence && !running
    ? "Smalltalk will quietly keep enough context to help you continue later."
    : running && !hasEvidence
      ? "Keep working. Smalltalk will surface a continuation when there is enough evidence."
      : "Smalltalk can answer from local evidence without stopping memory first.";
  const showMemoryStartState = !running && (
    !hasEvidence
    || !decision
    || noClearCurrentTask
  );

  if (showMemoryStartState) {
    return (
      <section className="continue-card continuation-answer empty memory-start" aria-label="Start local memory">
        <div className="answer-shell">
          <MosaicLeafBackground />
          <div className="answer-hero memory-start-hero">
            <h2>Keep your place</h2>
            <span>
              Smalltalk quietly remembers your work, so you can pick it back up later.
            </span>
            <button
              className="memory-start-button"
              type="button"
              disabled={busyAction !== null}
              aria-busy={busyAction === "start_capture"}
              onClick={onStartMemory}
            >
              {busyAction === "start_capture" ? "Starting memory…" : "Start memory"}
            </button>
          </div>
        </div>
        <ContinuationDetailsPlaceholder hasEvidence={hasEvidence} running={running} />
      </section>
    );
  }

  if (busyAction === "get_continue_decision") {
    return (
      <section className="continue-card continuation-answer generating" aria-label="Continue decision">
        <div className="answer-shell">
          <MosaicLeafBackground />
          <ContinueGeneratingIndicator />
        </div>
        <ContinuationDetailsPlaceholder generating hasEvidence={hasEvidence} running={running} />
      </section>
    );
  }

  if (!decision) {
    return (
      <section className="continue-card continuation-answer empty" aria-label="Continue decision">
        <div className="answer-shell">
          <MosaicLeafBackground />
          <div className="answer-hero">
            <p>{hasEvidence ? "Ready to find your continuation" : running ? "Local memory is on" : "Turn on local memory once"}</p>
            <h2>{primaryMessage}</h2>
            <span>{emptySubcopy}</span>
          </div>
        </div>
        <ContinuationDetailsPlaceholder hasEvidence={hasEvidence} running={running} />
      </section>
    );
  }

  return (
    <section
      className={`continue-card continuation-answer ${lowConfidence || targetLooksInternal ? "low-confidence" : ""}`}
      aria-label="Continue decision"
      ref={answerRef}
    >
      <div className="answer-shell">
        <MosaicLeafBackground />
        <div className="answer-hero answer-hero-public" aria-live="polite">
          {!publicProjection ? <p>{noClearCurrentTask ? "Continue" : "You were"}</p> : null}
          <h2>{publicHeadline}</h2>
          {publicMemoryLine ? <p className="answer-memory-line">{publicMemoryLine}</p> : null}
        </div>

      </div>

      {!showCoreContinuation ? (
        <ContinuationDetailsPlaceholder
          hasEvidence={hasEvidence}
          running={running}
          checkpointCopy={
            productState?.lastStateLine
              || taskInferenceFailure?.detail
              || NO_CLEAR_CURRENT_TASK_COPY.lastStateLine
          }
          continuationCopy={
            productState?.nextActionLine
              || NO_CLEAR_CURRENT_TASK_COPY.nextActionLine
          }
        />
      ) : null}

      {hasDetails ? (
        <section className="continuation-field" aria-label="Continuation">
          {showCoreContinuation ? (
            <div className="continuation-core">
              {continuationField.checkpoint ? (
                <section className="continuation-copy-block" aria-labelledby="last-checkpoint-heading">
                  <div className="continuation-copy-heading">
                    <h3 id="last-checkpoint-heading">Last checkpoint</h3>
                    {continuationField.checkpointSurface ? (
                      <ContinuationSurfaceBadge surface={continuationField.checkpointSurface} />
                    ) : null}
                  </div>
                  <p>{continuationField.checkpoint}</p>
                </section>
              ) : null}
              <section className="continuation-copy-block continuation-next" aria-labelledby="continue-from-here-heading">
                <div className="continuation-copy-heading">
                  <h3 id="continue-from-here-heading">Continue from here</h3>
                  {continuationField.continuationSurface ? (
                    <ContinuationSurfaceBadge surface={continuationField.continuationSurface} />
                  ) : null}
                </div>
                <p>{continuationField.continuation}</p>
                {canOpenResumeTarget ? (
                  <button
                    className="primary-button continuation-open-action"
                    type="button"
                    disabled={busyAction !== null}
                    aria-busy={busyAction === "open_continue_target"}
                    onClick={onOpenTarget}
                  >
                    {publicActionLabel}
                  </button>
                ) : null}
              </section>

              {openResult ? (
                <div className="continue-open-result" role="status">
                  <strong>Open target</strong>
                  <span>{presentOpenResult(openResult)}</span>
                </div>
              ) : null}

              {hasContextDisclosure ? (
                <section className={`continuation-context ${contextOpen ? "open" : ""}`} aria-label="Context trail">
                  <div className="continuation-context-heading">
                    <h3>Context trail</h3>
                    {evidencePreviewFrameId ? (
                      <button
                        className="continuation-context-trigger"
                        type="button"
                        aria-expanded={contextOpen}
                        aria-controls="continuation-context-content"
                        onClick={toggleContext}
                      >
                        <span>{contextOpen ? "Hide visual cue" : "Show visual cue"}</span>
                        <CaretDown size={14} weight="bold" aria-hidden="true" />
                      </button>
                    ) : null}
                  </div>

                  {recentContext.length > 0 ? (
                    <ol
                      className="continuation-context-rail"
                      aria-label="Work surfaces in chronological order"
                      tabIndex={0}
                    >
                      {recentContext.map((visit) => (
                        <RecentContextVisit
                          key={`${visit.sequence_index}:${visit.first_observed_at_ms}`}
                          visit={visit}
                        />
                      ))}
                    </ol>
                  ) : null}

                  {contextOpen ? (
                    <div className="continuation-context-content" id="continuation-context-content">
                      {visualCueMatchesAnswer && selectedFrame && imageData ? (
                        <section className="context-visual-cue" aria-label="Visual cue">
                          <div>
                            <span>Visual cue</span>
                            <strong>{evidenceAnchorLabel(selectedFrame)}</strong>
                          </div>
                          <div className="context-visual-image" style={stageStyle(selectedFrame)}>
                            <img src={imageData} alt={`Visual cue from ${evidenceAnchorLabel(selectedFrame)}`} />
                          </div>
                        </section>
                      ) : evidencePreviewFrameId ? (
                        <p className="continuation-visual-loading" role="status">Loading the last meaningful screen…</p>
                      ) : null}
                    </div>
                  ) : null}
                </section>
              ) : null}

              {productState?.olderContextLine ? (
                <p className="continuation-note">{productState.olderContextLine}</p>
              ) : null}

              {!publicProjection && (lowConfidence || uncertaintyLine) ? (
                <p className="continuation-note">
                  {uncertaintyLine || "Evidence is thin, so this is the best available local recommendation."}
                </p>
              ) : null}
            </div>
          ) : null}

          {taskTruthAlternatives.length > 0 ? (
            <div className="alternative-list" aria-label="Possible task interpretations">
              <div className="alternative-heading">
                <strong>Another task interpretation is similarly supported</strong>
                <span>Choose one</span>
              </div>
              {taskTruthAlternatives.slice(0, 2).map((hypothesis) => (
                <div className="alternative-row" key={hypothesis.hypothesis_id}>
                  <div>
                    <strong>{taskTruthTaskProductLine(hypothesis.task_summary)}</strong>
                  </div>
                  <div className="answer-actions">
                    <button
                      className="secondary-button"
                      type="button"
                      disabled={busyAction !== null}
                      onClick={() => recordAndClose("corrected", {
                        taskSnapshotId: cardTaskTruthAnswer?.snapshot_id,
                        taskSnapshotRevision: cardTaskTruthAnswer?.snapshot_revision,
                        affectedTaskField: "hypothesis",
                        taskHypothesisId: hypothesis.hypothesis_id,
                      })}
                    >
                      This one
                    </button>
                    <button
                      className="text-button"
                      type="button"
                      disabled={busyAction !== null}
                      onClick={() => recordAndClose("rejected", {
                        taskSnapshotId: cardTaskTruthAnswer?.snapshot_id,
                        taskSnapshotRevision: cardTaskTruthAnswer?.snapshot_revision,
                        affectedTaskField: "hypothesis",
                        taskHypothesisId: hypothesis.hypothesis_id,
                      })}
                    >
                      Not right
                    </button>
                  </div>
                </div>
              ))}
            </div>
          ) : null}

          {feedbackStatus ? (
            <p className="correction-feedback" role="status">{feedbackStatus}</p>
          ) : null}

          {visibleAlternatives.length > 0 ? (
            <div className="alternative-list" aria-label="Alternative continuations">
              <div className="alternative-heading">
                <strong>{isThinCurrentWork ? "Older possible locations" : "Alternatives"}</strong>
                <span>{visibleAlternatives.length}</span>
              </div>
              {visibleAlternatives.map((candidate) => (
                <div className="alternative-row" key={candidate.candidate_id}>
                  <div>
                    <strong>{presentAlternativeCandidate(candidate)}</strong>
                    <span>
                      {[
                        productizeInternalLabel(candidate.reason) || candidate.confidence_label || "Possible continuation",
                        isThinCurrentWork ? "Older than your latest current work." : "",
                      ].filter(Boolean).join(" ")}
                    </span>
                  </div>
                  <button
                    className="secondary-button"
                    type="button"
                    disabled={busyAction !== null}
                    onClick={() => onUseAlternative(candidate)}
                  >
                    Use this
                  </button>
                </div>
              ))}
            </div>
          ) : null}

        </section>
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
          Workstream detail appears after local memory has enough evidence to build evidence anchors, artifact roles, and return targets.
        </p>
      </section>
    );
  }

  const primaryCandidate =
    detail.candidates.find(
      (candidate) =>
        candidate.candidate_id === detail.latest_decision?.selected_candidate_id &&
        !candidateIsSupportEvidenceOnly(candidate),
    ) || detail.candidates.find((candidate) => !candidateIsSupportEvidenceOnly(candidate)) || null;
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
              "No return target"}
          </strong>
          <small>
            {[
              primaryCandidate ? sentenceCase(primaryCandidate.candidate_kind) : null,
              primaryCandidate?.target_openability,
              primaryCandidate ? confidenceLabel(primaryCandidate.score) : null,
            ].filter(Boolean).join(" / ") || "Target details unavailable."}
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
          {detail.candidates.slice(0, 6).map((candidate) => {
            const supportOnly = candidateIsSupportEvidenceOnly(candidate);
            return (
              <div className="candidate-row" key={candidate.candidate_id}>
                <div>
                  <strong>{candidate.target_title || candidate.target_artifact_id || sentenceCase(candidate.candidate_kind)}</strong>
                  <span>
                    {supportOnly
                      ? productizeInternalLabel(candidate.why_not_primary || candidate.branch_promotion_reason || candidate.reason) ||
                        "Supporting evidence, not a continuation target."
                      : candidate.reason || "No local reason returned."}
                  </span>
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
                  disabled={busyAction !== null || supportOnly}
                  onClick={() => onContinueFromCandidate(candidate)}
                >
                  {supportOnly ? "Evidence only" : "Continue from this"}
                </button>
              </div>
            );
          })}
          {detail.candidates.length === 0 ? (
            <div className="workstream-empty">
              <strong>No return targets yet</strong>
              <span>Refresh Continue to generate continuation targets.</span>
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
                  Inspect evidence
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
          <p>Recent return targets and supporting work.</p>
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
  tone?: "quiet" | "good" | "warn" | "bad";
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
          <b>{productizeEvidenceTrigger(frame.capture_trigger)}</b>
        </span>
        <strong>{frameTitle(frame)}</strong>
        <small>{cleanSnippet(snippet || frame.full_text)}</small>
        <span className="badge-row">
          <EvidenceBadge label="screen" ok={Boolean(frame.snapshot_path)} />
          <EvidenceBadge label={productizeCaptureProvider(frame.capture_provider)} ok={frame.capture_provider === "screen_capture_kit"} />
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
      <strong>{hasFrames && hasQuery ? "No matching evidence" : "No evidence yet"}</strong>
      <span>
        {hasFrames && hasQuery
          ? "Clear the search or use a broader term to inspect existing evidence."
          : "Turn on local memory to collect inspectable evidence, text sources, and missing-signal checks."}
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
        <strong>No evidence anchor selected</strong>
        <span>Select an evidence anchor to inspect stored evidence.</span>
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
            <strong>No raw event row linked</strong>
            <span>Manual evidence updates may not have event provenance.</span>
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
        {detail?.weak_surface_classification && (
          <div className="drawer-row">
            <strong>{sentenceCase(detail.weak_surface_classification.domain)}</strong>
            <span>
              {sentenceCase(detail.weak_surface_classification.enrichment_need)}
              {detail.weak_surface_classification.adapter_key ? ` via ${detail.weak_surface_classification.adapter_key}` : ""}
            </span>
            <small>
              {confidenceLabel(detail.weak_surface_classification.confidence)}; {detail.weak_surface_classification.reasons.slice(0, 4).join(", ") || "no reasons"}
            </small>
          </div>
        )}
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
          <dt>Evidence trigger</dt>
          <dd>{frame.capture_trigger_id || "No trigger id"}</dd>
        </div>
        <div>
          <dt>Memory window</dt>
          <dd>{frame.session_id || "No memory-window id"}</dd>
        </div>
        <div>
          <dt>App bundle</dt>
          <dd>{frame.app_bundle_id || "Unknown"}</dd>
        </div>
        <div>
          <dt>Evidence provider</dt>
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

function deriveMemoryProductStatus(
  status: CaptureStatus,
  hasEvidence: boolean,
  busyAction: string | null,
  privateOrExcluded: boolean,
): MemoryProductStatus {
  if (
    busyAction === "delete_all_frames" ||
    busyAction === "delete_recent_captures" ||
    busyAction === "dev_reset_local_memory"
  ) {
    return "deleting";
  }
  if (busyAction === "start_capture") {
    return "starting";
  }
  if (status.last_error) {
    return isPermissionMemoryError(status.last_error) ? "needs_permission" : "needs_attention";
  }
  if (privateOrExcluded) {
    return "private_or_excluded";
  }
  if (status.running) {
    return "on";
  }
  if (hasEvidence) {
    return "paused_with_evidence";
  }
  return "off";
}

function getMemoryProductCopy(
  status: MemoryProductStatus,
  errorMessage?: string | null,
): { label: string; detail: string } {
  if (!errorMessage) return memoryProductCopy[status];
  return {
    label: memoryProductCopy[status].label,
    detail: productizeMemoryError(errorMessage),
  };
}

function isPrivateMemorySurface(status: CaptureStatus) {
  const latestFrameAt = status.latest_frame?.captured_at || 0;
  const latestPrivacy = status.latest_frame?.privacy_status || "";
  if (isPrivatePrivacyLabel(latestPrivacy)) return true;
  return Boolean(
    status.running &&
      status.last_skipped_at &&
      status.last_skipped_at >= latestFrameAt &&
      status.runtime_diagnostics.heavy_captures_skipped_privacy > 0,
  );
}

function isPrivatePrivacyLabel(value?: string | null) {
  const label = normalizeToken(value);
  return Boolean(label && !["normal", "ok", "allowed", "manual_evidence", "high_value", "decision_anchor"].includes(label));
}

function isPermissionMemoryError(value: string) {
  const error = value.toLowerCase();
  const accessibilityPermissionError = error.includes("accessibility") && (
    error.includes("permission") ||
    error.includes("not authorized") ||
    error.includes("not trusted") ||
    error.includes("denied")
  );
  return error.includes("permission") ||
    error.includes("screen access") ||
    accessibilityPermissionError ||
    error.includes("not authorized") ||
    error.includes("operation not permitted");
}

function productizeMemoryError(value: string) {
  const error = value.toLowerCase();
  if (
    error.includes("no such column") ||
    error.includes("no such table") ||
    error.includes("database schema") ||
    error.includes("malformed")
  ) {
    return "Local memory needs a database update. Restart Smalltalk to finish it.";
  }
  if (error.includes("screen") && (error.includes("permission") || error.includes("access"))) {
    return "Screen access is needed for local memory.";
  }
  if (error.includes("accessibility") || error.includes("ax")) {
    return "Accessibility access is needed to understand app context.";
  }
  if (error.includes("ocr") || error.includes("vision")) {
    return "Some visible text could not be read.";
  }
  if (error.includes("database") && (error.includes("locked") || error.includes("busy"))) {
    return "Local memory is temporarily unavailable.";
  }
  if (error.includes("no space") || error.includes("storage") || error.includes("disk")) {
    return "Local memory needs cleanup.";
  }
  if (error.includes("privacy") || error.includes("excluded") || error.includes("never_send_to_ai")) {
    return "This surface was skipped for privacy.";
  }
  if (error.includes("capture")) {
    return value
      .replace(/Capture/g, "Memory")
      .replace(/capture/g, "memory");
  }
  return "Local memory needs attention. Try again in a moment.";
}

function sitePatternFromUrl(value?: string | null) {
  if (!value) return "";
  try {
    const host = new URL(value).hostname.replace(/^www\./, "");
    return host.trim();
  } catch {
    return "";
  }
}

function siteLabelFromUrl(value?: string | null) {
  const pattern = sitePatternFromUrl(value);
  return pattern ? pattern : "";
}

function hasEnabledExclusion(rules: ExclusionRule[], ruleType: string, pattern: string) {
  const normalizedPattern = pattern.trim().toLowerCase();
  return rules.some((rule) => {
    if (!rule.enabled || rule.rule_type !== ruleType) return false;
    const candidates = rule.pattern
      .split("|")
      .map((part) => part.trim().toLowerCase())
      .filter(Boolean);
    return candidates.some((candidate) =>
      normalizedPattern.includes(candidate) || candidate.includes(normalizedPattern),
    );
  });
}

function formatExclusionRule(rule: ExclusionRule) {
  const typeLabels: Record<string, string> = {
    app_bundle: "App",
    url_regex: "Website",
    window_title_regex: "Window title",
    content_regex: "Content",
  };
  const type = typeLabels[rule.rule_type] || sentenceCase(rule.rule_type);
  return `${type}: ${rule.pattern}`;
}

function formatExclusionAction(action: string) {
  const labels: Record<string, string> = {
    skip_capture: "Not observed",
    store_redacted: "Stored with redaction",
    never_send_to_ai: "Never sent to AI",
  };
  return labels[action] || sentenceCase(action);
}

function dangerousActionCopy(action: DangerousAction) {
  if (action === "delete_recent") {
    return {
      title: "Delete recent memory?",
      body: "This removes local evidence stored in the last hour from this device. Older local evidence may remain.",
      confirmLabel: "Delete recent memory",
    };
  }
  if (action === "dev_reset") {
    return {
      title: "Developer reset?",
      body: "This clears local evidence, derived Continue rows, snapshots, generated debug exports, and diagnostics.",
      confirmLabel: "Reset for development",
    };
  }
  return {
    title: "Delete local memory?",
    body: "This removes stored evidence and Continue history from this device. It cannot be undone.",
    confirmLabel: "Delete local memory",
  };
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

type ContinueProductStateKind =
  | "openable_enriched"
  | "enriched_not_openable"
  | "thin_current_work"
  | "older_context_with_thin_current_work"
  | "no_clear_continuation";

type ContinueProductStateCopy = {
  kind: ContinueProductStateKind;
  heroLabel: string;
  headline: string;
  targetBlockLabel: string;
  targetLine: string;
  targetMeta: string;
  lastStateLine: string | null;
  nextActionLine: string;
  currentFocusLine: string;
  uncertaintyLine: string;
  missingEvidenceLines: string[];
  statusPills: string[];
  olderContextLine?: string;
};

function presentContinueDecision(decision: ContinueDecisionResult): ContinuePresentation {
  const target = decision.resume_work_target || decision.return_target || null;
  const recap = usableActivityRecap(decision.activity_recap);
  const answer = decision.answer;
  if (
    getContinueTaskResolutionStatus(decision) === "no_clear_current_task"
    && !hasSupportedWorkTruth(decision)
  ) {
    const reason = (decision.task_resolution_reason_codes || [])
      .map(productizeInternalLabel)
      .find(Boolean);
    return {
      workstreamTitle: NO_CLEAR_CURRENT_TASK_HEADLINE,
      currentFocus: "",
      currentActivity: "",
      returnTarget: NO_CLEAR_CURRENT_TASK_COPY.targetLine,
      targetMeta: NO_CLEAR_CURRENT_TASK_COPY.targetMeta,
      lastState: NO_CLEAR_CURRENT_TASK_COPY.lastStateLine,
      unresolvedState: "",
      nextAction: NO_CLEAR_CURRENT_TASK_COPY.nextActionLine,
      confidenceLabel: "None",
      confidenceSummary: "No exact current task is supported by the available evidence.",
      missingEvidenceSummary: reason || "More current task evidence is needed.",
      decisionReason: "Recent activity was captured, but it does not support one exact current task.",
    };
  }
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
  const workstreamTitle = selectPrimaryTaskHeadline(
    cleanHumanText(decision.work_truth?.activity_summary || answer?.what_you_were_doing),
    cleanHumanText(recap?.primary_work_summary),
    cleanHumanText(decision.selected_workstream?.title_candidate) || targetLabel,
    humanFocusLabel(decision.current_focus),
    decision.task_resolution_status,
  );
  const confidence = decision.confidence_label
    ? sentenceCase(decision.confidence_label)
    : confidenceLabel(decision.confidence);
  const confidenceSummary = decision.work_truth?.resolution_status === "activity_supported"
    ? "Broader goal not captured."
    : missingEvidence.length
    ? `${confidence}; ${missingEvidence[0]}`
    : `${confidence}; evidence is enough for a local recommendation.`;

  return {
    workstreamTitle,
    currentFocus: humanFocusLabel(decision.current_focus),
    currentActivity: cleanHumanText(recap?.primary_work_label)
      || productizeInternalLabel(decision.current_activity || ""),
    returnTarget: targetLabel || "Exact location unavailable",
    targetMeta: humanTargetMeta(target),
    lastState: cleanHumanText(recap?.last_meaningful_state)
      || lastAction
      || unresolvedState
      || "No meaningful prior state is clear yet.",
    unresolvedState: cleanHumanText(recap?.unfinished_state) || unresolvedState,
    nextAction: productizeInternalLabel(
      recap?.next_action_summary
        || decision.next_action
        || (target ? "Continue from the last meaningful state." : "Inspect the captured evidence before deciding what to do next."),
    ),
    confidenceLabel: confidence,
    confidenceSummary,
    missingEvidenceSummary: summarizeProductEvidence(missingEvidence),
    decisionReason: cleanHumanText(recap?.why_this_target || recap?.why_no_safe_target)
      || productizeCandidateKind(decision.candidate_kind)
      || unresolvedState
      || "Selected from local workstream evidence.",
  };
}

function buildTaskTruthProductStateCopy(
  answer: TaskTruthPublicAnswer,
  actionState: ContinueCardActionState,
  diagnostic?: TaskTruthInferenceDiagnostic | null,
): ContinueProductStateCopy {
  const unresolved = answer.task_resolution_status === "unresolved";
  const ambiguous = answer.task_resolution_status === "ambiguous";
  const target = answer.direct_return_target || null;
  const taskLine = taskTruthTaskProductLine(answer.task_summary || "");
  const stateLine = [
    taskTruthStateProductLine(answer.last_meaningful_progress || "", "progress"),
    taskTruthStateProductLine(answer.unfinished_state || "", "unfinished"),
  ]
    .filter(Boolean)
    .join(" ");
  const currentActivity = taskTruthTaskProductLine(
    answer.current_activity?.current_subtask
      || answer.current_subtask
      || answer.current_activity?.immediate_user_operation
      || answer.immediate_user_operation
      || answer.current_activity?.observed_surface
      || answer.observed_surface
      || "",
  );
  // A relationship such as `primary_work` is meaningless when no primary
  // task survived evidence admission. Keep the current surface factual, but
  // never let a stale role label turn it back into the task in product copy.
  const relationship = taskLine
    ? normalizeToken(
        answer.current_activity?.relationship_to_primary || answer.relationship_to_prior,
      )
    : "unrelated_or_unknown";
  const currentActivityLine = currentActivity
    ? (() => {
        const relationshipCopy: Record<string, string> = {
          continuation: "This continues the primary task.",
          supporting_research: "This is supporting research for the primary task.",
          verification: "This is verification work for the primary task.",
          temporary_detour: "This is a temporary detour from the primary task.",
          interruption: "This interrupted the primary task without completing it.",
          new_task: "This appears to be a separate new task.",
          return_to_prior_task: "This returns to the earlier primary task.",
          primary_work: "This is the primary work.",
          supporting_work: "This supports the primary work.",
          detour_or_unrelated: "This appears to be a detour or unrelated work.",
          unclear: "Its relationship to the primary work is unclear.",
          unrelated_or_unknown: "Its relationship to the earlier task is not clear.",
          unknown: "Its relationship to the earlier task is not clear.",
        };
        return `${currentActivity} ${relationshipCopy[relationship] || ""}`.trim();
      })()
    : "";
  const inspectCopy = inspectTargetCopy({
    taskKnown: !unresolved && Boolean(taskLine),
    evidencePreviewAvailable: Boolean(answer.evidence_preview),
    appFocusOnly: false,
  });

  if (unresolved) {
    const visibleSemantics = hasVisibleTaskTruthSemantics(answer);
    const failureStatus = normalizeToken(diagnostic?.status) === "success"
      ? answer.inference_status
      : diagnostic?.status || answer.inference_status;
    const failure = taskInferenceFailurePresentation(
      failureStatus,
      diagnostic?.verification_status,
      diagnostic?.origin,
      diagnostic?.image_count,
      diagnostic?.provider_attempt_count,
    );
    if (visibleSemantics) {
      return {
        kind: "enriched_not_openable",
        heroLabel: "Recent work",
        headline: taskLine || currentActivity || stateLine || "Recent work was understood",
        targetBlockLabel: inspectCopy.targetBlockLabel,
        targetLine: "The understood work is ready to inspect",
        targetMeta: "No exact page, conversation, or file was opened automatically.",
        lastStateLine: stateLine || null,
        nextActionLine: taskTruthStateProductLine(answer.next_action || answer.unfinished_state || "", "next"),
        currentFocusLine: currentActivityLine,
        uncertaintyLine:
          "The model answer is shown. The broader task or exact return target did not pass the stricter local check.",
        missingEvidenceLines: [],
        statusPills: [],
      };
    }
    return {
      kind: "no_clear_continuation",
      heroLabel: NO_CLEAR_CURRENT_TASK_COPY.heroLabel,
      headline: failure.headline,
      targetBlockLabel: inspectCopy.targetBlockLabel,
      targetLine: inspectCopy.targetLine,
      targetMeta: inspectCopy.targetMeta,
      lastStateLine: failure.detail,
      nextActionLine: "",
      // Observed surface is diagnostic evidence, not inferred task truth. Keep
      // it under “Why this answer?” whenever model-first inference is unresolved.
      currentFocusLine: "",
      uncertaintyLine: failure.kind === "insufficient_evidence"
        ? NO_CLEAR_CURRENT_TASK_COPY.uncertaintyLine
        : "No local semantic fallback was substituted.",
      missingEvidenceLines: [],
      statusPills: [],
    };
  }

  const targetOpenable = actionState.kind === "openable_return_target";
  return {
    kind: targetOpenable ? "openable_enriched" : "enriched_not_openable",
    heroLabel: "You were",
    headline: taskLine || currentActivity || stateLine || (ambiguous
      ? "Two tasks are similarly supported"
      : "The current task is supported, but its precise summary is unavailable"),
    targetBlockLabel: targetOpenable ? "Continue at" : inspectCopy.targetBlockLabel,
    targetLine: targetOpenable
      ? humanTargetLabel(target) || "Verified return location"
      : inspectCopy.targetLine,
    targetMeta: targetOpenable ? humanTargetMeta(target) : inspectCopy.targetMeta,
    lastStateLine: stateLine || null,
    nextActionLine: taskTruthStateProductLine(answer.next_action || "", "next"),
    currentFocusLine: currentActivityLine,
    uncertaintyLine: ambiguous
      ? "Two task hypotheses remain close. Choose the one that matches your work."
      : targetOpenable
        ? ""
        : normalizeToken(answer.inference_status).includes("verification_limits")
          ? "The model answer is shown, but Smalltalk could not verify it enough to open a location safely."
          : "The model answer is shown, but no strict direct return location is available.",
    missingEvidenceLines: [],
    statusPills: [],
  };
}

function buildContinueProductStateCopy(
  decision: ContinueDecisionResult,
  actionState: ContinueCardActionState,
  presentation: ContinuePresentation,
  primaryMessage: string,
): ContinueProductStateCopy {
  const target = decision.resume_work_target || decision.return_target || null;
  const focus = decision.current_focus || null;
  const activeWork = decision.active_current_work_unresolved || null;
  const recap = usableActivityRecap(decision.activity_recap);
  const answer = decision.answer;
  const targetLooksInternal = isInternalFacingText(presentation.returnTarget);
  const currentFocusLine = humanEnrichedFocusLabel(focus, activeWork);
  const missingEvidenceLines = productMissingEvidenceLines(decision).slice(0, 4);
  if (
    getContinueTaskResolutionStatus(decision) === "no_clear_current_task"
    && !hasSupportedWorkTruth(decision)
  ) {
    return {
      kind: "no_clear_continuation",
      heroLabel: NO_CLEAR_CURRENT_TASK_COPY.heroLabel,
      headline: NO_CLEAR_CURRENT_TASK_HEADLINE,
      targetBlockLabel: NO_CLEAR_CURRENT_TASK_COPY.targetBlockLabel,
      targetLine: NO_CLEAR_CURRENT_TASK_COPY.targetLine,
      targetMeta: NO_CLEAR_CURRENT_TASK_COPY.targetMeta,
      lastStateLine: NO_CLEAR_CURRENT_TASK_COPY.lastStateLine,
      nextActionLine: NO_CLEAR_CURRENT_TASK_COPY.nextActionLine,
      currentFocusLine: "",
      uncertaintyLine: NO_CLEAR_CURRENT_TASK_COPY.uncertaintyLine,
      missingEvidenceLines,
      statusPills: ["No clear task", "Evidence available"],
    };
  }
  if (decision.work_truth?.resolution_status === "activity_supported") {
    const targetOpenable = actionState.kind === "openable_return_target";
    const activityLine = safeProductLine(
      decision.work_truth.activity_summary || presentation.workstreamTitle,
      "Recent work",
    );
    return {
      kind: targetOpenable ? "openable_enriched" : "enriched_not_openable",
      heroLabel: "You were",
      headline: activityLine,
      targetBlockLabel: targetOpenable ? "Continue at" : "Observed at",
      targetLine: targetOpenable
        ? safeProductLine(presentation.returnTarget, "Verified return location")
        : safeProductLine(decision.work_truth.where_summary || presentation.returnTarget, "Exact location unavailable"),
      targetMeta: presentation.targetMeta,
      lastStateLine: activityLine,
      nextActionLine: "",
      currentFocusLine: safeProductLine(decision.work_truth.app_name || "", ""),
      uncertaintyLine: "Broader goal wasn’t captured.",
      missingEvidenceLines,
      statusPills: [sentenceCase(decision.work_truth.activity_kind), "Observed activity"],
    };
  }
  const safeLastState = safeProductLine(
    answer?.where_you_left_off
      || recap?.last_meaningful_state
      || recap?.unfinished_state
      || decision.handoff?.last_state_line
      || presentation.lastState
      || "",
    "No last meaningful state is clear yet.",
  );
  const safeNextAction = safeProductLine(
    answer?.next || recap?.next_action_summary || decision.handoff?.next_action || presentation.nextAction || "",
    "Inspect the latest evidence before deciding what to do next.",
  );
  const openability = normalizeToken(focus?.openability || target?.openability);
  const quality = surfaceQualityToken(focus?.evidence_quality, focus?.identity_confidence, activeWork);
  const hasEnrichedSurface = hasEnrichedCurrentSurface(focus, activeWork);
  const hasUsefulActivityMemory = Boolean(
    recap &&
      (
        safeProductLine(recap.primary_work_summary || "", "") ||
        safeProductLine(recap.primary_where_summary || "", "") ||
        safeProductLine(recap.last_meaningful_state || recap.unfinished_state || "", "") ||
        recap.recent_detours.length ||
        recap.supporting_context.length
      ),
  );
  const activityHeadline = safeProductLine(
    selectPrimaryTaskHeadline(
      answer?.what_you_were_doing,
      recap?.primary_work_summary,
      presentation.workstreamTitle,
      currentFocusLine || primaryMessage,
      decision.task_resolution_status,
    ),
    primaryMessage,
  );
  const inspectCopy = inspectTargetCopy({
    taskKnown: Boolean(answer?.what_you_were_doing) || decision.target_truth?.state === "task_known_target_unknown",
    evidencePreviewAvailable: Boolean(decision.evidence_preview),
    appFocusOnly: openability === "app_focus_only",
    targetNote: answer?.target_note,
  });
  const hasOlderContext =
    Boolean(target && actionState.kind !== "openable_return_target") &&
    (
      Boolean(activeWork) ||
      continueDecisionEvidenceNotes(decision).some((note) =>
        normalizeToken(note).includes("newer_current_focus"),
      )
    );

  if (actionState.kind === "openable_return_target") {
    const targetLine = targetLooksInternal
      ? "Openable return target"
      : safeProductLine(presentation.returnTarget, "Openable return target");
    return {
      kind: "openable_enriched",
      heroLabel: recap?.primary_work_summary ? "What you were doing" : "Current focus",
      headline: activityHeadline || currentFocusLine,
      targetBlockLabel: "Return target",
      targetLine,
      targetMeta: safeProductLine(answer?.target_note || humanTargetMeta(target), humanTargetMeta(target)),
      lastStateLine: safeLastState,
      nextActionLine: safeProductLine(
        answer?.next || recap?.next_action_summary || decision.handoff?.next_action || presentation.nextAction || "",
        "Open the target and continue from the last meaningful state.",
      ),
      currentFocusLine,
      uncertaintyLine: presentation.missingEvidenceSummary || "",
      missingEvidenceLines,
      statusPills: continueStatusPills({ quality, openability, actionState, activeWork, hasEnrichedSurface }),
    };
  }

  if (hasOlderContext) {
    return {
      kind: "older_context_with_thin_current_work",
      heroLabel: recap?.primary_work_summary ? "What you were doing" : "Current focus",
      headline: activityHeadline || `${currentFocusLine || "Current work"} - current work is visible but thin`,
      targetBlockLabel: "Older context",
      targetLine: humanTargetLabel(target) || "Older possible location",
      targetMeta:
        "There is an older target that may be related, but newer current work makes it unsafe as the main action.",
      lastStateLine: safeLastState,
      nextActionLine: safeNextAction,
      currentFocusLine,
      uncertaintyLine:
        missingEvidenceLines[0] ||
        "Smalltalk saw newer current work, but the exact return target is not grounded.",
      missingEvidenceLines,
      statusPills: continueStatusPills({ quality, openability, actionState, activeWork, hasEnrichedSurface }),
      olderContextLine:
        "Older targets stay secondary until local evidence proves they are still the right continuation.",
    };
  }

  if (actionState.kind === "thin_current_work") {
    return {
      kind: "thin_current_work",
      heroLabel: recap?.primary_work_summary ? "What you were doing" : "Current focus",
      headline: activityHeadline || currentFocusLine || activeWork?.app_name || "Current work is visible but thin",
      targetBlockLabel: inspectCopy.targetBlockLabel,
      targetLine: inspectCopy.targetLine,
      targetMeta: safeProductLine(inspectCopy.targetMeta, "No direct page or file locator is available."),
      lastStateLine: safeLastState,
      nextActionLine: safeNextAction,
      currentFocusLine,
      uncertaintyLine:
        missingEvidenceLines[0] ||
        "Evidence is thin, so Smalltalk will not show a fake confident continuation target.",
      missingEvidenceLines,
      statusPills: continueStatusPills({ quality, openability, actionState, activeWork, hasEnrichedSurface }),
    };
  }

  if (hasEnrichedSurface && ["strong", "medium"].includes(quality)) {
    return {
      kind: "enriched_not_openable",
      heroLabel: recap?.primary_work_summary ? "What you were doing" : "Current focus",
      headline: activityHeadline || currentFocusLine || safeProductLine(presentation.currentFocus, "Current work"),
      targetBlockLabel: inspectCopy.targetBlockLabel,
      targetLine: inspectCopy.targetLine,
      targetMeta: safeProductLine(inspectCopy.targetMeta, "No direct page or file locator is available."),
      lastStateLine: safeLastState,
      nextActionLine: safeNextAction,
      currentFocusLine,
      uncertaintyLine:
        missingEvidenceLines[0] ||
        "The current work is enriched, but the opener does not have a safe exact target.",
      missingEvidenceLines,
      statusPills: continueStatusPills({ quality, openability, actionState, activeWork, hasEnrichedSurface }),
    };
  }

  return {
    kind: "no_clear_continuation",
    heroLabel: hasUsefulActivityMemory ? "What you were doing" : "Not enough evidence yet",
    headline: hasUsefulActivityMemory
      ? activityHeadline
      : "There is not enough activity evidence yet",
    targetBlockLabel: inspectCopy.targetBlockLabel,
    targetLine: inspectCopy.targetLine,
    targetMeta: safeProductLine(inspectCopy.targetMeta, "Exact location unavailable."),
    lastStateLine: safeLastState,
    nextActionLine: safeNextAction,
    currentFocusLine,
    uncertaintyLine:
      missingEvidenceLines[0] ||
      "Use more local evidence before selecting a continuation target.",
    missingEvidenceLines,
    statusPills: continueStatusPills({ quality, openability, actionState, activeWork, hasEnrichedSurface }),
  };
}

function hasEnrichedCurrentSurface(
  focus?: ContinueFocusSummary | null,
  activeWork?: ActiveCurrentWorkUnresolved | null,
) {
  return Boolean(
    focus?.domain ||
      focus?.display_title ||
      focus?.activity_state ||
      focus?.task_state ||
      focus?.evidence_quality ||
      focus?.openability ||
      focus?.snapshot_id ||
      (focus?.missing_fields && focus.missing_fields.length > 0) ||
      activeWork,
  );
}

function surfaceQualityToken(
  evidenceQuality?: string | null,
  identityConfidence?: number | null,
  activeWork?: ActiveCurrentWorkUnresolved | null,
) {
  const quality = normalizeToken(evidenceQuality || activeWork?.evidence_quality);
  if (["strong", "medium", "thin", "unknown"].includes(quality)) return quality;
  const confidence =
    typeof identityConfidence === "number"
      ? identityConfidence
      : typeof activeWork?.identity_confidence === "number"
        ? activeWork.identity_confidence
        : null;
  if (typeof confidence === "number") {
    if (confidence >= 0.78) return "strong";
    if (confidence >= 0.55) return "medium";
    if (confidence > 0) return "thin";
  }
  return "unknown";
}

function continueStatusPills({
  quality,
  openability,
  actionState,
  activeWork,
  hasEnrichedSurface,
}: {
  quality: string;
  openability: string;
  actionState: ContinueCardActionState;
  activeWork?: ActiveCurrentWorkUnresolved | null;
  hasEnrichedSurface: boolean;
}) {
  const pills = new Set<string>();
  if (activeWork || hasEnrichedSurface) pills.add("Fresh current work");
  if (quality === "thin" || actionState.kind === "thin_current_work") pills.add("Thin evidence");
  if (actionState.kind !== "openable_return_target") pills.add("Exact target missing");
  if (openability === "app_focus_only") pills.add("App focus only");
  if (openability === "frame_fallback") pills.add("Evidence preview only");
  if (quality === "strong" || quality === "medium") pills.add(`${sentenceCase(quality)} evidence`);
  return [...pills].slice(0, 4);
}

type HumanReadableTarget = {
  title?: string | null;
  document_path?: string | null;
  browser_url?: string | null;
  artifact_kind?: string | null;
  openability?: string | null;
};

function humanTargetLabel(target?: HumanReadableTarget | null) {
  if (!target) return "";
  return cleanHumanText(target.title)
    || pathBasename(target.document_path || target.browser_url)
    || productizeArtifactKind(target.artifact_kind)
    || "";
}

function humanTargetMeta(target?: HumanReadableTarget | null) {
  if (!target) return "I know the task separately from whether I can reopen its exact location.";
  const parts = [
    productizeArtifactKind(target.artifact_kind),
    productizeOpenability(target.openability),
  ].filter(Boolean);
  return parts.join(" / ") || "I don't have a reliable app or page target for this yet.";
}

function isDirectResumeTargetOpenable(target?: ContinueReturnTarget | null) {
  return isDirectPresentationTargetOpenable(target);
}

function continueEvidencePreviewFrameId(decision: ContinueDecisionResult) {
  const taskTruthAnswer = authoritativeTaskTruthAnswer(decision);
  return taskTruthAnswer?.evidence_preview?.frame_id
    || decision.evidence_anchors.frame_ids[0]
    || null;
}

function getContinueTaskResolutionStatus(
  decision: ContinueDecisionResult,
): ContinueTaskResolutionStatus {
  const explicit = normalizeTaskResolutionStatus(decision.task_resolution_status);
  if (explicit !== "unknown") return explicit;
  if (
    decision.continue_output_mode === "no_clear_continuation" ||
    decision.target_truth?.state === "no_clear_task"
  ) {
    return "no_clear_current_task";
  }
  return decision.current_task_turn ? "resolved_current_task" : "unknown";
}

function hasSupportedWorkTruth(decision: ContinueDecisionResult) {
  return decision.work_truth?.resolution_status === "task_supported"
    || decision.work_truth?.resolution_status === "activity_supported";
}

function getContinueCardActionState(decision: ContinueDecisionResult): ContinueCardActionState {
  const authoritativeAction = authoritativeTaskTruthActionState(decision);
  if (authoritativeAction) return authoritativeAction;
  const target = decision.resume_work_target || decision.return_target || null;
  const hasOpenableReturnTarget = isDirectResumeTargetOpenable(target);
  const evidenceNotes = continueDecisionEvidenceNotes(decision);
  const unresolvedCurrentWork = decision.active_current_work_unresolved;
  const hasThinCurrentWork =
    decision.target_truth?.state === "task_known_target_unknown" ||
    decision.target_truth?.state === "activity_known_target_unknown" ||
    decision.target_truth?.state === "thin_task_seen" ||
    Boolean(unresolvedCurrentWork && !unresolvedCurrentWork.has_openable_target) ||
    evidenceNotes.includes("stale_return_target_suppressed:newer_current_focus") ||
    (
      normalizeToken(decision.candidate_kind) === "continue_current_work" &&
      !hasOpenableReturnTarget
    ) ||
    evidenceNotes.includes("thin_evidence:no_human_return_target");

  return getContinuePresentationActionState({
    decisionId: decision.decision_id,
    outputMode: decision.continue_output_mode,
    taskResolutionStatus: getContinueTaskResolutionStatus(decision),
    workResolutionStatus: decision.work_truth?.resolution_status,
    target,
    targetTruthState: decision.target_truth?.state,
    directTargetAllowed: Boolean(decision.direct_target_policy?.direct_target_allowed),
    answerAction: decision.answer?.action,
    supportEvidenceOnly: decisionReturnTargetIsSupportEvidence(decision),
    thinCurrentWork: hasThinCurrentWork,
  });
}

function decisionReturnTargetIsSupportEvidence(decision: ContinueDecisionResult) {
  const target = decision.resume_work_target || decision.return_target || null;
  if (!target) return false;
  const selectedCandidate = decision.alternatives?.find(
    (candidate) =>
      candidate.candidate_id === decision.selected_candidate_id ||
      candidate.target_artifact_id === target.artifact_id,
  );
  if (selectedCandidate && candidateIsSupportEvidenceOnly(selectedCandidate)) {
    return true;
  }
  if (
    decision.support_evidence?.some(
      (item) => item.artifact_id && item.artifact_id === target.artifact_id && !item.public_return_eligible,
    )
  ) {
    return true;
  }
  return continueDecisionEvidenceNotes(decision).some((note) => {
    const key = normalizeToken(note);
    return (
      key === "branch_surface_is_evidence_not_default_return_target" ||
      key === "model_selected_unpromoted_branch" ||
      key === "model_selected_diagnostic_self" ||
      key === "model_selected_interrupt_without_promotion" ||
      key === "model_ignored_branch_eligibility" ||
      key.includes("branch_promotion_state_unpromoted") ||
      key.includes("branch_support")
    );
  });
}

function isPublicAlternativeCandidate(candidate: ContinueCandidateSummary) {
  return !candidateIsSupportEvidenceOnly(candidate);
}

function candidateIsSupportEvidenceOnly(candidate: ContinueCandidateSummary | ContinueWorkstreamCandidateDetail) {
  if (candidate.branch_public_return_eligible === false) return true;
  const branchState = normalizeToken(candidate.branch_promotion_state);
  if (
    [
      "unpromoted",
      "blocked_diagnostic_self",
      "blocked_feedback_suppressed",
      "blocked_thin_current_focus",
    ].includes(branchState)
  ) {
    return true;
  }
  const role = normalizeToken(candidate.continuation_role);
  if (
    [
      "support_context",
      "interruption",
      "diagnostic_only",
      "current_focus_only",
      "background_consumption",
      "suppressed",
      "needs_fresh_capture",
    ].includes(role)
  ) {
    return true;
  }
  if (candidate.candidate_kind === "evidence_only") return true;
  if (candidate.eligible_for_primary_selection === false) return true;
  if (candidate.public_alternative_eligible_after_feedback === false) return true;
  const notes = [
    ...(candidate.risk_flags || []),
    ...(candidate.score_caps_applied || []),
    candidate.why_not_primary || "",
    candidate.branch_promotion_reason || "",
    candidate.reason || "",
  ].map(normalizeToken);
  return notes.some(
    (note) =>
      note.includes("support_branch") ||
      note.includes("support_evidence") ||
      note.includes("unpromoted_branch") ||
      note.includes("branch_support") ||
      note.includes("not_primary_return_target") ||
      note.includes("only_evidence"),
  );
}

function continueDecisionEvidenceNotes(decision: ContinueDecisionResult) {
  return [
    ...decision.missing_evidence,
    ...decision.warnings,
    ...decision.validation_failures,
    ...(decision.active_current_work_unresolved?.missing_evidence || []),
    ...(decision.active_current_work_unresolved?.warnings || []),
  ].filter(Boolean);
}

function productMissingEvidenceLines(decision: ContinueDecisionResult) {
  const notes = [
    ...(decision.task_resolution_reason_codes || []),
    ...(decision.current_focus?.missing_fields || []),
    ...(decision.active_current_work_unresolved?.missing_evidence || []),
    ...(decision.missing_evidence || []),
    ...(decision.weak_surface_enrichment?.latest_weak_surface_attempt?.missing_fields || []),
  ];
  return [...new Set(notes.map(productSafeEvidenceNote).filter(Boolean))];
}

function productSafeEvidenceNote(value?: string | null) {
  const key = normalizeToken(value);
  const labels: Record<string, string> = {
    repo_root_missing: "Workspace or repository identity was not visible.",
    workspace_identity_missing: "Workspace identity was not visible.",
    project_identity_missing: "Workspace or repository identity was not visible.",
    thread_identity_missing: "Exact Codex thread was not visible.",
    thread_identity_uncertain: "Exact Codex thread was not visible.",
    active_file_missing: "Active file could not be identified.",
    command_state_missing: "Terminal command state was not clear.",
    command_signature_missing: "Terminal command state was not clear.",
    focused_control_missing: "Focused control was not available.",
    fresh_heavy_frame_missing: "Latest surface was event-backed without a fresh screenshot.",
    missing_fresh_heavy_frame_for_current_focus: "Latest surface was event-backed without a fresh screenshot.",
    openable_target_missing: "There is no safe exact target to open.",
    no_direct_url_or_document_path: "There is no safe exact target to open.",
    no_clear_current_task: "The exact current task is not supported by the available evidence.",
    no_current_goal: "No current user goal is supported by the available evidence.",
    no_valid_current_user_goal: "No current user-authored goal could be verified.",
    no_eligible_current_user_goal: "No current user-authored goal could be verified.",
    prior_boundary_history_only: "Older task text is available only as history.",
    control_only_current_evidence: "Recent visible controls were excluded from task understanding.",
    privacy_blocked_text: "Privacy rules blocked some visible evidence.",
  };
  return labels[key] || "";
}

function humanFocusLabel(focus?: ContinueFocusSummary | null) {
  if (!focus) return "No current focus returned.";
  return cleanHumanText(focus.title || focus.window_title || focus.app_name)
    || productizeArtifactKind(focus.artifact_kind)
    || "Current focus";
}

function humanEnrichedFocusLabel(
  focus?: ContinueFocusSummary | null,
  activeWork?: ActiveCurrentWorkUnresolved | null,
) {
  const appName = cleanHumanText(focus?.app_name || activeWork?.app_name || "");
  const title = cleanHumanText(
    focus?.display_title ||
      focus?.title ||
      focus?.window_title ||
      activeWork?.window_title ||
      "",
  );
  if (appName && title && normalizeToken(title) !== normalizeToken(appName)) {
    return `${appName} - ${title}`;
  }
  return appName || title || productizeArtifactKind(focus?.artifact_kind) || "";
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
    possible_distraction: "The current focus looks like a possible distraction.",
  };
  return labels[key] || (key ? sentenceCase(key) : "");
}

function productizeCandidateKind(value?: string | null) {
  const key = normalizeToken(value);
  const labels: Record<string, string> = {
    continue_edit: "Continue the edit in the primary target.",
    return_to_primary_artifact: "Return to the primary work target.",
    resolve_error: "Resolve the visible blocker.",
    review_completed_changes: "Review completed changes, commit them, or verify the app behavior.",
    commit_completed_changes: "Commit the completed changes.",
    manual_verify_app_behavior: "Run the app and manually verify behavior.",
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
    completed_progress: "Work appears completed and verified.",
    verification_without_return: "Verification branch has not been applied back to the target.",
    branch_without_return: "Search branch has not been applied back to the target.",
  };
  return labels[kind] || sentenceCase(kind) || "Something remains unresolved.";
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
  if (isInternalFacingText(raw)) return "";
  const key = normalizeToken(raw);
  const labels: Record<string, string> = {
    error_signal: "An error or failure was visible.",
    unresolved_error_signal: "There appears to be an unresolved error.",
    typing_in_composer: "A draft or composer was active.",
    frame_fallback: "This target is based on visible screen evidence.",
    primary_artifact_fallback: "This looks like the main place to continue.",
    last_meaningful_error: "The last meaningful state was an error/blocker.",
    secondary_artifact_for_searching: "Search was treated as supporting evidence.",
    current_focus_differs_from_return_target: "Current focus is not the return target.",
    thin_evidence: "Evidence is thin.",
    no_last_meaningful_action: "No clear last action is grounded yet.",
    no_openable_target: "No directly openable target was found.",
    no_candidate_generated: "No continuation target could be grounded yet.",
    missing_current_work_target_identity: "Exact task/thread identity is missing.",
    missing_current_work_openable_target: "No URL or document path is available for this current work.",
    missing_fresh_heavy_frame_for_current_focus: "No fresh screenshot/text snapshot is available for the current surface.",
    missing_current_work_visible_text: "Current work needs clearer visible text.",
    missing_current_work_thread_or_document_id: "Current work needs a thread or document identity.",
    active_current_work_unresolved: "Fresh current work is visible but not safely reopenable yet.",
    stale_return_target_suppressed_newer_current_focus: "An older return point was hidden because newer work was detected elsewhere.",
    feedback_no_eligible_candidates_after_suppression: "I won't suggest the target you marked wrong unless there is fresh evidence that you returned to it.",
    feedback_fresh_reconfirmation_required_before_target_reuse: "Fresh evidence is required before a corrected target can be recommended again.",
    feedback_suppressed_target_not_opened: "That target is no longer recommended because of your correction.",
    feedback_all_candidates_suppressed: "I won't suggest targets you marked wrong unless there is fresh evidence that you returned to them.",
    micro_inference_missing_openai_api_key: "AI ranking is unavailable; using local evidence.",
    smalltalk_self_observation_downranked: "Smalltalk ignored its own UI as evidence.",
    branch_surface_is_evidence_not_default_return_target: "Branch surface is evidence, not the default return target.",
    model_selected_unpromoted_branch: "Support branch was blocked as a return target.",
    model_selected_diagnostic_self: "Diagnostic evidence was blocked as a return target.",
    model_selected_interrupt_without_promotion: "Interruption context was blocked as a return target.",
    model_ignored_branch_eligibility: "Branch evidence was kept out of the return target.",
    model_promoted_support_without_local_evidence: "Support evidence was not promoted without local proof.",
    branch_or_support_target_promoted_without_strong_local_score: "Support branch lacked enough local evidence to become the target.",
    branch_support_unpromoted_public_return_gate: "Support branch was not promoted to a return target.",
    branch_support_not_default_return_target: "Support branch is evidence, not the default return target.",
    privacy_sensitive_or_redacted_target_local_only: "Target contains sensitive or redacted evidence and stays local.",
    current_focus_mismatch: "Current focus is not the return target.",
  };
  if (labels[key]) return labels[key];
  if (key.includes("scoring") || key.includes("score_component")) {
    return "Selected from local evidence.";
  }
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
    app_focus_only: "App focus only",
    frame_fallback: "Needs evidence preview",
    blocked: "Opening may be blocked",
    unknown: "Openability unknown",
  };
  return labels[key] || "";
}

function productizeEvidenceTrigger(value?: string | null) {
  const key = normalizeToken(value);
  const labels: Record<string, string> = {
    manual_capture: "Manual evidence",
    explicit_user_capture: "Manual evidence",
    scheduled_capture: "Scheduled evidence",
    event: "Event-backed",
    app_switch: "App changed",
    window_change: "Window changed",
    content_change: "Content changed",
    idle: "Idle evidence",
    startup: "Initial evidence",
  };
  return labels[key] || productizeInternalLabel(value) || "Evidence";
}

function productizeCaptureProvider(value?: string | null) {
  const key = normalizeToken(value);
  const labels: Record<string, string> = {
    screen_capture_kit: "ScreenCaptureKit",
    screenshot_cli: "Screenshot",
    accessibility: "Accessibility",
    ocr: "OCR",
    manual: "Manual",
  };
  return labels[key] || "Local evidence";
}

function safeProductLine(value: string, fallback: string) {
  const cleaned = cleanHumanText(value);
  if (!cleaned || isInternalFacingText(cleaned)) return fallback;
  return cleaned;
}

function boundedProductPhrase(value: string, maxChars: number) {
  if (value.length <= maxChars) return value;
  const clipped = value
    .slice(0, maxChars + 1)
    .replace(/\s+\S*$/, "")
    .replace(/[\s,;:.]+$/, "");
  return clipped ? `${clipped}…` : value.slice(0, maxChars);
}

function taskTruthTaskProductLine(value: string) {
  let line = safeProductLine(value, "")
    .replace(/^the user (?:was|is) (?:likely )?/i, "")
    .replace(/^the likely primary task is\s+/i, "")
    .replace(/^the (?:current|immediate) (?:subtask|task) is\s+/i, "")
    .replace(/^the user\s+/i, "")
    .trim();
  line = line.split(/,\s+so\s+the\s+primary\s+task\b/i)[0].trim();
  if (line) line = `${line.charAt(0).toUpperCase()}${line.slice(1)}`;
  return boundedProductPhrase(line, 150);
}

function taskTruthStateProductLine(
  value: string,
  kind: "progress" | "unfinished" | "next",
) {
  let line = safeProductLine(value, "")
    .replace(/^the most recent meaningful progress is that\s+/i, "")
    .replace(/^the unfinished (?:state|work) is\s+/i, "")
    .replace(/^the user\s+/i, "")
    .trim();
  line = line.split(/,\s+(?:confirming|which confirms|showing that)\b/i)[0].trim();
  line = line.split(/;\s*/)[0].trim();
  if (!line) return "";
  line = `${line.charAt(0).toUpperCase()}${line.slice(1)}`;
  const prefix = kind === "unfinished" ? "Still: " : "";
  return `${prefix}${boundedProductPhrase(line, kind === "progress" ? 150 : 130)}`;
}

function usableActivityRecap(
  recap?: ContinueActivityRecap | null,
): ContinueActivityRecap | null {
  if (!recap) return null;
  if (recap.schema !== "smalltalk.activity_recap.v1") return null;
  return recap;
}

function stripCurrentFocusPrefix(value: string) {
  return value.replace(/^current focus:\s*/i, "").trim();
}

function isInternalFacingText(value?: string | null) {
  const lower = (value || "").toLowerCase();
  if (!lower) return false;
  if (
    lower.includes("://") ||
    lower.includes("www.") ||
    lower.includes("file:") ||
    lower.includes("/users/") ||
    lower.includes("/private/") ||
    lower.includes("~/") ||
    lower.includes("\\") ||
    lower.includes("continue-candidate-") ||
    lower.includes("continue-decision-") ||
    lower.includes("workstream-") ||
    lower.includes("artifact-") ||
    lower.includes("action-") ||
    lower.includes("task-action-") ||
    lower.includes("frame-fallback") ||
    lower.includes("frame_fallback") ||
    lower.includes("frame fallback") ||
    lower.includes("semantic moment") ||
    lower.includes("open loop") ||
    lower.includes("sqlite") ||
    lower.includes("resume query") ||
    lower.includes("cloud resume") ||
    lower.includes("candidate scorer") ||
    lower.includes("scorer") ||
    lower.includes("target metadata") ||
    lower.includes("selected candidate") ||
    lower.includes("candidate id") ||
    lower.includes("action id") ||
    lower.includes("workstream id") ||
    lower.includes("artifact id") ||
    lower.includes("episode id") ||
    lower.includes("frame_id") ||
    lower.includes("frame id")
  ) {
    return true;
  }
  if (lower.split(/\s+/).some((token) => token.replace(/^[([\],]+|[)\],.]+$/g, "").startsWith("/"))) {
    return true;
  }
  return /\b(frame|action|artifact|episode|workstream)[-_]\d+\b/.test(lower) ||
    /\b(frame|action|artifact|episode|workstream)\s+\d+\b/.test(lower);
}

function presentOpenResult(result: OpenResumePointResult) {
  if (result.warnings.length > 0) {
    if (result.warnings.some((warning) => warning.includes("suppressed by feedback"))) {
      return "That target is no longer recommended because of your correction.";
    }
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
  return /^(frame|action|artifact|episode|workstream|continue-candidate|continue-decision|task-action)[-_]?[a-z0-9_-]+$/i.test(trimmed)
    || /^-?\d+$/.test(trimmed);
}

function continueTargetLabel(target?: ContinueReturnTarget | null) {
  if (!target) return "";
  return (
    cleanHumanText(target.title) ||
    pathBasename(target.document_path || target.browser_url) ||
    productizeArtifactKind(target.artifact_kind) ||
    ""
  );
}

function continueFocusLabel(focus?: ContinueFocusSummary | null) {
  if (!focus) return "No current focus returned.";
  return [
    focus.title || focus.window_title || focus.app_name || "Current focus",
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
  return frame.window_name || frame.app_name || "Evidence anchor";
}

function evidenceAnchorLabel(frame?: CaptureFrame | null) {
  if (!frame) return "No evidence selected";
  return cleanHumanText(frame.window_name || frame.app_name || "") || "Selected evidence";
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
  if (typeof value !== "number") return "unknown";
  return `${Math.round(value * 100)}%`;
}

function topContentUnit(detail: FrameDetail | null) {
  if (!detail) return null;
  return detail.content_units.find((unit) => unit.text && unit.text.length > 24) || detail.content_units[0] || null;
}

export default App;
