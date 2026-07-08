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
    semantic_moments: number;
    task_actions: number;
    episodes: number;
    workstreams: number;
    open_loops: number;
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

type ExclusionRule = {
  id: string;
  rule_type: string;
  pattern: string;
  action: string;
  enabled: boolean;
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
    open_loops: number;
    open_loop_artifacts: number;
    open_loop_evidence: number;
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

type ActiveCurrentWorkUnresolved = {
  id: string;
  observed_at_ms: number;
  app_name?: string | null;
  bundle_id?: string | null;
  window_title?: string | null;
  artifact_id?: string | null;
  frame_id?: string | null;
  event_ids: string[];
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

type ContinueDecisionResult = {
  decision_id: string;
  mode: string;
  cache_hit: boolean;
  source: string;
  model?: string | null;
  response_id?: string | null;
  current_focus?: ContinueFocusSummary | null;
  active_current_work_unresolved?: ActiveCurrentWorkUnresolved | null;
  current_activity?: string | null;
  selected_workstream?: ContinueSelectedWorkstream | null;
  selected_candidate_id?: string | null;
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
  observe_before_decide?: unknown | null;
  app_activity?: unknown | null;
  activity_summary?: ContinueActivitySummary | null;
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

type ContinueCardActionState =
  | { kind: "openable_return_target"; label: "Continue here" }
  | { kind: "thin_current_work"; label: "Inspect latest evidence" }
  | { kind: "no_clear_continuation"; label: "Inspect evidence" };

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

type ViewMode = "continue" | "developer";
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
type ContinueFreshness =
  | "waiting_for_evidence"
  | "ready"
  | "updating"
  | "current"
  | "new_evidence"
  | "thin_evidence"
  | "needs_attention";
type ContinueRequestTrigger = "manual" | "startup" | "background";

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

type ContinueFreshnessPresentation = {
  state: ContinueFreshness;
  label: string;
  detail: string;
  stale: boolean;
  thin: boolean;
  openable: boolean;
  updatedAtLabel?: string;
};

const RECENT_MEMORY_DELETE_RANGE_MS = 60 * 60 * 1000;
const BACKGROUND_CONTINUE_VISIBLE_DEBOUNCE_MS = 5000;
const BACKGROUND_CONTINUE_IDLE_DEBOUNCE_MS = 30000;
const BACKGROUND_CONTINUE_MIN_INTERVAL_MS = 60000;
const STATUS_HEARTBEAT_RUNNING_MS = 15000;
const STATUS_HEARTBEAT_IDLE_MS = 30000;

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
    label: "Not observing this app",
    detail: "Smalltalk is respecting your privacy boundary here.",
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
  const [continueDecisionEvidenceSnapshot, setContinueDecisionEvidenceSnapshot] =
    useState<ContinueEvidenceSnapshot | null>(null);
  const [continueError, setContinueError] = useState<string | null>(null);
  const [backgroundContinueError, setBackgroundContinueError] = useState<string | null>(null);
  const [continueOpenResult, setContinueOpenResult] = useState<OpenResumePointResult | null>(null);
  const [quietContinueRefreshing, setQuietContinueRefreshing] = useState(false);
  const [continueUpdatedAtMs, setContinueUpdatedAtMs] = useState<number | null>(null);
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
  const [memoryMenuOpen, setMemoryMenuOpen] = useState(false);
  const [privacyPanelOpen, setPrivacyPanelOpen] = useState(false);
  const [exclusionRules, setExclusionRules] = useState<ExclusionRule[]>([]);
  const [privacyActionStatus, setPrivacyActionStatus] = useState<string | null>(null);
  const [pendingDangerAction, setPendingDangerAction] = useState<DangerousAction | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [appVisible, setAppVisible] = useState(() => document.visibilityState === "visible");
  const storeGenerationRef = useRef(0);
  const autoContinueRef = useRef(false);
  const continueRequestInFlightRef = useRef(false);
  const lastBackgroundContinueAttemptRef = useRef(0);
  const failedBackgroundContinueSignatureRef = useRef<string | null>(null);
  const captureMenuRef = useRef<HTMLDetailsElement | null>(null);
  const isDeleting = busyAction === "delete_all_frames" || busyAction === "delete_recent_captures";
  const diagnosticsOpen = viewMode === "developer";
  const currentSession = status.active_session || status.latest_session || null;
  const currentSessionId = currentSession?.id || null;

  const refreshStatus = useCallback(async (): Promise<CaptureStatus | null> => {
    const requestGeneration = storeGenerationRef.current;
    try {
      const nextStatus = await invoke<CaptureStatus>("capture_status");
      if (requestGeneration !== storeGenerationRef.current) return null;
      setStatus(nextStatus);
      setError(null);
      if (!selectedFrame && nextStatus.latest_frame) {
        setSelectedFrame(nextStatus.latest_frame);
      }
      return nextStatus;
    } catch (err) {
      setError(String(err));
      return null;
    }
  }, [selectedFrame]);

  const refreshContinueMemory = useCallback(async (): Promise<ContinueMemoryStatus | null> => {
    try {
      const nextMemory = await invoke<ContinueMemoryStatus>("get_continue_memory_status");
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
    async (decision: ContinueDecisionResult) => {
      setContinueDecision(decision);
      setSelectedWorkstreamId(decision.selected_workstream?.workstream_id || null);
      setContinueUpdatedAtMs(Date.now());
      setBackgroundContinueError(null);
      failedBackgroundContinueSignatureRef.current = null;

      const [nextStatus, nextMemory] = await Promise.all([
        refreshStatus(),
        refreshContinueMemory(),
      ]);
      const evidenceStatus = nextStatus || status;
      const evidenceMemory = nextMemory || continueMemory;
      setContinueDecisionFrameCount(evidenceStatus.frame_count);
      setContinueDecisionEvidenceSnapshot(
        buildContinueEvidenceSnapshot(evidenceStatus, evidenceMemory),
      );
    },
    [continueMemory, refreshContinueMemory, refreshStatus, status],
  );

  const runContinueDecision = useCallback(async (options: {
    forceRebuild?: boolean;
    writeAudit?: boolean;
    trigger?: ContinueRequestTrigger;
  } = {}) => {
    if (continueRequestInFlightRef.current) return;
    const trigger = options.trigger || (options.writeAudit === true ? "manual" : "startup");
    const background = trigger === "background";
    continueRequestInFlightRef.current = true;
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
      const decision = await invoke<ContinueDecisionResult>("get_continue_decision", {
        input: {
          mode: options.forceRebuild === true ? "rebuild" : "normal",
          rebuild_layers: options.forceRebuild === true,
          micro_inference_enabled: true,
          max_candidates_for_model: 5,
          audit_output_enabled: options.writeAudit === true,
        },
      });
      await applyContinueDecision(decision);
      if (diagnosticsOpen) {
        await refreshWorkstreams();
      }
      const firstEvidenceFrame = decision.evidence_anchors.frame_ids[0];
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
        setContinueError(`Continue failed: ${String(err)}`);
      }
    } finally {
      continueRequestInFlightRef.current = false;
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
    const resumeTarget = continueDecision.resume_work_target || continueDecision.return_target || null;
    setBusyAction("open_continue_target");
    setContinueOpenResult(null);
    setContinueError(null);
    try {
      const result = await invoke<OpenResumePointResult>("open_resume_point", {
        input: {
          continue_decision_id: continueDecision.decision_id,
          target_artifact_id: resumeTarget?.artifact_id || null,
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
            },
          },
        );
        setFeedbackStatus("Got it. Smalltalk will use that correction next time.");
        await loadWorkstreamDetail(workstreamId);
        if (
          feedbackKind === "rejected" ||
          feedbackKind === "ignored" ||
          feedbackKind === "corrected" ||
          feedbackKind === "artifact_only_evidence" ||
          feedbackKind === "ignored_workstream"
        ) {
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
    [continueDecision, recordContinueFeedback, selectedWorkstreamId, workstreamDetail],
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
      setContinueDecision(null);
      setContinueDecisionFrameCount(null);
      setContinueDecisionEvidenceSnapshot(null);
      setContinueUpdatedAtMs(null);
      setBackgroundContinueError(null);
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
      } finally {
        setBusyAction(null);
      }
    },
    [currentSessionId, diagnosticsOpen, query, refreshStatus, refreshTimeline, runSearch, selectFrame],
  );

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
      setContinueDecision(null);
      setContinueDecisionFrameCount(null);
      setContinueDecisionEvidenceSnapshot(null);
      setContinueUpdatedAtMs(null);
      setBackgroundContinueError(null);
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
      setContinueDecision(null);
      setContinueDecisionFrameCount(null);
      setContinueDecisionEvidenceSnapshot(null);
      setContinueUpdatedAtMs(null);
      setBackgroundContinueError(null);
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
    void refreshContinueMemory();
    void refreshExclusionRules();
  }, [refreshContinueMemory, refreshExclusionRules, refreshStatus]);

  useEffect(() => {
    if (privacyPanelOpen) {
      void refreshExclusionRules();
    }
  }, [privacyPanelOpen, refreshExclusionRules]);

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
    if (
      autoContinueRef.current ||
      busyAction !== null ||
      continueDecision ||
      status.frame_count === 0
    ) {
      return;
    }
    autoContinueRef.current = true;
    void runContinueDecision({ writeAudit: false, trigger: "startup" });
  }, [busyAction, continueDecision, runContinueDecision, status.frame_count]);

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
  const continueIsThin = isThinContinueDecision(continueDecision);
  const continueTargetOpenable = isDirectResumeTargetOpenable(continueResumeTarget);
  const continueRefreshBusy = busyAction === "get_continue_decision" || quietContinueRefreshing;
  const continueFreshness = deriveContinueFreshness({
    hasEvidence: continueHasEvidence,
    decision: continueDecision,
    stale: continueIsStale,
    updating: continueRefreshBusy,
    thin: continueIsThin,
    openable: continueTargetOpenable,
    error: continueDecision ? backgroundContinueError : continueError,
    updatedAtMs: continueUpdatedAtMs,
  });
  const continueFreshnessLabel = continueFreshness.label;
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
  const memoryProductStatus = deriveMemoryProductStatus(
    status,
    continueHasEvidence,
    busyAction,
    memorySurfacePrivate,
  );
  const memoryProduct = getMemoryProductCopy(memoryProductStatus, status.last_error);
  const continueStatusLabel = memoryProduct.label;
  const memoryCueLabel = status.last_error
    ? memoryProduct.label
    : continueIsStale
      ? "New evidence"
      : continueFreshness.state === "updating"
        ? "Updating"
      : memoryProduct.label;
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
  const openDeveloperMode = useCallback(() => {
    setMemoryMenuOpen(false);
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

    listen<CaptureStatus>("capture-status", (event) => {
      setStatus(event.payload);
      if (diagnosticsOpen) {
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
      void applyContinueDecision(event.payload);
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
    diagnosticsOpen,
    selectedFrame,
  ]);

  return (
    <main className={`capture-shell ${viewMode === "developer" ? "developer-mode" : "continue-mode"}`}>
      <header className="capture-topbar">
        <div className="identity-block">
          <div className="brand-mark" aria-hidden="true">S</div>
          <div>
            <p className="product-kicker">Smalltalk</p>
            <h1>{viewMode === "developer" ? "Evidence inspection" : "Continue"}</h1>
          </div>
        </div>

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
          ) : (
	            <span className={`memory-dot ${memoryProductStatus} freshness-${continueFreshness.state}`}>
	              {memoryCueLabel}
	            </span>
          )}
        </div>

        <div className="topbar-meta" aria-label={viewMode === "developer" ? "Developer status" : "Continue controls"}>
          {viewMode === "developer" ? (
            <>
              <StatusPill label="Local memory" value={continueStatusLabel} tone={status.running ? "good" : status.last_error ? "bad" : "quiet"} />
              <StatusPill label="Evidence age" value={latestEvidenceAgeLabel} />
              <StatusPill label="Continue" value={continueFreshnessLabel} tone={continueFreshnessTone(continueFreshness.state)} />
            </>
          ) : (
            <div className="topbar-actions">
              <button
                className="primary-button topbar-continue-button"
                type="button"
                disabled={!continueHasEvidence || busyAction !== null || quietContinueRefreshing}
                aria-busy={continueRefreshBusy}
                onClick={() => void runContinueDecision({ writeAudit: true })}
              >
                {continueRefreshBusy ? "Finding" : "Continue"}
              </button>
              <details
                className="capture-menu topbar-memory-menu"
                open={memoryMenuOpen}
                ref={captureMenuRef}
                onToggle={(event) => setMemoryMenuOpen(event.currentTarget.open)}
              >
                <summary>Memory</summary>
                <div>
                  {status.running ? (
                    <button
                      className="secondary-button"
                      disabled={busyAction !== null}
                      aria-busy={busyAction === "stop_capture"}
                      onClick={() => {
                        setMemoryMenuOpen(false);
                        void runAction("stop_capture");
                      }}
                      type="button"
                    >
	                      {busyAction === "stop_capture" ? "Pausing" : "Pause memory"}
                    </button>
                  ) : (
                    <button
                      className="secondary-button"
                      disabled={busyAction !== null}
                      aria-busy={busyAction === "start_capture"}
                      onClick={() => {
                        setMemoryMenuOpen(false);
                        void runAction("start_capture");
                      }}
                      type="button"
                    >
                      {busyAction === "start_capture" ? "Starting" : "Turn on local memory"}
                    </button>
	                  )}
	                  <button
	                    className="secondary-button"
	                    disabled={busyAction !== null}
	                    onClick={openPrivacyPanel}
	                    type="button"
	                  >
	                    Privacy
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
                    {busyAction === "capture_once" ? "Updating" : "Update memory now"}
                  </button>
                  <button
                    className="secondary-button"
                    disabled={!continueHasEvidence || busyAction !== null || quietContinueRefreshing}
                    aria-busy={continueRefreshBusy}
                    onClick={() => {
                      setMemoryMenuOpen(false);
                      void runContinueDecision({ writeAudit: true });
                    }}
                    type="button"
	                  >
	                    {continueRefreshBusy ? "Refreshing" : "Refresh Continue"}
	                  </button>
	                  <button
	                    className="danger-button"
	                    disabled={!continueHasEvidence || busyAction !== null}
	                    aria-busy={isDeleting}
	                    onClick={() => {
	                      setMemoryMenuOpen(false);
	                      deleteAllFrames();
	                    }}
	                    type="button"
	                  >
	                    {isDeleting ? "Deleting" : "Delete local memory"}
	                  </button>
	                  {showInspectEntry ? (
	                    <>
	                      <span className="menu-section-label">Advanced</span>
                      <button
                        className="text-button"
                        onClick={openDeveloperMode}
                        type="button"
                      >
                        Inspect local evidence
                      </button>
                    </>
                  ) : null}
                </div>
              </details>
            </div>
          )}
        </div>
      </header>

      <div
        className="app-scroll"
        onScroll={() => {
          if (memoryMenuOpen) setMemoryMenuOpen(false);
        }}
      >
      <section className="continue-home" aria-label="Continue">
        <div className="continue-stage">
          <ContinuationAnswer
            decision={continueDecision}
            primaryMessage={continuePrimaryMessage}
            hasEvidence={continueHasEvidence}
            running={status.running}
            busyAction={busyAction}
            continueRefreshBusy={continueRefreshBusy}
            openResult={continueOpenResult}
            freshness={continueFreshness}
            onStartMemory={() => void runAction("start_capture")}
            onContinue={() => void runContinueDecision({ writeAudit: true })}
            onOpenTarget={() => void openContinueTarget()}
            onInspectEvidence={() => {
              const firstEvidenceFrame = continueDecision?.evidence_anchors.frame_ids[0] || null;
              void revealContinueFrame(firstEvidenceFrame);
            }}
            feedbackStatus={feedbackStatus}
            onRecordFeedback={(kind) => void recordContinueFeedback(kind)}
            onUseAlternative={(candidate) => void continueFromAlternative(candidate)}
          />
          <ContinueCompanionPanel
            status={status}
            hasEvidence={continueHasEvidence}
            decision={continueDecision}
            busyAction={busyAction}
            continueRefreshBusy={continueRefreshBusy}
            statusLabel={continueStatusLabel}
            freshness={continueFreshness}
	            memoryProductStatus={memoryProductStatus}
	            memoryProduct={memoryProduct}
	            privacyActionStatus={privacyActionStatus}
	            onStartMemory={() => void runAction("start_capture")}
	            onPauseMemory={() => void runAction("stop_capture")}
	            onCaptureEvidence={() => void runAction("capture_once")}
	            onRefreshContinue={() => void runContinueDecision({ writeAudit: true })}
	            onOpenPrivacy={openPrivacyPanel}
	            onDeleteLocalMemory={deleteAllFrames}
	          />
	        </div>
	      </section>

	      {privacyPanelOpen ? (
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
	      ) : null}

	      {continueError ? (
	        <MemoryErrorBox message={continueError} />
	      ) : null}

	      {error || status.last_error ? (
	        <MemoryErrorBox message={error || status.last_error || ""} />
	      ) : null}

      {evidenceOpen ? (
        <ContinueEvidencePanel
          decision={continueDecision}
          selectedFrame={selectedFrame}
          imageData={imageData}
          onClose={() => setEvidenceOpen(false)}
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
              {continueRefreshBusy ? "Finding" : "Continue"}
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
                onClick={() => void runContinueDecision({ forceRebuild: true, writeAudit: true })}
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

function ContinueCompanionPanel({
  status,
  hasEvidence,
  decision,
  busyAction,
  continueRefreshBusy,
  statusLabel,
  freshness,
  memoryProductStatus,
  memoryProduct,
  privacyActionStatus,
  onStartMemory,
  onPauseMemory,
  onCaptureEvidence,
  onRefreshContinue,
  onOpenPrivacy,
  onDeleteLocalMemory,
}: {
  status: CaptureStatus;
  hasEvidence: boolean;
  decision: ContinueDecisionResult | null;
  busyAction: string | null;
  continueRefreshBusy: boolean;
  statusLabel: string;
  freshness: ContinueFreshnessPresentation;
  memoryProductStatus: MemoryProductStatus;
  memoryProduct: { label: string; detail: string };
  privacyActionStatus: string | null;
  onStartMemory: () => void;
  onPauseMemory: () => void;
  onCaptureEvidence: () => void;
  onRefreshContinue: () => void;
  onOpenPrivacy: () => void;
  onDeleteLocalMemory: () => void;
}) {
  const memoryTone = companionToneForStatus(memoryProductStatus, freshness.state);

  return (
    <aside className={`continue-companion freshness-${freshness.state}`} aria-label="Local memory and trust status">
      <div className={`companion-orb ${memoryTone}`} aria-hidden="true">
        <span />
      </div>
      <div className="companion-copy">
        <span>{statusLabel}</span>
        <strong>{freshness.state === "new_evidence" ? "New evidence" : memoryProduct.label}</strong>
        <p>
          {freshness.state === "new_evidence"
            ? "Continue will refresh quietly."
            : decision || hasEvidence
              ? freshness.detail
              : memoryProduct.detail}
        </p>
      </div>

      <div className="privacy-note">
        <span>Privacy boundary</span>
        <p>Local memory is private to this device. Raw typed characters and full clipboard contents are not stored.</p>
      </div>

      <details className="companion-controls">
        <summary>Memory controls</summary>
        <div className="companion-actions" aria-label="Local memory controls">
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
              {busyAction === "start_capture" ? "Starting" : "Turn on local memory"}
            </button>
          )}
          <button
            className="secondary-button"
            type="button"
            disabled={!status.running || busyAction !== null}
            aria-busy={busyAction === "capture_once"}
            onClick={onCaptureEvidence}
          >
            Update memory
          </button>
          <button
            className="text-button"
            type="button"
            disabled={!hasEvidence || busyAction !== null || continueRefreshBusy}
            aria-busy={continueRefreshBusy}
            onClick={onRefreshContinue}
          >
            {continueRefreshBusy ? "Refreshing" : "Refresh Continue"}
          </button>
          <button
            className="secondary-button"
            type="button"
            disabled={busyAction !== null}
            onClick={onOpenPrivacy}
          >
            Privacy
          </button>
          <button
            className="danger-button"
            type="button"
            disabled={!hasEvidence || busyAction !== null}
            aria-busy={busyAction === "delete_all_frames"}
            onClick={onDeleteLocalMemory}
          >
            Delete local memory
          </button>
          {privacyActionStatus ? (
            <p className="privacy-action-status" role="status">{privacyActionStatus}</p>
          ) : null}
        </div>
      </details>
    </aside>
  );
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

  return (
    <section className="privacy-panel" aria-label="Privacy">
      <div className="privacy-panel-head">
        <div>
          <p className="product-kicker">Privacy</p>
          <h2>Local memory boundaries</h2>
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
        Smalltalk stores local work context such as app/window signals, visible text when available, evidence quality, and derived workstream metadata. It does not store raw typed characters or full clipboard contents.
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
            ? `${currentAppLabel || "This app"} can be excluded from future local memory.`
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
                  <span>{formatExclusionAction(rule.action)}</span>
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
      {message && productCopy !== message ? (
        <details>
          <summary>Details</summary>
          <span>{message}</span>
        </details>
      ) : null}
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

function isThinContinueDecision(decision: ContinueDecisionResult | null) {
  if (!decision) return false;
  const validation = normalizeToken(decision.validation_status);
  const confidenceLabelValue = normalizeToken(decision.confidence_label);
  return decision.confidence < 0.55 ||
    validation.includes("thin") ||
    validation.includes("no_clear") ||
    confidenceLabelValue.includes("thin") ||
    decision.missing_evidence.length > 0 ||
    decision.validation_failures.length > 0;
}

function deriveContinueFreshness({
  hasEvidence,
  decision,
  stale,
  updating,
  thin,
  openable,
  error,
  updatedAtMs,
}: {
  hasEvidence: boolean;
  decision: ContinueDecisionResult | null;
  stale: boolean;
  updating: boolean;
  thin: boolean;
  openable: boolean;
  error?: string | null;
  updatedAtMs?: number | null;
}): ContinueFreshnessPresentation {
  if (updating) {
    return {
      state: "updating",
      label: "Updating",
      detail: "Finding the latest continuation.",
      stale,
      thin,
      openable,
    };
  }
  if (!hasEvidence) {
    return {
      state: "waiting_for_evidence",
      label: "Waiting for evidence",
      detail: "Local memory has not collected enough evidence yet.",
      stale: false,
      thin: false,
      openable: false,
    };
  }
  if (!decision) {
    return {
      state: "ready",
      label: "Ready",
      detail: "Evidence exists and Continue can run.",
      stale: false,
      thin: false,
      openable: false,
    };
  }
  if (stale) {
    return {
      state: "new_evidence",
      label: "New evidence",
      detail: "The previous answer is still visible while Smalltalk refreshes quietly.",
      stale: true,
      thin,
      openable,
    };
  }
  if (error) {
    return {
      state: "needs_attention",
      label: "Needs attention",
      detail: productizeMemoryError(error),
      stale,
      thin,
      openable,
    };
  }
  if (thin || !openable) {
    return {
      state: "thin_evidence",
      label: "Thin evidence",
      detail: openable
        ? "This is the best available answer from thin local evidence."
        : "No reliable return target is grounded yet.",
      stale: false,
      thin: true,
      openable,
      updatedAtLabel: updatedAtMs ? `Updated ${formatRelativeAge(updatedAtMs)}` : undefined,
    };
  }
  return {
    state: "current",
    label: "Ready to continue",
    detail: "The current Continue answer matches the latest local evidence.",
    stale: false,
    thin: false,
    openable: true,
    updatedAtLabel: updatedAtMs ? `Updated ${formatRelativeAge(updatedAtMs)}` : undefined,
  };
}

function continueFreshnessTone(state: ContinueFreshness): "quiet" | "good" | "warn" | "bad" {
  if (state === "current") return "good";
  if (state === "new_evidence" || state === "thin_evidence" || state === "updating") return "warn";
  if (state === "needs_attention") return "bad";
  return "quiet";
}

function freshnessBadgeLabel(
  freshness: ContinueFreshnessPresentation,
  bestAvailable: boolean,
) {
  if (freshness.state === "new_evidence") return "New evidence since this answer";
  if (freshness.state === "updating") return "Updating";
  if (bestAvailable || freshness.state === "thin_evidence") return "Best available answer";
  if (freshness.openable) return "Ready to continue";
  return freshness.label;
}

function ContinuationAnswer({
  decision,
  primaryMessage,
  hasEvidence,
  running,
  busyAction,
  continueRefreshBusy,
  openResult,
  freshness,
  feedbackStatus,
  onStartMemory,
  onContinue,
  onOpenTarget,
  onInspectEvidence,
  onRecordFeedback,
  onUseAlternative,
}: {
  decision: ContinueDecisionResult | null;
  primaryMessage: string;
  hasEvidence: boolean;
  running: boolean;
  busyAction: string | null;
  continueRefreshBusy: boolean;
  openResult: OpenResumePointResult | null;
  freshness: ContinueFreshnessPresentation;
  feedbackStatus: string | null;
  onStartMemory: () => void;
  onContinue: () => void;
  onOpenTarget: () => void;
  onInspectEvidence: () => void;
  onRecordFeedback: (feedbackKind: string) => void;
  onUseAlternative: (candidate: ContinueCandidateSummary) => void;
}) {
  const resumeTarget = decision?.resume_work_target || decision?.return_target || null;
  const actionState = decision ? getContinueCardActionState(decision) : null;
  const canOpenResumeTarget = actionState?.kind === "openable_return_target";
  const isThinCurrentWork = actionState?.kind === "thin_current_work";
  const isInspectPrimary = Boolean(actionState && actionState.kind !== "openable_return_target");
  const lowConfidence = decision ? decision.confidence < 0.55 : false;
  const handoff = decision?.handoff || null;
  const presentation = decision && !handoff ? presentContinueDecision(decision) : null;
  const [correctionOpen, setCorrectionOpen] = useState(false);
  const [alternativesOpen, setAlternativesOpen] = useState(false);
  const alternatives = (decision?.alternatives || []).filter(isPublicAlternativeCandidate);
  const visibleAlternatives = alternativesOpen ? alternatives.slice(0, 4) : [];
  const evidenceLines = decision ? productEvidenceLines(decision).slice(0, 3) : [];
  const supportEvidenceLines = decision ? supportEvidenceProductLines(decision).slice(0, 3) : [];
  const rawWorkstreamLine = handoff?.headline || presentation?.workstreamTitle || primaryMessage;
  const rawTargetLine = handoff?.return_line || presentation?.returnTarget || "No stable place to continue yet.";
  const targetLooksInternal = isInternalFacingText(rawTargetLine);
  const workstreamLine = isThinCurrentWork
    ? "Most recent work seen"
    : targetLooksInternal
    ? "No reliable continuation target yet"
    : safeProductLine(rawWorkstreamLine, "Recent work");
  const targetLine = isInspectPrimary
    ? "Exact return target missing"
    : targetLooksInternal
    ? "No reliable return target is grounded yet."
    : safeProductLine(rawTargetLine, "No stable place to continue yet.");
  const targetMeta = isThinCurrentWork
    ? "Smalltalk saw recent activity here, but does not yet have enough evidence to reopen the exact task safely."
    : isInspectPrimary
    ? "Smalltalk does not yet have enough evidence to reopen an exact task safely."
    : targetLooksInternal
    ? "I don't have a reliable app or page target for this yet."
    : presentation?.targetMeta || humanTargetMeta(resumeTarget);
  const lastStateLine = targetLooksInternal
    ? "I do not have enough local evidence to identify a reliable unfinished task."
    : safeProductLine(
        handoff?.last_state_line || presentation?.lastState || "No last meaningful state is clear yet.",
        "No last meaningful state is clear yet.",
      );
  const nextActionLine =
    targetLooksInternal
      ? "Use more local evidence before selecting a continuation target."
      : safeProductLine(
          handoff?.next_action ||
            presentation?.nextAction ||
            "Open the target and continue from the last meaningful state.",
          "Open the target and continue from the last meaningful state.",
        );
  const currentFocusLine = stripCurrentFocusPrefix(
    safeProductLine(handoff?.current_focus_line || presentation?.currentFocus || "", ""),
  ) || humanFocusLabel(decision?.current_focus);
  const provenanceLabel = decision ? continueProvenanceLabel(decision) : "";
  const provenanceTone = decision ? continueProvenanceTone(decision) : "local";
  const whyLines = (
    handoff?.why_this
      ?.map(productizeInternalLabel)
      .filter((line) => line && !isInternalFacingText(line)) ||
    (presentation?.decisionReason ? [presentation.decisionReason] : [])
  )
    .map((line) => safeProductLine(line, ""))
    .filter(Boolean)
    .slice(0, 3);
  const targetBlockLabel = isInspectPrimary
    ? "No safe return target yet"
    : lowConfidence
      ? "Best available place to continue"
      : "Continue at";
  const openButtonLabel =
    busyAction === "open_continue_target" && canOpenResumeTarget
      ? "Opening"
      : actionState?.label || "Inspect evidence";
  const uncertaintyLine =
    isThinCurrentWork
      ? evidenceLines[0] || "Evidence is thin."
      : isInspectPrimary
      ? evidenceLines[0] || "No safe return target is grounded yet."
      :
    targetLooksInternal
      ? "I saw the current focus, but I don't have a reliable return target yet."
      : safeProductLine(
          handoff?.user_visible_uncertainty ||
            handoff?.missing_evidence_line ||
            presentation?.missingEvidenceSummary ||
            "",
          "",
        );
  const showCurrentFocus =
    Boolean(currentFocusLine) &&
    currentFocusLine !== "No current focus returned." &&
    (isThinCurrentWork || currentFocusLine !== targetLine) &&
    (
      isThinCurrentWork ||
      decision?.warnings.includes("current_focus_differs_from_return_target") ||
      decision?.warnings.includes("current_focus_mismatch") ||
      decision?.current_focus?.artifact_id !== resumeTarget?.artifact_id
    );

  useEffect(() => {
    setCorrectionOpen(false);
    setAlternativesOpen(false);
  }, [decision?.decision_id]);

  const recordAndClose = (feedbackKind: string) => {
    onRecordFeedback(feedbackKind);
    setCorrectionOpen(false);
  };
  const emptyPrimaryStartsMemory = !hasEvidence && !running;
  const emptyPrimaryBusy = emptyPrimaryStartsMemory
    ? busyAction === "start_capture"
    : continueRefreshBusy;
  const emptyPrimaryLabel = emptyPrimaryStartsMemory
    ? emptyPrimaryBusy ? "Starting" : "Turn on local memory"
    : continueRefreshBusy
      ? running && !hasEvidence ? "Finding best available answer" : "Finding where to continue"
      : running && !hasEvidence ? "Find best available answer" : "Find where to continue";
  const emptySubcopy = emptyPrimaryStartsMemory
    ? "Smalltalk will quietly keep enough context to help you continue later."
    : running && !hasEvidence
      ? "Keep working. Smalltalk will surface a continuation when there is enough evidence."
      : "Smalltalk can answer from local evidence without stopping memory first.";

  if (!decision) {
    return (
      <section className="continue-card continuation-answer empty" aria-label="Continue decision">
        <div className="answer-shell">
          <div className="answer-eyebrow">
            <span>{freshness.label}</span>
          </div>
          <div className="answer-hero">
            <p>{hasEvidence ? "Ready to find your continuation" : running ? "Local memory is on" : "Turn on local memory once"}</p>
            <h2>{primaryMessage}</h2>
            <span>{emptySubcopy}</span>
          </div>
          <div className="answer-actions">
            <button
              className="primary-button"
              type="button"
              disabled={busyAction !== null || continueRefreshBusy}
              aria-busy={emptyPrimaryBusy}
              onClick={emptyPrimaryStartsMemory ? onStartMemory : onContinue}
            >
              {emptyPrimaryLabel}
            </button>
          </div>
        </div>
      </section>
    );
  }

  return (
    <section className={`continue-card continuation-answer ${lowConfidence || targetLooksInternal ? "low-confidence" : ""}`} aria-label="Continue decision">
      <div className="answer-shell">
        <div className="answer-eyebrow answer-provenance">
          <span>{freshnessBadgeLabel(freshness, lowConfidence || targetLooksInternal)}</span>
          {freshness.updatedAtLabel ? (
            <span className="freshness-updated">{freshness.updatedAtLabel}</span>
          ) : null}
          <span className={`provenance-pill ${provenanceTone}`}>{provenanceLabel}</span>
        </div>

        <div className="answer-hero">
          <p>{isThinCurrentWork ? "Current work detected" : "You were working on"}</p>
          <h2>{workstreamLine}</h2>
        </div>

        <div className="answer-target">
          <div>
            <span>{targetBlockLabel}</span>
            <strong>{targetLine}</strong>
            <small>{targetMeta}</small>
          </div>
        </div>

        <div className="answer-state">
          <div>
            <span>Last meaningful state</span>
            <strong>{lastStateLine}</strong>
          </div>
          <div>
            <span>Next action</span>
            <strong>{nextActionLine}</strong>
          </div>
        </div>

        {whyLines.length ? (
          <div className="answer-why-strip" aria-label="Why this continuation">
            {whyLines.map((line) => (
              <span key={line}>{line}</span>
            ))}
          </div>
        ) : null}

        {isInspectPrimary && evidenceLines.length ? (
          <div className="answer-why-strip" aria-label="Missing evidence">
            {evidenceLines.map((line) => (
              <span key={line}>{line}</span>
            ))}
          </div>
        ) : null}

        {supportEvidenceLines.length ? (
          <div className="answer-why-strip support-evidence-strip" aria-label="Support evidence">
            {supportEvidenceLines.map((line) => (
              <span key={line}>{line}</span>
            ))}
          </div>
        ) : null}

        {showCurrentFocus ? (
          <p className="answer-context">
            Current focus: <strong>{currentFocusLine}</strong>
          </p>
        ) : null}

        {lowConfidence || uncertaintyLine ? (
          <p className="answer-uncertainty">
            {uncertaintyLine || "Evidence is thin, so this is the best available local recommendation."}
          </p>
        ) : null}

        <div className="answer-actions">
          <button
            className="primary-button"
            type="button"
            disabled={busyAction !== null}
            aria-busy={busyAction === "open_continue_target"}
            onClick={canOpenResumeTarget ? onOpenTarget : onInspectEvidence}
          >
            {openButtonLabel}
          </button>
          <button
            className="secondary-button"
            type="button"
            disabled={busyAction !== null}
            onClick={onInspectEvidence}
          >
            Why this?
          </button>
          <button
            className="secondary-button"
            type="button"
            disabled={busyAction !== null || continueRefreshBusy}
            aria-busy={continueRefreshBusy}
            onClick={onContinue}
          >
            {continueRefreshBusy ? "Refreshing" : freshness.stale ? "Refresh Continue" : "Refresh"}
          </button>
        </div>

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
                Not this
              </button>
              <button
                className="secondary-button"
                type="button"
                disabled={busyAction !== null || alternatives.length === 0}
                onClick={() => {
                  setAlternativesOpen((open) => !open);
                }}
              >
                {alternativesOpen ? "Hide alternatives" : "Show alternatives"}
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
          {feedbackStatus ? (
            <p className="correction-feedback" role="status">{feedbackStatus}</p>
          ) : null}
        </div>

        {visibleAlternatives.length > 0 ? (
          <div className="alternative-list" aria-label="Alternative continuations">
            <div className="alternative-heading">
              <strong>{isThinCurrentWork ? "Older possible return points" : "Alternatives"}</strong>
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

        {openResult ? (
          <div className="continue-open-result">
            <strong>Open target</strong>
            <span>{presentOpenResult(openResult)}</span>
          </div>
        ) : null}
      </div>
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
  const provenanceLabel = decision ? continueProvenanceLabel(decision) : "";

  if (!decision) {
    return (
      <section className="continue-evidence-panel empty" aria-label="Why this continuation">
        <div className="continue-evidence-head">
          <div>
            <p className="product-kicker">Why this continuation?</p>
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
    <section className="continue-evidence-panel" aria-label="Why this continuation">
      <div className="continue-evidence-head">
        <div>
          <p className="product-kicker">Why this continuation?</p>
          <h2>{presentation?.workstreamTitle || humanTargetLabel(target) || "Selected workstream"}</h2>
        </div>
        <button className="secondary-button" type="button" onClick={onClose}>
          Close
        </button>
      </div>

      <div className="continue-evidence-grid">
        <dl className="continue-evidence-facts">
          <div>
            <dt>Why this workstream</dt>
            <dd>{presentation?.decisionReason || "Selected from local evidence."}</dd>
          </div>
          <div>
            <dt>Return target</dt>
            <dd>{presentation?.returnTarget || "No return target returned."}</dd>
          </div>
          <div>
            <dt>Current focus</dt>
            <dd>{presentation?.currentFocus || "No current focus returned."}</dd>
          </div>
          <div>
            <dt>Last meaningful action</dt>
            <dd>{presentation?.lastState || "No action returned."}</dd>
          </div>
          <div>
            <dt>What is unresolved</dt>
            <dd>{presentation?.unresolvedState || "No unresolved state returned."}</dd>
          </div>
          <div>
            <dt>Missing evidence</dt>
            <dd>{presentation?.missingEvidenceSummary || "No missing evidence called out."}</dd>
          </div>
          <div>
            <dt>How this was chosen</dt>
            <dd>{provenanceLabel}</dd>
          </div>
        </dl>

        <div className="anchor-preview">
          <div className="anchor-preview-head">
            <strong>Evidence anchor</strong>
            <span>{evidenceAnchorLabel(selectedFrame)}</span>
          </div>
          {selectedFrame && imageData ? (
            <div className="anchor-image" style={stageStyle(selectedFrame)}>
              <img src={imageData} alt="Evidence preview" />
            </div>
          ) : (
            <div className="anchor-empty">
              <strong>No preview loaded</strong>
              <span>{selectedFrame ? evidenceAnchorLabel(selectedFrame) : "No evidence preview is selected."}</span>
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

function companionToneForStatus(status: MemoryProductStatus, freshness?: ContinueFreshness) {
  if (freshness === "updating") return "updating";
  if (freshness === "new_evidence") return "noticed";
  if (freshness === "current") return "ready";
  if (freshness === "thin_evidence") return "thin";
  if (status === "on" || status === "starting") return "active";
  if (status === "paused_with_evidence") return "paused";
  if (status === "private_or_excluded") return "private";
  if (status === "needs_attention" || status === "needs_permission") return "attention";
  return "quiet";
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
  return error.includes("permission") ||
    error.includes("screen access") ||
    error.includes("accessibility") ||
    error.includes("not authorized") ||
    error.includes("denied");
}

function productizeMemoryError(value: string) {
  const error = value.toLowerCase();
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
  return value || "Local memory needs attention.";
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

function continueProvenanceLabel(decision: ContinueDecisionResult) {
  if (decision.source === "cloud_micro_inference" && decision.response_id) {
    return "AI-assisted";
  }
  if (decision.source === "local_fallback") {
    return "Local fallback";
  }
  return "Local only";
}

function continueProvenanceTone(decision: ContinueDecisionResult) {
  if (decision.source === "cloud_micro_inference" && decision.response_id) {
    return "ai";
  }
  if (decision.source === "local_fallback") {
    return "fallback";
  }
  return "local";
}

function humanTargetLabel(target?: ContinueReturnTarget | null) {
  if (!target) return "";
  return cleanHumanText(target.title)
    || pathBasename(target.document_path || target.browser_url)
    || productizeArtifactKind(target.artifact_kind)
    || "";
}

function humanTargetMeta(target?: ContinueReturnTarget | null) {
  if (!target) return "I don't have a reliable app or page target for this yet.";
  const parts = [
    productizeArtifactKind(target.artifact_kind),
    productizeOpenability(target.openability),
  ].filter(Boolean);
  return parts.join(" / ") || "I don't have a reliable app or page target for this yet.";
}

function isDirectResumeTargetOpenable(target?: ContinueReturnTarget | null) {
  return Boolean(
    target &&
      normalizeToken(target.openability) === "openable" &&
      (target.browser_url || target.document_path),
  );
}

function getContinueCardActionState(decision: ContinueDecisionResult): ContinueCardActionState {
  const target = decision.resume_work_target || decision.return_target || null;
  const hasOpenableReturnTarget = isDirectResumeTargetOpenable(target);
  if (hasOpenableReturnTarget && !decisionReturnTargetIsSupportEvidence(decision)) {
    return { kind: "openable_return_target", label: "Continue here" };
  }

  const evidenceNotes = continueDecisionEvidenceNotes(decision);
  const unresolvedCurrentWork = decision.active_current_work_unresolved;
  const hasThinCurrentWork =
    Boolean(unresolvedCurrentWork && !unresolvedCurrentWork.has_openable_target) ||
    evidenceNotes.includes("stale_return_target_suppressed:newer_current_focus") ||
    (
      normalizeToken(decision.candidate_kind) === "continue_current_work" &&
      !hasOpenableReturnTarget
    ) ||
    evidenceNotes.includes("thin_evidence:no_human_return_target");

  if (hasThinCurrentWork) {
    return { kind: "thin_current_work", label: "Inspect latest evidence" };
  }

  return { kind: "no_clear_continuation", label: "Inspect evidence" };
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

function supportEvidenceProductLines(decision: ContinueDecisionResult) {
  return [...new Set((decision.support_evidence || [])
    .filter((item) => !item.public_return_eligible)
    .map(supportEvidenceProductLine)
    .filter(Boolean))];
}

function supportEvidenceProductLine(item: ContinueSupportEvidenceItem) {
  const title = cleanHumanText(item.title || "") || productizeArtifactKind(item.artifact_kind);
  const role = productizeSupportEvidenceRole(item.branch_kind || item.role);
  const reason = productizeInternalLabel(item.reason) || productizeSupportEvidenceRole(item.reason);
  if (title && role) return `${title}: ${role}`;
  if (role) return role;
  if (reason) return reason;
  return "Supporting evidence, not the return target.";
}

function productizeSupportEvidenceRole(value?: string | null) {
  const key = normalizeToken(value);
  const labels: Record<string, string> = {
    support_context: "Supporting evidence, not the return target.",
    support_evidence: "Supporting evidence, not the return target.",
    search_branch: "Search evidence, not the return target.",
    documentation_reference: "Reference material for the task.",
    source_evidence: "Source evidence for the task.",
    message_interrupt: "Interruption context, not the unfinished task.",
    interruption: "Interruption context, not the unfinished task.",
    diagnostic_only: "Diagnostic evidence, not the return target.",
    current_focus_only: "Current focus only; no safe return target yet.",
    branch_search_without_origin: "Search branch has no grounded origin target.",
    branch_no_origin: "Support branch has no grounded origin target.",
  };
  return labels[key] || "";
}

function productEvidenceLines(decision: ContinueDecisionResult) {
  return [...new Set(
    continueDecisionEvidenceNotes(decision)
      .map(productizeInternalLabel)
      .filter(Boolean),
  )];
}

function humanFocusLabel(focus?: ContinueFocusSummary | null) {
  if (!focus) return "No current focus returned.";
  return cleanHumanText(focus.title || focus.window_title || focus.app_name)
    || productizeArtifactKind(focus.artifact_kind)
    || "Current focus";
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

function stripCurrentFocusPrefix(value: string) {
  return value.replace(/^current focus:\s*/i, "").trim();
}

function isInternalFacingText(value?: string | null) {
  const lower = (value || "").toLowerCase();
  if (!lower) return false;
  if (
    lower.includes("continue-candidate-") ||
    lower.includes("continue-decision-") ||
    lower.includes("workstream-") ||
    lower.includes("artifact-") ||
    lower.includes("action-") ||
    lower.includes("task-action-") ||
    lower.includes("frame-fallback") ||
    lower.includes("frame_fallback") ||
    lower.includes("frame fallback") ||
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
