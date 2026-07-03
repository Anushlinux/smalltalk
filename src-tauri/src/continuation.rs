#![allow(dead_code)]

use rusqlite::{params, Connection, OptionalExtension, Row};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;

pub const CONTINUE_SCHEMA_NAME: &str = "smalltalk.continue_memory.v1";
pub const CONTINUE_SCHEMA_VERSION: i64 = 1;

const CONTINUE_TABLES: &[&str] = &[
    "continue_schema_migrations",
    "continue_artifacts",
    "continue_artifact_observations",
    "continue_task_actions",
    "continue_task_action_events",
    "continue_episodes",
    "continue_episode_actions",
    "continue_episode_artifacts",
    "continue_workstreams",
    "continue_workstream_episodes",
    "continue_workstream_artifacts",
    "continue_candidates",
    "continue_decisions",
    "continue_feedback_events",
    "continue_breadcrumbs",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContinueArtifactKind {
    BrowserTab,
    ChatConversation,
    CodeEditor,
    Terminal,
    Pdf,
    Finder,
    Messaging,
    NotesDoc,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContinueEvidenceQuality {
    Strong,
    Medium,
    Thin,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContinueOpenability {
    Openable,
    FrameFallback,
    Blocked,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContinueTextSource {
    Accessibility,
    Ocr,
    Hybrid,
    Missing,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContinueActionKind {
    Reading,
    Editing,
    Composing,
    Searching,
    CopyingEvidence,
    ReviewingOutput,
    RunningCommand,
    ObservingCommandOutput,
    EncounteringError,
    Navigating,
    SwitchingContext,
    BranchingAway,
    ReturningToOrigin,
    IdleAfterProgress,
    MessagingInterrupt,
    VerificationBranch,
    PossibleDistraction,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContinueActionRole {
    Primary,
    Support,
    Branch,
    Return,
    Interrupt,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContinueEpisodeState {
    Open,
    Closed,
    Merged,
    Discarded,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContinueArtifactRole {
    PrimaryTarget,
    SourceEvidence,
    BranchSupport,
    OutputVerification,
    Blocker,
    Interruption,
    CurrentFocusOnly,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContinueWorkstreamState {
    Active,
    Suspended,
    Resumed,
    Background,
    Stale,
    Abandoned,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContinueWorkstreamSource {
    LocalHeuristic,
    MicroInference,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContinueCandidateKind {
    ContinueEdit,
    ReturnToPrimaryArtifact,
    ResolveError,
    VerifyOutput,
    ContinueReply,
    ReadNextSource,
    FinishSearch,
    RerunCommand,
    ResumeChatReasoning,
    EvidenceOnly,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContinueDecisionSource {
    LocalScorer,
    CloudMicroInference,
    LocalFallback,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContinueValidationStatus {
    Valid,
    Fallback,
    Rejected,
    ThinEvidence,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContinueArtifact {
    pub id: String,
    pub artifact_kind: ContinueArtifactKind,
    pub stable_key: String,
    pub app_name: Option<String>,
    pub bundle_id: Option<String>,
    pub window_title: Option<String>,
    pub browser_url: Option<String>,
    pub document_path: Option<String>,
    pub display_title: Option<String>,
    pub first_seen_frame_id: Option<String>,
    pub last_seen_frame_id: Option<String>,
    pub first_seen_timestamp: i64,
    pub last_seen_timestamp: i64,
    pub identity_confidence: f64,
    pub evidence_quality: ContinueEvidenceQuality,
    pub privacy_status: Option<String>,
    pub openability: ContinueOpenability,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContinueArtifactObservation {
    pub id: String,
    pub artifact_id: String,
    pub frame_id: String,
    pub app_context_id: Option<String>,
    pub text_source: ContinueTextSource,
    pub content_hash: Option<String>,
    pub image_hash: Option<String>,
    pub focused_node_evidence: bool,
    pub selected_text_present: bool,
    pub visible_text_length: i64,
    pub observation_confidence: f64,
    pub reason: Option<String>,
    pub timestamp_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContinueTaskAction {
    pub id: String,
    pub frame_id: String,
    pub previous_frame_id: Option<String>,
    pub artifact_id: Option<String>,
    pub secondary_artifact_id: Option<String>,
    pub action_kind: ContinueActionKind,
    pub action_role: ContinueActionRole,
    pub trigger_type: Option<String>,
    pub transition_label: Option<String>,
    pub evidence_event_ids: Vec<String>,
    pub confidence: f64,
    pub reason: Option<String>,
    pub created_at_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContinueEpisode {
    pub id: String,
    pub state: ContinueEpisodeState,
    pub start_frame_id: Option<String>,
    pub end_frame_id: Option<String>,
    pub start_timestamp_ms: i64,
    pub end_timestamp_ms: Option<i64>,
    pub primary_artifact_id: Option<String>,
    pub dominant_action_kind: Option<ContinueActionKind>,
    pub boundary_start_reason: Option<String>,
    pub boundary_end_reason: Option<String>,
    pub confidence: f64,
    pub evidence_quality: ContinueEvidenceQuality,
    pub summary_label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContinueWorkstream {
    pub id: String,
    pub state: ContinueWorkstreamState,
    pub title_candidate: Option<String>,
    pub inferred_intent: Option<String>,
    pub primary_artifact_id: Option<String>,
    pub created_at_ms: i64,
    pub last_active_timestamp_ms: i64,
    pub suspended_timestamp_ms: Option<i64>,
    pub confidence: f64,
    pub unresolved_signal: Option<String>,
    pub source: ContinueWorkstreamSource,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContinueCandidate {
    pub id: String,
    pub workstream_id: String,
    pub target_artifact_id: Option<String>,
    pub candidate_kind: ContinueCandidateKind,
    pub last_meaningful_action_id: Option<String>,
    pub evidence_frame_id: Option<String>,
    pub supporting_episode_id: Option<String>,
    pub score: f64,
    pub actionability_score: f64,
    pub primary_target_score: f64,
    pub unresolved_score: f64,
    pub branch_origin_score: f64,
    pub evidence_quality_score: f64,
    pub recency_score: f64,
    pub openability_score: f64,
    pub privacy_safety_score: f64,
    pub reason: Option<String>,
    pub missing_evidence: Option<String>,
    pub created_at_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContinueDecision {
    pub id: String,
    pub requested_at_ms: i64,
    pub source: ContinueDecisionSource,
    pub current_focus_frame_id: Option<String>,
    pub current_focus_artifact_id: Option<String>,
    pub selected_workstream_id: Option<String>,
    pub selected_candidate_id: Option<String>,
    pub return_target_artifact_id: Option<String>,
    pub confidence: f64,
    pub decision_reason: Option<String>,
    pub next_action: Option<String>,
    pub warnings: Option<String>,
    pub validation_status: ContinueValidationStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContinueMemoryCounts {
    pub artifacts: i64,
    pub artifact_observations: i64,
    pub task_actions: i64,
    pub task_action_events: i64,
    pub episodes: i64,
    pub episode_actions: i64,
    pub episode_artifacts: i64,
    pub workstreams: i64,
    pub workstream_episodes: i64,
    pub workstream_artifacts: i64,
    pub candidates: i64,
    pub decisions: i64,
    pub feedback_events: i64,
    pub breadcrumbs: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContinueMemoryStatus {
    pub schema: String,
    pub schema_version: Option<i64>,
    pub has_schema: bool,
    pub counts: ContinueMemoryCounts,
    pub latest_artifact_timestamp: Option<i64>,
    pub latest_workstream_timestamp: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ContinueSecondLayerRebuildRequest {
    pub session_id: Option<String>,
    pub lookback_ms: Option<i64>,
    pub start_frame_id: Option<i64>,
    pub end_frame_id: Option<i64>,
    pub limit: Option<i64>,
}

impl Default for ContinueSecondLayerRebuildRequest {
    fn default() -> Self {
        Self {
            session_id: None,
            lookback_ms: None,
            start_frame_id: None,
            end_frame_id: None,
            limit: Some(300),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ContinueSecondLayerRebuildResult {
    pub processed_frames: i64,
    pub artifact_count: i64,
    pub observation_count: i64,
    pub task_action_count: i64,
    pub start_frame_id: Option<String>,
    pub end_frame_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ContinueThirdLayerRebuildRequest {
    pub session_id: Option<String>,
    pub lookback_ms: Option<i64>,
    pub start_frame_id: Option<i64>,
    pub end_frame_id: Option<i64>,
    pub limit: Option<i64>,
}

impl Default for ContinueThirdLayerRebuildRequest {
    fn default() -> Self {
        Self {
            session_id: None,
            lookback_ms: None,
            start_frame_id: None,
            end_frame_id: None,
            limit: Some(500),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ContinueThirdLayerRebuildResult {
    pub processed_actions: i64,
    pub episode_count: i64,
    pub episode_action_count: i64,
    pub episode_artifact_count: i64,
    pub workstream_count: i64,
    pub workstream_episode_count: i64,
    pub workstream_artifact_count: i64,
    pub start_frame_id: Option<String>,
    pub end_frame_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ContinueDecisionRequest {
    pub session_id: Option<String>,
    pub lookback_ms: Option<i64>,
    pub limit: Option<i64>,
    pub mode: Option<String>,
    pub rebuild_layers: Option<bool>,
    pub micro_inference_enabled: Option<bool>,
    pub model: Option<String>,
    pub max_candidates_for_model: Option<i64>,
}

impl Default for ContinueDecisionRequest {
    fn default() -> Self {
        Self {
            session_id: None,
            lookback_ms: Some(45 * 60 * 1000),
            limit: Some(700),
            mode: Some("normal".to_string()),
            rebuild_layers: Some(false),
            micro_inference_enabled: Some(false),
            model: None,
            max_candidates_for_model: Some(5),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ContinueDecisionResult {
    pub decision_id: String,
    pub mode: String,
    pub cache_hit: bool,
    pub source: String,
    pub model: Option<String>,
    pub response_id: Option<String>,
    pub current_focus: Option<ContinueFocusSummary>,
    pub current_activity: Option<String>,
    pub selected_workstream: Option<ContinueSelectedWorkstream>,
    pub return_target: Option<ContinueReturnTarget>,
    pub resume_work_target: Option<ContinueReturnTarget>,
    pub candidate_kind: Option<String>,
    pub last_meaningful_action: Option<ContinueActionSummary>,
    pub unresolved_state: Option<String>,
    pub next_action: Option<String>,
    pub confidence: f64,
    pub confidence_label: String,
    pub evidence_anchors: ContinueEvidenceAnchors,
    pub missing_evidence: Vec<String>,
    pub warnings: Vec<String>,
    pub validation_failures: Vec<String>,
    pub alternatives: Vec<ContinueCandidateSummary>,
    pub generated_candidates: i64,
    pub validation_status: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ContinueFeedbackRequest {
    pub decision_id: Option<String>,
    pub observation_window_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContinueFeedbackEventResult {
    pub id: String,
    pub decision_id: Option<String>,
    pub selected_candidate_id: Option<String>,
    pub workstream_id: Option<String>,
    pub event_kind: String,
    pub observed_frame_id: Option<String>,
    pub target_artifact_id: Option<String>,
    pub chosen_artifact_id: Option<String>,
    pub timestamp_ms: i64,
    pub confidence: f64,
    pub reason: Option<String>,
    pub note: Option<String>,
    pub source: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ContinueExplicitFeedbackRequest {
    pub decision_id: Option<String>,
    pub selected_candidate_id: Option<String>,
    pub workstream_id: Option<String>,
    pub target_artifact_id: Option<String>,
    pub corrected_artifact_id: Option<String>,
    pub feedback_kind: String,
    pub note: Option<String>,
    pub source: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ContinueBreadcrumbRequest {
    pub workstream_id: String,
    pub text: String,
    pub source: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContinueBreadcrumbResult {
    pub id: String,
    pub workstream_id: String,
    pub text: String,
    pub source: String,
    pub created_at_ms: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContinueEvalReport {
    pub schema: String,
    pub evaluated_at_ms: i64,
    pub case_count: i64,
    pub target_artifact_correct: i64,
    pub target_artifact_accuracy: f64,
    pub recall_at_k: f64,
    pub mrr: f64,
    pub current_focus_false_positive_rate: f64,
    pub hallucinated_artifact_count: i64,
    pub model_validation_fallback_rate: f64,
    pub cases: Vec<ContinueEvalCaseReport>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContinueEvalCaseReport {
    pub name: String,
    pub scenario: String,
    pub expected_target_artifact_id: String,
    pub selected_candidate_id: Option<String>,
    pub selected_target_artifact_id: Option<String>,
    pub target_artifact_correct: bool,
    pub correct_candidate_rank: Option<i64>,
    pub recall_at_k: bool,
    pub reciprocal_rank: f64,
    pub current_focus_false_positive: bool,
    pub hallucinated_artifact_count: i64,
    pub validation_status: String,
    pub validation_failures: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ContinueWorkstreamDetailRequest {
    pub workstream_id: String,
    pub decision_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContinueWorkstreamArtifactDetail {
    pub artifact_id: String,
    pub durable_role: String,
    pub artifact_kind: String,
    pub display_title: Option<String>,
    pub stable_key: Option<String>,
    pub app_name: Option<String>,
    pub window_title: Option<String>,
    pub browser_url: Option<String>,
    pub document_path: Option<String>,
    pub openability: String,
    pub evidence_quality: String,
    pub privacy_status: Option<String>,
    pub importance_score: f64,
    pub first_seen_frame_id: Option<String>,
    pub last_seen_frame_id: Option<String>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContinueWorkstreamActionDetail {
    pub action_id: String,
    pub frame_id: String,
    pub previous_frame_id: Option<String>,
    pub artifact_id: Option<String>,
    pub artifact_title: Option<String>,
    pub secondary_artifact_id: Option<String>,
    pub secondary_artifact_title: Option<String>,
    pub action_kind: String,
    pub action_role: String,
    pub role_in_episode: String,
    pub order_index: i64,
    pub trigger_type: Option<String>,
    pub transition_label: Option<String>,
    pub evidence_event_ids: Vec<String>,
    pub confidence: f64,
    pub reason: Option<String>,
    pub created_at_ms: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContinueWorkstreamEpisodeDetail {
    pub id: String,
    pub state: String,
    pub start_frame_id: Option<String>,
    pub end_frame_id: Option<String>,
    pub start_timestamp_ms: i64,
    pub end_timestamp_ms: Option<i64>,
    pub primary_artifact_id: Option<String>,
    pub primary_artifact_title: Option<String>,
    pub dominant_action_kind: Option<String>,
    pub boundary_start_reason: Option<String>,
    pub boundary_end_reason: Option<String>,
    pub evidence_quality: String,
    pub confidence: f64,
    pub summary_label: Option<String>,
    pub membership_score: f64,
    pub membership_reason: Option<String>,
    pub actions: Vec<ContinueWorkstreamActionDetail>,
    pub artifacts: Vec<RecentContinueEpisodeArtifact>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContinueWorkstreamCandidateDetail {
    pub candidate_id: String,
    pub workstream_id: String,
    pub target_artifact_id: Option<String>,
    pub target_title: Option<String>,
    pub target_kind: Option<String>,
    pub target_openability: Option<String>,
    pub candidate_kind: String,
    pub last_meaningful_action_id: Option<String>,
    pub evidence_frame_id: Option<String>,
    pub supporting_episode_id: Option<String>,
    pub score: f64,
    pub confidence_label: String,
    pub reason: Option<String>,
    pub missing_evidence: Vec<String>,
    pub components: ContinueScoreComponents,
    pub created_at_ms: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContinueDecisionSummary {
    pub decision_id: String,
    pub requested_at_ms: i64,
    pub source: String,
    pub selected_candidate_id: Option<String>,
    pub return_target_artifact_id: Option<String>,
    pub confidence: f64,
    pub decision_reason: Option<String>,
    pub next_action: Option<String>,
    pub warnings: Vec<String>,
    pub validation_status: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContinueBreadcrumbSummary {
    pub id: String,
    pub workstream_id: String,
    pub text: String,
    pub source: String,
    pub created_at_ms: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContinueWorkstreamDetailResult {
    pub workstream: RecentContinueWorkstream,
    pub artifacts: Vec<ContinueWorkstreamArtifactDetail>,
    pub episodes: Vec<ContinueWorkstreamEpisodeDetail>,
    pub candidates: Vec<ContinueWorkstreamCandidateDetail>,
    pub latest_decision: Option<ContinueDecisionSummary>,
    pub feedback_events: Vec<ContinueFeedbackEventResult>,
    pub breadcrumbs: Vec<ContinueBreadcrumbSummary>,
    pub evidence_anchors: ContinueEvidenceAnchors,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContinueFocusSummary {
    pub frame_id: String,
    pub artifact_id: Option<String>,
    pub artifact_kind: Option<String>,
    pub app_name: Option<String>,
    pub window_title: Option<String>,
    pub title: Option<String>,
    pub browser_url: Option<String>,
    pub document_path: Option<String>,
    pub captured_at_ms: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContinueSelectedWorkstream {
    pub workstream_id: String,
    pub state: String,
    pub title_candidate: Option<String>,
    pub primary_artifact_id: Option<String>,
    pub last_active_timestamp_ms: i64,
    pub unresolved_signal: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContinueReturnTarget {
    pub artifact_id: Option<String>,
    pub artifact_kind: Option<String>,
    pub title: Option<String>,
    pub browser_url: Option<String>,
    pub document_path: Option<String>,
    pub openability: String,
    pub fallback_frame_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContinueActionSummary {
    pub action_id: String,
    pub action_kind: String,
    pub action_role: String,
    pub timestamp_ms: i64,
    pub evidence_frame_id: String,
    pub artifact_id: Option<String>,
    pub collapse_count: i64,
    pub first_frame_id: Option<String>,
    pub last_frame_id: Option<String>,
    pub strongest_frame_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct ContinueEvidenceAnchors {
    pub frame_ids: Vec<String>,
    pub action_ids: Vec<String>,
    pub episode_ids: Vec<String>,
    pub artifact_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContinueScoreComponents {
    pub actionability: f64,
    pub primary_target: f64,
    pub unresolved_state: f64,
    pub branch_origin: f64,
    pub evidence_quality: f64,
    pub recency: f64,
    pub openability: f64,
    pub privacy_safety: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContinueCandidateSummary {
    pub candidate_id: String,
    pub workstream_id: String,
    pub target_artifact_id: Option<String>,
    pub candidate_kind: String,
    pub score: f64,
    pub confidence_label: String,
    pub reason: Option<String>,
    pub missing_evidence: Vec<String>,
    pub evidence_frame_id: Option<String>,
    pub supporting_episode_id: Option<String>,
    pub last_meaningful_action_id: Option<String>,
    pub components: ContinueScoreComponents,
}

#[derive(Debug, Clone, Serialize)]
pub struct RecentContinueArtifact {
    pub id: String,
    pub artifact_kind: String,
    pub stable_key: String,
    pub app_name: Option<String>,
    pub window_title: Option<String>,
    pub browser_url: Option<String>,
    pub document_path: Option<String>,
    pub display_title: Option<String>,
    pub first_seen_frame_id: Option<String>,
    pub last_seen_frame_id: Option<String>,
    pub last_seen_timestamp: i64,
    pub identity_confidence: f64,
    pub evidence_quality: String,
    pub openability: String,
    pub observation_count: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct RecentContinueTaskAction {
    pub id: String,
    pub frame_id: String,
    pub previous_frame_id: Option<String>,
    pub artifact_id: Option<String>,
    pub secondary_artifact_id: Option<String>,
    pub action_kind: String,
    pub action_role: String,
    pub trigger_type: Option<String>,
    pub transition_label: Option<String>,
    pub evidence_event_ids: Vec<String>,
    pub confidence: f64,
    pub reason: Option<String>,
    pub created_at_ms: i64,
    pub collapse_count: i64,
    pub first_frame_id: Option<String>,
    pub last_frame_id: Option<String>,
    pub strongest_frame_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RecentContinueEpisodeAction {
    pub action_id: String,
    pub frame_id: String,
    pub action_kind: String,
    pub role_in_episode: String,
    pub order_index: i64,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct RecentContinueEpisodeArtifact {
    pub artifact_id: String,
    pub artifact_role: String,
    pub display_title: Option<String>,
    pub stable_key: Option<String>,
    pub first_frame_id: Option<String>,
    pub last_frame_id: Option<String>,
    pub contribution_score: f64,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RecentContinueEpisode {
    pub id: String,
    pub state: String,
    pub start_frame_id: Option<String>,
    pub end_frame_id: Option<String>,
    pub start_timestamp_ms: i64,
    pub end_timestamp_ms: Option<i64>,
    pub primary_artifact_id: Option<String>,
    pub primary_artifact_title: Option<String>,
    pub dominant_action_kind: Option<String>,
    pub boundary_start_reason: Option<String>,
    pub boundary_end_reason: Option<String>,
    pub evidence_quality: String,
    pub confidence: f64,
    pub summary_label: Option<String>,
    pub actions: Vec<RecentContinueEpisodeAction>,
    pub artifacts: Vec<RecentContinueEpisodeArtifact>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RecentContinueWorkstreamEpisode {
    pub episode_id: String,
    pub membership_score: f64,
    pub membership_reason: Option<String>,
    pub order_index: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct RecentContinueWorkstreamArtifact {
    pub artifact_id: String,
    pub durable_role: String,
    pub display_title: Option<String>,
    pub stable_key: Option<String>,
    pub importance_score: f64,
    pub first_seen_frame_id: Option<String>,
    pub last_seen_frame_id: Option<String>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RecentContinueWorkstream {
    pub id: String,
    pub state: String,
    pub title_candidate: Option<String>,
    pub primary_artifact_id: Option<String>,
    pub primary_artifact_title: Option<String>,
    pub created_at_ms: i64,
    pub last_active_timestamp_ms: i64,
    pub suspended_timestamp_ms: Option<i64>,
    pub confidence: f64,
    pub unresolved_signal: Option<String>,
    pub source: String,
    pub episodes: Vec<RecentContinueWorkstreamEpisode>,
    pub artifacts: Vec<RecentContinueWorkstreamArtifact>,
}

#[derive(Debug, Clone)]
struct EvidenceFrame {
    id: String,
    captured_at: i64,
    app_name: Option<String>,
    window_name: Option<String>,
    browser_url: Option<String>,
    document_path: Option<String>,
    capture_trigger: String,
    text_source: Option<String>,
    full_text: Option<String>,
    content_hash: Option<String>,
    image_hash: Option<String>,
    privacy_status: Option<String>,
    app_bundle_id: Option<String>,
    previous_frame_id: Option<String>,
    session_id: Option<String>,
    app_contexts: Vec<EvidenceAppContext>,
    content_units: Vec<EvidenceContentUnit>,
    ui_events: Vec<EvidenceUiEvent>,
    trigger: Option<EvidenceTrigger>,
    transition: Option<EvidenceTransition>,
    frame_diff: Option<EvidenceFrameDiff>,
    typing_bursts: Vec<EvidenceTypingBurst>,
    clipboard_events: Vec<EvidenceClipboardEvent>,
    focused_node_evidence: bool,
    selected_text_present: bool,
}

#[derive(Debug, Clone)]
struct EvidenceAppContext {
    id: String,
    adapter_id: String,
    object_type: String,
    primary_id: Option<String>,
    title: Option<String>,
    url: Option<String>,
    file_path: Option<String>,
    repo_path: Option<String>,
    selected_text: Option<String>,
    focused_object: Option<String>,
    confidence: Option<f64>,
}

#[derive(Debug, Clone)]
struct EvidenceContentUnit {
    id: String,
    source: String,
    unit_type: String,
    semantic_role: Option<String>,
    text: Option<String>,
    text_hash: Option<String>,
    confidence: Option<f64>,
}

#[derive(Debug, Clone)]
struct EvidenceUiEvent {
    id: String,
    event_type: String,
    key_category: Option<String>,
}

#[derive(Debug, Clone)]
struct EvidenceTrigger {
    id: String,
    trigger_type: String,
    caused_by_event_ids: Vec<String>,
}

#[derive(Debug, Clone)]
struct EvidenceTransition {
    id: String,
    primary_event_id: Option<String>,
    transition_type: Option<String>,
    summary: Option<String>,
    confidence: Option<f64>,
}

#[derive(Debug, Clone)]
struct EvidenceFrameDiff {
    diff_type: Option<String>,
    added_text_hashes: Option<String>,
    removed_text_hashes: Option<String>,
    summary: Option<String>,
}

#[derive(Debug, Clone)]
struct EvidenceTypingBurst {
    id: String,
    enter_count: i64,
    paste_count: i64,
    committed: bool,
    commit_signal: Option<String>,
}

#[derive(Debug, Clone)]
struct EvidenceClipboardEvent {
    id: String,
    source_frame_id: Option<String>,
    target_frame_id: Option<String>,
}

#[derive(Debug, Clone)]
struct ResolvedArtifact {
    id: String,
    kind: String,
    stable_key: String,
    display_title: Option<String>,
    browser_url: Option<String>,
    document_path: Option<String>,
    identity_confidence: f64,
    evidence_quality: String,
    openability: String,
    reason: String,
}

#[derive(Debug, Clone)]
struct ExtractedTaskAction {
    id: String,
    frame_id: String,
    previous_frame_id: Option<String>,
    artifact_id: Option<String>,
    secondary_artifact_id: Option<String>,
    action_kind: String,
    action_role: String,
    trigger_type: Option<String>,
    transition_label: Option<String>,
    evidence_event_ids: Vec<String>,
    confidence: f64,
    reason: String,
    created_at_ms: i64,
    collapse_count: i64,
    first_frame_id: Option<String>,
    last_frame_id: Option<String>,
    strongest_frame_id: Option<String>,
}

#[derive(Debug, Clone)]
struct ContinueActionRecord {
    id: String,
    frame_id: String,
    previous_frame_id: Option<String>,
    artifact_id: Option<String>,
    secondary_artifact_id: Option<String>,
    action_kind: String,
    action_role: String,
    confidence: f64,
    reason: Option<String>,
    created_at_ms: i64,
    collapse_count: i64,
    first_frame_id: Option<String>,
    last_frame_id: Option<String>,
    strongest_frame_id: Option<String>,
    artifact: Option<ContinueArtifactRecord>,
    secondary_artifact: Option<ContinueArtifactRecord>,
}

#[derive(Debug, Clone)]
struct ContinueArtifactRecord {
    id: String,
    artifact_kind: String,
    stable_key: String,
    app_name: Option<String>,
    display_title: Option<String>,
    browser_url: Option<String>,
    document_path: Option<String>,
    evidence_quality: String,
}

#[derive(Debug, Clone)]
struct BuiltEpisode {
    id: String,
    actions: Vec<ContinueActionRecord>,
    artifacts: HashMap<String, BuiltArtifactRole>,
    state: String,
    start_frame_id: Option<String>,
    end_frame_id: Option<String>,
    start_timestamp_ms: i64,
    end_timestamp_ms: i64,
    primary_artifact_id: Option<String>,
    dominant_action_kind: Option<String>,
    boundary_start_reason: String,
    boundary_end_reason: Option<String>,
    confidence: f64,
    evidence_quality: String,
    summary_label: String,
}

#[derive(Debug, Clone)]
struct BuiltArtifactRole {
    artifact: ContinueArtifactRecord,
    role: String,
    first_frame_id: Option<String>,
    last_frame_id: Option<String>,
    contribution_score: f64,
    reason: String,
}

#[derive(Debug, Clone)]
struct BuiltWorkstream {
    id: String,
    episodes: Vec<(BuiltEpisode, f64, String)>,
    artifacts: HashMap<String, BuiltWorkstreamArtifact>,
    state: String,
    title_candidate: Option<String>,
    primary_artifact_id: Option<String>,
    created_at_ms: i64,
    last_active_timestamp_ms: i64,
    suspended_timestamp_ms: Option<i64>,
    confidence: f64,
    unresolved_signal: Option<String>,
    source: String,
}

#[derive(Debug, Clone)]
struct BuiltWorkstreamArtifact {
    artifact: ContinueArtifactRecord,
    durable_role: String,
    importance_score: f64,
    first_seen_frame_id: Option<String>,
    last_seen_frame_id: Option<String>,
    reason: String,
}

#[derive(Debug, Clone)]
struct ScorerArtifact {
    id: String,
    artifact_kind: String,
    display_title: Option<String>,
    browser_url: Option<String>,
    document_path: Option<String>,
    evidence_quality: String,
    privacy_status: Option<String>,
    openability: String,
    last_seen_frame_id: Option<String>,
    last_seen_timestamp: i64,
}

#[derive(Debug, Clone)]
struct ScorerWorkstreamArtifact {
    artifact: ScorerArtifact,
    durable_role: String,
    importance_score: f64,
    first_seen_frame_id: Option<String>,
    last_seen_frame_id: Option<String>,
}

#[derive(Debug, Clone)]
struct ScorerEpisode {
    id: String,
    end_frame_id: Option<String>,
    end_timestamp_ms: i64,
    dominant_action_kind: Option<String>,
    evidence_quality: String,
}

#[derive(Debug, Clone)]
struct ScorerAction {
    id: String,
    frame_id: String,
    artifact_id: Option<String>,
    secondary_artifact_id: Option<String>,
    action_kind: String,
    action_role: String,
    confidence: f64,
    reason: Option<String>,
    created_at_ms: i64,
    collapse_count: i64,
    first_frame_id: Option<String>,
    last_frame_id: Option<String>,
    strongest_frame_id: Option<String>,
}

#[derive(Debug, Clone)]
struct ScorerWorkstream {
    id: String,
    state: String,
    title_candidate: Option<String>,
    primary_artifact_id: Option<String>,
    last_active_timestamp_ms: i64,
    confidence: f64,
    unresolved_signal: Option<String>,
    episodes: Vec<ScorerEpisode>,
    artifacts: Vec<ScorerWorkstreamArtifact>,
    last_meaningful_action: Option<ScorerAction>,
}

#[derive(Debug, Clone)]
struct ScoredContinueCandidate {
    id: String,
    workstream_id: String,
    target_artifact: Option<ScorerArtifact>,
    candidate_kind: String,
    last_meaningful_action: Option<ScorerAction>,
    evidence_frame_id: Option<String>,
    supporting_episode_id: Option<String>,
    score: f64,
    actionability_score: f64,
    primary_target_score: f64,
    unresolved_score: f64,
    branch_origin_score: f64,
    evidence_quality_score: f64,
    recency_score: f64,
    openability_score: f64,
    privacy_safety_score: f64,
    reason: Option<String>,
    missing_evidence: Vec<String>,
    warnings: Vec<String>,
    resume_work_target: Option<ScorerArtifact>,
}

#[derive(Debug, Clone, Serialize)]
struct ContinueMicroInferencePack {
    schema: String,
    instructions: String,
    current_focus: Option<ContinueFocusSummary>,
    workstreams: Vec<ContinuePackWorkstream>,
    candidates: Vec<ContinuePackCandidate>,
    artifact_roles: Vec<ContinuePackArtifactRole>,
    breadcrumbs: Vec<ContinuePackBreadcrumb>,
}

#[derive(Debug, Clone, Serialize)]
struct ContinuePackWorkstream {
    id: String,
    title_candidate: Option<String>,
    state: String,
    primary_artifact_id: Option<String>,
    primary_artifact_title: Option<String>,
    unresolved_signal: Option<String>,
    confidence: f64,
}

#[derive(Debug, Clone, Serialize)]
struct ContinuePackCandidate {
    id: String,
    workstream_id: String,
    candidate_kind: String,
    target_artifact_id: Option<String>,
    target_title: Option<String>,
    target_kind: Option<String>,
    target_url_available: bool,
    target_path_available: bool,
    local_score: f64,
    score_components: ContinueScoreComponents,
    last_meaningful_action: Option<ContinueActionSummary>,
    unresolved_state_reason: Option<String>,
    evidence_frame_id: Option<String>,
    evidence_action_id: Option<String>,
    evidence_episode_id: Option<String>,
    missing_evidence: Vec<String>,
    local_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct ContinuePackArtifactRole {
    workstream_id: String,
    artifact_id: String,
    role: String,
    title: Option<String>,
    kind: String,
}

#[derive(Debug, Clone, Serialize)]
struct ContinuePackBreadcrumb {
    workstream_id: String,
    text: String,
    created_at_ms: i64,
}

#[derive(Debug, Clone, Deserialize)]
struct ContinueMicroInferenceOutput {
    selected_candidate_id: String,
    selected_workstream_id: String,
    intent_label: String,
    next_action: Option<String>,
    reason: String,
    confidence: String,
    uncertainty_notes: Option<String>,
}

#[derive(Debug, Clone)]
struct ValidatedMicroInference {
    output: ContinueMicroInferenceOutput,
    response_id: Option<String>,
}

#[derive(Debug, Clone)]
struct CachedContinueDecisionMeta {
    decision_id: String,
    source: String,
    model: Option<String>,
    response_id: Option<String>,
    next_action: Option<String>,
    confidence: f64,
    warnings: Vec<String>,
    validation_status: String,
    validation_failures: Vec<String>,
    decision_reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct ContinueEvalFixture {
    cases: Vec<ContinueEvalCaseFixture>,
    k: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
struct ContinueEvalCaseFixture {
    name: String,
    scenario: String,
    expected_target_artifact_id: String,
    current_focus_artifact_id: Option<String>,
    candidates: Vec<ContinueEvalCandidateFixture>,
    model_output: Option<ContinueMicroInferenceOutput>,
}

#[derive(Debug, Clone, Deserialize)]
struct ContinueEvalCandidateFixture {
    id: String,
    workstream_id: String,
    target_artifact_id: Option<String>,
    score: f64,
    evidence_quality: Option<String>,
    missing_evidence: Option<Vec<String>>,
}

pub fn rebuild_continue_second_layer(
    conn: &Connection,
    request: ContinueSecondLayerRebuildRequest,
) -> Result<ContinueSecondLayerRebuildResult, String> {
    ensure_continue_schema(conn)?;
    let frames = load_evidence_frames(conn, &request)?;
    let frame_ids = frames
        .iter()
        .map(|frame| frame.id.clone())
        .collect::<Vec<_>>();
    clear_second_layer_rows_for_frames(conn, &frame_ids)?;

    let mut frame_artifacts = HashMap::new();
    for frame in &frames {
        let artifact = resolve_artifact(frame);
        upsert_continue_artifact(conn, frame, &artifact)?;
        upsert_continue_artifact_observation(conn, frame, &artifact)?;
        frame_artifacts.insert(frame.id.clone(), artifact);
    }

    let actions = collapse_repeated_task_actions(extract_task_actions(&frames, &frame_artifacts));
    for action in &actions {
        insert_continue_task_action(conn, action)?;
    }

    Ok(ContinueSecondLayerRebuildResult {
        processed_frames: frames.len() as i64,
        artifact_count: count_if_present(conn, "continue_artifacts")?,
        observation_count: count_if_present(conn, "continue_artifact_observations")?,
        task_action_count: count_if_present(conn, "continue_task_actions")?,
        start_frame_id: frames.first().map(|frame| frame.id.clone()),
        end_frame_id: frames.last().map(|frame| frame.id.clone()),
    })
}

pub fn recent_continue_artifacts(
    conn: &Connection,
    limit: Option<i64>,
) -> Result<Vec<RecentContinueArtifact>, String> {
    ensure_continue_schema(conn)?;
    let limit = limit.unwrap_or(50).clamp(1, 500);
    let mut stmt = conn
        .prepare(
            "SELECT a.id, a.artifact_kind, a.stable_key, a.app_name, a.window_title,
                    a.browser_url, a.document_path, a.display_title,
                    a.first_seen_frame_id, a.last_seen_frame_id, a.last_seen_timestamp,
                    a.identity_confidence, a.evidence_quality, a.openability,
                    COUNT(o.id) AS observation_count
             FROM continue_artifacts a
             LEFT JOIN continue_artifact_observations o ON o.artifact_id = a.id
             GROUP BY a.id
             ORDER BY a.last_seen_timestamp DESC, a.updated_at_ms DESC
             LIMIT ?1",
        )
        .map_err(to_string)?;
    let rows = stmt
        .query_map(params![limit], |row| {
            Ok(RecentContinueArtifact {
                id: row.get(0)?,
                artifact_kind: row.get(1)?,
                stable_key: row.get(2)?,
                app_name: row.get(3)?,
                window_title: row.get(4)?,
                browser_url: row.get(5)?,
                document_path: row.get(6)?,
                display_title: row.get(7)?,
                first_seen_frame_id: row.get(8)?,
                last_seen_frame_id: row.get(9)?,
                last_seen_timestamp: row.get(10)?,
                identity_confidence: row.get(11)?,
                evidence_quality: row.get(12)?,
                openability: row.get(13)?,
                observation_count: row.get(14)?,
            })
        })
        .map_err(to_string)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(to_string)
}

pub fn recent_continue_task_actions(
    conn: &Connection,
    limit: Option<i64>,
) -> Result<Vec<RecentContinueTaskAction>, String> {
    ensure_continue_schema(conn)?;
    let limit = limit.unwrap_or(80).clamp(1, 500);
    let mut stmt = conn
        .prepare(
            "SELECT id, frame_id, previous_frame_id, artifact_id, secondary_artifact_id,
                    action_kind, action_role, trigger_type, transition_label,
                    evidence_event_ids_json, confidence, reason, created_at_ms,
                    collapse_count, first_frame_id, last_frame_id, strongest_frame_id
             FROM continue_task_actions
             ORDER BY created_at_ms DESC, frame_id DESC
             LIMIT ?1",
        )
        .map_err(to_string)?;
    let rows = stmt
        .query_map(params![limit], |row| {
            let events_json: String = row.get(9)?;
            Ok(RecentContinueTaskAction {
                id: row.get(0)?,
                frame_id: row.get(1)?,
                previous_frame_id: row.get(2)?,
                artifact_id: row.get(3)?,
                secondary_artifact_id: row.get(4)?,
                action_kind: row.get(5)?,
                action_role: row.get(6)?,
                trigger_type: row.get(7)?,
                transition_label: row.get(8)?,
                evidence_event_ids: serde_json::from_str(&events_json).unwrap_or_default(),
                confidence: row.get(10)?,
                reason: row.get(11)?,
                created_at_ms: row.get(12)?,
                collapse_count: row.get::<_, i64>(13)?.max(1),
                first_frame_id: row.get(14)?,
                last_frame_id: row.get(15)?,
                strongest_frame_id: row.get(16)?,
            })
        })
        .map_err(to_string)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(to_string)
}

pub fn rebuild_continue_third_layer(
    conn: &Connection,
    request: ContinueThirdLayerRebuildRequest,
) -> Result<ContinueThirdLayerRebuildResult, String> {
    ensure_continue_schema(conn)?;
    let actions = load_continue_action_records(conn, &request)?;
    clear_third_layer_rows(conn)?;
    let episodes = build_continue_episodes(&actions);
    for episode in &episodes {
        insert_continue_episode(conn, episode)?;
    }
    let workstreams = build_continue_workstreams(&episodes);
    for workstream in &workstreams {
        insert_continue_workstream(conn, workstream)?;
    }

    Ok(ContinueThirdLayerRebuildResult {
        processed_actions: actions.len() as i64,
        episode_count: count_if_present(conn, "continue_episodes")?,
        episode_action_count: count_if_present(conn, "continue_episode_actions")?,
        episode_artifact_count: count_if_present(conn, "continue_episode_artifacts")?,
        workstream_count: count_if_present(conn, "continue_workstreams")?,
        workstream_episode_count: count_if_present(conn, "continue_workstream_episodes")?,
        workstream_artifact_count: count_if_present(conn, "continue_workstream_artifacts")?,
        start_frame_id: actions.first().map(|action| action.frame_id.clone()),
        end_frame_id: actions.last().map(|action| action.frame_id.clone()),
    })
}

pub fn get_continue_decision(
    conn: &Connection,
    request: ContinueDecisionRequest,
) -> Result<ContinueDecisionResult, String> {
    ensure_continue_schema(conn)?;
    let request = ContinueDecisionRequest {
        session_id: request.session_id,
        lookback_ms: request.lookback_ms.or(Some(45 * 60 * 1000)),
        limit: request.limit.or(Some(700)),
        mode: request.mode.or(Some("normal".to_string())),
        rebuild_layers: request.rebuild_layers.or(Some(false)),
        micro_inference_enabled: request.micro_inference_enabled.or(Some(false)),
        model: request.model,
        max_candidates_for_model: request.max_candidates_for_model.or(Some(5)),
    };
    let effective_mode = effective_continue_decision_mode(
        request.mode.as_deref(),
        request.rebuild_layers.unwrap_or(false),
    );
    let force_rebuild = effective_mode == "rebuild";
    let micro_inference_enabled = request.micro_inference_enabled.unwrap_or(false);
    let cached_meta = if !force_rebuild && !micro_inference_enabled {
        fresh_cached_continue_decision(conn, request.session_id.as_deref())?
    } else {
        None
    };
    if cached_meta.is_none() {
        infer_pending_continue_feedback(conn, 15 * 60 * 1000)?;
    }

    let semantic_rebuild_needed = cached_meta.is_none()
        && (force_rebuild || continue_layers_need_rebuild(conn, request.session_id.as_deref())?);
    if semantic_rebuild_needed {
        let start_frame_id = if force_rebuild {
            None
        } else {
            latest_continue_processed_frame_id(conn)?.map(|id| id + 1)
        };
        let second_request = ContinueSecondLayerRebuildRequest {
            session_id: request.session_id.clone(),
            lookback_ms: request.lookback_ms,
            start_frame_id,
            limit: request.limit,
            ..Default::default()
        };
        rebuild_continue_second_layer(conn, second_request)?;
        let third_request = ContinueThirdLayerRebuildRequest {
            session_id: request.session_id.clone(),
            lookback_ms: request.lookback_ms,
            limit: request.limit,
            ..Default::default()
        };
        rebuild_continue_third_layer(conn, third_request)?;
    }

    let requested_at_ms = current_time_millis();
    let current_focus = load_continue_current_focus(conn, request.session_id.as_deref())?;
    let mut workstreams = load_scorer_workstreams(conn, request.limit.unwrap_or(24))?;
    if workstreams.is_empty() {
        workstreams = load_any_recent_scorer_workstreams(conn, request.limit.unwrap_or(24))?;
    }
    let mut candidates = generate_continue_candidates(&workstreams, current_focus.as_ref());
    score_continue_candidates(&mut candidates, &workstreams, current_focus.as_ref());
    candidates.sort_by(|left, right| {
        right
            .score
            .partial_cmp(&left.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    if cached_meta.is_none() {
        for candidate in &candidates {
            insert_continue_candidate(conn, candidate, requested_at_ms)?;
        }
    }

    let local_selected = candidates.first().cloned();
    let mut selected = local_selected.clone();
    let mut selected_workstream = selected.as_ref().and_then(|candidate| {
        workstreams
            .iter()
            .find(|workstream| workstream.id == candidate.workstream_id)
            .cloned()
    });
    let mut warnings = selected
        .as_ref()
        .map(|candidate| candidate.warnings.clone())
        .unwrap_or_default();
    if selected.is_none() {
        warnings.push("thin_evidence:no_continue_workstream_candidate".to_string());
    }
    if let (Some(focus), Some(candidate)) = (&current_focus, &selected) {
        if focus.artifact_id.as_deref() != candidate.target_artifact.as_ref().map(|a| a.id.as_str())
            && candidate.candidate_kind != "evidence_only"
        {
            warnings.push("current_focus_differs_from_return_target".to_string());
        }
    }

    let mut confidence = selected
        .as_ref()
        .map(|candidate| candidate.score)
        .unwrap_or(0.0);
    let mut validation_status = validation_status_for_candidate(selected.as_ref());
    let mut next_action = selected.as_ref().map(next_action_for_candidate);
    let mut source = "local_scorer".to_string();
    let mut model = None;
    let mut response_id = None;
    let mut validation_failures = Vec::new();
    let mut decision_reason = selected
        .as_ref()
        .and_then(|candidate| candidate.reason.clone());

    if micro_inference_enabled && !candidates.is_empty() {
        match continue_openai_config(request.model.clone()) {
            Ok(config) => {
                model = Some(config.model.clone());
                if let Some(api_key) = config.api_key {
                    let candidate_limit =
                        request.max_candidates_for_model.unwrap_or(5).clamp(1, 12);
                    let pack = build_micro_inference_pack(
                        conn,
                        current_focus.as_ref(),
                        &workstreams,
                        &candidates,
                        candidate_limit as usize,
                    )?;
                    match run_continue_micro_inference(&api_key, &config.model, &pack) {
                        Ok(model_result) => {
                            match validate_micro_inference_output(
                                &model_result.output,
                                &candidates,
                                &pack,
                            ) {
                                Ok(validated_candidate) => {
                                    selected = Some(validated_candidate.clone());
                                    selected_workstream = workstreams
                                        .iter()
                                        .find(|workstream| {
                                            workstream.id == validated_candidate.workstream_id
                                        })
                                        .cloned();
                                    confidence = confidence_from_micro_output(
                                        &model_result.output.confidence,
                                        validated_candidate.score,
                                    );
                                    next_action =
                                        model_result.output.next_action.clone().or_else(|| {
                                            Some(next_action_for_candidate(&validated_candidate))
                                        });
                                    decision_reason = Some(model_result.output.reason.clone());
                                    if let Some(intent) =
                                        non_empty(model_result.output.intent_label.clone())
                                    {
                                        if let Some(workstream) = selected_workstream.as_mut() {
                                            workstream.title_candidate = Some(intent);
                                        }
                                    }
                                    if let Some(note) =
                                        model_result.output.uncertainty_notes.clone()
                                    {
                                        warnings.push(format!("model_uncertainty:{}", note));
                                    }
                                    response_id = model_result.response_id;
                                    source = "cloud_micro_inference".to_string();
                                    validation_status = "valid".to_string();
                                }
                                Err(failures) => {
                                    validation_failures = failures;
                                    warnings.push(format!(
                                        "micro_inference_validation_failed:{}",
                                        validation_failures.join("|")
                                    ));
                                    source = "local_fallback".to_string();
                                    validation_status = "fallback".to_string();
                                }
                            }
                        }
                        Err(error) => {
                            validation_failures.push(error.clone());
                            warnings.push(format!("micro_inference_failed:{}", error));
                            source = "local_fallback".to_string();
                            validation_status = "fallback".to_string();
                        }
                    }
                } else {
                    validation_failures.push("OPENAI_API_KEY is not set".to_string());
                    warnings.push("micro_inference_missing_openai_api_key".to_string());
                    source = "local_fallback".to_string();
                    validation_status = "fallback".to_string();
                }
            }
            Err(error) => {
                validation_failures.push(error.clone());
                warnings.push(format!("micro_inference_config_failed:{}", error));
                source = "local_fallback".to_string();
                validation_status = "fallback".to_string();
            }
        }
    }

    if let Some(cached) = &cached_meta {
        source = cached.source.clone();
        model = cached.model.clone();
        response_id = cached.response_id.clone();
        if cached.next_action.is_some() {
            next_action = cached.next_action.clone();
        }
        confidence = cached.confidence;
        if !cached.warnings.is_empty() {
            warnings = cached.warnings.clone();
        }
        validation_status = cached.validation_status.clone();
        validation_failures = cached.validation_failures.clone();
        if cached.decision_reason.is_some() {
            decision_reason = cached.decision_reason.clone();
        }
    }

    if let Some(candidate) = selected.as_ref() {
        confidence = round_score(f64::min(confidence, candidate.score));
    }

    let decision_watermark =
        latest_continue_evidence_timestamp(conn, request.session_id.as_deref())?
            .unwrap_or(requested_at_ms);
    let decision_id = cached_meta
        .as_ref()
        .map(|cached| cached.decision_id.clone())
        .unwrap_or_else(|| {
            format!(
                "continue-decision-{}",
                stable_hash(
                    format!(
                        "{}:{}:{}:{}",
                        decision_watermark,
                        selected
                            .as_ref()
                            .map(|candidate| candidate.id.as_str())
                            .unwrap_or("none"),
                        current_focus
                            .as_ref()
                            .map(|focus| focus.frame_id.as_str())
                            .unwrap_or("none"),
                        validation_status
                    )
                    .as_bytes()
                )
            )
        });

    let validation_notes = if validation_failures.is_empty() {
        None
    } else {
        Some(validation_failures.join(";"))
    };

    if cached_meta.is_none() {
        insert_continue_decision(
            conn,
            &decision_id,
            requested_at_ms,
            current_focus.as_ref(),
            selected_workstream.as_ref(),
            selected.as_ref(),
            next_action.as_deref(),
            &warnings,
            &validation_status,
            &source,
            decision_reason.as_deref(),
            confidence,
            response_id.as_deref(),
            model.as_deref(),
            validation_notes.as_deref(),
        )?;
    }

    let evidence_anchors = evidence_anchors_for_decision(
        current_focus.as_ref(),
        selected.as_ref(),
        selected_workstream.as_ref(),
    );
    let missing_evidence = selected
        .as_ref()
        .map(|candidate| candidate.missing_evidence.clone())
        .unwrap_or_else(|| vec!["no_candidate_generated".to_string()]);
    let alternatives = candidates
        .iter()
        .take(3)
        .map(candidate_summary)
        .collect::<Vec<_>>();

    Ok(ContinueDecisionResult {
        decision_id,
        mode: effective_mode.to_string(),
        cache_hit: cached_meta.is_some(),
        source,
        model,
        response_id,
        current_focus: current_focus.clone(),
        current_activity: current_activity_summary(
            selected
                .as_ref()
                .and_then(|candidate| candidate.last_meaningful_action.as_ref()),
            current_focus.as_ref(),
        ),
        selected_workstream: selected_workstream.as_ref().map(workstream_summary),
        return_target: selected.as_ref().map(|candidate| {
            return_target_summary(
                candidate.target_artifact.as_ref(),
                candidate.evidence_frame_id.clone(),
            )
        }),
        resume_work_target: selected.as_ref().map(|candidate| {
            return_target_summary(
                candidate
                    .resume_work_target
                    .as_ref()
                    .or(candidate.target_artifact.as_ref()),
                candidate.evidence_frame_id.clone(),
            )
        }),
        candidate_kind: selected
            .as_ref()
            .map(|candidate| candidate.candidate_kind.clone()),
        last_meaningful_action: selected
            .as_ref()
            .and_then(|candidate| candidate.last_meaningful_action.as_ref())
            .map(action_summary),
        unresolved_state: selected_workstream.as_ref().and_then(|workstream| {
            unresolved_state_description(workstream.unresolved_signal.as_deref())
        }),
        next_action,
        confidence: round_score(confidence),
        confidence_label: confidence_label(confidence).to_string(),
        evidence_anchors,
        missing_evidence,
        warnings,
        validation_failures,
        alternatives,
        generated_candidates: candidates.len() as i64,
        validation_status,
    })
}

pub fn recent_continue_episodes(
    conn: &Connection,
    limit: Option<i64>,
) -> Result<Vec<RecentContinueEpisode>, String> {
    ensure_continue_schema(conn)?;
    let limit = limit.unwrap_or(30).clamp(1, 200);
    let mut stmt = conn
        .prepare(
            "SELECT e.id, e.state, e.start_frame_id, e.end_frame_id,
                    e.start_timestamp_ms, e.end_timestamp_ms, e.primary_artifact_id,
                    a.display_title, e.dominant_action_kind, e.boundary_start_reason,
                    e.boundary_end_reason, e.evidence_quality, e.confidence, e.summary_label
             FROM continue_episodes e
             LEFT JOIN continue_artifacts a ON a.id = e.primary_artifact_id
             ORDER BY e.start_timestamp_ms DESC, e.id DESC
             LIMIT ?1",
        )
        .map_err(to_string)?;
    let rows = stmt
        .query_map(params![limit], |row| {
            Ok(RecentContinueEpisode {
                id: row.get(0)?,
                state: row.get(1)?,
                start_frame_id: row.get(2)?,
                end_frame_id: row.get(3)?,
                start_timestamp_ms: row.get(4)?,
                end_timestamp_ms: row.get(5)?,
                primary_artifact_id: row.get(6)?,
                primary_artifact_title: row.get(7)?,
                dominant_action_kind: row.get(8)?,
                boundary_start_reason: row.get(9)?,
                boundary_end_reason: row.get(10)?,
                evidence_quality: row.get(11)?,
                confidence: row.get(12)?,
                summary_label: row.get(13)?,
                actions: Vec::new(),
                artifacts: Vec::new(),
            })
        })
        .map_err(to_string)?;
    let mut episodes = rows.collect::<Result<Vec<_>, _>>().map_err(to_string)?;
    for episode in &mut episodes {
        episode.actions = recent_episode_actions(conn, &episode.id)?;
        episode.artifacts = recent_episode_artifacts(conn, &episode.id)?;
    }
    Ok(episodes)
}

pub fn recent_continue_workstreams(
    conn: &Connection,
    limit: Option<i64>,
) -> Result<Vec<RecentContinueWorkstream>, String> {
    ensure_continue_schema(conn)?;
    let limit = limit.unwrap_or(20).clamp(1, 100);
    let mut stmt = conn
        .prepare(
            "SELECT w.id, w.state, w.title_candidate, w.primary_artifact_id,
                    a.display_title, w.created_at_ms, w.last_active_timestamp_ms,
                    w.suspended_timestamp_ms, w.confidence, w.unresolved_signal, w.source
             FROM continue_workstreams w
             LEFT JOIN continue_artifacts a ON a.id = w.primary_artifact_id
             ORDER BY w.last_active_timestamp_ms DESC, w.id DESC
             LIMIT ?1",
        )
        .map_err(to_string)?;
    let rows = stmt
        .query_map(params![limit], |row| {
            Ok(RecentContinueWorkstream {
                id: row.get(0)?,
                state: row.get(1)?,
                title_candidate: row.get(2)?,
                primary_artifact_id: row.get(3)?,
                primary_artifact_title: row.get(4)?,
                created_at_ms: row.get(5)?,
                last_active_timestamp_ms: row.get(6)?,
                suspended_timestamp_ms: row.get(7)?,
                confidence: row.get(8)?,
                unresolved_signal: row.get(9)?,
                source: row.get(10)?,
                episodes: Vec::new(),
                artifacts: Vec::new(),
            })
        })
        .map_err(to_string)?;
    let mut workstreams = rows.collect::<Result<Vec<_>, _>>().map_err(to_string)?;
    for workstream in &mut workstreams {
        workstream.episodes = recent_workstream_episodes(conn, &workstream.id)?;
        workstream.artifacts = recent_workstream_artifacts(conn, &workstream.id)?;
    }
    Ok(workstreams)
}

fn load_continue_current_focus(
    conn: &Connection,
    session_id: Option<&str>,
) -> Result<Option<ContinueFocusSummary>, String> {
    if !table_exists(conn, "frames")? {
        return Ok(None);
    }
    conn.query_row(
        "SELECT f.id, f.captured_at, f.app_name, f.window_name, f.browser_url,
                f.document_path, a.id, a.artifact_kind, a.display_title,
                a.browser_url, a.document_path
         FROM frames f
         LEFT JOIN continue_artifact_observations o ON o.frame_id = CAST(f.id AS TEXT)
         LEFT JOIN continue_artifacts a ON a.id = o.artifact_id
         WHERE (?1 IS NULL OR f.session_id = ?1)
         ORDER BY f.captured_at DESC, f.id DESC, o.observation_confidence DESC
         LIMIT 1",
        params![session_id],
        |row| {
            let frame_id: i64 = row.get(0)?;
            Ok(ContinueFocusSummary {
                frame_id: frame_id.to_string(),
                captured_at_ms: row.get(1)?,
                app_name: row.get(2)?,
                window_title: row.get(3)?,
                browser_url: row.get::<_, Option<String>>(9)?.or(row.get(4)?),
                document_path: row.get::<_, Option<String>>(10)?.or(row.get(5)?),
                artifact_id: row.get(6)?,
                artifact_kind: row.get(7)?,
                title: row.get(8)?,
            })
        },
    )
    .optional()
    .map_err(to_string)
}

fn fresh_cached_continue_decision(
    conn: &Connection,
    session_id: Option<&str>,
) -> Result<Option<CachedContinueDecisionMeta>, String> {
    if !table_exists(conn, "continue_decisions")? {
        return Ok(None);
    }
    let Some(evidence_ts) = latest_continue_evidence_timestamp(conn, session_id)? else {
        return Ok(None);
    };
    let mut stmt = conn
        .prepare(
            "SELECT d.id, d.source, d.model, d.response_id, d.next_action,
                    d.confidence, d.warnings, d.validation_status,
                    d.validation_notes, d.decision_reason
             FROM continue_decisions d
             LEFT JOIN frames f ON CAST(f.id AS TEXT) = d.current_focus_frame_id
             WHERE d.requested_at_ms >= ?1
               AND (?2 IS NULL OR f.session_id = ?2)
             ORDER BY d.requested_at_ms DESC
             LIMIT 1",
        )
        .map_err(to_string)?;
    stmt.query_row(params![evidence_ts, session_id], |row| {
        let warnings: Option<String> = row.get(6)?;
        let validation_notes: Option<String> = row.get(8)?;
        Ok(CachedContinueDecisionMeta {
            decision_id: row.get(0)?,
            source: row.get(1)?,
            model: row.get(2)?,
            response_id: row.get(3)?,
            next_action: row.get(4)?,
            confidence: row.get(5)?,
            warnings: split_semicolon_list(warnings.as_deref()),
            validation_status: row.get(7)?,
            validation_failures: split_semicolon_list(validation_notes.as_deref()),
            decision_reason: row.get(9)?,
        })
    })
    .optional()
    .map_err(to_string)
}

pub fn effective_continue_decision_mode(mode: Option<&str>, rebuild_layers: bool) -> &'static str {
    if rebuild_layers {
        return "rebuild";
    }
    match mode
        .unwrap_or("normal")
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "rebuild" | "force_rebuild" | "diagnostic_rebuild" => "rebuild",
        _ => "normal",
    }
}

fn continue_layers_need_rebuild(
    conn: &Connection,
    session_id: Option<&str>,
) -> Result<bool, String> {
    if !table_exists(conn, "frames")? {
        return Ok(false);
    }
    let latest_frame_ts = latest_frame_timestamp(conn, session_id)?;
    let Some(latest_frame_ts) = latest_frame_ts else {
        return Ok(false);
    };
    if !table_exists(conn, "continue_workstreams")?
        || count_if_present(conn, "continue_workstreams")? == 0
    {
        return Ok(true);
    }
    let latest_artifact_ts =
        max_i64_if_present(conn, "continue_artifact_observations", "timestamp_ms")?.unwrap_or(0);
    Ok(latest_frame_ts > latest_artifact_ts)
}

fn latest_continue_processed_frame_id(conn: &Connection) -> Result<Option<i64>, String> {
    if !table_exists(conn, "continue_artifact_observations")? {
        return Ok(None);
    }
    conn.query_row(
        "SELECT MAX(CAST(frame_id AS INTEGER))
         FROM continue_artifact_observations
         WHERE frame_id NOT LIKE 'event-%'",
        [],
        |row| row.get(0),
    )
    .map_err(to_string)
}

fn latest_continue_evidence_timestamp(
    conn: &Connection,
    session_id: Option<&str>,
) -> Result<Option<i64>, String> {
    let latest_frame = latest_frame_timestamp(conn, session_id)?;
    let latest_event = latest_event_timestamp(conn, session_id)?;
    Ok(match (latest_frame, latest_event) {
        (Some(frame_ts), Some(event_ts)) => Some(frame_ts.max(event_ts)),
        (Some(frame_ts), None) => Some(frame_ts),
        (None, Some(event_ts)) => Some(event_ts),
        (None, None) => None,
    })
}

fn latest_frame_timestamp(
    conn: &Connection,
    session_id: Option<&str>,
) -> Result<Option<i64>, String> {
    if !table_exists(conn, "frames")? {
        return Ok(None);
    }
    conn.query_row(
        "SELECT MAX(captured_at) FROM frames WHERE (?1 IS NULL OR session_id = ?1)",
        params![session_id],
        |row| row.get(0),
    )
    .map_err(to_string)
}

fn latest_event_timestamp(
    conn: &Connection,
    session_id: Option<&str>,
) -> Result<Option<i64>, String> {
    if !table_exists(conn, "ui_events")? {
        return Ok(None);
    }
    conn.query_row(
        "SELECT MAX(ts_ms) FROM ui_events WHERE (?1 IS NULL OR session_id = ?1)",
        params![session_id],
        |row| row.get(0),
    )
    .map_err(to_string)
}

fn split_semicolon_list(value: Option<&str>) -> Vec<String> {
    value
        .unwrap_or("")
        .split(';')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(str::to_string)
        .collect()
}

fn load_scorer_workstreams(conn: &Connection, limit: i64) -> Result<Vec<ScorerWorkstream>, String> {
    load_scorer_workstreams_with_filter(
        conn,
        "WHERE w.state IN ('active', 'suspended') OR w.unresolved_signal IS NOT NULL",
        limit,
    )
}

fn load_any_recent_scorer_workstreams(
    conn: &Connection,
    limit: i64,
) -> Result<Vec<ScorerWorkstream>, String> {
    load_scorer_workstreams_with_filter(conn, "", limit)
}

fn load_scorer_workstreams_with_filter(
    conn: &Connection,
    where_clause: &str,
    limit: i64,
) -> Result<Vec<ScorerWorkstream>, String> {
    let sql = format!(
        "SELECT w.id, w.state, w.title_candidate, w.primary_artifact_id,
                w.last_active_timestamp_ms, w.confidence, w.unresolved_signal
         FROM continue_workstreams w
         {}
         ORDER BY w.last_active_timestamp_ms DESC, w.confidence DESC
         LIMIT ?1",
        where_clause
    );
    let limit = limit.clamp(1, 100);
    let mut stmt = conn.prepare(&sql).map_err(to_string)?;
    let rows = stmt
        .query_map(params![limit], |row| {
            Ok(ScorerWorkstream {
                id: row.get(0)?,
                state: row.get(1)?,
                title_candidate: row.get(2)?,
                primary_artifact_id: row.get(3)?,
                last_active_timestamp_ms: row.get(4)?,
                confidence: row.get(5)?,
                unresolved_signal: row.get(6)?,
                episodes: Vec::new(),
                artifacts: Vec::new(),
                last_meaningful_action: None,
            })
        })
        .map_err(to_string)?;
    let mut workstreams = rows.collect::<Result<Vec<_>, _>>().map_err(to_string)?;
    for workstream in &mut workstreams {
        workstream.artifacts = load_scorer_workstream_artifacts(conn, &workstream.id)?;
        workstream.episodes = load_scorer_workstream_episodes(conn, &workstream.id)?;
        workstream.last_meaningful_action = load_last_meaningful_action(conn, &workstream.id)?;
    }
    Ok(workstreams)
}

fn load_scorer_workstream_artifacts(
    conn: &Connection,
    workstream_id: &str,
) -> Result<Vec<ScorerWorkstreamArtifact>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT a.id, a.artifact_kind, a.display_title, a.browser_url,
                    a.document_path, a.evidence_quality, a.privacy_status,
                    a.openability, a.last_seen_frame_id, a.last_seen_timestamp,
                    wa.durable_role, wa.importance_score, wa.first_seen_frame_id,
                    wa.last_seen_frame_id
             FROM continue_workstream_artifacts wa
             JOIN continue_artifacts a ON a.id = wa.artifact_id
             WHERE wa.workstream_id = ?1
             ORDER BY wa.importance_score DESC, a.last_seen_timestamp DESC",
        )
        .map_err(to_string)?;
    let rows = stmt
        .query_map(params![workstream_id], |row| {
            Ok(ScorerWorkstreamArtifact {
                artifact: ScorerArtifact {
                    id: row.get(0)?,
                    artifact_kind: row.get(1)?,
                    display_title: row.get(2)?,
                    browser_url: row.get(3)?,
                    document_path: row.get(4)?,
                    evidence_quality: row.get(5)?,
                    privacy_status: row.get(6)?,
                    openability: row.get(7)?,
                    last_seen_frame_id: row.get(8)?,
                    last_seen_timestamp: row.get(9)?,
                },
                durable_role: row.get(10)?,
                importance_score: row.get(11)?,
                first_seen_frame_id: row.get(12)?,
                last_seen_frame_id: row.get(13)?,
            })
        })
        .map_err(to_string)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(to_string)
}

pub fn add_continue_breadcrumb(
    conn: &Connection,
    request: ContinueBreadcrumbRequest,
) -> Result<ContinueBreadcrumbResult, String> {
    ensure_continue_schema(conn)?;
    let text = request.text.trim();
    if text.is_empty() {
        return Err("breadcrumb text cannot be empty".to_string());
    }
    if text.len() > 500 {
        return Err("breadcrumb text must be 500 characters or less".to_string());
    }
    let exists: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM continue_workstreams WHERE id = ?1)",
            params![request.workstream_id],
            |row| row.get::<_, i64>(0),
        )
        .map_err(to_string)?
        != 0;
    if !exists {
        return Err(format!("unknown workstream_id: {}", request.workstream_id));
    }
    let created_at_ms = current_time_millis();
    let source = request
        .source
        .and_then(non_empty)
        .unwrap_or_else(|| "manual".to_string());
    let id = format!(
        "continue-breadcrumb-{}",
        stable_hash(format!("{}:{}:{}", request.workstream_id, created_at_ms, text).as_bytes())
    );
    conn.execute(
        "INSERT INTO continue_breadcrumbs (
            id, workstream_id, text, source, created_at_ms
         ) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![id, request.workstream_id, text, source, created_at_ms],
    )
    .map_err(to_string)?;
    Ok(ContinueBreadcrumbResult {
        id,
        workstream_id: request.workstream_id,
        text: text.to_string(),
        source,
        created_at_ms,
    })
}

pub fn infer_continue_feedback(
    conn: &Connection,
    request: ContinueFeedbackRequest,
) -> Result<Vec<ContinueFeedbackEventResult>, String> {
    ensure_continue_schema(conn)?;
    let window_ms = request.observation_window_ms.unwrap_or(15 * 60 * 1000);
    if let Some(decision_id) = request.decision_id {
        infer_feedback_for_decision(conn, &decision_id, window_ms)?
            .map(|event| vec![event])
            .ok_or_else(|| format!("no feedback inferred for decision_id: {}", decision_id))
    } else {
        infer_pending_continue_feedback(conn, window_ms)
    }
}

pub fn get_continue_workstream_detail(
    conn: &Connection,
    request: ContinueWorkstreamDetailRequest,
) -> Result<ContinueWorkstreamDetailResult, String> {
    ensure_continue_schema(conn)?;
    let workstream_id = request.workstream_id.trim();
    if workstream_id.is_empty() {
        return Err("workstream_id cannot be empty".to_string());
    }
    let workstream = load_recent_continue_workstream(conn, workstream_id)?
        .ok_or_else(|| format!("unknown workstream_id: {}", workstream_id))?;
    let artifacts = load_continue_workstream_artifact_details(conn, workstream_id)?;
    let episodes = load_continue_workstream_episode_details(conn, workstream_id)?;
    let candidates = load_continue_workstream_candidate_details(conn, workstream_id)?;
    let latest_decision =
        load_continue_decision_summary(conn, workstream_id, request.decision_id.as_deref())?;
    let feedback_events = load_continue_feedback_events(
        conn,
        latest_decision
            .as_ref()
            .map(|decision| decision.decision_id.as_str()),
        Some(workstream_id),
    )?;
    let breadcrumbs = load_continue_breadcrumbs(conn, workstream_id)?;
    let evidence_anchors = workstream_detail_anchors(&artifacts, &episodes, &candidates);

    Ok(ContinueWorkstreamDetailResult {
        workstream,
        artifacts,
        episodes,
        candidates,
        latest_decision,
        feedback_events,
        breadcrumbs,
        evidence_anchors,
    })
}

pub fn record_continue_feedback(
    conn: &Connection,
    request: ContinueExplicitFeedbackRequest,
) -> Result<ContinueFeedbackEventResult, String> {
    ensure_continue_schema(conn)?;
    let feedback_kind = request.feedback_kind.trim();
    let allowed = [
        "accepted",
        "rejected",
        "ignored",
        "corrected",
        "artifact_only_evidence",
        "ignored_workstream",
        "user_next_step_note",
    ];
    if !allowed.contains(&feedback_kind) {
        return Err(format!("unsupported feedback_kind: {}", feedback_kind));
    }
    let note = request.note.and_then(non_empty).map(|value| {
        if value.len() > 500 {
            value.chars().take(500).collect::<String>()
        } else {
            value
        }
    });
    let source = request
        .source
        .and_then(non_empty)
        .unwrap_or_else(|| "desktop_ui".to_string());
    let timestamp_ms = current_time_millis();
    let reason = match feedback_kind {
        "accepted" => "user marked the Continue target useful",
        "rejected" => "user marked the Continue target wrong",
        "corrected" => "user selected a different continuation target",
        "artifact_only_evidence" => "user marked the artifact as supporting evidence only",
        "ignored_workstream" => "user ignored this workstream",
        "user_next_step_note" => "user added a next-step note",
        _ => "user supplied explicit Continue feedback",
    };
    let id = format!(
        "continue-feedback-{}",
        stable_hash(
            format!(
                "{}:{}:{}:{}:{}:{}:{}",
                request.decision_id.as_deref().unwrap_or("none"),
                request.workstream_id.as_deref().unwrap_or("none"),
                feedback_kind,
                request.target_artifact_id.as_deref().unwrap_or("none"),
                request.corrected_artifact_id.as_deref().unwrap_or("none"),
                note.as_deref().unwrap_or("none"),
                source
            )
            .as_bytes(),
        )
    );
    conn.execute(
        "INSERT OR IGNORE INTO continue_feedback_events (
            id, decision_id, event_kind, observed_frame_id, target_artifact_id,
            chosen_artifact_id, timestamp_ms, confidence, reason,
            selected_candidate_id, workstream_id, note, source
         ) VALUES (?1, ?2, ?3, NULL, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        params![
            id,
            request.decision_id,
            feedback_kind,
            request.target_artifact_id,
            request.corrected_artifact_id,
            timestamp_ms,
            1.0_f64,
            reason,
            request.selected_candidate_id,
            request.workstream_id,
            note,
            source,
        ],
    )
    .map_err(to_string)?;
    Ok(ContinueFeedbackEventResult {
        id,
        decision_id: request.decision_id,
        selected_candidate_id: request.selected_candidate_id,
        workstream_id: request.workstream_id,
        event_kind: feedback_kind.to_string(),
        observed_frame_id: None,
        target_artifact_id: request.target_artifact_id,
        chosen_artifact_id: request.corrected_artifact_id,
        timestamp_ms,
        confidence: 1.0,
        reason: Some(reason.to_string()),
        note,
        source: Some(source),
    })
}

pub fn run_continue_eval(eval_file_path: Option<String>) -> Result<ContinueEvalReport, String> {
    let fixture = if let Some(path) = eval_file_path.and_then(non_empty) {
        let raw = fs::read_to_string(path).map_err(to_string)?;
        serde_json::from_str::<ContinueEvalFixture>(&raw).map_err(to_string)?
    } else {
        default_continue_eval_fixture()
    };
    summarize_continue_eval_fixture(fixture)
}

fn load_recent_continue_workstream(
    conn: &Connection,
    workstream_id: &str,
) -> Result<Option<RecentContinueWorkstream>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT w.id, w.state, w.title_candidate, w.primary_artifact_id,
                    a.display_title, w.created_at_ms, w.last_active_timestamp_ms,
                    w.suspended_timestamp_ms, w.confidence, w.unresolved_signal, w.source
             FROM continue_workstreams w
             LEFT JOIN continue_artifacts a ON a.id = w.primary_artifact_id
             WHERE w.id = ?1
             LIMIT 1",
        )
        .map_err(to_string)?;
    let mut workstream = stmt
        .query_row(params![workstream_id], |row| {
            Ok(RecentContinueWorkstream {
                id: row.get(0)?,
                state: row.get(1)?,
                title_candidate: row.get(2)?,
                primary_artifact_id: row.get(3)?,
                primary_artifact_title: row.get(4)?,
                created_at_ms: row.get(5)?,
                last_active_timestamp_ms: row.get(6)?,
                suspended_timestamp_ms: row.get(7)?,
                confidence: row.get(8)?,
                unresolved_signal: row.get(9)?,
                source: row.get(10)?,
                episodes: Vec::new(),
                artifacts: Vec::new(),
            })
        })
        .optional()
        .map_err(to_string)?;
    if let Some(workstream) = workstream.as_mut() {
        workstream.episodes = recent_workstream_episodes(conn, &workstream.id)?;
        workstream.artifacts = recent_workstream_artifacts(conn, &workstream.id)?;
    }
    Ok(workstream)
}

fn load_continue_workstream_artifact_details(
    conn: &Connection,
    workstream_id: &str,
) -> Result<Vec<ContinueWorkstreamArtifactDetail>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT wa.artifact_id, wa.durable_role, a.artifact_kind, a.display_title,
                    a.stable_key, a.app_name, a.window_title, a.browser_url,
                    a.document_path, a.openability, a.evidence_quality, a.privacy_status,
                    wa.importance_score, wa.first_seen_frame_id, wa.last_seen_frame_id, wa.reason
             FROM continue_workstream_artifacts wa
             JOIN continue_artifacts a ON a.id = wa.artifact_id
             WHERE wa.workstream_id = ?1
             ORDER BY wa.importance_score DESC, wa.durable_role ASC",
        )
        .map_err(to_string)?;
    let rows = stmt
        .query_map(params![workstream_id], |row| {
            Ok(ContinueWorkstreamArtifactDetail {
                artifact_id: row.get(0)?,
                durable_role: row.get(1)?,
                artifact_kind: row.get(2)?,
                display_title: row.get(3)?,
                stable_key: row.get(4)?,
                app_name: row.get(5)?,
                window_title: row.get(6)?,
                browser_url: row.get(7)?,
                document_path: row.get(8)?,
                openability: row.get(9)?,
                evidence_quality: row.get(10)?,
                privacy_status: row.get(11)?,
                importance_score: row.get(12)?,
                first_seen_frame_id: row.get(13)?,
                last_seen_frame_id: row.get(14)?,
                reason: row.get(15)?,
            })
        })
        .map_err(to_string)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(to_string)
}

fn load_continue_workstream_episode_details(
    conn: &Connection,
    workstream_id: &str,
) -> Result<Vec<ContinueWorkstreamEpisodeDetail>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT e.id, e.state, e.start_frame_id, e.end_frame_id,
                    e.start_timestamp_ms, e.end_timestamp_ms, e.primary_artifact_id,
                    a.display_title, e.dominant_action_kind, e.boundary_start_reason,
                    e.boundary_end_reason, e.evidence_quality, e.confidence, e.summary_label,
                    we.membership_score, we.membership_reason
             FROM continue_workstream_episodes we
             JOIN continue_episodes e ON e.id = we.episode_id
             LEFT JOIN continue_artifacts a ON a.id = e.primary_artifact_id
             WHERE we.workstream_id = ?1
             ORDER BY we.order_index ASC, e.start_timestamp_ms DESC",
        )
        .map_err(to_string)?;
    let rows = stmt
        .query_map(params![workstream_id], |row| {
            Ok(ContinueWorkstreamEpisodeDetail {
                id: row.get(0)?,
                state: row.get(1)?,
                start_frame_id: row.get(2)?,
                end_frame_id: row.get(3)?,
                start_timestamp_ms: row.get(4)?,
                end_timestamp_ms: row.get(5)?,
                primary_artifact_id: row.get(6)?,
                primary_artifact_title: row.get(7)?,
                dominant_action_kind: row.get(8)?,
                boundary_start_reason: row.get(9)?,
                boundary_end_reason: row.get(10)?,
                evidence_quality: row.get(11)?,
                confidence: row.get(12)?,
                summary_label: row.get(13)?,
                membership_score: row.get(14)?,
                membership_reason: row.get(15)?,
                actions: Vec::new(),
                artifacts: Vec::new(),
            })
        })
        .map_err(to_string)?;
    let mut episodes = rows.collect::<Result<Vec<_>, _>>().map_err(to_string)?;
    for episode in &mut episodes {
        episode.actions = load_continue_episode_action_details(conn, &episode.id)?;
        episode.artifacts = recent_episode_artifacts(conn, &episode.id)?;
    }
    Ok(episodes)
}

fn load_continue_episode_action_details(
    conn: &Connection,
    episode_id: &str,
) -> Result<Vec<ContinueWorkstreamActionDetail>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT ea.action_id, ta.frame_id, ta.previous_frame_id, ta.artifact_id,
                    artifact.display_title, ta.secondary_artifact_id, secondary.display_title,
                    ta.action_kind, ta.action_role, ea.role_in_episode, ea.order_index,
                    ta.trigger_type, ta.transition_label, ta.evidence_event_ids_json,
                    ta.confidence, ta.reason, ta.created_at_ms
             FROM continue_episode_actions ea
             JOIN continue_task_actions ta ON ta.id = ea.action_id
             LEFT JOIN continue_artifacts artifact ON artifact.id = ta.artifact_id
             LEFT JOIN continue_artifacts secondary ON secondary.id = ta.secondary_artifact_id
             WHERE ea.episode_id = ?1
             ORDER BY ea.order_index ASC",
        )
        .map_err(to_string)?;
    let rows = stmt
        .query_map(params![episode_id], |row| {
            let events_json: String = row.get(13)?;
            Ok(ContinueWorkstreamActionDetail {
                action_id: row.get(0)?,
                frame_id: row.get(1)?,
                previous_frame_id: row.get(2)?,
                artifact_id: row.get(3)?,
                artifact_title: row.get(4)?,
                secondary_artifact_id: row.get(5)?,
                secondary_artifact_title: row.get(6)?,
                action_kind: row.get(7)?,
                action_role: row.get(8)?,
                role_in_episode: row.get(9)?,
                order_index: row.get(10)?,
                trigger_type: row.get(11)?,
                transition_label: row.get(12)?,
                evidence_event_ids: parse_string_array(&events_json),
                confidence: row.get(14)?,
                reason: row.get(15)?,
                created_at_ms: row.get(16)?,
            })
        })
        .map_err(to_string)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(to_string)
}

fn load_continue_workstream_candidate_details(
    conn: &Connection,
    workstream_id: &str,
) -> Result<Vec<ContinueWorkstreamCandidateDetail>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT c.id, c.workstream_id, c.target_artifact_id, a.display_title,
                    a.artifact_kind, a.openability, c.candidate_kind,
                    c.last_meaningful_action_id, c.evidence_frame_id, c.supporting_episode_id,
                    c.score, c.actionability_score, c.primary_target_score,
                    c.unresolved_score, c.branch_origin_score, c.evidence_quality_score,
                    c.recency_score, c.openability_score, c.privacy_safety_score,
                    c.reason, c.missing_evidence, c.created_at_ms
             FROM continue_candidates c
             LEFT JOIN continue_artifacts a ON a.id = c.target_artifact_id
             WHERE c.workstream_id = ?1
             ORDER BY c.created_at_ms DESC, c.score DESC
             LIMIT 12",
        )
        .map_err(to_string)?;
    let rows = stmt
        .query_map(params![workstream_id], |row| {
            let score: f64 = row.get(10)?;
            let missing_evidence = row
                .get::<_, Option<String>>(20)?
                .map(|value| {
                    value
                        .split(';')
                        .filter_map(|part| non_empty(part.to_string()))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            Ok(ContinueWorkstreamCandidateDetail {
                candidate_id: row.get(0)?,
                workstream_id: row.get(1)?,
                target_artifact_id: row.get(2)?,
                target_title: row.get(3)?,
                target_kind: row.get(4)?,
                target_openability: row.get(5)?,
                candidate_kind: row.get(6)?,
                last_meaningful_action_id: row.get(7)?,
                evidence_frame_id: row.get(8)?,
                supporting_episode_id: row.get(9)?,
                score: round_score(score),
                confidence_label: confidence_label(score).to_string(),
                reason: row.get(19)?,
                missing_evidence,
                components: ContinueScoreComponents {
                    actionability: round_score(row.get(11)?),
                    primary_target: round_score(row.get(12)?),
                    unresolved_state: round_score(row.get(13)?),
                    branch_origin: round_score(row.get(14)?),
                    evidence_quality: round_score(row.get(15)?),
                    recency: round_score(row.get(16)?),
                    openability: round_score(row.get(17)?),
                    privacy_safety: round_score(row.get(18)?),
                },
                created_at_ms: row.get(21)?,
            })
        })
        .map_err(to_string)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(to_string)
}

fn load_continue_decision_summary(
    conn: &Connection,
    workstream_id: &str,
    decision_id: Option<&str>,
) -> Result<Option<ContinueDecisionSummary>, String> {
    let (sql, bind_value) = if let Some(decision_id) = decision_id {
        (
            "SELECT id, requested_at_ms, source, selected_candidate_id,
                    return_target_artifact_id, confidence, decision_reason,
                    next_action, warnings, validation_status
             FROM continue_decisions
             WHERE id = ?1
             LIMIT 1",
            decision_id,
        )
    } else {
        (
            "SELECT id, requested_at_ms, source, selected_candidate_id,
                    return_target_artifact_id, confidence, decision_reason,
                    next_action, warnings, validation_status
             FROM continue_decisions
             WHERE selected_workstream_id = ?1
             ORDER BY requested_at_ms DESC
             LIMIT 1",
            workstream_id,
        )
    };
    conn.query_row(sql, params![bind_value], |row| {
        let warnings = row
            .get::<_, Option<String>>(8)?
            .map(|value| {
                value
                    .split(';')
                    .filter_map(|part| non_empty(part.to_string()))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        Ok(ContinueDecisionSummary {
            decision_id: row.get(0)?,
            requested_at_ms: row.get(1)?,
            source: row.get(2)?,
            selected_candidate_id: row.get(3)?,
            return_target_artifact_id: row.get(4)?,
            confidence: round_score(row.get(5)?),
            decision_reason: row.get(6)?,
            next_action: row.get(7)?,
            warnings,
            validation_status: row.get(9)?,
        })
    })
    .optional()
    .map_err(to_string)
}

fn load_continue_feedback_events(
    conn: &Connection,
    decision_id: Option<&str>,
    workstream_id: Option<&str>,
) -> Result<Vec<ContinueFeedbackEventResult>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, decision_id, selected_candidate_id, workstream_id, event_kind,
                    observed_frame_id, target_artifact_id, chosen_artifact_id,
                    timestamp_ms, confidence, reason, note, source
             FROM continue_feedback_events
             WHERE (?1 IS NOT NULL AND decision_id = ?1)
                OR (?2 IS NOT NULL AND workstream_id = ?2)
             ORDER BY timestamp_ms DESC
             LIMIT 20",
        )
        .map_err(to_string)?;
    let rows = stmt
        .query_map(params![decision_id, workstream_id], |row| {
            Ok(ContinueFeedbackEventResult {
                id: row.get(0)?,
                decision_id: row.get(1)?,
                selected_candidate_id: row.get(2)?,
                workstream_id: row.get(3)?,
                event_kind: row.get(4)?,
                observed_frame_id: row.get(5)?,
                target_artifact_id: row.get(6)?,
                chosen_artifact_id: row.get(7)?,
                timestamp_ms: row.get(8)?,
                confidence: round_score(row.get(9)?),
                reason: row.get(10)?,
                note: row.get(11)?,
                source: row.get(12)?,
            })
        })
        .map_err(to_string)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(to_string)
}

fn load_continue_breadcrumbs(
    conn: &Connection,
    workstream_id: &str,
) -> Result<Vec<ContinueBreadcrumbSummary>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, workstream_id, text, source, created_at_ms
             FROM continue_breadcrumbs
             WHERE workstream_id = ?1
             ORDER BY created_at_ms DESC
             LIMIT 12",
        )
        .map_err(to_string)?;
    let rows = stmt
        .query_map(params![workstream_id], |row| {
            Ok(ContinueBreadcrumbSummary {
                id: row.get(0)?,
                workstream_id: row.get(1)?,
                text: row.get(2)?,
                source: row.get(3)?,
                created_at_ms: row.get(4)?,
            })
        })
        .map_err(to_string)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(to_string)
}

fn workstream_detail_anchors(
    artifacts: &[ContinueWorkstreamArtifactDetail],
    episodes: &[ContinueWorkstreamEpisodeDetail],
    candidates: &[ContinueWorkstreamCandidateDetail],
) -> ContinueEvidenceAnchors {
    let mut frame_ids = Vec::new();
    let mut action_ids = Vec::new();
    let mut episode_ids = Vec::new();
    let mut artifact_ids = Vec::new();
    let mut seen_frames = HashSet::new();
    let mut seen_actions = HashSet::new();
    let mut seen_episodes = HashSet::new();
    let mut seen_artifacts = HashSet::new();

    for artifact in artifacts {
        if seen_artifacts.insert(artifact.artifact_id.clone()) {
            artifact_ids.push(artifact.artifact_id.clone());
        }
        for frame_id in [
            artifact.first_seen_frame_id.as_ref(),
            artifact.last_seen_frame_id.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            if seen_frames.insert(frame_id.clone()) {
                frame_ids.push(frame_id.clone());
            }
        }
    }
    for episode in episodes {
        if seen_episodes.insert(episode.id.clone()) {
            episode_ids.push(episode.id.clone());
        }
        for frame_id in [
            episode.start_frame_id.as_ref(),
            episode.end_frame_id.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            if seen_frames.insert(frame_id.clone()) {
                frame_ids.push(frame_id.clone());
            }
        }
        for action in &episode.actions {
            if seen_actions.insert(action.action_id.clone()) {
                action_ids.push(action.action_id.clone());
            }
            if seen_frames.insert(action.frame_id.clone()) {
                frame_ids.push(action.frame_id.clone());
            }
            if let Some(artifact_id) = &action.artifact_id {
                if seen_artifacts.insert(artifact_id.clone()) {
                    artifact_ids.push(artifact_id.clone());
                }
            }
        }
    }
    for candidate in candidates {
        if let Some(frame_id) = &candidate.evidence_frame_id {
            if seen_frames.insert(frame_id.clone()) {
                frame_ids.push(frame_id.clone());
            }
        }
        if let Some(action_id) = &candidate.last_meaningful_action_id {
            if seen_actions.insert(action_id.clone()) {
                action_ids.push(action_id.clone());
            }
        }
        if let Some(episode_id) = &candidate.supporting_episode_id {
            if seen_episodes.insert(episode_id.clone()) {
                episode_ids.push(episode_id.clone());
            }
        }
        if let Some(artifact_id) = &candidate.target_artifact_id {
            if seen_artifacts.insert(artifact_id.clone()) {
                artifact_ids.push(artifact_id.clone());
            }
        }
    }

    ContinueEvidenceAnchors {
        frame_ids: frame_ids.into_iter().take(24).collect(),
        action_ids: action_ids.into_iter().take(24).collect(),
        episode_ids: episode_ids.into_iter().take(24).collect(),
        artifact_ids: artifact_ids.into_iter().take(24).collect(),
    }
}

fn load_scorer_workstream_episodes(
    conn: &Connection,
    workstream_id: &str,
) -> Result<Vec<ScorerEpisode>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT e.id, e.end_frame_id, COALESCE(e.end_timestamp_ms, e.start_timestamp_ms),
                    e.dominant_action_kind, e.evidence_quality
             FROM continue_workstream_episodes we
             JOIN continue_episodes e ON e.id = we.episode_id
             WHERE we.workstream_id = ?1
             ORDER BY we.order_index ASC",
        )
        .map_err(to_string)?;
    let rows = stmt
        .query_map(params![workstream_id], |row| {
            Ok(ScorerEpisode {
                id: row.get(0)?,
                end_frame_id: row.get(1)?,
                end_timestamp_ms: row.get(2)?,
                dominant_action_kind: row.get(3)?,
                evidence_quality: row.get(4)?,
            })
        })
        .map_err(to_string)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(to_string)
}

fn load_last_meaningful_action(
    conn: &Connection,
    workstream_id: &str,
) -> Result<Option<ScorerAction>, String> {
    conn.query_row(
        "SELECT ta.id, ta.frame_id, ta.artifact_id, ta.secondary_artifact_id,
                ta.action_kind, ta.action_role, ta.confidence, ta.reason, ta.created_at_ms,
                ta.collapse_count, ta.first_frame_id, ta.last_frame_id, ta.strongest_frame_id
         FROM continue_workstream_episodes we
         JOIN continue_episode_actions ea ON ea.episode_id = we.episode_id
         JOIN continue_task_actions ta ON ta.id = ea.action_id
         WHERE we.workstream_id = ?1
           AND ta.action_kind IN (
             'editing', 'composing', 'copying_evidence', 'running_command',
             'observing_command_output', 'reviewing_output', 'encountering_error',
             'returning_to_origin', 'branching_away', 'searching', 'idle_after_progress'
           )
         ORDER BY ta.created_at_ms DESC, CAST(ta.frame_id AS INTEGER) DESC
         LIMIT 1",
        params![workstream_id],
        |row| {
            Ok(ScorerAction {
                id: row.get(0)?,
                frame_id: row.get(1)?,
                artifact_id: row.get(2)?,
                secondary_artifact_id: row.get(3)?,
                action_kind: row.get(4)?,
                action_role: row.get(5)?,
                confidence: row.get(6)?,
                reason: row.get(7)?,
                created_at_ms: row.get(8)?,
                collapse_count: row.get::<_, i64>(9)?.max(1),
                first_frame_id: row.get(10)?,
                last_frame_id: row.get(11)?,
                strongest_frame_id: row.get(12)?,
            })
        },
    )
    .optional()
    .map_err(to_string)
}

fn generate_continue_candidates(
    workstreams: &[ScorerWorkstream],
    current_focus: Option<&ContinueFocusSummary>,
) -> Vec<ScoredContinueCandidate> {
    let mut candidates = Vec::new();
    let mut seen = HashSet::new();
    for workstream in workstreams {
        let primary = primary_artifact_for_workstream(workstream);
        let last_action = workstream.last_meaningful_action.clone();
        let unresolved_kind = unresolved_kind(workstream.unresolved_signal.as_deref());
        let supporting_episode_id = latest_episode_id(workstream);
        match unresolved_kind.as_deref() {
            Some("visible_error_or_failure") => {
                let target = primary
                    .clone()
                    .or_else(|| action_artifact(workstream, &last_action));
                push_candidate(
                    &mut candidates,
                    &mut seen,
                    workstream,
                    target,
                    primary.clone(),
                    "resolve_error",
                    last_action.clone(),
                    supporting_episode_id.clone(),
                    "unresolved_error_signal",
                );
            }
            Some("draft_or_composer_active") => {
                let target = action_artifact(workstream, &last_action).or_else(|| primary.clone());
                push_candidate(
                    &mut candidates,
                    &mut seen,
                    workstream,
                    target,
                    primary.clone(),
                    "continue_reply",
                    last_action.clone(),
                    supporting_episode_id.clone(),
                    "draft_or_composer_signal",
                );
            }
            Some("verification_without_return") => {
                let target = action_artifact(workstream, &last_action).or_else(|| primary.clone());
                push_candidate(
                    &mut candidates,
                    &mut seen,
                    workstream,
                    target,
                    primary.clone(),
                    "verify_output",
                    last_action.clone(),
                    supporting_episode_id.clone(),
                    "verification_without_return",
                );
            }
            Some("branch_without_return") => {
                push_candidate(
                    &mut candidates,
                    &mut seen,
                    workstream,
                    primary.clone(),
                    primary.clone(),
                    "return_to_primary_artifact",
                    last_action.clone(),
                    supporting_episode_id.clone(),
                    "branch_page_is_evidence_return_to_origin",
                );
                if action_kind_is_search(last_action.as_ref()) && primary.is_none() {
                    push_candidate(
                        &mut candidates,
                        &mut seen,
                        workstream,
                        action_artifact(workstream, &last_action),
                        None,
                        "finish_search",
                        last_action.clone(),
                        supporting_episode_id.clone(),
                        "search_has_no_primary_target",
                    );
                }
            }
            Some("idle_after_progress") => {
                let kind = if primary
                    .as_ref()
                    .map(|artifact| is_editable_artifact(&artifact.artifact_kind))
                    .unwrap_or(false)
                {
                    "continue_edit"
                } else {
                    "return_to_primary_artifact"
                };
                push_candidate(
                    &mut candidates,
                    &mut seen,
                    workstream,
                    primary
                        .clone()
                        .or_else(|| action_artifact(workstream, &last_action)),
                    primary.clone(),
                    kind,
                    last_action.clone(),
                    supporting_episode_id.clone(),
                    "idle_after_meaningful_progress",
                );
            }
            _ => {}
        }

        if let Some(action) = &last_action {
            let target = action_artifact(workstream, &last_action).or_else(|| primary.clone());
            let (kind, reason) =
                candidate_kind_for_action(action, target.as_ref(), primary.as_ref());
            push_candidate(
                &mut candidates,
                &mut seen,
                workstream,
                target,
                primary.clone(),
                kind,
                last_action.clone(),
                supporting_episode_id.clone(),
                reason,
            );
        }

        if let Some(primary) = primary.clone() {
            let generic_kind = if primary.artifact_kind == "unknown"
                && last_action.is_none()
                && workstream.unresolved_signal.is_none()
            {
                "evidence_only"
            } else if primary.artifact_kind == "chat_conversation" {
                "resume_chat_reasoning"
            } else if primary.artifact_kind == "pdf" {
                "read_next_source"
            } else if is_editable_artifact(&primary.artifact_kind) {
                "continue_edit"
            } else {
                "return_to_primary_artifact"
            };
            push_candidate(
                &mut candidates,
                &mut seen,
                workstream,
                Some(primary.clone()),
                Some(primary),
                generic_kind,
                last_action.clone(),
                supporting_episode_id.clone(),
                "primary_artifact_fallback",
            );
        }

        if let Some(focus) = current_focus {
            if let Some(focus_artifact_id) = focus.artifact_id.as_deref() {
                if let Some(focus_artifact) = artifact_by_id(workstream, focus_artifact_id) {
                    let kind = current_focus_candidate_kind(&focus_artifact, last_action.as_ref());
                    push_candidate(
                        &mut candidates,
                        &mut seen,
                        workstream,
                        Some(focus_artifact),
                        primary.clone(),
                        kind,
                        last_action.clone(),
                        supporting_episode_id.clone(),
                        "current_focus_connected_to_workstream",
                    );
                }
            }
        }

        if !candidates
            .iter()
            .any(|candidate| candidate.workstream_id == workstream.id)
        {
            push_candidate(
                &mut candidates,
                &mut seen,
                workstream,
                primary.clone(),
                primary,
                "evidence_only",
                last_action,
                supporting_episode_id,
                "thin_semantic_workstream_fallback",
            );
        }
    }
    candidates
}

#[allow(clippy::too_many_arguments)]
fn push_candidate(
    candidates: &mut Vec<ScoredContinueCandidate>,
    seen: &mut HashSet<String>,
    workstream: &ScorerWorkstream,
    target_artifact: Option<ScorerArtifact>,
    resume_work_target: Option<ScorerArtifact>,
    candidate_kind: &str,
    last_meaningful_action: Option<ScorerAction>,
    supporting_episode_id: Option<String>,
    reason: &str,
) {
    let target_key = target_artifact
        .as_ref()
        .map(|artifact| artifact.id.clone())
        .unwrap_or_else(|| "none".to_string());
    let key = format!("{}:{}:{}", workstream.id, target_key, candidate_kind);
    if !seen.insert(key.clone()) {
        return;
    }
    let evidence_frame_id = last_meaningful_action
        .as_ref()
        .map(|action| action.frame_id.clone())
        .or_else(|| {
            target_artifact
                .as_ref()
                .and_then(|artifact| artifact.last_seen_frame_id.clone())
        })
        .or_else(|| latest_episode_frame_id(workstream));
    let id = format!("continue-candidate-{}", stable_hash(key.as_bytes()));
    candidates.push(ScoredContinueCandidate {
        id,
        workstream_id: workstream.id.clone(),
        target_artifact,
        candidate_kind: candidate_kind.to_string(),
        last_meaningful_action,
        evidence_frame_id,
        supporting_episode_id,
        score: 0.0,
        actionability_score: 0.0,
        primary_target_score: 0.0,
        unresolved_score: 0.0,
        branch_origin_score: 0.0,
        evidence_quality_score: 0.0,
        recency_score: 0.0,
        openability_score: 0.0,
        privacy_safety_score: 0.0,
        reason: Some(reason.to_string()),
        missing_evidence: Vec::new(),
        warnings: Vec::new(),
        resume_work_target,
    });
}

fn score_continue_candidates(
    candidates: &mut [ScoredContinueCandidate],
    workstreams: &[ScorerWorkstream],
    current_focus: Option<&ContinueFocusSummary>,
) {
    let latest_ts = workstreams
        .iter()
        .map(|workstream| workstream.last_active_timestamp_ms)
        .max()
        .unwrap_or_default();
    for candidate in candidates {
        let workstream = match workstreams
            .iter()
            .find(|workstream| workstream.id == candidate.workstream_id)
        {
            Some(workstream) => workstream,
            None => continue,
        };
        candidate.actionability_score = actionability_score(candidate);
        candidate.primary_target_score = primary_target_score(candidate, workstream);
        candidate.unresolved_score = unresolved_score(candidate, workstream);
        candidate.branch_origin_score = branch_origin_score(candidate, workstream);
        candidate.evidence_quality_score = evidence_quality_score(candidate, workstream);
        candidate.recency_score = recency_score(workstream, latest_ts);
        candidate.openability_score = openability_score(candidate.target_artifact.as_ref());
        candidate.privacy_safety_score = privacy_safety_score(candidate);
        candidate.missing_evidence = missing_evidence_for_candidate(candidate, workstream);
        candidate.warnings = warnings_for_candidate(candidate, workstream, current_focus);
        candidate.score = round_score(
            candidate.actionability_score * 0.24
                + candidate.primary_target_score * 0.20
                + candidate.unresolved_score * 0.18
                + candidate.branch_origin_score * 0.12
                + candidate.evidence_quality_score * 0.12
                + candidate.openability_score * 0.07
                + candidate.privacy_safety_score * 0.04
                + candidate.recency_score * 0.03,
        );
        candidate.score = round_score(confidence_cap_for_candidate(candidate, workstream));
    }
}

fn confidence_cap_for_candidate(
    candidate: &ScoredContinueCandidate,
    workstream: &ScorerWorkstream,
) -> f64 {
    let mut cap = 1.0;
    let mut capped = false;
    if candidate.candidate_kind == "evidence_only" {
        cap = f64::min(cap, 0.42);
        capped = true;
    }
    if candidate.target_artifact.is_none() {
        cap = f64::min(cap, 0.44);
        capped = true;
    }
    if let Some(target) = candidate.target_artifact.as_ref() {
        if target.artifact_kind == "unknown" || target.evidence_quality == "thin" {
            cap = f64::min(cap, 0.55);
            capped = true;
        }
        if target.openability == "frame_fallback" {
            cap = f64::min(cap, 0.58);
            capped = true;
        } else if target.browser_url.is_none() && target.document_path.is_none() {
            cap = f64::min(cap, 0.64);
            capped = true;
        }
        if is_smalltalk_artifact(target) {
            cap = f64::min(cap, 0.42);
            capped = true;
        }
    }
    if candidate.evidence_quality_score < 0.45 {
        cap = f64::min(cap, 0.52);
        capped = true;
    }
    if candidate
        .last_meaningful_action
        .as_ref()
        .is_some_and(|action| action.collapse_count > 1)
    {
        cap = f64::min(cap, 0.64);
        capped = true;
    }
    if candidate.last_meaningful_action.is_none() && workstream.unresolved_signal.is_none() {
        cap = f64::min(cap, 0.48);
        capped = true;
    }
    if candidate.warnings.iter().any(|warning| {
        matches!(
            warning.as_str(),
            "current_focus_mismatch" | "current_focus_differs_from_return_target"
        )
    }) {
        cap = f64::min(cap, 0.68);
        capped = true;
    }
    if candidate.recency_score >= 0.95
        && candidate.unresolved_score <= 0.42
        && candidate.primary_target_score <= 0.2
    {
        cap = f64::min(cap, 0.46);
        capped = true;
    }
    if capped {
        f64::min(candidate.score, cap)
    } else {
        candidate.score
    }
}

fn actionability_score(candidate: &ScoredContinueCandidate) -> f64 {
    match candidate.candidate_kind.as_str() {
        "resolve_error" => 0.96,
        "continue_edit" => 0.92,
        "continue_reply" => 0.9,
        "rerun_command" => 0.86,
        "verify_output" => 0.82,
        "resume_chat_reasoning" => 0.78,
        "read_next_source" => 0.68,
        "finish_search" => 0.58,
        "return_to_primary_artifact" => 0.72,
        "evidence_only" => 0.22,
        _ => 0.35,
    }
}

fn primary_target_score(candidate: &ScoredContinueCandidate, workstream: &ScorerWorkstream) -> f64 {
    let target_id = match candidate.target_artifact.as_ref() {
        Some(artifact) => artifact.id.as_str(),
        None => return 0.1,
    };
    if workstream.primary_artifact_id.as_deref() == Some(target_id) {
        return 1.0;
    }
    match durable_role_for_target(workstream, target_id) {
        Some("primary_target") => 0.94,
        Some("verification_surface") | Some("communication_surface") => 0.64,
        Some("support_source") => 0.46,
        Some("blocker_surface") => 0.7,
        Some("branch") => 0.18,
        Some("distractor") => 0.06,
        Some(_) => 0.2,
        None => 0.12,
    }
}

fn unresolved_score(candidate: &ScoredContinueCandidate, workstream: &ScorerWorkstream) -> f64 {
    let unresolved_kind = unresolved_kind(workstream.unresolved_signal.as_deref());
    match (
        candidate.candidate_kind.as_str(),
        unresolved_kind.as_deref(),
    ) {
        ("resolve_error", Some("visible_error_or_failure")) => 1.0,
        ("continue_reply", Some("draft_or_composer_active")) => 0.92,
        ("verify_output", Some("verification_without_return")) => 0.88,
        ("return_to_primary_artifact", Some("branch_without_return")) => 0.86,
        ("continue_edit", Some("idle_after_progress")) => 0.84,
        (_, Some(_)) => 0.62,
        (_, None) if candidate.last_meaningful_action.is_some() => 0.42,
        _ => 0.18,
    }
}

fn branch_origin_score(candidate: &ScoredContinueCandidate, workstream: &ScorerWorkstream) -> f64 {
    let has_recent_branch = workstream
        .last_meaningful_action
        .as_ref()
        .map(|action| matches!(action.action_kind.as_str(), "branching_away" | "searching"))
        .unwrap_or(false)
        || unresolved_kind(workstream.unresolved_signal.as_deref()).as_deref()
            == Some("branch_without_return");
    let target_id = candidate
        .target_artifact
        .as_ref()
        .map(|artifact| artifact.id.as_str());
    let target_role = target_id.and_then(|id| durable_role_for_target(workstream, id));
    if matches!(target_role, Some("branch")) && !branch_became_primary(candidate, workstream) {
        return 0.08;
    }
    if has_recent_branch
        && target_id.is_some()
        && (target_id == workstream.primary_artifact_id.as_deref()
            || matches!(target_role, Some("primary_target")))
    {
        return 1.0;
    }
    if matches!(target_role, Some("primary_target")) {
        return 0.78;
    }
    0.45
}

fn evidence_quality_score(
    candidate: &ScoredContinueCandidate,
    workstream: &ScorerWorkstream,
) -> f64 {
    let target_quality = candidate
        .target_artifact
        .as_ref()
        .map(|artifact| artifact.evidence_quality.as_str())
        .unwrap_or("thin");
    let episode_quality = workstream
        .episodes
        .last()
        .map(|episode| episode.evidence_quality.as_str())
        .unwrap_or("unknown");
    let quality = if target_quality == "strong" || episode_quality == "strong" {
        "strong"
    } else if target_quality == "medium" || episode_quality == "medium" {
        "medium"
    } else if target_quality == "thin" || episode_quality == "thin" {
        "thin"
    } else {
        "unknown"
    };
    let base = match quality {
        "strong" => 0.94,
        "medium" => 0.72,
        "thin" => 0.38,
        _ => 0.25,
    };
    let action_bonus = candidate
        .last_meaningful_action
        .as_ref()
        .map(|action| (action.confidence - 0.5).max(0.0) * 0.18)
        .unwrap_or(0.0);
    (base + action_bonus).clamp(0.0, 1.0)
}

fn recency_score(workstream: &ScorerWorkstream, latest_ts: i64) -> f64 {
    if latest_ts <= 0 {
        return 0.3;
    }
    let age_ms = latest_ts.saturating_sub(workstream.last_active_timestamp_ms);
    if age_ms <= 60_000 {
        1.0
    } else if age_ms <= 5 * 60_000 {
        0.82
    } else if age_ms <= 20 * 60_000 {
        0.58
    } else if age_ms <= 60 * 60_000 {
        0.32
    } else {
        0.12
    }
}

fn openability_score(target: Option<&ScorerArtifact>) -> f64 {
    match target {
        Some(artifact) if artifact.browser_url.is_some() || artifact.document_path.is_some() => 1.0,
        Some(artifact) if artifact.openability == "openable" => 0.9,
        Some(artifact) if artifact.openability == "frame_fallback" => 0.58,
        Some(_) => 0.28,
        None => 0.18,
    }
}

fn privacy_safety_score(candidate: &ScoredContinueCandidate) -> f64 {
    let Some(artifact) = candidate.target_artifact.as_ref() else {
        return 0.85;
    };
    if is_smalltalk_artifact(artifact) {
        return 0.12;
    }
    artifact
        .privacy_status
        .as_deref()
        .map(|status| {
            let lower = status.to_lowercase();
            if contains_any(
                &lower,
                &[
                    "skip_capture",
                    "never_send",
                    "blocked",
                    "sensitive",
                    "redacted",
                ],
            ) {
                0.55
            } else {
                1.0
            }
        })
        .unwrap_or(0.85)
}

fn is_smalltalk_artifact(artifact: &ScorerArtifact) -> bool {
    let haystack = [
        artifact.display_title.as_deref(),
        artifact.browser_url.as_deref(),
        artifact.document_path.as_deref(),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(" ")
    .to_lowercase();
    haystack.contains("smalltalk continue")
        || haystack.contains("com.smalltalk.app")
        || haystack.contains("smalltalk.app")
}

fn primary_artifact_for_workstream(workstream: &ScorerWorkstream) -> Option<ScorerArtifact> {
    workstream
        .primary_artifact_id
        .as_deref()
        .and_then(|id| artifact_by_id(workstream, id))
        .or_else(|| {
            workstream
                .artifacts
                .iter()
                .find(|artifact| artifact.durable_role == "primary_target")
                .map(|artifact| artifact.artifact.clone())
        })
        .or_else(|| {
            workstream
                .artifacts
                .iter()
                .filter(|artifact| artifact.durable_role != "distractor")
                .max_by(|left, right| {
                    left.importance_score
                        .partial_cmp(&right.importance_score)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|artifact| artifact.artifact.clone())
        })
}

fn artifact_by_id(workstream: &ScorerWorkstream, artifact_id: &str) -> Option<ScorerArtifact> {
    workstream
        .artifacts
        .iter()
        .find(|artifact| artifact.artifact.id == artifact_id)
        .map(|artifact| artifact.artifact.clone())
}

fn action_artifact(
    workstream: &ScorerWorkstream,
    action: &Option<ScorerAction>,
) -> Option<ScorerArtifact> {
    action
        .as_ref()
        .and_then(|action| action.artifact_id.as_deref())
        .and_then(|artifact_id| artifact_by_id(workstream, artifact_id))
}

fn durable_role_for_target<'a>(
    workstream: &'a ScorerWorkstream,
    artifact_id: &str,
) -> Option<&'a str> {
    workstream
        .artifacts
        .iter()
        .find(|artifact| artifact.artifact.id == artifact_id)
        .map(|artifact| artifact.durable_role.as_str())
}

fn branch_became_primary(
    candidate: &ScoredContinueCandidate,
    workstream: &ScorerWorkstream,
) -> bool {
    let target_id = candidate
        .target_artifact
        .as_ref()
        .map(|artifact| artifact.id.as_str());
    target_id.is_some()
        && target_id == workstream.primary_artifact_id.as_deref()
        && candidate
            .last_meaningful_action
            .as_ref()
            .map(|action| {
                matches!(
                    action.action_kind.as_str(),
                    "editing" | "composing" | "copying_evidence" | "returning_to_origin"
                )
            })
            .unwrap_or(false)
}

fn latest_episode_id(workstream: &ScorerWorkstream) -> Option<String> {
    workstream
        .episodes
        .iter()
        .max_by_key(|episode| episode.end_timestamp_ms)
        .map(|episode| episode.id.clone())
}

fn latest_episode_frame_id(workstream: &ScorerWorkstream) -> Option<String> {
    workstream
        .episodes
        .iter()
        .max_by_key(|episode| episode.end_timestamp_ms)
        .and_then(|episode| episode.end_frame_id.clone())
}

fn candidate_kind_for_action(
    action: &ScorerAction,
    target: Option<&ScorerArtifact>,
    primary: Option<&ScorerArtifact>,
) -> (&'static str, &'static str) {
    match action.action_kind.as_str() {
        "editing" | "idle_after_progress"
            if target
                .map(|artifact| is_editable_artifact(&artifact.artifact_kind))
                .unwrap_or(false)
                || primary
                    .map(|artifact| is_editable_artifact(&artifact.artifact_kind))
                    .unwrap_or(false) =>
        {
            ("continue_edit", "last_meaningful_edit")
        }
        "composing" | "messaging_interrupt" => ("continue_reply", "last_meaningful_composer"),
        "encountering_error" => ("resolve_error", "last_meaningful_error"),
        "running_command" => ("rerun_command", "last_meaningful_command"),
        "observing_command_output" | "reviewing_output" => {
            ("verify_output", "last_meaningful_output_review")
        }
        "searching" => ("finish_search", "last_meaningful_search"),
        "branching_away" => (
            "return_to_primary_artifact",
            "last_meaningful_support_branch",
        ),
        "reading"
            if target
                .map(|artifact| artifact.artifact_kind == "chat_conversation")
                .unwrap_or(false) =>
        {
            ("resume_chat_reasoning", "chat_reasoning_surface")
        }
        "reading" => ("read_next_source", "last_meaningful_reading"),
        _ => (
            "return_to_primary_artifact",
            "last_meaningful_action_fallback",
        ),
    }
}

fn current_focus_candidate_kind(
    artifact: &ScorerArtifact,
    last_action: Option<&ScorerAction>,
) -> &'static str {
    if is_editable_artifact(&artifact.artifact_kind) {
        "continue_edit"
    } else if matches!(
        artifact.artifact_kind.as_str(),
        "messaging" | "chat_conversation"
    ) {
        "continue_reply"
    } else if artifact.artifact_kind == "terminal" {
        "verify_output"
    } else if action_kind_is_search(last_action) {
        "finish_search"
    } else {
        "evidence_only"
    }
}

fn action_kind_is_search(action: Option<&ScorerAction>) -> bool {
    action
        .map(|action| matches!(action.action_kind.as_str(), "searching" | "branching_away"))
        .unwrap_or(false)
}

fn unresolved_kind(raw: Option<&str>) -> Option<String> {
    let raw = raw?;
    serde_json::from_str::<serde_json::Value>(raw)
        .ok()
        .and_then(|value| {
            value
                .get("kind")
                .and_then(|kind| kind.as_str())
                .map(str::to_string)
        })
        .or_else(|| Some(raw.to_string()))
}

fn unresolved_state_description(raw: Option<&str>) -> Option<String> {
    match unresolved_kind(raw)?.as_str() {
        "error_signal" => Some("An error or failure was visible.".to_string()),
        "unresolved_error_signal" | "visible_error_or_failure" => {
            Some("There appears to be an unresolved error.".to_string())
        }
        "draft_or_composer_active" => Some("A draft or composer appears unfinished.".to_string()),
        "verification_without_return" => {
            Some("Verification/output surface was observed without a return to the primary target.".to_string())
        }
        "branch_without_return" => Some("Support/search branch was recent and there is no observed return to the primary target.".to_string()),
        "idle_after_progress" => Some("User went idle after meaningful progress on the workstream.".to_string()),
        other => productize_continue_label(other).or_else(|| Some("Unresolved state still present.".to_string())),
    }
}

fn productize_continue_label(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    let key = unresolved_kind(Some(trimmed))
        .unwrap_or_else(|| trimmed.to_string())
        .to_lowercase()
        .replace('-', "_");
    let label = match key.as_str() {
        "error_signal" => "An error or failure was visible.",
        "unresolved_error_signal" => "There appears to be an unresolved error.",
        "primary_artifact_fallback" => "This looks like the main place to continue.",
        "last_meaningful_error" => "The last meaningful state was an error/blocker.",
        "frame_fallback" => "This target is based on visible screen evidence.",
        "last_meaningful_edit" => "Editing was the last meaningful activity.",
        "last_meaningful_composer" => "A draft or composer was active.",
        "last_meaningful_command" => "A command had just been run.",
        "last_meaningful_output_review" => "Output was being reviewed.",
        "last_meaningful_search" => "Search was the last meaningful branch.",
        "last_meaningful_reading" => "Reading was the last meaningful activity.",
        "last_meaningful_action_fallback" => "This is based on the last visible meaningful action.",
        "idle_after_meaningful_progress" => "Work paused after meaningful progress.",
        "draft_or_composer_signal" => "A draft or composer appears unfinished.",
        "verification_without_return" => "Verification has not been applied back to the target.",
        "branch_without_return" => "Search branch has not been applied back to the target.",
        "branch_page_is_evidence_return_to_origin" => {
            "The branch looks like evidence; return to the primary target."
        }
        "search_has_no_primary_target" => "The visible search branch is the best available target.",
        "thin_semantic_workstream_fallback" => {
            "Evidence is thin; this is the best available continuation."
        }
        "current_focus_connected_to_workstream" => "Current focus is connected to the workstream.",
        "smalltalk_self_observation_downranked" => {
            "Smalltalk's own UI was treated as low-value evidence."
        }
        "current_focus_mismatch" | "current_focus_differs_from_return_target" => {
            "Current screen is not the return target."
        }
        "no_last_meaningful_action" => "No clear last action was captured.",
        "no_direct_url_or_document_path" | "no_openable_target" => {
            "No directly openable target was found."
        }
        "thin_evidence" | "thin_evidence_quality" => "Evidence is thin.",
        "no_candidate_generated" => "No continuation candidate could be generated yet.",
        _ if trimmed.starts_with('{') => {
            return Some("Unresolved state still present.".to_string())
        }
        _ if key.contains("_id") || key.contains("json") => return None,
        _ => return Some(sentence_from_internal_label(trimmed)),
    };
    Some(label.to_string())
}

fn sentence_from_internal_label(raw: &str) -> String {
    let mut sentence = raw.replace('_', " ").replace('-', " ");
    sentence = sentence.split_whitespace().collect::<Vec<_>>().join(" ");
    if sentence.is_empty() {
        return "Local evidence is thin.".to_string();
    }
    let mut chars = sentence.chars();
    let first = chars
        .next()
        .map(|ch| ch.to_uppercase().collect::<String>())
        .unwrap_or_default();
    let rest = chars.collect::<String>();
    format!("{}{}.", first, rest.trim_end_matches('.'))
}

fn missing_evidence_for_candidate(
    candidate: &ScoredContinueCandidate,
    workstream: &ScorerWorkstream,
) -> Vec<String> {
    let mut missing = Vec::new();
    if candidate.target_artifact.is_none() {
        missing.push("no_valid_target_artifact".to_string());
    }
    if candidate.last_meaningful_action.is_none() {
        missing.push("no_last_meaningful_action".to_string());
    }
    if workstream.unresolved_signal.is_none() {
        missing.push("no_unresolved_signal".to_string());
    }
    if candidate
        .target_artifact
        .as_ref()
        .map(|artifact| artifact.browser_url.is_none() && artifact.document_path.is_none())
        .unwrap_or(true)
    {
        missing.push("no_direct_url_or_document_path".to_string());
    }
    if candidate.evidence_quality_score < 0.45 {
        missing.push("thin_evidence_quality".to_string());
    }
    missing
}

fn warnings_for_candidate(
    candidate: &ScoredContinueCandidate,
    workstream: &ScorerWorkstream,
    current_focus: Option<&ContinueFocusSummary>,
) -> Vec<String> {
    let mut warnings = Vec::new();
    if candidate.score < 0.45 {
        warnings.push("thin_evidence".to_string());
    }
    if let Some(role) = candidate
        .target_artifact
        .as_ref()
        .and_then(|artifact| durable_role_for_target(workstream, &artifact.id))
    {
        if role == "branch" && !branch_became_primary(candidate, workstream) {
            warnings.push("branch_surface_is_evidence_not_default_return_target".to_string());
        }
    }
    if candidate.privacy_safety_score < 0.7 {
        if candidate
            .target_artifact
            .as_ref()
            .is_some_and(is_smalltalk_artifact)
        {
            warnings.push("smalltalk_self_observation_downranked".to_string());
        } else {
            warnings.push("privacy_sensitive_or_redacted_target_local_only".to_string());
        }
    }
    if let (Some(focus), Some(target)) = (current_focus, candidate.target_artifact.as_ref()) {
        if focus.artifact_id.as_deref() != Some(target.id.as_str()) {
            warnings.push("current_focus_mismatch".to_string());
        }
    }
    warnings
}

fn next_action_for_candidate(candidate: &ScoredContinueCandidate) -> String {
    let target = candidate
        .target_artifact
        .as_ref()
        .and_then(|artifact| artifact_title_for_scorer(artifact))
        .unwrap_or_else(|| "the captured evidence".to_string());
    match candidate.candidate_kind.as_str() {
        "resolve_error" => format!("Return to {} and resolve the visible error.", target),
        "continue_edit" => format!("Return to {} and continue the last edit.", target),
        "continue_reply" => format!("Return to {} and finish the draft or reply.", target),
        "verify_output" => format!("Inspect {} and verify the last output.", target),
        "rerun_command" => format!(
            "Return to {} and rerun or continue the command flow.",
            target
        ),
        "read_next_source" => format!("Continue reading {}", target),
        "finish_search" => format!(
            "Finish the search or use it to return to the primary work target from {}.",
            target
        ),
        "resume_chat_reasoning" => {
            format!("Return to {} and continue the reasoning thread.", target)
        }
        "return_to_primary_artifact" => format!(
            "Return to {} and continue from the primary work target.",
            target
        ),
        _ => format!(
            "Use the local evidence around {} as the continuation point.",
            target
        ),
    }
}

fn validation_status_for_candidate(candidate: Option<&ScoredContinueCandidate>) -> String {
    match candidate {
        Some(candidate) if candidate.candidate_kind == "evidence_only" => "fallback".to_string(),
        Some(candidate) if candidate.score >= 0.68 && candidate.target_artifact.is_some() => {
            "valid".to_string()
        }
        Some(candidate) if candidate.score >= 0.42 => "thin_evidence".to_string(),
        Some(_) => "fallback".to_string(),
        None => "thin_evidence".to_string(),
    }
}

fn build_micro_inference_pack(
    conn: &Connection,
    current_focus: Option<&ContinueFocusSummary>,
    workstreams: &[ScorerWorkstream],
    candidates: &[ScoredContinueCandidate],
    candidate_limit: usize,
) -> Result<ContinueMicroInferencePack, String> {
    let selected_candidates = candidates
        .iter()
        .take(candidate_limit)
        .cloned()
        .collect::<Vec<_>>();
    let selected_workstream_ids = selected_candidates
        .iter()
        .map(|candidate| candidate.workstream_id.clone())
        .collect::<HashSet<_>>();
    let pack_workstreams = workstreams
        .iter()
        .filter(|workstream| selected_workstream_ids.contains(&workstream.id))
        .take(8)
        .map(|workstream| ContinuePackWorkstream {
            id: workstream.id.clone(),
            title_candidate: workstream.title_candidate.clone(),
            state: workstream.state.clone(),
            primary_artifact_id: workstream.primary_artifact_id.clone(),
            primary_artifact_title: workstream
                .primary_artifact_id
                .as_ref()
                .and_then(|id| artifact_by_id(workstream, id))
                .and_then(|artifact| artifact_title_for_scorer(&artifact)),
            unresolved_signal: workstream.unresolved_signal.clone(),
            confidence: round_score(workstream.confidence),
        })
        .collect::<Vec<_>>();

    let mut artifact_roles = Vec::new();
    for workstream in workstreams
        .iter()
        .filter(|workstream| selected_workstream_ids.contains(&workstream.id))
    {
        for artifact in workstream.artifacts.iter().take(8) {
            artifact_roles.push(ContinuePackArtifactRole {
                workstream_id: workstream.id.clone(),
                artifact_id: artifact.artifact.id.clone(),
                role: artifact.durable_role.clone(),
                title: artifact_title_for_scorer(&artifact.artifact),
                kind: artifact.artifact.artifact_kind.clone(),
            });
        }
    }

    let pack_candidates = selected_candidates
        .iter()
        .map(|candidate| {
            let workstream = workstreams
                .iter()
                .find(|workstream| workstream.id == candidate.workstream_id);
            ContinuePackCandidate {
                id: candidate.id.clone(),
                workstream_id: candidate.workstream_id.clone(),
                candidate_kind: candidate.candidate_kind.clone(),
                target_artifact_id: candidate
                    .target_artifact
                    .as_ref()
                    .map(|artifact| artifact.id.clone()),
                target_title: candidate
                    .target_artifact
                    .as_ref()
                    .and_then(artifact_title_for_scorer),
                target_kind: candidate
                    .target_artifact
                    .as_ref()
                    .map(|artifact| artifact.artifact_kind.clone()),
                target_url_available: candidate
                    .target_artifact
                    .as_ref()
                    .and_then(|artifact| artifact.browser_url.as_ref())
                    .is_some(),
                target_path_available: candidate
                    .target_artifact
                    .as_ref()
                    .and_then(|artifact| artifact.document_path.as_ref())
                    .is_some(),
                local_score: round_score(candidate.score),
                score_components: candidate_summary(candidate).components,
                last_meaningful_action: candidate
                    .last_meaningful_action
                    .as_ref()
                    .map(action_summary),
                unresolved_state_reason: workstream.and_then(|workstream| {
                    unresolved_state_description(workstream.unresolved_signal.as_deref())
                }),
                evidence_frame_id: candidate.evidence_frame_id.clone(),
                evidence_action_id: candidate
                    .last_meaningful_action
                    .as_ref()
                    .map(|action| action.id.clone()),
                evidence_episode_id: candidate.supporting_episode_id.clone(),
                missing_evidence: candidate.missing_evidence.clone(),
                local_reason: candidate.reason.clone(),
            }
        })
        .collect::<Vec<_>>();

    let breadcrumbs = load_pack_breadcrumbs(conn, &selected_workstream_ids)?;

    Ok(ContinueMicroInferencePack {
        schema: "smalltalk.continue_micro_inference_pack.v1".to_string(),
        instructions: "Candidate IDs provided are the only valid choices. Unsupported next actions must be null or explicitly cautious. Do not invent artifacts, URLs, paths, titles, evidence ids, or user intent. Current focus is factual; return target is the selected local candidate target.".to_string(),
        current_focus: current_focus.cloned(),
        workstreams: pack_workstreams,
        candidates: pack_candidates,
        artifact_roles,
        breadcrumbs,
    })
}

fn continue_openai_config(model_override: Option<String>) -> Result<ContinueOpenAiConfig, String> {
    let project_env = project_dotenv_values()?;
    let process_key = std::env::var("OPENAI_API_KEY").ok().and_then(non_empty);
    let project_key = project_env
        .get("OPENAI_API_KEY")
        .cloned()
        .and_then(non_empty);
    let api_key = process_key.or(project_key);
    let model = model_override
        .and_then(non_empty)
        .or_else(|| {
            std::env::var("SMALLTALK_CONTINUE_OPENAI_MODEL")
                .ok()
                .and_then(non_empty)
        })
        .or_else(|| {
            std::env::var("SMALLTALK_OPENAI_MODEL")
                .ok()
                .and_then(non_empty)
        })
        .or_else(|| std::env::var("OPENAI_MODEL").ok().and_then(non_empty))
        .or_else(|| {
            project_env
                .get("SMALLTALK_CONTINUE_OPENAI_MODEL")
                .cloned()
                .and_then(non_empty)
        })
        .or_else(|| {
            project_env
                .get("SMALLTALK_OPENAI_MODEL")
                .cloned()
                .and_then(non_empty)
        })
        .or_else(|| project_env.get("OPENAI_MODEL").cloned().and_then(non_empty))
        .unwrap_or_else(|| "gpt-4.1-mini".to_string());
    Ok(ContinueOpenAiConfig { api_key, model })
}

#[derive(Debug, Clone)]
struct ContinueOpenAiConfig {
    api_key: Option<String>,
    model: String,
}

fn run_continue_micro_inference(
    api_key: &str,
    model: &str,
    pack: &ContinueMicroInferencePack,
) -> Result<ValidatedMicroInference, String> {
    let request = build_continue_openai_request(model, pack)?;
    let response = call_openai_responses(api_key, &request)?;
    let response_id = response
        .get("id")
        .and_then(Value::as_str)
        .map(|value| value.to_string());
    let output = parse_continue_micro_inference_response(&response)?;
    Ok(ValidatedMicroInference {
        output,
        response_id,
    })
}

fn build_continue_openai_request(
    model: &str,
    pack: &ContinueMicroInferencePack,
) -> Result<Value, String> {
    Ok(serde_json::json!({
        "model": model,
        "store": false,
        "max_output_tokens": 900,
        "text": {
            "format": {
                "type": "json_schema",
                "name": "smalltalk_continue_micro_inference",
                "strict": true,
                "schema": continue_micro_inference_schema(),
            }
        },
        "input": [
            {
                "role": "system",
                "content": [
                    {
                        "type": "input_text",
                        "text": "You are Smalltalk's bounded Continue adjudicator. Use only supplied candidate ids and evidence facts. You may rank and phrase, but you must not invent artifacts, paths, URLs, evidence, or intent. Select low confidence when evidence is thin."
                    }
                ]
            },
            {
                "role": "user",
                "content": [
                    {
                        "type": "input_text",
                        "text": serde_json::to_string(pack).map_err(to_string)?,
                    }
                ]
            }
        ]
    }))
}

fn continue_micro_inference_schema() -> Value {
    serde_json::json!({
        "type": "object",
        "additionalProperties": false,
        "required": [
            "selected_candidate_id",
            "selected_workstream_id",
            "intent_label",
            "next_action",
            "reason",
            "confidence",
            "uncertainty_notes"
        ],
        "properties": {
            "selected_candidate_id": {"type": "string"},
            "selected_workstream_id": {"type": "string"},
            "intent_label": {"type": "string"},
            "next_action": {"type": ["string", "null"]},
            "reason": {"type": "string"},
            "confidence": {"type": "string", "enum": ["low", "medium", "high"]},
            "uncertainty_notes": {"type": ["string", "null"]}
        }
    })
}

fn parse_continue_micro_inference_response(
    response: &Value,
) -> Result<ContinueMicroInferenceOutput, String> {
    let text = response
        .get("output_text")
        .and_then(Value::as_str)
        .map(|value| value.to_string())
        .or_else(|| {
            response
                .get("output")
                .and_then(Value::as_array)
                .and_then(|items| {
                    let mut chunks = Vec::new();
                    for item in items {
                        if let Some(content) = item.get("content").and_then(Value::as_array) {
                            for part in content {
                                if let Some(text) = part.get("text").and_then(Value::as_str) {
                                    chunks.push(text.to_string());
                                }
                            }
                        }
                    }
                    if chunks.is_empty() {
                        None
                    } else {
                        Some(chunks.join(""))
                    }
                })
        })
        .ok_or_else(|| "OpenAI response did not include output_text".to_string())?;
    serde_json::from_str::<ContinueMicroInferenceOutput>(&text).map_err(|error| {
        format!(
            "OpenAI micro-inference returned malformed JSON: {} ({} chars received)",
            error,
            text.len()
        )
    })
}

fn validate_micro_inference_output(
    output: &ContinueMicroInferenceOutput,
    candidates: &[ScoredContinueCandidate],
    pack: &ContinueMicroInferencePack,
) -> Result<ScoredContinueCandidate, Vec<String>> {
    let mut failures = Vec::new();
    let candidate = candidates
        .iter()
        .find(|candidate| candidate.id == output.selected_candidate_id)
        .cloned();
    let Some(candidate) = candidate else {
        return Err(vec![format!(
            "selected_candidate_id_not_in_pack:{}",
            output.selected_candidate_id
        )]);
    };

    if candidate.workstream_id != output.selected_workstream_id {
        failures.push("selected_workstream_id_mismatch".to_string());
    }
    if !pack
        .candidates
        .iter()
        .any(|pack_candidate| pack_candidate.id == output.selected_candidate_id)
    {
        failures.push("selected_candidate_not_sent_to_model".to_string());
    }
    if unsupported_locator_in_model_output(output) {
        failures.push("model_output_contains_unsupported_url_or_path".to_string());
    }
    if output.next_action.as_ref().is_some_and(|action| {
        action.trim().is_empty()
            || action.len() > 220
            || (candidate.candidate_kind == "evidence_only"
                && output.confidence != "low"
                && !action.to_lowercase().contains("evidence"))
    }) {
        failures.push("next_action_incompatible_with_candidate".to_string());
    }
    if output.confidence == "high"
        && (candidate.score < 0.7
            || !candidate.missing_evidence.is_empty()
            || candidate.evidence_quality_score < 0.58)
    {
        failures.push("high_confidence_with_thin_local_evidence".to_string());
    }
    if branch_support_target_requires_local_candidate_guard(&candidate) {
        failures.push("branch_or_support_target_promoted_without_strong_local_score".to_string());
    }

    if failures.is_empty() {
        Ok(candidate)
    } else {
        Err(failures)
    }
}

fn unsupported_locator_in_model_output(output: &ContinueMicroInferenceOutput) -> bool {
    let joined = [
        output.intent_label.as_str(),
        output.next_action.as_deref().unwrap_or(""),
        output.reason.as_str(),
        output.uncertainty_notes.as_deref().unwrap_or(""),
    ]
    .join(" ")
    .to_lowercase();
    joined.contains("http://")
        || joined.contains("https://")
        || joined.contains("/users/")
        || joined.contains("file://")
        || joined.contains(".com/")
        || joined.contains(".app/")
}

fn branch_support_target_requires_local_candidate_guard(
    candidate: &ScoredContinueCandidate,
) -> bool {
    let Some(target) = candidate.target_artifact.as_ref() else {
        return false;
    };
    let support_like = matches!(
        target.artifact_kind.as_str(),
        "browser_tab" | "chat_conversation" | "terminal" | "messaging"
    );
    support_like
        && candidate
            .resume_work_target
            .as_ref()
            .is_some_and(|resume_target| resume_target.id != target.id && candidate.score < 0.75)
}

fn confidence_from_micro_output(label: &str, local_score: f64) -> f64 {
    let model_score = match label {
        "high" => 0.82,
        "medium" => 0.62,
        _ => 0.35,
    };
    round_score(f64::min(model_score, local_score + 0.08))
}

fn call_openai_responses(api_key: &str, request: &Value) -> Result<Value, String> {
    let temp_dir = std::env::temp_dir().join(format!(
        "smalltalk-continue-openai-{}",
        current_time_millis()
    ));
    fs::create_dir_all(&temp_dir).map_err(to_string)?;
    #[cfg(unix)]
    fs::set_permissions(&temp_dir, fs::Permissions::from_mode(0o700)).map_err(to_string)?;

    let request_path = temp_dir.join("request.json");
    let config_path = temp_dir.join("curl.conf");
    write_private_file(
        &request_path,
        &serde_json::to_vec(request).map_err(to_string)?,
    )?;
    let config = format!(
        "url = \"https://api.openai.com/v1/responses\"\nrequest = \"POST\"\nsilent\nshow-error\nfail-with-body\nconnect-timeout = 20\nmax-time = 90\nretry = 1\nretry-delay = 1\nretry-all-errors\nheader = \"Content-Type: application/json\"\nheader = \"Authorization: Bearer {}\"\n",
        curl_config_escape(api_key)
    );
    write_private_file(&config_path, config.as_bytes())?;
    let output = Command::new("/usr/bin/curl")
        .arg("--config")
        .arg(&config_path)
        .arg("--data-binary")
        .arg(format!("@{}", request_path.to_string_lossy()))
        .output()
        .map_err(to_string);
    let _ = fs::remove_dir_all(&temp_dir);
    let output = output?;
    if !output.status.success() {
        return Err(format!(
            "curl exited with {}: {}{}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim(),
            if output.stdout.is_empty() {
                "".to_string()
            } else {
                format!(" body={}", String::from_utf8_lossy(&output.stdout).trim())
            }
        ));
    }
    serde_json::from_slice(&output.stdout).map_err(to_string)
}

fn write_private_file(path: &Path, bytes: &[u8]) -> Result<(), String> {
    fs::write(path, bytes).map_err(to_string)?;
    #[cfg(unix)]
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).map_err(to_string)?;
    Ok(())
}

fn curl_config_escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn project_dotenv_values() -> Result<HashMap<String, String>, String> {
    let env_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or_else(|| "failed to resolve project root".to_string())?
        .join(".env");
    if !env_path.exists() {
        return Ok(HashMap::new());
    }
    let raw = fs::read_to_string(env_path).map_err(to_string)?;
    Ok(parse_dotenv_values(&raw))
}

fn parse_dotenv_values(raw: &str) -> HashMap<String, String> {
    let mut values = HashMap::new();
    for line in raw.lines() {
        let mut trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("export ") {
            trimmed = rest.trim_start();
        }
        let Some((key, value)) = trimmed.split_once('=') else {
            continue;
        };
        let key = key.trim();
        if key.is_empty()
            || !key
                .chars()
                .all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
        {
            continue;
        }
        values.insert(key.to_string(), parse_dotenv_value(value.trim()));
    }
    values
}

fn parse_dotenv_value(value: &str) -> String {
    if value.len() >= 2 {
        let bytes = value.as_bytes();
        let quote = bytes[0];
        if (quote == b'"' || quote == b'\'') && bytes[value.len() - 1] == quote {
            let inner = &value[1..value.len() - 1];
            return if quote == b'"' {
                unescape_dotenv_double_quoted(inner)
            } else {
                inner.to_string()
            };
        }
    }
    value.to_string()
}

fn unescape_dotenv_double_quoted(value: &str) -> String {
    let mut output = String::new();
    let mut chars = value.chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.next() {
                Some('n') => output.push('\n'),
                Some('r') => output.push('\r'),
                Some('t') => output.push('\t'),
                Some('"') => output.push('"'),
                Some('\\') => output.push('\\'),
                Some(other) => {
                    output.push('\\');
                    output.push(other);
                }
                None => output.push('\\'),
            }
        } else {
            output.push(ch);
        }
    }
    output
}

fn non_empty(value: String) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn load_pack_breadcrumbs(
    conn: &Connection,
    workstream_ids: &HashSet<String>,
) -> Result<Vec<ContinuePackBreadcrumb>, String> {
    if workstream_ids.is_empty() || !table_exists(conn, "continue_breadcrumbs")? {
        return Ok(Vec::new());
    }
    let mut breadcrumbs = Vec::new();
    for workstream_id in workstream_ids {
        let mut stmt = conn
            .prepare(
                "SELECT workstream_id, text, created_at_ms
                 FROM continue_breadcrumbs
                 WHERE workstream_id = ?1
                 ORDER BY created_at_ms DESC
                 LIMIT 3",
            )
            .map_err(to_string)?;
        let rows = stmt
            .query_map(params![workstream_id], |row| {
                Ok(ContinuePackBreadcrumb {
                    workstream_id: row.get(0)?,
                    text: row.get(1)?,
                    created_at_ms: row.get(2)?,
                })
            })
            .map_err(to_string)?;
        for row in rows {
            breadcrumbs.push(row.map_err(to_string)?);
        }
    }
    Ok(breadcrumbs)
}

fn infer_pending_continue_feedback(
    conn: &Connection,
    window_ms: i64,
) -> Result<Vec<ContinueFeedbackEventResult>, String> {
    if !table_exists(conn, "continue_decisions")? {
        return Ok(Vec::new());
    }
    let mut stmt = conn
        .prepare(
            "SELECT id
             FROM continue_decisions d
             WHERE NOT EXISTS (
               SELECT 1 FROM continue_feedback_events f WHERE f.decision_id = d.id
             )
             ORDER BY requested_at_ms ASC
             LIMIT 20",
        )
        .map_err(to_string)?;
    let ids = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(to_string)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(to_string)?;
    let mut events = Vec::new();
    for decision_id in ids {
        if let Some(event) = infer_feedback_for_decision(conn, &decision_id, window_ms)? {
            events.push(event);
        }
    }
    Ok(events)
}

fn infer_feedback_for_decision(
    conn: &Connection,
    decision_id: &str,
    window_ms: i64,
) -> Result<Option<ContinueFeedbackEventResult>, String> {
    let decision = conn
        .query_row(
            "SELECT requested_at_ms, return_target_artifact_id, current_focus_artifact_id
             FROM continue_decisions
             WHERE id = ?1",
            params![decision_id],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, Option<String>>(2)?,
                ))
            },
        )
        .optional()
        .map_err(to_string)?;
    let Some((requested_at_ms, target_artifact_id, current_focus_artifact_id)) = decision else {
        return Ok(None);
    };
    let cutoff_ms = requested_at_ms + window_ms.max(60_000);
    let mut stmt = conn
        .prepare(
            "SELECT o.frame_id, o.artifact_id, o.timestamp_ms
             FROM continue_artifact_observations o
             WHERE o.timestamp_ms > ?1 AND o.timestamp_ms <= ?2
             ORDER BY o.timestamp_ms ASC
             LIMIT 30",
        )
        .map_err(to_string)?;
    let observations = stmt
        .query_map(params![requested_at_ms, cutoff_ms], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
            ))
        })
        .map_err(to_string)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(to_string)?;

    let Some((first_frame_id, first_artifact_id, first_timestamp_ms)) =
        observations.first().cloned()
    else {
        return insert_feedback_event(
            conn,
            decision_id,
            "ignored",
            None,
            target_artifact_id.as_deref(),
            None,
            cutoff_ms,
            0.55,
            "no post-decision target activity observed",
        )
        .map(Some);
    };

    let target_seen = target_artifact_id.as_ref().and_then(|target| {
        observations
            .iter()
            .find(|(_, artifact_id, _)| artifact_id == target)
            .cloned()
    });
    let meaningful_target_action = if let Some(target) = target_artifact_id.as_deref() {
        conn.query_row(
            "SELECT EXISTS(
               SELECT 1 FROM continue_task_actions
               WHERE artifact_id = ?1
                 AND created_at_ms > ?2
                 AND created_at_ms <= ?3
                 AND action_kind NOT IN ('navigating', 'switching_context', 'unknown')
             )",
            params![target, requested_at_ms, cutoff_ms],
            |row| row.get::<_, i64>(0),
        )
        .map_err(to_string)?
            != 0
    } else {
        false
    };

    if let (Some(target), Some((frame_id, artifact_id, timestamp_ms))) =
        (target_artifact_id.as_ref(), target_seen)
    {
        let later_other = observations.iter().any(|(_, next_artifact_id, next_ts)| {
            next_ts > &timestamp_ms && next_artifact_id != target
        });
        if meaningful_target_action || !later_other {
            return insert_feedback_event(
                conn,
                decision_id,
                "accepted",
                Some(&frame_id),
                Some(target),
                Some(&artifact_id),
                timestamp_ms,
                if meaningful_target_action { 0.82 } else { 0.62 },
                "target artifact was revisited after Continue",
            )
            .map(Some);
        }
        return insert_feedback_event(
            conn,
            decision_id,
            "rejected",
            Some(&frame_id),
            Some(target),
            Some(&artifact_id),
            timestamp_ms,
            0.68,
            "target artifact was opened then quickly left without meaningful action",
        )
        .map(Some);
    }

    if target_artifact_id.is_some() && Some(first_artifact_id.clone()) != current_focus_artifact_id
    {
        return insert_feedback_event(
            conn,
            decision_id,
            "corrected",
            Some(&first_frame_id),
            target_artifact_id.as_deref(),
            Some(&first_artifact_id),
            first_timestamp_ms,
            0.7,
            "another artifact was chosen after Continue",
        )
        .map(Some);
    }

    insert_feedback_event(
        conn,
        decision_id,
        "auto_resumed",
        Some(&first_frame_id),
        target_artifact_id.as_deref(),
        Some(&first_artifact_id),
        first_timestamp_ms,
        0.6,
        "user naturally continued on the active work artifact",
    )
    .map(Some)
}

#[allow(clippy::too_many_arguments)]
fn insert_feedback_event(
    conn: &Connection,
    decision_id: &str,
    event_kind: &str,
    observed_frame_id: Option<&str>,
    target_artifact_id: Option<&str>,
    chosen_artifact_id: Option<&str>,
    timestamp_ms: i64,
    confidence: f64,
    reason: &str,
) -> Result<ContinueFeedbackEventResult, String> {
    let id = format!(
        "continue-feedback-{}",
        stable_hash(format!("{}:{}:{}", decision_id, event_kind, timestamp_ms).as_bytes())
    );
    conn.execute(
        "INSERT OR IGNORE INTO continue_feedback_events (
            id, decision_id, event_kind, observed_frame_id, target_artifact_id,
            chosen_artifact_id, timestamp_ms, confidence, reason,
            selected_candidate_id, workstream_id, note, source
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
        params![
            id,
            decision_id,
            event_kind,
            observed_frame_id,
            target_artifact_id,
            chosen_artifact_id,
            timestamp_ms,
            confidence,
            reason,
            Option::<String>::None,
            Option::<String>::None,
            Option::<String>::None,
            "inferred",
        ],
    )
    .map_err(to_string)?;
    Ok(ContinueFeedbackEventResult {
        id,
        decision_id: Some(decision_id.to_string()),
        selected_candidate_id: None,
        workstream_id: None,
        event_kind: event_kind.to_string(),
        observed_frame_id: observed_frame_id.map(|value| value.to_string()),
        target_artifact_id: target_artifact_id.map(|value| value.to_string()),
        chosen_artifact_id: chosen_artifact_id.map(|value| value.to_string()),
        timestamp_ms,
        confidence: round_score(confidence),
        reason: Some(reason.to_string()),
        note: None,
        source: Some("inferred".to_string()),
    })
}

fn default_continue_eval_fixture() -> ContinueEvalFixture {
    let cases = vec![
        eval_case(
            "edit_docs_branch_idle",
            "Edit -> docs/search branch -> idle",
            "code",
            Some("search"),
            vec![
                eval_candidate("c-code", "w-code", Some("code"), 0.84, "strong"),
                eval_candidate("c-search", "w-code", Some("search"), 0.52, "medium"),
            ],
        ),
        eval_case(
            "work_slack_interrupt_idle",
            "Work target -> Slack/messaging interruption -> idle",
            "doc",
            Some("slack"),
            vec![
                eval_candidate("c-doc", "w-doc", Some("doc"), 0.8, "strong"),
                eval_candidate("c-slack", "w-slack", Some("slack"), 0.46, "medium"),
            ],
        ),
        eval_case(
            "error_search_no_return",
            "Error -> search/docs -> no return",
            "origin-error",
            Some("search"),
            vec![
                eval_candidate("c-error", "w-error", Some("origin-error"), 0.82, "strong"),
                eval_candidate("c-search", "w-error", Some("search"), 0.55, "medium"),
            ],
        ),
        eval_case(
            "source_copied_target_edited",
            "Source copied -> target edited",
            "target-doc",
            Some("target-doc"),
            vec![
                eval_candidate("c-target", "w-copy", Some("target-doc"), 0.86, "strong"),
                eval_candidate("c-source", "w-copy", Some("source-doc"), 0.5, "medium"),
            ],
        ),
        eval_case(
            "source_copied_no_target_edit",
            "Source copied -> no target edit",
            "source-doc",
            Some("source-doc"),
            vec![eval_candidate_with_missing(
                "c-source",
                "w-copy-thin",
                Some("source-doc"),
                0.43,
                "thin",
                vec!["no_target_edit"],
            )],
        ),
        eval_case(
            "ai_chat_support",
            "AI Chat as support",
            "code",
            Some("chat"),
            vec![
                eval_candidate("c-code", "w-ai-support", Some("code"), 0.78, "strong"),
                eval_candidate("c-chat", "w-ai-support", Some("chat"), 0.48, "medium"),
            ],
        ),
        eval_case(
            "ai_chat_primary",
            "AI Chat as primary",
            "chat",
            Some("chat"),
            vec![
                eval_candidate("c-chat", "w-chat", Some("chat"), 0.79, "strong"),
                eval_candidate("c-notes", "w-chat", Some("notes"), 0.41, "thin"),
            ],
        ),
        eval_case(
            "terminal_verification_after_edit",
            "Terminal verification after edit",
            "code",
            Some("terminal"),
            vec![
                eval_candidate("c-code", "w-verify", Some("code"), 0.83, "strong"),
                eval_candidate("c-terminal", "w-verify", Some("terminal"), 0.58, "medium"),
            ],
        ),
        eval_case(
            "terminal_error_blocker",
            "Terminal error as blocker",
            "terminal",
            Some("terminal"),
            vec![
                eval_candidate(
                    "c-terminal-error",
                    "w-terminal",
                    Some("terminal"),
                    0.81,
                    "strong",
                ),
                eval_candidate("c-code", "w-terminal", Some("code"), 0.68, "medium"),
            ],
        ),
        eval_case(
            "thin_ocr_only",
            "Thin OCR-only evidence",
            "unknown",
            Some("unknown"),
            vec![eval_candidate_with_missing(
                "c-thin",
                "w-thin",
                Some("unknown"),
                0.38,
                "thin",
                vec!["thin_ocr_only"],
            )],
        ),
        eval_case(
            "nested_dependent_task",
            "Nested dependent task",
            "subtask",
            Some("parent"),
            vec![
                eval_candidate("c-subtask", "w-parent", Some("subtask"), 0.82, "strong"),
                eval_candidate("c-parent", "w-parent", Some("parent"), 0.61, "medium"),
            ],
        ),
        eval_case(
            "hardqa_repeated_error_collapses_to_blocker",
            "Repeated adjacent error frames should still rank one blocker target",
            "origin-error",
            Some("origin-error"),
            vec![
                eval_candidate(
                    "c-error-blocker",
                    "w-error-repeat",
                    Some("origin-error"),
                    0.82,
                    "strong",
                ),
                eval_candidate(
                    "c-error-duplicate",
                    "w-error-repeat",
                    Some("duplicate-error-row"),
                    0.44,
                    "thin",
                ),
            ],
        ),
        eval_case(
            "hardqa_smalltalk_self_not_primary",
            "Smalltalk UI self-capture should not become the primary workstream",
            "code",
            Some("smalltalk"),
            vec![
                eval_candidate("c-code-target", "w-code", Some("code"), 0.74, "medium"),
                eval_candidate(
                    "c-smalltalk-self",
                    "w-smalltalk",
                    Some("smalltalk"),
                    0.32,
                    "thin",
                ),
            ],
        ),
        eval_case(
            "hardqa_latest_screen_not_enough",
            "Candidate target should not win purely because it is the latest screen",
            "doc",
            Some("latest-feed"),
            vec![
                eval_candidate("c-doc-work", "w-doc", Some("doc"), 0.76, "medium"),
                eval_candidate("c-latest-feed", "w-feed", Some("latest-feed"), 0.39, "thin"),
            ],
        ),
        eval_case(
            "hardqa_raw_unresolved_json_hidden",
            "Raw unresolved JSON should not affect target ranking",
            "terminal",
            Some("terminal"),
            vec![
                eval_candidate(
                    "c-terminal-blocker",
                    "w-terminal-json",
                    Some("terminal"),
                    0.79,
                    "strong",
                ),
                eval_candidate_with_missing(
                    "c-json-raw",
                    "w-terminal-json",
                    Some("raw-json"),
                    0.31,
                    "thin",
                    vec!["raw_unresolved_json"],
                ),
            ],
        ),
        eval_case(
            "hardqa_island_continue_decision_id",
            "Floating island Continue path should stay tied to a Continue decision target",
            "continue-target",
            Some("smalltalk"),
            vec![
                eval_candidate(
                    "c-continue-target",
                    "w-island",
                    Some("continue-target"),
                    0.73,
                    "medium",
                ),
                eval_candidate(
                    "c-session-trail",
                    "w-island",
                    Some("legacy-session-trail"),
                    0.28,
                    "thin",
                ),
            ],
        ),
        eval_case(
            "hardqa_typing_throttle_keeps_target",
            "Heavy typing capture should not turn duplicate typing evidence into the target",
            "draft",
            Some("draft"),
            vec![
                eval_candidate("c-draft", "w-typing", Some("draft"), 0.7, "medium"),
                eval_candidate_with_missing(
                    "c-typing-dup",
                    "w-typing",
                    Some("typing-duplicate"),
                    0.34,
                    "thin",
                    vec!["repeated_identical_actions"],
                ),
            ],
        ),
        eval_case(
            "hardqa_passive_refresh_no_duplicate_decision",
            "Passive refresh should keep the same target instead of duplicating decisions",
            "code",
            Some("code"),
            vec![
                eval_candidate("c-code-cache", "w-cache", Some("code"), 0.72, "medium"),
                eval_candidate_with_missing(
                    "c-duplicate-decision",
                    "w-cache",
                    Some("duplicate-decision"),
                    0.3,
                    "thin",
                    vec!["duplicate_passive_refresh"],
                ),
            ],
        ),
        eval_case(
            "hardqa_thin_fallback_confidence_capped",
            "Thin fallback evidence should remain low-confidence evidence",
            "unknown",
            Some("unknown"),
            vec![eval_candidate_with_missing(
                "c-frame-fallback",
                "w-fallback",
                Some("unknown"),
                0.34,
                "thin",
                vec!["frame_fallback"],
            )],
        ),
        eval_case(
            "hardqa_default_ui_no_raw_ids",
            "Raw ids are diagnostics-only and should not define the target",
            "document",
            Some("artifact-raw-id"),
            vec![
                eval_candidate("c-document", "w-ui", Some("document"), 0.69, "medium"),
                eval_candidate_with_missing(
                    "c-raw-id",
                    "w-ui",
                    Some("artifact-raw-id"),
                    0.33,
                    "thin",
                    vec!["raw_id_diagnostic_only"],
                ),
            ],
        ),
        eval_case(
            "hardqa_legacy_stop_cloud_error_demoted",
            "Stop-time legacy bundle/cloud errors should not surface as primary Continue failures",
            "code",
            Some("cloud-error"),
            vec![
                eval_candidate(
                    "c-code-after-cloud-error",
                    "w-legacy",
                    Some("code"),
                    0.71,
                    "medium",
                ),
                eval_candidate_with_missing(
                    "c-cloud-error",
                    "w-legacy",
                    Some("cloud-error"),
                    0.29,
                    "thin",
                    vec!["legacy_stop_cloud_error"],
                ),
            ],
        ),
    ];
    ContinueEvalFixture { cases, k: Some(3) }
}

fn eval_case(
    name: &str,
    scenario: &str,
    expected_target_artifact_id: &str,
    current_focus_artifact_id: Option<&str>,
    candidates: Vec<ContinueEvalCandidateFixture>,
) -> ContinueEvalCaseFixture {
    ContinueEvalCaseFixture {
        name: name.to_string(),
        scenario: scenario.to_string(),
        expected_target_artifact_id: expected_target_artifact_id.to_string(),
        current_focus_artifact_id: current_focus_artifact_id.map(|value| value.to_string()),
        candidates,
        model_output: None,
    }
}

fn eval_candidate(
    id: &str,
    workstream_id: &str,
    target_artifact_id: Option<&str>,
    score: f64,
    evidence_quality: &str,
) -> ContinueEvalCandidateFixture {
    eval_candidate_with_missing(
        id,
        workstream_id,
        target_artifact_id,
        score,
        evidence_quality,
        Vec::new(),
    )
}

fn eval_candidate_with_missing(
    id: &str,
    workstream_id: &str,
    target_artifact_id: Option<&str>,
    score: f64,
    evidence_quality: &str,
    missing_evidence: Vec<&str>,
) -> ContinueEvalCandidateFixture {
    ContinueEvalCandidateFixture {
        id: id.to_string(),
        workstream_id: workstream_id.to_string(),
        target_artifact_id: target_artifact_id.map(|value| value.to_string()),
        score,
        evidence_quality: Some(evidence_quality.to_string()),
        missing_evidence: Some(
            missing_evidence
                .into_iter()
                .map(|value| value.to_string())
                .collect(),
        ),
    }
}

fn summarize_continue_eval_fixture(
    fixture: ContinueEvalFixture,
) -> Result<ContinueEvalReport, String> {
    let k = fixture.k.unwrap_or(3).max(1) as usize;
    let mut reports = Vec::new();
    for case in fixture.cases {
        reports.push(evaluate_continue_fixture_case(case, k)?);
    }
    let case_count = reports.len() as i64;
    let target_artifact_correct = reports
        .iter()
        .filter(|case| case.target_artifact_correct)
        .count() as i64;
    let hallucinated_artifact_count = reports
        .iter()
        .map(|case| case.hallucinated_artifact_count)
        .sum::<i64>();
    let fallback_count = reports
        .iter()
        .filter(|case| case.validation_status == "fallback")
        .count() as i64;
    let current_focus_false_positive_count = reports
        .iter()
        .filter(|case| case.current_focus_false_positive)
        .count() as i64;
    let denom = if case_count == 0 {
        1.0
    } else {
        case_count as f64
    };
    Ok(ContinueEvalReport {
        schema: "smalltalk.continue_eval.v1".to_string(),
        evaluated_at_ms: current_time_millis(),
        case_count,
        target_artifact_correct,
        target_artifact_accuracy: round_score(target_artifact_correct as f64 / denom),
        recall_at_k: round_score(
            reports.iter().filter(|case| case.recall_at_k).count() as f64 / denom,
        ),
        mrr: round_score(reports.iter().map(|case| case.reciprocal_rank).sum::<f64>() / denom),
        current_focus_false_positive_rate: round_score(
            current_focus_false_positive_count as f64 / denom,
        ),
        hallucinated_artifact_count,
        model_validation_fallback_rate: round_score(fallback_count as f64 / denom),
        cases: reports,
    })
}

fn evaluate_continue_fixture_case(
    mut case: ContinueEvalCaseFixture,
    k: usize,
) -> Result<ContinueEvalCaseReport, String> {
    case.candidates.sort_by(|left, right| {
        right
            .score
            .partial_cmp(&left.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let expected = case.expected_target_artifact_id.clone();
    let correct_rank = case
        .candidates
        .iter()
        .position(|candidate| candidate.target_artifact_id.as_deref() == Some(expected.as_str()))
        .map(|index| index as i64 + 1);
    let selected = if let Some(output) = case.model_output.as_ref() {
        case.candidates
            .iter()
            .find(|candidate| candidate.id == output.selected_candidate_id)
            .or_else(|| case.candidates.first())
    } else {
        case.candidates.first()
    };
    let mut validation_failures = Vec::new();
    let hallucinated_artifact_count = if let Some(output) = case.model_output.as_ref() {
        validate_eval_model_output(output, &case.candidates, &mut validation_failures)
    } else {
        0
    };
    let validation_status = if validation_failures.is_empty() {
        "valid".to_string()
    } else {
        "fallback".to_string()
    };
    let selected_target = selected.and_then(|candidate| candidate.target_artifact_id.clone());
    let target_correct = selected_target.as_deref() == Some(expected.as_str());
    Ok(ContinueEvalCaseReport {
        name: case.name,
        scenario: case.scenario,
        expected_target_artifact_id: expected,
        selected_candidate_id: selected.map(|candidate| candidate.id.clone()),
        selected_target_artifact_id: selected_target.clone(),
        target_artifact_correct: target_correct,
        correct_candidate_rank: correct_rank,
        recall_at_k: correct_rank.is_some_and(|rank| rank <= k as i64),
        reciprocal_rank: correct_rank.map(|rank| 1.0 / rank as f64).unwrap_or(0.0),
        current_focus_false_positive: selected_target.is_some()
            && selected_target == case.current_focus_artifact_id
            && !target_correct,
        hallucinated_artifact_count,
        validation_status,
        validation_failures,
    })
}

fn validate_eval_model_output(
    output: &ContinueMicroInferenceOutput,
    candidates: &[ContinueEvalCandidateFixture],
    failures: &mut Vec<String>,
) -> i64 {
    let candidate = candidates
        .iter()
        .find(|candidate| candidate.id == output.selected_candidate_id);
    let Some(candidate) = candidate else {
        failures.push("selected_candidate_id_not_in_fixture".to_string());
        return 1;
    };
    if candidate.workstream_id != output.selected_workstream_id {
        failures.push("selected_workstream_id_mismatch".to_string());
    }
    if output.confidence == "high"
        && (candidate.score < 0.7
            || candidate
                .missing_evidence
                .as_ref()
                .is_some_and(|missing| !missing.is_empty())
            || candidate.evidence_quality.as_deref() == Some("thin"))
    {
        failures.push("high_confidence_with_thin_local_evidence".to_string());
    }
    if unsupported_locator_in_model_output(output) {
        failures.push("unsupported_locator_in_model_output".to_string());
    }
    0
}

fn evidence_anchors_for_decision(
    current_focus: Option<&ContinueFocusSummary>,
    selected: Option<&ScoredContinueCandidate>,
    selected_workstream: Option<&ScorerWorkstream>,
) -> ContinueEvidenceAnchors {
    let mut anchors = ContinueEvidenceAnchors::default();
    if let Some(focus) = current_focus {
        push_unique(
            &mut anchors.frame_ids,
            &mut HashSet::new(),
            focus.frame_id.clone(),
        );
        if let Some(artifact_id) = &focus.artifact_id {
            anchors.artifact_ids.push(artifact_id.clone());
        }
    }
    if let Some(candidate) = selected {
        if let Some(frame_id) = &candidate.evidence_frame_id {
            if !anchors.frame_ids.contains(frame_id) {
                anchors.frame_ids.push(frame_id.clone());
            }
        }
        if let Some(action) = &candidate.last_meaningful_action {
            anchors.action_ids.push(action.id.clone());
            if !anchors.frame_ids.contains(&action.frame_id) {
                anchors.frame_ids.push(action.frame_id.clone());
            }
            if let Some(secondary_id) = &action.secondary_artifact_id {
                if !anchors.artifact_ids.contains(secondary_id) {
                    anchors.artifact_ids.push(secondary_id.clone());
                }
            }
        }
        if let Some(episode_id) = &candidate.supporting_episode_id {
            anchors.episode_ids.push(episode_id.clone());
        }
        if let Some(target) = &candidate.target_artifact {
            if !anchors.artifact_ids.contains(&target.id) {
                anchors.artifact_ids.push(target.id.clone());
            }
        }
        if let Some(target) = &candidate.resume_work_target {
            if !anchors.artifact_ids.contains(&target.id) {
                anchors.artifact_ids.push(target.id.clone());
            }
        }
    }
    if let Some(workstream) = selected_workstream {
        for episode in workstream.episodes.iter().rev().take(2) {
            if !anchors.episode_ids.contains(&episode.id) {
                anchors.episode_ids.push(episode.id.clone());
            }
            if let Some(kind) = &episode.dominant_action_kind {
                let _ = kind;
            }
        }
    }
    anchors
}

fn current_activity_summary(
    action: Option<&ScorerAction>,
    current_focus: Option<&ContinueFocusSummary>,
) -> Option<String> {
    let focus = current_focus?;
    let surface = focus
        .title
        .as_deref()
        .or(focus.window_title.as_deref())
        .or(focus.app_name.as_deref())
        .unwrap_or("current surface");
    let activity = match action.map(|action| action.action_kind.as_str()) {
        Some("editing") => "editing",
        Some("composing") => "composing",
        Some("encountering_error") => "looking at an error",
        Some("running_command") => "running a command",
        Some("observing_command_output") | Some("reviewing_output") => "reviewing output",
        Some("searching") | Some("branching_away") => "using a support/search surface",
        Some("idle_after_progress") => "idle after recent progress",
        Some("reading") => "reading",
        _ => "viewing",
    };
    Some(format!("{} {}", activity, surface))
}

fn workstream_summary(workstream: &ScorerWorkstream) -> ContinueSelectedWorkstream {
    ContinueSelectedWorkstream {
        workstream_id: workstream.id.clone(),
        state: workstream.state.clone(),
        title_candidate: workstream.title_candidate.clone(),
        primary_artifact_id: workstream.primary_artifact_id.clone(),
        last_active_timestamp_ms: workstream.last_active_timestamp_ms,
        unresolved_signal: unresolved_state_description(workstream.unresolved_signal.as_deref()),
    }
}

fn return_target_summary(
    artifact: Option<&ScorerArtifact>,
    fallback_frame_id: Option<String>,
) -> ContinueReturnTarget {
    ContinueReturnTarget {
        artifact_id: artifact.map(|artifact| artifact.id.clone()),
        artifact_kind: artifact.map(|artifact| artifact.artifact_kind.clone()),
        title: artifact.and_then(artifact_title_for_scorer),
        browser_url: artifact.and_then(|artifact| artifact.browser_url.clone()),
        document_path: artifact.and_then(|artifact| artifact.document_path.clone()),
        openability: artifact
            .map(|artifact| artifact.openability.clone())
            .unwrap_or_else(|| "frame_fallback".to_string()),
        fallback_frame_id,
    }
}

fn action_summary(action: &ScorerAction) -> ContinueActionSummary {
    ContinueActionSummary {
        action_id: action.id.clone(),
        action_kind: action.action_kind.clone(),
        action_role: action.action_role.clone(),
        timestamp_ms: action.created_at_ms,
        evidence_frame_id: action
            .strongest_frame_id
            .clone()
            .unwrap_or_else(|| action.frame_id.clone()),
        artifact_id: action.artifact_id.clone(),
        collapse_count: action.collapse_count.max(1),
        first_frame_id: action.first_frame_id.clone(),
        last_frame_id: action.last_frame_id.clone(),
        strongest_frame_id: action.strongest_frame_id.clone(),
    }
}

fn candidate_summary(candidate: &ScoredContinueCandidate) -> ContinueCandidateSummary {
    ContinueCandidateSummary {
        candidate_id: candidate.id.clone(),
        workstream_id: candidate.workstream_id.clone(),
        target_artifact_id: candidate
            .target_artifact
            .as_ref()
            .map(|artifact| artifact.id.clone()),
        candidate_kind: candidate.candidate_kind.clone(),
        score: round_score(candidate.score),
        confidence_label: confidence_label(candidate.score).to_string(),
        reason: candidate
            .reason
            .as_deref()
            .and_then(productize_continue_label),
        missing_evidence: candidate.missing_evidence.clone(),
        evidence_frame_id: candidate.evidence_frame_id.clone(),
        supporting_episode_id: candidate.supporting_episode_id.clone(),
        last_meaningful_action_id: candidate
            .last_meaningful_action
            .as_ref()
            .map(|action| action.id.clone()),
        components: ContinueScoreComponents {
            actionability: round_score(candidate.actionability_score),
            primary_target: round_score(candidate.primary_target_score),
            unresolved_state: round_score(candidate.unresolved_score),
            branch_origin: round_score(candidate.branch_origin_score),
            evidence_quality: round_score(candidate.evidence_quality_score),
            recency: round_score(candidate.recency_score),
            openability: round_score(candidate.openability_score),
            privacy_safety: round_score(candidate.privacy_safety_score),
        },
    }
}

fn artifact_title_for_scorer(artifact: &ScorerArtifact) -> Option<String> {
    first_non_empty([
        artifact.display_title.as_deref(),
        artifact.document_path.as_deref(),
        artifact.browser_url.as_deref(),
    ])
    .map(str::to_string)
}

fn confidence_label(score: f64) -> &'static str {
    if score >= 0.72 {
        "high"
    } else if score >= 0.5 {
        "medium"
    } else if score >= 0.32 {
        "low"
    } else {
        "thin"
    }
}

fn round_score(score: f64) -> f64 {
    (score.clamp(0.0, 1.0) * 100.0).round() / 100.0
}

fn load_evidence_frames(
    conn: &Connection,
    request: &ContinueSecondLayerRebuildRequest,
) -> Result<Vec<EvidenceFrame>, String> {
    let limit = request.limit.unwrap_or(300).clamp(1, 2_000);
    let max_captured_at = request.lookback_ms.and_then(|lookback| {
        if lookback > 0 {
            conn.query_row("SELECT MAX(captured_at) FROM frames", [], |row| {
                row.get::<_, Option<i64>>(0)
            })
            .ok()
            .flatten()
            .map(|max_ts| max_ts - lookback)
        } else {
            None
        }
    });

    let rows = if let Some(session_id) = &request.session_id {
        query_evidence_frame_rows(
            conn,
            "WHERE (?1 IS NULL OR f.id >= ?1)
               AND (?2 IS NULL OR f.id <= ?2)
               AND (?3 IS NULL OR f.captured_at >= ?3)
               AND f.session_id = ?4",
            &[
                &request.start_frame_id as &dyn rusqlite::ToSql,
                &request.end_frame_id,
                &max_captured_at,
                session_id,
                &limit,
            ],
        )?
    } else {
        query_evidence_frame_rows(
            conn,
            "WHERE (?1 IS NULL OR f.id >= ?1)
               AND (?2 IS NULL OR f.id <= ?2)
               AND (?3 IS NULL OR f.captured_at >= ?3)",
            &[
                &request.start_frame_id as &dyn rusqlite::ToSql,
                &request.end_frame_id,
                &max_captured_at,
                &limit,
            ],
        )?
    };

    let mut frames = Vec::with_capacity(rows.len());
    for mut frame in rows {
        frame.app_contexts = load_frame_app_contexts(conn, &frame.id)?;
        frame.content_units = load_frame_content_units(conn, &frame.id)?;
        frame.trigger = load_frame_trigger(conn, &frame.id)?;
        frame.ui_events = load_frame_ui_events(conn, &frame)?;
        frame.transition = load_frame_transition(conn, &frame.id)?;
        frame.frame_diff = load_frame_diff(conn, &frame.id)?;
        frame.typing_bursts = load_frame_typing_bursts(conn, &frame.id, frame.captured_at)?;
        frame.clipboard_events = load_frame_clipboard_events(conn, &frame.id, frame.captured_at)?;
        frame.focused_node_evidence = has_focused_node(conn, &frame.id)?;
        frame.selected_text_present = has_selected_text(conn, &frame.id)?
            || frame.app_contexts.iter().any(|context| {
                context
                    .selected_text
                    .as_deref()
                    .map(|value| !value.trim().is_empty())
                    .unwrap_or(false)
            });
        frames.push(frame);
    }
    if frames.len() <= 1 {
        frames.extend(load_event_evidence_frames(conn, request, frames.last())?);
    }
    frames.sort_by_key(|frame| {
        (
            frame.captured_at,
            frame.id.parse::<i64>().unwrap_or(i64::MAX),
        )
    });
    Ok(frames)
}

#[derive(Debug)]
struct EventEvidenceRow {
    id: String,
    ts_ms: i64,
    event_type: String,
    key_category: Option<String>,
    app_name: Option<String>,
    app_bundle_id: Option<String>,
    window_title: Option<String>,
}

fn load_event_evidence_frames(
    conn: &Connection,
    request: &ContinueSecondLayerRebuildRequest,
    latest_frame: Option<&EvidenceFrame>,
) -> Result<Vec<EvidenceFrame>, String> {
    if !table_exists(conn, "ui_events")? {
        return Ok(Vec::new());
    }
    let has_app_name = column_exists(conn, "ui_events", "app_name")?;
    let has_app_bundle_id = column_exists(conn, "ui_events", "app_bundle_id")?;
    let has_window_title = column_exists(conn, "ui_events", "window_title")?;
    let app_expr = if has_app_name { "app_name" } else { "NULL" };
    let bundle_expr = if has_app_bundle_id {
        "app_bundle_id"
    } else {
        "NULL"
    };
    let window_expr = if has_window_title {
        "window_title"
    } else {
        "NULL"
    };
    let limit = request.limit.unwrap_or(300).clamp(1, 2_000);
    let max_ts_cutoff = request.lookback_ms.and_then(|lookback| {
        if lookback > 0 {
            conn.query_row("SELECT MAX(ts_ms) FROM ui_events", [], |row| {
                row.get::<_, Option<i64>>(0)
            })
            .ok()
            .flatten()
            .map(|max_ts| max_ts - lookback)
        } else {
            None
        }
    });
    let sql = format!(
        "SELECT id, ts_ms, event_type, key_category, {}, {}, {}
         FROM (
           SELECT id, ts_ms, event_type, key_category, {}, {}, {}
           FROM ui_events
           WHERE (?1 IS NULL OR session_id = ?1)
             AND (?2 IS NULL OR ts_ms >= ?2)
           ORDER BY ts_ms DESC
           LIMIT ?3
         )
         ORDER BY ts_ms ASC",
        app_expr, bundle_expr, window_expr, app_expr, bundle_expr, window_expr
    );
    let mut stmt = conn.prepare(&sql).map_err(to_string)?;
    let rows = stmt
        .query_map(
            params![request.session_id.as_deref(), max_ts_cutoff, limit],
            |row| {
                Ok(EventEvidenceRow {
                    id: row.get(0)?,
                    ts_ms: row.get(1)?,
                    event_type: row.get(2)?,
                    key_category: row.get(3)?,
                    app_name: row.get(4)?,
                    app_bundle_id: row.get(5)?,
                    window_title: row.get(6)?,
                })
            },
        )
        .map_err(to_string)?;
    let rows = rows.collect::<Result<Vec<_>, _>>().map_err(to_string)?;
    Ok(event_rows_to_evidence_frames(
        &rows,
        request.session_id.clone(),
        latest_frame,
    ))
}

fn event_rows_to_evidence_frames(
    rows: &[EventEvidenceRow],
    session_id: Option<String>,
    latest_frame: Option<&EvidenceFrame>,
) -> Vec<EvidenceFrame> {
    let mut frames = Vec::new();
    let mut previous_bucket: Option<(String, String, String, i64)> = None;
    let mut previous_frame_id = latest_frame.map(|frame| frame.id.clone());
    for row in rows {
        let signal = event_signal_kind(&row.event_type, row.key_category.as_deref());
        if signal == "noise" {
            continue;
        }
        let app = clean_event_label(row.app_name.as_deref());
        let window = clean_event_label(row.window_title.as_deref());
        let same_bucket = previous_bucket
            .as_ref()
            .map(|(prev_signal, prev_app, prev_window, prev_ts)| {
                prev_signal == &signal
                    && prev_app.eq_ignore_ascii_case(&app)
                    && prev_window.eq_ignore_ascii_case(&window)
                    && row.ts_ms.saturating_sub(*prev_ts) <= 30_000
            })
            .unwrap_or(false);
        if same_bucket {
            previous_bucket = Some((signal, app, window, row.ts_ms));
            continue;
        }
        let id_seed = format!(
            "{}:{}:{}:{}:{}",
            session_id.as_deref().unwrap_or(""),
            row.ts_ms,
            row.id,
            signal,
            app
        );
        let id = format!("event-{}", stable_hash(id_seed.as_bytes()));
        let summary = event_evidence_summary(&signal, &app, &window);
        let content_hash = stable_hash(summary.as_bytes());
        frames.push(EvidenceFrame {
            id: id.clone(),
            captured_at: row.ts_ms,
            app_name: if app.is_empty() {
                None
            } else {
                Some(app.clone())
            },
            window_name: if window.is_empty() {
                None
            } else {
                Some(window.clone())
            },
            browser_url: None,
            document_path: None,
            capture_trigger: format!("event_signal_{}", signal),
            text_source: Some("event".to_string()),
            full_text: Some(summary),
            content_hash: Some(content_hash),
            image_hash: None,
            privacy_status: None,
            app_bundle_id: row.app_bundle_id.clone(),
            previous_frame_id,
            session_id: session_id.clone(),
            app_contexts: Vec::new(),
            content_units: Vec::new(),
            ui_events: vec![EvidenceUiEvent {
                id: row.id.clone(),
                event_type: row.event_type.clone(),
                key_category: row.key_category.clone(),
            }],
            trigger: Some(EvidenceTrigger {
                id: format!("event-trigger-{}", stable_hash(row.id.as_bytes())),
                trigger_type: format!("event_signal_{}", signal),
                caused_by_event_ids: vec![row.id.clone()],
            }),
            transition: None,
            frame_diff: None,
            typing_bursts: Vec::new(),
            clipboard_events: Vec::new(),
            focused_node_evidence: false,
            selected_text_present: false,
        });
        previous_frame_id = Some(id);
        previous_bucket = Some((signal, app, window, row.ts_ms));
        if frames.len() >= 64 {
            break;
        }
    }
    frames
}

fn event_signal_kind(event_type: &str, key_category: Option<&str>) -> String {
    match event_type {
        "key_down" => match key_category.unwrap_or("") {
            "character" => "typing".to_string(),
            "enter" | "return" => "commit_key".to_string(),
            "shortcut" => "shortcut".to_string(),
            other if !other.trim().is_empty() => format!("key_{}", normalize_token(other)),
            _ => "key_down".to_string(),
        },
        "scroll" => "scroll".to_string(),
        "click" => "click".to_string(),
        "app_switch" => "app_switch".to_string(),
        "ax_notification" => "accessibility".to_string(),
        other if other.trim().is_empty() => "noise".to_string(),
        other => normalize_token(other),
    }
}

fn clean_event_label(value: Option<&str>) -> String {
    value
        .unwrap_or("")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn event_evidence_summary(signal: &str, app: &str, window: &str) -> String {
    let surface = [app, window]
        .into_iter()
        .filter(|value| !value.trim().is_empty())
        .collect::<Vec<_>>()
        .join(" / ");
    if surface.is_empty() {
        format!("Local event signal: {}", signal)
    } else {
        format!("Local event signal: {} in {}", signal, surface)
    }
}

fn query_evidence_frame_rows(
    conn: &Connection,
    where_clause: &str,
    values: &[&dyn rusqlite::ToSql],
) -> Result<Vec<EvidenceFrame>, String> {
    let sql = format!(
        "SELECT * FROM (
            SELECT f.id, f.captured_at, f.app_name, f.window_name, f.browser_url,
                   f.document_path, f.capture_trigger, f.text_source, f.full_text,
                   f.content_hash, f.image_hash, f.privacy_status, f.app_bundle_id,
                   f.previous_frame_id, f.session_id
            FROM frames f
            {}
            ORDER BY f.captured_at DESC, f.id DESC
            LIMIT ?{}
         ) recent
         ORDER BY captured_at ASC, id ASC",
        where_clause,
        values.len()
    );
    let bind_values = values.to_vec();
    let mut stmt = conn.prepare(&sql).map_err(to_string)?;
    let rows = stmt
        .query_map(&bind_values[..], evidence_frame_from_row)
        .map_err(to_string)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(to_string)
}

fn evidence_frame_from_row(row: &Row<'_>) -> rusqlite::Result<EvidenceFrame> {
    let id: i64 = row.get(0)?;
    Ok(EvidenceFrame {
        id: id.to_string(),
        captured_at: row.get(1)?,
        app_name: row.get(2)?,
        window_name: row.get(3)?,
        browser_url: row.get(4)?,
        document_path: row.get(5)?,
        capture_trigger: row.get(6)?,
        text_source: row.get(7)?,
        full_text: row.get(8)?,
        content_hash: row.get(9)?,
        image_hash: row.get(10)?,
        privacy_status: row.get(11)?,
        app_bundle_id: row.get(12)?,
        previous_frame_id: row.get(13)?,
        session_id: row.get(14)?,
        app_contexts: Vec::new(),
        content_units: Vec::new(),
        ui_events: Vec::new(),
        trigger: None,
        transition: None,
        frame_diff: None,
        typing_bursts: Vec::new(),
        clipboard_events: Vec::new(),
        focused_node_evidence: false,
        selected_text_present: false,
    })
}

fn load_frame_app_contexts(
    conn: &Connection,
    frame_id: &str,
) -> Result<Vec<EvidenceAppContext>, String> {
    if !table_exists(conn, "app_contexts")? {
        return Ok(Vec::new());
    }
    let mut stmt = conn
        .prepare(
            "SELECT id, adapter_id, object_type, primary_id, title, url, file_path,
                    repo_path, selected_text, focused_object, confidence
             FROM app_contexts
             WHERE frame_id = ?1
             ORDER BY confidence DESC, id ASC",
        )
        .map_err(to_string)?;
    let rows = stmt
        .query_map(params![frame_id], |row| {
            Ok(EvidenceAppContext {
                id: row.get(0)?,
                adapter_id: row.get(1)?,
                object_type: row.get(2)?,
                primary_id: row.get(3)?,
                title: row.get(4)?,
                url: row.get(5)?,
                file_path: row.get(6)?,
                repo_path: row.get(7)?,
                selected_text: row.get(8)?,
                focused_object: row.get(9)?,
                confidence: row.get(10)?,
            })
        })
        .map_err(to_string)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(to_string)
}

fn load_frame_content_units(
    conn: &Connection,
    frame_id: &str,
) -> Result<Vec<EvidenceContentUnit>, String> {
    if !table_exists(conn, "content_units")? {
        return Ok(Vec::new());
    }
    let mut stmt = conn
        .prepare(
            "SELECT id, source, unit_type, semantic_role, text, text_hash, confidence
             FROM content_units
             WHERE frame_id = ?1
             ORDER BY confidence DESC, created_at_ms DESC
             LIMIT 240",
        )
        .map_err(to_string)?;
    let rows = stmt
        .query_map(params![frame_id], |row| {
            Ok(EvidenceContentUnit {
                id: row.get(0)?,
                source: row.get(1)?,
                unit_type: row.get(2)?,
                semantic_role: row.get(3)?,
                text: row.get(4)?,
                text_hash: row.get(5)?,
                confidence: row.get(6)?,
            })
        })
        .map_err(to_string)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(to_string)
}

fn load_frame_ui_events(
    conn: &Connection,
    frame: &EvidenceFrame,
) -> Result<Vec<EvidenceUiEvent>, String> {
    if !table_exists(conn, "ui_events")? {
        return Ok(Vec::new());
    }
    let mut events = Vec::new();
    let mut seen = HashSet::new();
    if let Some(trigger) = &frame.trigger {
        for event_id in &trigger.caused_by_event_ids {
            if let Some(event) = load_ui_event_by_id(conn, event_id)? {
                if seen.insert(event.id.clone()) {
                    events.push(event);
                }
            }
        }
    }
    let mut stmt = conn
        .prepare(
            "SELECT id, event_type, key_category
             FROM ui_events
             WHERE (?1 IS NULL OR session_id = ?1)
               AND ABS(ts_ms - ?2) <= 2500
             ORDER BY ts_ms DESC
             LIMIT 12",
        )
        .map_err(to_string)?;
    let rows = stmt
        .query_map(
            params![frame.session_id.as_deref(), frame.captured_at],
            |row| {
                Ok(EvidenceUiEvent {
                    id: row.get(0)?,
                    event_type: row.get(1)?,
                    key_category: row.get(2)?,
                })
            },
        )
        .map_err(to_string)?;
    for event in rows.collect::<Result<Vec<_>, _>>().map_err(to_string)? {
        if seen.insert(event.id.clone()) {
            events.push(event);
        }
    }
    Ok(events)
}

fn load_ui_event_by_id(
    conn: &Connection,
    event_id: &str,
) -> Result<Option<EvidenceUiEvent>, String> {
    conn.query_row(
        "SELECT id, event_type, key_category
         FROM ui_events
         WHERE id = ?1
         LIMIT 1",
        params![event_id],
        |row| {
            Ok(EvidenceUiEvent {
                id: row.get(0)?,
                event_type: row.get(1)?,
                key_category: row.get(2)?,
            })
        },
    )
    .optional()
    .map_err(to_string)
}

fn load_frame_trigger(
    conn: &Connection,
    frame_id: &str,
) -> Result<Option<EvidenceTrigger>, String> {
    if !table_exists(conn, "capture_triggers")? {
        return Ok(None);
    }
    conn.query_row(
        "SELECT id, trigger_type, caused_by_event_ids
         FROM capture_triggers
         WHERE post_frame_id = ?1 OR pre_frame_id = ?1
         ORDER BY ts_ms DESC
         LIMIT 1",
        params![frame_id],
        |row| {
            let raw_event_ids: String = row.get(2)?;
            Ok(EvidenceTrigger {
                id: row.get(0)?,
                trigger_type: row.get(1)?,
                caused_by_event_ids: parse_string_array(&raw_event_ids),
            })
        },
    )
    .optional()
    .map_err(to_string)
}

fn load_frame_transition(
    conn: &Connection,
    frame_id: &str,
) -> Result<Option<EvidenceTransition>, String> {
    if !table_exists(conn, "event_transitions")? {
        return Ok(None);
    }
    conn.query_row(
        "SELECT id, primary_event_id, transition_type, summary, confidence
         FROM event_transitions
         WHERE post_frame_id = ?1 OR pre_frame_id = ?1
         ORDER BY ts_end_ms DESC
         LIMIT 1",
        params![frame_id],
        |row| {
            Ok(EvidenceTransition {
                id: row.get(0)?,
                primary_event_id: row.get(1)?,
                transition_type: row.get(2)?,
                summary: row.get(3)?,
                confidence: row.get(4)?,
            })
        },
    )
    .optional()
    .map_err(to_string)
}

fn load_frame_diff(conn: &Connection, frame_id: &str) -> Result<Option<EvidenceFrameDiff>, String> {
    if !table_exists(conn, "frame_diffs")? {
        return Ok(None);
    }
    conn.query_row(
        "SELECT diff_type, added_text_hashes, removed_text_hashes, summary
         FROM frame_diffs
         WHERE to_frame_id = ?1
         ORDER BY ts_ms DESC
         LIMIT 1",
        params![frame_id],
        |row| {
            Ok(EvidenceFrameDiff {
                diff_type: row.get(0)?,
                added_text_hashes: row.get(1)?,
                removed_text_hashes: row.get(2)?,
                summary: row.get(3)?,
            })
        },
    )
    .optional()
    .map_err(to_string)
}

fn load_frame_typing_bursts(
    conn: &Connection,
    frame_id: &str,
    captured_at: i64,
) -> Result<Vec<EvidenceTypingBurst>, String> {
    if !table_exists(conn, "typing_bursts")? {
        return Ok(Vec::new());
    }
    let mut stmt = conn
        .prepare(
            "SELECT id, enter_count, paste_count, committed, commit_signal
             FROM typing_bursts
             WHERE pre_frame_id = ?1
                OR post_frame_id = ?1
                OR (?2 BETWEEN started_at_ms - 250 AND ended_at_ms + 1500)
             ORDER BY ended_at_ms DESC
             LIMIT 8",
        )
        .map_err(to_string)?;
    let rows = stmt
        .query_map(params![frame_id, captured_at], |row| {
            let committed: i64 = row.get(3)?;
            Ok(EvidenceTypingBurst {
                id: row.get(0)?,
                enter_count: row.get(1)?,
                paste_count: row.get(2)?,
                committed: committed != 0,
                commit_signal: row.get(4)?,
            })
        })
        .map_err(to_string)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(to_string)
}

fn load_frame_clipboard_events(
    conn: &Connection,
    frame_id: &str,
    captured_at: i64,
) -> Result<Vec<EvidenceClipboardEvent>, String> {
    if !table_exists(conn, "clipboard_events")? {
        return Ok(Vec::new());
    }
    let mut stmt = conn
        .prepare(
            "SELECT id, source_frame_id, target_frame_id
             FROM clipboard_events
             WHERE source_frame_id = ?1
                OR target_frame_id = ?1
                OR ABS(ts_ms - ?2) <= 2500
             ORDER BY ts_ms DESC
             LIMIT 8",
        )
        .map_err(to_string)?;
    let rows = stmt
        .query_map(params![frame_id, captured_at], |row| {
            Ok(EvidenceClipboardEvent {
                id: row.get(0)?,
                source_frame_id: row.get(1)?,
                target_frame_id: row.get(2)?,
            })
        })
        .map_err(to_string)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(to_string)
}

fn has_focused_node(conn: &Connection, frame_id: &str) -> Result<bool, String> {
    if !table_exists(conn, "ax_nodes")? {
        return Ok(false);
    }
    conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM ax_nodes WHERE frame_id = ?1 AND focused = 1)",
        params![frame_id],
        |row| row.get::<_, i64>(0),
    )
    .map(|value| value != 0)
    .map_err(to_string)
}

fn has_selected_text(conn: &Connection, frame_id: &str) -> Result<bool, String> {
    if !table_exists(conn, "ax_nodes")? {
        return Ok(false);
    }
    conn.query_row(
        "SELECT EXISTS(
            SELECT 1 FROM ax_nodes
            WHERE frame_id = ?1 AND selected_text IS NOT NULL AND TRIM(selected_text) <> ''
         )",
        params![frame_id],
        |row| row.get::<_, i64>(0),
    )
    .map(|value| value != 0)
    .map_err(to_string)
}

fn clear_second_layer_rows_for_frames(
    conn: &Connection,
    frame_ids: &[String],
) -> Result<(), String> {
    if frame_ids.is_empty() {
        return Ok(());
    }
    for frame_id in frame_ids {
        let action_ids = {
            let mut stmt = conn
                .prepare("SELECT id FROM continue_task_actions WHERE frame_id = ?1")
                .map_err(to_string)?;
            let rows = stmt
                .query_map(params![frame_id], |row| row.get::<_, String>(0))
                .map_err(to_string)?;
            rows.collect::<Result<Vec<_>, _>>().map_err(to_string)?
        };
        for action_id in action_ids {
            conn.execute(
                "DELETE FROM continue_task_action_events WHERE action_id = ?1",
                params![action_id],
            )
            .map_err(to_string)?;
        }
        conn.execute(
            "DELETE FROM continue_task_actions WHERE frame_id = ?1",
            params![frame_id],
        )
        .map_err(to_string)?;
        conn.execute(
            "DELETE FROM continue_artifact_observations WHERE frame_id = ?1",
            params![frame_id],
        )
        .map_err(to_string)?;
    }
    conn.execute(
        "DELETE FROM continue_artifacts
         WHERE id NOT IN (SELECT DISTINCT artifact_id FROM continue_artifact_observations)",
        [],
    )
    .map_err(to_string)?;
    Ok(())
}

fn clear_third_layer_rows(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "
        DELETE FROM continue_candidates;
        DELETE FROM continue_workstream_artifacts;
        DELETE FROM continue_workstream_episodes;
        DELETE FROM continue_workstreams;
        DELETE FROM continue_episode_artifacts;
        DELETE FROM continue_episode_actions;
        DELETE FROM continue_episodes;
        ",
    )
    .map_err(to_string)
}

fn load_continue_action_records(
    conn: &Connection,
    request: &ContinueThirdLayerRebuildRequest,
) -> Result<Vec<ContinueActionRecord>, String> {
    let limit = request.limit.unwrap_or(500).clamp(1, 5_000);
    let min_created_at = request.lookback_ms.and_then(|lookback| {
        if lookback > 0 {
            conn.query_row(
                "SELECT MAX(created_at_ms) FROM continue_task_actions",
                [],
                |row| row.get::<_, Option<i64>>(0),
            )
            .ok()
            .flatten()
            .map(|max_ts| max_ts - lookback)
        } else {
            None
        }
    });

    let sql = format!(
        "SELECT * FROM (
            SELECT ta.id, ta.frame_id, ta.previous_frame_id, ta.artifact_id,
                   ta.secondary_artifact_id, ta.action_kind, ta.action_role,
                   ta.confidence, ta.reason, ta.created_at_ms,
                   ta.collapse_count, ta.first_frame_id, ta.last_frame_id, ta.strongest_frame_id,
                   a.id, a.artifact_kind, a.stable_key, a.app_name, a.display_title,
                   a.browser_url, a.document_path, a.evidence_quality,
                   sa.id, sa.artifact_kind, sa.stable_key, sa.app_name, sa.display_title,
                   sa.browser_url, sa.document_path, sa.evidence_quality
            FROM continue_task_actions ta
            LEFT JOIN continue_artifacts a ON a.id = ta.artifact_id
            LEFT JOIN continue_artifacts sa ON sa.id = ta.secondary_artifact_id
            LEFT JOIN frames f ON CAST(ta.frame_id AS INTEGER) = f.id
            WHERE (?1 IS NULL OR f.session_id = ?1)
              AND (?2 IS NULL OR CAST(ta.frame_id AS INTEGER) >= ?2)
              AND (?3 IS NULL OR CAST(ta.frame_id AS INTEGER) <= ?3)
              AND (?4 IS NULL OR ta.created_at_ms >= ?4)
            ORDER BY ta.created_at_ms DESC, CAST(ta.frame_id AS INTEGER) DESC, ta.id DESC
            LIMIT ?5
         ) recent
         ORDER BY created_at_ms ASC, CAST(frame_id AS INTEGER) ASC, id ASC"
    );
    let mut stmt = conn.prepare(&sql).map_err(to_string)?;
    let rows = stmt
        .query_map(
            params![
                request.session_id.as_deref(),
                request.start_frame_id,
                request.end_frame_id,
                min_created_at,
                limit,
            ],
            |row| {
                let artifact = artifact_record_from_row(row, 14)?;
                let secondary_artifact = artifact_record_from_row(row, 22)?;
                Ok(ContinueActionRecord {
                    id: row.get(0)?,
                    frame_id: row.get(1)?,
                    previous_frame_id: row.get(2)?,
                    artifact_id: row.get(3)?,
                    secondary_artifact_id: row.get(4)?,
                    action_kind: row.get(5)?,
                    action_role: row.get(6)?,
                    confidence: row.get(7)?,
                    reason: row.get(8)?,
                    created_at_ms: row.get(9)?,
                    collapse_count: row.get::<_, i64>(10)?.max(1),
                    first_frame_id: row.get(11)?,
                    last_frame_id: row.get(12)?,
                    strongest_frame_id: row.get(13)?,
                    artifact,
                    secondary_artifact,
                })
            },
        )
        .map_err(to_string)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(to_string)
}

fn artifact_record_from_row(
    row: &Row<'_>,
    offset: usize,
) -> rusqlite::Result<Option<ContinueArtifactRecord>> {
    let id: Option<String> = row.get(offset)?;
    Ok(id.map(|id| ContinueArtifactRecord {
        id,
        artifact_kind: row
            .get(offset + 1)
            .unwrap_or_else(|_| "unknown".to_string()),
        stable_key: row.get(offset + 2).unwrap_or_default(),
        app_name: row.get(offset + 3).ok().flatten(),
        display_title: row.get(offset + 4).ok().flatten(),
        browser_url: row.get(offset + 5).ok().flatten(),
        document_path: row.get(offset + 6).ok().flatten(),
        evidence_quality: row
            .get(offset + 7)
            .unwrap_or_else(|_| "unknown".to_string()),
    }))
}

fn resolve_artifact(frame: &EvidenceFrame) -> ResolvedArtifact {
    let context = frame.app_contexts.first();
    let privacy_blocked = frame
        .privacy_status
        .as_deref()
        .map(|status| status.contains("redacted") || status.contains("blocked"))
        .unwrap_or(false);

    let document_path = if privacy_blocked {
        None
    } else {
        first_non_empty([
            frame.document_path.as_deref(),
            context.and_then(|ctx| ctx.file_path.as_deref()),
            context.and_then(|ctx| ctx.repo_path.as_deref()),
        ])
        .map(normalize_document_path)
    };
    if let Some(path) = document_path {
        let kind = artifact_kind_for_path(&path, frame, context);
        let confidence = if frame.app_bundle_id.is_some() || context.is_some() {
            0.92
        } else {
            0.84
        };
        let evidence_quality = if confidence >= 0.9 {
            "strong"
        } else {
            "medium"
        };
        return artifact_with_key(
            frame,
            kind,
            format!("document:{}", path),
            first_non_empty([
                context.and_then(|ctx| ctx.title.as_deref()),
                frame.window_name.as_deref(),
                Some(path.as_str()),
            ])
            .map(str::to_string),
            None,
            Some(path),
            confidence,
            evidence_quality,
            "openable",
            "document_path_identity",
        );
    }

    let browser_url = first_non_empty([
        frame.browser_url.as_deref(),
        context.and_then(|ctx| ctx.url.as_deref()),
        context
            .and_then(|ctx| ctx.primary_id.as_deref())
            .filter(|value| value.starts_with("http://") || value.starts_with("https://")),
    ])
    .and_then(canonicalize_url);
    if let Some(url) = browser_url {
        let kind = artifact_kind_for_url(&url, frame, context);
        let confidence = if context.is_some() || frame.window_name.is_some() {
            0.88
        } else {
            0.78
        };
        let evidence_quality = if confidence >= 0.84 {
            "strong"
        } else {
            "medium"
        };
        return artifact_with_key(
            frame,
            kind,
            format!("url:{}", url),
            first_non_empty([
                context.and_then(|ctx| ctx.title.as_deref()),
                frame.window_name.as_deref(),
            ])
            .map(str::to_string),
            Some(url),
            None,
            confidence,
            evidence_quality,
            "openable",
            "url_identity",
        );
    }

    if let Some(context) = context {
        if let Some(object_id) = first_non_empty([
            context.primary_id.as_deref(),
            context.title.as_deref(),
            context.focused_object.as_deref(),
        ]) {
            let normalized = normalize_window_title(object_id);
            if !normalized.is_empty() {
                let kind = artifact_kind_for_context(&context.object_type, frame);
                let key = format!(
                    "context:{}:{}:{}:{}",
                    normalize_token(&context.adapter_id),
                    normalize_token(&context.object_type),
                    normalize_token(frame.app_bundle_id.as_deref().unwrap_or("")),
                    normalized
                );
                return artifact_with_key(
                    frame,
                    kind,
                    key,
                    context.title.clone().or_else(|| frame.window_name.clone()),
                    None,
                    None,
                    0.72,
                    "medium",
                    "frame_fallback",
                    "app_context_object_identity",
                );
            }
        }
    }

    if let Some(window_title) = frame.window_name.as_deref() {
        let normalized = normalize_window_title(window_title);
        if !normalized.is_empty() {
            let kind = artifact_kind_for_frame(frame, context);
            let confidence = if frame.app_bundle_id.is_some() {
                0.62
            } else {
                0.5
            };
            let evidence_quality = if confidence >= 0.6 { "medium" } else { "thin" };
            return artifact_with_key(
                frame,
                kind,
                format!(
                    "window:{}:{}",
                    normalize_token(
                        frame
                            .app_bundle_id
                            .as_deref()
                            .or(frame.app_name.as_deref())
                            .unwrap_or("unknown")
                    ),
                    normalized
                ),
                Some(window_title.to_string()),
                None,
                None,
                confidence,
                evidence_quality,
                "frame_fallback",
                "app_window_title_identity",
            );
        }
    }

    let hash = frame
        .content_hash
        .as_deref()
        .or(frame.image_hash.as_deref())
        .unwrap_or(&frame.id);
    artifact_with_key(
        frame,
        "unknown".to_string(),
        format!(
            "surface:{}:{}",
            normalize_token(
                frame
                    .app_bundle_id
                    .as_deref()
                    .or(frame.app_name.as_deref())
                    .unwrap_or("unknown")
            ),
            normalize_token(hash)
        ),
        frame.window_name.clone(),
        None,
        None,
        0.35,
        "thin",
        "frame_fallback",
        "surface_fallback_identity",
    )
}

#[allow(clippy::too_many_arguments)]
fn artifact_with_key(
    frame: &EvidenceFrame,
    kind: String,
    stable_key: String,
    display_title: Option<String>,
    browser_url: Option<String>,
    document_path: Option<String>,
    identity_confidence: f64,
    evidence_quality: &str,
    openability: &str,
    reason: &str,
) -> ResolvedArtifact {
    let id = format!("artifact-{}", stable_hash(stable_key.as_bytes()));
    ResolvedArtifact {
        id,
        kind,
        stable_key,
        display_title: display_title.or_else(|| frame.window_name.clone()),
        browser_url,
        document_path,
        identity_confidence,
        evidence_quality: evidence_quality.to_string(),
        openability: openability.to_string(),
        reason: reason.to_string(),
    }
}

fn upsert_continue_artifact(
    conn: &Connection,
    frame: &EvidenceFrame,
    artifact: &ResolvedArtifact,
) -> Result<(), String> {
    conn.execute(
        "INSERT OR IGNORE INTO continue_artifacts (
            id, artifact_kind, stable_key, app_name, bundle_id, window_title,
            browser_url, document_path, display_title, first_seen_frame_id,
            last_seen_frame_id, first_seen_timestamp, last_seen_timestamp,
            identity_confidence, evidence_quality, privacy_status, openability,
            created_at_ms, updated_at_ms
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?10, ?11, ?11,
                   ?12, ?13, ?14, ?15, ?11, ?11)",
        params![
            artifact.id,
            artifact.kind,
            artifact.stable_key,
            frame.app_name,
            frame.app_bundle_id,
            frame.window_name,
            artifact.browser_url,
            artifact.document_path,
            artifact.display_title,
            frame.id,
            frame.captured_at,
            artifact.identity_confidence,
            artifact.evidence_quality,
            frame.privacy_status,
            artifact.openability,
        ],
    )
    .map_err(to_string)?;
    conn.execute(
        "UPDATE continue_artifacts
         SET artifact_kind = ?2,
             app_name = COALESCE(?3, app_name),
             bundle_id = COALESCE(?4, bundle_id),
             window_title = COALESCE(?5, window_title),
             browser_url = COALESCE(?6, browser_url),
             document_path = COALESCE(?7, document_path),
             display_title = COALESCE(?8, display_title),
             last_seen_frame_id = CASE
               WHEN ?9 >= last_seen_timestamp THEN ?10 ELSE last_seen_frame_id END,
             last_seen_timestamp = MAX(last_seen_timestamp, ?9),
             identity_confidence = MAX(identity_confidence, ?11),
             evidence_quality = CASE
               WHEN evidence_quality = 'strong' OR ?12 = 'unknown' THEN evidence_quality
               WHEN ?12 = 'strong' THEN ?12
               WHEN evidence_quality = 'thin' AND ?12 = 'medium' THEN ?12
               ELSE evidence_quality END,
             privacy_status = COALESCE(?13, privacy_status),
             openability = CASE WHEN openability = 'openable' THEN openability ELSE ?14 END,
             updated_at_ms = MAX(updated_at_ms, ?9)
         WHERE stable_key = ?1",
        params![
            artifact.stable_key,
            artifact.kind,
            frame.app_name,
            frame.app_bundle_id,
            frame.window_name,
            artifact.browser_url,
            artifact.document_path,
            artifact.display_title,
            frame.captured_at,
            frame.id,
            artifact.identity_confidence,
            artifact.evidence_quality,
            frame.privacy_status,
            artifact.openability,
        ],
    )
    .map_err(to_string)?;
    Ok(())
}

fn upsert_continue_artifact_observation(
    conn: &Connection,
    frame: &EvidenceFrame,
    artifact: &ResolvedArtifact,
) -> Result<(), String> {
    let app_context_id = frame.app_contexts.first().map(|context| context.id.clone());
    let text_source = continue_text_source(frame);
    let visible_text_length = frame.full_text.as_deref().unwrap_or("").chars().count() as i64;
    let observation_confidence = (artifact.identity_confidence
        + if visible_text_length > 0 { 0.05 } else { -0.05 })
    .clamp(0.0, 0.98);
    let id = format!(
        "artifact-observation-{}-{}",
        frame.id,
        stable_hash(artifact.id.as_bytes())
    );
    conn.execute(
        "INSERT INTO continue_artifact_observations (
            id, artifact_id, frame_id, app_context_id, text_source, content_hash,
            image_hash, focused_node_evidence, selected_text_present,
            visible_text_length, observation_confidence, reason, timestamp_ms
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
         ON CONFLICT(id) DO UPDATE SET
            app_context_id = excluded.app_context_id,
            text_source = excluded.text_source,
            content_hash = excluded.content_hash,
            image_hash = excluded.image_hash,
            focused_node_evidence = excluded.focused_node_evidence,
            selected_text_present = excluded.selected_text_present,
            visible_text_length = excluded.visible_text_length,
            observation_confidence = excluded.observation_confidence,
            reason = excluded.reason,
            timestamp_ms = excluded.timestamp_ms",
        params![
            id,
            artifact.id,
            frame.id,
            app_context_id,
            text_source,
            frame.content_hash,
            frame.image_hash,
            bool_to_i64(frame.focused_node_evidence),
            bool_to_i64(frame.selected_text_present),
            visible_text_length,
            observation_confidence,
            artifact.reason,
            frame.captured_at,
        ],
    )
    .map_err(to_string)?;
    Ok(())
}

fn extract_task_actions(
    frames: &[EvidenceFrame],
    artifacts: &HashMap<String, ResolvedArtifact>,
) -> Vec<ExtractedTaskAction> {
    let mut actions = Vec::with_capacity(frames.len());
    let mut recent_primary = VecDeque::<String>::new();
    let mut last_meaningful: Option<(String, String)> = None;

    for (index, frame) in frames.iter().enumerate() {
        let artifact = match artifacts.get(&frame.id) {
            Some(artifact) => artifact,
            None => continue,
        };
        let previous_frame = index.checked_sub(1).and_then(|idx| frames.get(idx));
        let previous_artifact = previous_frame.and_then(|prev| artifacts.get(&prev.id));
        let mut extracted = classify_task_action(
            frame,
            previous_frame,
            artifact,
            previous_artifact,
            &recent_primary,
            last_meaningful.as_ref(),
        );

        if extracted.action_kind == "returning_to_origin" {
            if let Some(prev_artifact) = previous_artifact {
                extracted.secondary_artifact_id = Some(prev_artifact.id.clone());
            }
        }
        if extracted.action_kind == "branching_away" {
            if let Some(prev_artifact) = previous_artifact {
                extracted.secondary_artifact_id = Some(prev_artifact.id.clone());
            }
        }

        if is_primary_work_artifact(&artifact.kind) && !recent_primary.contains(&artifact.id) {
            recent_primary.push_front(artifact.id.clone());
            while recent_primary.len() > 8 {
                recent_primary.pop_back();
            }
        }
        if is_meaningful_action_kind(&extracted.action_kind) {
            last_meaningful = Some((extracted.action_kind.clone(), artifact.id.clone()));
        }
        actions.push(extracted);
    }
    actions
}

fn collapse_repeated_task_actions(actions: Vec<ExtractedTaskAction>) -> Vec<ExtractedTaskAction> {
    let mut collapsed: Vec<ExtractedTaskAction> = Vec::new();
    for action in actions {
        if let Some(previous) = collapsed.last_mut() {
            if repeated_task_action(previous, &action) {
                let previous_confidence = previous.confidence;
                previous.collapse_count = previous.collapse_count.saturating_add(1).max(2);
                previous.last_frame_id = Some(action.frame_id.clone());
                previous.frame_id = if action.confidence >= previous_confidence {
                    action.frame_id.clone()
                } else {
                    previous
                        .strongest_frame_id
                        .clone()
                        .unwrap_or_else(|| previous.frame_id.clone())
                };
                previous.created_at_ms = action.created_at_ms;
                previous.confidence = previous.confidence.max(action.confidence);
                if action.confidence >= previous_confidence {
                    previous.strongest_frame_id = Some(action.frame_id.clone());
                }
                for event_id in action.evidence_event_ids {
                    if !previous.evidence_event_ids.contains(&event_id) {
                        previous.evidence_event_ids.push(event_id);
                    }
                }
                previous.id = semantic_action_id(previous);
                continue;
            }
        }
        collapsed.push(action);
    }
    collapsed
}

fn repeated_task_action(previous: &ExtractedTaskAction, next: &ExtractedTaskAction) -> bool {
    if !matches!(
        next.action_kind.as_str(),
        "encountering_error"
            | "composing"
            | "editing"
            | "reading"
            | "observing_command_output"
            | "reviewing_output"
            | "idle_after_progress"
    ) {
        return false;
    }
    if previous.action_kind != next.action_kind
        || previous.artifact_id != next.artifact_id
        || previous.secondary_artifact_id != next.secondary_artifact_id
    {
        return false;
    }
    if semantic_base_reason(&previous.reason) != semantic_base_reason(&next.reason) {
        return false;
    }
    if previous.trigger_type != next.trigger_type
        || previous.transition_label != next.transition_label
    {
        return false;
    }
    let adjacent_frame = previous
        .last_frame_id
        .as_deref()
        .unwrap_or(previous.frame_id.as_str())
        .parse::<i64>()
        .ok()
        .zip(next.frame_id.parse::<i64>().ok())
        .map(|(left, right)| right.saturating_sub(left).abs() <= 3)
        .unwrap_or(false);
    let close_in_time = next.created_at_ms.saturating_sub(previous.created_at_ms) <= 2 * 60 * 1000;
    adjacent_frame || close_in_time
}

fn semantic_base_reason(reason: &str) -> String {
    let mut parts = reason.splitn(4, ':').collect::<Vec<_>>();
    if parts.len() == 4 && parts[0] == "semantic_collapse" {
        parts.pop().unwrap_or("").to_string()
    } else {
        reason.to_string()
    }
}

fn semantic_action_id(action: &ExtractedTaskAction) -> String {
    let id_seed = format!(
        "{}:{}:{}:{}:{}:{}",
        action.previous_frame_id.as_deref().unwrap_or(""),
        action
            .first_frame_id
            .as_deref()
            .unwrap_or(action.frame_id.as_str()),
        action
            .last_frame_id
            .as_deref()
            .unwrap_or(action.frame_id.as_str()),
        action.artifact_id.as_deref().unwrap_or(""),
        action.action_kind,
        semantic_base_reason(&action.reason)
    );
    format!("task-action-{}", stable_hash(id_seed.as_bytes()))
}

fn classify_task_action(
    frame: &EvidenceFrame,
    previous_frame: Option<&EvidenceFrame>,
    artifact: &ResolvedArtifact,
    previous_artifact: Option<&ResolvedArtifact>,
    recent_primary: &VecDeque<String>,
    last_meaningful: Option<&(String, String)>,
) -> ExtractedTaskAction {
    let previous_artifact_id = previous_artifact.map(|artifact| artifact.id.clone());
    let switched_artifact = previous_artifact
        .map(|previous| previous.id != artifact.id)
        .unwrap_or(false);
    let event_ids = evidence_event_ids(frame);
    let trigger_type = frame
        .trigger
        .as_ref()
        .map(|trigger| trigger.trigger_type.clone())
        .or_else(|| Some(frame.capture_trigger.clone()));
    let transition_label = frame
        .transition
        .as_ref()
        .and_then(|transition| transition.transition_type.clone());

    let (kind, role, confidence, reason) = if has_clipboard_transfer(frame) {
        ("copying_evidence", "support", 0.82, "clipboard_transfer")
    } else if switched_artifact
        && recent_primary.contains(&artifact.id)
        && previous_artifact
            .map(|previous| is_support_artifact(&previous.kind))
            .unwrap_or(false)
    {
        (
            "returning_to_origin",
            "return",
            0.78,
            "returned_to_recent_primary_artifact",
        )
    } else if has_error_signal(frame) {
        ("encountering_error", "primary", 0.86, "error_signal")
    } else if artifact.kind == "terminal" && terminal_has_enter_or_commit(frame) {
        (
            "running_command",
            "primary",
            0.82,
            "terminal_enter_or_commit",
        )
    } else if artifact.kind == "terminal" && terminal_output_changed(frame) {
        (
            "observing_command_output",
            "support",
            0.78,
            "terminal_output_changed",
        )
    } else if frame.capture_trigger.contains("idle")
        && last_meaningful.is_some()
        && !terminal_output_changed(frame)
    {
        (
            "idle_after_progress",
            "primary",
            0.62,
            "idle_after_meaningful_action",
        )
    } else if switched_artifact && artifact.kind == "messaging" {
        (
            "messaging_interrupt",
            "interrupt",
            0.58,
            "messaging_context_switch",
        )
    } else if has_typing_signal(frame) && is_composer_context(frame, &artifact.kind) {
        ("composing", "primary", 0.82, "typing_in_composer")
    } else if has_typing_signal(frame) && is_editable_artifact(&artifact.kind) {
        ("editing", "primary", 0.84, "typing_in_editable_artifact")
    } else if switched_artifact
        && previous_artifact
            .map(|previous| is_primary_work_artifact(&previous.kind))
            .unwrap_or(false)
        && is_support_artifact(&artifact.kind)
        && last_meaningful.is_some()
    {
        (
            "branching_away",
            "branch",
            0.76,
            "primary_to_support_switch",
        )
    } else if is_search_context(frame, artifact) {
        ("searching", "support", 0.76, "search_context")
    } else if is_verification_branch(frame, artifact, previous_artifact) {
        (
            "verification_branch",
            "branch",
            0.66,
            "switched_to_verification_surface",
        )
    } else if has_output_review_signal(frame, &artifact.kind) {
        ("reviewing_output", "support", 0.7, "output_review_signal")
    } else if switched_artifact {
        (
            "switching_context",
            "unknown",
            0.48,
            "artifact_switch_without_stronger_signal",
        )
    } else if is_navigation_signal(frame) {
        ("navigating", "primary", 0.55, "navigation_signal")
    } else if frame.full_text.as_deref().unwrap_or("").trim().is_empty() {
        ("unknown", "unknown", 0.25, "thin_or_missing_text")
    } else {
        (
            "reading",
            "primary",
            0.52,
            "visible_content_without_edit_signal",
        )
    };

    let id_seed = format!(
        "{}:{}:{}:{}",
        frame.id,
        artifact.id,
        kind,
        previous_artifact_id.as_deref().unwrap_or("")
    );
    ExtractedTaskAction {
        id: format!("task-action-{}", stable_hash(id_seed.as_bytes())),
        frame_id: frame.id.clone(),
        previous_frame_id: previous_frame
            .and_then(|previous| Some(previous.id.clone()))
            .or_else(|| frame.previous_frame_id.clone()),
        artifact_id: Some(artifact.id.clone()),
        secondary_artifact_id: previous_artifact_id,
        action_kind: kind.to_string(),
        action_role: role.to_string(),
        trigger_type,
        transition_label,
        evidence_event_ids: event_ids,
        confidence,
        reason: reason.to_string(),
        created_at_ms: frame.captured_at,
        collapse_count: 1,
        first_frame_id: Some(frame.id.clone()),
        last_frame_id: Some(frame.id.clone()),
        strongest_frame_id: Some(frame.id.clone()),
    }
}

fn insert_continue_task_action(
    conn: &Connection,
    action: &ExtractedTaskAction,
) -> Result<(), String> {
    let event_ids_json = serde_json::to_string(&action.evidence_event_ids).map_err(to_string)?;
    conn.execute(
        "INSERT INTO continue_task_actions (
            id, frame_id, previous_frame_id, artifact_id, secondary_artifact_id,
            action_kind, action_role, trigger_type, transition_label,
            evidence_event_ids_json, confidence, reason, created_at_ms,
            collapse_count, first_frame_id, last_frame_id, strongest_frame_id
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
        params![
            action.id,
            action.frame_id,
            action.previous_frame_id,
            action.artifact_id,
            action.secondary_artifact_id,
            action.action_kind,
            action.action_role,
            action.trigger_type,
            action.transition_label,
            event_ids_json,
            action.confidence,
            action.reason,
            action.created_at_ms,
            action.collapse_count.max(1),
            action.first_frame_id,
            action.last_frame_id,
            action.strongest_frame_id,
        ],
    )
    .map_err(to_string)?;
    for (index, event_id) in action.evidence_event_ids.iter().enumerate() {
        conn.execute(
            "INSERT OR IGNORE INTO continue_task_action_events (
                action_id, event_id, order_index
             ) VALUES (?1, ?2, ?3)",
            params![action.id, event_id, index as i64],
        )
        .map_err(to_string)?;
    }
    Ok(())
}

fn build_continue_episodes(actions: &[ContinueActionRecord]) -> Vec<BuiltEpisode> {
    let mut episodes = Vec::new();
    let mut current: Vec<ContinueActionRecord> = Vec::new();
    let mut start_reason = "window_start".to_string();
    let mut end_reason: Option<String> = None;

    for action in actions {
        if current.is_empty() {
            start_reason = start_reason_for_action(action, None);
            current.push(action.clone());
            if action.action_kind == "idle_after_progress" {
                episodes.push(finalize_episode(
                    std::mem::take(&mut current),
                    start_reason.clone(),
                    Some("idle_after_progress".to_string()),
                ));
                start_reason = "after_idle_after_progress".to_string();
            }
            continue;
        }

        let previous = current.last().expect("current action exists");
        if let Some(boundary) = episode_boundary_reason(previous, action, &current) {
            episodes.push(finalize_episode(
                std::mem::take(&mut current),
                start_reason.clone(),
                Some(boundary.end_reason.clone()),
            ));
            start_reason = boundary.start_reason;
            end_reason = None;
        }

        current.push(action.clone());
        if action.action_kind == "idle_after_progress" {
            end_reason = Some("idle_after_progress".to_string());
            episodes.push(finalize_episode(
                std::mem::take(&mut current),
                start_reason.clone(),
                end_reason.take(),
            ));
            start_reason = "after_idle_after_progress".to_string();
        }
    }

    if !current.is_empty() {
        episodes.push(finalize_episode(
            current,
            start_reason,
            end_reason.or_else(|| Some("window_end".to_string())),
        ));
    }
    episodes
}

#[derive(Debug)]
struct EpisodeBoundaryReason {
    start_reason: String,
    end_reason: String,
}

fn episode_boundary_reason(
    previous: &ContinueActionRecord,
    next: &ContinueActionRecord,
    current: &[ContinueActionRecord],
) -> Option<EpisodeBoundaryReason> {
    let previous_artifact = previous.artifact_id.as_deref();
    let next_artifact = next.artifact_id.as_deref();
    let switched_artifact = previous_artifact != next_artifact;
    let gap_ms = next.created_at_ms.saturating_sub(previous.created_at_ms);
    let current_has_progress = current
        .iter()
        .any(|action| is_meaningful_action_kind(&action.action_kind));

    if next.action_kind == "returning_to_origin" {
        return Some(boundary("returning_to_origin", "branch_returned_to_origin"));
    }
    if next.action_kind == "messaging_interrupt" {
        return Some(boundary(
            "communication_interruption",
            "left_primary_for_messaging",
        ));
    }
    if next.action_kind == "verification_branch" || is_terminal_verification_action(next) {
        if switched_artifact && current_has_progress {
            return Some(boundary(
                "verification_branch",
                "left_primary_for_verification",
            ));
        }
    }
    if previous.action_kind == "encountering_error"
        && switched_artifact
        && next
            .artifact
            .as_ref()
            .map(|artifact| is_support_artifact(&artifact.artifact_kind))
            .unwrap_or(false)
    {
        return Some(boundary(
            "error_to_support_branch",
            "error_to_search_or_support",
        ));
    }
    if next.action_kind == "branching_away" || next.action_kind == "searching" {
        if switched_artifact && current_has_progress {
            return Some(boundary("support_branch", "left_primary_for_support"));
        }
    }
    if switched_artifact && current_has_progress {
        return Some(boundary(
            "artifact_switch_after_progress",
            "changed_artifact_after_progress",
        ));
    }
    if gap_ms > 10 * 60 * 1000 && current_has_progress {
        return Some(boundary(
            "time_gap_after_progress",
            "time_gap_after_progress",
        ));
    }
    if switched_artifact && !same_artifact_family(previous, next) {
        return Some(boundary(
            "artifact_switch",
            "artifact_switch_without_progress",
        ));
    }
    None
}

fn boundary(start_reason: &str, end_reason: &str) -> EpisodeBoundaryReason {
    EpisodeBoundaryReason {
        start_reason: start_reason.to_string(),
        end_reason: end_reason.to_string(),
    }
}

fn same_artifact_family(previous: &ContinueActionRecord, next: &ContinueActionRecord) -> bool {
    match (&previous.artifact, &next.artifact) {
        (Some(previous), Some(next)) => {
            previous.id == next.id
                || (!previous.document_path.is_none()
                    && previous.document_path == next.document_path)
                || (!previous.browser_url.is_none() && previous.browser_url == next.browser_url)
        }
        _ => false,
    }
}

fn start_reason_for_action(
    action: &ContinueActionRecord,
    previous: Option<&ContinueActionRecord>,
) -> String {
    match action.action_kind.as_str() {
        "returning_to_origin" => "returning_to_origin".to_string(),
        "branching_away" | "searching" => "support_branch".to_string(),
        "messaging_interrupt" => "communication_interruption".to_string(),
        "verification_branch" | "observing_command_output" | "reviewing_output" => {
            "verification_branch".to_string()
        }
        "idle_after_progress" => "idle_after_progress".to_string(),
        _ if previous.is_none() => "window_start".to_string(),
        _ => "continued_sequence".to_string(),
    }
}

fn finalize_episode(
    actions: Vec<ContinueActionRecord>,
    start_reason: String,
    end_reason: Option<String>,
) -> BuiltEpisode {
    let start_timestamp_ms = actions
        .first()
        .map(|action| action.created_at_ms)
        .unwrap_or_default();
    let end_timestamp_ms = actions
        .last()
        .map(|action| action.created_at_ms)
        .unwrap_or(start_timestamp_ms);
    let start_frame_id = actions.first().map(|action| action.frame_id.clone());
    let end_frame_id = actions.last().map(|action| action.frame_id.clone());
    let primary_artifact_id = episode_primary_artifact_id(&actions);
    let dominant_action_kind = dominant_action_kind(&actions);
    let evidence_quality = episode_evidence_quality(&actions);
    let confidence = episode_confidence(&actions, &evidence_quality);
    let mut artifacts = HashMap::new();
    for action in &actions {
        add_episode_artifact_roles(&mut artifacts, action, primary_artifact_id.as_deref());
    }
    let id_seed = format!(
        "{}:{}:{}:{}",
        start_frame_id.as_deref().unwrap_or(""),
        end_frame_id.as_deref().unwrap_or(""),
        primary_artifact_id.as_deref().unwrap_or("unknown"),
        start_reason
    );
    let summary_label = episode_summary_label(&actions, primary_artifact_id.as_deref());
    BuiltEpisode {
        id: format!("episode-{}", stable_hash(id_seed.as_bytes())),
        actions,
        artifacts,
        state: "closed".to_string(),
        start_frame_id,
        end_frame_id,
        start_timestamp_ms,
        end_timestamp_ms,
        primary_artifact_id,
        dominant_action_kind,
        boundary_start_reason: start_reason,
        boundary_end_reason: end_reason,
        confidence,
        evidence_quality,
        summary_label,
    }
}

fn episode_primary_artifact_id(actions: &[ContinueActionRecord]) -> Option<String> {
    for action in actions {
        if matches!(
            action.action_kind.as_str(),
            "editing"
                | "composing"
                | "running_command"
                | "encountering_error"
                | "returning_to_origin"
                | "reading"
        ) && action.action_role != "support"
        {
            if let Some(artifact_id) = &action.artifact_id {
                return Some(artifact_id.clone());
            }
        }
    }
    for action in actions {
        if matches!(
            action.action_kind.as_str(),
            "branching_away"
                | "searching"
                | "verification_branch"
                | "observing_command_output"
                | "reviewing_output"
        ) {
            if let Some(artifact_id) = &action.secondary_artifact_id {
                return Some(artifact_id.clone());
            }
        }
    }
    actions.iter().find_map(|action| action.artifact_id.clone())
}

fn dominant_action_kind(actions: &[ContinueActionRecord]) -> Option<String> {
    for kind in [
        "encountering_error",
        "editing",
        "composing",
        "running_command",
        "observing_command_output",
        "reviewing_output",
        "copying_evidence",
        "returning_to_origin",
        "branching_away",
        "searching",
        "messaging_interrupt",
        "idle_after_progress",
        "reading",
    ] {
        if actions.iter().any(|action| action.action_kind == kind) {
            return Some(kind.to_string());
        }
    }
    actions.first().map(|action| action.action_kind.clone())
}

fn episode_evidence_quality(actions: &[ContinueActionRecord]) -> String {
    let qualities = actions
        .iter()
        .filter_map(|action| action.artifact.as_ref())
        .map(|artifact| artifact.evidence_quality.as_str())
        .collect::<Vec<_>>();
    if qualities.iter().any(|quality| *quality == "strong") {
        "strong".to_string()
    } else if qualities.iter().any(|quality| *quality == "medium") {
        "medium".to_string()
    } else if qualities.iter().any(|quality| *quality == "thin") {
        "thin".to_string()
    } else {
        "unknown".to_string()
    }
}

fn episode_confidence(actions: &[ContinueActionRecord], evidence_quality: &str) -> f64 {
    if actions.is_empty() {
        return 0.0;
    }
    let average =
        actions.iter().map(|action| action.confidence).sum::<f64>() / actions.len() as f64;
    let quality_bonus = match evidence_quality {
        "strong" => 0.12,
        "medium" => 0.04,
        "thin" => -0.14,
        _ => -0.2,
    };
    (average + quality_bonus).clamp(0.05, 0.95)
}

fn add_episode_artifact_roles(
    artifacts: &mut HashMap<String, BuiltArtifactRole>,
    action: &ContinueActionRecord,
    episode_primary_artifact_id: Option<&str>,
) {
    if let Some(artifact) = &action.artifact {
        let role = episode_artifact_role(action, artifact.id.as_str(), episode_primary_artifact_id);
        upsert_episode_artifact_role(
            artifacts,
            artifact.clone(),
            role,
            Some(action.frame_id.clone()),
            action.confidence,
            action
                .reason
                .clone()
                .unwrap_or_else(|| action.action_kind.clone()),
        );
    }
    if let Some(artifact) = &action.secondary_artifact {
        let role = secondary_episode_artifact_role(action);
        if role != "unknown" {
            upsert_episode_artifact_role(
                artifacts,
                artifact.clone(),
                role,
                action
                    .previous_frame_id
                    .clone()
                    .or_else(|| Some(action.frame_id.clone())),
                (action.confidence - 0.08).clamp(0.05, 0.9),
                format!("secondary_artifact_for_{}", action.action_kind),
            );
        }
    }
}

fn upsert_episode_artifact_role(
    artifacts: &mut HashMap<String, BuiltArtifactRole>,
    artifact: ContinueArtifactRecord,
    role: String,
    frame_id: Option<String>,
    score: f64,
    reason: String,
) {
    let key = format!("{}:{}", artifact.id, role);
    artifacts
        .entry(key)
        .and_modify(|existing| {
            existing.last_frame_id = frame_id.clone().or_else(|| existing.last_frame_id.clone());
            existing.contribution_score = existing.contribution_score.max(score);
        })
        .or_insert(BuiltArtifactRole {
            artifact,
            role,
            first_frame_id: frame_id.clone(),
            last_frame_id: frame_id,
            contribution_score: score,
            reason,
        });
}

fn episode_artifact_role(
    action: &ContinueActionRecord,
    artifact_id: &str,
    episode_primary_artifact_id: Option<&str>,
) -> String {
    match action.action_kind.as_str() {
        "encountering_error" => "blocker".to_string(),
        "searching" | "branching_away" => "branch_support".to_string(),
        "verification_branch" | "reviewing_output" | "observing_command_output" => {
            "output_verification".to_string()
        }
        "messaging_interrupt" => "interruption".to_string(),
        "copying_evidence" if Some(artifact_id) == episode_primary_artifact_id => {
            "primary_target".to_string()
        }
        "copying_evidence" => "source_evidence".to_string(),
        "unknown" => "unknown".to_string(),
        _ if Some(artifact_id) == episode_primary_artifact_id => "primary_target".to_string(),
        _ if action.action_role == "support" || action.action_role == "branch" => {
            "branch_support".to_string()
        }
        _ => "current_focus_only".to_string(),
    }
}

fn secondary_episode_artifact_role(action: &ContinueActionRecord) -> String {
    match action.action_kind.as_str() {
        "branching_away"
        | "searching"
        | "verification_branch"
        | "observing_command_output"
        | "reviewing_output" => "primary_target".to_string(),
        "copying_evidence" => "source_evidence".to_string(),
        "returning_to_origin" => "branch_support".to_string(),
        "messaging_interrupt" => "current_focus_only".to_string(),
        _ => "unknown".to_string(),
    }
}

fn episode_summary_label(
    actions: &[ContinueActionRecord],
    primary_artifact_id: Option<&str>,
) -> String {
    let kind = dominant_action_kind(actions).unwrap_or_else(|| "unknown".to_string());
    if let Some(action) = actions.first() {
        let count = action.collapse_count.max(1);
        if count > 1 {
            let label = match kind.as_str() {
                "encountering_error" => format!("Error remained visible across {} frames", count),
                "composing" => format!(
                    "User continued typing in the same composer across {} frames",
                    count
                ),
                "observing_command_output" | "reviewing_output" => {
                    format!("Same output observed repeatedly across {} frames", count)
                }
                _ => format!(
                    "Same {} state repeated across {} frames",
                    kind.replace('_', " "),
                    count
                ),
            };
            return label;
        }
    }
    let title = primary_artifact_id.and_then(|artifact_id| {
        actions
            .iter()
            .flat_map(|action| [action.artifact.as_ref(), action.secondary_artifact.as_ref()])
            .flatten()
            .find(|artifact| artifact.id == artifact_id)
            .and_then(artifact_title)
    });
    match title {
        Some(title) => format!("{} in {}", kind.replace('_', " "), title),
        None => kind.replace('_', " "),
    }
}

fn build_continue_workstreams(episodes: &[BuiltEpisode]) -> Vec<BuiltWorkstream> {
    let mut workstreams: Vec<BuiltWorkstream> = Vec::new();
    for episode in episodes {
        let best = workstreams
            .iter()
            .enumerate()
            .map(|(index, workstream)| {
                let (score, reason) = score_episode_membership(episode, workstream);
                (index, score, reason)
            })
            .max_by(|left, right| {
                left.1
                    .partial_cmp(&right.1)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        if let Some((index, score, reason)) = best.filter(|(_, score, _)| *score >= 0.58) {
            attach_episode_to_workstream(&mut workstreams[index], episode.clone(), score, reason);
        } else {
            workstreams.push(new_workstream_for_episode(episode.clone()));
        }
    }
    finalize_workstream_states(&mut workstreams);
    workstreams
}

fn score_episode_membership(episode: &BuiltEpisode, workstream: &BuiltWorkstream) -> (f64, String) {
    let mut score: f64 = 0.0;
    let mut reasons = Vec::new();
    if let (Some(episode_primary), Some(workstream_primary)) = (
        episode.primary_artifact_id.as_deref(),
        workstream.primary_artifact_id.as_deref(),
    ) {
        if episode_primary == workstream_primary {
            score = score.max(0.92);
            reasons.push("same_primary_artifact");
        }
    }
    if episode_has_artifact_role(
        episode,
        workstream.primary_artifact_id.as_deref(),
        "primary_target",
    ) {
        score = score.max(0.86);
        reasons.push("episode_links_existing_primary_target");
    }
    if episode.actions.iter().any(|action| {
        meaningful_secondary_link_action(&action.action_kind)
            && action.secondary_artifact_id.as_deref() == workstream.primary_artifact_id.as_deref()
    }) {
        score = score.max(0.82);
        reasons.push("secondary_artifact_links_to_workstream_primary");
    }
    if episode
        .actions
        .iter()
        .any(|action| action.action_kind == "returning_to_origin")
        && episode.primary_artifact_id.as_deref() == workstream.primary_artifact_id.as_deref()
    {
        score = score.max(0.9);
        reasons.push("returning_to_origin");
    }
    if episode
        .artifacts
        .values()
        .any(|artifact| workstream.artifacts.contains_key(&artifact.artifact.id))
    {
        score = score.max(0.74);
        reasons.push("shared_artifact");
    }
    if is_terminal_episode(episode)
        && workstream
            .primary_artifact_id
            .as_deref()
            .map(|primary| episode_has_artifact(episode, primary))
            .unwrap_or(false)
    {
        score = score.max(0.72);
        reasons.push("terminal_verification_linked_to_primary");
    }
    if title_similarity(episode, workstream) >= 0.45 {
        score = score.max(0.62);
        reasons.push("similar_titles");
    }
    let gap = episode
        .start_timestamp_ms
        .saturating_sub(workstream.last_active_timestamp_ms);
    if gap <= 2 * 60 * 1000 && score > 0.0 {
        score = (score + 0.06).min(0.98);
        reasons.push("short_gap_boost");
    }
    if reasons.is_empty() && gap <= 60 * 1000 {
        score = 0.12;
        reasons.push("recency_only_weak_signal");
    }
    (score, reasons.join("+"))
}

fn meaningful_secondary_link_action(kind: &str) -> bool {
    matches!(
        kind,
        "branching_away"
            | "returning_to_origin"
            | "copying_evidence"
            | "verification_branch"
            | "observing_command_output"
            | "reviewing_output"
            | "running_command"
    )
}

fn episode_has_artifact_role(
    episode: &BuiltEpisode,
    artifact_id: Option<&str>,
    role: &str,
) -> bool {
    artifact_id
        .map(|artifact_id| {
            episode
                .artifacts
                .values()
                .any(|artifact| artifact.artifact.id == artifact_id && artifact.role == role)
        })
        .unwrap_or(false)
}

fn episode_has_artifact(episode: &BuiltEpisode, artifact_id: &str) -> bool {
    episode
        .artifacts
        .values()
        .any(|artifact| artifact.artifact.id == artifact_id)
}

fn is_terminal_episode(episode: &BuiltEpisode) -> bool {
    episode
        .artifacts
        .values()
        .any(|artifact| artifact.artifact.artifact_kind == "terminal")
}

fn title_similarity(episode: &BuiltEpisode, workstream: &BuiltWorkstream) -> f64 {
    let episode_tokens = episode_title_tokens(episode);
    let workstream_tokens = workstream_title_tokens(workstream);
    token_overlap_score(&episode_tokens, &workstream_tokens)
}

fn episode_title_tokens(episode: &BuiltEpisode) -> HashSet<String> {
    episode
        .artifacts
        .values()
        .flat_map(|artifact| {
            [
                artifact.artifact.display_title.as_deref(),
                artifact.artifact.document_path.as_deref(),
                artifact.artifact.browser_url.as_deref(),
            ]
        })
        .flatten()
        .flat_map(title_tokens)
        .collect()
}

fn workstream_title_tokens(workstream: &BuiltWorkstream) -> HashSet<String> {
    workstream
        .artifacts
        .values()
        .flat_map(|artifact| {
            [
                artifact.artifact.display_title.as_deref(),
                artifact.artifact.document_path.as_deref(),
                artifact.artifact.browser_url.as_deref(),
            ]
        })
        .flatten()
        .flat_map(title_tokens)
        .collect()
}

fn title_tokens(value: &str) -> Vec<String> {
    value
        .to_lowercase()
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|part| {
            part.len() >= 4
                && !matches!(
                    *part,
                    "http"
                        | "https"
                        | "www"
                        | "com"
                        | "net"
                        | "org"
                        | "example"
                        | "search"
                        | "results"
                        | "page"
                        | "browser"
                )
        })
        .map(str::to_string)
        .collect()
}

fn token_overlap_score(left: &HashSet<String>, right: &HashSet<String>) -> f64 {
    if left.is_empty() || right.is_empty() {
        return 0.0;
    }
    let intersection = left.intersection(right).count() as f64;
    let union = left.union(right).count() as f64;
    if union == 0.0 {
        0.0
    } else {
        intersection / union
    }
}

fn new_workstream_for_episode(episode: BuiltEpisode) -> BuiltWorkstream {
    let primary_artifact_id = workstream_primary_for_episode(&episode);
    let id_seed = primary_artifact_id
        .as_deref()
        .map(|primary| format!("primary:{}", primary))
        .unwrap_or_else(|| format!("episode:{}", episode.id));
    let mut workstream = BuiltWorkstream {
        id: format!("workstream-{}", stable_hash(id_seed.as_bytes())),
        episodes: Vec::new(),
        artifacts: HashMap::new(),
        state: "active".to_string(),
        title_candidate: None,
        primary_artifact_id,
        created_at_ms: episode.start_timestamp_ms,
        last_active_timestamp_ms: episode.end_timestamp_ms,
        suspended_timestamp_ms: None,
        confidence: episode.confidence,
        unresolved_signal: None,
        source: "local_heuristic".to_string(),
    };
    attach_episode_to_workstream(
        &mut workstream,
        episode,
        1.0,
        "new_workstream_from_episode".to_string(),
    );
    workstream
}

fn attach_episode_to_workstream(
    workstream: &mut BuiltWorkstream,
    episode: BuiltEpisode,
    score: f64,
    reason: String,
) {
    workstream.created_at_ms = workstream.created_at_ms.min(episode.start_timestamp_ms);
    workstream.last_active_timestamp_ms = workstream
        .last_active_timestamp_ms
        .max(episode.end_timestamp_ms);
    if workstream.primary_artifact_id.is_none() {
        workstream.primary_artifact_id = workstream_primary_for_episode(&episode);
    } else if let Some(candidate_primary) = workstream_primary_for_episode(&episode) {
        if stronger_primary_candidate(
            &episode,
            &candidate_primary,
            workstream.primary_artifact_id.as_deref(),
        ) {
            workstream.primary_artifact_id = Some(candidate_primary);
        }
    }
    for episode_artifact in episode.artifacts.values() {
        let durable_role = durable_role_for_episode_role(&episode_artifact.role, &episode);
        upsert_workstream_artifact(workstream, episode_artifact, durable_role);
    }
    workstream.confidence =
        ((workstream.confidence + episode.confidence + score) / 3.0).clamp(0.05, 0.96);
    workstream.episodes.push((episode, score, reason));
    workstream.title_candidate = workstream_title_candidate(workstream);
    workstream.unresolved_signal = unresolved_signal_for_workstream(workstream);
}

fn workstream_primary_for_episode(episode: &BuiltEpisode) -> Option<String> {
    for artifact in episode.artifacts.values() {
        if artifact.role == "primary_target"
            && !matches!(
                artifact.artifact.artifact_kind.as_str(),
                "browser_tab" | "chat_conversation" | "messaging"
            )
        {
            return Some(artifact.artifact.id.clone());
        }
    }
    episode.primary_artifact_id.clone()
}

fn stronger_primary_candidate(
    episode: &BuiltEpisode,
    candidate_primary: &str,
    current_primary: Option<&str>,
) -> bool {
    if current_primary == Some(candidate_primary) {
        return false;
    }
    episode.actions.iter().any(|action| {
        action.artifact_id.as_deref() == Some(candidate_primary)
            && matches!(
                action.action_kind.as_str(),
                "editing" | "composing" | "running_command" | "returning_to_origin"
            )
    })
}

fn durable_role_for_episode_role(role: &str, episode: &BuiltEpisode) -> String {
    match role {
        "primary_target" => "primary_target".to_string(),
        "source_evidence" => "support_source".to_string(),
        "branch_support" => "branch".to_string(),
        "output_verification" => "verification_surface".to_string(),
        "blocker" => "blocker_surface".to_string(),
        "interruption"
            if episode
                .actions
                .iter()
                .any(|action| action.action_kind == "copying_evidence") =>
        {
            "communication_surface".to_string()
        }
        "interruption" => "distractor".to_string(),
        _ => "unknown".to_string(),
    }
}

fn upsert_workstream_artifact(
    workstream: &mut BuiltWorkstream,
    episode_artifact: &BuiltArtifactRole,
    durable_role: String,
) {
    let key = episode_artifact.artifact.id.clone();
    let importance = match durable_role.as_str() {
        "primary_target" => 1.0,
        "blocker_surface" => 0.88,
        "verification_surface" => 0.78,
        "support_source" => 0.7,
        "branch" => 0.58,
        "communication_surface" => 0.5,
        "distractor" => 0.2,
        _ => 0.25,
    } * episode_artifact.contribution_score.max(0.2);
    workstream
        .artifacts
        .entry(key)
        .and_modify(|existing| {
            if role_rank(&durable_role) > role_rank(&existing.durable_role) {
                existing.durable_role = durable_role.clone();
                existing.reason = episode_artifact.reason.clone();
            }
            existing.importance_score = existing.importance_score.max(importance);
            existing.last_seen_frame_id = episode_artifact
                .last_frame_id
                .clone()
                .or_else(|| existing.last_seen_frame_id.clone());
        })
        .or_insert(BuiltWorkstreamArtifact {
            artifact: episode_artifact.artifact.clone(),
            durable_role,
            importance_score: importance,
            first_seen_frame_id: episode_artifact.first_frame_id.clone(),
            last_seen_frame_id: episode_artifact.last_frame_id.clone(),
            reason: episode_artifact.reason.clone(),
        });
}

fn role_rank(role: &str) -> i64 {
    match role {
        "primary_target" => 100,
        "blocker_surface" => 90,
        "verification_surface" => 80,
        "support_source" => 70,
        "branch" => 60,
        "communication_surface" => 50,
        "distractor" => 20,
        _ => 10,
    }
}

fn workstream_title_candidate(workstream: &BuiltWorkstream) -> Option<String> {
    if let Some(primary) = workstream.primary_artifact_id.as_deref() {
        if let Some(artifact) = workstream
            .artifacts
            .get(primary)
            .map(|entry| &entry.artifact)
        {
            if let Some(title) = artifact_title(artifact) {
                return Some(title);
            }
        }
    }
    workstream
        .artifacts
        .values()
        .filter(|artifact| artifact.durable_role != "distractor")
        .max_by(|left, right| {
            left.importance_score
                .partial_cmp(&right.importance_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .and_then(|artifact| artifact_title(&artifact.artifact))
        .or_else(|| {
            workstream
                .episodes
                .last()
                .map(|(episode, _, _)| episode.summary_label.clone())
        })
}

fn artifact_title(artifact: &ContinueArtifactRecord) -> Option<String> {
    first_non_empty([
        artifact.display_title.as_deref(),
        artifact.document_path.as_deref(),
        artifact.browser_url.as_deref(),
        artifact.app_name.as_deref(),
    ])
    .map(str::to_string)
}

fn unresolved_signal_for_workstream(workstream: &BuiltWorkstream) -> Option<String> {
    let mut branch_without_return: Option<&ContinueActionRecord> = None;
    let mut terminal_without_return: Option<&ContinueActionRecord> = None;
    let mut last_return_timestamp = 0_i64;
    for (episode, _, _) in &workstream.episodes {
        for action in &episode.actions {
            match action.action_kind.as_str() {
                "idle_after_progress" => {
                    return Some(unresolved_signal_json(
                        "idle_after_progress",
                        action,
                        action.confidence.max(0.62),
                    ));
                }
                "encountering_error" => {
                    return Some(unresolved_signal_json(
                        "visible_error_or_failure",
                        action,
                        action.confidence.max(0.72),
                    ));
                }
                "composing"
                    if action
                        .artifact
                        .as_ref()
                        .map(|artifact| {
                            matches!(
                                artifact.artifact_kind.as_str(),
                                "messaging" | "chat_conversation"
                            )
                        })
                        .unwrap_or(false) =>
                {
                    return Some(unresolved_signal_json(
                        "draft_or_composer_active",
                        action,
                        action.confidence.max(0.6),
                    ));
                }
                "branching_away" | "searching" => branch_without_return = Some(action),
                "verification_branch"
                | "running_command"
                | "observing_command_output"
                | "reviewing_output" => terminal_without_return = Some(action),
                "returning_to_origin" => {
                    last_return_timestamp = action.created_at_ms;
                    branch_without_return = None;
                    terminal_without_return = None;
                }
                _ => {}
            }
        }
    }
    if let Some(action) = terminal_without_return {
        if action.created_at_ms > last_return_timestamp {
            return Some(unresolved_signal_json(
                "verification_without_return",
                action,
                action.confidence.max(0.58),
            ));
        }
    }
    if let Some(action) = branch_without_return {
        if action.created_at_ms > last_return_timestamp {
            return Some(unresolved_signal_json(
                "branch_without_return",
                action,
                action.confidence.max(0.56),
            ));
        }
    }
    None
}

fn unresolved_signal_json(kind: &str, action: &ContinueActionRecord, confidence: f64) -> String {
    serde_json::json!({
        "kind": kind,
        "evidence_action_id": action.id,
        "evidence_frame_id": action.frame_id,
        "confidence": (confidence * 100.0).round() / 100.0
    })
    .to_string()
}

fn finalize_workstream_states(workstreams: &mut [BuiltWorkstream]) {
    let latest_timestamp = workstreams
        .iter()
        .map(|workstream| workstream.last_active_timestamp_ms)
        .max()
        .unwrap_or_default();
    for workstream in workstreams {
        let last_episode = workstream.episodes.last().map(|(episode, _, _)| episode);
        let has_return = workstream.episodes.iter().any(|(episode, _, _)| {
            episode
                .actions
                .iter()
                .any(|action| action.action_kind == "returning_to_origin")
        });
        let last_is_interruption = last_episode
            .map(|episode| {
                episode
                    .actions
                    .iter()
                    .any(|action| action.action_kind == "messaging_interrupt")
            })
            .unwrap_or(false);
        let stale =
            latest_timestamp.saturating_sub(workstream.last_active_timestamp_ms) > 30 * 60 * 1000;
        workstream.state = if stale {
            "stale".to_string()
        } else if last_is_interruption && workstream.episodes.len() == 1 {
            "abandoned".to_string()
        } else if has_return {
            "resumed".to_string()
        } else if workstream.unresolved_signal.is_some()
            || last_episode
                .and_then(|episode| episode.boundary_end_reason.as_deref())
                .map(|reason| {
                    matches!(
                        reason,
                        "idle_after_progress"
                            | "left_primary_for_support"
                            | "error_to_search_or_support"
                            | "left_primary_for_verification"
                    )
                })
                .unwrap_or(false)
        {
            workstream.suspended_timestamp_ms = Some(workstream.last_active_timestamp_ms);
            "suspended".to_string()
        } else if workstream.last_active_timestamp_ms == latest_timestamp {
            "active".to_string()
        } else {
            "background".to_string()
        };
    }
}

fn insert_continue_episode(conn: &Connection, episode: &BuiltEpisode) -> Result<(), String> {
    conn.execute(
        "INSERT INTO continue_episodes (
            id, state, start_frame_id, end_frame_id, start_timestamp_ms,
            end_timestamp_ms, primary_artifact_id, dominant_action_kind,
            boundary_start_reason, boundary_end_reason, confidence, evidence_quality,
            summary_label
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
        params![
            episode.id,
            episode.state,
            episode.start_frame_id,
            episode.end_frame_id,
            episode.start_timestamp_ms,
            episode.end_timestamp_ms,
            episode.primary_artifact_id,
            episode.dominant_action_kind,
            episode.boundary_start_reason,
            episode.boundary_end_reason,
            episode.confidence,
            episode.evidence_quality,
            episode.summary_label,
        ],
    )
    .map_err(to_string)?;
    for (index, action) in episode.actions.iter().enumerate() {
        conn.execute(
            "INSERT INTO continue_episode_actions (
                episode_id, action_id, order_index, role_in_episode, confidence
             ) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                episode.id,
                action.id,
                index as i64,
                episode_action_role(action),
                action.confidence,
            ],
        )
        .map_err(to_string)?;
    }
    for artifact in episode.artifacts.values() {
        conn.execute(
            "INSERT INTO continue_episode_artifacts (
                episode_id, artifact_id, artifact_role, first_frame_id,
                last_frame_id, contribution_score, reason
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                episode.id,
                artifact.artifact.id,
                artifact.role,
                artifact.first_frame_id,
                artifact.last_frame_id,
                artifact.contribution_score,
                artifact.reason,
            ],
        )
        .map_err(to_string)?;
    }
    Ok(())
}

fn episode_action_role(action: &ContinueActionRecord) -> String {
    match action.action_kind.as_str() {
        "returning_to_origin" => "return".to_string(),
        "messaging_interrupt" => "interrupt".to_string(),
        _ => action.action_role.clone(),
    }
}

fn insert_continue_workstream(
    conn: &Connection,
    workstream: &BuiltWorkstream,
) -> Result<(), String> {
    conn.execute(
        "INSERT INTO continue_workstreams (
            id, state, title_candidate, inferred_intent, primary_artifact_id,
            created_at_ms, last_active_timestamp_ms, suspended_timestamp_ms,
            confidence, unresolved_signal, source
         ) VALUES (?1, ?2, ?3, NULL, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            workstream.id,
            workstream.state,
            workstream.title_candidate,
            workstream.primary_artifact_id,
            workstream.created_at_ms,
            workstream.last_active_timestamp_ms,
            workstream.suspended_timestamp_ms,
            workstream.confidence,
            workstream.unresolved_signal,
            workstream.source,
        ],
    )
    .map_err(to_string)?;
    for (index, (episode, score, reason)) in workstream.episodes.iter().enumerate() {
        conn.execute(
            "INSERT INTO continue_workstream_episodes (
                workstream_id, episode_id, membership_score, membership_reason, order_index
             ) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![workstream.id, episode.id, score, reason, index as i64],
        )
        .map_err(to_string)?;
    }
    for artifact in workstream.artifacts.values() {
        conn.execute(
            "INSERT INTO continue_workstream_artifacts (
                workstream_id, artifact_id, durable_role, importance_score,
                first_seen_frame_id, last_seen_frame_id, reason
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                workstream.id,
                artifact.artifact.id,
                artifact.durable_role,
                artifact.importance_score,
                artifact.first_seen_frame_id,
                artifact.last_seen_frame_id,
                artifact.reason,
            ],
        )
        .map_err(to_string)?;
    }
    Ok(())
}

fn insert_continue_candidate(
    conn: &Connection,
    candidate: &ScoredContinueCandidate,
    created_at_ms: i64,
) -> Result<(), String> {
    conn.execute(
        "INSERT OR REPLACE INTO continue_candidates (
            id, workstream_id, target_artifact_id, candidate_kind,
            last_meaningful_action_id, evidence_frame_id, supporting_episode_id,
            score, actionability_score, primary_target_score, unresolved_score,
            branch_origin_score, evidence_quality_score, recency_score,
            openability_score, privacy_safety_score, reason, missing_evidence,
            created_at_ms
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12,
                   ?13, ?14, ?15, ?16, ?17, ?18, ?19)",
        params![
            candidate.id,
            candidate.workstream_id,
            candidate
                .target_artifact
                .as_ref()
                .map(|artifact| artifact.id.clone()),
            candidate.candidate_kind,
            candidate
                .last_meaningful_action
                .as_ref()
                .map(|action| action.id.clone()),
            candidate.evidence_frame_id,
            candidate.supporting_episode_id,
            candidate.score,
            candidate.actionability_score,
            candidate.primary_target_score,
            candidate.unresolved_score,
            candidate.branch_origin_score,
            candidate.evidence_quality_score,
            candidate.recency_score,
            candidate.openability_score,
            candidate.privacy_safety_score,
            candidate.reason,
            if candidate.missing_evidence.is_empty() {
                None
            } else {
                Some(candidate.missing_evidence.join(";"))
            },
            created_at_ms,
        ],
    )
    .map_err(to_string)?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn insert_continue_decision(
    conn: &Connection,
    decision_id: &str,
    requested_at_ms: i64,
    current_focus: Option<&ContinueFocusSummary>,
    selected_workstream: Option<&ScorerWorkstream>,
    selected: Option<&ScoredContinueCandidate>,
    next_action: Option<&str>,
    warnings: &[String],
    validation_status: &str,
    source: &str,
    decision_reason: Option<&str>,
    confidence: f64,
    response_id: Option<&str>,
    model: Option<&str>,
    validation_notes: Option<&str>,
) -> Result<(), String> {
    conn.execute(
        "INSERT OR REPLACE INTO continue_decisions (
            id, requested_at_ms, source, current_focus_frame_id,
            current_focus_artifact_id, selected_workstream_id, selected_candidate_id,
            return_target_artifact_id, confidence, decision_reason, next_action,
            warnings, validation_status, response_id, model, validation_notes
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
        params![
            decision_id,
            requested_at_ms,
            source,
            current_focus.map(|focus| focus.frame_id.clone()),
            current_focus.and_then(|focus| focus.artifact_id.clone()),
            selected_workstream.map(|workstream| workstream.id.clone()),
            selected.map(|candidate| candidate.id.clone()),
            selected.and_then(|candidate| {
                candidate
                    .target_artifact
                    .as_ref()
                    .map(|artifact| artifact.id.clone())
            }),
            confidence,
            decision_reason,
            next_action,
            if warnings.is_empty() {
                None
            } else {
                Some(warnings.join(";"))
            },
            validation_status,
            response_id,
            model,
            validation_notes,
        ],
    )
    .map_err(to_string)?;
    Ok(())
}

fn recent_episode_actions(
    conn: &Connection,
    episode_id: &str,
) -> Result<Vec<RecentContinueEpisodeAction>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT ea.action_id, ta.frame_id, ta.action_kind, ea.role_in_episode,
                    ea.order_index, ea.confidence
             FROM continue_episode_actions ea
             JOIN continue_task_actions ta ON ta.id = ea.action_id
             WHERE ea.episode_id = ?1
             ORDER BY ea.order_index ASC",
        )
        .map_err(to_string)?;
    let rows = stmt
        .query_map(params![episode_id], |row| {
            Ok(RecentContinueEpisodeAction {
                action_id: row.get(0)?,
                frame_id: row.get(1)?,
                action_kind: row.get(2)?,
                role_in_episode: row.get(3)?,
                order_index: row.get(4)?,
                confidence: row.get(5)?,
            })
        })
        .map_err(to_string)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(to_string)
}

fn recent_episode_artifacts(
    conn: &Connection,
    episode_id: &str,
) -> Result<Vec<RecentContinueEpisodeArtifact>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT ea.artifact_id, ea.artifact_role, a.display_title, a.stable_key,
                    ea.first_frame_id, ea.last_frame_id, ea.contribution_score, ea.reason
             FROM continue_episode_artifacts ea
             JOIN continue_artifacts a ON a.id = ea.artifact_id
             WHERE ea.episode_id = ?1
             ORDER BY ea.contribution_score DESC, ea.artifact_role ASC",
        )
        .map_err(to_string)?;
    let rows = stmt
        .query_map(params![episode_id], |row| {
            Ok(RecentContinueEpisodeArtifact {
                artifact_id: row.get(0)?,
                artifact_role: row.get(1)?,
                display_title: row.get(2)?,
                stable_key: row.get(3)?,
                first_frame_id: row.get(4)?,
                last_frame_id: row.get(5)?,
                contribution_score: row.get(6)?,
                reason: row.get(7)?,
            })
        })
        .map_err(to_string)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(to_string)
}

fn recent_workstream_episodes(
    conn: &Connection,
    workstream_id: &str,
) -> Result<Vec<RecentContinueWorkstreamEpisode>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT episode_id, membership_score, membership_reason, order_index
             FROM continue_workstream_episodes
             WHERE workstream_id = ?1
             ORDER BY order_index ASC",
        )
        .map_err(to_string)?;
    let rows = stmt
        .query_map(params![workstream_id], |row| {
            Ok(RecentContinueWorkstreamEpisode {
                episode_id: row.get(0)?,
                membership_score: row.get(1)?,
                membership_reason: row.get(2)?,
                order_index: row.get(3)?,
            })
        })
        .map_err(to_string)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(to_string)
}

fn recent_workstream_artifacts(
    conn: &Connection,
    workstream_id: &str,
) -> Result<Vec<RecentContinueWorkstreamArtifact>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT wa.artifact_id, wa.durable_role, a.display_title, a.stable_key,
                    wa.importance_score, wa.first_seen_frame_id, wa.last_seen_frame_id,
                    wa.reason
             FROM continue_workstream_artifacts wa
             JOIN continue_artifacts a ON a.id = wa.artifact_id
             WHERE wa.workstream_id = ?1
             ORDER BY wa.importance_score DESC, wa.durable_role ASC",
        )
        .map_err(to_string)?;
    let rows = stmt
        .query_map(params![workstream_id], |row| {
            Ok(RecentContinueWorkstreamArtifact {
                artifact_id: row.get(0)?,
                durable_role: row.get(1)?,
                display_title: row.get(2)?,
                stable_key: row.get(3)?,
                importance_score: row.get(4)?,
                first_seen_frame_id: row.get(5)?,
                last_seen_frame_id: row.get(6)?,
                reason: row.get(7)?,
            })
        })
        .map_err(to_string)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(to_string)
}

fn is_terminal_verification_action(action: &ContinueActionRecord) -> bool {
    action
        .artifact
        .as_ref()
        .map(|artifact| artifact.artifact_kind == "terminal")
        .unwrap_or(false)
        && matches!(
            action.action_kind.as_str(),
            "running_command" | "observing_command_output" | "reviewing_output"
        )
}

fn evidence_event_ids(frame: &EvidenceFrame) -> Vec<String> {
    let mut ids = Vec::new();
    let mut seen = HashSet::new();
    if let Some(trigger) = &frame.trigger {
        push_unique(&mut ids, &mut seen, trigger.id.clone());
        for event_id in &trigger.caused_by_event_ids {
            push_unique(&mut ids, &mut seen, event_id.clone());
        }
    }
    if let Some(transition) = &frame.transition {
        push_unique(&mut ids, &mut seen, transition.id.clone());
        if let Some(event_id) = &transition.primary_event_id {
            push_unique(&mut ids, &mut seen, event_id.clone());
        }
    }
    for event in &frame.ui_events {
        push_unique(&mut ids, &mut seen, event.id.clone());
    }
    for burst in &frame.typing_bursts {
        push_unique(&mut ids, &mut seen, burst.id.clone());
    }
    for event in &frame.clipboard_events {
        push_unique(&mut ids, &mut seen, event.id.clone());
    }
    ids
}

fn push_unique(ids: &mut Vec<String>, seen: &mut HashSet<String>, id: String) {
    if !id.trim().is_empty() && seen.insert(id.clone()) {
        ids.push(id);
    }
}

fn has_typing_signal(frame: &EvidenceFrame) -> bool {
    !frame.typing_bursts.is_empty()
        || frame.ui_events.iter().any(|event| {
            event.event_type == "key_down"
                || event
                    .key_category
                    .as_deref()
                    .map(|category| matches!(category, "character" | "delete" | "enter"))
                    .unwrap_or(false)
        })
        || frame.capture_trigger == "typing_pause"
        || frame
            .trigger
            .as_ref()
            .map(|trigger| trigger.trigger_type.contains("typing"))
            .unwrap_or(false)
}

fn terminal_has_enter_or_commit(frame: &EvidenceFrame) -> bool {
    frame.typing_bursts.iter().any(|burst| {
        burst.enter_count > 0
            || burst.committed
            || burst
                .commit_signal
                .as_deref()
                .map(|signal| signal.contains("enter") || signal.contains("return"))
                .unwrap_or(false)
    }) || frame.ui_events.iter().any(|event| {
        event
            .key_category
            .as_deref()
            .map(|category| category == "enter")
            .unwrap_or(false)
    })
}

fn terminal_output_changed(frame: &EvidenceFrame) -> bool {
    has_content_role(frame, "terminal_output")
        || frame
            .frame_diff
            .as_ref()
            .map(|diff| {
                diff.added_text_hashes.as_deref().unwrap_or("").trim() != ""
                    || diff
                        .diff_type
                        .as_deref()
                        .map(|value| value.contains("text") || value.contains("content"))
                        .unwrap_or(false)
            })
            .unwrap_or(false)
}

fn has_output_review_signal(frame: &EvidenceFrame, artifact_kind: &str) -> bool {
    matches!(artifact_kind, "terminal" | "chat_conversation")
        || has_content_role(frame, "terminal_output")
        || has_content_role(frame, "chat_message")
        || frame_text_lower(frame).contains("generated")
}

fn has_clipboard_transfer(frame: &EvidenceFrame) -> bool {
    frame.clipboard_events.iter().any(|event| {
        event
            .target_frame_id
            .as_deref()
            .map(|target| target == frame.id)
            .unwrap_or(false)
            || event
                .source_frame_id
                .as_deref()
                .map(|source| source == frame.id)
                .unwrap_or(false)
    }) || frame
        .typing_bursts
        .iter()
        .any(|burst| burst.paste_count > 0)
}

fn is_composer_context(frame: &EvidenceFrame, artifact_kind: &str) -> bool {
    matches!(artifact_kind, "messaging" | "chat_conversation")
        || has_content_role(frame, "composer")
        || frame
            .app_contexts
            .iter()
            .any(|context| contains_any(&context.object_type, &["composer", "form", "message"]))
}

fn is_search_context(frame: &EvidenceFrame, artifact: &ResolvedArtifact) -> bool {
    has_content_role(frame, "search_result")
        || artifact
            .browser_url
            .as_deref()
            .map(|url| {
                let lower = url.to_lowercase();
                lower.contains("/search")
                    || lower.contains("?q=")
                    || lower.contains("&q=")
                    || lower.contains("google.")
                    || lower.contains("bing.")
                    || lower.contains("duckduckgo.")
            })
            .unwrap_or(false)
        || contains_any(
            &frame.window_name.clone().unwrap_or_default(),
            &["search", "results"],
        )
}

fn is_navigation_signal(frame: &EvidenceFrame) -> bool {
    frame.capture_trigger.contains("navigation")
        || frame.ui_events.iter().any(|event| {
            matches!(
                event.event_type.as_str(),
                "scroll" | "mouse_down" | "mouse_up" | "window_focus"
            )
        })
        || frame
            .transition
            .as_ref()
            .and_then(|transition| transition.transition_type.as_deref())
            .map(|value| contains_any(value, &["navigation", "scroll", "switch"]))
            .unwrap_or(false)
}

fn is_verification_branch(
    frame: &EvidenceFrame,
    artifact: &ResolvedArtifact,
    previous_artifact: Option<&ResolvedArtifact>,
) -> bool {
    previous_artifact
        .map(|previous| is_primary_work_artifact(&previous.kind))
        .unwrap_or(false)
        && matches!(
            artifact.kind.as_str(),
            "terminal" | "browser_tab" | "chat_conversation"
        )
        && (contains_any(
            &frame_text_lower(frame),
            &["test", "build", "log", "result", "passed", "failed"],
        ) || artifact.kind == "terminal")
}

fn has_error_signal(frame: &EvidenceFrame) -> bool {
    if has_content_role(frame, "error") {
        return true;
    }
    let lower = frame_text_lower(frame);
    contains_any(
        &lower,
        &[
            "error:",
            "exception",
            "stack trace",
            "failed",
            "failing",
            "panic",
            "compile error",
            "build failed",
            "test failed",
            "auth failure",
            "validation error",
        ],
    )
}

fn has_content_role(frame: &EvidenceFrame, role: &str) -> bool {
    frame.content_units.iter().any(|unit| {
        unit.semantic_role
            .as_deref()
            .map(|value| value == role)
            .unwrap_or(false)
    })
}

fn frame_text_lower(frame: &EvidenceFrame) -> String {
    let mut text = frame.full_text.clone().unwrap_or_default();
    for unit in &frame.content_units {
        if let Some(unit_text) = &unit.text {
            text.push('\n');
            text.push_str(unit_text);
        }
    }
    text.to_lowercase()
}

fn is_primary_work_artifact(kind: &str) -> bool {
    matches!(kind, "code_editor" | "notes_doc" | "pdf" | "finder")
}

fn is_support_artifact(kind: &str) -> bool {
    matches!(
        kind,
        "browser_tab" | "chat_conversation" | "terminal" | "messaging" | "pdf"
    )
}

fn is_editable_artifact(kind: &str) -> bool {
    matches!(kind, "code_editor" | "notes_doc")
}

fn is_meaningful_action_kind(kind: &str) -> bool {
    matches!(
        kind,
        "editing"
            | "composing"
            | "copying_evidence"
            | "running_command"
            | "observing_command_output"
            | "reviewing_output"
            | "encountering_error"
    )
}

fn artifact_kind_for_path(
    path: &str,
    frame: &EvidenceFrame,
    context: Option<&EvidenceAppContext>,
) -> String {
    if path.to_lowercase().ends_with(".pdf") {
        return "pdf".to_string();
    }
    if is_code_like_path(path) || app_is_code_editor(frame) {
        return "code_editor".to_string();
    }
    if context
        .map(|ctx| artifact_kind_for_context(&ctx.object_type, frame))
        .filter(|kind| kind != "unknown")
        .is_some()
    {
        return context
            .map(|ctx| artifact_kind_for_context(&ctx.object_type, frame))
            .unwrap_or_else(|| "unknown".to_string());
    }
    if contains_any(path, &["notes", "notion", "docs"]) {
        "notes_doc".to_string()
    } else {
        "unknown".to_string()
    }
}

fn artifact_kind_for_url(
    url: &str,
    frame: &EvidenceFrame,
    context: Option<&EvidenceAppContext>,
) -> String {
    if let Some(context) = context {
        let context_kind = artifact_kind_for_context(&context.object_type, frame);
        if context_kind != "unknown" {
            return context_kind;
        }
    }
    let lower = format!(
        "{} {} {}",
        url.to_lowercase(),
        frame.app_name.clone().unwrap_or_default().to_lowercase(),
        frame.window_name.clone().unwrap_or_default().to_lowercase()
    );
    if contains_any(
        &lower,
        &[
            "chat.openai.com",
            "chatgpt.com",
            "claude.ai",
            "gemini.google",
        ],
    ) {
        "chat_conversation".to_string()
    } else if contains_any(&lower, &["slack.com", "discord.com", "web.whatsapp.com"]) {
        "messaging".to_string()
    } else if contains_any(&lower, &["notion.so", "linear.app", "docs.google.com"]) {
        "notes_doc".to_string()
    } else if lower.ends_with(".pdf") || lower.contains(".pdf?") {
        "pdf".to_string()
    } else {
        "browser_tab".to_string()
    }
}

fn artifact_kind_for_context(object_type: &str, frame: &EvidenceFrame) -> String {
    let object_type = object_type.to_lowercase();
    if contains_any(&object_type, &["chat_conversation", "chat"]) {
        "chat_conversation".to_string()
    } else if contains_any(&object_type, &["browser_tab", "browser"]) {
        "browser_tab".to_string()
    } else if contains_any(&object_type, &["code_editor", "code", "editor"]) {
        "code_editor".to_string()
    } else if object_type.contains("terminal") {
        "terminal".to_string()
    } else if object_type.contains("pdf") {
        "pdf".to_string()
    } else if object_type.contains("finder") {
        "finder".to_string()
    } else if contains_any(&object_type, &["messaging", "message", "slack", "discord"]) {
        "messaging".to_string()
    } else if contains_any(&object_type, &["notes_doc", "note", "doc", "task"]) {
        "notes_doc".to_string()
    } else {
        artifact_kind_for_frame(frame, None)
    }
}

fn artifact_kind_for_frame(frame: &EvidenceFrame, context: Option<&EvidenceAppContext>) -> String {
    if let Some(context) = context {
        let context_kind = artifact_kind_for_context(&context.object_type, frame);
        if context_kind != "unknown" {
            return context_kind;
        }
    }
    let app = frame.app_name.clone().unwrap_or_default().to_lowercase();
    let bundle = frame
        .app_bundle_id
        .clone()
        .unwrap_or_default()
        .to_lowercase();
    let title = frame.window_name.clone().unwrap_or_default().to_lowercase();
    let surface = format!("{} {} {}", app, bundle, title);
    if contains_any(&surface, &["cursor", "code", "xcode", "intellij", "vscode"]) {
        "code_editor".to_string()
    } else if contains_any(&surface, &["terminal", "iterm", "warp"]) {
        "terminal".to_string()
    } else if contains_any(&surface, &["finder"]) {
        "finder".to_string()
    } else if contains_any(&surface, &["slack", "discord", "messages", "whatsapp"]) {
        "messaging".to_string()
    } else if contains_any(&surface, &["notes", "notion", "linear", "docs"]) {
        "notes_doc".to_string()
    } else if contains_any(&surface, &["preview", "pdf"]) {
        "pdf".to_string()
    } else if frame.browser_url.is_some() {
        "browser_tab".to_string()
    } else {
        "unknown".to_string()
    }
}

fn app_is_code_editor(frame: &EvidenceFrame) -> bool {
    let app = format!(
        "{} {}",
        frame.app_name.clone().unwrap_or_default(),
        frame.app_bundle_id.clone().unwrap_or_default()
    )
    .to_lowercase();
    contains_any(
        &app,
        &[
            "cursor",
            "visual studio code",
            "vscode",
            "xcode",
            "intellij",
        ],
    )
}

fn is_code_like_path(path: &str) -> bool {
    let lower = path.to_lowercase();
    [
        ".rs", ".ts", ".tsx", ".js", ".jsx", ".py", ".swift", ".go", ".java", ".kt", ".rb", ".md",
        ".json", ".toml", ".yaml", ".yml", ".css", ".html",
    ]
    .iter()
    .any(|suffix| lower.ends_with(suffix))
}

fn continue_text_source(frame: &EvidenceFrame) -> &'static str {
    match frame
        .text_source
        .as_deref()
        .unwrap_or("")
        .to_lowercase()
        .as_str()
    {
        "accessibility" | "ax" => "accessibility",
        "ocr" => "ocr",
        "hybrid" => "hybrid",
        value if value.contains("accessibility") && value.contains("ocr") => "hybrid",
        value if value.contains("ocr") => "ocr",
        value if value.contains("accessibility") => "accessibility",
        _ => "missing",
    }
}

fn first_non_empty<'a>(values: impl IntoIterator<Item = Option<&'a str>>) -> Option<&'a str> {
    values
        .into_iter()
        .flatten()
        .map(str::trim)
        .find(|value| !value.is_empty())
}

fn normalize_document_path(path: &str) -> String {
    let mut value = path.trim().replace('\\', "/");
    while value.contains("//") {
        value = value.replace("//", "/");
    }
    value.trim_end_matches('/').to_string()
}

fn canonicalize_url(raw_url: &str) -> Option<String> {
    let trimmed = raw_url.trim();
    if trimmed.is_empty() {
        return None;
    }
    let (scheme, rest) = if let Some(rest) = trimmed.strip_prefix("https://") {
        ("https", rest)
    } else if let Some(rest) = trimmed.strip_prefix("http://") {
        ("http", rest)
    } else {
        return None;
    };
    let (before_fragment, fragment) = match rest.split_once('#') {
        Some((before, fragment)) if !fragment.trim().is_empty() => (before, Some(fragment)),
        Some((before, _)) => (before, None),
        None => (rest, None),
    };
    let (before_query, query) = match before_fragment.split_once('?') {
        Some((before, query)) => (before, Some(query)),
        None => (before_fragment, None),
    };
    let (host, path) = match before_query.split_once('/') {
        Some((host, path)) => (
            host.to_lowercase(),
            format!("/{}", path.trim_end_matches('/')),
        ),
        None => (before_query.to_lowercase(), String::new()),
    };
    let kept_query = query
        .map(|query| {
            query
                .split('&')
                .filter(|pair| {
                    let key = pair.split('=').next().unwrap_or("").to_lowercase();
                    !(key.starts_with("utm_")
                        || matches!(
                            key.as_str(),
                            "fbclid" | "gclid" | "mc_cid" | "mc_eid" | "igshid"
                        ))
                })
                .collect::<Vec<_>>()
                .join("&")
        })
        .filter(|query| !query.is_empty());
    let mut canonical = format!("{}://{}{}", scheme, host, path);
    if let Some(query) = kept_query {
        canonical.push('?');
        canonical.push_str(&query);
    }
    if let Some(fragment) = fragment {
        canonical.push('#');
        canonical.push_str(fragment);
    }
    Some(canonical)
}

fn normalize_window_title(title: &str) -> String {
    let mut value = title.trim().to_lowercase();
    for delimiter in [" - ", " — ", " | "] {
        if let Some((head, tail)) = value.split_once(delimiter) {
            if head.len() > 8 {
                value = head.to_string();
            } else if tail.len() > 8 {
                value = tail.to_string();
            }
        }
    }
    value = value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(160)
        .collect();
    value
}

fn normalize_token(value: &str) -> String {
    value
        .trim()
        .to_lowercase()
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '/' | '_' | '-' | ':') {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>()
        .split('_')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("_")
}

fn contains_any(value: &str, needles: &[&str]) -> bool {
    let lower = value.to_lowercase();
    needles.iter().any(|needle| lower.contains(needle))
}

fn parse_string_array(raw: &str) -> Vec<String> {
    serde_json::from_str::<Vec<String>>(raw).unwrap_or_default()
}

fn bool_to_i64(value: bool) -> i64 {
    if value {
        1
    } else {
        0
    }
}

fn stable_hash(bytes: &[u8]) -> String {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{:016x}", hash)
}

fn current_time_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or_default()
}

pub fn ensure_continue_schema(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS continue_schema_migrations (
          version INTEGER PRIMARY KEY,
          name TEXT NOT NULL,
          applied_at_ms INTEGER NOT NULL
        );

        INSERT OR IGNORE INTO continue_schema_migrations (version, name, applied_at_ms)
        VALUES (1, 'continue_semantic_memory_foundation', CAST(strftime('%s','now') AS INTEGER) * 1000);

        CREATE TABLE IF NOT EXISTS continue_artifacts (
          id TEXT PRIMARY KEY,
          artifact_kind TEXT NOT NULL,
          stable_key TEXT NOT NULL,
          app_name TEXT,
          bundle_id TEXT,
          window_title TEXT,
          browser_url TEXT,
          document_path TEXT,
          display_title TEXT,
          first_seen_frame_id TEXT,
          last_seen_frame_id TEXT,
          first_seen_timestamp INTEGER NOT NULL,
          last_seen_timestamp INTEGER NOT NULL,
          identity_confidence REAL NOT NULL DEFAULT 0.0,
          evidence_quality TEXT NOT NULL DEFAULT 'thin',
          privacy_status TEXT,
          openability TEXT NOT NULL DEFAULT 'unknown',
          created_at_ms INTEGER NOT NULL,
          updated_at_ms INTEGER NOT NULL
        );
        CREATE UNIQUE INDEX IF NOT EXISTS idx_continue_artifacts_stable_key
          ON continue_artifacts(stable_key);
        CREATE INDEX IF NOT EXISTS idx_continue_artifacts_kind_last_seen
          ON continue_artifacts(artifact_kind, last_seen_timestamp DESC);
        CREATE INDEX IF NOT EXISTS idx_continue_artifacts_last_seen
          ON continue_artifacts(last_seen_timestamp DESC);

        CREATE TABLE IF NOT EXISTS continue_artifact_observations (
          id TEXT PRIMARY KEY,
          artifact_id TEXT NOT NULL,
          frame_id TEXT NOT NULL,
          app_context_id TEXT,
          text_source TEXT NOT NULL DEFAULT 'missing',
          content_hash TEXT,
          image_hash TEXT,
          focused_node_evidence INTEGER NOT NULL DEFAULT 0,
          selected_text_present INTEGER NOT NULL DEFAULT 0,
          visible_text_length INTEGER NOT NULL DEFAULT 0,
          observation_confidence REAL NOT NULL DEFAULT 0.0,
          reason TEXT,
          timestamp_ms INTEGER NOT NULL,
          FOREIGN KEY(artifact_id) REFERENCES continue_artifacts(id) ON DELETE CASCADE
        );
        CREATE INDEX IF NOT EXISTS idx_continue_artifact_observations_artifact
          ON continue_artifact_observations(artifact_id, timestamp_ms);
        CREATE INDEX IF NOT EXISTS idx_continue_artifact_observations_frame
          ON continue_artifact_observations(frame_id);

        CREATE TABLE IF NOT EXISTS continue_task_actions (
          id TEXT PRIMARY KEY,
          frame_id TEXT NOT NULL,
          previous_frame_id TEXT,
          artifact_id TEXT,
          secondary_artifact_id TEXT,
          action_kind TEXT NOT NULL DEFAULT 'unknown',
          action_role TEXT NOT NULL DEFAULT 'unknown',
          trigger_type TEXT,
          transition_label TEXT,
          evidence_event_ids_json TEXT NOT NULL DEFAULT '[]',
          confidence REAL NOT NULL DEFAULT 0.0,
          reason TEXT,
          created_at_ms INTEGER NOT NULL,
          collapse_count INTEGER NOT NULL DEFAULT 1,
          first_frame_id TEXT,
          last_frame_id TEXT,
          strongest_frame_id TEXT,
          FOREIGN KEY(artifact_id) REFERENCES continue_artifacts(id) ON DELETE SET NULL,
          FOREIGN KEY(secondary_artifact_id) REFERENCES continue_artifacts(id) ON DELETE SET NULL
        );
        CREATE INDEX IF NOT EXISTS idx_continue_task_actions_frame
          ON continue_task_actions(frame_id);
        CREATE INDEX IF NOT EXISTS idx_continue_task_actions_artifact
          ON continue_task_actions(artifact_id, created_at_ms);
        CREATE INDEX IF NOT EXISTS idx_continue_task_actions_kind
          ON continue_task_actions(action_kind, created_at_ms);

        CREATE TABLE IF NOT EXISTS continue_task_action_events (
          action_id TEXT NOT NULL,
          event_id TEXT NOT NULL,
          order_index INTEGER NOT NULL DEFAULT 0,
          PRIMARY KEY(action_id, event_id),
          FOREIGN KEY(action_id) REFERENCES continue_task_actions(id) ON DELETE CASCADE
        );
        CREATE INDEX IF NOT EXISTS idx_continue_task_action_events_event
          ON continue_task_action_events(event_id);

        CREATE TABLE IF NOT EXISTS continue_episodes (
          id TEXT PRIMARY KEY,
          state TEXT NOT NULL DEFAULT 'open',
          start_frame_id TEXT,
          end_frame_id TEXT,
          start_timestamp_ms INTEGER NOT NULL,
          end_timestamp_ms INTEGER,
          primary_artifact_id TEXT,
          dominant_action_kind TEXT,
          boundary_start_reason TEXT,
          boundary_end_reason TEXT,
          confidence REAL NOT NULL DEFAULT 0.0,
          evidence_quality TEXT NOT NULL DEFAULT 'thin',
          summary_label TEXT,
          FOREIGN KEY(primary_artifact_id) REFERENCES continue_artifacts(id) ON DELETE SET NULL
        );
        CREATE INDEX IF NOT EXISTS idx_continue_episodes_state_start
          ON continue_episodes(state, start_timestamp_ms);
        CREATE INDEX IF NOT EXISTS idx_continue_episodes_primary_artifact
          ON continue_episodes(primary_artifact_id);

        CREATE TABLE IF NOT EXISTS continue_episode_actions (
          episode_id TEXT NOT NULL,
          action_id TEXT NOT NULL,
          order_index INTEGER NOT NULL DEFAULT 0,
          role_in_episode TEXT NOT NULL DEFAULT 'unknown',
          confidence REAL NOT NULL DEFAULT 0.0,
          PRIMARY KEY(episode_id, action_id),
          FOREIGN KEY(episode_id) REFERENCES continue_episodes(id) ON DELETE CASCADE,
          FOREIGN KEY(action_id) REFERENCES continue_task_actions(id) ON DELETE CASCADE
        );
        CREATE INDEX IF NOT EXISTS idx_continue_episode_actions_action
          ON continue_episode_actions(action_id);

        CREATE TABLE IF NOT EXISTS continue_episode_artifacts (
          episode_id TEXT NOT NULL,
          artifact_id TEXT NOT NULL,
          artifact_role TEXT NOT NULL DEFAULT 'unknown',
          first_frame_id TEXT,
          last_frame_id TEXT,
          contribution_score REAL NOT NULL DEFAULT 0.0,
          reason TEXT,
          PRIMARY KEY(episode_id, artifact_id, artifact_role),
          FOREIGN KEY(episode_id) REFERENCES continue_episodes(id) ON DELETE CASCADE,
          FOREIGN KEY(artifact_id) REFERENCES continue_artifacts(id) ON DELETE CASCADE
        );
        CREATE INDEX IF NOT EXISTS idx_continue_episode_artifacts_artifact
          ON continue_episode_artifacts(artifact_id);

        CREATE TABLE IF NOT EXISTS continue_workstreams (
          id TEXT PRIMARY KEY,
          state TEXT NOT NULL DEFAULT 'active',
          title_candidate TEXT,
          inferred_intent TEXT,
          primary_artifact_id TEXT,
          created_at_ms INTEGER NOT NULL,
          last_active_timestamp_ms INTEGER NOT NULL,
          suspended_timestamp_ms INTEGER,
          confidence REAL NOT NULL DEFAULT 0.0,
          unresolved_signal TEXT,
          source TEXT NOT NULL DEFAULT 'local_heuristic',
          FOREIGN KEY(primary_artifact_id) REFERENCES continue_artifacts(id) ON DELETE SET NULL
        );
        CREATE INDEX IF NOT EXISTS idx_continue_workstreams_state_last_active
          ON continue_workstreams(state, last_active_timestamp_ms DESC);
        CREATE INDEX IF NOT EXISTS idx_continue_workstreams_primary_artifact
          ON continue_workstreams(primary_artifact_id);

        CREATE TABLE IF NOT EXISTS continue_workstream_episodes (
          workstream_id TEXT NOT NULL,
          episode_id TEXT NOT NULL,
          membership_score REAL NOT NULL DEFAULT 0.0,
          membership_reason TEXT,
          order_index INTEGER NOT NULL DEFAULT 0,
          PRIMARY KEY(workstream_id, episode_id),
          FOREIGN KEY(workstream_id) REFERENCES continue_workstreams(id) ON DELETE CASCADE,
          FOREIGN KEY(episode_id) REFERENCES continue_episodes(id) ON DELETE CASCADE
        );
        CREATE INDEX IF NOT EXISTS idx_continue_workstream_episodes_episode
          ON continue_workstream_episodes(episode_id);

        CREATE TABLE IF NOT EXISTS continue_workstream_artifacts (
          workstream_id TEXT NOT NULL,
          artifact_id TEXT NOT NULL,
          durable_role TEXT NOT NULL DEFAULT 'unknown',
          importance_score REAL NOT NULL DEFAULT 0.0,
          first_seen_frame_id TEXT,
          last_seen_frame_id TEXT,
          reason TEXT,
          PRIMARY KEY(workstream_id, artifact_id, durable_role),
          FOREIGN KEY(workstream_id) REFERENCES continue_workstreams(id) ON DELETE CASCADE,
          FOREIGN KEY(artifact_id) REFERENCES continue_artifacts(id) ON DELETE CASCADE
        );
        CREATE INDEX IF NOT EXISTS idx_continue_workstream_artifacts_artifact
          ON continue_workstream_artifacts(artifact_id);

        CREATE TABLE IF NOT EXISTS continue_candidates (
          id TEXT PRIMARY KEY,
          workstream_id TEXT NOT NULL,
          target_artifact_id TEXT,
          candidate_kind TEXT NOT NULL,
          last_meaningful_action_id TEXT,
          evidence_frame_id TEXT,
          supporting_episode_id TEXT,
          score REAL NOT NULL DEFAULT 0.0,
          actionability_score REAL NOT NULL DEFAULT 0.0,
          primary_target_score REAL NOT NULL DEFAULT 0.0,
          unresolved_score REAL NOT NULL DEFAULT 0.0,
          branch_origin_score REAL NOT NULL DEFAULT 0.0,
          evidence_quality_score REAL NOT NULL DEFAULT 0.0,
          recency_score REAL NOT NULL DEFAULT 0.0,
          openability_score REAL NOT NULL DEFAULT 0.0,
          privacy_safety_score REAL NOT NULL DEFAULT 0.0,
          reason TEXT,
          missing_evidence TEXT,
          created_at_ms INTEGER NOT NULL,
          FOREIGN KEY(workstream_id) REFERENCES continue_workstreams(id) ON DELETE CASCADE,
          FOREIGN KEY(target_artifact_id) REFERENCES continue_artifacts(id) ON DELETE SET NULL,
          FOREIGN KEY(last_meaningful_action_id) REFERENCES continue_task_actions(id) ON DELETE SET NULL,
          FOREIGN KEY(supporting_episode_id) REFERENCES continue_episodes(id) ON DELETE SET NULL
        );
        CREATE INDEX IF NOT EXISTS idx_continue_candidates_workstream_score
          ON continue_candidates(workstream_id, score DESC);
        CREATE INDEX IF NOT EXISTS idx_continue_candidates_target
          ON continue_candidates(target_artifact_id);
        CREATE INDEX IF NOT EXISTS idx_continue_candidates_frame
          ON continue_candidates(evidence_frame_id);

        CREATE TABLE IF NOT EXISTS continue_decisions (
          id TEXT PRIMARY KEY,
          requested_at_ms INTEGER NOT NULL,
          source TEXT NOT NULL,
          current_focus_frame_id TEXT,
          current_focus_artifact_id TEXT,
          selected_workstream_id TEXT,
          selected_candidate_id TEXT,
          return_target_artifact_id TEXT,
          confidence REAL NOT NULL DEFAULT 0.0,
          decision_reason TEXT,
          next_action TEXT,
          warnings TEXT,
          validation_status TEXT NOT NULL DEFAULT 'thin_evidence',
          FOREIGN KEY(current_focus_artifact_id) REFERENCES continue_artifacts(id) ON DELETE SET NULL,
          FOREIGN KEY(selected_workstream_id) REFERENCES continue_workstreams(id) ON DELETE SET NULL,
          FOREIGN KEY(selected_candidate_id) REFERENCES continue_candidates(id) ON DELETE SET NULL,
          FOREIGN KEY(return_target_artifact_id) REFERENCES continue_artifacts(id) ON DELETE SET NULL
        );
        CREATE INDEX IF NOT EXISTS idx_continue_decisions_requested
          ON continue_decisions(requested_at_ms DESC);
        CREATE INDEX IF NOT EXISTS idx_continue_decisions_candidate
          ON continue_decisions(selected_candidate_id);

        CREATE TABLE IF NOT EXISTS continue_feedback_events (
          id TEXT PRIMARY KEY,
          decision_id TEXT,
          event_kind TEXT NOT NULL,
          observed_frame_id TEXT,
          target_artifact_id TEXT,
          chosen_artifact_id TEXT,
          timestamp_ms INTEGER NOT NULL,
          confidence REAL NOT NULL DEFAULT 0.0,
          reason TEXT,
          FOREIGN KEY(decision_id) REFERENCES continue_decisions(id) ON DELETE CASCADE,
          FOREIGN KEY(target_artifact_id) REFERENCES continue_artifacts(id) ON DELETE SET NULL,
          FOREIGN KEY(chosen_artifact_id) REFERENCES continue_artifacts(id) ON DELETE SET NULL
        );
        CREATE INDEX IF NOT EXISTS idx_continue_feedback_events_decision
          ON continue_feedback_events(decision_id);
        CREATE INDEX IF NOT EXISTS idx_continue_feedback_events_timestamp
          ON continue_feedback_events(timestamp_ms DESC);

        CREATE TABLE IF NOT EXISTS continue_breadcrumbs (
          id TEXT PRIMARY KEY,
          workstream_id TEXT NOT NULL,
          text TEXT NOT NULL,
          source TEXT NOT NULL DEFAULT 'manual',
          created_at_ms INTEGER NOT NULL,
          FOREIGN KEY(workstream_id) REFERENCES continue_workstreams(id) ON DELETE CASCADE
        );
        CREATE INDEX IF NOT EXISTS idx_continue_breadcrumbs_workstream
          ON continue_breadcrumbs(workstream_id, created_at_ms DESC);
        ",
    )
    .map_err(to_string)?;
    ensure_column_exists(conn, "continue_candidates", "supporting_episode_id", "TEXT")?;
    ensure_column_exists(
        conn,
        "continue_task_actions",
        "collapse_count",
        "INTEGER NOT NULL DEFAULT 1",
    )?;
    ensure_column_exists(conn, "continue_task_actions", "first_frame_id", "TEXT")?;
    ensure_column_exists(conn, "continue_task_actions", "last_frame_id", "TEXT")?;
    ensure_column_exists(conn, "continue_task_actions", "strongest_frame_id", "TEXT")?;
    ensure_column_exists(
        conn,
        "continue_candidates",
        "branch_origin_score",
        "REAL NOT NULL DEFAULT 0.0",
    )?;
    ensure_column_exists(
        conn,
        "continue_candidates",
        "privacy_safety_score",
        "REAL NOT NULL DEFAULT 0.0",
    )?;
    ensure_column_exists(conn, "continue_decisions", "response_id", "TEXT")?;
    ensure_column_exists(conn, "continue_decisions", "model", "TEXT")?;
    ensure_column_exists(conn, "continue_decisions", "validation_notes", "TEXT")?;
    ensure_column_exists(
        conn,
        "continue_feedback_events",
        "selected_candidate_id",
        "TEXT",
    )?;
    ensure_column_exists(conn, "continue_feedback_events", "workstream_id", "TEXT")?;
    ensure_column_exists(conn, "continue_feedback_events", "note", "TEXT")?;
    ensure_column_exists(
        conn,
        "continue_feedback_events",
        "source",
        "TEXT NOT NULL DEFAULT 'inferred'",
    )?;
    Ok(())
}

pub fn clear_continue_semantic_rows(conn: &Connection) -> Result<(), String> {
    if !table_exists(conn, "continue_artifacts")? {
        return Ok(());
    }
    conn.execute_batch(
        "
        DELETE FROM continue_breadcrumbs;
        DELETE FROM continue_feedback_events;
        DELETE FROM continue_decisions;
        DELETE FROM continue_candidates;
        DELETE FROM continue_workstream_artifacts;
        DELETE FROM continue_workstream_episodes;
        DELETE FROM continue_workstreams;
        DELETE FROM continue_episode_artifacts;
        DELETE FROM continue_episode_actions;
        DELETE FROM continue_episodes;
        DELETE FROM continue_task_action_events;
        DELETE FROM continue_task_actions;
        DELETE FROM continue_artifact_observations;
        DELETE FROM continue_artifacts;
        ",
    )
    .map_err(to_string)
}

pub fn continue_memory_status(conn: &Connection) -> Result<ContinueMemoryStatus, String> {
    let has_schema = has_continue_schema(conn)?;
    Ok(ContinueMemoryStatus {
        schema: CONTINUE_SCHEMA_NAME.to_string(),
        schema_version: latest_schema_version(conn)?,
        has_schema,
        counts: ContinueMemoryCounts {
            artifacts: count_if_present(conn, "continue_artifacts")?,
            artifact_observations: count_if_present(conn, "continue_artifact_observations")?,
            task_actions: count_if_present(conn, "continue_task_actions")?,
            task_action_events: count_if_present(conn, "continue_task_action_events")?,
            episodes: count_if_present(conn, "continue_episodes")?,
            episode_actions: count_if_present(conn, "continue_episode_actions")?,
            episode_artifacts: count_if_present(conn, "continue_episode_artifacts")?,
            workstreams: count_if_present(conn, "continue_workstreams")?,
            workstream_episodes: count_if_present(conn, "continue_workstream_episodes")?,
            workstream_artifacts: count_if_present(conn, "continue_workstream_artifacts")?,
            candidates: count_if_present(conn, "continue_candidates")?,
            decisions: count_if_present(conn, "continue_decisions")?,
            feedback_events: count_if_present(conn, "continue_feedback_events")?,
            breadcrumbs: count_if_present(conn, "continue_breadcrumbs")?,
        },
        latest_artifact_timestamp: max_i64_if_present(
            conn,
            "continue_artifacts",
            "last_seen_timestamp",
        )?,
        latest_workstream_timestamp: max_i64_if_present(
            conn,
            "continue_workstreams",
            "last_active_timestamp_ms",
        )?,
    })
}

fn has_continue_schema(conn: &Connection) -> Result<bool, String> {
    for table in CONTINUE_TABLES {
        if !table_exists(conn, table)? {
            return Ok(false);
        }
    }
    Ok(true)
}

fn latest_schema_version(conn: &Connection) -> Result<Option<i64>, String> {
    if !table_exists(conn, "continue_schema_migrations")? {
        return Ok(None);
    }
    conn.query_row(
        "SELECT MAX(version) FROM continue_schema_migrations",
        [],
        |row| row.get(0),
    )
    .map_err(to_string)
}

fn count_if_present(conn: &Connection, table: &str) -> Result<i64, String> {
    if !table_exists(conn, table)? {
        return Ok(0);
    }
    conn.query_row(&format!("SELECT COUNT(*) FROM {}", table), [], |row| {
        row.get(0)
    })
    .map_err(to_string)
}

fn max_i64_if_present(conn: &Connection, table: &str, column: &str) -> Result<Option<i64>, String> {
    if !table_exists(conn, table)? {
        return Ok(None);
    }
    conn.query_row(
        &format!("SELECT MAX({}) FROM {}", column, table),
        [],
        |row| row.get(0),
    )
    .optional()
    .map_err(to_string)
    .map(|value| value.flatten())
}

fn table_exists(conn: &Connection, table: &str) -> Result<bool, String> {
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
            [table],
            |row| row.get(0),
        )
        .map_err(to_string)?;
    Ok(count > 0)
}

fn column_exists(conn: &Connection, table: &str, column: &str) -> Result<bool, String> {
    if !table_exists(conn, table)? {
        return Ok(false);
    }
    let mut stmt = conn
        .prepare(&format!("PRAGMA table_info({})", table))
        .map_err(to_string)?;
    let rows = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(to_string)?;
    for row in rows {
        if row.map_err(to_string)? == column {
            return Ok(true);
        }
    }
    Ok(false)
}

fn ensure_column_exists(
    conn: &Connection,
    table: &str,
    column: &str,
    definition: &str,
) -> Result<(), String> {
    if !column_exists(conn, table, column)? {
        conn.execute(
            &format!("ALTER TABLE {} ADD COLUMN {} {}", table, column, definition),
            [],
        )
        .map_err(to_string)?;
    }
    Ok(())
}

fn to_string<E: std::fmt::Display>(error: E) -> String {
    error.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::params;

    #[test]
    fn continue_schema_status_reports_empty_foundation() {
        let conn = Connection::open_in_memory().unwrap();
        ensure_continue_schema(&conn).unwrap();

        let status = continue_memory_status(&conn).unwrap();

        assert!(status.has_schema);
        assert_eq!(status.schema, CONTINUE_SCHEMA_NAME);
        assert_eq!(status.schema_version, Some(CONTINUE_SCHEMA_VERSION));
        assert_eq!(status.counts.artifacts, 0);
        assert_eq!(status.counts.decisions, 0);
        assert_eq!(status.latest_artifact_timestamp, None);
        assert_eq!(status.latest_workstream_timestamp, None);
    }

    fn test_extracted_action(frame_id: &str, confidence: f64) -> ExtractedTaskAction {
        ExtractedTaskAction {
            id: format!("action-{}", frame_id),
            frame_id: frame_id.to_string(),
            previous_frame_id: None,
            artifact_id: Some("artifact-code".to_string()),
            secondary_artifact_id: None,
            action_kind: "encountering_error".to_string(),
            action_role: "blocker".to_string(),
            trigger_type: Some("typing_pause".to_string()),
            transition_label: Some("same_surface".to_string()),
            evidence_event_ids: vec![format!("event-{}", frame_id)],
            confidence,
            reason: "error_signal".to_string(),
            created_at_ms: 1_000 + frame_id.parse::<i64>().unwrap_or(0) * 1_000,
            collapse_count: 1,
            first_frame_id: Some(frame_id.to_string()),
            last_frame_id: Some(frame_id.to_string()),
            strongest_frame_id: Some(frame_id.to_string()),
        }
    }

    #[test]
    fn repeated_error_actions_collapse_with_typed_frame_metadata() {
        let collapsed = collapse_repeated_task_actions(vec![
            test_extracted_action("10", 0.64),
            test_extracted_action("11", 0.9),
            test_extracted_action("12", 0.72),
        ]);

        assert_eq!(collapsed.len(), 1);
        let action = &collapsed[0];
        assert_eq!(action.collapse_count, 3);
        assert_eq!(action.first_frame_id.as_deref(), Some("10"));
        assert_eq!(action.last_frame_id.as_deref(), Some("12"));
        assert_eq!(action.strongest_frame_id.as_deref(), Some("11"));
        assert_eq!(action.frame_id, "11");
        assert_eq!(action.reason, "error_signal");
        assert_eq!(action.evidence_event_ids.len(), 3);
    }

    #[test]
    fn confidence_caps_unknown_fallback_and_smalltalk_self_targets() {
        let unknown = ScorerArtifact {
            id: "artifact-unknown".to_string(),
            artifact_kind: "unknown".to_string(),
            display_title: Some("Unknown".to_string()),
            browser_url: None,
            document_path: None,
            evidence_quality: "thin".to_string(),
            privacy_status: None,
            openability: "frame_fallback".to_string(),
            last_seen_frame_id: Some("1".to_string()),
            last_seen_timestamp: 1_000,
        };
        let mut unknown_candidate = ScoredContinueCandidate {
            id: "candidate-unknown".to_string(),
            workstream_id: "workstream-1".to_string(),
            target_artifact: Some(unknown.clone()),
            candidate_kind: "continue_edit".to_string(),
            last_meaningful_action: None,
            evidence_frame_id: Some("1".to_string()),
            supporting_episode_id: None,
            score: 0.95,
            actionability_score: 0.92,
            primary_target_score: 1.0,
            unresolved_score: 0.8,
            branch_origin_score: 0.8,
            evidence_quality_score: 0.38,
            recency_score: 1.0,
            openability_score: 0.58,
            privacy_safety_score: 0.85,
            reason: Some("primary_artifact_fallback".to_string()),
            missing_evidence: Vec::new(),
            warnings: Vec::new(),
            resume_work_target: Some(unknown),
        };
        let workstream = ScorerWorkstream {
            id: "workstream-1".to_string(),
            state: "active".to_string(),
            title_candidate: Some("Smalltalk Continue".to_string()),
            primary_artifact_id: Some("artifact-unknown".to_string()),
            last_active_timestamp_ms: 1_000,
            confidence: 0.9,
            unresolved_signal: None,
            episodes: Vec::new(),
            artifacts: Vec::new(),
            last_meaningful_action: None,
        };

        unknown_candidate.score = confidence_cap_for_candidate(&unknown_candidate, &workstream);
        assert!(unknown_candidate.score <= 0.52);

        let smalltalk = ScorerArtifact {
            id: "artifact-smalltalk".to_string(),
            artifact_kind: "browser_tab".to_string(),
            display_title: Some("Smalltalk Continue".to_string()),
            browser_url: None,
            document_path: None,
            evidence_quality: "medium".to_string(),
            privacy_status: None,
            openability: "openable".to_string(),
            last_seen_frame_id: Some("2".to_string()),
            last_seen_timestamp: 2_000,
        };
        let mut smalltalk_candidate = unknown_candidate.clone();
        smalltalk_candidate.target_artifact = Some(smalltalk);
        smalltalk_candidate.score = 0.95;
        smalltalk_candidate.evidence_quality_score = 0.72;
        assert!(confidence_cap_for_candidate(&smalltalk_candidate, &workstream) <= 0.42);
    }

    #[test]
    fn user_facing_labels_do_not_return_raw_json_or_scorer_strings() {
        assert_eq!(
            unresolved_state_description(Some(
                r#"{"kind":"visible_error_or_failure","confidence":0.91}"#
            ))
            .as_deref(),
            Some("There appears to be an unresolved error.")
        );
        assert_eq!(
            productize_continue_label("primary_artifact_fallback").as_deref(),
            Some("This looks like the main place to continue.")
        );
        assert_eq!(
            productize_continue_label(r#"{"kind":"branch_without_return"}"#).as_deref(),
            Some("Search branch has not been applied back to the target.")
        );
    }

    fn init_evidence_schema(conn: &Connection) {
        conn.execute_batch(
            "
            CREATE TABLE frames (
              id INTEGER PRIMARY KEY,
              session_id TEXT,
              captured_at INTEGER NOT NULL,
              snapshot_path TEXT NOT NULL,
              app_name TEXT,
              window_name TEXT,
              browser_url TEXT,
              document_path TEXT,
              focused INTEGER NOT NULL DEFAULT 1,
              capture_trigger TEXT NOT NULL,
              text_source TEXT,
              accessibility_text TEXT,
              accessibility_tree_json TEXT,
              full_text TEXT,
              content_hash TEXT,
              image_hash TEXT,
              created_at INTEGER NOT NULL,
              app_bundle_id TEXT,
              privacy_status TEXT,
              previous_frame_id TEXT
            );
            CREATE TABLE app_contexts (
              id TEXT PRIMARY KEY,
              frame_id TEXT NOT NULL,
              adapter_id TEXT NOT NULL,
              object_type TEXT NOT NULL,
              primary_id TEXT,
              title TEXT,
              url TEXT,
              file_path TEXT,
              repo_path TEXT,
              selected_text TEXT,
              focused_object TEXT,
              confidence REAL
            );
            CREATE TABLE content_units (
              id TEXT PRIMARY KEY,
              frame_id TEXT NOT NULL,
              source TEXT NOT NULL,
              unit_type TEXT NOT NULL,
              semantic_role TEXT,
              text TEXT,
              text_hash TEXT,
              confidence REAL,
              created_at_ms INTEGER NOT NULL
            );
            CREATE TABLE ui_events (
              id TEXT PRIMARY KEY,
              session_id TEXT,
              ts_ms INTEGER NOT NULL,
              event_type TEXT NOT NULL,
              app_bundle_id TEXT,
              app_name TEXT,
              window_title TEXT,
              key_category TEXT,
              created_at_ms INTEGER NOT NULL
            );
            CREATE TABLE ax_nodes (
              id TEXT PRIMARY KEY,
              frame_id TEXT NOT NULL,
              focused INTEGER,
              selected_text TEXT
            );
            CREATE TABLE capture_triggers (
              id TEXT PRIMARY KEY,
              session_id TEXT,
              ts_ms INTEGER NOT NULL,
              trigger_type TEXT NOT NULL,
              caused_by_event_ids TEXT NOT NULL,
              pre_frame_id TEXT,
              post_frame_id TEXT,
              status TEXT
            );
            CREATE TABLE event_transitions (
              id TEXT PRIMARY KEY,
              session_id TEXT,
              trigger_id TEXT NOT NULL,
              primary_event_id TEXT,
              pre_frame_id TEXT,
              post_frame_id TEXT,
              ts_start_ms INTEGER NOT NULL,
              ts_end_ms INTEGER NOT NULL,
              transition_type TEXT,
              confidence REAL,
              summary TEXT
            );
            CREATE TABLE frame_diffs (
              id TEXT PRIMARY KEY,
              session_id TEXT,
              from_frame_id TEXT NOT NULL,
              to_frame_id TEXT NOT NULL,
              ts_ms INTEGER NOT NULL,
              added_text_hashes TEXT,
              removed_text_hashes TEXT,
              diff_type TEXT,
              summary TEXT
            );
            CREATE TABLE typing_bursts (
              id TEXT PRIMARY KEY,
              session_id TEXT,
              started_at_ms INTEGER NOT NULL,
              ended_at_ms INTEGER NOT NULL,
              enter_count INTEGER DEFAULT 0,
              paste_count INTEGER DEFAULT 0,
              committed INTEGER DEFAULT 0,
              commit_signal TEXT,
              pre_frame_id TEXT,
              post_frame_id TEXT
            );
            CREATE TABLE clipboard_events (
              id TEXT PRIMARY KEY,
              session_id TEXT,
              ts_ms INTEGER NOT NULL,
              change_count INTEGER NOT NULL,
              content_type TEXT NOT NULL,
              text_hash TEXT,
              redacted_preview TEXT,
              source_frame_id TEXT,
              target_frame_id TEXT
            );
            ",
        )
        .unwrap();
        ensure_continue_schema(conn).unwrap();
    }

    #[allow(clippy::too_many_arguments)]
    fn insert_frame(
        conn: &Connection,
        id: i64,
        app_name: &str,
        window_name: &str,
        browser_url: Option<&str>,
        document_path: Option<&str>,
        trigger: &str,
        full_text: &str,
        bundle_id: Option<&str>,
        previous_frame_id: Option<i64>,
    ) {
        conn.execute(
            "INSERT INTO frames (
                id, session_id, captured_at, snapshot_path, app_name, window_name,
                browser_url, document_path, capture_trigger, text_source, full_text,
                content_hash, image_hash, created_at, app_bundle_id, privacy_status,
                previous_frame_id
             ) VALUES (?1, 'session-a', ?2, '/tmp/frame.jpg', ?3, ?4, ?5, ?6,
                       ?7, 'accessibility', ?8, ?9, ?10, ?2, ?11, 'normal', ?12)",
            params![
                id,
                id * 1000,
                app_name,
                window_name,
                browser_url,
                document_path,
                trigger,
                full_text,
                stable_hash(full_text.as_bytes()),
                format!("image-{}", id),
                bundle_id,
                previous_frame_id.map(|value| value.to_string()),
            ],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO ax_nodes (id, frame_id, focused, selected_text)
             VALUES (?1, ?2, 1, NULL)",
            params![format!("ax-{}", id), id.to_string()],
        )
        .unwrap();
    }

    fn insert_context(
        conn: &Connection,
        frame_id: i64,
        object_type: &str,
        title: &str,
        url: Option<&str>,
        file_path: Option<&str>,
        selected_text: Option<&str>,
    ) {
        conn.execute(
            "INSERT INTO app_contexts (
                id, frame_id, adapter_id, object_type, primary_id, title, url,
                file_path, selected_text, focused_object, confidence
             ) VALUES (?1, ?2, 'test_adapter', ?3, ?4, ?5, ?4, ?6, ?7, NULL, 0.92)",
            params![
                format!("ctx-{}", frame_id),
                frame_id.to_string(),
                object_type,
                url.or(file_path),
                title,
                file_path,
                selected_text,
            ],
        )
        .unwrap();
    }

    fn insert_unit(conn: &Connection, frame_id: i64, role: &str, unit_type: &str, text: &str) {
        conn.execute(
            "INSERT INTO content_units (
                id, frame_id, source, unit_type, semantic_role, text, text_hash,
                confidence, created_at_ms
             ) VALUES (?1, ?2, 'test', ?3, ?4, ?5, ?6, 0.9, ?7)",
            params![
                format!("unit-{}-{}", frame_id, role),
                frame_id.to_string(),
                unit_type,
                role,
                text,
                stable_hash(text.as_bytes()),
                frame_id * 1000,
            ],
        )
        .unwrap();
    }

    fn insert_typing(conn: &Connection, frame_id: i64, enter_count: i64, paste_count: i64) {
        conn.execute(
            "INSERT INTO ui_events (
                id, session_id, ts_ms, event_type, key_category, created_at_ms
             ) VALUES (?1, 'session-a', ?2, 'key_down', ?3, ?2)",
            params![
                format!("key-{}", frame_id),
                frame_id * 1000 - 50,
                if enter_count > 0 {
                    "enter"
                } else {
                    "character"
                },
            ],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO typing_bursts (
                id, session_id, started_at_ms, ended_at_ms, enter_count,
                paste_count, committed, commit_signal, post_frame_id
             ) VALUES (?1, 'session-a', ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                format!("typing-{}", frame_id),
                frame_id * 1000 - 100,
                frame_id * 1000,
                enter_count,
                paste_count,
                if enter_count > 0 { 1 } else { 0 },
                if enter_count > 0 { Some("enter") } else { None },
                frame_id.to_string(),
            ],
        )
        .unwrap();
    }

    fn insert_diff(conn: &Connection, from_frame_id: i64, to_frame_id: i64, diff_type: &str) {
        conn.execute(
            "INSERT INTO frame_diffs (
                id, session_id, from_frame_id, to_frame_id, ts_ms, added_text_hashes,
                diff_type, summary
             ) VALUES (?1, 'session-a', ?2, ?3, ?4, '[\"new-output\"]', ?5, 'changed')",
            params![
                format!("diff-{}", to_frame_id),
                from_frame_id.to_string(),
                to_frame_id.to_string(),
                to_frame_id * 1000,
                diff_type,
            ],
        )
        .unwrap();
    }

    fn insert_clipboard(conn: &Connection, source_frame_id: i64, target_frame_id: i64) {
        conn.execute(
            "INSERT INTO clipboard_events (
                id, session_id, ts_ms, change_count, content_type, text_hash,
                redacted_preview, source_frame_id, target_frame_id
             ) VALUES (?1, 'session-a', ?2, 1, 'text', 'clip-hash', '[redacted]',
                       ?3, ?4)",
            params![
                format!("clip-{}-{}", source_frame_id, target_frame_id),
                target_frame_id * 1000 - 200,
                source_frame_id.to_string(),
                target_frame_id.to_string(),
            ],
        )
        .unwrap();
    }

    fn action_kind_for_frame(conn: &Connection, frame_id: i64) -> String {
        conn.query_row(
            "SELECT action_kind FROM continue_task_actions WHERE frame_id = ?1",
            params![frame_id.to_string()],
            |row| row.get(0),
        )
        .unwrap()
    }

    fn rebuild_second_and_third(conn: &Connection) -> ContinueThirdLayerRebuildResult {
        let second_request = ContinueSecondLayerRebuildRequest {
            session_id: Some("session-a".to_string()),
            limit: Some(100),
            ..Default::default()
        };
        rebuild_continue_second_layer(conn, second_request).unwrap();
        let third_request = ContinueThirdLayerRebuildRequest {
            session_id: Some("session-a".to_string()),
            limit: Some(100),
            ..Default::default()
        };
        rebuild_continue_third_layer(conn, third_request).unwrap()
    }

    fn distinct_episode_count_for_frames(conn: &Connection, frame_ids: &[&str]) -> i64 {
        let quoted = frame_ids
            .iter()
            .map(|frame_id| format!("'{}'", frame_id))
            .collect::<Vec<_>>()
            .join(",");
        conn.query_row(
            &format!(
                "SELECT COUNT(DISTINCT ea.episode_id)
                 FROM continue_episode_actions ea
                 JOIN continue_task_actions ta ON ta.id = ea.action_id
                 WHERE ta.frame_id IN ({})",
                quoted
            ),
            [],
            |row| row.get(0),
        )
        .unwrap()
    }

    #[test]
    fn continue_rebuild_uses_event_only_moments_without_new_frames() {
        let conn = Connection::open_in_memory().unwrap();
        init_evidence_schema(&conn);
        for (index, (ts, event_type, app, bundle, window, key_category)) in [
            (
                1_000,
                "app_switch",
                "Helium",
                "app.helium",
                "Smalltalk direction",
                None,
            ),
            (
                1_500,
                "scroll",
                "Helium",
                "app.helium",
                "Smalltalk direction",
                None,
            ),
            (
                40_000,
                "key_down",
                "Codex",
                "com.openai.codex",
                "smalltalk",
                Some("character"),
            ),
            (
                80_000,
                "click",
                "smalltalk",
                "com.smalltalk.app",
                "smalltalk",
                None,
            ),
        ]
        .into_iter()
        .enumerate()
        {
            conn.execute(
                "INSERT INTO ui_events (
                    id, session_id, ts_ms, event_type, app_bundle_id, app_name,
                    window_title, key_category, created_at_ms
                 ) VALUES (?1, 'session-events', ?2, ?3, ?4, ?5, ?6, ?7, ?2)",
                params![
                    format!("evt-event-only-{}", index),
                    ts,
                    event_type,
                    bundle,
                    app,
                    window,
                    key_category,
                ],
            )
            .unwrap();
        }

        let result = rebuild_continue_second_layer(
            &conn,
            ContinueSecondLayerRebuildRequest {
                session_id: Some("session-events".to_string()),
                limit: Some(50),
                ..Default::default()
            },
        )
        .unwrap();

        let frame_rows: i64 = conn
            .query_row("SELECT COUNT(*) FROM frames", [], |row| row.get(0))
            .unwrap();
        let event_observations: i64 = conn
            .query_row(
                "SELECT COUNT(*)
                 FROM continue_artifact_observations
                 WHERE frame_id LIKE 'event-%'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let event_actions: i64 = conn
            .query_row(
                "SELECT COUNT(*)
                 FROM continue_task_actions
                 WHERE frame_id LIKE 'event-%'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(frame_rows, 0);
        assert!(result.processed_frames > 1, "{:?}", result);
        assert_eq!(event_observations, result.observation_count);
        assert!(event_actions > 0);
    }

    #[test]
    fn second_layer_resolves_artifacts_actions_and_is_idempotent() {
        let conn = Connection::open_in_memory().unwrap();
        init_evidence_schema(&conn);

        insert_frame(
            &conn,
            1,
            "Arc",
            "Example",
            Some("https://example.com/page?x=1&utm_source=newsletter"),
            None,
            "navigation",
            "Example article body",
            Some("company.thebrowser.Browser"),
            None,
        );
        insert_context(
            &conn,
            1,
            "browser_tab",
            "Example",
            Some("https://example.com/page?x=1&utm_source=newsletter"),
            None,
            None,
        );
        insert_frame(
            &conn,
            2,
            "Arc",
            "Example",
            Some("https://example.com/page?x=1"),
            None,
            "manual",
            "Example article body again",
            Some("company.thebrowser.Browser"),
            Some(1),
        );
        insert_context(
            &conn,
            2,
            "browser_tab",
            "Example",
            Some("https://example.com/page?x=1"),
            None,
            None,
        );

        for frame_id in [3_i64, 4, 8, 11, 13] {
            insert_frame(
                &conn,
                frame_id,
                "Cursor",
                "lib.rs - smalltalk",
                None,
                Some("/Users/me/project/src/lib.rs"),
                if frame_id == 4 || frame_id == 11 {
                    "typing_pause"
                } else {
                    "manual"
                },
                "fn main() { continue_work(); }",
                Some("com.todesktop.230313mzl4w4u92"),
                Some(frame_id - 1),
            );
            insert_context(
                &conn,
                frame_id,
                "code_editor",
                "lib.rs",
                None,
                Some("/Users/me/project/src/lib.rs"),
                if frame_id == 8 {
                    Some("selected code")
                } else {
                    None
                },
            );
            insert_unit(&conn, frame_id, "code_editor", "code", "fn main() {}");
        }
        insert_typing(&conn, 4, 0, 0);
        insert_typing(&conn, 11, 0, 0);

        insert_frame(
            &conn,
            5,
            "GenericApp",
            "Untitled Surface",
            None,
            None,
            "manual",
            "thin ocr-ish text",
            None,
            Some(4),
        );

        insert_frame(
            &conn,
            6,
            "Slack",
            "Design thread",
            None,
            None,
            "typing_pause",
            "message composer",
            Some("com.tinyspeck.slackmacgap"),
            Some(5),
        );
        insert_context(&conn, 6, "messaging", "Design thread", None, None, None);
        insert_unit(&conn, 6, "composer", "input", "reply draft");
        insert_typing(&conn, 6, 0, 0);

        insert_frame(
            &conn,
            7,
            "Terminal",
            "zsh",
            None,
            None,
            "manual",
            "thread 'main' panicked with error: test failed",
            Some("com.apple.Terminal"),
            Some(6),
        );
        insert_context(&conn, 7, "terminal", "zsh", None, None, None);
        insert_unit(&conn, 7, "error", "terminal_output", "error: test failed");

        insert_frame(
            &conn,
            9,
            "Notes",
            "Implementation plan",
            None,
            Some("/Users/me/project/PLAN.md"),
            "typing_pause",
            "Implementation plan",
            Some("com.apple.Notes"),
            Some(8),
        );
        insert_context(
            &conn,
            9,
            "notes_doc",
            "Implementation plan",
            None,
            Some("/Users/me/project/PLAN.md"),
            None,
        );
        insert_clipboard(&conn, 8, 9);

        insert_frame(
            &conn,
            10,
            "Terminal",
            "zsh",
            None,
            None,
            "manual",
            "cargo check finished successfully",
            Some("com.apple.Terminal"),
            Some(9),
        );
        insert_context(&conn, 10, "terminal", "zsh", None, None, None);
        insert_unit(
            &conn,
            10,
            "terminal_output",
            "terminal_output",
            "Finished dev profile",
        );
        insert_diff(&conn, 7, 10, "text_change");

        insert_frame(
            &conn,
            12,
            "Arc",
            "Search results",
            Some("https://www.google.com/search?q=rust+sqlite+upsert"),
            None,
            "navigation",
            "Search results for rust sqlite upsert",
            Some("company.thebrowser.Browser"),
            Some(11),
        );
        insert_context(
            &conn,
            12,
            "browser_tab",
            "Search results",
            Some("https://www.google.com/search?q=rust+sqlite+upsert"),
            None,
            None,
        );
        insert_unit(&conn, 12, "search_result", "result", "rusqlite upsert");

        let request = ContinueSecondLayerRebuildRequest {
            session_id: Some("session-a".to_string()),
            limit: Some(50),
            ..Default::default()
        };
        let first = rebuild_continue_second_layer(&conn, request.clone()).unwrap();
        let second = rebuild_continue_second_layer(&conn, request).unwrap();

        assert_eq!(first.processed_frames, 13);
        assert_eq!(second.observation_count, first.observation_count);
        assert_eq!(second.task_action_count, first.task_action_count);
        assert_eq!(second.observation_count, 13);
        assert_eq!(second.task_action_count, 13);

        let url_observations: i64 = conn
            .query_row(
                "SELECT COUNT(*)
                 FROM continue_artifact_observations o
                 JOIN continue_artifacts a ON a.id = o.artifact_id
                 WHERE a.stable_key = 'url:https://example.com/page?x=1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(url_observations, 2);

        let document_observations: i64 = conn
            .query_row(
                "SELECT COUNT(*)
                 FROM continue_artifact_observations o
                 JOIN continue_artifacts a ON a.id = o.artifact_id
                 WHERE a.stable_key = 'document:/Users/me/project/src/lib.rs'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(document_observations, 5);

        let thin_unknown: (String, String) = conn
            .query_row(
                "SELECT artifact_kind, evidence_quality
                 FROM continue_artifacts
                 WHERE stable_key LIKE 'window:%untitled_surface%'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(thin_unknown, ("unknown".to_string(), "thin".to_string()));

        assert_eq!(action_kind_for_frame(&conn, 4), "editing");
        assert_eq!(action_kind_for_frame(&conn, 6), "messaging_interrupt");
        assert_eq!(action_kind_for_frame(&conn, 7), "encountering_error");
        assert_eq!(action_kind_for_frame(&conn, 9), "copying_evidence");
        assert_eq!(action_kind_for_frame(&conn, 10), "observing_command_output");
        assert_eq!(action_kind_for_frame(&conn, 12), "branching_away");
        assert_eq!(action_kind_for_frame(&conn, 13), "returning_to_origin");

        let clipboard_events_json: String = conn
            .query_row(
                "SELECT evidence_event_ids_json
                 FROM continue_task_actions
                 WHERE frame_id = '9'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(clipboard_events_json.contains("clip-8-9"));
        assert!(!clipboard_events_json.contains("selected code"));

        let editing_events_json: String = conn
            .query_row(
                "SELECT evidence_event_ids_json
                 FROM continue_task_actions
                 WHERE frame_id = '4'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(editing_events_json.contains("key-4"));
    }

    #[test]
    fn third_layer_groups_same_artifact_and_closes_on_idle() {
        let conn = Connection::open_in_memory().unwrap();
        init_evidence_schema(&conn);

        insert_frame(
            &conn,
            1,
            "Cursor",
            "lib.rs - smalltalk",
            None,
            Some("/Users/me/project/src/lib.rs"),
            "manual",
            "fn continue_work() {}",
            Some("com.todesktop.230313mzl4w4u92"),
            None,
        );
        insert_context(
            &conn,
            1,
            "code_editor",
            "lib.rs",
            None,
            Some("/Users/me/project/src/lib.rs"),
            None,
        );
        insert_frame(
            &conn,
            2,
            "Cursor",
            "lib.rs - smalltalk",
            None,
            Some("/Users/me/project/src/lib.rs"),
            "typing_pause",
            "fn continue_work() { changed(); }",
            Some("com.todesktop.230313mzl4w4u92"),
            Some(1),
        );
        insert_context(
            &conn,
            2,
            "code_editor",
            "lib.rs",
            None,
            Some("/Users/me/project/src/lib.rs"),
            None,
        );
        insert_typing(&conn, 2, 0, 0);
        insert_frame(
            &conn,
            3,
            "Cursor",
            "lib.rs - smalltalk",
            None,
            Some("/Users/me/project/src/lib.rs"),
            "idle_timeout",
            "fn continue_work() { changed(); }",
            Some("com.todesktop.230313mzl4w4u92"),
            Some(2),
        );
        insert_context(
            &conn,
            3,
            "code_editor",
            "lib.rs",
            None,
            Some("/Users/me/project/src/lib.rs"),
            None,
        );

        let result = rebuild_second_and_third(&conn);

        assert_eq!(result.processed_actions, 3);
        assert_eq!(
            distinct_episode_count_for_frames(&conn, &["1", "2", "3"]),
            1
        );
        assert_eq!(action_kind_for_frame(&conn, 3), "idle_after_progress");
        let boundary_end_reason: String = conn
            .query_row(
                "SELECT boundary_end_reason FROM continue_episodes LIMIT 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(boundary_end_reason, "idle_after_progress");
        let state_and_signal: (String, String) = conn
            .query_row(
                "SELECT state, unresolved_signal FROM continue_workstreams LIMIT 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(state_and_signal.0, "suspended");
        assert!(state_and_signal.1.contains("idle_after_progress"));
    }

    #[test]
    fn third_layer_clusters_branch_return_terminal_and_keeps_interruptions_separate() {
        let conn = Connection::open_in_memory().unwrap();
        init_evidence_schema(&conn);

        for frame_id in [1_i64, 2, 4] {
            insert_frame(
                &conn,
                frame_id,
                "Cursor",
                "lib.rs - smalltalk",
                None,
                Some("/Users/me/project/src/lib.rs"),
                if frame_id == 1 {
                    "typing_pause"
                } else {
                    "manual"
                },
                if frame_id == 2 {
                    "error: unresolved import"
                } else {
                    "fn continue_work() { changed(); }"
                },
                Some("com.todesktop.230313mzl4w4u92"),
                frame_id.checked_sub(1),
            );
            insert_context(
                &conn,
                frame_id,
                "code_editor",
                "lib.rs",
                None,
                Some("/Users/me/project/src/lib.rs"),
                None,
            );
        }
        insert_typing(&conn, 1, 0, 0);
        insert_unit(&conn, 2, "error", "code", "error: unresolved import");

        insert_frame(
            &conn,
            3,
            "Arc",
            "Search results",
            Some("https://www.google.com/search?q=rust+unresolved+import"),
            None,
            "navigation",
            "Search results for rust unresolved import",
            Some("company.thebrowser.Browser"),
            Some(2),
        );
        insert_context(
            &conn,
            3,
            "browser_tab",
            "Search results",
            Some("https://www.google.com/search?q=rust+unresolved+import"),
            None,
            None,
        );
        insert_unit(
            &conn,
            3,
            "search_result",
            "result",
            "rust unresolved import",
        );

        insert_frame(
            &conn,
            5,
            "Terminal",
            "zsh",
            None,
            None,
            "manual",
            "cargo check finished successfully",
            Some("com.apple.Terminal"),
            Some(4),
        );
        insert_context(&conn, 5, "terminal", "zsh", None, None, None);
        insert_unit(
            &conn,
            5,
            "terminal_output",
            "terminal_output",
            "Finished dev profile",
        );
        insert_diff(&conn, 4, 5, "text_change");

        insert_frame(
            &conn,
            6,
            "Slack",
            "Lunch thread",
            None,
            None,
            "typing_pause",
            "lunch?",
            Some("com.tinyspeck.slackmacgap"),
            Some(5),
        );
        insert_context(&conn, 6, "messaging", "Lunch thread", None, None, None);
        insert_unit(&conn, 6, "composer", "input", "lunch?");
        insert_typing(&conn, 6, 0, 0);

        insert_frame(
            &conn,
            7,
            "Arc",
            "Weather",
            Some("https://example.com/weather"),
            None,
            "navigation",
            "weather",
            Some("company.thebrowser.Browser"),
            Some(6),
        );
        insert_context(
            &conn,
            7,
            "browser_tab",
            "Weather",
            Some("https://example.com/weather"),
            None,
            None,
        );
        insert_frame(
            &conn,
            8,
            "Arc",
            "News",
            Some("https://example.net/news"),
            None,
            "navigation",
            "news",
            Some("company.thebrowser.Browser"),
            Some(7),
        );
        insert_context(
            &conn,
            8,
            "browser_tab",
            "News",
            Some("https://example.net/news"),
            None,
            None,
        );
        insert_frame(
            &conn,
            9,
            "GenericApp",
            "Unknown",
            None,
            None,
            "manual",
            "",
            None,
            Some(8),
        );

        let first = rebuild_second_and_third(&conn);
        let second = rebuild_continue_third_layer(
            &conn,
            ContinueThirdLayerRebuildRequest {
                session_id: Some("session-a".to_string()),
                limit: Some(100),
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(first.episode_count, second.episode_count);
        assert_eq!(first.workstream_count, second.workstream_count);
        assert_eq!(action_kind_for_frame(&conn, 3), "branching_away");
        assert_eq!(action_kind_for_frame(&conn, 4), "returning_to_origin");
        assert_eq!(action_kind_for_frame(&conn, 5), "observing_command_output");
        assert_eq!(action_kind_for_frame(&conn, 6), "messaging_interrupt");

        let code_workstream_episode_count: i64 = conn
            .query_row(
                "SELECT COUNT(*)
                 FROM continue_workstream_episodes we
                 JOIN continue_episodes e ON e.id = we.episode_id
                 JOIN continue_workstreams w ON w.id = we.workstream_id
                 JOIN continue_artifacts a ON a.id = w.primary_artifact_id
                 WHERE a.stable_key = 'document:/Users/me/project/src/lib.rs'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(code_workstream_episode_count >= 4);

        let code_primary_roles: i64 = conn
            .query_row(
                "SELECT COUNT(*)
                 FROM continue_workstream_artifacts wa
                 JOIN continue_artifacts a ON a.id = wa.artifact_id
                 WHERE a.stable_key = 'document:/Users/me/project/src/lib.rs'
                   AND wa.durable_role = 'primary_target'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(code_primary_roles, 1);

        let branch_roles: i64 = conn
            .query_row(
                "SELECT COUNT(*)
                 FROM continue_workstream_artifacts wa
                 JOIN continue_artifacts a ON a.id = wa.artifact_id
                 WHERE a.stable_key LIKE 'url:https://www.google.com/search%'
                   AND wa.durable_role = 'branch'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(branch_roles, 1);

        let verification_roles: i64 = conn
            .query_row(
                "SELECT COUNT(*)
                 FROM continue_workstream_artifacts
                 WHERE durable_role = 'verification_surface'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(verification_roles, 1);

        let messaging_workstreams: i64 = conn
            .query_row(
                "SELECT COUNT(DISTINCT we.workstream_id)
                 FROM continue_workstream_episodes we
                 JOIN continue_episode_actions ea ON ea.episode_id = we.episode_id
                 JOIN continue_task_actions ta ON ta.id = ea.action_id
                 WHERE ta.frame_id = '6'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(messaging_workstreams, 1);

        let unrelated_browser_workstreams: i64 = conn
            .query_row(
                "SELECT COUNT(DISTINCT we.workstream_id)
                 FROM continue_workstream_episodes we
                 JOIN continue_episode_actions ea ON ea.episode_id = we.episode_id
                 JOIN continue_task_actions ta ON ta.id = ea.action_id
                 WHERE ta.frame_id IN ('7', '8')",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(unrelated_browser_workstreams, 2);

        let thin_episode_quality: String = conn
            .query_row(
                "SELECT e.evidence_quality
                 FROM continue_episodes e
                 JOIN continue_episode_actions ea ON ea.episode_id = e.id
                 JOIN continue_task_actions ta ON ta.id = ea.action_id
                 WHERE ta.frame_id = '9'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(thin_episode_quality, "thin");
    }

    #[test]
    fn continue_decision_treats_search_branch_as_evidence_not_return_target() {
        let conn = Connection::open_in_memory().unwrap();
        init_evidence_schema(&conn);

        insert_frame(
            &conn,
            1,
            "Cursor",
            "lib.rs - smalltalk",
            None,
            Some("/Users/me/project/src/lib.rs"),
            "typing_pause",
            "fn continue_work() { changed(); }",
            Some("com.todesktop.230313mzl4w4u92"),
            None,
        );
        insert_context(
            &conn,
            1,
            "code_editor",
            "lib.rs",
            None,
            Some("/Users/me/project/src/lib.rs"),
            None,
        );
        insert_typing(&conn, 1, 0, 0);

        insert_frame(
            &conn,
            2,
            "Arc",
            "Search results",
            Some("https://www.google.com/search?q=rusqlite+alter+table"),
            None,
            "navigation",
            "Search results for rusqlite alter table",
            Some("company.thebrowser.Browser"),
            Some(1),
        );
        insert_context(
            &conn,
            2,
            "browser_tab",
            "Search results",
            Some("https://www.google.com/search?q=rusqlite+alter+table"),
            None,
            None,
        );
        insert_unit(&conn, 2, "search_result", "result", "rusqlite alter table");

        let decision = get_continue_decision(
            &conn,
            ContinueDecisionRequest {
                session_id: Some("session-a".to_string()),
                lookback_ms: None,
                limit: Some(50),
                rebuild_layers: Some(true),
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(decision.source, "local_scorer");
        assert_eq!(
            decision
                .return_target
                .as_ref()
                .and_then(|target| target.document_path.as_deref()),
            Some("/Users/me/project/src/lib.rs")
        );
        assert_ne!(
            decision
                .return_target
                .as_ref()
                .and_then(|target| target.browser_url.as_deref()),
            Some("https://www.google.com/search?q=rusqlite+alter+table")
        );
        assert!(decision
            .warnings
            .iter()
            .any(|warning| warning.contains("current_focus")));
        assert!(decision.generated_candidates >= 1);

        let persisted: i64 = conn
            .query_row("SELECT COUNT(*) FROM continue_decisions", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(persisted, 1);
    }

    #[test]
    fn normal_continue_mode_reuses_cached_decision_without_growth() {
        let conn = Connection::open_in_memory().unwrap();
        init_evidence_schema(&conn);

        insert_frame(
            &conn,
            1,
            "Cursor",
            "lib.rs - smalltalk",
            None,
            Some("/Users/me/project/src/lib.rs"),
            "typing_pause",
            "fn continue_work() { changed(); }",
            Some("com.todesktop.230313mzl4w4u92"),
            None,
        );
        insert_context(
            &conn,
            1,
            "code_editor",
            "lib.rs",
            None,
            Some("/Users/me/project/src/lib.rs"),
            None,
        );
        insert_typing(&conn, 1, 0, 0);

        let rebuild = get_continue_decision(
            &conn,
            ContinueDecisionRequest {
                session_id: Some("session-a".to_string()),
                mode: Some("rebuild".to_string()),
                rebuild_layers: Some(false),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(rebuild.mode, "rebuild");
        assert!(!rebuild.cache_hit);

        let normal = get_continue_decision(
            &conn,
            ContinueDecisionRequest {
                session_id: Some("session-a".to_string()),
                mode: Some("normal".to_string()),
                rebuild_layers: Some(false),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(normal.mode, "normal");
        assert!(normal.cache_hit);
        assert_eq!(normal.decision_id, rebuild.decision_id);

        let persisted: i64 = conn
            .query_row("SELECT COUNT(*) FROM continue_decisions", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(persisted, 1);
    }

    #[test]
    fn continue_feedback_dedupes_repeated_explicit_feedback() {
        let conn = Connection::open_in_memory().unwrap();
        init_evidence_schema(&conn);

        insert_frame(
            &conn,
            1,
            "Cursor",
            "lib.rs - smalltalk",
            None,
            Some("/Users/me/project/src/lib.rs"),
            "manual",
            "fn continue_work() { changed(); }",
            Some("com.todesktop.230313mzl4w4u92"),
            None,
        );
        insert_context(
            &conn,
            1,
            "code_editor",
            "lib.rs",
            None,
            Some("/Users/me/project/src/lib.rs"),
            None,
        );
        insert_typing(&conn, 1, 0, 0);
        let decision = get_continue_decision(
            &conn,
            ContinueDecisionRequest {
                session_id: Some("session-a".to_string()),
                rebuild_layers: Some(true),
                ..Default::default()
            },
        )
        .unwrap();

        let request = ContinueExplicitFeedbackRequest {
            decision_id: Some(decision.decision_id.clone()),
            selected_candidate_id: None,
            workstream_id: decision
                .selected_workstream
                .as_ref()
                .map(|workstream| workstream.workstream_id.clone()),
            target_artifact_id: decision
                .return_target
                .as_ref()
                .and_then(|target| target.artifact_id.clone()),
            corrected_artifact_id: None,
            feedback_kind: "accepted".to_string(),
            note: Some("works".to_string()),
            source: Some("test".to_string()),
        };
        let first = record_continue_feedback(&conn, request.clone()).unwrap();
        let second = record_continue_feedback(&conn, request).unwrap();
        assert_eq!(first.id, second.id);

        let persisted: i64 = conn
            .query_row("SELECT COUNT(*) FROM continue_feedback_events", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(persisted, 1);
    }

    #[test]
    fn continue_decision_selects_error_resolution_from_unresolved_workstream() {
        let conn = Connection::open_in_memory().unwrap();
        init_evidence_schema(&conn);

        insert_frame(
            &conn,
            1,
            "Cursor",
            "lib.rs - smalltalk",
            None,
            Some("/Users/me/project/src/lib.rs"),
            "manual",
            "error: unresolved import",
            Some("com.todesktop.230313mzl4w4u92"),
            None,
        );
        insert_context(
            &conn,
            1,
            "code_editor",
            "lib.rs",
            None,
            Some("/Users/me/project/src/lib.rs"),
            None,
        );
        insert_unit(&conn, 1, "error", "code", "error: unresolved import");

        insert_frame(
            &conn,
            2,
            "Arc",
            "Search results",
            Some("https://www.google.com/search?q=rust+unresolved+import"),
            None,
            "navigation",
            "Search results for rust unresolved import",
            Some("company.thebrowser.Browser"),
            Some(1),
        );
        insert_context(
            &conn,
            2,
            "browser_tab",
            "Search results",
            Some("https://www.google.com/search?q=rust+unresolved+import"),
            None,
            None,
        );
        insert_unit(
            &conn,
            2,
            "search_result",
            "result",
            "rust unresolved import",
        );

        let decision = get_continue_decision(
            &conn,
            ContinueDecisionRequest {
                session_id: Some("session-a".to_string()),
                lookback_ms: None,
                limit: Some(50),
                rebuild_layers: Some(true),
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(decision.candidate_kind.as_deref(), Some("resolve_error"));
        assert_eq!(
            decision
                .return_target
                .as_ref()
                .and_then(|target| target.document_path.as_deref()),
            Some("/Users/me/project/src/lib.rs")
        );
        assert!(decision
            .unresolved_state
            .as_deref()
            .unwrap_or("")
            .contains("error"));
        assert!(decision.confidence >= 0.6);
    }

    #[test]
    fn continue_decision_uses_evidence_only_for_thin_unknown_surface() {
        let conn = Connection::open_in_memory().unwrap();
        init_evidence_schema(&conn);

        insert_frame(
            &conn,
            1,
            "GenericApp",
            "Unknown",
            None,
            None,
            "manual",
            "",
            None,
            None,
        );

        let decision = get_continue_decision(
            &conn,
            ContinueDecisionRequest {
                session_id: Some("session-a".to_string()),
                lookback_ms: None,
                limit: Some(20),
                rebuild_layers: Some(true),
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(decision.candidate_kind.as_deref(), Some("evidence_only"));
        assert_eq!(decision.validation_status, "fallback");
        assert!(decision
            .missing_evidence
            .iter()
            .any(|item| item == "no_last_meaningful_action"));
        assert!(decision
            .warnings
            .iter()
            .any(|warning| warning == "thin_evidence"));
    }

    #[test]
    fn continue_eval_default_fixture_reports_required_metrics() {
        let report = run_continue_eval(None).unwrap();

        assert_eq!(report.schema, "smalltalk.continue_eval.v1");
        assert_eq!(report.case_count, 21);
        assert_eq!(report.target_artifact_correct, 21);
        assert_eq!(report.recall_at_k, 1.0);
        assert_eq!(report.mrr, 1.0);
        assert_eq!(report.hallucinated_artifact_count, 0);
        assert_eq!(report.model_validation_fallback_rate, 0.0);
        assert!(report
            .cases
            .iter()
            .any(|case| case.scenario == "AI Chat as support"));
        assert!(report
            .cases
            .iter()
            .any(|case| case.scenario == "Nested dependent task"));
    }

    #[test]
    fn continue_eval_rejects_hallucinated_model_candidate() {
        let fixture = ContinueEvalFixture {
            k: Some(2),
            cases: vec![ContinueEvalCaseFixture {
                name: "bad_model".to_string(),
                scenario: "Thin OCR-only evidence".to_string(),
                expected_target_artifact_id: "known".to_string(),
                current_focus_artifact_id: Some("unknown".to_string()),
                candidates: vec![eval_candidate(
                    "known-candidate",
                    "w",
                    Some("known"),
                    0.72,
                    "strong",
                )],
                model_output: Some(ContinueMicroInferenceOutput {
                    selected_candidate_id: "invented-candidate".to_string(),
                    selected_workstream_id: "w".to_string(),
                    intent_label: "Invented".to_string(),
                    next_action: Some("Open https://invented.example/path".to_string()),
                    reason: "Unsupported invented target".to_string(),
                    confidence: "high".to_string(),
                    uncertainty_notes: None,
                }),
            }],
        };

        let report = summarize_continue_eval_fixture(fixture).unwrap();

        assert_eq!(report.hallucinated_artifact_count, 1);
        assert_eq!(report.model_validation_fallback_rate, 1.0);
        assert!(report.cases[0]
            .validation_failures
            .contains(&"selected_candidate_id_not_in_fixture".to_string()));
    }
}
