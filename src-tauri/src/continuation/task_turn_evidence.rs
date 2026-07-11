use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet, HashMap};

use super::activity_recap::sanitize_public_text;

pub(crate) const TASK_TURN_EVIDENCE_SCHEMA_V1: &str = "smalltalk.task_turn_evidence.v2";
const MAX_SPAN_SUMMARY_CHARS: usize = 280;
const MAX_GOAL_SAMPLE_CHARS: usize = 320;
const MAX_AGENT_SAMPLE_CHARS: usize = 320;
const MAX_PRIOR_SAMPLE_CHARS: usize = 160;
const MAX_TOTAL_SAMPLE_CHARS: usize = 800;
const MIN_TYPED_ROLE_CONFIDENCE: f64 = 0.64;
const MIN_CAUSAL_TYPING_CONFIDENCE: f64 = 0.74;
const MAX_LEGACY_TYPING_TO_FRAME_MS: i64 = 60_000;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub(crate) enum EvidenceSourceKind {
    AccessibilityNode,
    OcrSpan,
    ContentUnit,
    FlattenedTextFallback,
}

impl EvidenceSourceKind {
    fn label(self) -> &'static str {
        match self {
            Self::AccessibilityNode => "accessibility_node",
            Self::OcrSpan => "ocr_span",
            Self::ContentUnit => "content_unit",
            Self::FlattenedTextFallback => "flattened_text_fallback",
        }
    }

    fn priority(self) -> u8 {
        match self {
            Self::AccessibilityNode => 4,
            Self::ContentUnit => 3,
            Self::OcrSpan => 2,
            Self::FlattenedTextFallback => 1,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TextStorageClass {
    SourceReferenceOnly,
    BoundedPublicSafeSummary,
}

impl TextStorageClass {
    fn label(self) -> &'static str {
        match self {
            Self::SourceReferenceOnly => "source_reference_only",
            Self::BoundedPublicSafeSummary => "bounded_public_safe_summary",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RegionRole {
    AppChrome,
    Navigation,
    Sidebar,
    ConversationHistory,
    UserMessage,
    AgentMessage,
    AgentStatus,
    SystemStatus,
    ToolOutput,
    EditorContent,
    TerminalInput,
    TerminalOutput,
    Composer,
    Dialog,
    Notification,
    Control,
    Unknown,
}

impl RegionRole {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::AppChrome => "app_chrome",
            Self::Navigation => "navigation",
            Self::Sidebar => "sidebar",
            Self::ConversationHistory => "conversation_history",
            Self::UserMessage => "user_message",
            Self::AgentMessage => "agent_message",
            Self::AgentStatus => "agent_status",
            Self::SystemStatus => "system_status",
            Self::ToolOutput => "tool_output",
            Self::EditorContent => "editor_content",
            Self::TerminalInput => "terminal_input",
            Self::TerminalOutput => "terminal_output",
            Self::Composer => "composer",
            Self::Dialog => "dialog",
            Self::Notification => "notification",
            Self::Control => "control",
            Self::Unknown => "unknown",
        }
    }

    fn is_agent(self) -> bool {
        matches!(self, Self::AgentMessage | Self::AgentStatus)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ConversationalRole {
    User,
    AssistantOrAgent,
    System,
    Tool,
    NonConversation,
    Unknown,
}

impl ConversationalRole {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::AssistantOrAgent => "assistant_or_agent",
            Self::System => "system",
            Self::Tool => "tool",
            Self::NonConversation => "non_conversation",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub(crate) struct SpanGeometry {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl SpanGeometry {
    fn center_x(self) -> f64 {
        self.x + self.width / 2.0
    }

    fn overlap_ratio(self, other: Self) -> f64 {
        let left = self.x.max(other.x);
        let top = self.y.max(other.y);
        let right = (self.x + self.width).min(other.x + other.width);
        let bottom = (self.y + self.height).min(other.y + other.height);
        if right <= left || bottom <= top {
            return 0.0;
        }
        let intersection = (right - left) * (bottom - top);
        let smaller = (self.width * self.height)
            .min(other.width * other.height)
            .max(1.0);
        (intersection / smaller).clamp(0.0, 1.0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct EvidenceSourceRef {
    pub source_kind: EvidenceSourceKind,
    pub source_record_id: String,
    pub source_text_reference: String,
    pub text_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct OrderedEvidenceSpan {
    pub schema: String,
    pub span_id: String,
    pub frame_id: String,
    pub session_id: Option<String>,
    pub surface_key: Option<String>,
    pub artifact_id: Option<String>,
    pub observed_at_ms: i64,
    pub primary_source: EvidenceSourceRef,
    pub contributing_sources: Vec<EvidenceSourceRef>,
    pub text_hash: String,
    pub text_storage_class: TextStorageClass,
    pub source_scope: String,
    pub ownership_kind: String,
    pub owner_window_id: Option<i64>,
    pub owner_app_id: Option<String>,
    pub region_role: RegionRole,
    pub conversational_role: ConversationalRole,
    pub pane_id: String,
    pub reading_order: i64,
    pub local_reading_order: i64,
    pub geometry: Option<SpanGeometry>,
    pub parent_or_group_id: Option<String>,
    pub focused: bool,
    pub selected: bool,
    pub active_artifact_match_confidence: f64,
    pub ownership_confidence: f64,
    pub region_confidence: f64,
    pub speaker_confidence: f64,
    pub order_confidence: f64,
    pub privacy_status: String,
    pub quality_flags: Vec<String>,
    pub reason_codes: Vec<String>,
    #[serde(skip)]
    text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct RejectedTurnSpan {
    pub span_id: String,
    pub reason_code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct LatestTurnEvidence {
    pub schema: String,
    pub frame_id: String,
    pub salient_span_ids: Vec<String>,
    pub latest_user_span_ids: Vec<String>,
    pub current_agent_span_ids: Vec<String>,
    pub prior_boundary_span_ids: Vec<String>,
    pub salient_user_goal_sample: Option<String>,
    pub salient_agent_state_sample: Option<String>,
    pub prior_boundary_sample: Option<String>,
    pub sample_storage_class: TextStorageClass,
    pub sampling_strategy: String,
    pub sampling_confidence: f64,
    pub missing_roles: Vec<String>,
    pub rejected_spans: Vec<RejectedTurnSpan>,
    pub fallback_flags: Vec<String>,
    pub causal_typing_attribution: Option<TypingBurstCausalAttribution>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct TypingBurstCausalAttribution {
    pub typing_burst_id: String,
    pub commit_signal: Option<String>,
    pub started_at_ms: i64,
    pub ended_at_ms: i64,
    pub pre_frame_id: Option<String>,
    pub post_frame_id: Option<String>,
    pub bounded_inferred_frame_id: Option<String>,
    pub surface_match_result: String,
    pub temporal_distance_ms: i64,
    pub association_source: String,
    pub association_confidence: f64,
    pub rejection_reasons: Vec<String>,
    pub capture_trigger_id: Option<String>,
    pub commit_event_id: Option<String>,
}

#[derive(Debug, Default)]
pub(crate) struct TaskTurnBuildResult {
    pub semantic_text_by_frame: HashMap<String, String>,
    pub span_count: usize,
    pub selection_count: usize,
}

#[derive(Debug, Default)]
pub(crate) struct TaskTurnAccuracyCheckpoints {
    pub region_roles: BTreeMap<String, Value>,
    pub conversational_roles: BTreeMap<String, Value>,
    pub ordered_turn_spans: BTreeMap<String, Value>,
    pub latest_task_turn: BTreeMap<String, Value>,
}

#[derive(Debug, Clone)]
struct FrameContext {
    frame_id: String,
    session_id: Option<String>,
    observed_at_ms: i64,
    app_name: Option<String>,
    bundle_id: Option<String>,
    window_id: Option<i64>,
    privacy_status: String,
    artifact_id: Option<String>,
    surface_key: Option<String>,
    family: SurfaceFamily,
    has_agent_status_event: bool,
    causal_typing: Option<TypingBurstCausalAttribution>,
    pre_frame_text_hashes: BTreeSet<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SurfaceFamily {
    AgentChat,
    BrowserChat,
    Editor,
    Terminal,
    Other,
}

#[derive(Debug, Clone)]
struct RawSpan {
    source: EvidenceSourceRef,
    text: String,
    source_scope: String,
    ownership_kind: String,
    owner_window_id: Option<i64>,
    owner_app_id: Option<String>,
    geometry: Option<SpanGeometry>,
    parent_id: Option<String>,
    source_order: i64,
    focused: bool,
    selected: bool,
    source_confidence: f64,
    ownership_confidence: f64,
    active_match_confidence: f64,
    structural_hint: String,
    semantic_role: Option<String>,
    quality_flags: Vec<String>,
}

pub(crate) fn ensure_task_turn_evidence_schema(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS continue_ordered_evidence_spans (
          span_id TEXT PRIMARY KEY,
          schema_version TEXT NOT NULL,
          frame_id TEXT NOT NULL,
          session_id TEXT,
          surface_key TEXT,
          artifact_id TEXT,
          observed_at_ms INTEGER NOT NULL,
          primary_source_kind TEXT NOT NULL,
          primary_source_record_id TEXT NOT NULL,
          source_text_reference TEXT NOT NULL,
          contributing_source_refs_json TEXT NOT NULL,
          text_hash TEXT NOT NULL,
          bounded_public_safe_summary TEXT,
          text_storage_class TEXT NOT NULL,
          source_scope TEXT NOT NULL,
          ownership_kind TEXT NOT NULL,
          owner_window_id INTEGER,
          owner_app_id TEXT,
          region_role TEXT NOT NULL,
          conversational_role TEXT NOT NULL,
          pane_id TEXT NOT NULL,
          reading_order INTEGER NOT NULL,
          local_reading_order INTEGER NOT NULL,
          geometry_json TEXT,
          parent_or_group_id TEXT,
          focused INTEGER NOT NULL DEFAULT 0,
          selected INTEGER NOT NULL DEFAULT 0,
          active_artifact_match_confidence REAL NOT NULL,
          ownership_confidence REAL NOT NULL,
          region_confidence REAL NOT NULL,
          speaker_confidence REAL NOT NULL,
          order_confidence REAL NOT NULL,
          privacy_status TEXT NOT NULL,
          quality_flags_json TEXT NOT NULL,
          reason_codes_json TEXT NOT NULL,
          created_at_ms INTEGER NOT NULL,
          updated_at_ms INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_continue_ordered_spans_frame_order
          ON continue_ordered_evidence_spans(frame_id, reading_order, span_id);
        CREATE INDEX IF NOT EXISTS idx_continue_ordered_spans_surface_time
          ON continue_ordered_evidence_spans(surface_key, observed_at_ms DESC);
        CREATE INDEX IF NOT EXISTS idx_continue_ordered_spans_source
          ON continue_ordered_evidence_spans(primary_source_kind, primary_source_record_id);

        CREATE TABLE IF NOT EXISTS continue_salient_turn_evidence (
          frame_id TEXT PRIMARY KEY,
          schema_version TEXT NOT NULL,
          session_id TEXT,
          surface_key TEXT,
          artifact_id TEXT,
          observed_at_ms INTEGER NOT NULL,
          salient_span_ids_json TEXT NOT NULL,
          latest_user_span_ids_json TEXT NOT NULL,
          current_agent_span_ids_json TEXT NOT NULL,
          prior_boundary_span_ids_json TEXT NOT NULL,
          salient_user_goal_sample TEXT,
          salient_user_goal_hash TEXT,
          salient_agent_state_sample TEXT,
          salient_agent_state_hash TEXT,
          prior_boundary_sample TEXT,
          prior_boundary_hash TEXT,
          sample_storage_class TEXT NOT NULL,
          sampling_strategy TEXT NOT NULL,
          sampling_confidence REAL NOT NULL,
          missing_roles_json TEXT NOT NULL,
          rejected_spans_json TEXT NOT NULL,
          fallback_flags_json TEXT NOT NULL,
          causal_typing_attribution_json TEXT,
          created_at_ms INTEGER NOT NULL,
          updated_at_ms INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_continue_salient_session_time
          ON continue_salient_turn_evidence(session_id, observed_at_ms DESC);
        ",
    )
    .map_err(to_string)?;
    if !column_exists(
        conn,
        "continue_salient_turn_evidence",
        "causal_typing_attribution_json",
    )? {
        conn.execute(
            "ALTER TABLE continue_salient_turn_evidence
             ADD COLUMN causal_typing_attribution_json TEXT",
            [],
        )
        .map_err(to_string)?;
    }
    Ok(())
}

pub(crate) fn rebuild_task_turn_evidence(
    conn: &Connection,
    frame_ids: &[String],
) -> Result<TaskTurnBuildResult, String> {
    ensure_task_turn_evidence_schema(conn)?;
    if frame_ids.is_empty() {
        return Ok(TaskTurnBuildResult::default());
    }

    let mut contexts = frame_ids
        .iter()
        .filter(|frame_id| frame_id.parse::<i64>().is_ok())
        .map(|frame_id| {
            load_frame_context(conn, frame_id)
                .map_err(|error| format!("task-turn frame context {frame_id}: {error}"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    contexts.sort_by_key(|context| (context.observed_at_ms, numeric_id(&context.frame_id)));

    for frame_id in frame_ids {
        conn.execute(
            "DELETE FROM continue_salient_turn_evidence WHERE frame_id = ?1",
            params![frame_id],
        )
        .map_err(to_string)?;
        conn.execute(
            "DELETE FROM continue_ordered_evidence_spans WHERE frame_id = ?1",
            params![frame_id],
        )
        .map_err(to_string)?;
    }

    let mut all_spans = Vec::new();
    let mut result = TaskTurnBuildResult::default();
    for context in &contexts {
        let raw = load_raw_spans(conn, context).map_err(|error| {
            format!(
                "task-turn raw spans for frame {}: {error}",
                context.frame_id
            )
        })?;
        let mut spans = build_ordered_spans(context, raw);
        for span in &spans {
            persist_span(conn, span)
                .map_err(|error| format!("task-turn persist span {}: {error}", span.span_id))?;
        }
        result.span_count += spans.len();
        all_spans.append(&mut spans);
        all_spans.sort_by_key(|span| {
            (
                span.observed_at_ms,
                span.reading_order,
                span.span_id.clone(),
            )
        });

        let selection = select_latest_turn(context, &all_spans);
        persist_selection(conn, context, &selection).map_err(|error| {
            format!("task-turn persist selection {}: {error}", context.frame_id)
        })?;
        update_surface_snapshot_sample(conn, context, &selection)?;
        let semantic_text = selection_semantic_text(&selection);
        if !semantic_text.is_empty() {
            result
                .semantic_text_by_frame
                .insert(context.frame_id.clone(), semantic_text);
        }
        result.selection_count += 1;
    }
    Ok(result)
}

fn load_frame_context(conn: &Connection, frame_id: &str) -> Result<FrameContext, String> {
    let (session_id, observed_at_ms, app_name, bundle_id, window_id, privacy_status) = conn
        .query_row(
            "SELECT session_id, captured_at, app_name, app_bundle_id, window_id,
                    COALESCE(privacy_status, 'normal')
             FROM frames WHERE id = ?1",
            params![frame_id],
            |row| {
                Ok((
                    row.get::<_, Option<String>>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, Option<i64>>(4)?,
                    row.get::<_, String>(5)?,
                ))
            },
        )
        .map_err(to_string)?;
    let artifact = conn
        .query_row(
            "SELECT o.artifact_id, a.stable_key, a.artifact_kind
             FROM continue_artifact_observations o
             JOIN continue_artifacts a ON a.id = o.artifact_id
             WHERE o.frame_id = ?1 ORDER BY o.timestamp_ms DESC LIMIT 1",
            params![frame_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )
        .optional()
        .map_err(to_string)?;
    let object_types = load_strings(
        conn,
        "SELECT object_type FROM app_contexts WHERE frame_id = ?1",
        frame_id,
    )?;
    let adapters = load_strings(
        conn,
        "SELECT adapter_id FROM app_contexts WHERE frame_id = ?1",
        frame_id,
    )?;
    let event_types = if table_exists(conn, "ui_events")? {
        let mut statement = conn
            .prepare(
                "SELECT event_type FROM ui_events
                 WHERE session_id = ?1 AND ts_ms BETWEEN ?2 - 2500 AND ?2 + 2500",
            )
            .map_err(to_string)?;
        let rows = statement
            .query_map(params![session_id, observed_at_ms], |row| {
                row.get::<_, String>(0)
            })
            .map_err(to_string)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(to_string)?
    } else {
        Vec::new()
    };
    let causal_typing = load_causal_typing_attribution(
        conn,
        frame_id,
        session_id.as_deref(),
        observed_at_ms,
        bundle_id.as_deref(),
        app_name.as_deref(),
        window_id,
        &privacy_status,
    )?;
    let pre_frame_text_hashes = causal_typing
        .as_ref()
        .and_then(|attribution| attribution.pre_frame_id.as_deref())
        .map(|pre_frame_id| load_frame_text_hashes(conn, pre_frame_id))
        .transpose()?
        .unwrap_or_default();
    let family_haystack = format!(
        "{} {} {} {} {}",
        app_name.as_deref().unwrap_or(""),
        bundle_id.as_deref().unwrap_or(""),
        artifact.as_ref().map(|item| item.2.as_str()).unwrap_or(""),
        object_types.join(" "),
        adapters.join(" ")
    )
    .to_ascii_lowercase();
    let family = if contains_any(
        &family_haystack,
        &[
            "agent",
            "codex",
            "chatgpt",
            "claude",
            "gemini",
            "chat_conversation",
        ],
    ) {
        SurfaceFamily::AgentChat
    } else if family_haystack.contains("browser") && family_haystack.contains("chat") {
        SurfaceFamily::BrowserChat
    } else if contains_any(
        &family_haystack,
        &["editor", "xcode", "code_editor", "vscode"],
    ) {
        SurfaceFamily::Editor
    } else if contains_any(&family_haystack, &["terminal", "shell", "iterm"]) {
        SurfaceFamily::Terminal
    } else {
        SurfaceFamily::Other
    };
    Ok(FrameContext {
        frame_id: frame_id.to_string(),
        session_id,
        observed_at_ms,
        app_name,
        bundle_id,
        window_id,
        privacy_status,
        artifact_id: artifact.as_ref().map(|item| item.0.clone()),
        surface_key: artifact.map(|item| item.1),
        family,
        has_agent_status_event: event_types.iter().any(|event| {
            contains_any(
                &event.to_ascii_lowercase(),
                &["ax_value_changed", "agent_status", "stream", "thinking"],
            )
        }),
        causal_typing,
        pre_frame_text_hashes,
    })
}

#[derive(Debug)]
struct TypingAttributionCandidate {
    id: String,
    commit_signal: Option<String>,
    started_at_ms: i64,
    ended_at_ms: i64,
    pre_frame_id: Option<String>,
    post_frame_id: Option<String>,
    app_bundle_id: Option<String>,
    app_name: Option<String>,
    window_id: Option<i64>,
    window_title: Option<String>,
    association_source: Option<String>,
    association_confidence: Option<f64>,
    capture_trigger_id: Option<String>,
    commit_event_id: Option<String>,
}

#[allow(clippy::too_many_arguments)]
fn load_causal_typing_attribution(
    conn: &Connection,
    frame_id: &str,
    session_id: Option<&str>,
    observed_at_ms: i64,
    frame_bundle_id: Option<&str>,
    frame_app_name: Option<&str>,
    frame_window_id: Option<i64>,
    privacy_status: &str,
) -> Result<Option<TypingBurstCausalAttribution>, String> {
    if is_privacy_blocked(privacy_status)
        || !table_exists(conn, "typing_bursts")?
        || !column_exists(conn, "typing_bursts", "post_frame_id")?
        || !column_exists(conn, "typing_bursts", "committed")?
    {
        return Ok(None);
    }
    let frame_window_title = conn
        .query_row(
            "SELECT window_name FROM frames WHERE id = ?1",
            params![frame_id],
            |row| row.get::<_, Option<String>>(0),
        )
        .optional()
        .map_err(to_string)?
        .flatten();
    let source_expr = column_or(
        conn,
        "typing_bursts",
        "post_frame_association_source",
        "NULL",
    )?;
    let confidence_expr = column_or(
        conn,
        "typing_bursts",
        "post_frame_association_confidence",
        "NULL",
    )?;
    let trigger_expr = column_or(conn, "typing_bursts", "capture_trigger_id", "NULL")?;
    let commit_event_expr = column_or(conn, "typing_bursts", "commit_event_id", "NULL")?;
    // Older databases and the broad continuation test corpus predate the
    // surface-identity columns. Treat their absence as unavailable evidence;
    // schema compatibility must not turn an optional causal signal into a
    // failure of the entire Continue pipeline.
    let app_bundle_expr = column_or(conn, "typing_bursts", "app_bundle_id", "NULL")?;
    let app_name_expr = column_or(conn, "typing_bursts", "app_name", "NULL")?;
    let window_id_expr = column_or(conn, "typing_bursts", "window_id", "NULL")?;
    let window_title_expr = column_or(conn, "typing_bursts", "window_title", "NULL")?;
    let raw_text_guard = if column_exists(conn, "typing_bursts", "raw_text_captured")? {
        "AND COALESCE(raw_text_captured, 0) = 0"
    } else {
        ""
    };
    let sql = format!(
        "SELECT id, commit_signal, started_at_ms, ended_at_ms, pre_frame_id,
                post_frame_id, {app_bundle_expr}, {app_name_expr},
                {window_id_expr}, {window_title_expr},
                {source_expr}, {confidence_expr}, {trigger_expr}, {commit_event_expr}
         FROM typing_bursts
         WHERE post_frame_id = ?1 AND committed = 1 {raw_text_guard}
           AND (?2 IS NULL OR session_id = ?2)
         ORDER BY ended_at_ms DESC, id DESC"
    );
    let mut statement = conn.prepare(&sql).map_err(to_string)?;
    let explicit_rows = statement
        .query_map(params![frame_id, session_id], typing_candidate_from_row)
        .map_err(to_string)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(to_string)?;
    let mut explicit = explicit_rows
        .into_iter()
        .filter(|candidate| {
            typing_surface_matches(
                candidate,
                frame_bundle_id,
                frame_app_name,
                frame_window_id,
                frame_window_title.as_deref(),
            ) && (0..=MAX_LEGACY_TYPING_TO_FRAME_MS)
                .contains(&observed_at_ms.saturating_sub(candidate.ended_at_ms))
        })
        .collect::<Vec<_>>();
    explicit.sort_by(|left, right| {
        right
            .association_confidence
            .unwrap_or(0.90)
            .total_cmp(&left.association_confidence.unwrap_or(0.90))
            .then(right.ended_at_ms.cmp(&left.ended_at_ms))
            .then(left.id.cmp(&right.id))
    });
    if let Some(candidate) = explicit.into_iter().next() {
        return Ok(Some(attribution_from_candidate(
            candidate,
            frame_id,
            false,
            observed_at_ms,
        )));
    }

    load_legacy_bounded_typing_attribution(
        conn,
        frame_id,
        session_id,
        observed_at_ms,
        frame_bundle_id,
        frame_app_name,
        frame_window_id,
        frame_window_title.as_deref(),
        raw_text_guard,
        &source_expr,
        &confidence_expr,
        &trigger_expr,
        &commit_event_expr,
        &app_bundle_expr,
        &app_name_expr,
        &window_id_expr,
        &window_title_expr,
    )
}

#[allow(clippy::too_many_arguments)]
fn load_legacy_bounded_typing_attribution(
    conn: &Connection,
    frame_id: &str,
    session_id: Option<&str>,
    observed_at_ms: i64,
    frame_bundle_id: Option<&str>,
    frame_app_name: Option<&str>,
    frame_window_id: Option<i64>,
    frame_window_title: Option<&str>,
    raw_text_guard: &str,
    source_expr: &str,
    confidence_expr: &str,
    trigger_expr: &str,
    commit_event_expr: &str,
    app_bundle_expr: &str,
    app_name_expr: &str,
    window_id_expr: &str,
    window_title_expr: &str,
) -> Result<Option<TypingBurstCausalAttribution>, String> {
    let previous_frame_expr = if column_exists(conn, "frames", "previous_frame_id")? {
        "CAST(f.previous_frame_id AS TEXT)"
    } else {
        "NULL"
    };
    let can_join_trigger = column_exists(conn, "frames", "capture_trigger_id")?
        && table_exists(conn, "capture_triggers")?
        && column_exists(conn, "capture_triggers", "id")?
        && column_exists(conn, "capture_triggers", "pre_frame_id")?;
    let trigger_pre_frame_expr = if can_join_trigger {
        "ct.pre_frame_id"
    } else {
        "NULL"
    };
    let trigger_join = if can_join_trigger {
        "LEFT JOIN capture_triggers ct ON ct.id = f.capture_trigger_id"
    } else {
        ""
    };
    let predecessor_sql = format!(
        "SELECT {previous_frame_expr}, {trigger_pre_frame_expr}\n         FROM frames f\n         {trigger_join}\n         WHERE f.id = ?1"
    );
    let (previous_frame_id, trigger_pre_frame_id) = conn
        .query_row(&predecessor_sql, params![frame_id], |row| {
            Ok((
                row.get::<_, Option<String>>(0)?,
                row.get::<_, Option<String>>(1)?,
            ))
        })
        .optional()
        .map_err(to_string)?
        .unwrap_or((None, None));
    let sql = format!(
        "SELECT id, commit_signal, started_at_ms, ended_at_ms, pre_frame_id,
                post_frame_id, {app_bundle_expr}, {app_name_expr},
                {window_id_expr}, {window_title_expr},
                {source_expr}, {confidence_expr}, {trigger_expr}, {commit_event_expr}
         FROM typing_bursts
         WHERE post_frame_id IS NULL AND committed = 1 {raw_text_guard}
           AND (?1 IS NULL OR session_id = ?1)
           AND ended_at_ms BETWEEN ?2 AND ?3
         ORDER BY ended_at_ms DESC, id DESC
         LIMIT 8"
    );
    let mut statement = conn.prepare(&sql).map_err(to_string)?;
    let candidates = statement
        .query_map(
            params![
                session_id,
                observed_at_ms.saturating_sub(MAX_LEGACY_TYPING_TO_FRAME_MS),
                observed_at_ms
            ],
            typing_candidate_from_row,
        )
        .map_err(to_string)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(to_string)?
        .into_iter()
        .filter(|candidate| {
            let direct_predecessor = candidate.pre_frame_id.as_deref().is_some_and(|pre| {
                previous_frame_id.as_deref() == Some(pre)
                    || trigger_pre_frame_id.as_deref() == Some(pre)
            });
            let close_unique_window = observed_at_ms.saturating_sub(candidate.ended_at_ms) <= 2_500;
            (direct_predecessor || close_unique_window)
                && typing_surface_matches(
                    candidate,
                    frame_bundle_id,
                    frame_app_name,
                    frame_window_id,
                    frame_window_title,
                )
        })
        .collect::<Vec<_>>();
    if candidates.len() != 1 {
        return Ok(None);
    }
    Ok(candidates
        .into_iter()
        .next()
        .map(|candidate| attribution_from_candidate(candidate, frame_id, true, observed_at_ms)))
}

fn typing_candidate_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<TypingAttributionCandidate> {
    Ok(TypingAttributionCandidate {
        id: row.get(0)?,
        commit_signal: row.get(1)?,
        started_at_ms: row.get(2)?,
        ended_at_ms: row.get(3)?,
        pre_frame_id: row.get(4)?,
        post_frame_id: row.get(5)?,
        app_bundle_id: row.get(6)?,
        app_name: row.get(7)?,
        window_id: row.get(8)?,
        window_title: row.get(9)?,
        association_source: row.get(10)?,
        association_confidence: row.get(11)?,
        capture_trigger_id: row.get(12)?,
        commit_event_id: row.get(13)?,
    })
}

fn typing_surface_matches(
    candidate: &TypingAttributionCandidate,
    frame_bundle_id: Option<&str>,
    frame_app_name: Option<&str>,
    frame_window_id: Option<i64>,
    frame_window_title: Option<&str>,
) -> bool {
    let app_matches = match (candidate.app_bundle_id.as_deref(), frame_bundle_id) {
        (Some(left), Some(right)) if !left.trim().is_empty() && !right.trim().is_empty() => {
            left == right
        }
        _ => normalized_identity(candidate.app_name.as_deref())
            .zip(normalized_identity(frame_app_name))
            .is_some_and(|(left, right)| left == right),
    };
    let window_matches = match (candidate.window_id, frame_window_id) {
        (Some(left), Some(right)) => left == right,
        _ => normalized_identity(candidate.window_title.as_deref())
            .zip(normalized_identity(frame_window_title))
            .is_some_and(|(left, right)| left == right),
    };
    app_matches && window_matches
}

fn normalized_identity(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_ascii_lowercase())
}

fn attribution_from_candidate(
    candidate: TypingAttributionCandidate,
    frame_id: &str,
    inferred: bool,
    observed_at_ms: i64,
) -> TypingBurstCausalAttribution {
    TypingBurstCausalAttribution {
        typing_burst_id: candidate.id,
        commit_signal: candidate.commit_signal,
        started_at_ms: candidate.started_at_ms,
        ended_at_ms: candidate.ended_at_ms,
        pre_frame_id: candidate.pre_frame_id,
        post_frame_id: candidate.post_frame_id,
        bounded_inferred_frame_id: inferred.then(|| frame_id.to_string()),
        surface_match_result: "exact_app_and_window".to_string(),
        temporal_distance_ms: observed_at_ms.saturating_sub(candidate.ended_at_ms),
        association_source: if inferred {
            "legacy_bounded_recovery".to_string()
        } else {
            candidate
                .association_source
                .unwrap_or_else(|| "stored_post_frame".to_string())
        },
        association_confidence: round_confidence(if inferred {
            0.78
        } else {
            candidate.association_confidence.unwrap_or(0.90)
        }),
        rejection_reasons: if inferred {
            vec!["legacy_null_post_frame_not_persisted".to_string()]
        } else {
            Vec::new()
        },
        capture_trigger_id: candidate.capture_trigger_id,
        commit_event_id: candidate.commit_event_id,
    }
}

fn load_frame_text_hashes(conn: &Connection, frame_id: &str) -> Result<BTreeSet<String>, String> {
    let mut hashes = BTreeSet::new();
    for (table, text_expr) in [
        (
            "ax_nodes",
            "COALESCE(NULLIF(value,''), NULLIF(title,''), NULLIF(description,''))",
        ),
        ("ocr_spans", "text"),
        ("content_units", "text"),
    ] {
        if !table_exists(conn, table)? || !column_exists(conn, table, "frame_id")? {
            continue;
        }
        let sql = format!("SELECT {text_expr} FROM {table} WHERE frame_id = ?1");
        let mut statement = conn.prepare(&sql).map_err(to_string)?;
        let values = statement
            .query_map(params![frame_id], |row| row.get::<_, Option<String>>(0))
            .map_err(to_string)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(to_string)?;
        for text in values
            .into_iter()
            .flatten()
            .filter(|text| !text.trim().is_empty())
        {
            hashes.insert(text_hash(&text));
        }
    }
    Ok(hashes)
}

fn load_raw_spans(conn: &Connection, context: &FrameContext) -> Result<Vec<RawSpan>, String> {
    let mut spans = Vec::new();
    if table_exists(conn, "ax_nodes")?
        && ["value", "title", "description"]
            .into_iter()
            .any(|column| column_exists(conn, "ax_nodes", column).unwrap_or(false))
    {
        let order_column = if column_exists(conn, "ax_nodes", "tree_order")? {
            "tree_order"
        } else if column_exists(conn, "ax_nodes", "depth")? {
            "depth"
        } else {
            "0 + 0"
        };
        let parent_expr = column_or(conn, "ax_nodes", "parent_id", "NULL")?;
        let role_expr = column_or(conn, "ax_nodes", "role", "NULL")?;
        let subrole_expr = column_or(conn, "ax_nodes", "subrole", "NULL")?;
        let role_description_expr = column_or(conn, "ax_nodes", "role_description", "NULL")?;
        let identifier_expr = column_or(conn, "ax_nodes", "identifier", "NULL")?;
        let value_expr = column_or(conn, "ax_nodes", "value", "NULL")?;
        let title_expr = column_or(conn, "ax_nodes", "title", "NULL")?;
        let description_expr = column_or(conn, "ax_nodes", "description", "NULL")?;
        let focused_expr = column_or(conn, "ax_nodes", "focused", "0")?;
        let selected_expr = column_or(conn, "ax_nodes", "selected", "0")?;
        let bounds_x_expr = column_or(conn, "ax_nodes", "bounds_x", "NULL")?;
        let bounds_y_expr = column_or(conn, "ax_nodes", "bounds_y", "NULL")?;
        let bounds_w_expr = column_or(conn, "ax_nodes", "bounds_w", "NULL")?;
        let bounds_h_expr = column_or(conn, "ax_nodes", "bounds_h", "NULL")?;
        let window_id_expr = column_or(conn, "ax_nodes", "window_id", "NULL")?;
        let actions_expr = column_or(conn, "ax_nodes", "actions_json", "'[]'")?;
        let enabled_expr = column_or(conn, "ax_nodes", "enabled", "NULL")?;
        let sql = format!(
            "SELECT id, {parent_expr}, {role_expr}, {subrole_expr},
                    {role_description_expr}, {identifier_expr},
                    COALESCE(NULLIF({value_expr},''), NULLIF({title_expr},''),
                             NULLIF({description_expr},'')),
                    {focused_expr}, {selected_expr}, {bounds_x_expr}, {bounds_y_expr},
                    {bounds_w_expr}, {bounds_h_expr}, COALESCE({order_column}, 0),
                    {window_id_expr}, {actions_expr}, {enabled_expr}
             FROM ax_nodes WHERE frame_id = ?1 ORDER BY COALESCE({order_column}, 0), id"
        );
        let mut statement = conn.prepare(&sql).map_err(to_string)?;
        let rows = statement
            .query_map(params![context.frame_id], |row| {
                let id: String = row.get(0)?;
                let text = row.get::<_, Option<String>>(6)?.unwrap_or_default();
                Ok(RawSpan {
                    source: source_ref(EvidenceSourceKind::AccessibilityNode, &id, &text),
                    text,
                    source_scope: "active_window".to_string(),
                    ownership_kind: "ActiveWindowOwned".to_string(),
                    owner_window_id: row.get(14)?,
                    owner_app_id: context
                        .bundle_id
                        .clone()
                        .or_else(|| context.app_name.clone()),
                    geometry: optional_geometry(
                        row.get(9)?,
                        row.get(10)?,
                        row.get(11)?,
                        row.get(12)?,
                    ),
                    parent_id: row.get(1)?,
                    source_order: row.get(13)?,
                    focused: row.get::<_, Option<i64>>(7)?.unwrap_or(0) != 0,
                    selected: row.get::<_, Option<i64>>(8)?.unwrap_or(0) != 0,
                    source_confidence: 0.88,
                    ownership_confidence: 0.90,
                    active_match_confidence: 0.90,
                    structural_hint: [
                        Some(id.as_str()),
                        row.get::<_, Option<String>>(2)?.as_deref(),
                        row.get::<_, Option<String>>(3)?.as_deref(),
                        row.get::<_, Option<String>>(4)?.as_deref(),
                        row.get::<_, Option<String>>(5)?.as_deref(),
                        row.get::<_, Option<String>>(1)?.as_deref(),
                        row.get::<_, Option<String>>(15)?.as_deref(),
                        row.get::<_, Option<i64>>(16)?.map(|enabled| {
                            if enabled != 0 {
                                "enabled"
                            } else {
                                "disabled"
                            }
                        }),
                    ]
                    .into_iter()
                    .flatten()
                    .collect::<Vec<_>>()
                    .join(" "),
                    semantic_role: None,
                    quality_flags: Vec::new(),
                })
            })
            .map_err(to_string)?;
        spans.extend(rows.collect::<Result<Vec<_>, _>>().map_err(to_string)?);
    }

    if table_exists(conn, "content_units")? {
        let order_expr = if column_exists(conn, "content_units", "source_order")? {
            "COALESCE(source_order, 0)"
        } else {
            "0 + 0"
        };
        let source_scope_expr = column_or(conn, "content_units", "source_scope", "NULL")?;
        let ownership_kind_expr = column_or(conn, "content_units", "ownership_kind", "NULL")?;
        let ownership_confidence_expr =
            column_or(conn, "content_units", "ownership_confidence", "NULL")?;
        let active_match_expr = column_or(
            conn,
            "content_units",
            "active_artifact_match_confidence",
            "NULL",
        )?;
        let owner_window_id_expr = column_or(conn, "content_units", "owner_window_id", "NULL")?;
        let owner_bundle_id_expr = column_or(conn, "content_units", "owner_bundle_id", "NULL")?;
        let quality_flags_expr = column_or(conn, "content_units", "quality_flags_json", "'[]'")?;
        let sql = format!(
            "SELECT id, source, unit_type, semantic_role, text, bounds_x, bounds_y,
                    bounds_w, bounds_h, {source_scope_expr}, {ownership_kind_expr},
                    {ownership_confidence_expr}, {active_match_expr},
                    {owner_window_id_expr}, {owner_bundle_id_expr}, {quality_flags_expr},
                    {order_expr}
             FROM content_units WHERE frame_id = ?1 ORDER BY {order_expr}, id"
        );
        let mut statement = conn.prepare(&sql).map_err(to_string)?;
        let rows = statement
            .query_map(params![context.frame_id], |row| {
                let id: String = row.get(0)?;
                let text = row.get::<_, Option<String>>(4)?.unwrap_or_default();
                Ok(RawSpan {
                    source: source_ref(EvidenceSourceKind::ContentUnit, &id, &text),
                    text,
                    source_scope: row
                        .get::<_, Option<String>>(9)?
                        .unwrap_or_else(|| "unknown".to_string()),
                    ownership_kind: row
                        .get::<_, Option<String>>(10)?
                        .unwrap_or_else(|| "Unknown".to_string()),
                    owner_window_id: row.get(13)?,
                    owner_app_id: row.get(14)?,
                    geometry: optional_geometry(row.get(5)?, row.get(6)?, row.get(7)?, row.get(8)?),
                    parent_id: None,
                    source_order: row.get(16)?,
                    focused: false,
                    selected: false,
                    source_confidence: row.get::<_, Option<f64>>(11)?.unwrap_or(0.72),
                    ownership_confidence: row.get::<_, Option<f64>>(11)?.unwrap_or(0.60),
                    active_match_confidence: row.get::<_, Option<f64>>(12)?.unwrap_or(0.65),
                    structural_hint: format!(
                        "{} {} {} {}",
                        id,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, Option<String>>(3)?.unwrap_or_default()
                    ),
                    semantic_role: row.get(3)?,
                    quality_flags: parse_string_array(row.get::<_, Option<String>>(15)?.as_deref()),
                })
            })
            .map_err(to_string)?;
        spans.extend(rows.collect::<Result<Vec<_>, _>>().map_err(to_string)?);
    }

    if table_exists(conn, "ocr_spans")? {
        let source_scope_expr = column_or(conn, "ocr_spans", "source_scope", "NULL")?;
        let ownership_kind_expr = column_or(conn, "ocr_spans", "ownership_kind", "NULL")?;
        let ownership_confidence_expr =
            column_or(conn, "ocr_spans", "ownership_confidence", "NULL")?;
        let active_match_expr = column_or(
            conn,
            "ocr_spans",
            "active_artifact_match_confidence",
            "NULL",
        )?;
        let owner_window_id_expr = column_or(conn, "ocr_spans", "owner_window_id", "NULL")?;
        let owner_bundle_id_expr = column_or(conn, "ocr_spans", "owner_bundle_id", "NULL")?;
        let quality_flags_expr = column_or(conn, "ocr_spans", "quality_flags_json", "'[]'")?;
        let block_index_expr = column_or(conn, "ocr_spans", "block_index", "0 + 0")?;
        let line_index_expr = column_or(conn, "ocr_spans", "line_index", "0 + 0")?;
        let word_index_expr = column_or(conn, "ocr_spans", "word_index", "0 + 0")?;
        let sql = format!(
            "SELECT id, text, confidence, bounds_x, bounds_y, bounds_w, bounds_h,
                    {source_scope_expr}, {ownership_kind_expr}, {ownership_confidence_expr},
                    {active_match_expr}, {owner_window_id_expr}, {owner_bundle_id_expr},
                    {quality_flags_expr}, COALESCE({block_index_expr},0),
                    COALESCE({line_index_expr},0), COALESCE({word_index_expr},0)
             FROM ocr_spans WHERE frame_id = ?1
             ORDER BY {block_index_expr}, {line_index_expr}, {word_index_expr}, id"
        );
        let mut statement = conn.prepare(&sql).map_err(to_string)?;
        let rows = statement
            .query_map(params![context.frame_id], |row| {
                let id: String = row.get(0)?;
                let text: String = row.get(1)?;
                let block: i64 = row.get(14)?;
                let line: i64 = row.get(15)?;
                let word: i64 = row.get(16)?;
                Ok(RawSpan {
                    source: source_ref(EvidenceSourceKind::OcrSpan, &id, &text),
                    text,
                    source_scope: row
                        .get::<_, Option<String>>(7)?
                        .unwrap_or_else(|| "unknown".to_string()),
                    ownership_kind: row
                        .get::<_, Option<String>>(8)?
                        .unwrap_or_else(|| "Unknown".to_string()),
                    owner_window_id: row.get(11)?,
                    owner_app_id: row.get(12)?,
                    geometry: optional_geometry(row.get(3)?, row.get(4)?, row.get(5)?, row.get(6)?),
                    parent_id: None,
                    source_order: block * 1_000_000 + line * 1_000 + word,
                    focused: false,
                    selected: false,
                    source_confidence: row.get::<_, Option<f64>>(2)?.unwrap_or(0.66),
                    ownership_confidence: row.get::<_, Option<f64>>(9)?.unwrap_or(0.45),
                    active_match_confidence: row.get::<_, Option<f64>>(10)?.unwrap_or(0.45),
                    structural_hint: id,
                    semantic_role: None,
                    quality_flags: parse_string_array(row.get::<_, Option<String>>(13)?.as_deref()),
                })
            })
            .map_err(to_string)?;
        spans.extend(rows.collect::<Result<Vec<_>, _>>().map_err(to_string)?);
    }

    spans.retain(|span| !span.text.trim().is_empty());
    if spans.is_empty() {
        let fallback_text = if table_exists(conn, "frame_text_resolutions")? {
            conn.query_row(
                "SELECT active_text FROM frame_text_resolutions WHERE frame_id = ?1",
                params![context.frame_id],
                |row| row.get::<_, Option<String>>(0),
            )
            .optional()
            .map_err(to_string)?
            .flatten()
        } else {
            conn.query_row(
                "SELECT full_text FROM frames WHERE id = ?1",
                params![context.frame_id],
                |row| row.get::<_, Option<String>>(0),
            )
            .optional()
            .map_err(to_string)?
            .flatten()
        };
        if let Some(text) = fallback_text.filter(|text| !text.trim().is_empty()) {
            let id = format!("frame:{}:active_text", context.frame_id);
            spans.push(RawSpan {
                source: source_ref(EvidenceSourceKind::FlattenedTextFallback, &id, &text),
                text,
                source_scope: "active_window".to_string(),
                ownership_kind: "ActiveWindowOwned".to_string(),
                owner_window_id: context.window_id,
                owner_app_id: context
                    .bundle_id
                    .clone()
                    .or_else(|| context.app_name.clone()),
                geometry: None,
                parent_id: None,
                source_order: 0,
                focused: false,
                selected: false,
                source_confidence: 0.30,
                ownership_confidence: 0.45,
                active_match_confidence: 0.40,
                structural_hint: "flattened_text_fallback".to_string(),
                semantic_role: None,
                quality_flags: vec!["flattened_text_fallback".to_string()],
            });
        }
    }
    Ok(spans)
}

fn build_ordered_spans(context: &FrameContext, raw: Vec<RawSpan>) -> Vec<OrderedEvidenceSpan> {
    let mut grouped: Vec<(RawSpan, Vec<RawSpan>)> = Vec::new();
    for candidate in raw {
        let normalized = normalize_text(&candidate.text);
        if normalized.is_empty() {
            continue;
        }
        if let Some((primary, contributors)) = grouped.iter_mut().find(|(primary, contributors)| {
            normalize_text(&primary.text) == normalized
                && sources_can_merge(primary, &candidate, contributors)
        }) {
            if candidate.source.source_kind.priority() > primary.source.source_kind.priority() {
                let previous = std::mem::replace(primary, candidate);
                contributors.push(previous);
            } else {
                contributors.push(candidate);
            }
        } else {
            grouped.push((candidate, Vec::new()));
        }
    }

    let pane_ids = assign_panes(grouped.iter().map(|item| item.0.geometry).collect());
    let mut spans = grouped
        .into_iter()
        .zip(pane_ids)
        .map(|((primary, contributors), pane_id)| {
            span_from_group(context, primary, contributors, pane_id)
        })
        .collect::<Vec<_>>();
    spans.sort_by(|left, right| {
        left.pane_id
            .cmp(&right.pane_id)
            .then_with(|| geometry_order(left.geometry, right.geometry))
            .then(left.local_reading_order.cmp(&right.local_reading_order))
            .then(left.span_id.cmp(&right.span_id))
    });
    let mut pane_orders = HashMap::<String, i64>::new();
    for (index, span) in spans.iter_mut().enumerate() {
        span.reading_order = index as i64;
        let next = pane_orders.entry(span.pane_id.clone()).or_default();
        span.local_reading_order = *next;
        *next += 1;
    }
    apply_surface_geometry_adapter(context, &mut spans);
    spans
}

fn apply_surface_geometry_adapter(context: &FrameContext, spans: &mut [OrderedEvidenceSpan]) {
    if !matches!(
        context.family,
        SurfaceFamily::AgentChat | SurfaceFamily::BrowserChat
    ) {
        return;
    }
    let mut pane_bounds = BTreeMap::<String, (f64, f64, usize)>::new();
    for span in spans.iter() {
        let Some(geometry) = span.geometry else {
            continue;
        };
        let entry = pane_bounds.entry(span.pane_id.clone()).or_insert((
            geometry.x,
            geometry.x + geometry.width,
            0,
        ));
        entry.0 = entry.0.min(geometry.x);
        entry.1 = entry.1.max(geometry.x + geometry.width);
        entry.2 += 1;
    }
    let mut panes = pane_bounds
        .iter()
        .map(|(id, (min_x, max_x, count))| (id.clone(), *min_x, *max_x, *count))
        .collect::<Vec<_>>();
    panes.sort_by(|left, right| left.1.total_cmp(&right.1));
    let Some(conversation_pane) = (match panes.len() {
        0 => None,
        1 => Some(panes[0].0.clone()),
        2 => Some(
            panes
                .iter()
                .max_by_key(|pane| pane.3)
                .map(|pane| pane.0.clone())
                .unwrap_or_else(|| panes[0].0.clone()),
        ),
        _ => Some(panes[panes.len() / 2].0.clone()),
    }) else {
        return;
    };
    if panes.len() >= 3 {
        let navigation_pane = &panes[0].0;
        let terminal_pane = &panes[panes.len() - 1].0;
        for span in spans
            .iter_mut()
            .filter(|span| span.region_role == RegionRole::Unknown)
        {
            if &span.pane_id == navigation_pane {
                span.region_role = RegionRole::Navigation;
                span.conversational_role = ConversationalRole::NonConversation;
                span.region_confidence = 0.72;
                span.speaker_confidence = 0.92;
                span.reason_codes
                    .push("agent_surface_left_navigation_pane".to_string());
            } else if &span.pane_id == terminal_pane {
                span.region_role = RegionRole::TerminalOutput;
                span.conversational_role = ConversationalRole::NonConversation;
                span.region_confidence = 0.70;
                span.speaker_confidence = 0.92;
                span.reason_codes
                    .push("agent_surface_right_terminal_pane".to_string());
            }
        }
    }
    let Some((_, min_x, max_x, _)) = panes.iter().find(|pane| pane.0 == conversation_pane) else {
        return;
    };
    let pane_width = (max_x - min_x).max(1.0);
    let right_alignment_threshold = min_x + pane_width * 0.58;
    let status_orders = spans
        .iter()
        .filter(|span| {
            span.pane_id == conversation_pane
                && contains_any(
                    &normalize_text(&span.text),
                    &[
                        "working",
                        "thinking",
                        "streaming",
                        "analyzing",
                        "generating",
                    ],
                )
        })
        .map(|span| span.reading_order)
        .collect::<Vec<_>>();
    for span in spans
        .iter_mut()
        .filter(|span| span.pane_id == conversation_pane && span.region_role == RegionRole::Unknown)
    {
        if status_orders.contains(&span.reading_order) {
            span.region_role = RegionRole::AgentStatus;
            span.conversational_role = ConversationalRole::AssistantOrAgent;
            span.region_confidence = 0.82;
            span.speaker_confidence = 0.78;
            span.reason_codes
                .push("agent_surface_status_marker".to_string());
        }
    }
    let composer_geometry = spans
        .iter()
        .find(|span| span.region_role == RegionRole::Composer)
        .and_then(|span| span.geometry);
    if let Some(composer) = composer_geometry {
        for span in spans.iter_mut().filter(|span| {
            span.region_role == RegionRole::Unknown
                && span.text.split_whitespace().count() <= 5
                && span.geometry.is_some_and(|geometry| {
                    geometry.y >= composer.y - 8.0
                        && geometry.y <= composer.y + composer.height + 120.0
                })
        }) {
            span.region_role = RegionRole::Control;
            span.conversational_role = ConversationalRole::NonConversation;
            span.region_confidence = 0.78;
            span.speaker_confidence = 0.92;
            span.reason_codes
                .push("composer_adjacent_action_control".to_string());
        }
    }
    let geometric_user_candidates = spans
        .iter()
        .filter(|span| {
            span.pane_id == conversation_pane
                && span.region_role == RegionRole::Unknown
                && user_goal_geometry_candidate_is_eligible(span)
                && span
                    .geometry
                    .is_some_and(|geometry| geometry.center_x() >= right_alignment_threshold)
                && span.text.split_whitespace().count() >= 3
        })
        .collect::<Vec<_>>();
    let causal_user_order = context.causal_typing.as_ref().and_then(|attribution| {
        if attribution.association_confidence < MIN_CAUSAL_TYPING_CONFIDENCE
            || attribution.surface_match_result != "exact_app_and_window"
        {
            return None;
        }
        let novel = geometric_user_candidates
            .iter()
            .filter(|span| !context.pre_frame_text_hashes.contains(&span.text_hash))
            .collect::<Vec<_>>();
        (novel.len() == 1).then(|| novel[0].reading_order)
    });
    let latest_user_order = causal_user_order.or_else(|| {
        geometric_user_candidates
            .iter()
            .max_by_key(|span| span.reading_order)
            .map(|span| span.reading_order)
    });
    if let Some(user_order) = latest_user_order {
        if let Some(user) = spans
            .iter_mut()
            .find(|span| span.reading_order == user_order)
        {
            user.region_role = RegionRole::UserMessage;
            user.conversational_role = ConversationalRole::User;
            let confidence = if causal_user_order == Some(user_order) {
                context
                    .causal_typing
                    .as_ref()
                    .map(|attribution| attribution.association_confidence.min(0.82))
                    .unwrap_or(0.58)
            } else {
                0.58
            };
            user.region_confidence = confidence;
            user.speaker_confidence = confidence;
            user.reason_codes
                .push(if causal_user_order == Some(user_order) {
                    "causal_typing_novel_post_frame_span".to_string()
                } else {
                    "agent_surface_right_aligned_turn_unconfirmed".to_string()
                });
        }
        let has_following_status =
            context.has_agent_status_event || status_orders.iter().any(|order| *order > user_order);
        if has_following_status {
            if let Some(agent) = spans
                .iter_mut()
                .filter(|span| {
                    span.pane_id == conversation_pane
                        && span.region_role == RegionRole::Unknown
                        && span.reading_order > user_order
                        && span
                            .geometry
                            .is_some_and(|geometry| geometry.center_x() < right_alignment_threshold)
                })
                .max_by_key(|span| span.reading_order)
            {
                agent.region_role = RegionRole::AgentStatus;
                agent.conversational_role = ConversationalRole::AssistantOrAgent;
                agent.region_confidence = 0.76;
                agent.speaker_confidence = 0.72;
                agent
                    .reason_codes
                    .push("agent_surface_following_status_turn".to_string());
            }
        }
        for prior in spans.iter_mut().filter(|span| {
            span.pane_id == conversation_pane
                && span.region_role == RegionRole::Unknown
                && span.reading_order < user_order
                && span.text.split_whitespace().count() >= 4
        }) {
            prior.region_role = RegionRole::ConversationHistory;
            prior.conversational_role = ConversationalRole::AssistantOrAgent;
            prior.region_confidence = 0.68;
            prior.speaker_confidence = 0.64;
            prior
                .reason_codes
                .push("agent_surface_prior_left_turn".to_string());
        }
    }
}

fn user_goal_geometry_candidate_is_eligible(span: &OrderedEvidenceSpan) -> bool {
    span.source_scope != "background_display"
        && !matches!(
            span.ownership_kind.as_str(),
            "OtherWindowOwned" | "DisplayOnlyUnattributed"
        )
        && !is_categorical_control_hint(
            &span
                .reason_codes
                .iter()
                .chain(span.quality_flags.iter())
                .cloned()
                .collect::<Vec<_>>()
                .join(" ")
                .to_ascii_lowercase(),
        )
        && span.region_role == RegionRole::Unknown
}

fn span_from_group(
    context: &FrameContext,
    primary: RawSpan,
    contributors: Vec<RawSpan>,
    pane_id: String,
) -> OrderedEvidenceSpan {
    let hint = std::iter::once(primary.structural_hint.as_str())
        .chain(
            contributors
                .iter()
                .map(|item| item.structural_hint.as_str()),
        )
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase();
    let semantic = primary
        .semantic_role
        .as_deref()
        .unwrap_or("")
        .to_ascii_lowercase();
    let (region_role, conversational_role, region_confidence, speaker_confidence, reason) =
        classify_role(context, &hint, &semantic, primary.focused);
    let mut source_refs = std::iter::once(primary.source.clone())
        .chain(contributors.iter().map(|item| item.source.clone()))
        .collect::<Vec<_>>();
    source_refs.sort();
    source_refs.dedup();
    let id_material = format!(
        "{}|{}|{}|{}",
        TASK_TURN_EVIDENCE_SCHEMA_V1,
        context.frame_id,
        source_refs
            .iter()
            .map(|item| format!("{}:{}", item.source_kind.label(), item.source_record_id))
            .collect::<Vec<_>>()
            .join("|"),
        primary.source.text_hash
    );
    let mut quality_flags = primary.quality_flags.clone();
    for contributor in &contributors {
        for flag in &contributor.quality_flags {
            push_unique(&mut quality_flags, flag.clone());
        }
    }
    if source_refs.len() > 1 {
        push_unique(
            &mut quality_flags,
            "merged_duplicate_provenance".to_string(),
        );
    }
    if primary.source.source_kind == EvidenceSourceKind::FlattenedTextFallback {
        push_unique(&mut quality_flags, "flattened_text_fallback".to_string());
    }
    let order_confidence = if primary.parent_id.is_some() {
        0.90
    } else if primary.geometry.is_some() {
        0.74
    } else if primary.source.source_kind == EvidenceSourceKind::AccessibilityNode {
        0.62
    } else {
        0.30
    };
    OrderedEvidenceSpan {
        schema: TASK_TURN_EVIDENCE_SCHEMA_V1.to_string(),
        span_id: format!("turn-span-{}", super::stable_hash(id_material.as_bytes())),
        frame_id: context.frame_id.clone(),
        session_id: context.session_id.clone(),
        surface_key: context.surface_key.clone(),
        artifact_id: context.artifact_id.clone(),
        observed_at_ms: context.observed_at_ms,
        primary_source: primary.source.clone(),
        contributing_sources: source_refs,
        text_hash: primary.source.text_hash.clone(),
        text_storage_class: TextStorageClass::SourceReferenceOnly,
        source_scope: primary.source_scope,
        ownership_kind: primary.ownership_kind,
        owner_window_id: primary.owner_window_id.or(context.window_id),
        owner_app_id: primary.owner_app_id.or_else(|| {
            context
                .bundle_id
                .clone()
                .or_else(|| context.app_name.clone())
        }),
        region_role,
        conversational_role,
        pane_id,
        reading_order: 0,
        local_reading_order: primary.source_order,
        geometry: primary.geometry,
        parent_or_group_id: primary.parent_id,
        focused: primary.focused,
        selected: primary.selected,
        active_artifact_match_confidence: primary.active_match_confidence.clamp(0.0, 1.0),
        ownership_confidence: primary.ownership_confidence.clamp(0.0, 1.0),
        region_confidence,
        speaker_confidence,
        order_confidence,
        privacy_status: context.privacy_status.clone(),
        quality_flags,
        reason_codes: vec![reason],
        text: primary.text,
    }
}

fn classify_role(
    context: &FrameContext,
    hint: &str,
    semantic_role: &str,
    focused: bool,
) -> (RegionRole, ConversationalRole, f64, f64, String) {
    if is_categorical_control_hint(hint) {
        return (
            RegionRole::Control,
            ConversationalRole::NonConversation,
            0.98,
            0.99,
            "categorical_control_ineligible".to_string(),
        );
    }
    if contains_any(hint, &["system_instruction", "system-instruction"]) {
        return (
            RegionRole::SystemStatus,
            ConversationalRole::System,
            0.96,
            0.98,
            "system_instruction_ineligible".to_string(),
        );
    }
    if contains_any(hint, &["sidebar", "source_list", "outline", "navigator"]) {
        return (
            RegionRole::Sidebar,
            ConversationalRole::NonConversation,
            0.92,
            0.95,
            "structural_sidebar".to_string(),
        );
    }
    if contains_any(
        hint,
        &["search_result", "navigation", "breadcrumb", "tab_list"],
    ) {
        return (
            RegionRole::Navigation,
            ConversationalRole::NonConversation,
            0.90,
            0.95,
            "structural_navigation".to_string(),
        );
    }
    if contains_any(hint, &["toolbar", "menu", "titlebar", "app_chrome"]) {
        return (
            RegionRole::AppChrome,
            ConversationalRole::NonConversation,
            0.90,
            0.95,
            "structural_app_chrome".to_string(),
        );
    }
    if contains_any(hint, &["composer", "text_area", "textarea", "prompt_input"]) && focused {
        return (
            RegionRole::Composer,
            ConversationalRole::User,
            0.90,
            0.72,
            "focused_composer".to_string(),
        );
    }
    if contains_any(hint, &["dialog", "sheet", "alert"]) {
        return (
            RegionRole::Dialog,
            ConversationalRole::System,
            0.86,
            0.82,
            "structural_dialog".to_string(),
        );
    }
    if contains_any(hint, &["notification", "toast"]) {
        return (
            RegionRole::Notification,
            ConversationalRole::System,
            0.88,
            0.84,
            "structural_notification".to_string(),
        );
    }
    if contains_any(hint, &["tool_output", "tool-result", "tool_result"])
        || semantic_role == "tool_output"
    {
        return (
            RegionRole::ToolOutput,
            ConversationalRole::Tool,
            0.90,
            0.90,
            "structural_tool_output".to_string(),
        );
    }
    if context.family == SurfaceFamily::Terminal {
        let input = contains_any(hint, &["prompt", "command", "terminal_input"]);
        return if input {
            (
                RegionRole::TerminalInput,
                ConversationalRole::NonConversation,
                0.84,
                0.95,
                "terminal_adapter_input".to_string(),
            )
        } else {
            (
                RegionRole::TerminalOutput,
                ConversationalRole::NonConversation,
                0.76,
                0.95,
                "terminal_adapter_output".to_string(),
            )
        };
    }
    if context.family == SurfaceFamily::Editor {
        return (
            RegionRole::EditorContent,
            ConversationalRole::NonConversation,
            0.82,
            0.95,
            "editor_adapter".to_string(),
        );
    }
    if matches!(
        context.family,
        SurfaceFamily::AgentChat | SurfaceFamily::BrowserChat
    ) {
        if contains_any(
            hint,
            &[
                "conversation_history",
                "history-assistant",
                "prior-assistant",
                "prior_result",
                "ax-prior",
            ],
        ) {
            return (
                RegionRole::ConversationHistory,
                ConversationalRole::AssistantOrAgent,
                0.91,
                0.88,
                "chat_history_structure".to_string(),
            );
        }
        if contains_any(
            hint,
            &[
                "user-message",
                "user_message",
                "human-message",
                "message-user",
                "ax-user",
            ],
        ) {
            return (
                RegionRole::UserMessage,
                ConversationalRole::User,
                0.94,
                0.94,
                "chat_user_structure".to_string(),
            );
        }
        if contains_any(
            hint,
            &[
                "agent-status",
                "assistant-status",
                "working-status",
                "thinking-status",
            ],
        ) {
            return (
                RegionRole::AgentStatus,
                ConversationalRole::AssistantOrAgent,
                0.95,
                0.95,
                "chat_agent_status_structure".to_string(),
            );
        }
        if contains_any(
            hint,
            &[
                "assistant-message",
                "assistant_message",
                "agent-message",
                "message-assistant",
                "ax-agent",
            ],
        ) {
            let region = if context.has_agent_status_event {
                RegionRole::AgentStatus
            } else {
                RegionRole::AgentMessage
            };
            return (
                region,
                ConversationalRole::AssistantOrAgent,
                0.90,
                0.92,
                "chat_agent_structure".to_string(),
            );
        }
        if contains_any(hint, &["system-message", "system_status"]) {
            return (
                RegionRole::SystemStatus,
                ConversationalRole::System,
                0.88,
                0.90,
                "chat_system_structure".to_string(),
            );
        }
    }
    (
        RegionRole::Unknown,
        ConversationalRole::Unknown,
        0.28,
        0.20,
        "insufficient_role_structure".to_string(),
    )
}

pub(super) fn is_categorical_control_hint(hint: &str) -> bool {
    contains_any(
        hint,
        &[
            "axbutton",
            " button",
            "button ",
            "axmenuitem",
            "menu_item",
            "menuitem",
            "axpopupbutton",
            "pop_up_button",
            "popup_button",
            "model_picker",
            "model-picker",
            "approval_chip",
            "approval-chip",
            "axradiobutton",
            "radio_button",
            "axcheckbox",
            "checkbox",
            "axswitch",
            "axlink",
            "axtabgroup",
            "axtabbutton",
            "axtoolbar",
            "segmented_control",
            "notification_action",
            "dialog_action",
            "status_badge",
            "composer_placeholder",
            "tab_chrome",
            "browser_chrome",
            "axpress",
        ],
    )
}

fn user_goal_ineligibility(span: &OrderedEvidenceSpan) -> Option<&'static str> {
    if span.region_role == RegionRole::Control {
        Some("categorical_control_ineligible")
    } else if matches!(
        span.region_role,
        RegionRole::AppChrome
            | RegionRole::Navigation
            | RegionRole::Sidebar
            | RegionRole::Composer
            | RegionRole::SystemStatus
            | RegionRole::Dialog
            | RegionRole::Notification
    ) {
        Some("non_authored_region_ineligible")
    } else if span.conversational_role != ConversationalRole::User {
        Some("not_user_authored")
    } else if span.source_scope == "background_display"
        || matches!(
            span.ownership_kind.as_str(),
            "OtherWindowOwned" | "DisplayOnlyUnattributed"
        )
    {
        Some("surface_ownership_ineligible")
    } else {
        None
    }
}

fn select_latest_turn(context: &FrameContext, spans: &[OrderedEvidenceSpan]) -> LatestTurnEvidence {
    let eligible = spans
        .iter()
        .filter(|span| {
            span.observed_at_ms <= context.observed_at_ms
                && span.session_id == context.session_id
                && span_matches_context_surface(span, context)
                && span.source_scope != "background_display"
                && !matches!(
                    span.ownership_kind.as_str(),
                    "OtherWindowOwned" | "DisplayOnlyUnattributed"
                )
        })
        .collect::<Vec<_>>();
    let latest_user = eligible
        .iter()
        .rev()
        .find(|span| {
            span.region_role == RegionRole::UserMessage
                && span.conversational_role == ConversationalRole::User
                && user_goal_ineligibility(span).is_none()
                && span.region_confidence >= MIN_TYPED_ROLE_CONFIDENCE
                && span.speaker_confidence >= MIN_TYPED_ROLE_CONFIDENCE
        })
        .copied();
    let current_agent = latest_user.and_then(|user| {
        eligible
            .iter()
            .rev()
            .find(|span| {
                span.region_role.is_agent()
                    && span.conversational_role == ConversationalRole::AssistantOrAgent
                    && (span.observed_at_ms > user.observed_at_ms
                        || span.observed_at_ms == user.observed_at_ms
                            && span.reading_order > user.reading_order)
                    && span.region_confidence >= MIN_TYPED_ROLE_CONFIDENCE
            })
            .copied()
    });
    let prior = if let Some(user) = latest_user {
        eligible
            .iter()
            .rev()
            .find(|span| {
                span.region_role == RegionRole::ConversationHistory
                    && (span.observed_at_ms < user.observed_at_ms
                        || span.observed_at_ms == user.observed_at_ms
                            && span.reading_order < user.reading_order)
            })
            .copied()
    } else {
        eligible
            .iter()
            .rev()
            .find(|span| span.region_role == RegionRole::ConversationHistory)
            .copied()
    };
    let mut salient = Vec::new();
    for span in [latest_user, current_agent, prior].into_iter().flatten() {
        push_unique(&mut salient, span.span_id.clone());
    }
    if let Some(composer) = eligible
        .iter()
        .rev()
        .find(|span| span.region_role == RegionRole::Composer)
    {
        push_unique(&mut salient, composer.span_id.clone());
    }
    let samples_allowed = !is_privacy_blocked(&context.privacy_status);
    let mut user_sample = samples_allowed
        .then(|| latest_user.and_then(|span| safe_sample(&span.text, MAX_GOAL_SAMPLE_CHARS)))
        .flatten();
    let mut agent_sample = samples_allowed
        .then(|| current_agent.and_then(|span| safe_sample(&span.text, MAX_AGENT_SAMPLE_CHARS)))
        .flatten();
    let mut prior_sample = samples_allowed
        .then(|| prior.and_then(|span| safe_sample(&span.text, MAX_PRIOR_SAMPLE_CHARS)))
        .flatten();
    enforce_total_cap(&mut user_sample, &mut agent_sample, &mut prior_sample);
    let mut missing_roles = Vec::new();
    if latest_user.is_none() {
        missing_roles.push("latest_user_message".to_string());
    }
    if latest_user.is_some() && current_agent.is_none() {
        missing_roles.push("current_agent_response_or_status".to_string());
    }
    let selected_ids = salient.iter().cloned().collect::<BTreeSet<_>>();
    let rejected_spans = eligible
        .iter()
        .filter(|span| !selected_ids.contains(&span.span_id))
        .take(24)
        .map(|span| RejectedTurnSpan {
            span_id: span.span_id.clone(),
            reason_code: rejection_reason(span, latest_user),
        })
        .collect::<Vec<_>>();
    let mut fallback_flags = eligible
        .iter()
        .flat_map(|span| span.quality_flags.iter())
        .filter(|flag| flag.as_str() == "flattened_text_fallback")
        .cloned()
        .collect::<Vec<_>>();
    if salient.is_empty() {
        fallback_flags.push("flattened_text_fallback".to_string());
        missing_roles.push("typed_salient_semantic_unit".to_string());
    }
    if let Some(attribution) = context.causal_typing.as_ref() {
        fallback_flags.push(format!("causal_typing:{}", attribution.association_source));
    }
    fallback_flags.sort();
    fallback_flags.dedup();
    let confidence = [latest_user, current_agent]
        .into_iter()
        .flatten()
        .map(|span| {
            span.region_confidence
                .min(span.speaker_confidence)
                .min(span.order_confidence)
        })
        .reduce(f64::min)
        .unwrap_or_else(|| if prior.is_some() { 0.58 } else { 0.20 });
    LatestTurnEvidence {
        schema: TASK_TURN_EVIDENCE_SCHEMA_V1.to_string(),
        frame_id: context.frame_id.clone(),
        salient_span_ids: salient,
        latest_user_span_ids: latest_user
            .into_iter()
            .map(|span| span.span_id.clone())
            .collect(),
        current_agent_span_ids: current_agent
            .into_iter()
            .map(|span| span.span_id.clone())
            .collect(),
        prior_boundary_span_ids: prior.into_iter().map(|span| span.span_id.clone()).collect(),
        salient_user_goal_sample: user_sample,
        salient_agent_state_sample: agent_sample,
        prior_boundary_sample: prior_sample,
        sample_storage_class: TextStorageClass::BoundedPublicSafeSummary,
        sampling_strategy: "causal_typed_latest_turn_v2".to_string(),
        sampling_confidence: round_confidence(confidence),
        missing_roles,
        rejected_spans,
        fallback_flags,
        causal_typing_attribution: context.causal_typing.clone(),
    }
}

fn span_matches_context_surface(span: &OrderedEvidenceSpan, context: &FrameContext) -> bool {
    let context_app = context.bundle_id.as_deref().or(context.app_name.as_deref());
    let app_matches = match (span.owner_app_id.as_deref(), context_app) {
        (Some(left), Some(right)) => left == right,
        _ => true,
    };
    let window_matches = match (span.owner_window_id, context.window_id) {
        (Some(left), Some(right)) => left == right,
        _ => true,
    };
    app_matches && window_matches
}

fn persist_span(conn: &Connection, span: &OrderedEvidenceSpan) -> Result<(), String> {
    let now = super::current_time_millis();
    conn.execute(
        "INSERT OR REPLACE INTO continue_ordered_evidence_spans (
           span_id, schema_version, frame_id, session_id, surface_key, artifact_id,
           observed_at_ms, primary_source_kind, primary_source_record_id,
           source_text_reference, contributing_source_refs_json, text_hash,
           bounded_public_safe_summary, text_storage_class, source_scope, ownership_kind,
           owner_window_id, owner_app_id, region_role, conversational_role, pane_id,
           reading_order, local_reading_order, geometry_json, parent_or_group_id,
           focused, selected, active_artifact_match_confidence, ownership_confidence,
           region_confidence, speaker_confidence, order_confidence, privacy_status,
           quality_flags_json, reason_codes_json, created_at_ms, updated_at_ms
         ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,NULL,?13,?14,?15,
                   ?16,?17,?18,?19,?20,?21,?22,?23,?24,?25,?26,?27,?28,?29,
                   ?30,?31,?32,?33,?34,?35,?36)",
        params![
            span.span_id,
            span.schema,
            span.frame_id,
            span.session_id,
            span.surface_key,
            span.artifact_id,
            span.observed_at_ms,
            span.primary_source.source_kind.label(),
            span.primary_source.source_record_id,
            span.primary_source.source_text_reference,
            serde_json::to_string(&span.contributing_sources).map_err(to_string)?,
            span.text_hash,
            span.text_storage_class.label(),
            span.source_scope,
            span.ownership_kind,
            span.owner_window_id,
            span.owner_app_id,
            span.region_role.label(),
            span.conversational_role.label(),
            span.pane_id,
            span.reading_order,
            span.local_reading_order,
            span.geometry.map(|geometry| json!({
                "x": geometry.x, "y": geometry.y,
                "width": geometry.width, "height": geometry.height,
                "coordinate_space": "capture_points"
            })
            .to_string()),
            span.parent_or_group_id,
            i64::from(span.focused),
            i64::from(span.selected),
            span.active_artifact_match_confidence,
            span.ownership_confidence,
            span.region_confidence,
            span.speaker_confidence,
            span.order_confidence,
            span.privacy_status,
            serde_json::to_string(&span.quality_flags).map_err(to_string)?,
            serde_json::to_string(&span.reason_codes).map_err(to_string)?,
            now,
            now,
        ],
    )
    .map_err(to_string)?;
    Ok(())
}

fn persist_selection(
    conn: &Connection,
    context: &FrameContext,
    selection: &LatestTurnEvidence,
) -> Result<(), String> {
    let now = super::current_time_millis();
    conn.execute(
        "INSERT OR REPLACE INTO continue_salient_turn_evidence (
           frame_id, schema_version, session_id, surface_key, artifact_id, observed_at_ms,
           salient_span_ids_json, latest_user_span_ids_json, current_agent_span_ids_json,
           prior_boundary_span_ids_json, salient_user_goal_sample, salient_user_goal_hash,
           salient_agent_state_sample, salient_agent_state_hash, prior_boundary_sample,
           prior_boundary_hash, sample_storage_class, sampling_strategy, sampling_confidence,
           missing_roles_json, rejected_spans_json, fallback_flags_json,
           causal_typing_attribution_json, created_at_ms, updated_at_ms
         ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,
                   ?17,?18,?19,?20,?21,?22,?23,?24,?25)",
        params![
            context.frame_id,
            selection.schema,
            context.session_id,
            context.surface_key,
            context.artifact_id,
            context.observed_at_ms,
            serde_json::to_string(&selection.salient_span_ids).map_err(to_string)?,
            serde_json::to_string(&selection.latest_user_span_ids).map_err(to_string)?,
            serde_json::to_string(&selection.current_agent_span_ids).map_err(to_string)?,
            serde_json::to_string(&selection.prior_boundary_span_ids).map_err(to_string)?,
            selection.salient_user_goal_sample,
            selection.salient_user_goal_sample.as_deref().map(text_hash),
            selection.salient_agent_state_sample,
            selection
                .salient_agent_state_sample
                .as_deref()
                .map(text_hash),
            selection.prior_boundary_sample,
            selection.prior_boundary_sample.as_deref().map(text_hash),
            selection.sample_storage_class.label(),
            selection.sampling_strategy,
            selection.sampling_confidence,
            serde_json::to_string(&selection.missing_roles).map_err(to_string)?,
            serde_json::to_string(&selection.rejected_spans).map_err(to_string)?,
            serde_json::to_string(&selection.fallback_flags).map_err(to_string)?,
            selection
                .causal_typing_attribution
                .as_ref()
                .map(serde_json::to_string)
                .transpose()
                .map_err(to_string)?,
            now,
            now,
        ],
    )
    .map_err(to_string)?;
    Ok(())
}

fn update_surface_snapshot_sample(
    conn: &Connection,
    context: &FrameContext,
    selection: &LatestTurnEvidence,
) -> Result<(), String> {
    if !table_exists(conn, "continue_surface_snapshots")?
        || is_privacy_blocked(&context.privacy_status)
    {
        return Ok(());
    }
    let sample = selection_semantic_text(selection);
    if sample.is_empty() {
        return Ok(());
    }
    conn.execute(
        "UPDATE continue_surface_snapshots
         SET visible_text_sample = ?2, visible_text_hash = ?3, updated_at_ms = ?4
         WHERE frame_id = ?1",
        params![
            context.frame_id,
            sample,
            text_hash(&sample),
            super::current_time_millis()
        ],
    )
    .map_err(to_string)?;
    Ok(())
}

pub(crate) fn load_accuracy_checkpoints(
    conn: &Connection,
) -> Result<Option<TaskTurnAccuracyCheckpoints>, String> {
    if !table_exists(conn, "continue_salient_turn_evidence")? {
        return Ok(None);
    }
    let row = conn
        .query_row(
            "SELECT frame_id, latest_user_span_ids_json, current_agent_span_ids_json,
                    prior_boundary_span_ids_json, salient_user_goal_sample,
                    salient_agent_state_sample
             FROM continue_salient_turn_evidence
             ORDER BY observed_at_ms DESC, CAST(frame_id AS INTEGER) DESC LIMIT 1",
            [],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, Option<String>>(5)?,
                ))
            },
        )
        .optional()
        .map_err(to_string)?;
    let Some((_frame_id, user_json, agent_json, prior_json, user_sample, agent_sample)) = row
    else {
        return Ok(None);
    };
    let user_ids = parse_string_array(Some(&user_json));
    let agent_ids = parse_string_array(Some(&agent_json));
    let prior_ids = parse_string_array(Some(&prior_json));
    let user = load_first_span_label(conn, &user_ids)?;
    let agent = load_first_span_label(conn, &agent_ids)?;
    let prior = load_first_span_label(conn, &prior_ids)?;
    let support = conn
        .query_row(
            "SELECT region_role FROM continue_ordered_evidence_spans
             WHERE region_role IN ('navigation','sidebar')
             ORDER BY observed_at_ms DESC, reading_order DESC LIMIT 1",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(to_string)?;
    let mut checkpoints = TaskTurnAccuracyCheckpoints::default();
    if let Some((region, role, _, _)) = &user {
        checkpoints
            .region_roles
            .insert("latest_user_region".to_string(), json!(region));
        checkpoints
            .conversational_roles
            .insert("latest_user_role".to_string(), json!(role));
    }
    if let Some((region, role, _, _)) = &agent {
        checkpoints
            .region_roles
            .insert("current_agent_region".to_string(), json!(region));
        checkpoints
            .conversational_roles
            .insert("current_agent_role".to_string(), json!(role));
    }
    if let Some((region, role, observed, order)) = &prior {
        let key = if user.is_some() {
            "prior_result_region"
        } else {
            "latest_result_region"
        };
        checkpoints
            .region_roles
            .insert(key.to_string(), json!(region));
        checkpoints
            .conversational_roles
            .insert("prior_result_role".to_string(), json!(role));
        if let Some((_, _, user_observed, user_order)) = &user {
            checkpoints.ordered_turn_spans.insert(
                "prior_completion_before_latest_user".to_string(),
                json!((*observed, *order) < (*user_observed, *user_order)),
            );
        }
    }
    if let Some(region) = support {
        checkpoints
            .region_roles
            .insert("support_region".to_string(), json!(region));
    }
    if let (Some((_, _, user_observed, user_order)), Some((_, _, agent_observed, agent_order))) =
        (&user, &agent)
    {
        checkpoints.ordered_turn_spans.insert(
            "latest_user_before_current_agent_status".to_string(),
            json!((*user_observed, *user_order) < (*agent_observed, *agent_order)),
        );
    }
    if let Some(sample) = user_sample {
        checkpoints
            .latest_task_turn
            .insert("latest_user_goal".to_string(), json!(sample));
    }
    if let Some(sample) = agent_sample {
        checkpoints
            .latest_task_turn
            .insert("current_agent_state".to_string(), json!(sample));
    }
    Ok(Some(checkpoints))
}

pub(crate) fn task_turn_evidence_audit_json(
    conn: &Connection,
    limit: usize,
) -> Result<Value, String> {
    ensure_task_turn_evidence_schema(conn)?;
    let limit = limit.clamp(1, 500) as i64;
    let mut span_statement = conn
        .prepare(
            "SELECT span_id, frame_id, surface_key, artifact_id, observed_at_ms,
                    primary_source_kind, primary_source_record_id,
                    contributing_source_refs_json, text_hash, text_storage_class,
                    source_scope, ownership_kind, owner_window_id, owner_app_id,
                    region_role, conversational_role, pane_id, reading_order,
                    local_reading_order, geometry_json, parent_or_group_id, focused,
                    selected, active_artifact_match_confidence, ownership_confidence,
                    region_confidence, speaker_confidence, order_confidence, privacy_status,
                    quality_flags_json, reason_codes_json
             FROM continue_ordered_evidence_spans
             ORDER BY observed_at_ms DESC, reading_order ASC LIMIT ?1",
        )
        .map_err(to_string)?;
    let spans = span_statement
        .query_map(params![limit], |row| {
            Ok(json!({
                "span_id": row.get::<_, String>(0)?,
                "frame_id": row.get::<_, String>(1)?,
                "surface_key_hash": row.get::<_, Option<String>>(2)?.map(|value| {
                    super::stable_hash(format!("task_turn_surface|{value}").as_bytes())
                }),
                "artifact_id": row.get::<_, Option<String>>(3)?,
                "observed_at_ms": row.get::<_, i64>(4)?,
                "primary_source": {
                    "kind": row.get::<_, String>(5)?,
                    "record_id": row.get::<_, String>(6)?,
                },
                "contributing_sources": parse_json_value(row.get::<_, String>(7)?),
                "text_hash": row.get::<_, String>(8)?,
                "text_storage_class": row.get::<_, String>(9)?,
                "source_scope": row.get::<_, String>(10)?,
                "ownership_kind": row.get::<_, String>(11)?,
                "owner_window_id": row.get::<_, Option<i64>>(12)?,
                "owner_app_id": row.get::<_, Option<String>>(13)?,
                "region_role": row.get::<_, String>(14)?,
                "conversational_role": row.get::<_, String>(15)?,
                "pane_id": row.get::<_, String>(16)?,
                "reading_order": row.get::<_, i64>(17)?,
                "local_reading_order": row.get::<_, i64>(18)?,
                "geometry": row.get::<_, Option<String>>(19)?.map(parse_json_value),
                "parent_or_group_id": row.get::<_, Option<String>>(20)?,
                "focused": row.get::<_, i64>(21)? != 0,
                "selected": row.get::<_, i64>(22)? != 0,
                "confidence": {
                    "active_artifact_match": row.get::<_, f64>(23)?,
                    "ownership": row.get::<_, f64>(24)?,
                    "region": row.get::<_, f64>(25)?,
                    "speaker": row.get::<_, f64>(26)?,
                    "order": row.get::<_, f64>(27)?,
                },
                "privacy_status": row.get::<_, String>(28)?,
                "quality_flags": parse_json_value(row.get::<_, String>(29)?),
                "reason_codes": parse_json_value(row.get::<_, String>(30)?),
            }))
        })
        .map_err(to_string)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(to_string)?;
    let mut selection_statement = conn
        .prepare(
            "SELECT frame_id, surface_key, artifact_id, observed_at_ms,
                    salient_span_ids_json, latest_user_span_ids_json,
                    current_agent_span_ids_json, prior_boundary_span_ids_json,
                    salient_user_goal_hash, salient_agent_state_hash, prior_boundary_hash,
                    sample_storage_class, sampling_strategy, sampling_confidence,
                    missing_roles_json, rejected_spans_json, fallback_flags_json
             FROM continue_salient_turn_evidence
             ORDER BY observed_at_ms DESC LIMIT ?1",
        )
        .map_err(to_string)?;
    let selections = selection_statement
        .query_map(params![limit], |row| {
            Ok(json!({
                "frame_id": row.get::<_, String>(0)?,
                "surface_key_hash": row.get::<_, Option<String>>(1)?.map(|value| {
                    super::stable_hash(format!("task_turn_surface|{value}").as_bytes())
                }),
                "artifact_id": row.get::<_, Option<String>>(2)?,
                "observed_at_ms": row.get::<_, i64>(3)?,
                "salient_span_ids": parse_json_value(row.get::<_, String>(4)?),
                "latest_user_span_ids": parse_json_value(row.get::<_, String>(5)?),
                "current_agent_span_ids": parse_json_value(row.get::<_, String>(6)?),
                "prior_boundary_span_ids": parse_json_value(row.get::<_, String>(7)?),
                "sample_hashes": {
                    "user_goal": row.get::<_, Option<String>>(8)?,
                    "agent_state": row.get::<_, Option<String>>(9)?,
                    "prior_boundary": row.get::<_, Option<String>>(10)?,
                },
                "sample_storage_class": row.get::<_, String>(11)?,
                "sampling_strategy": row.get::<_, String>(12)?,
                "sampling_confidence": row.get::<_, f64>(13)?,
                "missing_roles": parse_json_value(row.get::<_, String>(14)?),
                "rejected_spans": parse_json_value(row.get::<_, String>(15)?),
                "fallback_flags": parse_json_value(row.get::<_, String>(16)?),
            }))
        })
        .map_err(to_string)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(to_string)?;
    Ok(json!({
        "schema": TASK_TURN_EVIDENCE_SCHEMA_V1,
        "privacy_note": "No source text or bounded samples are included in this audit; only hashes, references, roles, order, confidence, and selection reasons are exported.",
        "ordered_spans": spans,
        "selections": selections,
    }))
}

fn load_first_span_label(
    conn: &Connection,
    span_ids: &[String],
) -> Result<Option<(String, String, i64, i64)>, String> {
    let Some(span_id) = span_ids.first() else {
        return Ok(None);
    };
    conn.query_row(
        "SELECT region_role, conversational_role, observed_at_ms, reading_order
         FROM continue_ordered_evidence_spans WHERE span_id = ?1",
        params![span_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
    )
    .optional()
    .map_err(to_string)
}

fn selection_semantic_text(selection: &LatestTurnEvidence) -> String {
    let mut parts = Vec::new();
    for value in [
        selection.salient_user_goal_sample.as_deref(),
        selection.salient_agent_state_sample.as_deref(),
    ]
    .into_iter()
    .flatten()
    {
        if !parts.iter().any(|existing| *existing == value) {
            parts.push(value);
        }
    }
    parts.join("\n")
}

fn sources_can_merge(primary: &RawSpan, candidate: &RawSpan, contributors: &[RawSpan]) -> bool {
    if primary.source.source_kind == candidate.source.source_kind {
        return primary.source.source_record_id == candidate.source.source_record_id;
    }
    let overlap = primary
        .geometry
        .zip(candidate.geometry)
        .map(|(left, right)| left.overlap_ratio(right))
        .unwrap_or(0.0);
    overlap >= 0.55
        || primary.geometry.is_none()
        || candidate.geometry.is_none()
        || contributors.iter().any(|item| {
            item.geometry
                .zip(candidate.geometry)
                .is_some_and(|(left, right)| left.overlap_ratio(right) >= 0.55)
        })
}

fn assign_panes(geometries: Vec<Option<SpanGeometry>>) -> Vec<String> {
    let mut centers = geometries
        .iter()
        .flatten()
        .map(|geometry| geometry.center_x())
        .collect::<Vec<_>>();
    centers.sort_by(f64::total_cmp);
    let mut boundaries = Vec::new();
    for pair in centers.windows(2) {
        if pair[1] - pair[0] >= 400.0 {
            boundaries.push((pair[0] + pair[1]) / 2.0);
        }
    }
    geometries
        .into_iter()
        .map(|geometry| {
            geometry
                .map(|geometry| {
                    let pane = boundaries
                        .iter()
                        .filter(|boundary| geometry.center_x() > **boundary)
                        .count();
                    format!("pane-{pane:02}")
                })
                .unwrap_or_else(|| "pane-unknown".to_string())
        })
        .collect()
}

fn geometry_order(left: Option<SpanGeometry>, right: Option<SpanGeometry>) -> std::cmp::Ordering {
    match (left, right) {
        (Some(left), Some(right)) => left.y.total_cmp(&right.y).then(left.x.total_cmp(&right.x)),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    }
}

fn source_ref(kind: EvidenceSourceKind, id: &str, text: &str) -> EvidenceSourceRef {
    EvidenceSourceRef {
        source_kind: kind,
        source_record_id: id.to_string(),
        source_text_reference: format!("{}:{}:text", kind.label(), id),
        text_hash: text_hash(text),
    }
}

fn safe_sample(text: &str, max_chars: usize) -> Option<String> {
    sanitize_public_text(
        text.to_string(),
        max_chars.min(MAX_SPAN_SUMMARY_CHARS.max(max_chars)),
    )
}

fn enforce_total_cap(
    user: &mut Option<String>,
    agent: &mut Option<String>,
    prior: &mut Option<String>,
) {
    let mut total = sample_length(user) + sample_length(agent) + sample_length(prior);
    if total <= MAX_TOTAL_SAMPLE_CHARS {
        return;
    }
    *prior = None;
    total = sample_length(user) + sample_length(agent);
    if total > MAX_TOTAL_SAMPLE_CHARS {
        let user_budget = MAX_TOTAL_SAMPLE_CHARS / 2;
        let agent_budget = MAX_TOTAL_SAMPLE_CHARS - user_budget;
        *user = user
            .take()
            .and_then(|value| sanitize_public_text(value, user_budget));
        *agent = agent
            .take()
            .and_then(|value| sanitize_public_text(value, agent_budget));
    }
}

fn sample_length(value: &Option<String>) -> usize {
    value.as_ref().map(|text| text.chars().count()).unwrap_or(0)
}

fn rejection_reason(
    span: &OrderedEvidenceSpan,
    latest_user: Option<&OrderedEvidenceSpan>,
) -> String {
    if let Some(reason) = user_goal_ineligibility(span) {
        reason.to_string()
    } else if matches!(
        span.region_role,
        RegionRole::Navigation | RegionRole::Sidebar | RegionRole::AppChrome
    ) {
        "non_conversation_region".to_string()
    } else if span.region_role == RegionRole::TerminalOutput {
        "unrelated_terminal_pane".to_string()
    } else if span.region_role == RegionRole::ConversationHistory {
        "prior_conversation_context".to_string()
    } else if latest_user.is_some_and(|user| span.observed_at_ms < user.observed_at_ms) {
        "older_than_latest_user_turn".to_string()
    } else if span.region_confidence < MIN_TYPED_ROLE_CONFIDENCE {
        "role_confidence_below_threshold".to_string()
    } else {
        "not_salient_for_latest_turn".to_string()
    }
}

fn is_privacy_blocked(value: &str) -> bool {
    contains_any(
        &value.to_ascii_lowercase(),
        &["blocked", "sensitive", "private", "excluded"],
    )
}

fn optional_geometry(
    x: Option<f64>,
    y: Option<f64>,
    width: Option<f64>,
    height: Option<f64>,
) -> Option<SpanGeometry> {
    match (x, y, width, height) {
        (Some(x), Some(y), Some(width), Some(height)) if width > 0.0 && height > 0.0 => {
            Some(SpanGeometry {
                x,
                y,
                width,
                height,
            })
        }
        _ => None,
    }
}

fn load_strings(conn: &Connection, sql: &str, frame_id: &str) -> Result<Vec<String>, String> {
    if !table_exists(conn, "app_contexts")? {
        return Ok(Vec::new());
    }
    let mut statement = conn.prepare(sql).map_err(to_string)?;
    let rows = statement
        .query_map(params![frame_id], |row| row.get::<_, String>(0))
        .map_err(to_string)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(to_string)
}

fn table_exists(conn: &Connection, table: &str) -> Result<bool, String> {
    conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1)",
        params![table],
        |row| row.get::<_, i64>(0),
    )
    .map(|value| value != 0)
    .map_err(to_string)
}

fn column_exists(conn: &Connection, table: &str, column: &str) -> Result<bool, String> {
    let mut statement = conn
        .prepare(&format!("PRAGMA table_info({table})"))
        .map_err(to_string)?;
    let columns = statement
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(to_string)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(to_string)?;
    Ok(columns.iter().any(|value| value == column))
}

fn column_or(
    conn: &Connection,
    table: &str,
    column: &str,
    fallback: &str,
) -> Result<String, String> {
    Ok(if column_exists(conn, table, column)? {
        column.to_string()
    } else {
        fallback.to_string()
    })
}

fn parse_string_array(raw: Option<&str>) -> Vec<String> {
    raw.and_then(|value| serde_json::from_str::<Vec<String>>(value).ok())
        .unwrap_or_default()
}

fn parse_json_value(raw: String) -> Value {
    serde_json::from_str(&raw).unwrap_or(Value::Null)
}

fn text_hash(value: &str) -> String {
    super::stable_hash(format!("task_turn_text_v1|{}", normalize_text(value)).as_bytes())
}

fn normalize_text(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase()
}

fn numeric_id(value: &str) -> i64 {
    value.parse().unwrap_or(i64::MAX)
}

fn contains_any(value: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| value.contains(needle))
}

fn push_unique(values: &mut Vec<String>, value: String) {
    if !values.contains(&value) {
        values.push(value);
    }
}

fn round_confidence(value: f64) -> f64 {
    (value.clamp(0.0, 1.0) * 100.0).round() / 100.0
}

fn to_string(error: impl std::fmt::Display) -> String {
    error.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context(family: SurfaceFamily) -> FrameContext {
        FrameContext {
            frame_id: "7".to_string(),
            session_id: Some("session".to_string()),
            observed_at_ms: 100,
            app_name: Some("Codex".to_string()),
            bundle_id: Some("com.openai.codex".to_string()),
            window_id: Some(1),
            privacy_status: "normal".to_string(),
            artifact_id: Some("artifact".to_string()),
            surface_key: Some("surface".to_string()),
            family,
            has_agent_status_event: true,
            causal_typing: Some(TypingBurstCausalAttribution {
                typing_burst_id: "typing".to_string(),
                commit_signal: Some("enter".to_string()),
                started_at_ms: 80,
                ended_at_ms: 90,
                pre_frame_id: Some("6".to_string()),
                post_frame_id: Some("7".to_string()),
                bounded_inferred_frame_id: None,
                surface_match_result: "exact_app_and_window".to_string(),
                temporal_distance_ms: 10,
                association_source: "stored_capture_trigger_commit_event".to_string(),
                association_confidence: 0.98,
                rejection_reasons: Vec::new(),
                capture_trigger_id: Some("trigger".to_string()),
                commit_event_id: Some("event".to_string()),
            }),
            pre_frame_text_hashes: BTreeSet::new(),
        }
    }

    fn raw(id: &str, text: &str, hint: &str, x: f64, y: f64) -> RawSpan {
        RawSpan {
            source: source_ref(EvidenceSourceKind::AccessibilityNode, id, text),
            text: text.to_string(),
            source_scope: "active_window".to_string(),
            ownership_kind: "ActiveWindowOwned".to_string(),
            owner_window_id: Some(1),
            owner_app_id: Some("com.openai.codex".to_string()),
            geometry: Some(SpanGeometry {
                x,
                y,
                width: 300.0,
                height: 50.0,
            }),
            parent_id: Some("conversation".to_string()),
            source_order: y as i64,
            focused: false,
            selected: false,
            source_confidence: 0.9,
            ownership_confidence: 0.9,
            active_match_confidence: 0.9,
            structural_hint: hint.to_string(),
            semantic_role: None,
            quality_flags: Vec::new(),
        }
    }

    fn live_shaped_legacy_fixture(
        committed: bool,
        typing_window_id: i64,
        second_legacy_candidate: bool,
    ) -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        crate::capture::init_db(&conn).unwrap();
        conn.execute(
            "INSERT INTO capture_sessions
             (id, sequence, started_at_ms, status, created_at_ms)
             VALUES ('session-013', 13, 1000, 'active', 1000)",
            [],
        )
        .unwrap();
        for (id, captured_at, previous) in [(1_i64, 1000_i64, None), (2, 2000, Some(1_i64))] {
            conn.execute(
                "INSERT INTO frames
                 (id, session_id, captured_at, snapshot_path, app_name, app_bundle_id,
                  window_name, window_id, focused, capture_trigger, privacy_status,
                  previous_frame_id, created_at)
                 VALUES (?1, 'session-013', ?2, 'fixture', 'AgentChat',
                         'com.example.agentchat', 'Synthetic task conversation', 42, 1,
                         'event_burst', 'normal', ?3, ?2)",
                params![id, captured_at, previous],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO app_contexts
                 (id, frame_id, adapter_id, object_type, confidence)
                 VALUES (?1, ?2, 'generic_agent_surface', 'chat_conversation', 0.9)",
                params![format!("context-{id}"), id.to_string()],
            )
            .unwrap();
        }
        for (id, frame_id, role, text, order, x, y) in [
            (
                "pre-prior",
                "1",
                "AXStaticText",
                "Earlier work was completed",
                1,
                280.0,
                220.0,
            ),
            (
                "pre-control",
                "1",
                "AXButton",
                "Approve for me",
                2,
                640.0,
                700.0,
            ),
            (
                "post-prior",
                "2",
                "AXStaticText",
                "Earlier work was completed",
                1,
                280.0,
                220.0,
            ),
            (
                "post-user",
                "2",
                "AXStaticText",
                "Repair causal task evidence now",
                2,
                620.0,
                420.0,
            ),
            (
                "post-agent",
                "2",
                "AXStaticText",
                "Working",
                3,
                280.0,
                520.0,
            ),
            (
                "post-control",
                "2",
                "AXButton",
                "Approve for me",
                4,
                640.0,
                700.0,
            ),
        ] {
            conn.execute(
                "INSERT INTO ax_nodes
                 (id, frame_id, role, value, enabled, bounds_x, bounds_y, bounds_w,
                  bounds_h, depth, tree_order, actions_json, raw_json)
                 VALUES (?1, ?2, ?3, ?4, 1, ?5, ?6, 340, 54, 1, ?7,
                         CASE WHEN ?3='AXButton' THEN '[\"AXPress\"]' ELSE '[]' END, '{}')",
                params![id, frame_id, role, text, x, y, order],
            )
            .unwrap();
        }
        conn.execute(
            "INSERT INTO typing_bursts
             (id, session_id, started_at_ms, ended_at_ms, app_bundle_id, app_name,
              window_id, window_title, char_count, enter_count, committed,
              commit_signal, raw_text_captured, pre_frame_id, post_frame_id)
             VALUES ('typing-live', 'session-013', 1800, 1900, 'com.example.agentchat',
                     'AgentChat', ?1, 'Synthetic task conversation', 27, 1, ?2,
                     'enter', 0, '1', NULL)",
            params![typing_window_id, i64::from(committed)],
        )
        .unwrap();
        if second_legacy_candidate {
            conn.execute(
                "INSERT INTO typing_bursts
                 (id, session_id, started_at_ms, ended_at_ms, app_bundle_id, app_name,
                  window_id, window_title, char_count, enter_count, committed,
                  commit_signal, raw_text_captured, pre_frame_id, post_frame_id)
                 VALUES ('typing-ambiguous', 'session-013', 1810, 1910,
                         'com.example.agentchat', 'AgentChat', 42,
                         'Synthetic task conversation', 9, 1, 1, 'enter', 0, '1', NULL)",
                [],
            )
            .unwrap();
        }
        conn
    }

    #[test]
    fn live_shaped_legacy_null_post_recovers_causal_user_and_excludes_control() {
        let conn = live_shaped_legacy_fixture(true, 42, false);
        rebuild_task_turn_evidence(&conn, &["2".to_string()]).unwrap();
        let (user, prior, attribution): (Option<String>, Option<String>, Option<String>) = conn
            .query_row(
                "SELECT salient_user_goal_sample, prior_boundary_sample,
                        causal_typing_attribution_json
                 FROM continue_salient_turn_evidence WHERE frame_id='2'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(user.as_deref(), Some("Repair causal task evidence now"));
        assert_eq!(prior.as_deref(), Some("Earlier work was completed"));
        let attribution: TypingBurstCausalAttribution =
            serde_json::from_str(attribution.as_deref().unwrap()).unwrap();
        assert_eq!(attribution.typing_burst_id, "typing-live");
        assert_eq!(attribution.association_source, "legacy_bounded_recovery");
        assert_eq!(attribution.bounded_inferred_frame_id.as_deref(), Some("2"));
        let control_role: String = conn
            .query_row(
                "SELECT region_role FROM continue_ordered_evidence_spans
                 WHERE primary_source_record_id='post-control'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(control_role, "control");
    }

    #[test]
    fn legacy_typing_never_transfers_across_window_or_from_uncommitted_or_ambiguous_rows() {
        for conn in [
            live_shaped_legacy_fixture(true, 99, false),
            live_shaped_legacy_fixture(false, 42, false),
            live_shaped_legacy_fixture(true, 42, true),
        ] {
            rebuild_task_turn_evidence(&conn, &["2".to_string()]).unwrap();
            let (user, attribution): (Option<String>, Option<String>) = conn
                .query_row(
                    "SELECT salient_user_goal_sample, causal_typing_attribution_json
                     FROM continue_salient_turn_evidence WHERE frame_id='2'",
                    [],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .unwrap();
            assert!(user.is_none());
            assert!(attribution.is_none());
        }
    }

    #[test]
    fn stored_post_frame_loads_full_causal_attribution() {
        let conn = live_shaped_legacy_fixture(true, 42, false);
        conn.execute(
            "UPDATE typing_bursts
             SET post_frame_id='2', capture_trigger_id='trigger-2',
                 commit_event_id='event-enter',
                 post_frame_association_source='stored_capture_trigger_commit_event',
                 post_frame_association_confidence=0.98
             WHERE id='typing-live'",
            [],
        )
        .unwrap();
        rebuild_task_turn_evidence(&conn, &["2".to_string()]).unwrap();
        let payload: String = conn
            .query_row(
                "SELECT causal_typing_attribution_json
                 FROM continue_salient_turn_evidence WHERE frame_id='2'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let attribution: TypingBurstCausalAttribution = serde_json::from_str(&payload).unwrap();
        assert_eq!(attribution.post_frame_id.as_deref(), Some("2"));
        assert_eq!(attribution.commit_event_id.as_deref(), Some("event-enter"));
        assert_eq!(attribution.capture_trigger_id.as_deref(), Some("trigger-2"));
        assert_eq!(attribution.association_confidence, 0.98);
    }

    #[test]
    fn categorical_controls_are_never_user_goals_even_with_task_like_verbs() {
        for (id, label, hint) in [
            ("approve", "Approve for me", "AXButton AXPress"),
            ("continue", "Continue", "AXMenuItem AXPress"),
            ("run", "Run", "approval_chip AXButton"),
            ("send", "Send", "composer_placeholder AXButton"),
            ("retry", "Try again", "notification_action AXButton"),
            ("model", "Choose model", "model_picker AXPopUpButton"),
        ] {
            let context = context(SurfaceFamily::AgentChat);
            let spans = build_ordered_spans(&context, vec![raw(id, label, hint, 650.0, 600.0)]);
            assert_eq!(spans[0].region_role, RegionRole::Control, "{label}");
            let selection = select_latest_turn(&context, &spans);
            assert!(selection.latest_user_span_ids.is_empty(), "{label}");
            assert!(selection.salient_user_goal_sample.is_none(), "{label}");
        }
    }

    #[test]
    fn ax_structure_preserves_user_agent_and_prior_order() {
        let context = context(SurfaceFamily::AgentChat);
        let spans = build_ordered_spans(
            &context,
            vec![
                raw(
                    "prior",
                    "Verification passed",
                    "conversation-history-assistant-message",
                    300.0,
                    100.0,
                ),
                raw(
                    "user",
                    "What should Capture do?",
                    "conversation-user-message",
                    500.0,
                    300.0,
                ),
                raw(
                    "agent",
                    "Tracing the bridge",
                    "conversation-assistant-message",
                    300.0,
                    400.0,
                ),
            ],
        );
        let selection = select_latest_turn(&context, &spans);
        assert_eq!(selection.latest_user_span_ids.len(), 1);
        assert_eq!(selection.current_agent_span_ids.len(), 1);
        assert_eq!(selection.prior_boundary_span_ids.len(), 1);
        let agent = spans
            .iter()
            .find(|span| selection.current_agent_span_ids.contains(&span.span_id))
            .unwrap();
        assert_eq!(agent.region_role, RegionRole::AgentStatus);
    }

    #[test]
    fn pane_segmentation_prevents_bottom_terminal_from_becoming_chat_turn() {
        let context = context(SurfaceFamily::AgentChat);
        let spans = build_ordered_spans(
            &context,
            vec![
                raw(
                    "user",
                    "Investigate Capture",
                    "conversation-user-message",
                    400.0,
                    300.0,
                ),
                raw(
                    "agent",
                    "Tracing implementation",
                    "conversation-assistant-message",
                    300.0,
                    400.0,
                ),
                raw(
                    "terminal",
                    "Verification passed",
                    "tool_output",
                    1_300.0,
                    900.0,
                ),
            ],
        );
        assert!(
            spans
                .iter()
                .map(|span| &span.pane_id)
                .collect::<BTreeSet<_>>()
                .len()
                >= 2
        );
        let selection = select_latest_turn(&context, &spans);
        assert_eq!(selection.current_agent_span_ids.len(), 1);
        assert!(selection.rejected_spans.iter().any(|item| {
            spans.iter().any(|span| {
                span.span_id == item.span_id && span.region_role == RegionRole::ToolOutput
            })
        }));
    }

    #[test]
    fn duplicate_ax_and_ocr_merge_retains_provenance() {
        let context = context(SurfaceFamily::AgentChat);
        let mut ocr = raw("ocr-user", "Investigate Capture", "", 400.0, 300.0);
        ocr.source = source_ref(EvidenceSourceKind::OcrSpan, "ocr-user", &ocr.text);
        let spans = build_ordered_spans(
            &context,
            vec![
                raw(
                    "ax-user",
                    "Investigate Capture",
                    "user-message",
                    400.0,
                    300.0,
                ),
                ocr,
            ],
        );
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].contributing_sources.len(), 2);
        assert!(spans[0]
            .quality_flags
            .contains(&"merged_duplicate_provenance".to_string()));
    }

    #[test]
    fn thin_ocr_uses_pane_geometry_without_global_tail_order() {
        let context = context(SurfaceFamily::AgentChat);
        let mut navigation = raw("nav", "Old project name", "static_text", 20.0, 700.0);
        navigation.geometry.as_mut().unwrap().width = 80.0;
        let mut user = raw(
            "ocr-user",
            "Investigate the Capture control",
            "static_text",
            650.0,
            420.0,
        );
        let mut agent = raw(
            "ocr-agent",
            "Tracing the bridge and handler",
            "static_text",
            320.0,
            500.0,
        );
        let mut terminal = raw(
            "terminal",
            "Verification passed",
            "static_text",
            1_300.0,
            900.0,
        );
        for span in [&mut navigation, &mut user, &mut agent, &mut terminal] {
            span.source = source_ref(
                EvidenceSourceKind::OcrSpan,
                &span.source.source_record_id,
                &span.text,
            );
            span.parent_id = None;
            span.source_confidence = 0.72;
        }
        let spans = build_ordered_spans(&context, vec![navigation, user, agent, terminal]);
        assert!(spans
            .iter()
            .any(|span| span.region_role == RegionRole::UserMessage));
        assert!(spans
            .iter()
            .any(|span| span.region_role == RegionRole::AgentStatus));
        assert!(spans
            .iter()
            .any(|span| span.region_role == RegionRole::Navigation));
        assert!(spans
            .iter()
            .any(|span| span.region_role == RegionRole::TerminalOutput));
        let selection = select_latest_turn(&context, &spans);
        assert_eq!(selection.latest_user_span_ids.len(), 1);
        assert_eq!(selection.current_agent_span_ids.len(), 1);
    }

    #[test]
    fn generic_static_text_abstains_and_flattened_fallback_is_downgraded() {
        let context = context(SurfaceFamily::AgentChat);
        let mut generic = raw("static", "I can help", "static_text", 300.0, 300.0);
        generic.source = source_ref(
            EvidenceSourceKind::FlattenedTextFallback,
            "frame:7",
            &generic.text,
        );
        generic.parent_id = None;
        generic.geometry = None;
        generic
            .quality_flags
            .push("flattened_text_fallback".to_string());
        let spans = build_ordered_spans(&context, vec![generic]);
        assert_eq!(spans[0].region_role, RegionRole::Unknown);
        assert_eq!(spans[0].conversational_role, ConversationalRole::Unknown);
        assert!(spans[0].speaker_confidence < MIN_TYPED_ROLE_CONFIDENCE);
        assert!(spans[0]
            .quality_flags
            .contains(&"flattened_text_fallback".to_string()));
        let selection = select_latest_turn(&context, &spans);
        assert!(selection
            .fallback_flags
            .contains(&"flattened_text_fallback".to_string()));
    }

    #[test]
    fn privacy_blocked_surface_keeps_roles_but_omits_samples() {
        let mut context = context(SurfaceFamily::AgentChat);
        context.privacy_status = "sensitive_blocked".to_string();
        let spans = build_ordered_spans(
            &context,
            vec![
                raw("user", "Private user request", "user-message", 500.0, 300.0),
                raw(
                    "agent",
                    "Private agent status",
                    "assistant-message",
                    280.0,
                    400.0,
                ),
            ],
        );
        let selection = select_latest_turn(&context, &spans);
        assert_eq!(selection.latest_user_span_ids.len(), 1);
        assert_eq!(selection.current_agent_span_ids.len(), 1);
        assert!(selection.salient_user_goal_sample.is_none());
        assert!(selection.salient_agent_state_sample.is_none());
    }

    #[test]
    fn samples_are_redacted_and_bounded() {
        let context = context(SurfaceFamily::AgentChat);
        let long = format!("token=secret {}", "work ".repeat(200));
        let spans = build_ordered_spans(
            &context,
            vec![
                raw("user", &long, "user-message", 400.0, 300.0),
                raw("agent", &long, "assistant-message", 300.0, 400.0),
            ],
        );
        let selection = select_latest_turn(&context, &spans);
        let combined = selection_semantic_text(&selection);
        assert!(combined.chars().count() <= MAX_TOTAL_SAMPLE_CHARS);
        assert!(!combined.contains("token=secret"));
    }

    #[test]
    fn stale_prefix_cannot_displace_latest_typed_turn() {
        let context = context(SurfaceFamily::AgentChat);
        let stale = format!("old navigation {}", "chrome ".repeat(180));
        let spans = build_ordered_spans(
            &context,
            vec![
                raw("nav", &stale, "sidebar-navigation", 20.0, 100.0),
                raw(
                    "user",
                    "What does Capture do?",
                    "user-message",
                    500.0,
                    420.0,
                ),
                raw(
                    "agent",
                    "Tracing the bridge",
                    "assistant-message",
                    280.0,
                    500.0,
                ),
            ],
        );
        let selection = select_latest_turn(&context, &spans);
        let sample = selection_semantic_text(&selection);
        assert!(sample.contains("What does Capture do?"));
        assert!(sample.contains("Tracing the bridge"));
        assert!(!sample.contains("old navigation"));
    }

    #[test]
    fn schema_rebuild_is_idempotent_and_does_not_duplicate_source_text() {
        let conn = Connection::open_in_memory().unwrap();
        crate::capture::init_db(&conn).unwrap();
        conn.execute(
            "INSERT INTO capture_sessions
             (id, sequence, started_at_ms, status, created_at_ms)
             VALUES ('session', 1, 1, 'active', 1)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO frames
             (id, session_id, captured_at, snapshot_path, app_name, window_name,
              focused, capture_trigger, privacy_status, created_at)
             VALUES (1, 'session', 100, 'fixture', 'Codex', 'Task', 1, 'manual', 'normal', 100)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO app_contexts
             (id, frame_id, adapter_id, object_type, confidence)
             VALUES ('context', '1', 'codex', 'chat_conversation', 0.9)",
            [],
        )
        .unwrap();
        for (id, identifier, text, order, x, y) in [
            (
                "ax-user",
                "conversation-user-message",
                "What does Capture do?",
                1,
                500.0,
                420.0,
            ),
            (
                "ax-agent",
                "conversation-assistant-message",
                "Tracing the bridge",
                2,
                280.0,
                500.0,
            ),
        ] {
            conn.execute(
                "INSERT INTO ax_nodes
                 (id, frame_id, role, identifier, value, bounds_x, bounds_y,
                  bounds_w, bounds_h, depth, tree_order, raw_json)
                 VALUES (?1, '1', 'static_text', ?2, ?3, ?4, ?5, 400, 60, 1, ?6, '{}')",
                params![id, identifier, text, x, y, order],
            )
            .unwrap();
        }
        conn.execute(
            "INSERT INTO typing_bursts
             (id, session_id, started_at_ms, ended_at_ms, char_count, enter_count,
              committed, raw_text_captured, post_frame_id)
             VALUES ('typing', 'session', 90, 95, 20, 1, 1, 0, '1')",
            [],
        )
        .unwrap();
        let first = rebuild_task_turn_evidence(&conn, &["1".to_string()]).unwrap();
        let second = rebuild_task_turn_evidence(&conn, &["1".to_string()]).unwrap();
        assert_eq!(first.span_count, second.span_count);
        let span_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM continue_ordered_evidence_spans",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(span_count, 2);
        let durable_text_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM continue_ordered_evidence_spans
                 WHERE bounded_public_safe_summary IS NOT NULL",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(durable_text_count, 0);
        let user_sample: Option<String> = conn
            .query_row(
                "SELECT salient_user_goal_sample FROM continue_salient_turn_evidence WHERE frame_id='1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(user_sample.as_deref(), Some("What does Capture do?"));
    }

    #[test]
    fn span_ids_and_order_are_deterministic() {
        let context = context(SurfaceFamily::AgentChat);
        let input = vec![
            raw("user", "Question", "user-message", 400.0, 300.0),
            raw("agent", "Working", "assistant-message", 300.0, 400.0),
        ];
        let first = build_ordered_spans(&context, input.clone());
        let second = build_ordered_spans(&context, input);
        assert_eq!(
            first
                .iter()
                .map(|span| (&span.span_id, span.reading_order))
                .collect::<Vec<_>>(),
            second
                .iter()
                .map(|span| (&span.span_id, span.reading_order))
                .collect::<Vec<_>>()
        );
    }
}
