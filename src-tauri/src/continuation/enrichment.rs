use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

const ENRICHMENT_POLICY_VERSION: &str = "weak_surface_enrichment_scheduler.v1";
const MAX_ATTEMPTS_PER_SURFACE_PER_30S: i64 = 4;
const MAX_ATTEMPTS_PER_SURFACE_PER_5M: i64 = 12;
const RECENT_STRONG_SNAPSHOT_WINDOW_MS: i64 = 30_000;
const RECENT_ENRICHMENT_WINDOW_MS: i64 = 15_000;
const RETRY_DELAYS_MS: [i64; 4] = [0, 400, 1_400, 3_000];
const SURFACE_SNAPSHOT_ADAPTER_VERSION: &str = "surface_snapshot_store.v1";
const MAX_VISIBLE_TEXT_SAMPLE_CHARS: usize = 800;
const MAX_FOCUSED_CONTROL_LABEL_CHARS: usize = 160;
const MAX_THREAD_TITLE_CHARS: usize = 160;
const MAX_WINDOW_TITLE_CHARS: usize = 240;
const MAX_RELATIVE_FILE_CHARS: usize = 240;
const MAX_JSON_FIELD_CHARS: usize = 4_096;
const SURFACE_SNAPSHOT_COLUMNS: &str = "id, session_id, surface_key, domain, adapter_key, adapter_version, observed_at_ms, frame_id, event_ids_json, artifact_id, app_name, bundle_id, app_pid, window_id, window_title, window_title_hash, workspace_path, workspace_path_hash, repo_root_path, repo_root_hash, git_branch, git_worktree_path, git_worktree_hash, thread_title, thread_key_hash, active_file_path, active_file_path_hash, active_relative_file, focused_control_role, focused_control_label, selected_text_hash, focused_text_hash, visible_text_sample, visible_text_hash, activity_state, task_state, command_state, error_markers_json, activity_signals_json, identity_confidence, evidence_quality, openability, privacy_status, missing_fields_json, evidence_sources_json, redaction_notes_json, created_at_ms, updated_at_ms";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WeakSurfaceDomain {
    CodexCli,
    CodexDesktopApp,
    CodexIdeExtension,
    CodeEditor,
    Terminal,
    NativeAgentWindow,
    UnknownWeakSurface,
    NotWeakSurface,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EnrichmentNeed {
    None,
    Light,
    Targeted,
    Retry,
    BlockedByPrivacy,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WeakSurfaceClassification {
    pub domain: WeakSurfaceDomain,
    pub enrichment_need: EnrichmentNeed,
    pub confidence: f64,
    pub reasons: Vec<String>,
    pub adapter_key: Option<String>,
    pub privacy_tier: String,
    pub observed_app_name: Option<String>,
    pub observed_bundle_id: Option<String>,
    pub observed_window_title: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct WeakSurfaceClassificationInput {
    pub app_name: Option<String>,
    pub bundle_id: Option<String>,
    pub window_title: Option<String>,
    pub browser_url: Option<String>,
    pub document_path: Option<String>,
    pub text_source: Option<String>,
    pub full_text_sample: Option<String>,
    pub content_unit_roles: Vec<String>,
    pub focused_node_role: Option<String>,
    pub event_types: Vec<String>,
    pub trigger_type: Option<String>,
    pub privacy_status: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EnrichmentAttemptStatus {
    Scheduled,
    Running,
    SucceededStrong,
    SucceededMedium,
    SucceededThin,
    SkippedRecentStrongSnapshot,
    SkippedPrivacy,
    SkippedBudget,
    SkippedSelfCapture,
    SkippedFocusChanged,
    FailedHelper,
    FailedNoTextOrIdentity,
}

impl EnrichmentAttemptStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Scheduled => "scheduled",
            Self::Running => "running",
            Self::SucceededStrong => "succeeded_strong",
            Self::SucceededMedium => "succeeded_medium",
            Self::SucceededThin => "succeeded_thin",
            Self::SkippedRecentStrongSnapshot => "skipped_recent_strong_snapshot",
            Self::SkippedPrivacy => "skipped_privacy",
            Self::SkippedBudget => "skipped_budget",
            Self::SkippedSelfCapture => "skipped_self_capture",
            Self::SkippedFocusChanged => "skipped_focus_changed",
            Self::FailedHelper => "failed_helper",
            Self::FailedNoTextOrIdentity => "failed_no_text_or_identity",
        }
    }

    fn counter_suffix(&self) -> &'static str {
        match self {
            Self::SucceededStrong => "success_strong",
            Self::SucceededMedium => "success_medium",
            Self::SucceededThin => "success_thin",
            Self::SkippedPrivacy => "skipped_privacy",
            Self::SkippedBudget => "skipped_budget",
            Self::FailedHelper | Self::FailedNoTextOrIdentity => "failed",
            _ => "attempts",
        }
    }

    fn produced_snapshot(&self) -> bool {
        matches!(
            self,
            Self::SucceededStrong | Self::SucceededMedium | Self::SucceededThin
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeakSurfaceEnrichmentAttempt {
    pub attempt_id: String,
    pub observed_at_ms: i64,
    pub scheduled_at_ms: i64,
    pub completed_at_ms: Option<i64>,
    pub surface_key: String,
    pub weak_domain: WeakSurfaceDomain,
    pub app_name: Option<String>,
    pub bundle_id: Option<String>,
    pub window_title_hash: Option<String>,
    pub window_title_capped: Option<String>,
    pub window_id: Option<i64>,
    pub trigger_event_ids: Vec<String>,
    pub trigger_type: String,
    pub attempt_index: i64,
    pub status: EnrichmentAttemptStatus,
    pub reason: Option<String>,
    pub snapshot_id: Option<String>,
    pub missing_fields: Vec<String>,
    pub adapter_key: Option<String>,
}

impl WeakSurfaceEnrichmentAttempt {
    pub fn produced_snapshot(&self) -> bool {
        self.status.produced_snapshot()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurfaceEnrichmentAttempt {
    pub id: String,
    pub session_id: Option<String>,
    pub surface_key: String,
    pub domain: String,
    pub adapter_key: Option<String>,
    pub trigger_type: Option<String>,
    pub trigger_event_ids_json: String,
    pub frame_id: Option<i64>,
    pub observed_at_ms: i64,
    pub scheduled_at_ms: Option<i64>,
    pub completed_at_ms: Option<i64>,
    pub attempt_index: i64,
    pub status: String,
    pub reason: Option<String>,
    pub missing_fields_json: String,
    pub snapshot_id: Option<String>,
    pub app_name: Option<String>,
    pub bundle_id: Option<String>,
    pub window_title: Option<String>,
    pub window_title_hash: Option<String>,
    pub window_id: Option<i64>,
    pub privacy_status: Option<String>,
    pub created_at_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurfaceSnapshot {
    pub id: String,
    pub session_id: Option<String>,
    pub surface_key: String,
    pub domain: String,
    pub adapter_key: Option<String>,
    pub adapter_version: String,
    pub observed_at_ms: i64,
    pub frame_id: Option<i64>,
    pub event_ids_json: Option<String>,
    pub artifact_id: Option<String>,
    pub app_name: Option<String>,
    pub bundle_id: Option<String>,
    pub app_pid: Option<i64>,
    pub window_id: Option<i64>,
    pub window_title: Option<String>,
    pub window_title_hash: Option<String>,
    pub workspace_path: Option<String>,
    pub workspace_path_hash: Option<String>,
    pub repo_root_path: Option<String>,
    pub repo_root_hash: Option<String>,
    pub git_branch: Option<String>,
    pub git_worktree_path: Option<String>,
    pub git_worktree_hash: Option<String>,
    pub thread_title: Option<String>,
    pub thread_key_hash: Option<String>,
    pub active_file_path: Option<String>,
    pub active_file_path_hash: Option<String>,
    pub active_relative_file: Option<String>,
    pub focused_control_role: Option<String>,
    pub focused_control_label: Option<String>,
    pub selected_text_hash: Option<String>,
    pub focused_text_hash: Option<String>,
    pub visible_text_sample: Option<String>,
    pub visible_text_hash: Option<String>,
    pub activity_state: Option<String>,
    pub task_state: Option<String>,
    pub command_state: Option<String>,
    pub error_markers_json: Option<String>,
    pub activity_signals_json: Option<String>,
    pub identity_confidence: String,
    pub evidence_quality: String,
    pub openability: String,
    pub privacy_status: Option<String>,
    pub missing_fields_json: Option<String>,
    pub evidence_sources_json: String,
    pub redaction_notes_json: Option<String>,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceQuality {
    Strong,
    Medium,
    Thin,
    Unknown,
}

impl EvidenceQuality {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Strong => "strong",
            Self::Medium => "medium",
            Self::Thin => "thin",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IdentityConfidence {
    Strong,
    Medium,
    Thin,
    Unknown,
}

impl IdentityConfidence {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Strong => "strong",
            Self::Medium => "medium",
            Self::Thin => "thin",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Openability {
    Openable,
    FrameFallback,
    AppFocusOnly,
    Blocked,
    Unknown,
}

impl Openability {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Openable => "openable",
            Self::FrameFallback => "frame_fallback",
            Self::AppFocusOnly => "app_focus_only",
            Self::Blocked => "blocked",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StaleSuppressionStrength {
    None,
    Weak,
    Medium,
    Strong,
}

impl StaleSuppressionStrength {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Weak => "weak",
            Self::Medium => "medium",
            Self::Strong => "strong",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurfaceSnapshotQuality {
    pub evidence_quality: EvidenceQuality,
    pub identity_confidence: IdentityConfidence,
    pub candidate_eligible: bool,
    pub stale_target_suppression_strength: StaleSuppressionStrength,
    pub openability: Openability,
    pub confidence_delta: f64,
    pub missing_evidence: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct WeakSurfaceIdentityInput {
    pub domain: WeakSurfaceDomain,
    pub app_name: Option<String>,
    pub bundle_id: Option<String>,
    pub window_id: Option<i64>,
    pub window_title: Option<String>,
    pub workspace_path: Option<String>,
    pub workspace_path_hash: Option<String>,
    pub repo_root_path: Option<String>,
    pub repo_root_hash: Option<String>,
    pub git_worktree_path: Option<String>,
    pub git_worktree_hash: Option<String>,
    pub git_branch: Option<String>,
    pub thread_title: Option<String>,
    pub thread_key_hash: Option<String>,
    pub active_file_path: Option<String>,
    pub active_file_path_hash: Option<String>,
    pub active_relative_file: Option<String>,
    pub command_signature_hash: Option<String>,
    pub observed_at_ms: Option<i64>,
}

impl Default for WeakSurfaceIdentityInput {
    fn default() -> Self {
        Self {
            domain: WeakSurfaceDomain::NotWeakSurface,
            app_name: None,
            bundle_id: None,
            window_id: None,
            window_title: None,
            workspace_path: None,
            workspace_path_hash: None,
            repo_root_path: None,
            repo_root_hash: None,
            git_worktree_path: None,
            git_worktree_hash: None,
            git_branch: None,
            thread_title: None,
            thread_key_hash: None,
            active_file_path: None,
            active_file_path_hash: None,
            active_relative_file: None,
            command_signature_hash: None,
            observed_at_ms: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WeakSurfaceIdentity {
    pub artifact_kind: String,
    pub stable_key: String,
    pub display_title: String,
    pub identity_confidence: String,
    pub openability: String,
    pub missing_fields: Vec<String>,
    pub merge_keys: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WeakSurfaceEnrichmentDiagnostics {
    pub weak_surface_enrichment_attempts: i64,
    pub weak_surface_enrichment_success_strong: i64,
    pub weak_surface_enrichment_success_medium: i64,
    pub weak_surface_enrichment_success_thin: i64,
    pub weak_surface_enrichment_skipped_privacy: i64,
    pub weak_surface_enrichment_skipped_budget: i64,
    pub weak_surface_enrichment_failed: i64,
    pub latest_weak_surface_attempt: Option<WeakSurfaceEnrichmentAttempt>,
    pub latest_weak_surface_snapshot_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct WeakSurfaceEnrichmentEventInput {
    pub session_id: Option<String>,
    pub observed_at_ms: i64,
    pub now_ms: i64,
    pub trigger_type: String,
    pub trigger_event_ids: Vec<String>,
    pub app_name: Option<String>,
    pub bundle_id: Option<String>,
    pub window_title: Option<String>,
    pub window_id: Option<i64>,
    pub key_category: Option<String>,
    pub privacy_status: Option<String>,
    pub force_attempt: bool,
}

pub trait SurfaceEnrichmentAdapter {
    fn key(&self) -> &'static str;
    fn domain(&self) -> WeakSurfaceDomain;
    fn enrich(&self, input: &SurfaceEnrichmentInput) -> SurfaceEnrichmentOutput;
}

#[derive(Debug, Clone)]
pub struct SurfaceEnrichmentInput {
    pub classification: WeakSurfaceClassification,
    pub observed_at_ms: i64,
    pub session_id: Option<String>,
    pub frame: Option<CaptureFrameLite>,
    pub recent_events: Vec<UiEventLite>,
    pub content_units: Vec<ContentUnitLite>,
    pub ax_nodes: Vec<AxNodeLite>,
    pub ocr_spans: Vec<OcrSpanLite>,
    pub app_contexts: Vec<AppContextLite>,
    pub window_snapshot: Option<WindowSnapshotLite>,
    pub typing_bursts: Vec<TypingBurstLite>,
    pub clipboard_metadata: Vec<ClipboardMetadataLite>,
}

#[derive(Debug, Clone, Default)]
pub struct CaptureFrameLite {
    pub id: Option<i64>,
    pub app_name: Option<String>,
    pub bundle_id: Option<String>,
    pub app_pid: Option<i64>,
    pub window_id: Option<i64>,
    pub window_title: Option<String>,
    pub browser_url: Option<String>,
    pub document_path: Option<String>,
    pub full_text: Option<String>,
    pub text_source: Option<String>,
    pub capture_trigger: Option<String>,
    pub privacy_status: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct UiEventLite {
    pub id: String,
    pub event_type: String,
    pub key_category: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct ContentUnitLite {
    pub id: String,
    pub source: String,
    pub unit_type: String,
    pub semantic_role: Option<String>,
    pub text: Option<String>,
    pub confidence: Option<f64>,
}

#[derive(Debug, Clone, Default)]
pub struct AxNodeLite {
    pub id: String,
    pub role: Option<String>,
    pub text: Option<String>,
    pub selected_text: Option<String>,
    pub focused: Option<bool>,
    pub depth: Option<i64>,
}

#[derive(Debug, Clone, Default)]
pub struct OcrSpanLite {
    pub id: String,
    pub text: String,
    pub confidence: Option<f64>,
    pub source_scope: Option<String>,
    pub ownership_kind: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct AppContextLite {
    pub id: String,
    pub adapter_id: String,
    pub object_type: String,
    pub title: Option<String>,
    pub url: Option<String>,
    pub file_path: Option<String>,
    pub selected_text: Option<String>,
    pub focused_object: Option<String>,
    pub confidence: Option<f64>,
}

#[derive(Debug, Clone, Default)]
pub struct WindowSnapshotLite {
    pub active_window_id: Option<i64>,
    pub active_app_pid: Option<i64>,
    pub active_app_bundle_id: Option<String>,
    pub active_app_name: Option<String>,
    pub active_window_title: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct TypingBurstLite {
    pub id: String,
    pub enter_count: i64,
    pub paste_count: i64,
    pub committed: bool,
    pub commit_signal: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct ClipboardMetadataLite {
    pub id: String,
    pub event_kind: String,
    pub source_frame_id: Option<String>,
    pub target_frame_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SurfaceEnrichmentOutput {
    pub snapshot: Option<SurfaceSnapshot>,
    pub status: EnrichmentAttemptStatus,
    pub missing_fields: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct WeakSurfaceEnrichmentPolicyInput {
    pub now_ms: i64,
    pub observed_at_ms: i64,
    pub trigger_type: String,
    pub classification: WeakSurfaceClassification,
    pub surface_key: String,
    pub app_name: Option<String>,
    pub bundle_id: Option<String>,
    pub window_title: Option<String>,
    pub window_id: Option<i64>,
    pub trigger_event_ids: Vec<String>,
    pub recent_attempts_30s: i64,
    pub recent_attempts_5m: i64,
    pub recent_strong_snapshot: bool,
    pub recent_enrichment: bool,
    pub privacy_blocked: bool,
    pub self_capture: bool,
    pub focus_changed: bool,
    pub force_attempt: bool,
}

pub fn ensure_weak_surface_enrichment_schema(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS continue_weak_surface_enrichment_attempts (
          attempt_id TEXT PRIMARY KEY,
          observed_at_ms INTEGER NOT NULL,
          scheduled_at_ms INTEGER NOT NULL,
          completed_at_ms INTEGER,
          surface_key TEXT NOT NULL,
          weak_domain TEXT NOT NULL,
          app_name TEXT,
          bundle_id TEXT,
          window_title_hash TEXT,
          window_title_capped TEXT,
          window_id INTEGER,
          trigger_event_ids_json TEXT NOT NULL,
          trigger_type TEXT NOT NULL,
          attempt_index INTEGER NOT NULL,
          status TEXT NOT NULL,
          reason TEXT,
          snapshot_id TEXT,
          missing_fields_json TEXT NOT NULL,
          adapter_key TEXT,
          policy_version TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_continue_weak_enrichment_surface_time
          ON continue_weak_surface_enrichment_attempts(surface_key, scheduled_at_ms DESC);
        CREATE INDEX IF NOT EXISTS idx_continue_weak_enrichment_status_time
          ON continue_weak_surface_enrichment_attempts(status, completed_at_ms DESC);
        CREATE TABLE IF NOT EXISTS continue_surface_enrichment_attempts (
          id TEXT PRIMARY KEY,
          session_id TEXT,
          surface_key TEXT NOT NULL,
          domain TEXT NOT NULL,
          adapter_key TEXT,
          trigger_type TEXT,
          trigger_event_ids_json TEXT,
          frame_id INTEGER,
          observed_at_ms INTEGER NOT NULL,
          scheduled_at_ms INTEGER,
          completed_at_ms INTEGER,
          attempt_index INTEGER NOT NULL DEFAULT 0,
          status TEXT NOT NULL,
          reason TEXT,
          missing_fields_json TEXT,
          snapshot_id TEXT,
          app_name TEXT,
          bundle_id TEXT,
          window_title TEXT,
          window_title_hash TEXT,
          window_id INTEGER,
          privacy_status TEXT,
          created_at_ms INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_surface_enrichment_attempts_observed
          ON continue_surface_enrichment_attempts(observed_at_ms DESC);
        CREATE INDEX IF NOT EXISTS idx_surface_enrichment_attempts_surface
          ON continue_surface_enrichment_attempts(surface_key, observed_at_ms DESC);
        CREATE INDEX IF NOT EXISTS idx_surface_enrichment_attempts_snapshot
          ON continue_surface_enrichment_attempts(snapshot_id);
        CREATE TABLE IF NOT EXISTS continue_surface_snapshots (
          id TEXT PRIMARY KEY,
          session_id TEXT,
          surface_key TEXT NOT NULL,
          domain TEXT NOT NULL,
          adapter_key TEXT,
          adapter_version TEXT NOT NULL,
          observed_at_ms INTEGER NOT NULL,
          frame_id INTEGER,
          event_ids_json TEXT,
          artifact_id TEXT,
          app_name TEXT,
          bundle_id TEXT,
          app_pid INTEGER,
          window_id INTEGER,
          window_title TEXT,
          window_title_hash TEXT,
          workspace_path TEXT,
          workspace_path_hash TEXT,
          repo_root_path TEXT,
          repo_root_hash TEXT,
          git_branch TEXT,
          git_worktree_path TEXT,
          git_worktree_hash TEXT,
          thread_title TEXT,
          thread_key_hash TEXT,
          active_file_path TEXT,
          active_file_path_hash TEXT,
          active_relative_file TEXT,
          focused_control_role TEXT,
          focused_control_label TEXT,
          selected_text_hash TEXT,
          focused_text_hash TEXT,
          visible_text_sample TEXT,
          visible_text_hash TEXT,
          activity_state TEXT,
          task_state TEXT,
          command_state TEXT,
          error_markers_json TEXT,
          activity_signals_json TEXT,
          identity_confidence TEXT NOT NULL,
          evidence_quality TEXT NOT NULL,
          openability TEXT NOT NULL,
          privacy_status TEXT,
          missing_fields_json TEXT,
          evidence_sources_json TEXT NOT NULL,
          redaction_notes_json TEXT,
          created_at_ms INTEGER NOT NULL,
          updated_at_ms INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_surface_snapshots_observed
          ON continue_surface_snapshots(observed_at_ms DESC);
        CREATE INDEX IF NOT EXISTS idx_surface_snapshots_surface
          ON continue_surface_snapshots(surface_key, observed_at_ms DESC);
        CREATE INDEX IF NOT EXISTS idx_surface_snapshots_artifact
          ON continue_surface_snapshots(artifact_id, observed_at_ms DESC);
        CREATE INDEX IF NOT EXISTS idx_surface_snapshots_repo
          ON continue_surface_snapshots(repo_root_hash, observed_at_ms DESC);
        CREATE TABLE IF NOT EXISTS local_memory_maintenance (
          key TEXT PRIMARY KEY,
          value TEXT NOT NULL,
          updated_at_ms INTEGER NOT NULL
        );
        ",
    )
    .map_err(to_string)
}

pub fn record_weak_surface_enrichment_for_event(
    conn: &Connection,
    input: WeakSurfaceEnrichmentEventInput,
) -> Result<Option<WeakSurfaceEnrichmentAttempt>, String> {
    ensure_weak_surface_enrichment_schema(conn)?;
    let event_label = input
        .key_category
        .as_ref()
        .map(|category| format!("{}:{}", input.trigger_type, category))
        .unwrap_or_else(|| input.trigger_type.clone());
    let classification = classify_weak_surface(&WeakSurfaceClassificationInput {
        app_name: input.app_name.clone(),
        bundle_id: input.bundle_id.clone(),
        window_title: input.window_title.clone(),
        event_types: vec![event_label],
        trigger_type: Some(input.trigger_type.clone()),
        privacy_status: input.privacy_status.clone(),
        ..Default::default()
    });
    if classification.domain == WeakSurfaceDomain::NotWeakSurface {
        return Ok(None);
    }
    let surface_key = enrichment_surface_key_with_window_id(
        input.app_name.as_deref(),
        input.bundle_id.as_deref(),
        input.window_title.as_deref(),
        input.window_id,
        &classification.domain,
    );
    let recent_attempts_30s =
        count_attempts_since(conn, &surface_key, input.now_ms.saturating_sub(30_000))?;
    let recent_attempts_5m =
        count_attempts_since(conn, &surface_key, input.now_ms.saturating_sub(5 * 60_000))?;
    let policy_input = WeakSurfaceEnrichmentPolicyInput {
        now_ms: input.now_ms,
        observed_at_ms: input.observed_at_ms,
        trigger_type: input.trigger_type,
        classification,
        recent_strong_snapshot: has_recent_strong_snapshot(conn, &surface_key, input.now_ms)?,
        recent_enrichment: has_recent_enrichment(conn, &surface_key, input.now_ms)?,
        privacy_blocked: false,
        self_capture: is_smalltalk_self_surface(
            input.app_name.as_deref(),
            input.bundle_id.as_deref(),
        ),
        surface_key,
        app_name: input.app_name,
        bundle_id: input.bundle_id,
        window_title: input.window_title,
        window_id: input.window_id,
        trigger_event_ids: input.trigger_event_ids,
        recent_attempts_30s,
        recent_attempts_5m,
        focus_changed: false,
        force_attempt: input.force_attempt,
    };
    let policy_input = WeakSurfaceEnrichmentPolicyInput {
        privacy_blocked: policy_input.classification.privacy_tier == "blocked",
        ..policy_input
    };
    let attempt = build_enrichment_attempt(policy_input);
    persist_enrichment_attempt(conn, &attempt)?;
    let surface_attempt = surface_enrichment_attempt_from_weak_attempt(
        &attempt,
        input.session_id.as_deref(),
        input.privacy_status.as_deref(),
    )?;
    insert_surface_enrichment_attempt(conn, &surface_attempt)?;
    if let Some(snapshot) = surface_snapshot_from_attempt(
        &attempt,
        input.session_id.as_deref(),
        input.privacy_status.as_deref(),
    )? {
        upsert_surface_snapshot(conn, &snapshot)?;
    }
    update_enrichment_counters(conn, &attempt)?;
    Ok(Some(attempt))
}

pub fn run_continue_request_weak_surface_enrichment(
    conn: &Connection,
    session_id: Option<&str>,
    now_ms: i64,
) -> Result<Option<WeakSurfaceEnrichmentAttempt>, String> {
    ensure_weak_surface_enrichment_schema(conn)?;
    let Some(row) = latest_enrichable_event(conn, session_id)? else {
        return Ok(None);
    };
    record_weak_surface_enrichment_for_event(
        conn,
        WeakSurfaceEnrichmentEventInput {
            session_id: session_id.map(str::to_string),
            observed_at_ms: row.ts_ms,
            now_ms,
            trigger_type: "continue_request".to_string(),
            trigger_event_ids: vec![row.id],
            app_name: row.app_name,
            bundle_id: row.bundle_id,
            window_title: row.window_title,
            window_id: row.window_id,
            key_category: row.key_category,
            privacy_status: None,
            force_attempt: true,
        },
    )
}

pub fn weak_surface_enrichment_diagnostics(
    conn: &Connection,
) -> Result<WeakSurfaceEnrichmentDiagnostics, String> {
    ensure_weak_surface_enrichment_schema(conn)?;
    let latest = latest_enrichment_attempt(conn)?;
    Ok(WeakSurfaceEnrichmentDiagnostics {
        weak_surface_enrichment_attempts: count_statuses(conn, None)?,
        weak_surface_enrichment_success_strong: count_statuses(
            conn,
            Some(EnrichmentAttemptStatus::SucceededStrong.as_str()),
        )?,
        weak_surface_enrichment_success_medium: count_statuses(
            conn,
            Some(EnrichmentAttemptStatus::SucceededMedium.as_str()),
        )?,
        weak_surface_enrichment_success_thin: count_statuses(
            conn,
            Some(EnrichmentAttemptStatus::SucceededThin.as_str()),
        )?,
        weak_surface_enrichment_skipped_privacy: count_statuses(
            conn,
            Some(EnrichmentAttemptStatus::SkippedPrivacy.as_str()),
        )?,
        weak_surface_enrichment_skipped_budget: count_statuses(
            conn,
            Some(EnrichmentAttemptStatus::SkippedBudget.as_str()),
        )?,
        weak_surface_enrichment_failed: count_statuses(
            conn,
            Some(EnrichmentAttemptStatus::FailedHelper.as_str()),
        )? + count_statuses(
            conn,
            Some(EnrichmentAttemptStatus::FailedNoTextOrIdentity.as_str()),
        )?,
        latest_weak_surface_snapshot_id: latest.as_ref().and_then(|attempt| {
            attempt
                .status
                .produced_snapshot()
                .then(|| attempt.snapshot_id.clone())
                .flatten()
        }),
        latest_weak_surface_attempt: latest,
    })
}

pub fn insert_surface_enrichment_attempt(
    conn: &Connection,
    attempt: &SurfaceEnrichmentAttempt,
) -> Result<(), String> {
    ensure_weak_surface_enrichment_schema(conn)?;
    let sanitized = sanitize_surface_enrichment_attempt(attempt)?;
    conn.execute(
        "INSERT INTO continue_surface_enrichment_attempts (
            id, session_id, surface_key, domain, adapter_key, trigger_type,
            trigger_event_ids_json, frame_id, observed_at_ms, scheduled_at_ms,
            completed_at_ms, attempt_index, status, reason, missing_fields_json,
            snapshot_id, app_name, bundle_id, window_title, window_title_hash,
            window_id, privacy_status, created_at_ms
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12,
                   ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23)
         ON CONFLICT(id) DO UPDATE SET
           session_id = excluded.session_id,
           surface_key = excluded.surface_key,
           domain = excluded.domain,
           adapter_key = excluded.adapter_key,
           trigger_type = excluded.trigger_type,
           trigger_event_ids_json = excluded.trigger_event_ids_json,
           frame_id = excluded.frame_id,
           observed_at_ms = excluded.observed_at_ms,
           scheduled_at_ms = excluded.scheduled_at_ms,
           completed_at_ms = excluded.completed_at_ms,
           attempt_index = excluded.attempt_index,
           status = excluded.status,
           reason = excluded.reason,
           missing_fields_json = excluded.missing_fields_json,
           snapshot_id = excluded.snapshot_id,
           app_name = excluded.app_name,
           bundle_id = excluded.bundle_id,
           window_title = excluded.window_title,
           window_title_hash = excluded.window_title_hash,
           window_id = excluded.window_id,
           privacy_status = excluded.privacy_status,
           created_at_ms = excluded.created_at_ms",
        params![
            sanitized.id.as_str(),
            sanitized.session_id.as_deref(),
            sanitized.surface_key.as_str(),
            sanitized.domain.as_str(),
            sanitized.adapter_key.as_deref(),
            sanitized.trigger_type.as_deref(),
            sanitized.trigger_event_ids_json.as_str(),
            sanitized.frame_id,
            sanitized.observed_at_ms,
            sanitized.scheduled_at_ms,
            sanitized.completed_at_ms,
            sanitized.attempt_index,
            sanitized.status.as_str(),
            sanitized.reason.as_deref(),
            sanitized.missing_fields_json.as_str(),
            sanitized.snapshot_id.as_deref(),
            sanitized.app_name.as_deref(),
            sanitized.bundle_id.as_deref(),
            sanitized.window_title.as_deref(),
            sanitized.window_title_hash.as_deref(),
            sanitized.window_id,
            sanitized.privacy_status.as_deref(),
            sanitized.created_at_ms,
        ],
    )
    .map_err(to_string)?;
    Ok(())
}

pub fn upsert_surface_snapshot(
    conn: &Connection,
    snapshot: &SurfaceSnapshot,
) -> Result<(), String> {
    ensure_weak_surface_enrichment_schema(conn)?;
    let sanitized = sanitize_surface_snapshot(snapshot)?;
    conn.execute(
        "INSERT INTO continue_surface_snapshots (
            id, session_id, surface_key, domain, adapter_key, adapter_version,
            observed_at_ms, frame_id, event_ids_json, artifact_id, app_name,
            bundle_id, app_pid, window_id, window_title, window_title_hash,
            workspace_path, workspace_path_hash, repo_root_path, repo_root_hash,
            git_branch, git_worktree_path, git_worktree_hash, thread_title,
            thread_key_hash, active_file_path, active_file_path_hash,
            active_relative_file, focused_control_role, focused_control_label,
            selected_text_hash, focused_text_hash, visible_text_sample,
            visible_text_hash, activity_state, task_state, command_state,
            error_markers_json, activity_signals_json, identity_confidence,
            evidence_quality, openability, privacy_status, missing_fields_json,
            evidence_sources_json, redaction_notes_json, created_at_ms, updated_at_ms
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12,
                   ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23,
                   ?24, ?25, ?26, ?27, ?28, ?29, ?30, ?31, ?32, ?33, ?34,
                   ?35, ?36, ?37, ?38, ?39, ?40, ?41, ?42, ?43, ?44, ?45,
                   ?46, ?47, ?48)
         ON CONFLICT(id) DO UPDATE SET
           session_id = excluded.session_id,
           surface_key = excluded.surface_key,
           domain = excluded.domain,
           adapter_key = excluded.adapter_key,
           adapter_version = excluded.adapter_version,
           observed_at_ms = excluded.observed_at_ms,
           frame_id = excluded.frame_id,
           event_ids_json = excluded.event_ids_json,
           artifact_id = COALESCE(excluded.artifact_id, continue_surface_snapshots.artifact_id),
           app_name = excluded.app_name,
           bundle_id = excluded.bundle_id,
           app_pid = excluded.app_pid,
           window_id = excluded.window_id,
           window_title = excluded.window_title,
           window_title_hash = excluded.window_title_hash,
           workspace_path = excluded.workspace_path,
           workspace_path_hash = excluded.workspace_path_hash,
           repo_root_path = excluded.repo_root_path,
           repo_root_hash = excluded.repo_root_hash,
           git_branch = excluded.git_branch,
           git_worktree_path = excluded.git_worktree_path,
           git_worktree_hash = excluded.git_worktree_hash,
           thread_title = excluded.thread_title,
           thread_key_hash = excluded.thread_key_hash,
           active_file_path = excluded.active_file_path,
           active_file_path_hash = excluded.active_file_path_hash,
           active_relative_file = excluded.active_relative_file,
           focused_control_role = excluded.focused_control_role,
           focused_control_label = excluded.focused_control_label,
           selected_text_hash = excluded.selected_text_hash,
           focused_text_hash = excluded.focused_text_hash,
           visible_text_sample = excluded.visible_text_sample,
           visible_text_hash = excluded.visible_text_hash,
           activity_state = excluded.activity_state,
           task_state = excluded.task_state,
           command_state = excluded.command_state,
           error_markers_json = excluded.error_markers_json,
           activity_signals_json = excluded.activity_signals_json,
           identity_confidence = excluded.identity_confidence,
           evidence_quality = excluded.evidence_quality,
           openability = excluded.openability,
           privacy_status = excluded.privacy_status,
           missing_fields_json = excluded.missing_fields_json,
           evidence_sources_json = excluded.evidence_sources_json,
           redaction_notes_json = excluded.redaction_notes_json,
           updated_at_ms = excluded.updated_at_ms",
        params![
            sanitized.id.as_str(),
            sanitized.session_id.as_deref(),
            sanitized.surface_key.as_str(),
            sanitized.domain.as_str(),
            sanitized.adapter_key.as_deref(),
            sanitized.adapter_version.as_str(),
            sanitized.observed_at_ms,
            sanitized.frame_id,
            sanitized.event_ids_json.as_deref(),
            sanitized.artifact_id.as_deref(),
            sanitized.app_name.as_deref(),
            sanitized.bundle_id.as_deref(),
            sanitized.app_pid,
            sanitized.window_id,
            sanitized.window_title.as_deref(),
            sanitized.window_title_hash.as_deref(),
            sanitized.workspace_path.as_deref(),
            sanitized.workspace_path_hash.as_deref(),
            sanitized.repo_root_path.as_deref(),
            sanitized.repo_root_hash.as_deref(),
            sanitized.git_branch.as_deref(),
            sanitized.git_worktree_path.as_deref(),
            sanitized.git_worktree_hash.as_deref(),
            sanitized.thread_title.as_deref(),
            sanitized.thread_key_hash.as_deref(),
            sanitized.active_file_path.as_deref(),
            sanitized.active_file_path_hash.as_deref(),
            sanitized.active_relative_file.as_deref(),
            sanitized.focused_control_role.as_deref(),
            sanitized.focused_control_label.as_deref(),
            sanitized.selected_text_hash.as_deref(),
            sanitized.focused_text_hash.as_deref(),
            sanitized.visible_text_sample.as_deref(),
            sanitized.visible_text_hash.as_deref(),
            sanitized.activity_state.as_deref(),
            sanitized.task_state.as_deref(),
            sanitized.command_state.as_deref(),
            sanitized.error_markers_json.as_deref(),
            sanitized.activity_signals_json.as_deref(),
            sanitized.identity_confidence.as_str(),
            sanitized.evidence_quality.as_str(),
            sanitized.openability.as_str(),
            sanitized.privacy_status.as_deref(),
            sanitized.missing_fields_json.as_deref(),
            sanitized.evidence_sources_json.as_str(),
            sanitized.redaction_notes_json.as_deref(),
            sanitized.created_at_ms,
            sanitized.updated_at_ms,
        ],
    )
    .map_err(to_string)?;
    Ok(())
}

pub fn load_latest_surface_snapshot_for_surface(
    conn: &Connection,
    surface_key: &str,
    max_age_ms: i64,
) -> Result<Option<SurfaceSnapshot>, String> {
    ensure_weak_surface_enrichment_schema(conn)?;
    let cutoff_ms = current_time_millis().saturating_sub(max_age_ms.max(0));
    conn.query_row(
        &format!(
            "SELECT {}
             FROM continue_surface_snapshots
             WHERE surface_key = ?1
               AND observed_at_ms >= ?2
             ORDER BY observed_at_ms DESC, updated_at_ms DESC
             LIMIT 1",
            SURFACE_SNAPSHOT_COLUMNS
        ),
        params![surface_key, cutoff_ms],
        surface_snapshot_from_row,
    )
    .optional()
    .map_err(to_string)
}

pub fn load_recent_surface_snapshots(
    conn: &Connection,
    since_ms: i64,
    limit: usize,
) -> Result<Vec<SurfaceSnapshot>, String> {
    ensure_weak_surface_enrichment_schema(conn)?;
    let limit = i64::try_from(limit.clamp(1, 500)).unwrap_or(500);
    let mut stmt = conn
        .prepare(&format!(
            "SELECT {}
             FROM continue_surface_snapshots
             WHERE observed_at_ms >= ?1
             ORDER BY observed_at_ms DESC, updated_at_ms DESC
             LIMIT ?2",
            SURFACE_SNAPSHOT_COLUMNS
        ))
        .map_err(to_string)?;
    let rows = stmt
        .query_map(params![since_ms, limit], surface_snapshot_from_row)
        .map_err(to_string)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(to_string)
}

pub fn load_latest_surface_snapshot_for_frame(
    conn: &Connection,
    frame_id: &str,
) -> Result<Option<SurfaceSnapshot>, String> {
    ensure_weak_surface_enrichment_schema(conn)?;
    conn.query_row(
        &format!(
            "SELECT {}
             FROM continue_surface_snapshots
             WHERE frame_id = ?1
             ORDER BY observed_at_ms DESC, updated_at_ms DESC
             LIMIT 1",
            SURFACE_SNAPSHOT_COLUMNS
        ),
        params![frame_id],
        surface_snapshot_from_row,
    )
    .optional()
    .map_err(to_string)
}

pub fn link_surface_snapshot_to_artifact(
    conn: &Connection,
    snapshot_id: &str,
    artifact_id: &str,
) -> Result<(), String> {
    ensure_weak_surface_enrichment_schema(conn)?;
    conn.execute(
        "UPDATE continue_surface_snapshots
         SET artifact_id = ?2,
             updated_at_ms = ?3
         WHERE id = ?1",
        params![snapshot_id, artifact_id, current_time_millis()],
    )
    .map_err(to_string)?;
    Ok(())
}

pub fn weak_surface_identity_from_snapshot(
    snapshot: &SurfaceSnapshot,
) -> Option<WeakSurfaceIdentity> {
    build_weak_surface_identity(WeakSurfaceIdentityInput {
        domain: parse_weak_domain(&snapshot.domain),
        app_name: snapshot.app_name.clone(),
        bundle_id: snapshot.bundle_id.clone(),
        window_id: snapshot.window_id,
        window_title: snapshot.window_title.clone(),
        workspace_path: snapshot.workspace_path.clone(),
        workspace_path_hash: snapshot.workspace_path_hash.clone(),
        repo_root_path: snapshot.repo_root_path.clone(),
        repo_root_hash: snapshot.repo_root_hash.clone(),
        git_worktree_path: snapshot.git_worktree_path.clone(),
        git_worktree_hash: snapshot.git_worktree_hash.clone(),
        git_branch: snapshot.git_branch.clone(),
        thread_title: snapshot.thread_title.clone(),
        thread_key_hash: snapshot.thread_key_hash.clone(),
        active_file_path: snapshot.active_file_path.clone(),
        active_file_path_hash: snapshot.active_file_path_hash.clone(),
        active_relative_file: snapshot.active_relative_file.clone(),
        command_signature_hash: command_signature_hash_from_snapshot(snapshot),
        observed_at_ms: Some(snapshot.observed_at_ms),
    })
}

pub fn build_weak_surface_identity(input: WeakSurfaceIdentityInput) -> Option<WeakSurfaceIdentity> {
    if input.domain == WeakSurfaceDomain::NotWeakSurface {
        return None;
    }
    let artifact_kind = domain_artifact_kind(&input.domain).to_string();
    let repo_hash = strongest_hash(
        input.repo_root_hash.as_deref(),
        input.repo_root_path.as_deref(),
        "repo_root_path",
    );
    let workspace_hash = strongest_hash(
        input.workspace_path_hash.as_deref(),
        input.workspace_path.as_deref(),
        "workspace_path",
    );
    let worktree_hash = strongest_hash(
        input.git_worktree_hash.as_deref(),
        input.git_worktree_path.as_deref(),
        "git_worktree_path",
    );
    let thread_key_hash = input.thread_key_hash.clone().or_else(|| {
        input
            .thread_title
            .as_deref()
            .map(|value| hash_sensitive_value("thread", value))
    });
    let thread_title_hash = input
        .thread_title
        .as_deref()
        .map(|value| hash_sensitive_value("thread_title", value));
    let active_file_hash = strongest_hash(
        input.active_file_path_hash.as_deref(),
        input.active_file_path.as_deref(),
        "active_file",
    );
    let relative_file_hash = input
        .active_relative_file
        .as_deref()
        .map(|value| hash_sensitive_value("relative_file", value));
    let window_surface_hash = weak_window_surface_hash(
        input.bundle_id.as_deref(),
        input.app_name.as_deref(),
        input.window_id,
        input.window_title.as_deref(),
    );
    let app_hash = strongest_hash(
        input.bundle_id.as_deref(),
        input.app_name.as_deref(),
        "app_identity",
    );
    let time_bucket = input
        .observed_at_ms
        .map(|ms| (ms / (5 * 60_000)).to_string());

    let mut missing_fields = Vec::new();
    let key = match input.domain {
        WeakSurfaceDomain::CodexCli => {
            if let (Some(repo), Some(thread)) = (repo_hash.as_ref(), thread_key_hash.as_ref()) {
                IdentityKeyChoice::new(
                    format!("codex_cli:{}:{}", repo, thread),
                    "strong",
                    "frame_fallback",
                )
            } else if let (Some(repo), Some(thread)) =
                (repo_hash.as_ref(), thread_title_hash.as_ref())
            {
                IdentityKeyChoice::new(
                    format!("codex_cli:{}:{}", repo, thread),
                    "strong",
                    "frame_fallback",
                )
            } else if let (Some(repo), Some(surface)) =
                (repo_hash.as_ref(), window_surface_hash.as_ref())
            {
                missing_fields.push("thread_identity_missing".to_string());
                IdentityKeyChoice::new(
                    format!("codex_cli:{}:{}", repo, surface),
                    "medium",
                    "frame_fallback",
                )
            } else if let (Some(app), Some(window), Some(bucket)) = (
                app_hash.as_ref(),
                window_surface_hash.as_ref(),
                time_bucket.as_ref(),
            ) {
                missing_fields.push("repo_root_missing".to_string());
                missing_fields.push("thread_identity_missing".to_string());
                IdentityKeyChoice::new(
                    format!("codex_cli:{}:{}:{}", app, window, bucket),
                    "thin",
                    "app_focus_only",
                )
            } else {
                return None;
            }
        }
        WeakSurfaceDomain::CodexDesktopApp => {
            let project_hash = repo_hash
                .as_ref()
                .or(worktree_hash.as_ref())
                .or(workspace_hash.as_ref());
            if let (Some(project), Some(thread)) = (project_hash, thread_key_hash.as_ref()) {
                IdentityKeyChoice::new(
                    format!("codex_app:{}:{}", project, thread),
                    "strong",
                    "frame_fallback",
                )
            } else if let (Some(project), Some(thread)) = (project_hash, thread_title_hash.as_ref())
            {
                IdentityKeyChoice::new(
                    format!("codex_app:{}:{}", project, thread),
                    "strong",
                    "frame_fallback",
                )
            } else if let (Some(project), Some(surface)) =
                (project_hash, window_surface_hash.as_ref())
            {
                missing_fields.push("thread_identity_missing".to_string());
                IdentityKeyChoice::new(
                    format!("codex_app:{}:{}", project, surface),
                    "medium",
                    "frame_fallback",
                )
            } else if let (Some(app), Some(window)) =
                (app_hash.as_ref(), window_surface_hash.as_ref())
            {
                missing_fields.push("project_identity_missing".to_string());
                missing_fields.push("thread_identity_missing".to_string());
                IdentityKeyChoice::new(
                    format!("codex_app:{}:{}", app, window),
                    "thin",
                    "app_focus_only",
                )
            } else {
                return None;
            }
        }
        WeakSurfaceDomain::CodexIdeExtension => {
            if let (Some(repo), Some(file), Some(thread)) = (
                repo_hash.as_ref(),
                relative_file_hash.as_ref(),
                thread_key_hash.as_ref().or(thread_title_hash.as_ref()),
            ) {
                IdentityKeyChoice::new(
                    format!("codex_ide:{}:{}:{}", repo, file, thread),
                    "strong",
                    "openable",
                )
            } else if let (Some(repo), Some(thread)) =
                (repo_hash.as_ref(), thread_key_hash.as_ref())
            {
                missing_fields.push("active_file_missing".to_string());
                IdentityKeyChoice::new(
                    format!("codex_ide:{}:{}", repo, thread),
                    "strong",
                    "frame_fallback",
                )
            } else if let (Some(repo), Some(file)) =
                (repo_hash.as_ref(), relative_file_hash.as_ref())
            {
                missing_fields.push("thread_identity_missing".to_string());
                IdentityKeyChoice::new(format!("codex_ide:{}:{}", repo, file), "medium", "openable")
            } else if let (Some(workspace), Some(surface)) =
                (workspace_hash.as_ref(), window_surface_hash.as_ref())
            {
                missing_fields.push("repo_root_missing".to_string());
                missing_fields.push("thread_identity_missing".to_string());
                IdentityKeyChoice::new(
                    format!("codex_ide:{}:{}", workspace, surface),
                    "thin",
                    "frame_fallback",
                )
            } else {
                return None;
            }
        }
        WeakSurfaceDomain::CodeEditor => {
            if let (Some(repo), Some(file)) = (repo_hash.as_ref(), relative_file_hash.as_ref()) {
                IdentityKeyChoice::new(
                    format!("code_editor:{}:{}", repo, file),
                    "strong",
                    "openable",
                )
            } else if let (Some(workspace), Some(file)) =
                (workspace_hash.as_ref(), active_file_hash.as_ref())
            {
                missing_fields.push("repo_root_missing".to_string());
                IdentityKeyChoice::new(
                    format!("code_editor:{}:{}", workspace, file),
                    "medium",
                    "openable",
                )
            } else if let (Some(repo), Some(surface)) =
                (repo_hash.as_ref(), window_surface_hash.as_ref())
            {
                missing_fields.push("active_file_missing".to_string());
                IdentityKeyChoice::new(
                    format!("code_editor:{}:{}", repo, surface),
                    "medium",
                    "frame_fallback",
                )
            } else if let (Some(app), Some(window)) =
                (app_hash.as_ref(), window_surface_hash.as_ref())
            {
                missing_fields.push("repo_root_missing".to_string());
                missing_fields.push("active_file_missing".to_string());
                IdentityKeyChoice::new(
                    format!("code_editor:{}:{}", app, window),
                    "thin",
                    "app_focus_only",
                )
            } else {
                return None;
            }
        }
        WeakSurfaceDomain::Terminal => {
            if let (Some(repo), Some(command)) =
                (repo_hash.as_ref(), input.command_signature_hash.as_ref())
            {
                IdentityKeyChoice::new(
                    format!("terminal:{}:{}", repo, command),
                    "strong",
                    "frame_fallback",
                )
            } else if let (Some(repo), Some(surface)) =
                (repo_hash.as_ref(), window_surface_hash.as_ref())
            {
                missing_fields.push("command_signature_missing".to_string());
                IdentityKeyChoice::new(
                    format!("terminal:{}:{}", repo, surface),
                    "medium",
                    "frame_fallback",
                )
            } else if let (Some(app), Some(window), Some(bucket)) = (
                app_hash.as_ref(),
                window_surface_hash.as_ref(),
                time_bucket.as_ref(),
            ) {
                missing_fields.push("repo_root_missing".to_string());
                missing_fields.push("command_signature_missing".to_string());
                IdentityKeyChoice::new(
                    format!("terminal:{}:{}:{}", app, window, bucket),
                    "thin",
                    "app_focus_only",
                )
            } else {
                return None;
            }
        }
        WeakSurfaceDomain::NativeAgentWindow => {
            let project_hash = repo_hash
                .as_ref()
                .or(worktree_hash.as_ref())
                .or(workspace_hash.as_ref());
            if let (Some(project), Some(thread)) = (project_hash, thread_key_hash.as_ref()) {
                IdentityKeyChoice::new(
                    format!("native_agent:{}:{}", project, thread),
                    "strong",
                    "frame_fallback",
                )
            } else if let (Some(project), Some(thread)) = (project_hash, thread_title_hash.as_ref())
            {
                IdentityKeyChoice::new(
                    format!("native_agent:{}:{}", project, thread),
                    "strong",
                    "frame_fallback",
                )
            } else if let (Some(app), Some(window)) =
                (app_hash.as_ref(), window_surface_hash.as_ref())
            {
                missing_fields.push("project_identity_missing".to_string());
                missing_fields.push("thread_identity_missing".to_string());
                IdentityKeyChoice::new(
                    format!("native_agent:{}:{}", app, window),
                    "thin",
                    "app_focus_only",
                )
            } else {
                return None;
            }
        }
        WeakSurfaceDomain::UnknownWeakSurface | WeakSurfaceDomain::NotWeakSurface => {
            return None;
        }
    };

    add_required_missing_fields(&input, &mut missing_fields);
    let mut merge_keys = identity_merge_keys(
        repo_hash.as_deref(),
        workspace_hash.as_deref(),
        thread_key_hash.as_deref().or(thread_title_hash.as_deref()),
        relative_file_hash.as_deref(),
        input.command_signature_hash.as_deref(),
        window_surface_hash.as_deref(),
        &input.domain,
    );
    sort_dedup(&mut missing_fields);
    sort_dedup(&mut merge_keys);

    Some(WeakSurfaceIdentity {
        artifact_kind,
        stable_key: key.stable_key,
        display_title: weak_identity_display_title(&input),
        identity_confidence: key.identity_confidence,
        openability: key.openability,
        missing_fields,
        merge_keys,
    })
}

#[derive(Debug)]
struct IdentityKeyChoice {
    stable_key: String,
    identity_confidence: String,
    openability: String,
}

impl IdentityKeyChoice {
    fn new(stable_key: String, identity_confidence: &str, openability: &str) -> Self {
        Self {
            stable_key,
            identity_confidence: identity_confidence.to_string(),
            openability: openability.to_string(),
        }
    }
}

fn strongest_hash(
    existing_hash: Option<&str>,
    raw_value: Option<&str>,
    domain: &str,
) -> Option<String> {
    existing_hash
        .and_then(|value| clean_string(Some(value), 80))
        .or_else(|| raw_value.map(|value| hash_sensitive_value(domain, value)))
}

fn weak_window_surface_hash(
    bundle_id: Option<&str>,
    app_name: Option<&str>,
    window_id: Option<i64>,
    window_title: Option<&str>,
) -> Option<String> {
    let owner = normalize_key_component(bundle_id).or_else(|| normalize_key_component(app_name))?;
    let title_hash = window_title
        .and_then(|title| clean_string(Some(title), MAX_WINDOW_TITLE_CHARS))
        .map(|title| hash_sensitive_value("window_title", &title));
    let raw = match (window_id, title_hash) {
        (Some(id), Some(title)) => format!("{}:window:{}:{}", owner, id, title),
        (Some(id), None) => format!("{}:window:{}", owner, id),
        (None, Some(title)) => format!("{}:title:{}", owner, title),
        (None, None) => return None,
    };
    Some(hash_sensitive_value("window_surface", &raw))
}

fn command_signature_hash_from_snapshot(snapshot: &SurfaceSnapshot) -> Option<String> {
    if !matches!(
        parse_weak_domain(&snapshot.domain),
        WeakSurfaceDomain::Terminal | WeakSurfaceDomain::CodexCli
    ) {
        return None;
    }
    snapshot
        .activity_signals_json
        .as_deref()
        .and_then(|json| serde_json::from_str::<serde_json::Value>(json).ok())
        .and_then(|value| {
            value
                .get("terminal_markers")
                .or_else(|| value.get("trigger_type"))
                .cloned()
        })
        .map(|value| hash_sensitive_value("command_signature", &value.to_string()))
}

fn add_required_missing_fields(input: &WeakSurfaceIdentityInput, missing_fields: &mut Vec<String>) {
    if input.repo_root_hash.is_none() && input.repo_root_path.is_none() {
        match input.domain {
            WeakSurfaceDomain::CodexCli
            | WeakSurfaceDomain::CodexIdeExtension
            | WeakSurfaceDomain::CodeEditor
            | WeakSurfaceDomain::Terminal => {
                missing_fields.push("repo_root_missing".to_string());
            }
            _ => {}
        }
    }
    if input.thread_key_hash.is_none() && input.thread_title.is_none() {
        match input.domain {
            WeakSurfaceDomain::CodexCli
            | WeakSurfaceDomain::CodexDesktopApp
            | WeakSurfaceDomain::CodexIdeExtension
            | WeakSurfaceDomain::NativeAgentWindow => {
                missing_fields.push("thread_identity_missing".to_string());
            }
            _ => {}
        }
    }
    if input.active_file_path.is_none() && input.active_relative_file.is_none() {
        match input.domain {
            WeakSurfaceDomain::CodexIdeExtension | WeakSurfaceDomain::CodeEditor => {
                missing_fields.push("active_file_missing".to_string());
            }
            _ => {}
        }
    }
    if input.command_signature_hash.is_none()
        && matches!(
            input.domain,
            WeakSurfaceDomain::Terminal | WeakSurfaceDomain::CodexCli
        )
    {
        missing_fields.push("command_signature_missing".to_string());
    }
}

fn identity_merge_keys(
    repo_hash: Option<&str>,
    workspace_hash: Option<&str>,
    thread_hash: Option<&str>,
    relative_file_hash: Option<&str>,
    command_signature_hash: Option<&str>,
    window_surface_hash: Option<&str>,
    domain: &WeakSurfaceDomain,
) -> Vec<String> {
    let mut keys = Vec::new();
    if let Some(repo) = repo_hash {
        keys.push(format!("repo:{}", repo));
        if let Some(file) = relative_file_hash {
            keys.push(format!("active_file:{}:{}", repo, file));
        }
        if let Some(command) = command_signature_hash {
            keys.push(format!("command:{}:{}", repo, command));
        }
    }
    if let Some(workspace) = workspace_hash {
        keys.push(format!("workspace:{}", workspace));
    }
    if let Some(thread) = thread_hash {
        keys.push(format!("codex_thread:{}", thread));
    }
    if let Some(surface) = window_surface_hash {
        match domain {
            WeakSurfaceDomain::CodexCli | WeakSurfaceDomain::Terminal => {
                keys.push(format!("terminal_surface:{}", surface));
            }
            WeakSurfaceDomain::CodeEditor | WeakSurfaceDomain::CodexIdeExtension => {
                keys.push(format!("editor_surface:{}", surface));
            }
            WeakSurfaceDomain::CodexDesktopApp | WeakSurfaceDomain::NativeAgentWindow => {
                keys.push(format!("agent_surface:{}", surface));
            }
            _ => {}
        }
    }
    keys
}

fn weak_identity_display_title(input: &WeakSurfaceIdentityInput) -> String {
    let app = input
        .app_name
        .as_deref()
        .and_then(|value| clean_string(Some(value), 80));
    let project = input
        .repo_root_path
        .as_deref()
        .or(input.workspace_path.as_deref())
        .and_then(|path| Path::new(path).file_name())
        .and_then(|name| name.to_str())
        .and_then(|name| clean_string(Some(name), 80));
    let file = input
        .active_relative_file
        .as_deref()
        .and_then(|value| clean_string(Some(value), MAX_RELATIVE_FILE_CHARS));
    let thread = input
        .thread_title
        .as_deref()
        .and_then(|value| clean_string(Some(value), MAX_THREAD_TITLE_CHARS));
    match input.domain {
        WeakSurfaceDomain::CodexCli => {
            join_display("Codex CLI", project.as_deref(), thread.as_deref())
        }
        WeakSurfaceDomain::CodexDesktopApp => {
            join_display("Codex", project.as_deref(), thread.as_deref())
        }
        WeakSurfaceDomain::CodexIdeExtension => join_display(
            "Codex IDE",
            project.as_deref().or(app.as_deref()),
            file.as_deref().or(thread.as_deref()),
        ),
        WeakSurfaceDomain::CodeEditor => join_display(
            app.as_deref().unwrap_or("Editor"),
            project.as_deref(),
            file.as_deref(),
        ),
        WeakSurfaceDomain::Terminal => join_display(
            app.as_deref().unwrap_or("Terminal"),
            project.as_deref(),
            Some("command output"),
        ),
        WeakSurfaceDomain::NativeAgentWindow => join_display(
            app.as_deref().unwrap_or("Agent window"),
            project.as_deref(),
            thread.as_deref(),
        ),
        WeakSurfaceDomain::UnknownWeakSurface | WeakSurfaceDomain::NotWeakSurface => {
            join_display(app.as_deref().unwrap_or("Local app activity"), None, None)
        }
    }
}

fn join_display(prefix: &str, middle: Option<&str>, suffix: Option<&str>) -> String {
    let mut parts = vec![prefix.to_string()];
    if let Some(middle) = middle.and_then(|value| clean_string(Some(value), 80)) {
        parts.push(middle);
    }
    if let Some(suffix) = suffix.and_then(|value| clean_string(Some(value), 160)) {
        if !parts.iter().any(|part| part.eq_ignore_ascii_case(&suffix)) {
            parts.push(suffix);
        }
    }
    parts.join(" - ")
}

fn surface_enrichment_attempt_from_weak_attempt(
    attempt: &WeakSurfaceEnrichmentAttempt,
    session_id: Option<&str>,
    privacy_status: Option<&str>,
) -> Result<SurfaceEnrichmentAttempt, String> {
    Ok(SurfaceEnrichmentAttempt {
        id: attempt.attempt_id.clone(),
        session_id: clean_string(session_id, 160),
        surface_key: attempt.surface_key.clone(),
        domain: weak_domain_label(&attempt.weak_domain),
        adapter_key: attempt.adapter_key.clone(),
        trigger_type: Some(attempt.trigger_type.clone()),
        trigger_event_ids_json: bounded_json_array(&attempt.trigger_event_ids)?,
        frame_id: None,
        observed_at_ms: attempt.observed_at_ms,
        scheduled_at_ms: Some(attempt.scheduled_at_ms),
        completed_at_ms: attempt.completed_at_ms,
        attempt_index: attempt.attempt_index,
        status: attempt.status.as_str().to_string(),
        reason: attempt.reason.clone(),
        missing_fields_json: bounded_json_array(&attempt.missing_fields)?,
        snapshot_id: attempt.snapshot_id.clone(),
        app_name: attempt.app_name.clone(),
        bundle_id: attempt.bundle_id.clone(),
        window_title: attempt.window_title_capped.clone(),
        window_title_hash: attempt.window_title_hash.clone(),
        window_id: attempt.window_id,
        privacy_status: privacy_status
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string),
        created_at_ms: attempt.completed_at_ms.unwrap_or(attempt.scheduled_at_ms),
    })
}

fn surface_snapshot_from_attempt(
    attempt: &WeakSurfaceEnrichmentAttempt,
    session_id: Option<&str>,
    privacy_status: Option<&str>,
) -> Result<Option<SurfaceSnapshot>, String> {
    if !attempt.produced_snapshot() {
        return Ok(None);
    }
    let Some(snapshot_id) = attempt.snapshot_id.clone() else {
        return Ok(None);
    };
    let quality = match attempt.status {
        EnrichmentAttemptStatus::SucceededStrong => "strong",
        EnrichmentAttemptStatus::SucceededMedium => "medium",
        EnrichmentAttemptStatus::SucceededThin => "thin",
        _ => "unknown",
    };
    let trigger_lower = attempt.trigger_type.to_ascii_lowercase();
    let activity_state = if trigger_lower.contains("typing") || trigger_lower.contains("key") {
        "actively_editing"
    } else if trigger_lower.contains("command") {
        "running_command"
    } else {
        "unknown"
    };
    let task_state = if trigger_lower.contains("typing") || trigger_lower.contains("key") {
        "editing_file"
    } else if trigger_lower.contains("command") {
        "command_running"
    } else {
        "unknown"
    };
    let command_state = if matches!(
        attempt.weak_domain,
        WeakSurfaceDomain::Terminal | WeakSurfaceDomain::CodexCli
    ) {
        if trigger_lower.contains("command") {
            "command_running"
        } else {
            "unknown"
        }
    } else {
        "not_terminal"
    };
    let evidence_sources_json = bounded_json_value(&serde_json::json!([
        {
            "kind": "ui_event",
            "ids": attempt.trigger_event_ids,
            "trigger_type": attempt.trigger_type,
            "source": "weak_surface_enrichment"
        }
    ]))?;
    let adapter_input = SurfaceEnrichmentInput {
        classification: WeakSurfaceClassification {
            domain: attempt.weak_domain.clone(),
            enrichment_need: EnrichmentNeed::Targeted,
            confidence: match quality {
                "strong" => 0.9,
                "medium" => 0.72,
                _ => 0.45,
            },
            reasons: vec!["event_only_weak_surface_snapshot".to_string()],
            adapter_key: attempt.adapter_key.clone(),
            privacy_tier: privacy_tier(privacy_status),
            observed_app_name: attempt.app_name.clone(),
            observed_bundle_id: attempt.bundle_id.clone(),
            observed_window_title: attempt.window_title_capped.clone(),
        },
        observed_at_ms: attempt.observed_at_ms,
        session_id: clean_string(session_id, 160),
        frame: Some(CaptureFrameLite {
            id: None,
            app_name: attempt.app_name.clone(),
            bundle_id: attempt.bundle_id.clone(),
            app_pid: None,
            window_id: attempt.window_id,
            window_title: attempt.window_title_capped.clone(),
            browser_url: None,
            document_path: None,
            full_text: None,
            text_source: Some("event_only".to_string()),
            capture_trigger: Some(attempt.trigger_type.clone()),
            privacy_status: privacy_status
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string),
        }),
        recent_events: attempt
            .trigger_event_ids
            .iter()
            .map(|id| UiEventLite {
                id: id.clone(),
                event_type: attempt.trigger_type.clone(),
                key_category: None,
            })
            .collect(),
        content_units: Vec::new(),
        ax_nodes: Vec::new(),
        ocr_spans: Vec::new(),
        app_contexts: Vec::new(),
        window_snapshot: None,
        typing_bursts: trigger_lower
            .contains("typing")
            .then(|| {
                vec![TypingBurstLite {
                    id: "event-only-typing".to_string(),
                    enter_count: i64::from(trigger_lower.contains("enter")),
                    paste_count: i64::from(trigger_lower.contains("paste")),
                    committed: false,
                    commit_signal: Some(attempt.trigger_type.clone()),
                }]
            })
            .unwrap_or_default(),
        clipboard_metadata: Vec::new(),
    };
    if let Some(mut snapshot) = enrich_surface(&adapter_input).snapshot {
        snapshot.id = snapshot_id;
        snapshot.event_ids_json = Some(bounded_json_array(&attempt.trigger_event_ids)?);
        snapshot.evidence_sources_json = evidence_sources_json.clone();
        snapshot.redaction_notes_json = Some(bounded_json_value(&serde_json::json!([
            "event_only_snapshot",
            "visible_text_not_stored"
        ]))?);
        snapshot.missing_fields_json = Some(bounded_json_array(&attempt.missing_fields)?);
        snapshot.created_at_ms = attempt.completed_at_ms.unwrap_or(attempt.scheduled_at_ms);
        snapshot.updated_at_ms = attempt.completed_at_ms.unwrap_or(attempt.scheduled_at_ms);
        apply_surface_snapshot_quality(&mut snapshot)?;
        return Ok(Some(snapshot));
    }
    let mut snapshot = SurfaceSnapshot {
        id: snapshot_id,
        session_id: clean_string(session_id, 160),
        surface_key: attempt.surface_key.clone(),
        domain: weak_domain_label(&attempt.weak_domain),
        adapter_key: attempt.adapter_key.clone(),
        adapter_version: SURFACE_SNAPSHOT_ADAPTER_VERSION.to_string(),
        observed_at_ms: attempt.observed_at_ms,
        frame_id: None,
        event_ids_json: Some(bounded_json_array(&attempt.trigger_event_ids)?),
        artifact_id: None,
        app_name: attempt.app_name.clone(),
        bundle_id: attempt.bundle_id.clone(),
        app_pid: None,
        window_id: attempt.window_id,
        window_title: attempt.window_title_capped.clone(),
        window_title_hash: attempt.window_title_hash.clone(),
        workspace_path: None,
        workspace_path_hash: None,
        repo_root_path: None,
        repo_root_hash: None,
        git_branch: None,
        git_worktree_path: None,
        git_worktree_hash: None,
        thread_title: None,
        thread_key_hash: None,
        active_file_path: None,
        active_file_path_hash: None,
        active_relative_file: None,
        focused_control_role: None,
        focused_control_label: None,
        selected_text_hash: None,
        focused_text_hash: None,
        visible_text_sample: None,
        visible_text_hash: None,
        activity_state: Some(activity_state.to_string()),
        task_state: Some(task_state.to_string()),
        command_state: Some(command_state.to_string()),
        error_markers_json: Some("[]".to_string()),
        activity_signals_json: Some(bounded_json_value(&serde_json::json!({
            "trigger_type": attempt.trigger_type,
            "trigger_event_count": attempt.trigger_event_ids.len(),
            "event_only": true
        }))?),
        identity_confidence: quality.to_string(),
        evidence_quality: quality.to_string(),
        openability: "app_focus_only".to_string(),
        privacy_status: privacy_status
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string),
        missing_fields_json: Some(bounded_json_array(&attempt.missing_fields)?),
        evidence_sources_json,
        redaction_notes_json: Some(bounded_json_value(&serde_json::json!([
            "event_only_snapshot",
            "visible_text_not_stored"
        ]))?),
        created_at_ms: attempt.completed_at_ms.unwrap_or(attempt.scheduled_at_ms),
        updated_at_ms: attempt.completed_at_ms.unwrap_or(attempt.scheduled_at_ms),
    };
    apply_surface_snapshot_quality(&mut snapshot)?;
    Ok(Some(snapshot))
}

pub fn evaluate_surface_snapshot_quality(snapshot: &SurfaceSnapshot) -> SurfaceSnapshotQuality {
    let domain = parse_weak_domain(&snapshot.domain);
    let known_weak_domain = !matches!(
        domain,
        WeakSurfaceDomain::UnknownWeakSurface | WeakSurfaceDomain::NotWeakSurface
    );
    let privacy_blocked = privacy_tier(snapshot.privacy_status.as_deref()) == "blocked";
    let raw_missing = snapshot_missing_fields(snapshot);
    let has_repo_or_workspace = has_any([
        snapshot.repo_root_path.as_deref(),
        snapshot.repo_root_hash.as_deref(),
        snapshot.workspace_path.as_deref(),
        snapshot.workspace_path_hash.as_deref(),
        snapshot.git_worktree_path.as_deref(),
        snapshot.git_worktree_hash.as_deref(),
    ]);
    let has_thread = has_any([
        snapshot.thread_title.as_deref(),
        snapshot.thread_key_hash.as_deref(),
    ]);
    let has_file = has_any([
        snapshot.active_file_path.as_deref(),
        snapshot.active_file_path_hash.as_deref(),
        snapshot.active_relative_file.as_deref(),
    ]);
    let has_command_state = snapshot.command_state.as_deref().is_some_and(|value| {
        matches!(
            value,
            "prompt_ready" | "command_running" | "command_completed" | "command_failed"
        )
    });
    let has_error_state = snapshot
        .error_markers_json
        .as_deref()
        .map(parse_string_array)
        .is_some_and(|markers| !markers.is_empty())
        || matches!(snapshot.command_state.as_deref(), Some("command_failed"))
        || matches!(
            snapshot.task_state.as_deref(),
            Some("visible_error_unresolved")
        );
    let has_meaningful_activity = snapshot.activity_state.as_deref().is_some_and(|value| {
        matches!(
            value,
            "actively_editing"
                | "composing_prompt"
                | "running_command"
                | "observing_output"
                | "reviewing_diff"
                | "encountering_error"
                | "idle_after_progress"
                | "reading"
        )
    }) || snapshot
        .task_state
        .as_deref()
        .is_some_and(|value| value != "unknown")
        || has_command_state
        || has_error_state;
    let has_recent_interaction = snapshot
        .event_ids_json
        .as_deref()
        .map(parse_string_array)
        .is_some_and(|ids| !ids.is_empty())
        || activity_signals_have_interaction(snapshot.activity_signals_json.as_deref())
        || snapshot.observed_at_ms > 0 && has_meaningful_activity;
    let has_focus_or_visible_state = has_any([
        snapshot.focused_control_role.as_deref(),
        snapshot.focused_control_label.as_deref(),
        snapshot.focused_text_hash.as_deref(),
        snapshot.selected_text_hash.as_deref(),
        snapshot.visible_text_sample.as_deref(),
        snapshot.visible_text_hash.as_deref(),
    ]);
    let openability = evaluated_openability(snapshot, privacy_blocked);

    let mut inferred_missing = raw_missing.clone();
    add_snapshot_missing_fields(
        snapshot,
        &domain,
        openability,
        has_repo_or_workspace,
        has_thread,
        has_file,
        has_command_state,
        has_focus_or_visible_state,
        privacy_blocked,
        &mut inferred_missing,
    );
    sort_dedup(&mut inferred_missing);

    let evidence_quality = if privacy_blocked {
        if has_repo_or_workspace && (has_thread || has_file) {
            EvidenceQuality::Thin
        } else {
            EvidenceQuality::Unknown
        }
    } else if !known_weak_domain {
        EvidenceQuality::Unknown
    } else if strong_snapshot_evidence(
        &domain,
        has_repo_or_workspace,
        has_thread,
        has_file,
        has_command_state,
        has_error_state,
        has_meaningful_activity,
        has_recent_interaction,
        has_focus_or_visible_state,
    ) {
        EvidenceQuality::Strong
    } else if medium_snapshot_evidence(
        &domain,
        has_repo_or_workspace,
        has_thread,
        has_file,
        has_command_state,
        has_error_state,
        has_meaningful_activity,
        has_recent_interaction,
        has_focus_or_visible_state,
    ) {
        EvidenceQuality::Medium
    } else if has_weak_presence(snapshot, known_weak_domain, has_recent_interaction) {
        EvidenceQuality::Thin
    } else {
        EvidenceQuality::Unknown
    };

    let identity_confidence = match evidence_quality {
        EvidenceQuality::Strong => IdentityConfidence::Strong,
        EvidenceQuality::Medium => IdentityConfidence::Medium,
        EvidenceQuality::Thin => IdentityConfidence::Thin,
        EvidenceQuality::Unknown => IdentityConfidence::Unknown,
    };

    let candidate_eligible = match evidence_quality {
        EvidenceQuality::Strong | EvidenceQuality::Medium => {
            !privacy_blocked
                && known_weak_domain
                && (has_repo_or_workspace || has_thread || has_file || has_command_state)
        }
        EvidenceQuality::Thin | EvidenceQuality::Unknown => false,
    };
    let stale_target_suppression_strength = match evidence_quality {
        EvidenceQuality::Strong => StaleSuppressionStrength::Strong,
        EvidenceQuality::Medium => StaleSuppressionStrength::Medium,
        EvidenceQuality::Thin if privacy_blocked => StaleSuppressionStrength::Weak,
        EvidenceQuality::Thin if has_recent_interaction && known_weak_domain => {
            StaleSuppressionStrength::Medium
        }
        EvidenceQuality::Thin => StaleSuppressionStrength::Weak,
        EvidenceQuality::Unknown if privacy_blocked || has_recent_interaction => {
            StaleSuppressionStrength::Weak
        }
        EvidenceQuality::Unknown => StaleSuppressionStrength::None,
    };
    let confidence_delta = match evidence_quality {
        EvidenceQuality::Strong => 0.0,
        EvidenceQuality::Medium => -0.12,
        EvidenceQuality::Thin => -0.35,
        EvidenceQuality::Unknown => -0.45,
    };
    let mut warnings = Vec::new();
    for missing in &inferred_missing {
        push_string_once(&mut warnings, &format!("missing_field:{}", missing));
    }
    if !candidate_eligible {
        push_string_once(&mut warnings, "surface_snapshot:not_candidate_eligible");
    }
    if privacy_blocked {
        push_string_once(&mut warnings, "surface_snapshot:privacy_blocked");
    }
    if evidence_quality == EvidenceQuality::Thin && has_recent_interaction {
        push_string_once(&mut warnings, "surface_snapshot:thin_fresh_current_work");
    }

    SurfaceSnapshotQuality {
        evidence_quality,
        identity_confidence,
        candidate_eligible,
        stale_target_suppression_strength,
        openability,
        confidence_delta,
        missing_evidence: product_safe_missing_evidence(&inferred_missing),
        warnings,
    }
}

fn apply_surface_snapshot_quality(snapshot: &mut SurfaceSnapshot) -> Result<(), String> {
    let quality = evaluate_surface_snapshot_quality(snapshot);
    let mut raw_missing = quality
        .warnings
        .iter()
        .filter_map(|warning| warning.strip_prefix("missing_field:"))
        .map(str::to_string)
        .collect::<Vec<_>>();
    sort_dedup(&mut raw_missing);
    snapshot.identity_confidence = quality.identity_confidence.as_str().to_string();
    snapshot.evidence_quality = quality.evidence_quality.as_str().to_string();
    snapshot.openability = quality.openability.as_str().to_string();
    snapshot.missing_fields_json = Some(serde_json::to_string(&raw_missing).map_err(to_string)?);
    Ok(())
}

fn strong_snapshot_evidence(
    domain: &WeakSurfaceDomain,
    has_repo_or_workspace: bool,
    has_thread: bool,
    has_file: bool,
    has_command_state: bool,
    has_error_state: bool,
    has_meaningful_activity: bool,
    has_recent_interaction: bool,
    has_focus_or_visible_state: bool,
) -> bool {
    if !has_recent_interaction || !has_meaningful_activity {
        return false;
    }
    match domain {
        WeakSurfaceDomain::CodexCli => {
            has_repo_or_workspace && has_thread && has_focus_or_visible_state
        }
        WeakSurfaceDomain::CodexDesktopApp => has_repo_or_workspace && has_thread,
        WeakSurfaceDomain::CodexIdeExtension => {
            has_repo_or_workspace && has_file && (has_thread || has_focus_or_visible_state)
        }
        WeakSurfaceDomain::CodeEditor => {
            has_file && has_focus_or_visible_state && has_repo_or_workspace
        }
        WeakSurfaceDomain::Terminal => {
            has_repo_or_workspace && (has_command_state || has_error_state)
        }
        WeakSurfaceDomain::NativeAgentWindow => {
            has_thread && has_focus_or_visible_state && has_repo_or_workspace
        }
        WeakSurfaceDomain::UnknownWeakSurface | WeakSurfaceDomain::NotWeakSurface => false,
    }
}

fn medium_snapshot_evidence(
    domain: &WeakSurfaceDomain,
    has_repo_or_workspace: bool,
    has_thread: bool,
    has_file: bool,
    has_command_state: bool,
    has_error_state: bool,
    has_meaningful_activity: bool,
    has_recent_interaction: bool,
    has_focus_or_visible_state: bool,
) -> bool {
    if !(has_recent_interaction || has_meaningful_activity || has_focus_or_visible_state) {
        return false;
    }
    match domain {
        WeakSurfaceDomain::CodexCli => has_repo_or_workspace || has_thread,
        WeakSurfaceDomain::CodexDesktopApp => {
            (has_repo_or_workspace || has_thread) && has_focus_or_visible_state
        }
        WeakSurfaceDomain::CodexIdeExtension => has_file || has_thread || has_repo_or_workspace,
        WeakSurfaceDomain::CodeEditor => has_file || has_repo_or_workspace,
        WeakSurfaceDomain::Terminal => {
            has_repo_or_workspace || has_command_state || has_error_state
        }
        WeakSurfaceDomain::NativeAgentWindow => has_thread || has_repo_or_workspace,
        WeakSurfaceDomain::UnknownWeakSurface | WeakSurfaceDomain::NotWeakSurface => false,
    }
}

fn has_weak_presence(
    snapshot: &SurfaceSnapshot,
    known_weak_domain: bool,
    has_recent_interaction: bool,
) -> bool {
    known_weak_domain
        || has_recent_interaction
        || has_any([
            snapshot.app_name.as_deref(),
            snapshot.bundle_id.as_deref(),
            snapshot.window_title.as_deref(),
        ])
}

#[allow(clippy::too_many_arguments)]
fn add_snapshot_missing_fields(
    snapshot: &SurfaceSnapshot,
    domain: &WeakSurfaceDomain,
    openability: Openability,
    has_repo_or_workspace: bool,
    has_thread: bool,
    has_file: bool,
    has_command_state: bool,
    has_focus_or_visible_state: bool,
    privacy_blocked: bool,
    missing: &mut Vec<String>,
) {
    if privacy_blocked {
        push_string_once(missing, "privacy_blocked_text");
    }
    if matches!(
        domain,
        WeakSurfaceDomain::CodexCli
            | WeakSurfaceDomain::CodexDesktopApp
            | WeakSurfaceDomain::CodexIdeExtension
            | WeakSurfaceDomain::CodeEditor
            | WeakSurfaceDomain::Terminal
            | WeakSurfaceDomain::NativeAgentWindow
    ) && !has_repo_or_workspace
    {
        push_string_once(missing, "repo_root_missing");
    }
    if matches!(
        domain,
        WeakSurfaceDomain::CodexCli
            | WeakSurfaceDomain::CodexDesktopApp
            | WeakSurfaceDomain::CodexIdeExtension
            | WeakSurfaceDomain::NativeAgentWindow
    ) && !has_thread
    {
        push_string_once(missing, "thread_identity_missing");
    }
    if matches!(
        domain,
        WeakSurfaceDomain::CodexIdeExtension | WeakSurfaceDomain::CodeEditor
    ) && !has_file
    {
        push_string_once(missing, "active_file_missing");
    }
    if matches!(
        domain,
        WeakSurfaceDomain::CodexCli | WeakSurfaceDomain::Terminal
    ) && !has_command_state
    {
        push_string_once(missing, "command_state_missing");
    }
    if !has_focus_or_visible_state {
        push_string_once(missing, "focused_control_missing");
    }
    if snapshot.frame_id.is_none() && !snapshot.evidence_sources_json.contains("\"frame\"") {
        push_string_once(missing, "fresh_heavy_frame_missing");
    }
    if !matches!(openability, Openability::Openable) {
        push_string_once(missing, "openable_target_missing");
    }
}

fn evaluated_openability(snapshot: &SurfaceSnapshot, privacy_blocked: bool) -> Openability {
    if privacy_blocked {
        return Openability::Blocked;
    }
    match normalize_one_of(
        &snapshot.openability,
        &[
            "openable",
            "app_focus_only",
            "frame_fallback",
            "blocked",
            "unknown",
        ],
    )
    .as_str()
    {
        "openable" => Openability::Openable,
        "frame_fallback" => Openability::FrameFallback,
        "app_focus_only" => Openability::AppFocusOnly,
        "blocked" => Openability::Blocked,
        _ if snapshot.active_file_path.is_some()
            || snapshot.active_file_path_hash.is_some()
            || snapshot.active_relative_file.is_some() =>
        {
            Openability::Openable
        }
        _ if snapshot.frame_id.is_some() => Openability::FrameFallback,
        _ if snapshot.window_id.is_some() || snapshot.app_name.is_some() => {
            Openability::AppFocusOnly
        }
        _ => Openability::Unknown,
    }
}

fn snapshot_missing_fields(snapshot: &SurfaceSnapshot) -> Vec<String> {
    let mut values = snapshot
        .missing_fields_json
        .as_deref()
        .map(parse_string_array)
        .unwrap_or_default();
    sort_dedup(&mut values);
    values
}

fn parse_string_array(value: &str) -> Vec<String> {
    serde_json::from_str::<Vec<String>>(value).unwrap_or_default()
}

fn activity_signals_have_interaction(value: Option<&str>) -> bool {
    let Some(value) = value else {
        return false;
    };
    let Ok(parsed) = serde_json::from_str::<serde_json::Value>(value) else {
        return false;
    };
    let haystack = parsed.to_string().to_ascii_lowercase();
    [
        "typing", "key", "enter", "command", "click", "scroll", "ax", "paste",
    ]
    .iter()
    .any(|needle| haystack.contains(needle))
}

fn has_any<'a>(values: impl IntoIterator<Item = Option<&'a str>>) -> bool {
    values
        .into_iter()
        .flatten()
        .any(|value| !value.trim().is_empty())
}

fn push_string_once(values: &mut Vec<String>, value: &str) {
    if !values.iter().any(|existing| existing == value) {
        values.push(value.to_string());
    }
}

pub fn product_safe_missing_evidence(raw_missing: &[String]) -> Vec<String> {
    let mut safe = raw_missing
        .iter()
        .map(|value| product_safe_missing_evidence_label(value))
        .collect::<Vec<_>>();
    sort_dedup(&mut safe);
    safe
}

pub fn product_safe_missing_evidence_label(value: &str) -> String {
    match value {
        "repo_root_missing" | "workspace_missing" | "project_identity_missing" => {
            "Workspace or repository identity was not visible."
        }
        "thread_identity_missing" | "thread_identity_uncertain" => {
            "Exact Codex thread was not visible."
        }
        "active_file_missing" => "Active file could not be identified.",
        "command_state_missing" | "command_signature_missing" => {
            "Terminal command state was not clear."
        }
        "focused_control_missing" => "Focused control was not available.",
        "fresh_heavy_frame_missing" | "missing_fresh_heavy_frame_for_current_focus" => {
            "Latest surface was event-backed without a fresh screenshot."
        }
        "openable_target_missing" | "missing_current_work_openable_target" => {
            "There is no safe exact target to open."
        }
        "privacy_blocked_text" => "Privacy rules blocked some visible evidence.",
        "missing_current_work_target_identity" | "missing_current_work_thread_or_document_id" => {
            "Exact current work target was not visible."
        }
        _ => "Local evidence is incomplete.",
    }
    .to_string()
}

fn sanitize_surface_enrichment_attempt(
    attempt: &SurfaceEnrichmentAttempt,
) -> Result<SurfaceEnrichmentAttempt, String> {
    let mut sanitized = attempt.clone();
    sanitized.domain = normalize_surface_domain(&sanitized.domain);
    sanitized.trigger_event_ids_json =
        sanitize_json_array_string(Some(&sanitized.trigger_event_ids_json))?
            .unwrap_or_else(|| "[]".to_string());
    sanitized.missing_fields_json =
        sanitize_json_array_string(Some(&sanitized.missing_fields_json))?
            .unwrap_or_else(|| "[]".to_string());
    sanitized.window_title =
        cap_optional(sanitized.window_title.as_deref(), MAX_WINDOW_TITLE_CHARS);
    sanitized.window_title_hash = sanitized.window_title_hash.or_else(|| {
        sanitized
            .window_title
            .as_deref()
            .map(|value| stable_hash_bytes(value.as_bytes()))
    });
    Ok(sanitized)
}

fn sanitize_surface_snapshot(snapshot: &SurfaceSnapshot) -> Result<SurfaceSnapshot, String> {
    let mut sanitized = snapshot.clone();
    sanitized.domain = normalize_surface_domain(&sanitized.domain);
    sanitized.adapter_version = if sanitized.adapter_version.trim().is_empty() {
        SURFACE_SNAPSHOT_ADAPTER_VERSION.to_string()
    } else {
        sanitized.adapter_version.trim().chars().take(80).collect()
    };
    sanitized.window_title =
        cap_optional(sanitized.window_title.as_deref(), MAX_WINDOW_TITLE_CHARS);
    sanitized.window_title_hash = sanitized.window_title_hash.or_else(|| {
        sanitized
            .window_title
            .as_deref()
            .map(|value| stable_hash_bytes(value.as_bytes()))
    });
    sanitized.thread_title =
        cap_optional(sanitized.thread_title.as_deref(), MAX_THREAD_TITLE_CHARS);
    sanitized.active_relative_file = cap_optional(
        sanitized.active_relative_file.as_deref(),
        MAX_RELATIVE_FILE_CHARS,
    );
    sanitized.focused_control_label = cap_optional(
        sanitized.focused_control_label.as_deref(),
        MAX_FOCUSED_CONTROL_LABEL_CHARS,
    );
    sanitized.visible_text_sample = cap_optional(
        sanitized.visible_text_sample.as_deref(),
        MAX_VISIBLE_TEXT_SAMPLE_CHARS,
    );
    sanitized.visible_text_hash = sanitized.visible_text_hash.or_else(|| {
        sanitized
            .visible_text_sample
            .as_deref()
            .map(|value| stable_hash_bytes(value.as_bytes()))
    });
    sanitized.workspace_path_hash = sanitized.workspace_path_hash.or_else(|| {
        sanitized
            .workspace_path
            .as_deref()
            .map(|value| stable_hash_bytes(value.as_bytes()))
    });
    sanitized.repo_root_hash = sanitized.repo_root_hash.or_else(|| {
        sanitized
            .repo_root_path
            .as_deref()
            .map(|value| stable_hash_bytes(value.as_bytes()))
    });
    sanitized.git_worktree_hash = sanitized.git_worktree_hash.or_else(|| {
        sanitized
            .git_worktree_path
            .as_deref()
            .map(|value| stable_hash_bytes(value.as_bytes()))
    });
    sanitized.active_file_path_hash = sanitized.active_file_path_hash.or_else(|| {
        sanitized
            .active_file_path
            .as_deref()
            .map(|value| stable_hash_bytes(value.as_bytes()))
    });
    sanitized.thread_key_hash = sanitized.thread_key_hash.or_else(|| {
        sanitized
            .thread_title
            .as_deref()
            .map(|value| stable_hash_bytes(value.as_bytes()))
    });
    sanitized.event_ids_json = sanitize_json_array_string(sanitized.event_ids_json.as_deref())?;
    sanitized.error_markers_json =
        sanitize_json_array_string(sanitized.error_markers_json.as_deref())?;
    sanitized.activity_signals_json =
        sanitize_json_string(sanitized.activity_signals_json.as_deref())?;
    sanitized.missing_fields_json =
        sanitize_json_array_string(sanitized.missing_fields_json.as_deref())?;
    sanitized.evidence_sources_json =
        sanitize_json_array_string(Some(&sanitized.evidence_sources_json))?
            .unwrap_or_else(|| "[]".to_string());
    sanitized.redaction_notes_json =
        sanitize_json_array_string(sanitized.redaction_notes_json.as_deref())?;
    sanitized.identity_confidence = normalize_one_of(
        &sanitized.identity_confidence,
        &["strong", "medium", "thin", "unknown"],
    );
    sanitized.evidence_quality = normalize_one_of(
        &sanitized.evidence_quality,
        &["strong", "medium", "thin", "unknown"],
    );
    sanitized.openability = normalize_one_of(
        &sanitized.openability,
        &[
            "openable",
            "app_focus_only",
            "frame_fallback",
            "blocked",
            "unknown",
        ],
    );
    sanitized.activity_state = normalize_optional_one_of(
        sanitized.activity_state.as_deref(),
        &[
            "actively_editing",
            "composing_prompt",
            "running_command",
            "observing_output",
            "reviewing_diff",
            "encountering_error",
            "idle_after_progress",
            "reading",
            "unknown",
        ],
    );
    sanitized.task_state = normalize_optional_one_of(
        sanitized.task_state.as_deref(),
        &[
            "draft_or_composer_active",
            "visible_error_unresolved",
            "command_running",
            "command_completed_with_output",
            "reviewing_changes",
            "editing_file",
            "asking_agent",
            "waiting_for_agent",
            "unknown",
        ],
    );
    sanitized.command_state = normalize_optional_one_of(
        sanitized.command_state.as_deref(),
        &[
            "not_terminal",
            "prompt_ready",
            "command_running",
            "command_completed",
            "command_failed",
            "unknown",
        ],
    );
    if privacy_tier(sanitized.privacy_status.as_deref()) == "blocked" {
        sanitized.visible_text_sample = None;
        sanitized.focused_control_label = None;
        sanitized.thread_title = None;
        sanitized.selected_text_hash = None;
        sanitized.focused_text_hash = None;
    }
    apply_surface_snapshot_quality(&mut sanitized)?;
    Ok(sanitized)
}

fn surface_snapshot_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<SurfaceSnapshot> {
    Ok(SurfaceSnapshot {
        id: row.get(0)?,
        session_id: row.get(1)?,
        surface_key: row.get(2)?,
        domain: row.get(3)?,
        adapter_key: row.get(4)?,
        adapter_version: row.get(5)?,
        observed_at_ms: row.get(6)?,
        frame_id: row.get(7)?,
        event_ids_json: row.get(8)?,
        artifact_id: row.get(9)?,
        app_name: row.get(10)?,
        bundle_id: row.get(11)?,
        app_pid: row.get(12)?,
        window_id: row.get(13)?,
        window_title: row.get(14)?,
        window_title_hash: row.get(15)?,
        workspace_path: row.get(16)?,
        workspace_path_hash: row.get(17)?,
        repo_root_path: row.get(18)?,
        repo_root_hash: row.get(19)?,
        git_branch: row.get(20)?,
        git_worktree_path: row.get(21)?,
        git_worktree_hash: row.get(22)?,
        thread_title: row.get(23)?,
        thread_key_hash: row.get(24)?,
        active_file_path: row.get(25)?,
        active_file_path_hash: row.get(26)?,
        active_relative_file: row.get(27)?,
        focused_control_role: row.get(28)?,
        focused_control_label: row.get(29)?,
        selected_text_hash: row.get(30)?,
        focused_text_hash: row.get(31)?,
        visible_text_sample: row.get(32)?,
        visible_text_hash: row.get(33)?,
        activity_state: row.get(34)?,
        task_state: row.get(35)?,
        command_state: row.get(36)?,
        error_markers_json: row.get(37)?,
        activity_signals_json: row.get(38)?,
        identity_confidence: row.get(39)?,
        evidence_quality: row.get(40)?,
        openability: row.get(41)?,
        privacy_status: row.get(42)?,
        missing_fields_json: row.get(43)?,
        evidence_sources_json: row.get(44)?,
        redaction_notes_json: row.get(45)?,
        created_at_ms: row.get(46)?,
        updated_at_ms: row.get(47)?,
    })
}

pub fn build_enrichment_attempt(
    input: WeakSurfaceEnrichmentPolicyInput,
) -> WeakSurfaceEnrichmentAttempt {
    let attempt_index = input
        .recent_attempts_30s
        .min(RETRY_DELAYS_MS.len() as i64 - 1);
    let scheduled_at_ms = input
        .now_ms
        .saturating_add(RETRY_DELAYS_MS[attempt_index as usize]);
    let missing_fields = missing_fields_for_attempt(
        input.app_name.as_deref(),
        input.bundle_id.as_deref(),
        input.window_title.as_deref(),
    );
    let (status, reason, snapshot_id) = if input.privacy_blocked {
        (
            EnrichmentAttemptStatus::SkippedPrivacy,
            Some("privacy_blocked".to_string()),
            None,
        )
    } else if input.self_capture {
        (
            EnrichmentAttemptStatus::SkippedSelfCapture,
            Some("smalltalk_self_focus".to_string()),
            None,
        )
    } else if input.focus_changed {
        (
            EnrichmentAttemptStatus::SkippedFocusChanged,
            Some("focus_changed_before_retry".to_string()),
            None,
        )
    } else if input.recent_strong_snapshot {
        (
            EnrichmentAttemptStatus::SkippedRecentStrongSnapshot,
            Some("recent_strong_snapshot".to_string()),
            None,
        )
    } else if input.recent_enrichment {
        (
            EnrichmentAttemptStatus::SkippedRecentStrongSnapshot,
            Some("recent_weak_surface_enrichment".to_string()),
            None,
        )
    } else if input.recent_attempts_30s >= MAX_ATTEMPTS_PER_SURFACE_PER_30S
        || input.recent_attempts_5m >= MAX_ATTEMPTS_PER_SURFACE_PER_5M
    {
        (
            EnrichmentAttemptStatus::SkippedBudget,
            Some("weak_surface_attempt_budget_exhausted".to_string()),
            None,
        )
    } else if missing_fields.len() >= 3 {
        (
            EnrichmentAttemptStatus::FailedNoTextOrIdentity,
            Some("no_app_window_identity".to_string()),
            None,
        )
    } else {
        let quality = event_only_snapshot_quality(
            input.window_title.as_deref(),
            input.trigger_event_ids.len(),
            &input.classification,
        );
        let status = match quality {
            "strong" => EnrichmentAttemptStatus::SucceededStrong,
            "medium" => EnrichmentAttemptStatus::SucceededMedium,
            _ => EnrichmentAttemptStatus::SucceededThin,
        };
        (
            status,
            Some("event_only_weak_surface_snapshot".to_string()),
            Some(enrichment_snapshot_id(
                &input.surface_key,
                input.observed_at_ms,
                &input.trigger_event_ids,
            )),
        )
    };
    WeakSurfaceEnrichmentAttempt {
        attempt_id: enrichment_attempt_id(
            &input.surface_key,
            input.now_ms,
            attempt_index,
            &input.trigger_event_ids,
        ),
        observed_at_ms: input.observed_at_ms,
        scheduled_at_ms,
        completed_at_ms: Some(input.now_ms),
        surface_key: input.surface_key,
        weak_domain: input.classification.domain,
        app_name: input.app_name,
        bundle_id: input.bundle_id,
        window_title_hash: input
            .window_title
            .as_deref()
            .map(|value| stable_hash_bytes(value.as_bytes())),
        window_title_capped: input.window_title.as_deref().and_then(capped_title),
        window_id: input.window_id,
        trigger_event_ids: input.trigger_event_ids,
        trigger_type: input.trigger_type,
        attempt_index,
        status,
        reason,
        snapshot_id,
        missing_fields,
        adapter_key: input.classification.adapter_key,
    }
}

#[derive(Debug)]
struct LatestUiEventForEnrichment {
    id: String,
    ts_ms: i64,
    key_category: Option<String>,
    app_name: Option<String>,
    bundle_id: Option<String>,
    window_title: Option<String>,
    window_id: Option<i64>,
}

fn latest_enrichable_event(
    conn: &Connection,
    session_id: Option<&str>,
) -> Result<Option<LatestUiEventForEnrichment>, String> {
    if !table_exists(conn, "ui_events")? {
        return Ok(None);
    }
    let has_key_category = column_exists(conn, "ui_events", "key_category")?;
    let has_app_name = column_exists(conn, "ui_events", "app_name")?;
    let has_bundle_id = column_exists(conn, "ui_events", "app_bundle_id")?;
    let has_window_title = column_exists(conn, "ui_events", "window_title")?;
    let has_window_id = column_exists(conn, "ui_events", "window_id")?;
    let key_category_expr = if has_key_category {
        "key_category"
    } else {
        "NULL"
    };
    let app_name_expr = if has_app_name { "app_name" } else { "NULL" };
    let bundle_id_expr = if has_bundle_id {
        "app_bundle_id"
    } else {
        "NULL"
    };
    let window_title_expr = if has_window_title {
        "window_title"
    } else {
        "NULL"
    };
    let window_id_expr = if has_window_id { "window_id" } else { "NULL" };
    let sql = match session_id {
        Some(_) => format!(
            "SELECT id, ts_ms, event_type, {}, {}, {}, {}, {}
             FROM ui_events
             WHERE session_id = ?1
             ORDER BY ts_ms DESC, id DESC
             LIMIT 80",
            key_category_expr, app_name_expr, bundle_id_expr, window_title_expr, window_id_expr
        ),
        None => format!(
            "SELECT id, ts_ms, event_type, {}, {}, {}, {}, {}
             FROM ui_events
             ORDER BY ts_ms DESC, id DESC
             LIMIT 80",
            key_category_expr, app_name_expr, bundle_id_expr, window_title_expr, window_id_expr
        ),
    };
    let mut stmt = conn.prepare(&sql).map_err(to_string)?;
    let map_row = |row: &rusqlite::Row<'_>| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, i64>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, Option<String>>(3)?,
            row.get::<_, Option<String>>(4)?,
            row.get::<_, Option<String>>(5)?,
            row.get::<_, Option<String>>(6)?,
            row.get::<_, Option<i64>>(7)?,
        ))
    };
    let rows = if let Some(session_id) = session_id {
        stmt.query_map(params![session_id], map_row)
            .map_err(to_string)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(to_string)?
    } else {
        stmt.query_map([], map_row)
            .map_err(to_string)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(to_string)?
    };
    for (id, ts_ms, event_type, key_category, app_name, bundle_id, window_title, window_id) in rows
    {
        if !trigger_is_enrichment_eligible(&event_type) {
            continue;
        }
        let classification = classify_weak_surface(&WeakSurfaceClassificationInput {
            app_name: app_name.clone(),
            bundle_id: bundle_id.clone(),
            window_title: window_title.clone(),
            event_types: vec![key_category
                .as_ref()
                .map(|category| format!("{}:{}", event_type, category))
                .unwrap_or_else(|| event_type.clone())],
            trigger_type: Some(event_type),
            ..Default::default()
        });
        if classification.domain == WeakSurfaceDomain::NotWeakSurface
            || is_smalltalk_self_surface(app_name.as_deref(), bundle_id.as_deref())
        {
            continue;
        }
        return Ok(Some(LatestUiEventForEnrichment {
            id,
            ts_ms,
            key_category,
            app_name,
            bundle_id,
            window_title,
            window_id,
        }));
    }
    Ok(None)
}

fn trigger_is_enrichment_eligible(event_type: &str) -> bool {
    matches!(
        event_type,
        "app_switch"
            | "window_focus"
            | "accessibility_change"
            | "ax_notification"
            | "typing_pause"
            | "key_down"
            | "scroll_stop"
            | "scroll"
            | "clipboard"
            | "manual"
    )
}

fn persist_enrichment_attempt(
    conn: &Connection,
    attempt: &WeakSurfaceEnrichmentAttempt,
) -> Result<(), String> {
    let weak_domain = serde_json::to_string(&attempt.weak_domain)
        .map_err(to_string)?
        .trim_matches('"')
        .to_string();
    let trigger_event_ids_json =
        serde_json::to_string(&attempt.trigger_event_ids).map_err(to_string)?;
    let missing_fields_json = serde_json::to_string(&attempt.missing_fields).map_err(to_string)?;
    conn.execute(
        "INSERT OR REPLACE INTO continue_weak_surface_enrichment_attempts (
            attempt_id, observed_at_ms, scheduled_at_ms, completed_at_ms, surface_key,
            weak_domain, app_name, bundle_id, window_title_hash, window_title_capped,
            window_id, trigger_event_ids_json, trigger_type, attempt_index, status,
            reason, snapshot_id, missing_fields_json, adapter_key, policy_version
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12,
                   ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20)",
        params![
            attempt.attempt_id.as_str(),
            attempt.observed_at_ms,
            attempt.scheduled_at_ms,
            attempt.completed_at_ms,
            attempt.surface_key.as_str(),
            weak_domain,
            attempt.app_name.as_deref(),
            attempt.bundle_id.as_deref(),
            attempt.window_title_hash.as_deref(),
            attempt.window_title_capped.as_deref(),
            attempt.window_id,
            trigger_event_ids_json,
            attempt.trigger_type.as_str(),
            attempt.attempt_index,
            attempt.status.as_str(),
            attempt.reason.as_deref(),
            attempt.snapshot_id.as_deref(),
            missing_fields_json,
            attempt.adapter_key.as_deref(),
            ENRICHMENT_POLICY_VERSION,
        ],
    )
    .map_err(to_string)?;
    Ok(())
}

fn latest_enrichment_attempt(
    conn: &Connection,
) -> Result<Option<WeakSurfaceEnrichmentAttempt>, String> {
    conn.query_row(
        "SELECT attempt_id, observed_at_ms, scheduled_at_ms, completed_at_ms, surface_key,
                weak_domain, app_name, bundle_id, window_title_hash, window_title_capped,
                window_id, trigger_event_ids_json, trigger_type, attempt_index, status,
                reason, snapshot_id, missing_fields_json, adapter_key
         FROM continue_weak_surface_enrichment_attempts
         ORDER BY COALESCE(completed_at_ms, scheduled_at_ms) DESC, attempt_id DESC
         LIMIT 1",
        [],
        attempt_from_row,
    )
    .optional()
    .map_err(to_string)
}

fn attempt_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<WeakSurfaceEnrichmentAttempt> {
    let domain: String = row.get(5)?;
    let status: String = row.get(14)?;
    let trigger_event_ids_json: String = row.get(11)?;
    let missing_fields_json: String = row.get(17)?;
    Ok(WeakSurfaceEnrichmentAttempt {
        attempt_id: row.get(0)?,
        observed_at_ms: row.get(1)?,
        scheduled_at_ms: row.get(2)?,
        completed_at_ms: row.get(3)?,
        surface_key: row.get(4)?,
        weak_domain: parse_weak_domain(&domain),
        app_name: row.get(6)?,
        bundle_id: row.get(7)?,
        window_title_hash: row.get(8)?,
        window_title_capped: row.get(9)?,
        window_id: row.get(10)?,
        trigger_event_ids: serde_json::from_str(&trigger_event_ids_json).unwrap_or_default(),
        trigger_type: row.get(12)?,
        attempt_index: row.get(13)?,
        status: parse_attempt_status(&status),
        reason: row.get(15)?,
        snapshot_id: row.get(16)?,
        missing_fields: serde_json::from_str(&missing_fields_json).unwrap_or_default(),
        adapter_key: row.get(18)?,
    })
}

fn update_enrichment_counters(
    conn: &Connection,
    attempt: &WeakSurfaceEnrichmentAttempt,
) -> Result<(), String> {
    increment_counter(conn, "weak_surface_enrichment_attempts", 1)?;
    match attempt.status.counter_suffix() {
        "attempts" => {}
        suffix => {
            increment_counter(conn, &format!("weak_surface_enrichment_{}", suffix), 1)?;
        }
    }
    if let Some(snapshot_id) = attempt.snapshot_id.as_ref() {
        record_maintenance_value(conn, "latest_weak_surface_snapshot_id", snapshot_id)?;
    }
    record_maintenance_value(conn, "latest_weak_surface_attempt", &attempt.attempt_id)?;
    Ok(())
}

fn count_attempts_since(
    conn: &Connection,
    surface_key: &str,
    cutoff_ms: i64,
) -> Result<i64, String> {
    ensure_weak_surface_enrichment_schema(conn)?;
    conn.query_row(
        "SELECT COUNT(*)
         FROM continue_weak_surface_enrichment_attempts
         WHERE surface_key = ?1
           AND scheduled_at_ms >= ?2",
        params![surface_key, cutoff_ms],
        |row| row.get(0),
    )
    .map_err(to_string)
}

fn has_recent_strong_snapshot(
    conn: &Connection,
    surface_key: &str,
    now_ms: i64,
) -> Result<bool, String> {
    if !table_exists(conn, "continue_artifacts")? {
        return Ok(false);
    }
    conn.query_row(
        "SELECT EXISTS(
            SELECT 1
            FROM continue_artifacts
            WHERE stable_key = ?1
              AND evidence_quality = 'strong'
              AND last_seen_timestamp >= ?2
         )",
        params![
            surface_key,
            now_ms.saturating_sub(RECENT_STRONG_SNAPSHOT_WINDOW_MS)
        ],
        |row| row.get::<_, i64>(0),
    )
    .map(|value| value != 0)
    .map_err(to_string)
}

fn has_recent_enrichment(
    conn: &Connection,
    surface_key: &str,
    now_ms: i64,
) -> Result<bool, String> {
    conn.query_row(
        "SELECT EXISTS(
            SELECT 1
            FROM continue_weak_surface_enrichment_attempts
            WHERE surface_key = ?1
              AND status IN ('succeeded_strong', 'succeeded_medium', 'succeeded_thin')
              AND COALESCE(completed_at_ms, scheduled_at_ms) >= ?2
         )",
        params![
            surface_key,
            now_ms.saturating_sub(RECENT_ENRICHMENT_WINDOW_MS)
        ],
        |row| row.get::<_, i64>(0),
    )
    .map(|value| value != 0)
    .map_err(to_string)
}

fn count_statuses(conn: &Connection, status: Option<&str>) -> Result<i64, String> {
    match status {
        Some(status) => conn
            .query_row(
                "SELECT COUNT(*) FROM continue_weak_surface_enrichment_attempts WHERE status = ?1",
                params![status],
                |row| row.get(0),
            )
            .map_err(to_string),
        None => conn
            .query_row(
                "SELECT COUNT(*) FROM continue_weak_surface_enrichment_attempts",
                [],
                |row| row.get(0),
            )
            .map_err(to_string),
    }
}

fn enrichment_surface_key(
    app_name: Option<&str>,
    bundle_id: Option<&str>,
    window_title: Option<&str>,
    domain: &WeakSurfaceDomain,
) -> String {
    enrichment_surface_key_with_window_id(app_name, bundle_id, window_title, None, domain)
}

fn enrichment_surface_key_with_window_id(
    app_name: Option<&str>,
    bundle_id: Option<&str>,
    window_title: Option<&str>,
    window_id: Option<i64>,
    domain: &WeakSurfaceDomain,
) -> String {
    let owner = normalize_key_component(bundle_id)
        .or_else(|| normalize_key_component(app_name))
        .unwrap_or_else(|| "unknown_app".to_string());
    let domain = weak_domain_label(domain);
    let identity = if bundle_id.is_some() {
        window_id
            .map(|id| format!("window_id_{}", id))
            .or_else(|| {
                window_title
                    .map(normalize_window_title_for_key)
                    .filter(|title| !title.is_empty())
                    .map(|title| format!("title_{}", stable_hash_bytes(title.as_bytes())))
            })
            .unwrap_or_else(|| {
                format!(
                    "event_bucket_{}",
                    stable_hash_bytes(format!("{}:{}", owner, domain).as_bytes())
                )
            })
    } else {
        window_title
            .map(normalize_window_title_for_key)
            .filter(|title| !title.is_empty())
            .map(|title| format!("title_{}", stable_hash_bytes(title.as_bytes())))
            .unwrap_or_else(|| {
                format!(
                    "event_bucket_{}",
                    stable_hash_bytes(format!("{}:{}", owner, domain).as_bytes())
                )
            })
    };
    let base = format!("event-surface:{}:{}:{}", domain, owner, identity);
    format!("{}:{}", base, stable_hash_bytes(base.as_bytes()))
}

fn normalize_window_title_for_key(value: &str) -> String {
    value
        .trim()
        .to_ascii_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(MAX_WINDOW_TITLE_CHARS)
        .collect()
}

fn enrichment_attempt_id(
    surface_key: &str,
    now_ms: i64,
    attempt_index: i64,
    event_ids: &[String],
) -> String {
    format!(
        "weak-enrich-{}",
        stable_hash_bytes(
            format!(
                "{}:{}:{}:{}",
                surface_key,
                now_ms,
                attempt_index,
                event_ids.join(",")
            )
            .as_bytes()
        )
    )
}

fn enrichment_snapshot_id(surface_key: &str, observed_at_ms: i64, event_ids: &[String]) -> String {
    format!(
        "event-surface-{}",
        stable_hash_bytes(
            format!("{}:{}:{}", surface_key, observed_at_ms, event_ids.join(",")).as_bytes()
        )
    )
}

fn event_only_snapshot_quality(
    window_title: Option<&str>,
    event_count: usize,
    classification: &WeakSurfaceClassification,
) -> &'static str {
    if classification.confidence >= 0.88 && window_title.map(human_title).unwrap_or(false) {
        "medium"
    } else if event_count >= 3 && window_title.map(human_title).unwrap_or(false) {
        "medium"
    } else {
        "thin"
    }
}

fn missing_fields_for_attempt(
    app_name: Option<&str>,
    bundle_id: Option<&str>,
    window_title: Option<&str>,
) -> Vec<String> {
    let mut missing = Vec::new();
    if app_name.map(str::trim).unwrap_or("").is_empty() {
        missing.push("app_name".to_string());
    }
    if bundle_id.map(str::trim).unwrap_or("").is_empty() {
        missing.push("bundle_id".to_string());
    }
    if window_title.map(str::trim).unwrap_or("").is_empty() {
        missing.push("window_title".to_string());
    }
    missing.push("fresh_heavy_frame_missing".to_string());
    missing
}

fn capped_title(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.chars().take(160).collect())
    }
}

fn human_title(value: &str) -> bool {
    let trimmed = value.trim();
    trimmed.chars().count() >= 3 && trimmed.chars().any(|ch| ch.is_ascii_alphabetic())
}

fn is_smalltalk_self_surface(app_name: Option<&str>, bundle_id: Option<&str>) -> bool {
    bundle_id
        .map(|value| value.eq_ignore_ascii_case("com.smalltalk.capture"))
        .unwrap_or(false)
        || app_name
            .map(|value| value.to_ascii_lowercase().contains("smalltalk"))
            .unwrap_or(false)
}

fn parse_weak_domain(value: &str) -> WeakSurfaceDomain {
    match value {
        "codex_cli" => WeakSurfaceDomain::CodexCli,
        "codex_desktop_app" => WeakSurfaceDomain::CodexDesktopApp,
        "codex_ide_extension" => WeakSurfaceDomain::CodexIdeExtension,
        "code_editor" => WeakSurfaceDomain::CodeEditor,
        "terminal" => WeakSurfaceDomain::Terminal,
        "native_agent_window" => WeakSurfaceDomain::NativeAgentWindow,
        "not_weak_surface" => WeakSurfaceDomain::NotWeakSurface,
        _ => WeakSurfaceDomain::UnknownWeakSurface,
    }
}

fn parse_attempt_status(value: &str) -> EnrichmentAttemptStatus {
    match value {
        "scheduled" => EnrichmentAttemptStatus::Scheduled,
        "running" => EnrichmentAttemptStatus::Running,
        "succeeded_strong" => EnrichmentAttemptStatus::SucceededStrong,
        "succeeded_medium" => EnrichmentAttemptStatus::SucceededMedium,
        "succeeded_thin" => EnrichmentAttemptStatus::SucceededThin,
        "skipped_recent_strong_snapshot" => EnrichmentAttemptStatus::SkippedRecentStrongSnapshot,
        "skipped_privacy" => EnrichmentAttemptStatus::SkippedPrivacy,
        "skipped_budget" => EnrichmentAttemptStatus::SkippedBudget,
        "skipped_self_capture" => EnrichmentAttemptStatus::SkippedSelfCapture,
        "skipped_focus_changed" => EnrichmentAttemptStatus::SkippedFocusChanged,
        "failed_helper" => EnrichmentAttemptStatus::FailedHelper,
        "failed_no_text_or_identity" => EnrichmentAttemptStatus::FailedNoTextOrIdentity,
        _ => EnrichmentAttemptStatus::FailedNoTextOrIdentity,
    }
}

fn weak_domain_label(domain: &WeakSurfaceDomain) -> String {
    serde_json::to_string(domain)
        .unwrap_or_else(|_| "\"unknown_weak_surface\"".to_string())
        .trim_matches('"')
        .to_string()
}

fn normalize_surface_domain(value: &str) -> String {
    weak_domain_label(&parse_weak_domain(value))
}

fn clean_string(value: Option<&str>, max_chars: usize) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.chars().take(max_chars).collect())
}

fn cap_optional(value: Option<&str>, max_chars: usize) -> Option<String> {
    clean_string(value, max_chars)
}

fn normalize_one_of(value: &str, allowed: &[&str]) -> String {
    let normalized = value.trim().to_ascii_lowercase();
    if allowed.iter().any(|allowed| *allowed == normalized) {
        normalized
    } else {
        "unknown".to_string()
    }
}

fn normalize_optional_one_of(value: Option<&str>, allowed: &[&str]) -> Option<String> {
    value.map(|value| normalize_one_of(value, allowed))
}

fn bounded_json_array(values: &[String]) -> Result<String, String> {
    bounded_json_value(&serde_json::json!(values))
}

fn bounded_json_value(value: &serde_json::Value) -> Result<String, String> {
    let raw = serde_json::to_string(value).map_err(to_string)?;
    Ok(raw.chars().take(MAX_JSON_FIELD_CHARS).collect())
}

fn sanitize_json_string(value: Option<&str>) -> Result<Option<String>, String> {
    let Some(value) = value else {
        return Ok(None);
    };
    if serde_json::from_str::<serde_json::Value>(value).is_err() {
        return Ok(Some("{}".to_string()));
    }
    Ok(Some(value.chars().take(MAX_JSON_FIELD_CHARS).collect()))
}

fn sanitize_json_array_string(value: Option<&str>) -> Result<Option<String>, String> {
    let Some(value) = value else {
        return Ok(None);
    };
    match serde_json::from_str::<serde_json::Value>(value) {
        Ok(serde_json::Value::Array(_)) => {
            Ok(Some(value.chars().take(MAX_JSON_FIELD_CHARS).collect()))
        }
        Ok(_) => Ok(Some("[]".to_string())),
        Err(_) => Ok(Some("[]".to_string())),
    }
}

fn normalize_key_component(input: Option<&str>) -> Option<String> {
    let normalized = input?
        .trim()
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>()
        .split('_')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("_");
    (!normalized.is_empty()).then_some(normalized)
}

fn stable_hash_bytes(bytes: &[u8]) -> String {
    let mut hasher = DefaultHasher::new();
    bytes.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

fn increment_counter(conn: &Connection, key: &str, delta: i64) -> Result<(), String> {
    let current = conn
        .query_row(
            "SELECT value FROM local_memory_maintenance WHERE key = ?1",
            params![key],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(to_string)?
        .and_then(|value| value.parse::<i64>().ok())
        .unwrap_or(0);
    record_maintenance_value(conn, key, &current.saturating_add(delta).to_string())
}

fn record_maintenance_value(conn: &Connection, key: &str, value: &str) -> Result<(), String> {
    conn.execute(
        "INSERT INTO local_memory_maintenance (key, value, updated_at_ms)
         VALUES (?1, ?2, ?3)
         ON CONFLICT(key) DO UPDATE SET
           value = excluded.value,
           updated_at_ms = excluded.updated_at_ms",
        params![key, value, current_time_millis()],
    )
    .map_err(to_string)?;
    Ok(())
}

fn table_exists(conn: &Connection, table: &str) -> Result<bool, String> {
    conn.query_row(
        "SELECT EXISTS(
            SELECT 1 FROM sqlite_master WHERE type IN ('table', 'view') AND name = ?1
         )",
        params![table],
        |row| row.get::<_, i64>(0),
    )
    .map(|exists| exists != 0)
    .map_err(to_string)
}

fn column_exists(conn: &Connection, table: &str, column: &str) -> Result<bool, String> {
    let mut stmt = conn
        .prepare(&format!("PRAGMA table_info({})", table))
        .map_err(to_string)?;
    let rows = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(to_string)?;
    for row in rows {
        if row.map_err(to_string)?.eq_ignore_ascii_case(column) {
            return Ok(true);
        }
    }
    Ok(false)
}

fn current_time_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0)
}

fn to_string<E: std::fmt::Display>(error: E) -> String {
    error.to_string()
}

pub fn classify_weak_surface(input: &WeakSurfaceClassificationInput) -> WeakSurfaceClassification {
    let privacy_tier = privacy_tier(input.privacy_status.as_deref());
    let mut reasons = Vec::new();
    if privacy_tier == "blocked" {
        push_reason(&mut reasons, "privacy_blocked");
        return classification(
            WeakSurfaceDomain::UnknownWeakSurface,
            EnrichmentNeed::BlockedByPrivacy,
            0.0,
            reasons,
            None,
            privacy_tier,
            input,
        );
    }

    let app_blob = normalized_blob([
        input.app_name.as_deref(),
        input.bundle_id.as_deref(),
        input.window_title.as_deref(),
    ]);
    let evidence_blob = normalized_blob([
        input.window_title.as_deref(),
        input.full_text_sample.as_deref(),
        input.text_source.as_deref(),
        input.focused_node_role.as_deref(),
    ]);
    let all_blob = normalized_blob([
        input.app_name.as_deref(),
        input.bundle_id.as_deref(),
        input.window_title.as_deref(),
        input.browser_url.as_deref(),
        input.document_path.as_deref(),
        input.full_text_sample.as_deref(),
        input.focused_node_role.as_deref(),
        input.trigger_type.as_deref(),
    ]);

    let terminal_family = is_terminal_family(&app_blob);
    let editor_family = is_editor_family(&app_blob);
    let browser_surface = is_browser_surface(&app_blob, input.browser_url.as_deref());
    let codex_marker = has_codex_marker(&evidence_blob) || has_codex_marker(&app_blob);
    let recent_activity = has_recent_activity(input);

    if terminal_family {
        push_reason(&mut reasons, "app_terminal");
        if codex_marker {
            if has_codex_marker(&app_blob) {
                push_reason(&mut reasons, "app_mentions_codex");
            }
            if has_codex_marker(&normalized_blob([input.window_title.as_deref()])) {
                push_reason(&mut reasons, "window_mentions_codex");
            }
            if has_codex_marker(&normalized_blob([input.full_text_sample.as_deref()])) {
                push_reason(&mut reasons, "content_mentions_codex");
            }
            return classification(
                WeakSurfaceDomain::CodexCli,
                EnrichmentNeed::Targeted,
                0.9,
                reasons,
                Some("codex_cli_adapter".to_string()),
                privacy_tier,
                input,
            );
        }
        push_activity_reasons(&mut reasons, input);
        return classification(
            WeakSurfaceDomain::Terminal,
            if terminal_needs_targeted(input) {
                EnrichmentNeed::Targeted
            } else {
                EnrichmentNeed::Light
            },
            if recent_activity { 0.74 } else { 0.62 },
            reasons,
            Some("terminal_adapter".to_string()),
            privacy_tier,
            input,
        );
    }

    if editor_family {
        push_reason(&mut reasons, "app_is_editor_family");
        if codex_marker {
            if has_codex_marker(&normalized_blob([input.window_title.as_deref()])) {
                push_reason(&mut reasons, "window_mentions_codex");
            }
            if has_codex_marker(&normalized_blob([input.full_text_sample.as_deref()])) {
                push_reason(&mut reasons, "content_mentions_codex");
            }
            return classification(
                WeakSurfaceDomain::CodexIdeExtension,
                EnrichmentNeed::Targeted,
                0.86,
                reasons,
                Some("codex_ide_extension_adapter".to_string()),
                privacy_tier,
                input,
            );
        }
        if input.document_path.is_some() {
            push_reason(&mut reasons, "document_path_has_source_extension");
        }
        if input
            .content_unit_roles
            .iter()
            .any(|role| role_label(role).contains("code"))
        {
            push_reason(&mut reasons, "content_role_code_editor");
        }
        push_activity_reasons(&mut reasons, input);
        return classification(
            WeakSurfaceDomain::CodeEditor,
            if code_editor_needs_targeted(input) {
                EnrichmentNeed::Targeted
            } else {
                EnrichmentNeed::Light
            },
            if recent_activity || input.document_path.is_some() {
                0.78
            } else {
                0.66
            },
            reasons,
            Some("code_editor_adapter".to_string()),
            privacy_tier,
            input,
        );
    }

    if is_codex_desktop_app(&app_blob) {
        push_reason(&mut reasons, "app_is_codex_desktop");
        return classification(
            WeakSurfaceDomain::CodexDesktopApp,
            EnrichmentNeed::Targeted,
            0.92,
            reasons,
            Some("codex_desktop_adapter".to_string()),
            privacy_tier,
            input,
        );
    }

    if browser_surface {
        push_reason(&mut reasons, "browser_surface");
        return classification(
            WeakSurfaceDomain::NotWeakSurface,
            EnrichmentNeed::None,
            0.86,
            reasons,
            None,
            privacy_tier,
            input,
        );
    }

    if strong_codex_desktop_evidence(&app_blob, &evidence_blob) {
        push_reason(&mut reasons, "window_content_indicates_codex_app");
        return classification(
            WeakSurfaceDomain::CodexDesktopApp,
            EnrichmentNeed::Targeted,
            0.82,
            reasons,
            Some("codex_desktop_adapter".to_string()),
            privacy_tier,
            input,
        );
    }

    if is_native_agent_window(&all_blob) {
        if codex_marker {
            push_reason(&mut reasons, "possible_codex_app");
        } else {
            push_reason(&mut reasons, "ai_agent_window_marker");
        }
        return classification(
            WeakSurfaceDomain::NativeAgentWindow,
            EnrichmentNeed::Targeted,
            if codex_marker { 0.68 } else { 0.62 },
            reasons,
            Some("native_agent_adapter".to_string()),
            privacy_tier,
            input,
        );
    }

    if unknown_weak_surface(input, &all_blob) {
        push_reason(&mut reasons, "low_text_custom_surface");
        push_activity_reasons(&mut reasons, input);
        return classification(
            WeakSurfaceDomain::UnknownWeakSurface,
            if recent_activity {
                EnrichmentNeed::Retry
            } else {
                EnrichmentNeed::Light
            },
            if recent_activity { 0.5 } else { 0.38 },
            reasons,
            Some("unknown_weak_surface_adapter".to_string()),
            privacy_tier,
            input,
        );
    }

    classification(
        WeakSurfaceDomain::NotWeakSurface,
        EnrichmentNeed::None,
        0.72,
        vec!["not_weak_surface".to_string()],
        None,
        privacy_tier,
        input,
    )
}

pub fn domain_artifact_kind(domain: &WeakSurfaceDomain) -> &'static str {
    match domain {
        WeakSurfaceDomain::CodexCli => "terminal",
        WeakSurfaceDomain::CodexDesktopApp
        | WeakSurfaceDomain::CodexIdeExtension
        | WeakSurfaceDomain::CodeEditor => "code_editor",
        WeakSurfaceDomain::Terminal => "terminal",
        WeakSurfaceDomain::NativeAgentWindow => "unknown",
        WeakSurfaceDomain::UnknownWeakSurface | WeakSurfaceDomain::NotWeakSurface => "unknown",
    }
}

pub fn domain_app_family(domain: &WeakSurfaceDomain) -> Option<&'static str> {
    match domain {
        WeakSurfaceDomain::CodexCli
        | WeakSurfaceDomain::CodexDesktopApp
        | WeakSurfaceDomain::CodexIdeExtension
        | WeakSurfaceDomain::NativeAgentWindow => Some("ai_coding_agent"),
        WeakSurfaceDomain::CodeEditor => Some("code_editor"),
        WeakSurfaceDomain::Terminal => Some("terminal"),
        WeakSurfaceDomain::UnknownWeakSurface | WeakSurfaceDomain::NotWeakSurface => None,
    }
}

fn classification(
    domain: WeakSurfaceDomain,
    enrichment_need: EnrichmentNeed,
    confidence: f64,
    mut reasons: Vec<String>,
    adapter_key: Option<String>,
    privacy_tier: String,
    input: &WeakSurfaceClassificationInput,
) -> WeakSurfaceClassification {
    reasons.sort();
    reasons.dedup();
    WeakSurfaceClassification {
        domain,
        enrichment_need,
        confidence: round_confidence(confidence),
        reasons,
        adapter_key,
        privacy_tier,
        observed_app_name: clean_observed(input.app_name.as_deref()),
        observed_bundle_id: clean_observed(input.bundle_id.as_deref()),
        observed_window_title: clean_observed(input.window_title.as_deref()),
    }
}

fn privacy_tier(status: Option<&str>) -> String {
    let normalized = status.unwrap_or("").trim().to_ascii_lowercase();
    if normalized.contains("exclude")
        || normalized.contains("blocked")
        || normalized.contains("privacy_skip")
        || normalized.contains("private")
    {
        "blocked".to_string()
    } else if normalized.contains("redact") {
        "redacted".to_string()
    } else {
        "standard".to_string()
    }
}

fn normalized_blob<'a>(parts: impl IntoIterator<Item = Option<&'a str>>) -> String {
    parts
        .into_iter()
        .flatten()
        .map(|part| {
            part.chars()
                .map(|ch| if ch.is_whitespace() { ' ' } else { ch })
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase()
}

fn is_terminal_family(blob: &str) -> bool {
    contains_any(
        blob,
        &[
            "terminal",
            "iterm",
            "iterm2",
            "warp",
            "ghostty",
            "alacritty",
            "kitty",
            "wezterm",
        ],
    )
}

fn is_editor_family(blob: &str) -> bool {
    contains_any(
        blob,
        &[
            "visual studio code",
            "vs code",
            "vscode",
            "com.microsoft.vscode",
            "cursor",
            "windsurf",
            "jetbrains",
            "intellij",
            "webstorm",
            "pycharm",
            "rustrover",
            "xcode",
            "zed",
            "sublime",
        ],
    )
}

fn is_browser_surface(blob: &str, browser_url: Option<&str>) -> bool {
    browser_url.is_some()
        || contains_any(
            blob,
            &[
                "safari",
                "chrome",
                "chromium",
                "arc",
                "brave",
                "firefox",
                "helium",
                "browser",
                "chatgpt.com",
            ],
        )
}

fn has_codex_marker(blob: &str) -> bool {
    contains_any(
        blob,
        &[
            "codex",
            "openai codex",
            "codex cli",
            "ask codex",
            "codex panel",
            "codex thread",
            "review with codex",
        ],
    )
}

fn is_codex_desktop_app(blob: &str) -> bool {
    contains_any(blob, &["com.openai.codex", "openai codex desktop"])
        || blob.split_whitespace().any(|token| token == "codex")
}

fn strong_codex_desktop_evidence(app_blob: &str, evidence_blob: &str) -> bool {
    has_codex_marker(evidence_blob)
        && contains_any(app_blob, &["openai", "desktop", "codex"])
        && !is_terminal_family(app_blob)
        && !is_editor_family(app_blob)
}

fn is_native_agent_window(blob: &str) -> bool {
    let agent_marker = contains_any(
        blob,
        &[
            "ai agent",
            "coding agent",
            "assistant",
            "apply patch",
            "run command",
            "diff",
            "plan",
            "compose",
            "claude code",
        ],
    );
    agent_marker && !contains_any(blob, &["chatgpt.com", "mail", "messages", "slack"])
}

fn unknown_weak_surface(input: &WeakSurfaceClassificationInput, blob: &str) -> bool {
    if is_browser_surface(blob, input.browser_url.as_deref())
        || input.document_path.is_some()
        || contains_any(
            blob,
            &[
                "slack", "messages", "discord", "whatsapp", "notion", "notes",
            ],
        )
    {
        return false;
    }
    let text_len = input
        .full_text_sample
        .as_deref()
        .map(|text| text.trim().chars().count())
        .unwrap_or(0);
    text_len < 24 && has_recent_activity(input)
}

fn has_recent_activity(input: &WeakSurfaceClassificationInput) -> bool {
    input.event_types.iter().any(|event| {
        let event = event.to_ascii_lowercase();
        contains_any(
            &event,
            &[
                "typing", "key", "enter", "return", "command", "output", "error", "paste", "click",
            ],
        )
    }) || input.trigger_type.as_deref().is_some_and(|trigger| {
        contains_any(
            &trigger.to_ascii_lowercase(),
            &["typing", "event", "command", "enter"],
        )
    })
}

fn terminal_needs_targeted(input: &WeakSurfaceClassificationInput) -> bool {
    input.event_types.iter().any(|event| {
        contains_any(
            &event.to_ascii_lowercase(),
            &["typing", "enter", "return", "command", "output", "error"],
        )
    })
}

fn code_editor_needs_targeted(input: &WeakSurfaceClassificationInput) -> bool {
    input.document_path.is_some()
        || input.event_types.iter().any(|event| {
            contains_any(
                &event.to_ascii_lowercase(),
                &["typing", "key", "paste", "save", "edit"],
            )
        })
}

fn push_activity_reasons(reasons: &mut Vec<String>, input: &WeakSurfaceClassificationInput) {
    for event in &input.event_types {
        let event = event.to_ascii_lowercase();
        if contains_any(&event, &["typing", "key", "paste"]) {
            push_reason(reasons, "recent_typing_activity");
        }
        if contains_any(&event, &["enter", "return", "command"]) {
            push_reason(reasons, "recent_command_activity");
        }
        if contains_any(&event, &["output"]) {
            push_reason(reasons, "recent_output_activity");
        }
        if contains_any(&event, &["error"]) {
            push_reason(reasons, "recent_error_activity");
        }
    }
}

fn role_label(role: &str) -> String {
    role.trim().to_ascii_lowercase()
}

fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| haystack.contains(needle))
}

fn push_reason(reasons: &mut Vec<String>, reason: &str) {
    if !reasons.iter().any(|existing| existing == reason) {
        reasons.push(reason.to_string());
    }
}

fn clean_observed(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.chars().take(160).collect::<String>())
}

fn round_confidence(value: f64) -> f64 {
    (value.clamp(0.0, 1.0) * 100.0).round() / 100.0
}

pub fn enrich_surface(input: &SurfaceEnrichmentInput) -> SurfaceEnrichmentOutput {
    match input.classification.domain {
        WeakSurfaceDomain::CodexCli => CodexCliAdapter.enrich(input),
        WeakSurfaceDomain::CodexDesktopApp => CodexDesktopAdapter.enrich(input),
        WeakSurfaceDomain::CodexIdeExtension => CodexIdeExtensionAdapter.enrich(input),
        WeakSurfaceDomain::CodeEditor => CodeEditorAdapter.enrich(input),
        WeakSurfaceDomain::Terminal => TerminalAdapter.enrich(input),
        WeakSurfaceDomain::NativeAgentWindow => NativeAgentAdapter.enrich(input),
        WeakSurfaceDomain::UnknownWeakSurface => UnknownWeakSurfaceAdapter.enrich(input),
        WeakSurfaceDomain::NotWeakSurface => SurfaceEnrichmentOutput {
            snapshot: None,
            status: EnrichmentAttemptStatus::FailedNoTextOrIdentity,
            missing_fields: vec!["adapter_uncertain".to_string()],
            warnings: vec!["surface_not_weak".to_string()],
        },
    }
}

struct CodexCliAdapter;
struct CodexDesktopAdapter;
struct CodexIdeExtensionAdapter;
struct CodeEditorAdapter;
struct TerminalAdapter;
struct NativeAgentAdapter;
struct UnknownWeakSurfaceAdapter;

impl SurfaceEnrichmentAdapter for CodexCliAdapter {
    fn key(&self) -> &'static str {
        "codex_cli_adapter"
    }

    fn domain(&self) -> WeakSurfaceDomain {
        WeakSurfaceDomain::CodexCli
    }

    fn enrich(&self, input: &SurfaceEnrichmentInput) -> SurfaceEnrichmentOutput {
        build_adapter_output(input, self.key(), self.domain(), AdapterProfile::CodexCli)
    }
}

impl SurfaceEnrichmentAdapter for CodexDesktopAdapter {
    fn key(&self) -> &'static str {
        "codex_desktop_adapter"
    }

    fn domain(&self) -> WeakSurfaceDomain {
        WeakSurfaceDomain::CodexDesktopApp
    }

    fn enrich(&self, input: &SurfaceEnrichmentInput) -> SurfaceEnrichmentOutput {
        build_adapter_output(
            input,
            self.key(),
            self.domain(),
            AdapterProfile::CodexDesktop,
        )
    }
}

impl SurfaceEnrichmentAdapter for CodexIdeExtensionAdapter {
    fn key(&self) -> &'static str {
        "codex_ide_extension_adapter"
    }

    fn domain(&self) -> WeakSurfaceDomain {
        WeakSurfaceDomain::CodexIdeExtension
    }

    fn enrich(&self, input: &SurfaceEnrichmentInput) -> SurfaceEnrichmentOutput {
        build_adapter_output(input, self.key(), self.domain(), AdapterProfile::CodexIde)
    }
}

impl SurfaceEnrichmentAdapter for CodeEditorAdapter {
    fn key(&self) -> &'static str {
        "code_editor_adapter"
    }

    fn domain(&self) -> WeakSurfaceDomain {
        WeakSurfaceDomain::CodeEditor
    }

    fn enrich(&self, input: &SurfaceEnrichmentInput) -> SurfaceEnrichmentOutput {
        build_adapter_output(input, self.key(), self.domain(), AdapterProfile::CodeEditor)
    }
}

impl SurfaceEnrichmentAdapter for TerminalAdapter {
    fn key(&self) -> &'static str {
        "terminal_adapter"
    }

    fn domain(&self) -> WeakSurfaceDomain {
        WeakSurfaceDomain::Terminal
    }

    fn enrich(&self, input: &SurfaceEnrichmentInput) -> SurfaceEnrichmentOutput {
        build_adapter_output(input, self.key(), self.domain(), AdapterProfile::Terminal)
    }
}

impl SurfaceEnrichmentAdapter for NativeAgentAdapter {
    fn key(&self) -> &'static str {
        "native_agent_adapter"
    }

    fn domain(&self) -> WeakSurfaceDomain {
        WeakSurfaceDomain::NativeAgentWindow
    }

    fn enrich(&self, input: &SurfaceEnrichmentInput) -> SurfaceEnrichmentOutput {
        build_adapter_output(
            input,
            self.key(),
            self.domain(),
            AdapterProfile::NativeAgent,
        )
    }
}

impl SurfaceEnrichmentAdapter for UnknownWeakSurfaceAdapter {
    fn key(&self) -> &'static str {
        "unknown_weak_surface_adapter"
    }

    fn domain(&self) -> WeakSurfaceDomain {
        WeakSurfaceDomain::UnknownWeakSurface
    }

    fn enrich(&self, input: &SurfaceEnrichmentInput) -> SurfaceEnrichmentOutput {
        build_adapter_output(input, self.key(), self.domain(), AdapterProfile::Unknown)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AdapterProfile {
    CodexCli,
    CodexDesktop,
    CodexIde,
    CodeEditor,
    Terminal,
    NativeAgent,
    Unknown,
}

fn build_adapter_output(
    input: &SurfaceEnrichmentInput,
    adapter_key: &str,
    domain: WeakSurfaceDomain,
    profile: AdapterProfile,
) -> SurfaceEnrichmentOutput {
    let privacy = input_privacy_status(input);
    if privacy_tier(privacy.as_deref()) == "blocked" {
        return SurfaceEnrichmentOutput {
            snapshot: None,
            status: EnrichmentAttemptStatus::SkippedPrivacy,
            missing_fields: vec!["privacy_blocked_text".to_string()],
            warnings: vec!["privacy_blocked_surface_not_enriched".to_string()],
        };
    }

    let text_sources = visible_text_sources(input);
    let visible_sample = extract_visible_main_content_sample(&text_sources);
    let error_markers = find_error_markers(visible_sample.as_deref().unwrap_or(""));
    let composer_markers = find_composer_markers(input, visible_sample.as_deref());
    let terminal_markers = find_terminal_markers(input, visible_sample.as_deref());
    let codex_markers = find_codex_markers(input, visible_sample.as_deref());
    let code_editor_markers = find_code_editor_markers(input, visible_sample.as_deref());
    let focused = extract_focused_control(input);
    let selected_text_hash = extract_selected_text_hash(input);
    let active_file_path = infer_active_file_path(input);
    let workspace_path = infer_workspace_path(input, active_file_path.as_deref());
    let repo_root_path = infer_repo_root_path(input, workspace_path.as_deref());
    let active_relative_file = active_file_path
        .as_deref()
        .and_then(|path| relative_file_from_repo(path, repo_root_path.as_deref()))
        .or_else(|| {
            active_file_path
                .as_deref()
                .and_then(safe_display_path_basename)
        })
        .or_else(|| infer_relative_file_from_title(input));
    let thread_title = infer_thread_title(input, visible_sample.as_deref(), profile);
    let window_title = normalize_window_title(active_window_title(input).as_deref());
    let app_name = active_app_name(input);
    let bundle_id = active_bundle_id(input);
    let window_id = active_window_id(input);
    let app_pid = input
        .frame
        .as_ref()
        .and_then(|frame| frame.app_pid)
        .or_else(|| {
            input
                .window_snapshot
                .as_ref()
                .and_then(|snapshot| snapshot.active_app_pid)
        });
    let event_ids = input
        .recent_events
        .iter()
        .map(|event| event.id.clone())
        .filter(|id| !id.trim().is_empty())
        .collect::<Vec<_>>();

    let mut missing_fields = missing_fields_for_adapter(
        profile,
        workspace_path.as_deref(),
        repo_root_path.as_deref(),
        thread_title.as_deref(),
        active_file_path.as_deref(),
        focused.as_ref(),
        visible_sample.as_deref(),
    );
    let mut warnings = Vec::new();
    if profile == AdapterProfile::CodexDesktop && thread_title.is_none() {
        missing_fields.push("thread_identity_uncertain".to_string());
    }
    if profile == AdapterProfile::Unknown {
        missing_fields.push("adapter_uncertain".to_string());
    }
    if active_file_path.is_none()
        && matches!(
            profile,
            AdapterProfile::CodeEditor | AdapterProfile::CodexIde
        )
    {
        missing_fields.push("openable_target_missing".to_string());
    }
    sort_dedup(&mut missing_fields);

    let activity_state = infer_activity_state(
        input,
        profile,
        !composer_markers.is_empty(),
        !terminal_markers.is_empty(),
        !code_editor_markers.is_empty(),
        !error_markers.is_empty(),
    );
    let task_state = infer_task_state(
        profile,
        active_file_path.is_some(),
        !composer_markers.is_empty(),
        !error_markers.is_empty(),
    );
    let command_state = infer_command_state(
        profile,
        !terminal_markers.is_empty(),
        !error_markers.is_empty(),
        visible_sample.as_deref(),
    );
    let identity_confidence = adapter_identity_confidence(
        profile,
        workspace_path.is_some() || repo_root_path.is_some(),
        active_file_path.is_some(),
        thread_title.is_some(),
        visible_sample.is_some(),
    );
    let openability = adapter_openability(
        profile,
        active_file_path.as_deref(),
        input.frame.as_ref().and_then(|frame| frame.id),
    );
    if openability != "openable" {
        warnings.push("exact_open_target_not_available".to_string());
    }

    let surface_key = enrichment_surface_key_with_window_id(
        app_name.as_deref(),
        bundle_id.as_deref(),
        window_title.as_deref(),
        window_id,
        &domain,
    );
    let snapshot_id = format!(
        "surface-snapshot-{}",
        hash_sensitive_value(
            "snapshot",
            &format!("{}:{}", surface_key, input.observed_at_ms)
        )
    );
    let evidence_sources = evidence_sources_for_input(input);
    let visible_hash = visible_sample
        .as_deref()
        .map(|value| hash_sensitive_value("visible_text", value));
    let mut snapshot = SurfaceSnapshot {
        id: snapshot_id,
        session_id: clean_string(input.session_id.as_deref(), 160),
        surface_key,
        domain: weak_domain_label(&domain),
        adapter_key: Some(adapter_key.to_string()),
        adapter_version: SURFACE_SNAPSHOT_ADAPTER_VERSION.to_string(),
        observed_at_ms: input.observed_at_ms,
        frame_id: input.frame.as_ref().and_then(|frame| frame.id),
        event_ids_json: Some(bounded_json_array(&event_ids).unwrap_or_else(|_| "[]".to_string())),
        artifact_id: None,
        app_name,
        bundle_id,
        app_pid,
        window_id,
        window_title: window_title.clone(),
        window_title_hash: window_title
            .as_deref()
            .map(|value| hash_sensitive_value("window_title", value)),
        workspace_path,
        workspace_path_hash: repo_root_path_or_workspace_hash(input, "workspace_path"),
        repo_root_path,
        repo_root_hash: repo_root_path_or_workspace_hash(input, "repo_root_path"),
        git_branch: infer_git_branch(input),
        git_worktree_path: None,
        git_worktree_hash: None,
        thread_title: thread_title.clone(),
        thread_key_hash: thread_title
            .as_deref()
            .map(|value| hash_sensitive_value("thread", value)),
        active_file_path: active_file_path.clone(),
        active_file_path_hash: active_file_path
            .as_deref()
            .map(|value| hash_sensitive_value("active_file", value)),
        active_relative_file,
        focused_control_role: focused.as_ref().and_then(|control| control.role.clone()),
        focused_control_label: focused.and_then(|control| control.label),
        selected_text_hash,
        focused_text_hash: None,
        visible_text_sample: visible_sample.clone(),
        visible_text_hash: visible_hash,
        activity_state: Some(activity_state.to_string()),
        task_state: Some(task_state.to_string()),
        command_state: Some(command_state.to_string()),
        error_markers_json: Some(
            bounded_json_array(&error_markers).unwrap_or_else(|_| "[]".to_string()),
        ),
        activity_signals_json: Some(
            summarize_activity_signals(
                input,
                &composer_markers,
                &terminal_markers,
                &codex_markers,
                &code_editor_markers,
            )
            .unwrap_or_else(|_| "{}".to_string()),
        ),
        identity_confidence: identity_confidence.to_string(),
        evidence_quality: identity_confidence.to_string(),
        openability: openability.to_string(),
        privacy_status: privacy,
        missing_fields_json: Some(
            bounded_json_array(&missing_fields).unwrap_or_else(|_| "[]".to_string()),
        ),
        evidence_sources_json: bounded_json_array(&evidence_sources)
            .unwrap_or_else(|_| "[]".to_string()),
        redaction_notes_json: Some(
            bounded_json_array(&vec![
                "visible_text_redacted_and_capped".to_string(),
                "selected_text_hashed_only".to_string(),
                "clipboard_metadata_only".to_string(),
            ])
            .unwrap_or_else(|_| "[]".to_string()),
        ),
        created_at_ms: input.observed_at_ms,
        updated_at_ms: input.observed_at_ms,
    };
    if let Err(error) = apply_surface_snapshot_quality(&mut snapshot) {
        warnings.push(format!("surface_snapshot_quality_failed:{}", error));
    }
    let status = status_for_identity_quality(&snapshot.evidence_quality);

    SurfaceEnrichmentOutput {
        snapshot: Some(snapshot),
        status,
        missing_fields,
        warnings,
    }
}

#[derive(Debug, Clone)]
struct FocusedControl {
    role: Option<String>,
    label: Option<String>,
}

fn normalize_window_title(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.split_whitespace().collect::<Vec<_>>().join(" "))
        .map(|value| value.chars().take(MAX_WINDOW_TITLE_CHARS).collect())
}

fn redact_and_cap_visible_sample(value: &str) -> Option<String> {
    let redacted = value
        .lines()
        .map(|line| redact_sensitive_line(line.trim()))
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    let redacted = redacted.trim();
    if redacted.is_empty() {
        None
    } else {
        Some(
            redacted
                .chars()
                .take(MAX_VISIBLE_TEXT_SAMPLE_CHARS)
                .collect(),
        )
    }
}

fn redact_sensitive_line(line: &str) -> String {
    line.split_whitespace()
        .map(|token| {
            let lower = token.to_ascii_lowercase();
            if lower.starts_with("sk-")
                || lower.contains("token=")
                || lower.contains("api_key")
                || lower.contains("apikey")
                || lower.contains("password")
                || lower.contains("secret")
            {
                "[redacted]".to_string()
            } else {
                token.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn extract_focused_control(input: &SurfaceEnrichmentInput) -> Option<FocusedControl> {
    input
        .ax_nodes
        .iter()
        .find(|node| node.focused.unwrap_or(false))
        .map(|node| FocusedControl {
            role: node
                .role
                .as_deref()
                .and_then(|role| clean_string(Some(role), 120)),
            label: node
                .text
                .as_deref()
                .and_then(|text| redact_and_cap_visible_sample(text))
                .and_then(|text| clean_string(Some(&text), MAX_FOCUSED_CONTROL_LABEL_CHARS)),
        })
        .or_else(|| {
            input.app_contexts.iter().find_map(|context| {
                context
                    .focused_object
                    .as_deref()
                    .map(|focused| FocusedControl {
                        role: Some(context.object_type.clone()),
                        label: clean_string(Some(focused), MAX_FOCUSED_CONTROL_LABEL_CHARS),
                    })
            })
        })
}

fn extract_selected_text_hash(input: &SurfaceEnrichmentInput) -> Option<String> {
    input
        .ax_nodes
        .iter()
        .find_map(|node| node.selected_text.as_deref())
        .or_else(|| {
            input
                .app_contexts
                .iter()
                .find_map(|context| context.selected_text.as_deref())
        })
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| hash_sensitive_value("selected_text", value))
}

fn extract_visible_main_content_sample(sources: &[String]) -> Option<String> {
    let joined = sources
        .iter()
        .map(String::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .take(12)
        .collect::<Vec<_>>()
        .join("\n");
    redact_and_cap_visible_sample(&joined)
}

fn find_error_markers(text: &str) -> Vec<String> {
    let lower = text.to_ascii_lowercase();
    let mut markers = Vec::new();
    for (needle, marker) in [
        ("error:", "error"),
        ("failed", "failed"),
        ("failure", "failure"),
        ("panic", "panic"),
        ("traceback", "traceback"),
        ("exception", "exception"),
        ("test result: failed", "test_failure"),
        ("cargo check failed", "cargo_check_failed"),
        ("npm err", "npm_error"),
        ("permission denied", "permission_denied"),
    ] {
        if lower.contains(needle) {
            markers.push(marker.to_string());
        }
    }
    sort_dedup(&mut markers);
    markers
}

fn find_composer_markers(input: &SurfaceEnrichmentInput, sample: Option<&str>) -> Vec<String> {
    let blob = adapter_blob(input, sample);
    let mut markers = Vec::new();
    for (needle, marker) in [
        ("ask codex", "ask_codex"),
        ("message codex", "message_codex"),
        ("composer", "composer"),
        ("prompt", "prompt"),
        ("type a message", "message_input"),
        ("▌", "cursor_block"),
        ("> ", "prompt_caret"),
    ] {
        if blob.contains(needle) {
            markers.push(marker.to_string());
        }
    }
    if input.typing_bursts.iter().any(|burst| !burst.committed) {
        markers.push("uncommitted_typing_burst".to_string());
    }
    sort_dedup(&mut markers);
    markers
}

fn find_code_editor_markers(input: &SurfaceEnrichmentInput, sample: Option<&str>) -> Vec<String> {
    let blob = adapter_blob(input, sample);
    let mut markers = Vec::new();
    if infer_active_file_path(input).is_some() {
        markers.push("document_path".to_string());
    }
    if input.ax_nodes.iter().any(|node| {
        node.role
            .as_deref()
            .map(|role| role.to_ascii_lowercase().contains("text"))
            .unwrap_or(false)
            && node.focused.unwrap_or(false)
    }) {
        markers.push("focused_text_editor".to_string());
    }
    for (needle, marker) in [
        ("problems", "diagnostics_panel"),
        ("terminal", "integrated_terminal"),
        ("source control", "source_control"),
        (".rs", "source_file"),
        (".ts", "source_file"),
        (".tsx", "source_file"),
        ("function ", "code_text"),
        ("const ", "code_text"),
        ("impl ", "code_text"),
    ] {
        if blob.contains(needle) {
            markers.push(marker.to_string());
        }
    }
    sort_dedup(&mut markers);
    markers
}

fn find_terminal_markers(input: &SurfaceEnrichmentInput, sample: Option<&str>) -> Vec<String> {
    let blob = adapter_blob(input, sample);
    let mut markers = Vec::new();
    for (needle, marker) in [
        ("$ ", "shell_prompt"),
        ("❯", "shell_prompt"),
        ("➜", "shell_prompt"),
        ("cargo ", "cargo_command"),
        ("npm ", "npm_command"),
        ("git ", "git_command"),
        ("running", "running_output"),
        ("finished", "completed_output"),
        ("test result", "test_output"),
    ] {
        if blob.contains(needle) {
            markers.push(marker.to_string());
        }
    }
    if input
        .typing_bursts
        .iter()
        .any(|burst| burst.enter_count > 0)
    {
        markers.push("recent_enter".to_string());
    }
    sort_dedup(&mut markers);
    markers
}

fn find_codex_markers(input: &SurfaceEnrichmentInput, sample: Option<&str>) -> Vec<String> {
    let blob = adapter_blob(input, sample);
    let mut markers = Vec::new();
    for (needle, marker) in [
        ("codex", "codex"),
        ("apply_patch", "apply_patch"),
        ("apply patch", "apply_patch"),
        ("approval", "approval"),
        ("diff", "diff"),
        ("plan updated", "plan"),
        ("agent", "agent"),
    ] {
        if blob.contains(needle) {
            markers.push(marker.to_string());
        }
    }
    sort_dedup(&mut markers);
    markers
}

fn summarize_activity_signals(
    input: &SurfaceEnrichmentInput,
    composer_markers: &[String],
    terminal_markers: &[String],
    codex_markers: &[String],
    code_editor_markers: &[String],
) -> Result<String, String> {
    bounded_json_value(&serde_json::json!({
        "event_count": input.recent_events.len(),
        "event_types": input.recent_events.iter().map(|event| event.event_type.clone()).collect::<Vec<_>>(),
        "typing_burst_count": input.typing_bursts.len(),
        "clipboard_metadata_count": input.clipboard_metadata.len(),
        "composer_markers": composer_markers,
        "terminal_markers": terminal_markers,
        "codex_markers": codex_markers,
        "code_editor_markers": code_editor_markers
    }))
}

fn safe_display_path_basename(value: &str) -> Option<String> {
    Path::new(value)
        .file_name()
        .and_then(|name| name.to_str())
        .and_then(|name| clean_string(Some(name), MAX_RELATIVE_FILE_CHARS))
}

fn hash_sensitive_value(domain: &str, value: &str) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in format!("smalltalk:{}:{}", domain, value).as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{:016x}", hash)
}

fn visible_text_sources(input: &SurfaceEnrichmentInput) -> Vec<String> {
    let mut sources = Vec::new();
    // Prefer typed task-turn hints before any positional fallback. The durable
    // task-turn companion replaces this sample during Continue rebuild; this
    // adapter remains the bounded pre-rebuild path for thin/older databases.
    for unit in input.content_units.iter().rev().filter(|unit| {
        unit.semantic_role.as_deref().is_some_and(|role| {
            matches!(
                role,
                "user_message"
                    | "agent_message"
                    | "agent_status"
                    | "chat_message"
                    | "composer"
                    | "editor_content"
                    | "terminal_input"
            )
        })
    }) {
        if let Some(text) = unit
            .text
            .as_deref()
            .and_then(|text| clean_string(Some(text), 280))
        {
            push_unique(&mut sources, text);
        }
        if sources.len() >= 4 {
            break;
        }
    }
    for node in input
        .ax_nodes
        .iter()
        .filter(|node| node.focused.unwrap_or(false))
        .take(3)
    {
        if let Some(text) = node
            .text
            .as_deref()
            .and_then(|text| clean_string(Some(text), 280))
        {
            push_unique(&mut sources, text);
        }
    }
    for unit in input.content_units.iter().rev().take(8) {
        if let Some(text) = unit
            .text
            .as_deref()
            .and_then(|text| clean_string(Some(text), 280))
        {
            push_unique(&mut sources, text);
        }
    }
    for span in input.ocr_spans.iter().rev().take(6) {
        if let Some(text) = clean_string(Some(&span.text), 220) {
            push_unique(&mut sources, text);
        }
    }
    if sources.is_empty() {
        if let Some(text) = input
            .frame
            .as_ref()
            .and_then(|frame| frame.full_text.as_deref())
            .and_then(|text| clean_string(Some(text), 600))
        {
            sources.push(text);
        }
    }
    sources
}

fn push_unique(values: &mut Vec<String>, value: String) {
    if !values.contains(&value) {
        values.push(value);
    }
}

fn input_privacy_status(input: &SurfaceEnrichmentInput) -> Option<String> {
    input
        .frame
        .as_ref()
        .and_then(|frame| frame.privacy_status.clone())
        .or_else(|| {
            (input.classification.privacy_tier != "standard")
                .then(|| input.classification.privacy_tier.clone())
        })
}

fn active_app_name(input: &SurfaceEnrichmentInput) -> Option<String> {
    input
        .frame
        .as_ref()
        .and_then(|frame| frame.app_name.clone())
        .or_else(|| {
            input
                .window_snapshot
                .as_ref()
                .and_then(|snapshot| snapshot.active_app_name.clone())
        })
        .or_else(|| input.classification.observed_app_name.clone())
}

fn active_bundle_id(input: &SurfaceEnrichmentInput) -> Option<String> {
    input
        .frame
        .as_ref()
        .and_then(|frame| frame.bundle_id.clone())
        .or_else(|| {
            input
                .window_snapshot
                .as_ref()
                .and_then(|snapshot| snapshot.active_app_bundle_id.clone())
        })
        .or_else(|| input.classification.observed_bundle_id.clone())
}

fn active_window_title(input: &SurfaceEnrichmentInput) -> Option<String> {
    input
        .frame
        .as_ref()
        .and_then(|frame| frame.window_title.clone())
        .or_else(|| {
            input
                .window_snapshot
                .as_ref()
                .and_then(|snapshot| snapshot.active_window_title.clone())
        })
        .or_else(|| {
            input
                .app_contexts
                .iter()
                .find_map(|context| context.title.clone())
        })
        .or_else(|| input.classification.observed_window_title.clone())
}

fn active_window_id(input: &SurfaceEnrichmentInput) -> Option<i64> {
    input
        .frame
        .as_ref()
        .and_then(|frame| frame.window_id)
        .or_else(|| {
            input
                .window_snapshot
                .as_ref()
                .and_then(|snapshot| snapshot.active_window_id)
        })
}

fn infer_active_file_path(input: &SurfaceEnrichmentInput) -> Option<String> {
    input
        .frame
        .as_ref()
        .and_then(|frame| frame.document_path.clone())
        .or_else(|| {
            input
                .app_contexts
                .iter()
                .find_map(|context| context.file_path.clone())
        })
        .and_then(|path| clean_string(Some(&path), 800))
}

fn infer_workspace_path(
    input: &SurfaceEnrichmentInput,
    active_file_path: Option<&str>,
) -> Option<String> {
    active_file_path
        .and_then(|path| Path::new(path).parent())
        .and_then(|path| path.to_str())
        .and_then(|path| clean_string(Some(path), 800))
        .or_else(|| infer_path_from_window_title(active_window_title(input).as_deref()))
}

fn infer_repo_root_path(
    input: &SurfaceEnrichmentInput,
    workspace_path: Option<&str>,
) -> Option<String> {
    let active_file_path = infer_active_file_path(input);
    active_file_path
        .as_deref()
        .and_then(find_repo_root_from_known_path)
        .or_else(|| workspace_path.and_then(find_repo_root_from_known_path))
        .or_else(|| {
            infer_path_from_window_title(active_window_title(input).as_deref())
                .as_deref()
                .and_then(find_repo_root_from_known_path)
        })
}

fn find_repo_root_from_known_path(path: &str) -> Option<String> {
    let path = Path::new(path);
    let start = if path.is_file() { path.parent()? } else { path };
    let mut current: PathBuf = start.to_path_buf();
    for _ in 0..=12 {
        if current.join(".git").exists() {
            return current
                .to_str()
                .and_then(|value| clean_string(Some(value), 800));
        }
        if !current.pop() {
            break;
        }
    }
    None
}

fn relative_file_from_repo(path: &str, repo_root: Option<&str>) -> Option<String> {
    let repo_root = repo_root?;
    let file_path = Path::new(path);
    let repo_path = Path::new(repo_root);
    file_path
        .strip_prefix(repo_path)
        .ok()
        .and_then(|relative| relative.to_str())
        .and_then(|relative| clean_string(Some(relative), MAX_RELATIVE_FILE_CHARS))
}

fn infer_path_from_window_title(title: Option<&str>) -> Option<String> {
    title.and_then(|title| {
        title
            .split_whitespace()
            .find(|part| part.starts_with("/Users/") || part.starts_with("~/"))
            .and_then(|part| clean_string(Some(part.trim_matches(['"', '\'', ','])), 800))
    })
}

fn infer_relative_file_from_title(input: &SurfaceEnrichmentInput) -> Option<String> {
    active_window_title(input).and_then(|title| {
        title
            .split([' ', '—', '-', '|'])
            .map(str::trim)
            .find(|part| {
                [".rs", ".ts", ".tsx", ".js", ".jsx", ".py", ".md", ".swift"]
                    .iter()
                    .any(|ext| part.ends_with(ext))
            })
            .and_then(|part| clean_string(Some(part), MAX_RELATIVE_FILE_CHARS))
    })
}

fn infer_thread_title(
    input: &SurfaceEnrichmentInput,
    sample: Option<&str>,
    profile: AdapterProfile,
) -> Option<String> {
    let candidates = [
        sample.map(str::to_string),
        active_window_title(input),
        input
            .app_contexts
            .iter()
            .find_map(|context| context.title.clone()),
    ];
    for candidate in candidates.into_iter().flatten() {
        for line in candidate.lines() {
            let trimmed = line.trim();
            let lower = trimmed.to_ascii_lowercase();
            for prefix in ["thread:", "task:", "goal:", "chat:"] {
                if lower.starts_with(prefix) {
                    return clean_string(
                        Some(trimmed[prefix.len()..].trim()),
                        MAX_THREAD_TITLE_CHARS,
                    );
                }
            }
        }
    }
    if matches!(
        profile,
        AdapterProfile::CodexDesktop | AdapterProfile::CodexCli | AdapterProfile::NativeAgent
    ) {
        active_window_title(input).and_then(|title| {
            title
                .split(['—', '|'])
                .map(str::trim)
                .find(|part| {
                    !part.eq_ignore_ascii_case("codex")
                        && !part.eq_ignore_ascii_case("terminal")
                        && part.chars().count() > 3
                })
                .and_then(|part| clean_string(Some(part), MAX_THREAD_TITLE_CHARS))
        })
    } else {
        None
    }
}

fn infer_git_branch(input: &SurfaceEnrichmentInput) -> Option<String> {
    let blob = adapter_blob(input, None);
    for marker in ["branch:", "git branch"] {
        if let Some(index) = blob.find(marker) {
            return blob[index + marker.len()..]
                .split_whitespace()
                .next()
                .and_then(|value| clean_string(Some(value), 120));
        }
    }
    None
}

fn adapter_blob(input: &SurfaceEnrichmentInput, sample: Option<&str>) -> String {
    normalized_blob([
        active_app_name(input).as_deref(),
        active_bundle_id(input).as_deref(),
        active_window_title(input).as_deref(),
        input
            .frame
            .as_ref()
            .and_then(|frame| frame.text_source.as_deref()),
        sample,
    ])
}

fn missing_fields_for_adapter(
    profile: AdapterProfile,
    workspace_path: Option<&str>,
    repo_root_path: Option<&str>,
    thread_title: Option<&str>,
    active_file_path: Option<&str>,
    focused: Option<&FocusedControl>,
    visible_sample: Option<&str>,
) -> Vec<String> {
    let mut missing = Vec::new();
    if workspace_path.is_none() {
        missing.push("workspace_identity_missing".to_string());
    }
    if repo_root_path.is_none()
        && matches!(profile, AdapterProfile::CodexCli | AdapterProfile::Terminal)
    {
        missing.push("repo_root_missing".to_string());
    }
    if thread_title.is_none()
        && matches!(
            profile,
            AdapterProfile::CodexCli
                | AdapterProfile::CodexDesktop
                | AdapterProfile::CodexIde
                | AdapterProfile::NativeAgent
        )
    {
        missing.push("thread_identity_missing".to_string());
    }
    if active_file_path.is_none()
        && matches!(
            profile,
            AdapterProfile::CodexIde | AdapterProfile::CodeEditor
        )
    {
        missing.push("active_file_missing".to_string());
        missing.push("document_path_missing".to_string());
    }
    if focused.is_none() {
        missing.push("focused_control_missing".to_string());
    }
    if visible_sample.is_none() {
        missing.push("visible_text_missing".to_string());
    }
    missing
}

fn infer_activity_state(
    input: &SurfaceEnrichmentInput,
    profile: AdapterProfile,
    composer: bool,
    terminal: bool,
    code_editor: bool,
    error: bool,
) -> &'static str {
    if error {
        return "encountering_error";
    }
    if matches!(profile, AdapterProfile::CodeEditor)
        && (code_editor || input.typing_bursts.iter().any(|burst| !burst.committed))
    {
        return "actively_editing";
    }
    if composer || input.typing_bursts.iter().any(|burst| !burst.committed) {
        return "composing_prompt";
    }
    if matches!(
        profile,
        AdapterProfile::CodeEditor | AdapterProfile::CodexIde
    ) && (code_editor || input.typing_bursts.iter().any(|burst| burst.committed))
    {
        return "actively_editing";
    }
    if matches!(profile, AdapterProfile::Terminal | AdapterProfile::CodexCli) && terminal {
        if input
            .typing_bursts
            .iter()
            .any(|burst| burst.enter_count > 0)
        {
            "running_command"
        } else {
            "observing_output"
        }
    } else if matches!(
        profile,
        AdapterProfile::CodexDesktop | AdapterProfile::CodexIde | AdapterProfile::NativeAgent
    ) && adapter_blob(input, None).contains("diff")
    {
        "reviewing_diff"
    } else if code_editor {
        "reading"
    } else {
        "idle_after_progress"
    }
}

fn infer_task_state(
    profile: AdapterProfile,
    has_active_file: bool,
    composer: bool,
    error: bool,
) -> &'static str {
    if error {
        "visible_error_unresolved"
    } else if matches!(profile, AdapterProfile::CodeEditor) && has_active_file {
        "editing_file"
    } else if composer {
        "draft_or_composer_active"
    } else if matches!(
        profile,
        AdapterProfile::CodeEditor | AdapterProfile::CodexIde
    ) && has_active_file
    {
        "editing_file"
    } else if matches!(
        profile,
        AdapterProfile::CodexCli | AdapterProfile::CodexDesktop | AdapterProfile::NativeAgent
    ) {
        "asking_agent"
    } else {
        "unknown"
    }
}

fn infer_command_state(
    profile: AdapterProfile,
    terminal: bool,
    error: bool,
    sample: Option<&str>,
) -> &'static str {
    if !matches!(profile, AdapterProfile::Terminal | AdapterProfile::CodexCli) {
        return "not_terminal";
    }
    if error {
        "command_failed"
    } else if sample
        .map(|sample| {
            contains_any(
                &sample.to_ascii_lowercase(),
                &["finished", "test result: ok"],
            )
        })
        .unwrap_or(false)
    {
        "command_completed"
    } else if terminal {
        "command_running"
    } else {
        "prompt_ready"
    }
}

fn adapter_identity_confidence(
    profile: AdapterProfile,
    has_workspace: bool,
    has_file: bool,
    has_thread: bool,
    has_visible_sample: bool,
) -> &'static str {
    match profile {
        AdapterProfile::CodexIde if has_workspace && has_file && has_thread => "strong",
        AdapterProfile::CodeEditor if has_file && has_visible_sample => "strong",
        AdapterProfile::CodexDesktop if has_workspace && has_thread => "strong",
        AdapterProfile::CodexCli if has_workspace && has_thread => "medium",
        AdapterProfile::Terminal if has_workspace && has_visible_sample => "medium",
        AdapterProfile::NativeAgent if has_thread && has_visible_sample => "medium",
        _ if has_workspace || has_file || has_thread || has_visible_sample => "thin",
        _ => "thin",
    }
}

fn status_for_identity_quality(identity_confidence: &str) -> EnrichmentAttemptStatus {
    match identity_confidence {
        "strong" => EnrichmentAttemptStatus::SucceededStrong,
        "medium" => EnrichmentAttemptStatus::SucceededMedium,
        _ => EnrichmentAttemptStatus::SucceededThin,
    }
}

fn adapter_openability(
    profile: AdapterProfile,
    active_file_path: Option<&str>,
    frame_id: Option<i64>,
) -> &'static str {
    if active_file_path.is_some()
        && matches!(
            profile,
            AdapterProfile::CodeEditor | AdapterProfile::CodexIde
        )
    {
        "openable"
    } else if frame_id.is_some() {
        "frame_fallback"
    } else {
        "app_focus_only"
    }
}

fn evidence_sources_for_input(input: &SurfaceEnrichmentInput) -> Vec<String> {
    let mut sources = Vec::new();
    if input.frame.is_some() {
        sources.push("frame_app_window".to_string());
    }
    if input.window_snapshot.is_some() {
        sources.push("window_snapshot".to_string());
    }
    if input
        .ax_nodes
        .iter()
        .any(|node| node.focused.unwrap_or(false))
    {
        sources.push("focused_ax_node".to_string());
    }
    if !input.ax_nodes.is_empty() {
        sources.push("accessibility_nodes".to_string());
    }
    if !input.ocr_spans.is_empty() {
        sources.push("ocr_text".to_string());
    }
    if !input.content_units.is_empty() {
        sources.push("content_units".to_string());
    }
    if !input.recent_events.is_empty() {
        sources.push("ui_events".to_string());
    }
    if !input.typing_bursts.is_empty() {
        sources.push("typing_bursts".to_string());
    }
    if !input.clipboard_metadata.is_empty() {
        sources.push("clipboard_metadata".to_string());
    }
    if !input.app_contexts.is_empty() {
        sources.push("app_contexts".to_string());
    }
    if infer_active_file_path(input).is_some() {
        sources.push("document_path".to_string());
    }
    sort_dedup(&mut sources);
    sources
}

fn repo_root_path_or_workspace_hash(input: &SurfaceEnrichmentInput, kind: &str) -> Option<String> {
    let active_file_path = infer_active_file_path(input);
    let workspace_path = infer_workspace_path(input, active_file_path.as_deref());
    let repo_root_path = infer_repo_root_path(input, workspace_path.as_deref());
    match kind {
        "workspace_path" => workspace_path
            .as_deref()
            .map(|value| hash_sensitive_value("workspace_path", value)),
        "repo_root_path" => repo_root_path
            .as_deref()
            .map(|value| hash_sensitive_value("repo_root_path", value)),
        _ => None,
    }
}

fn sort_dedup(values: &mut Vec<String>) {
    values.sort();
    values.dedup();
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::{params, Connection};

    fn classify(input: WeakSurfaceClassificationInput) -> WeakSurfaceClassification {
        classify_weak_surface(&input)
    }

    fn policy_input(
        app_name: &str,
        window_title: Option<&str>,
        trigger_type: &str,
    ) -> WeakSurfaceEnrichmentPolicyInput {
        let classification = classify(WeakSurfaceClassificationInput {
            app_name: Some(app_name.to_string()),
            window_title: window_title.map(str::to_string),
            event_types: vec![trigger_type.to_string()],
            trigger_type: Some(trigger_type.to_string()),
            ..Default::default()
        });
        WeakSurfaceEnrichmentPolicyInput {
            now_ms: 10_000,
            observed_at_ms: 9_900,
            trigger_type: trigger_type.to_string(),
            surface_key: enrichment_surface_key(
                Some(app_name),
                None,
                window_title,
                &classification.domain,
            ),
            classification,
            app_name: Some(app_name.to_string()),
            bundle_id: None,
            window_title: window_title.map(str::to_string),
            window_id: Some(42),
            trigger_event_ids: vec!["evt-1".to_string()],
            recent_attempts_30s: 0,
            recent_attempts_5m: 0,
            recent_strong_snapshot: false,
            recent_enrichment: false,
            privacy_blocked: false,
            self_capture: false,
            focus_changed: false,
            force_attempt: false,
        }
    }

    fn test_surface_snapshot(id: &str, surface_key: &str, observed_at_ms: i64) -> SurfaceSnapshot {
        SurfaceSnapshot {
            id: id.to_string(),
            session_id: Some("session-a".to_string()),
            surface_key: surface_key.to_string(),
            domain: "terminal".to_string(),
            adapter_key: Some("terminal_adapter".to_string()),
            adapter_version: SURFACE_SNAPSHOT_ADAPTER_VERSION.to_string(),
            observed_at_ms,
            frame_id: None,
            event_ids_json: Some("[\"evt-1\"]".to_string()),
            artifact_id: None,
            app_name: Some("Terminal".to_string()),
            bundle_id: Some("com.apple.Terminal".to_string()),
            app_pid: None,
            window_id: Some(42),
            window_title: Some("codex - smalltalk".to_string()),
            window_title_hash: None,
            workspace_path: None,
            workspace_path_hash: None,
            repo_root_path: None,
            repo_root_hash: None,
            git_branch: None,
            git_worktree_path: None,
            git_worktree_hash: None,
            thread_title: None,
            thread_key_hash: None,
            active_file_path: None,
            active_file_path_hash: None,
            active_relative_file: None,
            focused_control_role: None,
            focused_control_label: None,
            selected_text_hash: None,
            focused_text_hash: None,
            visible_text_sample: None,
            visible_text_hash: None,
            activity_state: Some("actively_editing".to_string()),
            task_state: Some("editing_file".to_string()),
            command_state: Some("unknown".to_string()),
            error_markers_json: Some("[]".to_string()),
            activity_signals_json: Some("{\"trigger_type\":\"typing_pause\"}".to_string()),
            identity_confidence: "medium".to_string(),
            evidence_quality: "medium".to_string(),
            openability: "app_focus_only".to_string(),
            privacy_status: None,
            missing_fields_json: Some("[\"fresh_heavy_frame_missing\"]".to_string()),
            evidence_sources_json: "[{\"kind\":\"ui_event\",\"ids\":[\"evt-1\"]}]".to_string(),
            redaction_notes_json: Some("[\"test\"]".to_string()),
            created_at_ms: observed_at_ms,
            updated_at_ms: observed_at_ms,
        }
    }

    #[test]
    fn quality_codex_cli_repo_composer_typing_is_strong_and_eligible() {
        let mut snapshot = test_surface_snapshot("snap-codex-cli", "surface-codex-cli", 10_000);
        snapshot.domain = "codex_cli".to_string();
        snapshot.repo_root_path = Some("/Users/me/smalltalk".to_string());
        snapshot.repo_root_hash = Some("repo-hash".to_string());
        snapshot.thread_title = Some("P3 evidence quality".to_string());
        snapshot.thread_key_hash = Some("thread-hash".to_string());
        snapshot.visible_text_sample = Some("Ask Codex to implement the quality model".to_string());
        snapshot.focused_control_role = Some("AXTextArea".to_string());
        snapshot.activity_state = Some("composing_prompt".to_string());
        snapshot.task_state = Some("draft_or_composer_active".to_string());
        snapshot.command_state = Some("prompt_ready".to_string());

        let quality = evaluate_surface_snapshot_quality(&snapshot);

        assert_eq!(quality.evidence_quality, EvidenceQuality::Strong);
        assert!(quality.candidate_eligible);
        assert_eq!(
            quality.stale_target_suppression_strength,
            StaleSuppressionStrength::Strong
        );
    }

    #[test]
    fn quality_codex_cli_repo_without_thread_is_medium_and_thread_missing() {
        let mut snapshot =
            test_surface_snapshot("snap-codex-cli-medium", "surface-codex-cli", 10_000);
        snapshot.domain = "codex_cli".to_string();
        snapshot.repo_root_hash = Some("repo-hash".to_string());
        snapshot.thread_title = None;
        snapshot.thread_key_hash = None;
        snapshot.visible_text_sample = Some("Ask Codex to continue".to_string());
        snapshot.focused_control_role = Some("AXTextArea".to_string());
        snapshot.activity_state = Some("composing_prompt".to_string());
        snapshot.task_state = Some("draft_or_composer_active".to_string());

        let quality = evaluate_surface_snapshot_quality(&snapshot);

        assert_eq!(quality.evidence_quality, EvidenceQuality::Medium);
        assert!(quality.candidate_eligible);
        assert!(quality
            .missing_evidence
            .contains(&"Exact Codex thread was not visible.".to_string()));
    }

    #[test]
    fn quality_codex_app_window_only_is_thin_with_medium_stale_suppression() {
        let mut snapshot =
            test_surface_snapshot("snap-codex-window", "surface-codex-window", 10_000);
        snapshot.domain = "codex_desktop_app".to_string();
        snapshot.app_name = Some("Codex".to_string());
        snapshot.bundle_id = Some("com.openai.codex".to_string());
        snapshot.window_title = Some("Codex".to_string());
        snapshot.repo_root_path = None;
        snapshot.repo_root_hash = None;
        snapshot.workspace_path = None;
        snapshot.workspace_path_hash = None;
        snapshot.thread_title = None;
        snapshot.thread_key_hash = None;
        snapshot.visible_text_sample = None;
        snapshot.visible_text_hash = None;
        snapshot.focused_control_role = None;
        snapshot.activity_state = Some("unknown".to_string());
        snapshot.task_state = Some("unknown".to_string());

        let quality = evaluate_surface_snapshot_quality(&snapshot);

        assert_eq!(quality.evidence_quality, EvidenceQuality::Thin);
        assert!(!quality.candidate_eligible);
        assert_eq!(
            quality.stale_target_suppression_strength,
            StaleSuppressionStrength::Medium
        );
    }

    #[test]
    fn quality_editor_active_file_typing_is_strong_with_repo() {
        let mut snapshot = test_surface_snapshot("snap-editor", "surface-editor", 10_000);
        snapshot.domain = "code_editor".to_string();
        snapshot.repo_root_path = Some("/Users/me/smalltalk".to_string());
        snapshot.active_file_path =
            Some("/Users/me/smalltalk/src-tauri/src/continuation.rs".to_string());
        snapshot.active_relative_file = Some("src-tauri/src/continuation.rs".to_string());
        snapshot.focused_control_role = Some("AXTextArea".to_string());
        snapshot.visible_text_sample = Some("fn score_continue_candidates()".to_string());
        snapshot.activity_state = Some("actively_editing".to_string());
        snapshot.task_state = Some("editing_file".to_string());

        let quality = evaluate_surface_snapshot_quality(&snapshot);

        assert_eq!(quality.evidence_quality, EvidenceQuality::Strong);
        assert!(quality.candidate_eligible);
    }

    #[test]
    fn quality_terminal_failed_command_with_repo_is_strong() {
        let mut snapshot = test_surface_snapshot("snap-terminal-error", "surface-terminal", 10_000);
        snapshot.domain = "terminal".to_string();
        snapshot.repo_root_path = Some("/Users/me/smalltalk".to_string());
        snapshot.command_state = Some("command_failed".to_string());
        snapshot.task_state = Some("visible_error_unresolved".to_string());
        snapshot.activity_state = Some("encountering_error".to_string());
        snapshot.error_markers_json = Some("[\"test_failure\"]".to_string());
        snapshot.visible_text_sample = Some("cargo test failed".to_string());

        let quality = evaluate_surface_snapshot_quality(&snapshot);

        assert_eq!(quality.evidence_quality, EvidenceQuality::Strong);
        assert!(quality.candidate_eligible);
    }

    #[test]
    fn quality_terminal_app_window_only_is_thin_without_command_state() {
        let mut snapshot = test_surface_snapshot("snap-terminal-thin", "surface-terminal", 10_000);
        snapshot.domain = "terminal".to_string();
        snapshot.repo_root_path = None;
        snapshot.repo_root_hash = None;
        snapshot.command_state = Some("unknown".to_string());
        snapshot.task_state = Some("unknown".to_string());
        snapshot.activity_state = Some("unknown".to_string());
        snapshot.visible_text_sample = None;
        snapshot.focused_control_role = None;

        let quality = evaluate_surface_snapshot_quality(&snapshot);

        assert_eq!(quality.evidence_quality, EvidenceQuality::Thin);
        assert!(!quality.candidate_eligible);
        assert!(quality
            .missing_evidence
            .contains(&"Terminal command state was not clear.".to_string()));
    }

    #[test]
    fn quality_downgrades_stale_strong_confidence_on_thin_snapshot() {
        let mut snapshot =
            test_surface_snapshot("snap-thin-stale-confidence", "surface-codex-window", 10_000);
        snapshot.domain = "codex_desktop_app".to_string();
        snapshot.app_name = Some("Codex".to_string());
        snapshot.window_title = Some("Codex".to_string());
        snapshot.repo_root_path = None;
        snapshot.repo_root_hash = None;
        snapshot.workspace_path = None;
        snapshot.workspace_path_hash = None;
        snapshot.thread_title = None;
        snapshot.thread_key_hash = None;
        snapshot.visible_text_sample = None;
        snapshot.focused_control_role = None;
        snapshot.identity_confidence = "strong".to_string();
        snapshot.evidence_quality = "strong".to_string();

        apply_surface_snapshot_quality(&mut snapshot).unwrap();

        assert_eq!(snapshot.evidence_quality, "thin");
        assert_eq!(snapshot.identity_confidence, "thin");
        assert_eq!(snapshot.openability, "app_focus_only");
        let missing = snapshot
            .missing_fields_json
            .as_deref()
            .map(parse_string_array)
            .unwrap_or_default();
        assert!(missing.contains(&"thread_identity_missing".to_string()));
        assert!(missing.contains(&"openable_target_missing".to_string()));
    }

    #[test]
    fn quality_privacy_blocked_is_not_eligible_and_reports_privacy_missing_evidence() {
        let mut snapshot = test_surface_snapshot("snap-private", "surface-private", 10_000);
        snapshot.domain = "codex_cli".to_string();
        snapshot.repo_root_hash = Some("repo-hash".to_string());
        snapshot.thread_key_hash = Some("thread-hash".to_string());
        snapshot.privacy_status = Some("privacy_skip".to_string());

        let quality = evaluate_surface_snapshot_quality(&snapshot);

        assert!(!quality.candidate_eligible);
        assert!(matches!(
            quality.evidence_quality,
            EvidenceQuality::Thin | EvidenceQuality::Unknown
        ));
        assert!(quality
            .missing_evidence
            .contains(&"Privacy rules blocked some visible evidence.".to_string()));
    }

    fn surface_input(
        app_name: &str,
        bundle_id: Option<&str>,
        window_title: Option<&str>,
        full_text: Option<&str>,
        document_path: Option<&str>,
        event_type: &str,
        privacy_status: Option<&str>,
    ) -> SurfaceEnrichmentInput {
        let classification = classify(WeakSurfaceClassificationInput {
            app_name: Some(app_name.to_string()),
            bundle_id: bundle_id.map(str::to_string),
            window_title: window_title.map(str::to_string),
            document_path: document_path.map(str::to_string),
            full_text_sample: full_text.map(str::to_string),
            event_types: vec![event_type.to_string()],
            trigger_type: Some(event_type.to_string()),
            privacy_status: privacy_status.map(str::to_string),
            ..Default::default()
        });
        SurfaceEnrichmentInput {
            classification,
            observed_at_ms: 12_345,
            session_id: Some("session-adapter".to_string()),
            frame: Some(CaptureFrameLite {
                id: Some(99),
                app_name: Some(app_name.to_string()),
                bundle_id: bundle_id.map(str::to_string),
                app_pid: Some(4242),
                window_id: Some(17),
                window_title: window_title.map(str::to_string),
                browser_url: None,
                document_path: document_path.map(str::to_string),
                full_text: full_text.map(str::to_string),
                text_source: Some("active_owned".to_string()),
                capture_trigger: Some(event_type.to_string()),
                privacy_status: privacy_status.map(str::to_string),
            }),
            recent_events: vec![UiEventLite {
                id: "evt-adapter".to_string(),
                event_type: event_type.to_string(),
                key_category: Some("character".to_string()),
            }],
            content_units: full_text
                .map(|text| {
                    vec![ContentUnitLite {
                        id: "unit-main".to_string(),
                        source: "accessibility".to_string(),
                        unit_type: "text".to_string(),
                        semantic_role: Some("main_content".to_string()),
                        text: Some(text.to_string()),
                        confidence: Some(0.92),
                    }]
                })
                .unwrap_or_default(),
            ax_nodes: vec![AxNodeLite {
                id: "ax-focused".to_string(),
                role: Some("AXTextArea".to_string()),
                text: full_text.map(str::to_string),
                selected_text: Some("selected code that must only hash".to_string()),
                focused: Some(true),
                depth: Some(2),
            }],
            ocr_spans: Vec::new(),
            app_contexts: document_path
                .map(|path| {
                    vec![AppContextLite {
                        id: "ctx-file".to_string(),
                        adapter_id: "test".to_string(),
                        object_type: "document".to_string(),
                        title: window_title.map(str::to_string),
                        url: None,
                        file_path: Some(path.to_string()),
                        selected_text: None,
                        focused_object: Some("editor".to_string()),
                        confidence: Some(0.9),
                    }]
                })
                .unwrap_or_default(),
            window_snapshot: Some(WindowSnapshotLite {
                active_window_id: Some(17),
                active_app_pid: Some(4242),
                active_app_bundle_id: bundle_id.map(str::to_string),
                active_app_name: Some(app_name.to_string()),
                active_window_title: window_title.map(str::to_string),
            }),
            typing_bursts: vec![TypingBurstLite {
                id: "burst-1".to_string(),
                enter_count: if event_type.contains("enter") { 1 } else { 0 },
                paste_count: 0,
                committed: !event_type.contains("typing"),
                commit_signal: Some(event_type.to_string()),
            }],
            clipboard_metadata: Vec::new(),
        }
    }

    #[test]
    fn terminal_visible_codex_typing_is_codex_cli() {
        let result = classify(WeakSurfaceClassificationInput {
            app_name: Some("Terminal".to_string()),
            full_text_sample: Some("▌ codex editing continuation.rs".to_string()),
            event_types: vec!["typing_pause".to_string()],
            ..Default::default()
        });
        assert_eq!(result.domain, WeakSurfaceDomain::CodexCli);
        assert_eq!(result.enrichment_need, EnrichmentNeed::Targeted);
        assert!(result.reasons.contains(&"app_terminal".to_string()));
        assert!(result
            .reasons
            .contains(&"content_mentions_codex".to_string()));
    }

    #[test]
    fn warp_title_codex_is_codex_cli() {
        let result = classify(WeakSurfaceClassificationInput {
            app_name: Some("Warp".to_string()),
            window_title: Some("codex - smalltalk".to_string()),
            ..Default::default()
        });
        assert_eq!(result.domain, WeakSurfaceDomain::CodexCli);
        assert_eq!(result.enrichment_need, EnrichmentNeed::Targeted);
        assert!(result
            .reasons
            .contains(&"window_mentions_codex".to_string()));
    }

    #[test]
    fn cursor_codex_panel_is_codex_ide_extension() {
        let result = classify(WeakSurfaceClassificationInput {
            app_name: Some("Cursor".to_string()),
            full_text_sample: Some("Ask Codex to review with Codex".to_string()),
            event_types: vec!["typing".to_string()],
            ..Default::default()
        });
        assert_eq!(result.domain, WeakSurfaceDomain::CodexIdeExtension);
        assert_eq!(result.enrichment_need, EnrichmentNeed::Targeted);
        assert!(result.reasons.contains(&"app_is_editor_family".to_string()));
        assert!(result
            .reasons
            .contains(&"content_mentions_codex".to_string()));
    }

    #[test]
    fn vscode_active_file_without_codex_is_code_editor() {
        let result = classify(WeakSurfaceClassificationInput {
            app_name: Some("Visual Studio Code".to_string()),
            document_path: Some("/Users/me/project/src/lib.rs".to_string()),
            event_types: vec!["key:character".to_string()],
            ..Default::default()
        });
        assert_eq!(result.domain, WeakSurfaceDomain::CodeEditor);
        assert_eq!(result.enrichment_need, EnrichmentNeed::Targeted);
        assert!(result
            .reasons
            .contains(&"document_path_has_source_extension".to_string()));
    }

    #[test]
    fn terminal_output_without_codex_is_terminal() {
        let result = classify(WeakSurfaceClassificationInput {
            app_name: Some("Terminal".to_string()),
            event_types: vec!["command_output".to_string()],
            ..Default::default()
        });
        assert_eq!(result.domain, WeakSurfaceDomain::Terminal);
        assert_eq!(result.enrichment_need, EnrichmentNeed::Targeted);
        assert!(result
            .reasons
            .contains(&"recent_output_activity".to_string()));
    }

    #[test]
    fn codex_app_name_is_codex_desktop() {
        let result = classify(WeakSurfaceClassificationInput {
            app_name: Some("Codex".to_string()),
            bundle_id: Some("com.openai.codex".to_string()),
            ..Default::default()
        });
        assert_eq!(result.domain, WeakSurfaceDomain::CodexDesktopApp);
        assert_eq!(result.enrichment_need, EnrichmentNeed::Targeted);
        assert!(result.reasons.contains(&"app_is_codex_desktop".to_string()));
    }

    #[test]
    fn low_text_custom_app_recent_typing_is_unknown_retry() {
        let result = classify(WeakSurfaceClassificationInput {
            app_name: Some("CanvasNative".to_string()),
            full_text_sample: Some("".to_string()),
            event_types: vec!["typing_pause".to_string()],
            ..Default::default()
        });
        assert_eq!(result.domain, WeakSurfaceDomain::UnknownWeakSurface);
        assert_eq!(result.enrichment_need, EnrichmentNeed::Retry);
        assert!(result
            .reasons
            .contains(&"low_text_custom_surface".to_string()));
    }

    #[test]
    fn browser_chatgpt_url_is_not_native_agent_window() {
        let result = classify(WeakSurfaceClassificationInput {
            app_name: Some("Google Chrome".to_string()),
            browser_url: Some("https://chatgpt.com/c/123".to_string()),
            full_text_sample: Some("compose plan apply patch".to_string()),
            event_types: vec!["typing".to_string()],
            ..Default::default()
        });
        assert_eq!(result.domain, WeakSurfaceDomain::NotWeakSurface);
        assert_eq!(result.enrichment_need, EnrichmentNeed::None);
        assert!(result.reasons.contains(&"browser_surface".to_string()));
    }

    #[test]
    fn privacy_skipped_surface_is_blocked() {
        let result = classify(WeakSurfaceClassificationInput {
            app_name: Some("Terminal".to_string()),
            privacy_status: Some("privacy_skip".to_string()),
            full_text_sample: Some("codex".to_string()),
            event_types: vec!["typing".to_string()],
            ..Default::default()
        });
        assert_eq!(result.domain, WeakSurfaceDomain::UnknownWeakSurface);
        assert_eq!(result.enrichment_need, EnrichmentNeed::BlockedByPrivacy);
        assert!(result.reasons.contains(&"privacy_blocked".to_string()));
    }

    #[test]
    fn codex_cli_composer_enriches_to_strong_snapshot() {
        let input = surface_input(
            "Terminal",
            Some("com.apple.Terminal"),
            Some("codex | Thread: P3 adapters | /Users/me/smalltalk"),
            Some("Ask Codex\nThread: P3 adapters\n▌ implement adapter tests"),
            None,
            "typing_pause",
            None,
        );
        let output = enrich_surface(&input);
        let snapshot = output.snapshot.expect("snapshot");
        assert_eq!(snapshot.domain, "codex_cli");
        assert_eq!(snapshot.adapter_key.as_deref(), Some("codex_cli_adapter"));
        assert_eq!(snapshot.activity_state.as_deref(), Some("composing_prompt"));
        assert_eq!(snapshot.identity_confidence, "strong");
        assert_eq!(snapshot.evidence_quality, "strong");
        assert!(snapshot.thread_title.is_some());
        assert!(snapshot.visible_text_sample.unwrap().contains("Ask Codex"));
    }

    #[test]
    fn codex_cli_app_window_only_stays_thin_with_missing_identity() {
        let mut input = surface_input(
            "Terminal",
            Some("com.apple.Terminal"),
            Some("codex"),
            None,
            None,
            "window_focus",
            None,
        );
        input.ax_nodes.clear();
        let output = enrich_surface(&input);
        let snapshot = output.snapshot.expect("thin snapshot");
        assert_eq!(snapshot.domain, "codex_cli");
        assert_eq!(snapshot.identity_confidence, "thin");
        assert_eq!(snapshot.openability, "frame_fallback");
        assert!(output
            .missing_fields
            .contains(&"repo_root_missing".to_string()));
        assert!(output
            .missing_fields
            .contains(&"thread_identity_missing".to_string()));
    }

    #[test]
    fn codex_desktop_project_thread_enriches_identity() {
        let input = surface_input(
            "Codex",
            Some("com.openai.codex"),
            Some("smalltalk | Thread: P3.04 adapters"),
            Some("Thread: P3.04 adapters\nPlan updated\nrunning cargo test"),
            Some("/Users/me/smalltalk/src-tauri/src/continuation/enrichment.rs"),
            "window_focus",
            None,
        );
        let output = enrich_surface(&input);
        let snapshot = output.snapshot.expect("snapshot");
        assert_eq!(snapshot.domain, "codex_desktop_app");
        assert_eq!(snapshot.identity_confidence, "strong");
        assert_eq!(snapshot.thread_title.as_deref(), Some("P3.04 adapters"));
        assert!(snapshot
            .workspace_path
            .as_deref()
            .unwrap()
            .ends_with("continuation"));
    }

    #[test]
    fn codex_ide_extension_preserves_file_and_panel_identity() {
        let input = surface_input(
            "Cursor",
            Some("com.todesktop.230313mzl4w4u92"),
            Some("enrichment.rs - smalltalk - Cursor"),
            Some("Ask Codex\nThread: adapter implementation\nfn enrich_surface()"),
            Some("/Users/me/smalltalk/src-tauri/src/continuation/enrichment.rs"),
            "typing_pause",
            None,
        );
        let output = enrich_surface(&input);
        let snapshot = output.snapshot.expect("snapshot");
        assert_eq!(snapshot.domain, "codex_ide_extension");
        assert_eq!(snapshot.identity_confidence, "strong");
        assert_eq!(
            snapshot.active_relative_file.as_deref(),
            Some("enrichment.rs")
        );
        assert_eq!(snapshot.openability, "openable");
        assert!(snapshot.thread_key_hash.is_some());
    }

    #[test]
    fn code_editor_active_file_typing_is_actively_editing_and_openable() {
        let input = surface_input(
            "Visual Studio Code",
            Some("com.microsoft.VSCode"),
            Some("lib.rs - smalltalk"),
            Some("fn main() {\n    println!(\"hello\");\n}"),
            Some("/Users/me/smalltalk/src/lib.rs"),
            "typing_pause",
            None,
        );
        let output = enrich_surface(&input);
        let snapshot = output.snapshot.expect("snapshot");
        assert_eq!(snapshot.domain, "code_editor");
        assert_eq!(snapshot.activity_state.as_deref(), Some("actively_editing"));
        assert_eq!(snapshot.task_state.as_deref(), Some("editing_file"));
        assert_eq!(snapshot.openability, "openable");
        assert!(snapshot.selected_text_hash.is_some());
    }

    #[test]
    fn terminal_test_failure_maps_to_unresolved_error() {
        let input = surface_input(
            "Terminal",
            Some("com.apple.Terminal"),
            Some("smalltalk — /Users/me/smalltalk"),
            Some("$ cargo test\nerror: test failed\ntest result: FAILED. 2 passed; 1 failed"),
            None,
            "enter",
            None,
        );
        let output = enrich_surface(&input);
        let snapshot = output.snapshot.expect("snapshot");
        assert_eq!(snapshot.domain, "terminal");
        assert_eq!(
            snapshot.task_state.as_deref(),
            Some("visible_error_unresolved")
        );
        assert_eq!(snapshot.command_state.as_deref(), Some("command_failed"));
        assert!(snapshot
            .error_markers_json
            .as_deref()
            .unwrap()
            .contains("test_failure"));
    }

    #[test]
    fn native_agent_composer_maps_to_composing_prompt() {
        let input = surface_input(
            "Claude Code",
            Some("com.anthropic.claudecode"),
            Some("Agent | Thread: polish adapters"),
            Some("Composer\napply patch\nType a message to the agent"),
            None,
            "typing_pause",
            None,
        );
        let output = enrich_surface(&input);
        let snapshot = output.snapshot.expect("snapshot");
        assert_eq!(snapshot.domain, "native_agent_window");
        assert_eq!(snapshot.activity_state.as_deref(), Some("composing_prompt"));
        assert_eq!(
            snapshot.task_state.as_deref(),
            Some("draft_or_composer_active")
        );
        assert_eq!(snapshot.identity_confidence, "medium");
    }

    #[test]
    fn privacy_blocked_adapter_produces_no_snapshot_or_text() {
        let input = surface_input(
            "Terminal",
            Some("com.apple.Terminal"),
            Some("codex | private"),
            Some("Ask Codex about sk-secret-token"),
            None,
            "typing_pause",
            Some("privacy_skip"),
        );
        let output = enrich_surface(&input);
        assert_eq!(output.status, EnrichmentAttemptStatus::SkippedPrivacy);
        assert!(output.snapshot.is_none());
        assert!(output
            .missing_fields
            .contains(&"privacy_blocked_text".to_string()));
    }

    #[test]
    fn weak_identity_codex_cli_repo_thread_is_strong() {
        let identity = build_weak_surface_identity(WeakSurfaceIdentityInput {
            domain: WeakSurfaceDomain::CodexCli,
            repo_root_hash: Some("repo-hash".to_string()),
            thread_key_hash: Some("thread-hash".to_string()),
            app_name: Some("Terminal".to_string()),
            window_title: Some("codex - smalltalk".to_string()),
            observed_at_ms: Some(600_000),
            ..Default::default()
        })
        .expect("identity");

        assert_eq!(identity.stable_key, "codex_cli:repo-hash:thread-hash");
        assert_eq!(identity.identity_confidence, "strong");
        assert!(identity.merge_keys.contains(&"repo:repo-hash".to_string()));
        assert!(identity
            .merge_keys
            .contains(&"codex_thread:thread-hash".to_string()));
    }

    #[test]
    fn weak_identity_codex_cli_repo_window_is_medium_with_thread_missing() {
        let identity = build_weak_surface_identity(WeakSurfaceIdentityInput {
            domain: WeakSurfaceDomain::CodexCli,
            repo_root_hash: Some("repo-hash".to_string()),
            app_name: Some("Terminal".to_string()),
            bundle_id: Some("com.apple.Terminal".to_string()),
            window_title: Some("codex - smalltalk".to_string()),
            observed_at_ms: Some(600_000),
            ..Default::default()
        })
        .expect("identity");

        assert!(identity.stable_key.starts_with("codex_cli:repo-hash:"));
        assert_eq!(identity.identity_confidence, "medium");
        assert!(identity
            .missing_fields
            .contains(&"thread_identity_missing".to_string()));
    }

    #[test]
    fn weak_identity_terminal_app_window_only_is_thin() {
        let identity = build_weak_surface_identity(WeakSurfaceIdentityInput {
            domain: WeakSurfaceDomain::Terminal,
            app_name: Some("Terminal".to_string()),
            bundle_id: Some("com.apple.Terminal".to_string()),
            window_title: Some("zsh".to_string()),
            observed_at_ms: Some(600_000),
            ..Default::default()
        })
        .expect("identity");

        assert!(identity.stable_key.starts_with("terminal:"));
        assert!(identity.stable_key.ends_with(":2"));
        assert_eq!(identity.identity_confidence, "thin");
        assert!(identity
            .missing_fields
            .contains(&"repo_root_missing".to_string()));
        assert!(identity
            .missing_fields
            .contains(&"command_signature_missing".to_string()));
    }

    #[test]
    fn weak_identity_code_editor_repo_relative_file_is_strong_and_redacted() {
        let identity = build_weak_surface_identity(WeakSurfaceIdentityInput {
            domain: WeakSurfaceDomain::CodeEditor,
            repo_root_path: Some("/Users/me/private/smalltalk".to_string()),
            active_file_path: Some("/Users/me/private/smalltalk/src/main.rs".to_string()),
            active_relative_file: Some("src/main.rs".to_string()),
            app_name: Some("Visual Studio Code".to_string()),
            ..Default::default()
        })
        .expect("identity");

        assert!(identity.stable_key.starts_with("code_editor:"));
        assert_eq!(identity.identity_confidence, "strong");
        assert_eq!(identity.openability, "openable");
        assert!(!identity.stable_key.contains("/Users/me/private"));
        assert!(!identity.merge_keys.join(" ").contains("/Users/me/private"));
    }

    #[test]
    fn weak_identity_codex_ide_repo_file_thread_is_strong() {
        let identity = build_weak_surface_identity(WeakSurfaceIdentityInput {
            domain: WeakSurfaceDomain::CodexIdeExtension,
            repo_root_hash: Some("repo-hash".to_string()),
            active_relative_file: Some("src/App.tsx".to_string()),
            thread_title: Some("Continue identity".to_string()),
            app_name: Some("Cursor".to_string()),
            ..Default::default()
        })
        .expect("identity");

        assert!(identity.stable_key.starts_with("codex_ide:repo-hash:"));
        assert_eq!(identity.identity_confidence, "strong");
        assert!(identity
            .merge_keys
            .iter()
            .any(|key| key.starts_with("active_file:repo-hash:")));
    }

    #[test]
    fn weak_identity_codex_desktop_project_thread_is_strong() {
        let identity = build_weak_surface_identity(WeakSurfaceIdentityInput {
            domain: WeakSurfaceDomain::CodexDesktopApp,
            workspace_path_hash: Some("project-hash".to_string()),
            thread_key_hash: Some("thread-hash".to_string()),
            app_name: Some("Codex".to_string()),
            ..Default::default()
        })
        .expect("identity");

        assert_eq!(identity.stable_key, "codex_app:project-hash:thread-hash");
        assert_eq!(identity.identity_confidence, "strong");
    }

    #[test]
    fn weak_identity_same_repo_merge_key_joins_editor_and_terminal() {
        let editor = build_weak_surface_identity(WeakSurfaceIdentityInput {
            domain: WeakSurfaceDomain::CodeEditor,
            repo_root_hash: Some("same-repo".to_string()),
            active_relative_file: Some("src/lib.rs".to_string()),
            app_name: Some("Cursor".to_string()),
            ..Default::default()
        })
        .expect("editor identity");
        let terminal = build_weak_surface_identity(WeakSurfaceIdentityInput {
            domain: WeakSurfaceDomain::Terminal,
            repo_root_hash: Some("same-repo".to_string()),
            command_signature_hash: Some("cargo-test".to_string()),
            app_name: Some("Terminal".to_string()),
            window_title: Some("smalltalk".to_string()),
            observed_at_ms: Some(600_000),
            ..Default::default()
        })
        .expect("terminal identity");

        assert!(editor.merge_keys.contains(&"repo:same-repo".to_string()));
        assert!(terminal.merge_keys.contains(&"repo:same-repo".to_string()));
    }

    #[test]
    fn weak_identity_app_name_alone_does_not_merge_unrelated_surfaces() {
        let identity = build_weak_surface_identity(WeakSurfaceIdentityInput {
            domain: WeakSurfaceDomain::Terminal,
            app_name: Some("Terminal".to_string()),
            observed_at_ms: Some(600_000),
            ..Default::default()
        });

        assert!(identity.is_none());
    }

    #[test]
    fn app_switch_into_codex_cli_schedules_targeted_enrichment() {
        let attempt = build_enrichment_attempt(policy_input(
            "Terminal",
            Some("codex - smalltalk"),
            "app_switch",
        ));
        assert_eq!(attempt.weak_domain, WeakSurfaceDomain::CodexCli);
        assert_eq!(attempt.status, EnrichmentAttemptStatus::SucceededMedium);
        assert_eq!(attempt.attempt_index, 0);
        assert!(attempt.snapshot_id.is_some());
    }

    #[test]
    fn vscode_codex_side_panel_focus_schedules_targeted_enrichment() {
        let attempt = build_enrichment_attempt(policy_input(
            "Visual Studio Code",
            Some("Ask Codex - smalltalk"),
            "window_focus",
        ));
        assert_eq!(attempt.weak_domain, WeakSurfaceDomain::CodexIdeExtension);
        assert!(attempt.produced_snapshot());
    }

    #[test]
    fn typing_pause_in_terminal_is_one_bucketed_attempt() {
        let mut input = policy_input("Terminal", Some("smalltalk"), "typing_pause");
        input.trigger_event_ids = vec!["evt-key-1".to_string(), "evt-key-2".to_string()];
        let attempt = build_enrichment_attempt(input);
        assert_eq!(attempt.trigger_type, "typing_pause");
        assert_eq!(attempt.trigger_event_ids.len(), 2);
        assert!(attempt.produced_snapshot());
    }

    #[test]
    fn recent_strong_snapshot_suppresses_attempt() {
        let mut input = policy_input("Terminal", Some("codex - smalltalk"), "app_switch");
        input.recent_strong_snapshot = true;
        let attempt = build_enrichment_attempt(input);
        assert_eq!(
            attempt.status,
            EnrichmentAttemptStatus::SkippedRecentStrongSnapshot
        );
        assert!(attempt.snapshot_id.is_none());
    }

    #[test]
    fn privacy_blocked_surface_records_skip_without_snapshot_text() {
        let mut input = policy_input("Terminal", Some("codex - smalltalk"), "typing_pause");
        input.privacy_blocked = true;
        input.classification.privacy_tier = "blocked".to_string();
        let attempt = build_enrichment_attempt(input);
        assert_eq!(attempt.status, EnrichmentAttemptStatus::SkippedPrivacy);
        assert!(attempt.snapshot_id.is_none());
        assert!(attempt.window_title_capped.is_some());
    }

    #[test]
    fn focus_changed_before_retry_skips_retry() {
        let mut input = policy_input("Terminal", Some("codex - smalltalk"), "app_switch");
        input.recent_attempts_30s = 2;
        input.focus_changed = true;
        let attempt = build_enrichment_attempt(input);
        assert_eq!(attempt.attempt_index, 2);
        assert_eq!(attempt.status, EnrichmentAttemptStatus::SkippedFocusChanged);
    }

    #[test]
    fn continue_request_with_weak_focus_records_bounded_attempt() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "
            CREATE TABLE ui_events (
                id TEXT PRIMARY KEY,
                ts_ms INTEGER NOT NULL,
                event_type TEXT NOT NULL,
                key_category TEXT,
                app_name TEXT,
                app_bundle_id TEXT,
                window_title TEXT,
                window_id INTEGER,
                session_id TEXT
            );
            INSERT INTO ui_events (
                id, ts_ms, event_type, key_category, app_name, app_bundle_id,
                window_title, window_id, session_id
            ) VALUES (
                'evt-codex', 1000, 'app_switch', NULL, 'Terminal', NULL,
                'codex - smalltalk', 42, 'session-a'
            );
            ",
        )
        .unwrap();
        let attempt = run_continue_request_weak_surface_enrichment(&conn, Some("session-a"), 1_100)
            .unwrap()
            .expect("attempt");
        assert_eq!(attempt.trigger_type, "continue_request");
        assert!(attempt.produced_snapshot());
        let diagnostics = weak_surface_enrichment_diagnostics(&conn).unwrap();
        assert_eq!(diagnostics.weak_surface_enrichment_attempts, 1);
        assert!(diagnostics.latest_weak_surface_snapshot_id.is_some());
        let canonical_attempts: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM continue_surface_enrichment_attempts",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let snapshots: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM continue_surface_snapshots",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(canonical_attempts, 1);
        assert_eq!(snapshots, 1);
    }

    #[test]
    fn surface_schema_creates_attempt_and_snapshot_tables() {
        let conn = Connection::open_in_memory().unwrap();
        ensure_weak_surface_enrichment_schema(&conn).unwrap();

        assert!(table_exists(&conn, "continue_surface_enrichment_attempts").unwrap());
        assert!(table_exists(&conn, "continue_surface_snapshots").unwrap());
        assert!(column_exists(&conn, "continue_surface_snapshots", "visible_text_sample").unwrap());
    }

    #[test]
    fn insert_surface_attempt_can_be_loaded_by_surface() {
        let conn = Connection::open_in_memory().unwrap();
        let attempt = SurfaceEnrichmentAttempt {
            id: "attempt-a".to_string(),
            session_id: Some("session-a".to_string()),
            surface_key: "surface-a".to_string(),
            domain: "terminal".to_string(),
            adapter_key: Some("terminal_adapter".to_string()),
            trigger_type: Some("typing_pause".to_string()),
            trigger_event_ids_json: "[\"evt-1\"]".to_string(),
            frame_id: None,
            observed_at_ms: 1_000,
            scheduled_at_ms: Some(1_010),
            completed_at_ms: Some(1_020),
            attempt_index: 0,
            status: "succeeded_thin".to_string(),
            reason: Some("event_only_weak_surface_snapshot".to_string()),
            missing_fields_json: "[\"fresh_heavy_frame_missing\"]".to_string(),
            snapshot_id: Some("snapshot-a".to_string()),
            app_name: Some("Terminal".to_string()),
            bundle_id: Some("com.apple.Terminal".to_string()),
            window_title: Some("codex - smalltalk".to_string()),
            window_title_hash: None,
            window_id: Some(42),
            privacy_status: None,
            created_at_ms: 1_020,
        };

        insert_surface_enrichment_attempt(&conn, &attempt).unwrap();

        let loaded: String = conn
            .query_row(
                "SELECT id FROM continue_surface_enrichment_attempts WHERE surface_key = ?1",
                params!["surface-a"],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(loaded, "attempt-a");
    }

    #[test]
    fn upsert_surface_snapshot_updates_existing_row() {
        let conn = Connection::open_in_memory().unwrap();
        let now = current_time_millis();
        let mut snapshot = test_surface_snapshot("snapshot-a", "surface-a", now);
        upsert_surface_snapshot(&conn, &snapshot).unwrap();
        snapshot.evidence_quality = "strong".to_string();
        snapshot.repo_root_path = Some("/Users/me/smalltalk".to_string());
        snapshot.repo_root_hash = Some("repo-hash".to_string());
        snapshot.command_state = Some("command_running".to_string());
        snapshot.focused_control_role = Some("AXTextArea".to_string());
        snapshot.visible_text_sample = Some("fresh visible text".to_string());
        snapshot.updated_at_ms = now + 1;
        upsert_surface_snapshot(&conn, &snapshot).unwrap();

        let loaded = load_latest_surface_snapshot_for_surface(&conn, "surface-a", 60_000)
            .unwrap()
            .expect("snapshot");
        assert_eq!(loaded.id, "snapshot-a");
        assert_eq!(loaded.evidence_quality, "strong");
        assert_eq!(
            loaded.visible_text_sample.as_deref(),
            Some("fresh visible text")
        );
    }

    #[test]
    fn latest_surface_snapshot_respects_max_age() {
        let conn = Connection::open_in_memory().unwrap();
        let now = current_time_millis();
        upsert_surface_snapshot(
            &conn,
            &test_surface_snapshot("snapshot-old", "surface-a", now - 10_000),
        )
        .unwrap();
        assert!(
            load_latest_surface_snapshot_for_surface(&conn, "surface-a", 1_000)
                .unwrap()
                .is_none()
        );

        upsert_surface_snapshot(
            &conn,
            &test_surface_snapshot("snapshot-fresh", "surface-a", current_time_millis()),
        )
        .unwrap();
        assert_eq!(
            load_latest_surface_snapshot_for_surface(&conn, "surface-a", 1_000)
                .unwrap()
                .unwrap()
                .id,
            "snapshot-fresh"
        );
    }

    #[test]
    fn surface_snapshot_can_link_to_artifact_id() {
        let conn = Connection::open_in_memory().unwrap();
        let snapshot = test_surface_snapshot("snapshot-a", "surface-a", current_time_millis());
        upsert_surface_snapshot(&conn, &snapshot).unwrap();
        link_surface_snapshot_to_artifact(&conn, "snapshot-a", "artifact-a").unwrap();

        let loaded = load_latest_surface_snapshot_for_surface(&conn, "surface-a", 60_000)
            .unwrap()
            .unwrap();
        assert_eq!(loaded.artifact_id.as_deref(), Some("artifact-a"));
    }

    #[test]
    fn surface_snapshot_redaction_caps_visible_text_and_titles() {
        let conn = Connection::open_in_memory().unwrap();
        let mut snapshot = test_surface_snapshot("snapshot-a", "surface-a", current_time_millis());
        snapshot.window_title = Some("w".repeat(400));
        snapshot.thread_title = Some("t".repeat(400));
        snapshot.active_relative_file = Some("r".repeat(400));
        snapshot.focused_control_label = Some("f".repeat(400));
        snapshot.visible_text_sample = Some("v".repeat(1_200));
        upsert_surface_snapshot(&conn, &snapshot).unwrap();

        let loaded = load_latest_surface_snapshot_for_surface(&conn, "surface-a", 60_000)
            .unwrap()
            .unwrap();
        assert_eq!(
            loaded.window_title.unwrap().chars().count(),
            MAX_WINDOW_TITLE_CHARS
        );
        assert_eq!(
            loaded.thread_title.unwrap().chars().count(),
            MAX_THREAD_TITLE_CHARS
        );
        assert_eq!(
            loaded.active_relative_file.unwrap().chars().count(),
            MAX_RELATIVE_FILE_CHARS
        );
        assert_eq!(
            loaded.focused_control_label.unwrap().chars().count(),
            MAX_FOCUSED_CONTROL_LABEL_CHARS
        );
        assert_eq!(
            loaded.visible_text_sample.unwrap().chars().count(),
            MAX_VISIBLE_TEXT_SAMPLE_CHARS
        );
    }

    #[test]
    fn privacy_blocked_snapshot_stores_no_visible_text_sample() {
        let conn = Connection::open_in_memory().unwrap();
        let mut snapshot = test_surface_snapshot("snapshot-a", "surface-a", current_time_millis());
        snapshot.privacy_status = Some("privacy_skip".to_string());
        snapshot.visible_text_sample = Some("secret text".to_string());
        snapshot.focused_control_label = Some("secret control".to_string());
        snapshot.thread_title = Some("secret thread".to_string());
        upsert_surface_snapshot(&conn, &snapshot).unwrap();

        let loaded = load_latest_surface_snapshot_for_surface(&conn, "surface-a", 60_000)
            .unwrap()
            .unwrap();
        assert!(loaded.visible_text_sample.is_none());
        assert!(loaded.focused_control_label.is_none());
        assert!(loaded.thread_title.is_none());
    }

    #[test]
    fn smalltalk_self_focus_does_not_produce_snapshot() {
        let mut input = policy_input("Smalltalk", Some("Smalltalk"), "window_focus");
        input.self_capture = true;
        let attempt = build_enrichment_attempt(input);
        assert_eq!(attempt.status, EnrichmentAttemptStatus::SkippedSelfCapture);
        assert!(attempt.snapshot_id.is_none());
    }

    #[test]
    fn attempt_budgets_cap_surface_retries() {
        let mut input = policy_input("Terminal", Some("codex - smalltalk"), "typing_pause");
        input.recent_attempts_30s = MAX_ATTEMPTS_PER_SURFACE_PER_30S;
        let attempt = build_enrichment_attempt(input);
        assert_eq!(attempt.status, EnrichmentAttemptStatus::SkippedBudget);
    }
}
