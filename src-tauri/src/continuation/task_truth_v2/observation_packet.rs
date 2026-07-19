use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use super::super::{
    stable_hash, task_turn_evidence::is_categorical_control_hint, EvidenceContentUnit,
    EvidenceFrame, EvidenceOcrSpan, Rect,
};

pub(crate) const OBSERVATION_PACKET_SCHEMA_V2: &str = "smalltalk.observation_packet.v2";
pub(crate) const TASK_RELEVANCE_EVIDENCE_SCHEMA_V1: &str = "smalltalk.task_relevance_evidence.v1";
const MAX_KEYFRAMES: usize = 4;
const MAX_SURFACE_VISITS: usize = 8;
const MAX_ELEMENTS: usize = 160;
const MAX_CAUSAL_EVENTS: usize = 96;
const MAX_NOTES: usize = 32;
const MAX_BROWSER_CHROME_ELEMENTS: usize = 16;
const MAX_PACKET_BYTES: usize = 512 * 1024;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub(crate) enum EvidencePartitionV2 {
    Current,
    Prior,
    Background,
    Support,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RegionRoleV2 {
    PrimaryContent,
    UserAuthoredContent,
    ApplicationAgentOutput,
    ComposerEditor,
    Navigation,
    Toolbar,
    Control,
    Status,
    Notification,
    Sidebar,
    Modal,
    BrowserChrome,
    TerminalInput,
    TerminalOutput,
    DocumentCanvas,
    Unknown,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AuthorshipStatusV2 {
    User,
    ApplicationOrAgent,
    Mixed,
    Unknown,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TaskEvidenceRoleV1 {
    LatestUserGoal,
    CurrentUnsentDraft,
    CurrentAgentState,
    PriorTaskBoundary,
    CurrentTaskContext,
    SupportingContext,
    FlattenedFallback,
    Unknown,
}

impl TaskEvidenceRoleV1 {
    pub(crate) fn authority_rank(self) -> i64 {
        match self {
            Self::LatestUserGoal => 700,
            Self::CurrentUnsentDraft => 710,
            Self::CurrentAgentState => 600,
            Self::PriorTaskBoundary => 500,
            Self::CurrentTaskContext => 400,
            Self::SupportingContext => 200,
            Self::FlattenedFallback => 50,
            Self::Unknown => 0,
        }
    }

    fn selection_reason(self) -> &'static str {
        match self {
            Self::LatestUserGoal => "p6_latest_user_goal",
            Self::CurrentUnsentDraft => "p6_current_unsent_draft",
            Self::CurrentAgentState => "p6_current_agent_state",
            Self::PriorTaskBoundary => "p6_prior_task_boundary",
            Self::CurrentTaskContext => "p6_current_task_context",
            Self::SupportingContext => "p6_supporting_context",
            Self::FlattenedFallback => "flattened_window_fallback",
            Self::Unknown => "task_relation_unknown",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct PacketTaskTurnV1 {
    pub(crate) task_turn_id: String,
    pub(crate) revision: i64,
    pub(crate) execution_state: String,
    pub(crate) current_actor: String,
    pub(crate) waiting_on: String,
    pub(crate) relation_to_prior: String,
    pub(crate) goal_confidence: f64,
    pub(crate) actor_state_confidence: f64,
    pub(crate) relation_confidence: f64,
    pub(crate) attribution_confidence: f64,
    pub(crate) missing_evidence: Vec<String>,
    pub(crate) quality_flags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct TaskRelevantSpanV1 {
    pub(crate) span_id: String,
    pub(crate) frame_id: String,
    pub(crate) surface_key_hash: Option<String>,
    pub(crate) pane_id: String,
    pub(crate) region_role: String,
    pub(crate) conversational_role: String,
    pub(crate) evidence_role: TaskEvidenceRoleV1,
    pub(crate) reading_order: i64,
    pub(crate) source_scope: String,
    pub(crate) ownership_kind: String,
    pub(crate) owner_window_id: Option<i64>,
    pub(crate) owner_app_id: Option<String>,
    pub(crate) ownership_confidence: f64,
    pub(crate) region_confidence: f64,
    pub(crate) speaker_confidence: f64,
    pub(crate) order_confidence: f64,
    pub(crate) privacy_status: String,
    pub(crate) text_hash: String,
    pub(crate) focused: bool,
    pub(crate) selected: bool,
    pub(crate) submitted: Option<bool>,
    pub(crate) geometry: Option<PacketBoundsV2>,
    pub(crate) quality_flags: Vec<String>,
    pub(crate) reason_codes: Vec<String>,
    pub(crate) missing_evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct TaskRelevanceEvidenceV1 {
    pub(crate) schema: String,
    pub(crate) source: String,
    pub(crate) current_task_turn: Option<PacketTaskTurnV1>,
    pub(crate) latest_user_goal_sample: Option<String>,
    pub(crate) current_agent_state_sample: Option<String>,
    pub(crate) prior_task_boundary_sample: Option<String>,
    pub(crate) current_unsent_draft_present: bool,
    pub(crate) spans: Vec<TaskRelevantSpanV1>,
    pub(crate) missing_evidence: Vec<String>,
    pub(crate) fallback_flags: Vec<String>,
    pub(crate) confidence_cap: f64,
}

impl Default for TaskRelevanceEvidenceV1 {
    fn default() -> Self {
        Self {
            schema: TASK_RELEVANCE_EVIDENCE_SCHEMA_V1.into(),
            source: "flattened_window_fallback".into(),
            current_task_turn: None,
            latest_user_goal_sample: None,
            current_agent_state_sample: None,
            prior_task_boundary_sample: None,
            current_unsent_draft_present: false,
            spans: Vec::new(),
            missing_evidence: vec!["p6_role_region_task_turn_evidence_unavailable".into()],
            fallback_flags: vec!["flattened_text_fallback_low_authority".into()],
            confidence_cap: 0.35,
        }
    }
}

#[derive(Debug, Clone)]
struct P6SelectionRow {
    frame_id: String,
    observed_at_ms: i64,
    salient_span_ids: Vec<String>,
    user_span_ids: Vec<String>,
    agent_span_ids: Vec<String>,
    prior_span_ids: Vec<String>,
    user_sample: Option<String>,
    agent_sample: Option<String>,
    prior_sample: Option<String>,
    sampling_confidence: f64,
    missing_roles: Vec<String>,
    rejected_span_ids: Vec<String>,
    fallback_flags: Vec<String>,
    committed_input_attribution: bool,
}

fn parse_string_array(value: &str) -> Vec<String> {
    serde_json::from_str::<Vec<String>>(value).unwrap_or_default()
}

fn parse_rejected_span_ids(value: &str) -> Vec<String> {
    serde_json::from_str::<Value>(value)
        .ok()
        .and_then(|value| value.as_array().cloned())
        .unwrap_or_default()
        .into_iter()
        .filter_map(|entry| {
            entry
                .get("span_id")
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .collect()
}

fn bounded_safe_sample(value: Option<String>, max_chars: usize) -> Option<String> {
    value.and_then(|value| {
        let normalized = value.split_whitespace().collect::<Vec<_>>().join(" ");
        (!normalized.is_empty()).then(|| normalized.chars().take(max_chars).collect())
    })
}

fn sqlite_table_exists(conn: &Connection, table: &str) -> Result<bool, String> {
    conn.query_row(
        "SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1 LIMIT 1",
        [table],
        |_| Ok(()),
    )
    .optional()
    .map(|row| row.is_some())
    .map_err(|error| error.to_string())
}

fn load_p6_selection_row(
    conn: &Connection,
    frame_id: &str,
) -> Result<Option<P6SelectionRow>, String> {
    conn.query_row(
        "SELECT frame_id, observed_at_ms, salient_span_ids_json, latest_user_span_ids_json,
                current_agent_span_ids_json, prior_boundary_span_ids_json,
                salient_user_goal_sample, salient_agent_state_sample,
                prior_boundary_sample, sampling_confidence, missing_roles_json,
                rejected_spans_json, fallback_flags_json, causal_typing_attribution_json
         FROM continue_salient_turn_evidence WHERE frame_id=?1",
        [frame_id],
        |row| {
            let salient_json = row.get::<_, String>(2)?;
            let user_json = row.get::<_, String>(3)?;
            let agent_json = row.get::<_, String>(4)?;
            let prior_json = row.get::<_, String>(5)?;
            let missing_json = row.get::<_, String>(10)?;
            let rejected_json = row.get::<_, String>(11)?;
            let fallback_json = row.get::<_, String>(12)?;
            Ok(P6SelectionRow {
                frame_id: row.get(0)?,
                observed_at_ms: row.get(1)?,
                salient_span_ids: parse_string_array(&salient_json),
                user_span_ids: parse_string_array(&user_json),
                agent_span_ids: parse_string_array(&agent_json),
                prior_span_ids: parse_string_array(&prior_json),
                user_sample: row.get(6)?,
                agent_sample: row.get(7)?,
                prior_sample: row.get(8)?,
                sampling_confidence: row.get(9)?,
                missing_roles: parse_string_array(&missing_json),
                rejected_span_ids: parse_rejected_span_ids(&rejected_json),
                fallback_flags: parse_string_array(&fallback_json),
                committed_input_attribution: row.get::<_, Option<String>>(13)?.is_some(),
            })
        },
    )
    .optional()
    .map_err(|error| error.to_string())
}

fn load_task_relevant_span(
    conn: &Connection,
    span_id: &str,
    evidence_role: TaskEvidenceRoleV1,
) -> Result<Option<TaskRelevantSpanV1>, String> {
    conn.query_row(
        "SELECT span_id, frame_id, surface_key, pane_id, region_role,
                conversational_role, reading_order, source_scope, ownership_kind,
                owner_window_id, owner_app_id, ownership_confidence, region_confidence,
                speaker_confidence, order_confidence, privacy_status, text_hash, focused,
                selected, geometry_json, quality_flags_json, reason_codes_json
         FROM continue_ordered_evidence_spans WHERE span_id=?1",
        [span_id],
        |row| {
            let geometry_json = row.get::<_, Option<String>>(19)?;
            let geometry = geometry_json
                .as_deref()
                .and_then(|value| serde_json::from_str::<PacketBoundsV2>(value).ok());
            let mut missing_evidence = Vec::new();
            if geometry.is_none() {
                missing_evidence.push("geometry_unavailable_or_unreliable".into());
            }
            let owner_window_id = row.get::<_, Option<i64>>(9)?;
            let owner_app_id = row.get::<_, Option<String>>(10)?;
            if owner_window_id.is_none() && owner_app_id.is_none() {
                missing_evidence.push("surface_owner_identifier_missing".into());
            }
            Ok(TaskRelevantSpanV1 {
                span_id: row.get(0)?,
                frame_id: row.get(1)?,
                surface_key_hash: row
                    .get::<_, Option<String>>(2)?
                    .map(|value| stable_hash(format!("task_surface|{value}").as_bytes())),
                pane_id: row.get(3)?,
                region_role: row.get(4)?,
                conversational_role: row.get(5)?,
                evidence_role,
                reading_order: row.get(6)?,
                source_scope: row.get(7)?,
                ownership_kind: row.get(8)?,
                owner_window_id,
                owner_app_id,
                ownership_confidence: row.get(11)?,
                region_confidence: row.get(12)?,
                speaker_confidence: row.get(13)?,
                order_confidence: row.get(14)?,
                privacy_status: row.get(15)?,
                text_hash: row.get(16)?,
                focused: row.get::<_, i64>(17)? != 0,
                selected: row.get::<_, i64>(18)? != 0,
                submitted: (evidence_role == TaskEvidenceRoleV1::CurrentUnsentDraft)
                    .then_some(false),
                geometry,
                quality_flags: parse_string_array(&row.get::<_, String>(20)?),
                reason_codes: parse_string_array(&row.get::<_, String>(21)?),
                missing_evidence,
            })
        },
    )
    .optional()
    .map_err(|error| error.to_string())
}

fn span_is_focused_composer(conn: &Connection, span_id: &str) -> Result<bool, String> {
    conn.query_row(
        "SELECT region_role, focused FROM continue_ordered_evidence_spans WHERE span_id=?1",
        [span_id],
        |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
    )
    .optional()
    .map(|row| {
        row.is_some_and(|(role, focused)| {
            focused != 0 && matches!(role.as_str(), "composer" | "terminal_input")
        })
    })
    .map_err(|error| error.to_string())
}

pub(super) fn load_task_relevance_evidence(
    conn: &Connection,
    frames: &[EvidenceFrame],
) -> Result<TaskRelevanceEvidenceV1, String> {
    let Some(current) = frames.last() else {
        return Ok(TaskRelevanceEvidenceV1::default());
    };
    if !sqlite_table_exists(conn, "continue_salient_turn_evidence")?
        || !sqlite_table_exists(conn, "continue_ordered_evidence_spans")?
    {
        return Ok(TaskRelevanceEvidenceV1::default());
    }

    let frame_ids = frames
        .iter()
        .map(|frame| frame.id.as_str())
        .collect::<BTreeSet<_>>();
    let mut selections = frames
        .iter()
        .filter_map(|frame| load_p6_selection_row(conn, &frame.id).transpose())
        .collect::<Result<Vec<_>, _>>()?;
    selections.sort_by_key(|row| (row.observed_at_ms, row.frame_id.clone()));
    let latest_selection = selections.last().cloned();

    let current_turn = if sqlite_table_exists(conn, "continue_task_turns")? {
        super::super::task_turn::selected_current_task_turn(conn)?
            .filter(|turn| turn.session_id == current.session_id)
            .filter(|turn| turn.last_observed_at_ms <= current.captured_at)
            .filter(|turn| {
                turn.latest_user_span_ids
                    .iter()
                    .chain(turn.current_state_span_ids.iter())
                    .chain(turn.prior_boundary_span_ids.iter())
                    .any(|span_id| {
                        selections.iter().any(|selection| {
                            selection.user_span_ids.contains(span_id)
                                || selection.agent_span_ids.contains(span_id)
                                || selection.prior_span_ids.contains(span_id)
                        })
                    })
            })
    } else {
        None
    };

    let mut user_ids = current_turn
        .as_ref()
        .map(|turn| turn.latest_user_span_ids.clone())
        .unwrap_or_default();
    let mut agent_ids = current_turn
        .as_ref()
        .map(|turn| turn.current_state_span_ids.clone())
        .unwrap_or_default();
    let mut prior_ids = current_turn
        .as_ref()
        .map(|turn| turn.prior_boundary_span_ids.clone())
        .unwrap_or_default();
    if let Some(selection) = latest_selection.as_ref() {
        if user_ids.is_empty() {
            user_ids = selection.user_span_ids.clone();
        }
        if agent_ids.is_empty() {
            agent_ids = selection.agent_span_ids.clone();
        }
        if prior_ids.is_empty() {
            prior_ids = selection.prior_span_ids.clone();
        }
    }
    user_ids.retain(|id| {
        selections
            .iter()
            .any(|selection| selection.user_span_ids.contains(id))
    });
    agent_ids.retain(|id| {
        selections
            .iter()
            .any(|selection| selection.agent_span_ids.contains(id))
    });
    prior_ids.retain(|id| {
        selections
            .iter()
            .any(|selection| selection.prior_span_ids.contains(id))
    });

    let role_by_span = user_ids
        .iter()
        .map(|id| (id.clone(), TaskEvidenceRoleV1::LatestUserGoal))
        .chain(
            agent_ids
                .iter()
                .map(|id| (id.clone(), TaskEvidenceRoleV1::CurrentAgentState)),
        )
        .chain(
            prior_ids
                .iter()
                .map(|id| (id.clone(), TaskEvidenceRoleV1::PriorTaskBoundary)),
        )
        .collect::<BTreeMap<_, _>>();
    let mut span_roles = role_by_span;
    for selection in &selections {
        if selection.committed_input_attribution {
            continue;
        }
        for span_id in &selection.salient_span_ids {
            if !span_roles.contains_key(span_id) && span_is_focused_composer(conn, span_id)? {
                span_roles.insert(span_id.clone(), TaskEvidenceRoleV1::CurrentUnsentDraft);
            }
        }
    }
    for selection in &selections {
        for span_id in selection.rejected_span_ids.iter().take(24) {
            span_roles
                .entry(span_id.clone())
                .or_insert(TaskEvidenceRoleV1::SupportingContext);
        }
    }
    let mut spans = span_roles
        .into_iter()
        .filter_map(|(span_id, role)| load_task_relevant_span(conn, &span_id, role).transpose())
        .collect::<Result<Vec<_>, _>>()?;
    spans.retain(|span| frame_ids.contains(span.frame_id.as_str()));
    spans.sort_by_key(|span| {
        (
            span.frame_id.clone(),
            span.reading_order,
            span.span_id.clone(),
        )
    });
    spans.truncate(64);

    if spans.is_empty() {
        return Ok(TaskRelevanceEvidenceV1::default());
    }

    let mut missing_evidence = latest_selection
        .as_ref()
        .map(|row| row.missing_roles.clone())
        .unwrap_or_default();
    if user_ids.is_empty() {
        missing_evidence.push("latest_user_goal_span_missing".into());
    }
    if agent_ids.is_empty() {
        missing_evidence.push("current_agent_state_span_missing".into());
    }
    missing_evidence.sort();
    missing_evidence.dedup();

    let mut fallback_flags = latest_selection
        .as_ref()
        .map(|row| row.fallback_flags.clone())
        .unwrap_or_default();
    fallback_flags.sort();
    fallback_flags.dedup();
    let sampling_confidence = latest_selection
        .as_ref()
        .map(|row| row.sampling_confidence)
        .unwrap_or(0.0)
        .clamp(0.0, 1.0);
    let packet_turn = current_turn.as_ref().map(|turn| PacketTaskTurnV1 {
        task_turn_id: turn.task_turn_id.clone(),
        revision: turn.revision,
        execution_state: turn.execution_state.label().into(),
        current_actor: turn.current_actor.label().into(),
        waiting_on: turn.waiting_on.label().into(),
        relation_to_prior: turn.relation_to_prior.label().into(),
        goal_confidence: turn.goal_confidence,
        actor_state_confidence: turn.actor_state_confidence,
        relation_confidence: turn.relation_confidence,
        attribution_confidence: turn.attribution_confidence,
        missing_evidence: turn.missing_evidence.clone(),
        quality_flags: turn.quality_flags.clone(),
    });
    let confidence_cap = packet_turn
        .as_ref()
        .map(|turn| {
            sampling_confidence
                .min(turn.attribution_confidence)
                .min(turn.goal_confidence.max(turn.actor_state_confidence))
        })
        .unwrap_or(sampling_confidence.min(0.7));
    let latest_user_goal_sample = current_turn
        .as_ref()
        .and_then(|turn| turn.latest_user_goal_summary.clone())
        .or_else(|| {
            latest_selection
                .as_ref()
                .and_then(|row| row.user_sample.clone())
        });
    let current_agent_state_sample = current_turn
        .as_ref()
        .and_then(|turn| turn.actor_activity_state.clone())
        .or_else(|| {
            latest_selection
                .as_ref()
                .and_then(|row| row.agent_sample.clone())
        });

    Ok(TaskRelevanceEvidenceV1 {
        schema: TASK_RELEVANCE_EVIDENCE_SCHEMA_V1.into(),
        source: "p6_role_region_task_turn".into(),
        current_task_turn: packet_turn,
        latest_user_goal_sample: bounded_safe_sample(latest_user_goal_sample, 280),
        current_agent_state_sample: bounded_safe_sample(current_agent_state_sample, 220),
        prior_task_boundary_sample: bounded_safe_sample(
            latest_selection.and_then(|row| row.prior_sample),
            220,
        ),
        current_unsent_draft_present: spans
            .iter()
            .any(|span| span.evidence_role == TaskEvidenceRoleV1::CurrentUnsentDraft),
        spans,
        missing_evidence,
        fallback_flags,
        confidence_cap,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct ImageCandidateAuditV1 {
    pub(crate) frame_id: String,
    pub(crate) task_turn_id: Option<String>,
    pub(crate) evidence_role: TaskEvidenceRoleV1,
    pub(crate) selected: bool,
    pub(crate) selection_reasons: Vec<String>,
    pub(crate) rejection_reasons: Vec<String>,
    pub(crate) supports_latest_user_goal: bool,
    pub(crate) supports_current_agent_state: bool,
    pub(crate) supports_prior_task_boundary: bool,
    pub(crate) same_task_relation: String,
    pub(crate) cross_pane_ambiguity: bool,
    pub(crate) near_duplicate_group: Option<String>,
    pub(crate) engagement_used_as_same_task_tiebreaker: bool,
    pub(crate) original_width: Option<i64>,
    pub(crate) original_height: Option<i64>,
    pub(crate) prepared_width: Option<i64>,
    pub(crate) prepared_height: Option<i64>,
    pub(crate) image_scope: String,
    pub(crate) crop_policy: String,
    pub(crate) redaction_status: String,
    pub(crate) preparation_reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct PacketBoundsV2 {
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) width: f64,
    pub(crate) height: f64,
}

impl From<Rect> for PacketBoundsV2 {
    fn from(value: Rect) -> Self {
        Self {
            x: value.x,
            y: value.y,
            width: value.w,
            height: value.h,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct EvidenceHandleV2 {
    pub(crate) source_kind: String,
    pub(crate) record_id: String,
    pub(crate) frame_id: Option<String>,
    pub(crate) content_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct CanonicalElementV2 {
    pub(crate) element_id: String,
    pub(crate) frame_id: String,
    pub(crate) bounds: Option<PacketBoundsV2>,
    pub(crate) display_id: Option<String>,
    pub(crate) window_id: Option<i64>,
    pub(crate) owning_app_bundle: Option<String>,
    pub(crate) source_scope: Option<String>,
    pub(crate) ownership_kind: Option<String>,
    pub(crate) ownership_confidence: Option<f64>,
    pub(crate) coordinate_space: String,
    pub(crate) freshness: String,
    pub(crate) text_reference: Option<String>,
    pub(crate) visual_description: Option<String>,
    pub(crate) native_role: Option<String>,
    pub(crate) native_subrole: Option<String>,
    pub(crate) native_actionability: bool,
    pub(crate) region_role: RegionRoleV2,
    pub(crate) focused: bool,
    pub(crate) editable: bool,
    pub(crate) selected: bool,
    pub(crate) interactive: bool,
    pub(crate) parent_element_id: Option<String>,
    pub(crate) child_element_ids: Vec<String>,
    pub(crate) source_votes: Vec<String>,
    pub(crate) source_conflicts: Vec<String>,
    pub(crate) first_seen_at_ms: i64,
    pub(crate) changed_at_ms: i64,
    pub(crate) authorship_status: AuthorshipStatusV2,
    pub(crate) causal_evidence_refs: Vec<EvidenceHandleV2>,
    pub(crate) task_eligible: bool,
    pub(crate) rejection_reasons: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct KeyframeReferenceV2 {
    pub(crate) frame_id: String,
    pub(crate) observed_at_ms: i64,
    pub(crate) partition: EvidencePartitionV2,
    pub(crate) surface_identity: ActiveSurfaceIdentityV2,
    pub(crate) surface_ownership_confidence: f64,
    pub(crate) privacy_status: String,
    pub(crate) model_eligible: bool,
    pub(crate) image_source_kind: String,
    pub(crate) image_scope: String,
    pub(crate) image_width: Option<i64>,
    pub(crate) image_height: Option<i64>,
    pub(crate) image_rejection_reason: Option<String>,
    pub(crate) crop_pixels: Option<PacketBoundsV2>,
    pub(crate) local_image_handle_hash: Option<String>,
    /// Available only while handling the explicit Continue request. The local path is
    /// deliberately omitted from serialization so checkpoints and audits retain only
    /// the hash above.
    #[serde(skip)]
    pub(crate) ephemeral_local_image_path: Option<String>,
    pub(crate) selection_reasons: Vec<String>,
    #[serde(default)]
    pub(crate) task_evidence_role: Option<TaskEvidenceRoleV1>,
    #[serde(default)]
    pub(crate) task_turn_id: Option<String>,
    #[serde(default)]
    pub(crate) same_task_relation: String,
    #[serde(default)]
    pub(crate) cross_pane_ambiguity: bool,
    #[serde(default)]
    pub(crate) near_duplicate_group: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct SurfaceVisitV2 {
    pub(crate) sequence_index: usize,
    pub(crate) app_label: String,
    pub(crate) site_hostname: Option<String>,
    pub(crate) first_observed_at_ms: i64,
    pub(crate) last_observed_at_ms: i64,
    pub(crate) is_current: bool,
    pub(crate) revisited: bool,
    pub(crate) private: bool,
    pub(crate) interaction_count: usize,
    pub(crate) frame_count: usize,
    pub(crate) engagement_score: i64,
    #[serde(default)]
    pub(crate) committed_input: bool,
    #[serde(default)]
    pub(crate) carried_into_current_surface: bool,
    #[serde(default)]
    pub(crate) hostname_mentioned_in_current_surface: bool,
    #[serde(default)]
    pub(crate) task_evidence_role: Option<TaskEvidenceRoleV1>,
    #[serde(default)]
    pub(crate) task_relevance_score: i64,
    #[serde(default)]
    pub(crate) same_task_relation: String,
    #[serde(default)]
    pub(crate) cross_pane_ambiguity: bool,
    #[serde(default)]
    pub(crate) engagement_used_as_same_task_tiebreaker: bool,
    pub(crate) evidence_refs: Vec<String>,
    pub(crate) representative_frame: Option<KeyframeReferenceV2>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ActiveSurfaceIdentityV2 {
    pub(crate) app_name: Option<String>,
    pub(crate) app_bundle_id: Option<String>,
    pub(crate) window_title_hash: Option<String>,
    pub(crate) window_id: Option<i64>,
    pub(crate) browser_url_hash: Option<String>,
    pub(crate) document_path_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct CausalEventV2 {
    pub(crate) event_id: String,
    pub(crate) event_kind: String,
    pub(crate) observed_at_ms: i64,
    pub(crate) frame_id: String,
    pub(crate) source_frame_id: String,
    pub(crate) target_frame_id: Option<String>,
    pub(crate) target_element_id: Option<String>,
    pub(crate) target_region: Option<RegionRoleV2>,
    pub(crate) focused_element_before: Option<String>,
    pub(crate) focused_element_after: Option<String>,
    pub(crate) window_id: Option<i64>,
    pub(crate) app_bundle_id: Option<String>,
    pub(crate) pointer_x: Option<f64>,
    pub(crate) pointer_y: Option<f64>,
    pub(crate) scroll_delta_x: Option<f64>,
    pub(crate) scroll_delta_y: Option<f64>,
    pub(crate) pre_state_reference: Option<String>,
    pub(crate) post_state_reference: Option<String>,
    pub(crate) semantic_delta_reference: Option<String>,
    pub(crate) grounding_confidence: f64,
    pub(crate) missing_evidence: Vec<String>,
    pub(crate) conflicting_evidence: Vec<String>,
    pub(crate) partition: EvidencePartitionV2,
    pub(crate) causal_parent_ids: Vec<String>,
    pub(crate) committed: Option<bool>,
    pub(crate) source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct FrameChangeV2 {
    pub(crate) delta_id: String,
    pub(crate) frame_id: String,
    pub(crate) prior_frame_id: Option<String>,
    pub(crate) next_frame_id: String,
    pub(crate) diff_kind: Option<String>,
    pub(crate) changed_regions: Vec<PacketBoundsV2>,
    pub(crate) observable_changes: Vec<String>,
    pub(crate) no_observable_change: bool,
    pub(crate) source_agreement: Vec<String>,
    pub(crate) source_conflicts: Vec<String>,
    pub(crate) causal_event_ids: Vec<String>,
    pub(crate) summary_hash: Option<String>,
    pub(crate) added_text_hashes: Option<String>,
    pub(crate) removed_text_hashes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct PacketSizeAccountingV2 {
    pub(crate) frame_count: usize,
    pub(crate) keyframe_count: usize,
    pub(crate) canonical_element_count: usize,
    pub(crate) causal_event_count: usize,
    pub(crate) serialized_bytes: usize,
    pub(crate) estimated_tokens: usize,
    pub(crate) truncated: bool,
    pub(crate) frame_accounting: Vec<FrameCapacityAccountingV2>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct FrameCapacityAccountingV2 {
    pub(crate) frame_id: String,
    pub(crate) partition: EvidencePartitionV2,
    pub(crate) age_rank: usize,
    pub(crate) retained_elements: usize,
    pub(crate) dropped_elements: usize,
    pub(crate) retained_events: usize,
    pub(crate) dropped_events: usize,
    pub(crate) retained_by_source: BTreeMap<String, usize>,
    pub(crate) dropped_by_source: BTreeMap<String, usize>,
    pub(crate) retained_by_role: BTreeMap<String, usize>,
    pub(crate) dropped_by_role: BTreeMap<String, usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct ObservationPacketV2 {
    pub(crate) schema: String,
    pub(crate) packet_id: String,
    pub(crate) observed_at_ms: i64,
    pub(crate) session_id: Option<String>,
    pub(crate) evidence_watermark: String,
    pub(crate) active_surface: ActiveSurfaceIdentityV2,
    pub(crate) current_frame: KeyframeReferenceV2,
    pub(crate) semantic_keyframes: Vec<KeyframeReferenceV2>,
    #[serde(default)]
    pub(crate) surface_timeline: Vec<SurfaceVisitV2>,
    #[serde(default)]
    pub(crate) task_relevance: TaskRelevanceEvidenceV1,
    #[serde(default)]
    pub(crate) image_candidates: Vec<ImageCandidateAuditV1>,
    pub(crate) canonical_elements: Vec<CanonicalElementV2>,
    pub(crate) focused_element_ids: Vec<String>,
    pub(crate) editable_element_ids: Vec<String>,
    pub(crate) selected_element_ids: Vec<String>,
    pub(crate) causal_events: Vec<CausalEventV2>,
    pub(crate) frame_changes: Vec<FrameChangeV2>,
    pub(crate) capture_trigger_ids: Vec<String>,
    pub(crate) transition_ids: Vec<String>,
    pub(crate) return_anchor_facts: Vec<EvidenceHandleV2>,
    pub(crate) previous_valid_snapshot_id: Option<String>,
    pub(crate) evidence_quality: String,
    pub(crate) missing_source_notes: Vec<String>,
    pub(crate) conflicting_observations: Vec<String>,
    pub(crate) partitions: BTreeMap<EvidencePartitionV2, Vec<String>>,
    pub(crate) size: PacketSizeAccountingV2,
}

pub(crate) fn is_private_status(status: Option<&str>) -> bool {
    let status = status.unwrap_or("unknown").trim().to_ascii_lowercase();
    matches!(
        status.as_str(),
        "private" | "blocked" | "secure" | "sensitive" | "denied" | "redacted"
    ) || status.contains("private")
        || status.contains("blocked")
        || status.contains("secure")
        || status.contains("diagnostic_self")
        || status.contains("identity_conflict")
}

fn hash_optional(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| stable_hash(value.as_bytes()))
}

fn normalize_text(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase()
}

fn logical_rect_to_pixels(
    logical: Rect,
    display_origin_x: f64,
    display_origin_y: f64,
    scale: f64,
    pixel_width: i64,
    pixel_height: i64,
) -> Option<PacketBoundsV2> {
    if scale <= 0.0 || pixel_width <= 0 || pixel_height <= 0 || logical.w <= 0.0 || logical.h <= 0.0
    {
        return None;
    }
    let mapped = PacketBoundsV2 {
        x: ((logical.x - display_origin_x) * scale).round(),
        y: ((logical.y - display_origin_y) * scale).round(),
        width: (logical.w * scale).round(),
        height: (logical.h * scale).round(),
    };
    (mapped.x >= 0.0
        && mapped.y >= 0.0
        && mapped.width >= 32.0
        && mapped.height >= 32.0
        && mapped.x + mapped.width <= pixel_width as f64
        && mapped.y + mapped.height <= pixel_height as f64)
        .then_some(mapped)
}

fn resolve_visual_input(
    frame: &EvidenceFrame,
) -> (
    String,
    String,
    Option<String>,
    Option<PacketBoundsV2>,
    Option<String>,
) {
    if is_private_status(frame.privacy_status.as_deref()) {
        return (
            "missing".into(),
            "none".into(),
            None,
            None,
            Some("privacy_blocked".into()),
        );
    }
    if let Some(path) = frame
        .active_window_crop_path
        .as_deref()
        .filter(|path| Path::new(path).is_file())
    {
        return (
            "native_active_window".into(),
            "active_window".into(),
            Some(path.into()),
            None,
            None,
        );
    }
    let Some(full_path) = frame
        .full_screenshot_path
        .as_deref()
        .filter(|path| Path::new(path).is_file())
    else {
        return (
            "missing".into(),
            "none".into(),
            None,
            None,
            Some("no_readable_image_asset".into()),
        );
    };
    let active_window = frame.visible_windows.iter().find(|window| {
        window.is_active
            && frame
                .window_id
                .zip(window.cg_window_id)
                .map(|(a, b)| a == b)
                .unwrap_or(true)
            && frame
                .app_bundle_id
                .as_deref()
                .zip(window.bundle_id.as_deref())
                .map(|(a, b)| a == b)
                .unwrap_or(true)
    });
    if let (Some(window), Some(scale), Some(width), Some(height)) = (
        active_window,
        frame.screen_scale,
        frame.pixel_width,
        frame.pixel_height,
    ) {
        if let Some(crop) = logical_rect_to_pixels(window.bounds, 0.0, 0.0, scale, width, height) {
            return (
                "derived_active_window_crop".into(),
                "active_window".into(),
                Some(full_path.into()),
                Some(crop),
                None,
            );
        }
        return (
            "missing".into(),
            "none".into(),
            None,
            None,
            Some("unverified_active_window_coordinate_mapping".into()),
        );
    }
    if frame.scope.as_deref() == Some("active_window") && frame.window_id.is_some() {
        return (
            "full_display".into(),
            "active_window_equivalent".into(),
            Some(full_path.into()),
            None,
            None,
        );
    }
    (
        "missing".into(),
        "none".into(),
        None,
        None,
        Some("full_display_ownership_not_permitted".into()),
    )
}

fn partition_frames(frames: &[EvidenceFrame]) -> BTreeMap<String, EvidencePartitionV2> {
    let Some(current) = frames.last() else {
        return BTreeMap::new();
    };
    let current_surface = (
        current
            .app_bundle_id
            .as_deref()
            .or(current.app_name.as_deref()),
        current.window_id,
    );
    let mut result = BTreeMap::new();
    for (index, frame) in frames.iter().enumerate() {
        let surface = (
            frame.app_bundle_id.as_deref().or(frame.app_name.as_deref()),
            frame.window_id,
        );
        let partition = if frame.id == current.id {
            EvidencePartitionV2::Current
        } else if surface == current_surface {
            EvidencePartitionV2::Prior
        } else if index + 3 >= frames.len()
            && (frame.focused_node_evidence || !frame.typing_bursts.is_empty())
        {
            EvidencePartitionV2::Support
        } else {
            EvidencePartitionV2::Background
        };
        result.insert(frame.id.clone(), partition);
    }
    if let Some(support) = frames.iter().rev().find(|frame| {
        frame.id != current.id
            && (
                frame.app_bundle_id.as_deref().or(frame.app_name.as_deref()),
                frame.window_id,
            ) != current_surface
            && !is_private_status(frame.privacy_status.as_deref())
            && !is_diagnostic_surface(frame)
            && has_structured_work_surface_evidence(frame)
    }) {
        result.insert(support.id.clone(), EvidencePartitionV2::Support);
    }
    result
}

fn is_diagnostic_surface(frame: &EvidenceFrame) -> bool {
    let bundle = frame
        .app_bundle_id
        .as_deref()
        .unwrap_or("")
        .to_ascii_lowercase();
    let app = frame.app_name.as_deref().unwrap_or("").to_ascii_lowercase();
    // Codex is sometimes used to diagnose Smalltalk, but it is also a normal
    // user workspace. Only Smalltalk's own UI is categorically self-evidence.
    if bundle == "com.smalltalk.app" || app == "smalltalk" {
        return true;
    }
    let captured_window_owner = frame.window_id.and_then(|window_id| {
        frame
            .visible_windows
            .iter()
            .find(|window| window.cg_window_id == Some(window_id))
    });
    captured_window_owner.is_some_and(|window| {
        window.bundle_id.as_deref() == Some("com.smalltalk.app")
            || window
                .owner_name
                .as_deref()
                .is_some_and(|name| name.eq_ignore_ascii_case("smalltalk"))
    })
}

fn event_matches_frame_surface(
    frame: &EvidenceFrame,
    event: &super::super::EvidenceUiEvent,
) -> bool {
    let frame_bundle = frame
        .app_bundle_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let event_bundle = event
        .app_bundle_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    if let (Some(frame_bundle), Some(event_bundle)) = (frame_bundle, event_bundle) {
        if !frame_bundle.eq_ignore_ascii_case(event_bundle) {
            return false;
        }
    }
    if let (Some(frame_window), Some(event_window)) = (frame.window_id, event.window_id) {
        if frame_window != event_window {
            return false;
        }
    }
    true
}

fn typing_burst_matches_frame_surface(
    frame: &EvidenceFrame,
    burst: &super::super::EvidenceTypingBurst,
) -> bool {
    let same_app = match (
        burst.app_bundle_id.as_deref(),
        frame.app_bundle_id.as_deref(),
    ) {
        (Some(expected), Some(actual)) if !expected.trim().is_empty() => {
            expected.eq_ignore_ascii_case(actual)
        }
        _ => match (burst.app_name.as_deref(), frame.app_name.as_deref()) {
            (Some(expected), Some(actual)) if !expected.trim().is_empty() => {
                expected.eq_ignore_ascii_case(actual)
            }
            _ => false,
        },
    };
    let same_window = match (burst.window_id, frame.window_id) {
        (Some(expected), Some(actual)) if expected > 0 && actual > 0 => expected == actual,
        _ => match (burst.window_title.as_deref(), frame.window_name.as_deref()) {
            (Some(expected), Some(actual)) if !expected.trim().is_empty() => {
                expected.trim().eq_ignore_ascii_case(actual.trim())
            }
            _ => false,
        },
    };
    same_app && same_window
}

fn has_structured_work_surface_evidence(frame: &EvidenceFrame) -> bool {
    frame.focused_node_evidence
        || frame.document_path.is_some()
        || frame.app_contexts.iter().any(|context| {
            context.file_path.is_some()
                || context.repo_path.is_some()
                || context.focused_object.is_some()
        })
}

fn browser_origin_key(frame: &EvidenceFrame) -> Option<String> {
    let raw = frame.browser_url.as_deref()?.trim();
    let after_scheme = raw.split_once("://").map(|(_, rest)| rest).unwrap_or(raw);
    let authority = after_scheme
        .split(['/', '?', '#'])
        .next()?
        .rsplit('@')
        .next()?
        .split(':')
        .next()?
        .trim()
        .trim_end_matches('.')
        .trim_start_matches("www.")
        .to_ascii_lowercase();
    if authority.is_empty() {
        None
    } else {
        Some(authority)
    }
}

fn frame_has_visual(frame: &EvidenceFrame) -> bool {
    frame
        .active_window_crop_path
        .as_deref()
        .or(frame.full_screenshot_path.as_deref())
        .is_some_and(|path| !path.trim().is_empty())
}

fn boundary_reasons(frame: &EvidenceFrame) -> Vec<String> {
    let mut reasons = BTreeSet::new();
    let trigger = frame.capture_trigger.to_ascii_lowercase();
    for (needle, reason) in [
        ("submit", "submit_boundary"),
        ("enter", "submit_boundary"),
        ("send", "submit_boundary"),
        ("switch", "surface_switch_boundary"),
        ("focus", "focus_boundary"),
        ("command", "command_boundary"),
        ("error", "visible_error_boundary"),
        ("idle", "idle_after_progress_boundary"),
        ("manual", "manual_continue_boundary"),
    ] {
        if trigger.contains(needle) {
            reasons.insert(reason.to_string());
        }
    }
    for event in frame
        .ui_events
        .iter()
        .filter(|event| event_matches_frame_surface(frame, event))
    {
        let kind = event.event_type.to_ascii_lowercase();
        if kind.contains("switch") || kind.contains("focus") {
            reasons.insert("surface_switch_boundary".into());
        }
        if kind.contains("submit") || kind.contains("command") {
            reasons.insert("causal_action_boundary".into());
        }
    }
    if frame.typing_bursts.iter().any(|burst| burst.committed) {
        reasons.insert("committed_typing_boundary".into());
    }
    if frame.frame_diff.is_some() {
        reasons.insert("material_change_boundary".into());
    }
    if frame.transition.is_some() {
        reasons.insert("event_transition_boundary".into());
    }
    reasons.into_iter().collect()
}

fn keyframe_reference(
    frame: &EvidenceFrame,
    partition: EvidencePartitionV2,
    mut reasons: Vec<String>,
) -> KeyframeReferenceV2 {
    reasons.sort();
    reasons.dedup();
    let private = is_private_status(frame.privacy_status.as_deref());
    let (image_source_kind, image_scope, image_path, crop_pixels, image_rejection_reason) =
        resolve_visual_input(frame);
    let model_eligible = !private && image_path.is_some();
    KeyframeReferenceV2 {
        frame_id: frame.id.clone(),
        observed_at_ms: frame.captured_at,
        partition,
        surface_identity: ActiveSurfaceIdentityV2 {
            app_name: frame.app_name.clone(),
            app_bundle_id: frame.app_bundle_id.clone(),
            window_title_hash: hash_optional(frame.window_name.as_deref()),
            window_id: frame.window_id,
            browser_url_hash: hash_optional(frame.browser_url.as_deref()),
            document_path_hash: hash_optional(frame.document_path.as_deref()),
        },
        surface_ownership_confidence: if frame.window_id.is_some() {
            0.95
        } else if frame.app_bundle_id.is_some() || frame.app_name.is_some() {
            0.75
        } else {
            0.25
        },
        privacy_status: frame
            .privacy_status
            .clone()
            .unwrap_or_else(|| "unknown".into()),
        model_eligible,
        image_source_kind,
        image_scope,
        image_width: frame.pixel_width,
        image_height: frame.pixel_height,
        image_rejection_reason,
        crop_pixels,
        local_image_handle_hash: image_path
            .as_deref()
            .and_then(|path| hash_optional(Some(path))),
        ephemeral_local_image_path: image_path,
        selection_reasons: reasons,
        task_evidence_role: None,
        task_turn_id: None,
        same_task_relation: "unknown".into(),
        cross_pane_ambiguity: false,
        near_duplicate_group: near_duplicate_group(frame),
    }
}

fn near_duplicate_group(frame: &EvidenceFrame) -> Option<String> {
    if let Some(content_hash) = frame
        .content_hash
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return Some(format!(
            "stable_content:{}",
            stable_hash(format!("{}|{content_hash}", surface_visit_key(frame)).as_bytes())
        ));
    }
    let image_hash = frame
        .image_hash
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())?;
    Some(format!("image:{}", stable_hash(image_hash.as_bytes())))
}

fn task_role_for_frame(
    task_relevance: &TaskRelevanceEvidenceV1,
    frame_id: &str,
) -> Option<TaskEvidenceRoleV1> {
    task_relevance
        .spans
        .iter()
        .filter(|span| span.frame_id == frame_id)
        .map(|span| span.evidence_role)
        .max_by_key(|role| role.authority_rank())
}

fn task_pane_ambiguity(task_relevance: &TaskRelevanceEvidenceV1, frame_id: &str) -> bool {
    let task_panes = task_relevance
        .spans
        .iter()
        .filter(|span| span.frame_id == frame_id)
        .filter(|span| {
            matches!(
                span.evidence_role,
                TaskEvidenceRoleV1::LatestUserGoal
                    | TaskEvidenceRoleV1::CurrentUnsentDraft
                    | TaskEvidenceRoleV1::CurrentAgentState
                    | TaskEvidenceRoleV1::PriorTaskBoundary
            )
        })
        .map(|span| span.pane_id.as_str())
        .collect::<BTreeSet<_>>();
    task_panes.contains("pane-unknown") || task_panes.len() > 1
}

fn same_task_relation(
    task_relevance: &TaskRelevanceEvidenceV1,
    role: Option<TaskEvidenceRoleV1>,
) -> String {
    if task_relevance.current_task_turn.is_none() {
        return if role == Some(TaskEvidenceRoleV1::FlattenedFallback) {
            "flattened_fallback_unverified".into()
        } else {
            "unknown".into()
        };
    }
    match role.unwrap_or(TaskEvidenceRoleV1::Unknown) {
        TaskEvidenceRoleV1::LatestUserGoal
        | TaskEvidenceRoleV1::CurrentUnsentDraft
        | TaskEvidenceRoleV1::CurrentAgentState
        | TaskEvidenceRoleV1::CurrentTaskContext => "current_task_turn".into(),
        TaskEvidenceRoleV1::PriorTaskBoundary => "immediately_prior_task_boundary".into(),
        TaskEvidenceRoleV1::SupportingContext => "supporting_or_adjacent".into(),
        TaskEvidenceRoleV1::FlattenedFallback => "flattened_fallback_unverified".into(),
        TaskEvidenceRoleV1::Unknown => "unknown".into(),
    }
}

fn annotate_keyframe(
    frame: &EvidenceFrame,
    mut reference: KeyframeReferenceV2,
    task_relevance: &TaskRelevanceEvidenceV1,
) -> KeyframeReferenceV2 {
    let role = task_role_for_frame(task_relevance, &frame.id).or_else(|| {
        (task_relevance.source == "flattened_window_fallback")
            .then_some(TaskEvidenceRoleV1::FlattenedFallback)
    });
    if let Some(role) = role {
        reference
            .selection_reasons
            .push(role.selection_reason().into());
        reference.selection_reasons.sort();
        reference.selection_reasons.dedup();
    }
    reference.task_evidence_role = role;
    reference.task_turn_id = task_relevance
        .current_task_turn
        .as_ref()
        .map(|turn| turn.task_turn_id.clone());
    reference.same_task_relation = same_task_relation(task_relevance, role);
    reference.cross_pane_ambiguity = task_pane_ambiguity(task_relevance, &frame.id);
    reference
}

fn downgrade_fallback_keyframe(mut reference: KeyframeReferenceV2) -> KeyframeReferenceV2 {
    reference.task_evidence_role = Some(TaskEvidenceRoleV1::FlattenedFallback);
    reference.same_task_relation = "flattened_fallback_unverified".into();
    reference
        .selection_reasons
        .push("flattened_window_fallback_non_authoritative".into());
    reference.selection_reasons.sort();
    reference.selection_reasons.dedup();
    reference
}

fn selected_has_semantic_duplicate(
    selected: &[KeyframeReferenceV2],
    candidate: &KeyframeReferenceV2,
) -> bool {
    candidate
        .near_duplicate_group
        .as_ref()
        .is_some_and(|group| {
            selected.iter().any(|existing| {
                existing.near_duplicate_group.as_ref() == Some(group)
                    && existing.task_evidence_role == candidate.task_evidence_role
            })
        })
}

fn select_keyframes(
    frames: &[EvidenceFrame],
    partitions: &BTreeMap<String, EvidencePartitionV2>,
    task_relevance: &TaskRelevanceEvidenceV1,
) -> Vec<KeyframeReferenceV2> {
    let mut scored = frames
        .iter()
        .map(|frame| {
            let reasons = boundary_reasons(frame);
            let boundary_score = reasons.len() as i64;
            let semantic_change = i64::from(frame.frame_diff.is_some())
                + i64::from(frame.transition.is_some())
                + i64::from(frame.typing_bursts.iter().any(|burst| burst.committed));
            (frame, reasons, boundary_score * 10 + semantic_change * 4)
        })
        .collect::<Vec<_>>();
    scored.sort_by(|left, right| {
        right
            .2
            .cmp(&left.2)
            .then_with(|| right.0.captured_at.cmp(&left.0.captured_at))
            .then_with(|| left.0.id.cmp(&right.0.id))
    });
    let current_id = frames.last().map(|frame| frame.id.as_str());
    let mut selected = Vec::new();
    if let Some(current) = frames.last() {
        selected.push(annotate_keyframe(
            current,
            keyframe_reference(
                current,
                EvidencePartitionV2::Current,
                vec!["current_frame_factual_state".into()],
            ),
            task_relevance,
        ));
    }
    for role in [
        TaskEvidenceRoleV1::LatestUserGoal,
        TaskEvidenceRoleV1::CurrentUnsentDraft,
        TaskEvidenceRoleV1::CurrentAgentState,
        TaskEvidenceRoleV1::PriorTaskBoundary,
        TaskEvidenceRoleV1::CurrentTaskContext,
    ] {
        if selected.len() >= MAX_KEYFRAMES {
            break;
        }
        let Some(frame) = frames.iter().rev().find(|frame| {
            task_role_for_frame(task_relevance, &frame.id) == Some(role)
                && !selected.iter().any(|item| item.frame_id == frame.id)
        }) else {
            continue;
        };
        let reference = annotate_keyframe(
            frame,
            keyframe_reference(
                frame,
                partitions
                    .get(&frame.id)
                    .copied()
                    .unwrap_or(EvidencePartitionV2::Prior),
                vec![role.selection_reason().into()],
            ),
            task_relevance,
        );
        if !selected_has_semantic_duplicate(&selected, &reference) {
            selected.push(reference);
        }
    }
    let structured_task_evidence = task_relevance.source == "p6_role_region_task_turn";
    if selected.len() < MAX_KEYFRAMES {
        if let Some(support) = frames.iter().rev().find(|frame| {
            partitions.get(&frame.id) == Some(&EvidencePartitionV2::Support)
                && !selected.iter().any(|item| item.frame_id == frame.id)
                && (!structured_task_evidence
                    || task_role_for_frame(task_relevance, &frame.id).is_some())
        }) {
            let mut reference = annotate_keyframe(
                support,
                keyframe_reference(
                    support,
                    EvidencePartitionV2::Support,
                    vec!["reserved_recent_structured_support_surface".into()],
                ),
                task_relevance,
            );
            if !structured_task_evidence && reference.task_evidence_role.is_none() {
                reference = downgrade_fallback_keyframe(reference);
            }
            if !selected_has_semantic_duplicate(&selected, &reference) {
                selected.push(reference);
            }
        }
    }
    // Browser activity often contains a short detour with several pages on
    // the same app/window. Reserve the most recent different origin and the
    // first frame on the current origin after it. Otherwise four newer pages
    // from the detour can crowd the task-bearing page out of the packet before
    // request-time boundary selection has a chance to inspect it.
    if selected.len() < MAX_KEYFRAMES {
        if let (Some(current), Some(current_origin)) =
            (frames.last(), frames.last().and_then(browser_origin_key))
        {
            if let Some(context) = frames.iter().rev().find(|frame| {
                let explicit_detour_return = frames.iter().any(|earlier| {
                    earlier.captured_at < frame.captured_at
                        && browser_origin_key(earlier).as_deref() == Some(current_origin.as_str())
                }) && frames.iter().any(|later| {
                    later.captured_at > frame.captured_at
                        && browser_origin_key(later).as_deref() == Some(current_origin.as_str())
                });
                frame.id != current.id
                    && !is_private_status(frame.privacy_status.as_deref())
                    && !is_diagnostic_surface(frame)
                    && frame_has_visual(frame)
                    && browser_origin_key(frame).is_some_and(|origin| origin != current_origin)
                    && (task_role_for_frame(task_relevance, &frame.id).is_some()
                        || explicit_detour_return
                        || !structured_task_evidence)
            }) {
                if !selected.iter().any(|item| item.frame_id == context.id) {
                    let explicit_detour_return = frames.iter().any(|earlier| {
                        earlier.captured_at < context.captured_at
                            && browser_origin_key(earlier).as_deref()
                                == Some(current_origin.as_str())
                    }) && frames.iter().any(|later| {
                        later.captured_at > context.captured_at
                            && browser_origin_key(later).as_deref() == Some(current_origin.as_str())
                    });
                    let mut reference = annotate_keyframe(
                        context,
                        keyframe_reference(
                            context,
                            EvidencePartitionV2::Support,
                            vec![if explicit_detour_return {
                                "explicit_browser_detour_return_context".into()
                            } else {
                                "reserved_recent_distinct_browser_origin".into()
                            }],
                        ),
                        task_relevance,
                    );
                    if explicit_detour_return && reference.task_evidence_role.is_none() {
                        reference.task_evidence_role = Some(TaskEvidenceRoleV1::SupportingContext);
                        reference.same_task_relation = "explicit_detour_return_context".into();
                    } else if !structured_task_evidence && reference.task_evidence_role.is_none() {
                        reference = downgrade_fallback_keyframe(reference);
                    }
                    if !selected_has_semantic_duplicate(&selected, &reference) {
                        selected.push(reference);
                    }
                }
                if selected.len() < MAX_KEYFRAMES {
                    if let Some(entry) = frames.iter().find(|frame| {
                        frame.captured_at > context.captured_at
                            && frame.id != current.id
                            && browser_origin_key(frame).as_deref() == Some(current_origin.as_str())
                    }) {
                        if !selected.iter().any(|item| item.frame_id == entry.id) {
                            let mut reference = annotate_keyframe(
                                entry,
                                keyframe_reference(
                                    entry,
                                    partitions
                                        .get(&entry.id)
                                        .copied()
                                        .unwrap_or(EvidencePartitionV2::Prior),
                                    vec!["entry_to_current_browser_origin".into()],
                                ),
                                task_relevance,
                            );
                            if !structured_task_evidence && reference.task_evidence_role.is_none() {
                                reference = downgrade_fallback_keyframe(reference);
                            }
                            if !selected_has_semantic_duplicate(&selected, &reference) {
                                selected.push(reference);
                            }
                        }
                    }
                }
            }
        }
    }
    for (frame, mut reasons, _) in scored {
        if selected.len() >= MAX_KEYFRAMES {
            break;
        }
        if selected.iter().any(|item| item.frame_id == frame.id) {
            continue;
        }
        if structured_task_evidence && task_role_for_frame(task_relevance, &frame.id).is_none() {
            continue;
        }
        if reasons.is_empty() && Some(frame.id.as_str()) != current_id {
            continue;
        }
        if Some(frame.id.as_str()) == current_id {
            reasons.push("current_frame".into());
        }
        let reference = annotate_keyframe(
            frame,
            keyframe_reference(
                frame,
                partitions
                    .get(&frame.id)
                    .copied()
                    .unwrap_or(EvidencePartitionV2::Background),
                reasons,
            ),
            task_relevance,
        );
        if !selected_has_semantic_duplicate(&selected, &reference) {
            selected.push(reference);
        }
    }
    if selected.len() < 2 && frames.len() > 1 {
        if let Some(baseline) = frames.iter().rev().find(|frame| {
            !selected.iter().any(|item| item.frame_id == frame.id)
                && (!structured_task_evidence
                    || task_role_for_frame(task_relevance, &frame.id).is_some())
        }) {
            selected.push(annotate_keyframe(
                baseline,
                keyframe_reference(
                    baseline,
                    partitions
                        .get(&baseline.id)
                        .copied()
                        .unwrap_or(EvidencePartitionV2::Prior),
                    vec!["causal_baseline_for_current_observation".into()],
                ),
                task_relevance,
            ));
        }
    }
    selected.sort_by_key(|item| (item.observed_at_ms, item.frame_id.clone()));
    selected
}

fn safe_surface_label(value: Option<&str>) -> String {
    let normalized = value
        .unwrap_or("Unknown application")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let normalized = if normalized.trim().is_empty() {
        "Unknown application".to_string()
    } else {
        normalized
    };
    normalized.chars().take(80).collect()
}

fn surface_visit_key(frame: &EvidenceFrame) -> String {
    let app = frame
        .app_bundle_id
        .as_deref()
        .or(frame.app_name.as_deref())
        .unwrap_or("unknown")
        .trim()
        .to_ascii_lowercase();
    if is_private_status(frame.privacy_status.as_deref()) {
        return format!("private:{app}");
    }
    if let Some(hostname) = browser_origin_key(frame) {
        return format!("browser:{app}:{hostname}");
    }
    format!("app:{app}")
}

fn visit_is_transient_browser_chrome(frames: &[&EvidenceFrame]) -> bool {
    !frames.is_empty()
        && frames
            .iter()
            .all(|frame| browser_origin_key(frame).is_none())
        && frames.iter().all(|frame| {
            frame
                .window_name
                .as_deref()
                .unwrap_or("")
                .to_ascii_lowercase()
                .contains("new tab")
        })
        && frames.iter().all(|frame| {
            frame
                .ui_events
                .iter()
                .all(|event| !event_matches_frame_surface(frame, event))
        })
        && frames.iter().all(|frame| {
            frame
                .typing_bursts
                .iter()
                .all(|burst| !burst.committed || !typing_burst_matches_frame_surface(frame, burst))
        })
}

fn build_surface_timeline(
    frames: &[EvidenceFrame],
    current_frame_id: &str,
    cutoff_ms: i64,
    task_relevance: &TaskRelevanceEvidenceV1,
) -> Vec<SurfaceVisitV2> {
    let current_frame = frames.iter().find(|frame| frame.id == current_frame_id);
    let current_is_chat = current_frame.is_some_and(|frame| {
        !is_private_status(frame.privacy_status.as_deref())
            && frame
                .app_contexts
                .iter()
                .any(|context| context.object_type == "chat_conversation")
    });
    let current_visible_content = current_frame
        .into_iter()
        .flat_map(|frame| frame.content_units.iter())
        .filter(|unit| {
            let Some(frame) = current_frame else {
                return false;
            };
            let hint = format!(
                "{} {}",
                unit.unit_type,
                unit.semantic_role.as_deref().unwrap_or("")
            );
            let page_content = !matches!(
                role_for(&hint),
                RegionRoleV2::BrowserChrome | RegionRoleV2::Navigation | RegionRoleV2::Toolbar
            );
            let foreground_owned = foreground_ownership(
                frame,
                unit.ownership_kind.as_deref(),
                unit.owner_window_id,
                unit.owner_bundle_id.as_deref(),
                &unit.quality_flags,
            )
            .0;
            page_content && foreground_owned
        })
        .filter_map(|unit| unit.text.as_deref())
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase();
    let mut ordered = frames
        .iter()
        .filter(|frame| frame.captured_at <= cutoff_ms)
        .collect::<Vec<_>>();
    ordered.sort_by_key(|frame| (frame.captured_at, frame.id.clone()));

    let mut groups = Vec::<(String, Vec<&EvidenceFrame>)>::new();
    let mut hidden_separator = false;
    for frame in ordered {
        // A hidden self frame is not emitted, but it is a real departure. It
        // must break adjacency so A -> Smalltalk -> A remains two visits.
        if is_diagnostic_surface(frame) {
            hidden_separator = true;
            continue;
        }
        let key = surface_visit_key(frame);
        if !hidden_separator
            && groups
                .last()
                .is_some_and(|(existing_key, _)| existing_key == &key)
        {
            groups.last_mut().expect("group exists").1.push(frame);
        } else {
            groups.push((key, vec![frame]));
        }
        hidden_separator = false;
    }

    let mut seen_keys = BTreeSet::new();
    let mut visits = groups
        .into_iter()
        .filter_map(|(key, frames)| {
            let first = *frames.first()?;
            let last = *frames.last()?;
            let private = frames
                .iter()
                .any(|frame| is_private_status(frame.privacy_status.as_deref()));
            if !private && visit_is_transient_browser_chrome(&frames) {
                return None;
            }
            let revisited = !seen_keys.insert(key.clone());
            let mut event_ids = frames
                .iter()
                .flat_map(|frame| {
                    frame
                        .ui_events
                        .iter()
                        .filter(|event| event_matches_frame_surface(frame, event))
                        .map(|event| event.id.clone())
                })
                .collect::<BTreeSet<_>>();
            for frame in &frames {
                for burst in frame.typing_bursts.iter().filter(|burst| {
                    burst.committed && typing_burst_matches_frame_surface(frame, burst)
                }) {
                    event_ids.insert(burst.id.clone());
                }
            }
            let interaction_count = event_ids.len();
            let committed_input = frames.iter().any(|frame| {
                frame.typing_bursts.iter().any(|burst| {
                    burst.committed
                        && burst.ended_at_ms <= cutoff_ms
                        && typing_burst_matches_frame_surface(frame, burst)
                })
            });
            let dwell_ms = last.captured_at.saturating_sub(first.captured_at);
            let engagement_score = (interaction_count as i64 * 1_000)
                + (dwell_ms.clamp(0, 30 * 60 * 1_000) / 1_000 * 10)
                + (frames.len() as i64 * 50);
            let representative_candidates = frames
                .iter()
                .map(|frame| {
                    let reference = annotate_keyframe(
                        frame,
                        keyframe_reference(
                            frame,
                            if frame.id == current_frame_id {
                                EvidencePartitionV2::Current
                            } else {
                                EvidencePartitionV2::Support
                            },
                            vec!["session_surface_representative".into()],
                        ),
                        task_relevance,
                    );
                    let activity = frame
                        .ui_events
                        .iter()
                        .filter(|event| event_matches_frame_surface(frame, event))
                        .count()
                        + frame
                            .typing_bursts
                            .iter()
                            .filter(|burst| {
                                burst.committed && typing_burst_matches_frame_surface(frame, burst)
                            })
                            .count();
                    let task_rank = reference
                        .task_evidence_role
                        .map(TaskEvidenceRoleV1::authority_rank)
                        .unwrap_or(0);
                    (reference, task_rank, activity)
                })
                .collect::<Vec<_>>();
            let highest_task_rank = representative_candidates
                .iter()
                .filter(|(reference, _, _)| reference.model_eligible)
                .map(|(_, rank, _)| *rank)
                .max()
                .unwrap_or(0);
            let engagement_used_as_same_task_tiebreaker = representative_candidates
                .iter()
                .filter(|(reference, rank, _)| {
                    reference.model_eligible && *rank == highest_task_rank
                })
                .count()
                > 1;
            let representative_frame = (!private)
                .then(|| {
                    representative_candidates
                        .into_iter()
                        .max_by_key(|(reference, task_rank, activity)| {
                            (
                                reference.model_eligible,
                                *task_rank,
                                *activity,
                                reference.observed_at_ms,
                                reference.frame_id.clone(),
                            )
                        })
                        .map(|(reference, _, _)| reference)
                })
                .flatten();
            let app_label = if private {
                "Private activity".into()
            } else {
                safe_surface_label(last.app_name.as_deref().or(first.app_name.as_deref()))
            };
            let site_hostname = (!private)
                .then(|| {
                    frames
                        .iter()
                        .rev()
                        .find_map(|frame| browser_origin_key(frame))
                })
                .flatten();
            let hostname_mentioned_in_current_surface = !private
                && current_is_chat
                && !frames.iter().any(|frame| frame.id == current_frame_id)
                && site_hostname.as_deref().is_some_and(|hostname| {
                    current_visible_content.contains(&hostname.to_ascii_lowercase())
                });
            let task_evidence_role = frames
                .iter()
                .filter_map(|frame| task_role_for_frame(task_relevance, &frame.id))
                .max_by_key(|role| role.authority_rank());
            let task_relevance_score = task_evidence_role
                .map(TaskEvidenceRoleV1::authority_rank)
                .unwrap_or(0);
            let cross_pane_ambiguity = frames
                .iter()
                .any(|frame| task_pane_ambiguity(task_relevance, &frame.id));
            let mut evidence_refs = BTreeSet::new();
            evidence_refs.insert(first.id.clone());
            evidence_refs.insert(last.id.clone());
            if let Some(representative) = representative_frame.as_ref() {
                evidence_refs.insert(representative.frame_id.clone());
            }
            Some(SurfaceVisitV2 {
                sequence_index: 0,
                app_label,
                site_hostname,
                first_observed_at_ms: first.captured_at,
                last_observed_at_ms: last.captured_at,
                is_current: frames.iter().any(|frame| frame.id == current_frame_id),
                revisited,
                private,
                interaction_count,
                frame_count: frames.len(),
                engagement_score,
                committed_input,
                // Retained for old-row compatibility only. A hostname mention is
                // not proof that two surfaces belong to the same task turn.
                carried_into_current_surface: false,
                hostname_mentioned_in_current_surface,
                task_evidence_role,
                task_relevance_score,
                same_task_relation: same_task_relation(task_relevance, task_evidence_role),
                cross_pane_ambiguity,
                engagement_used_as_same_task_tiebreaker,
                evidence_refs: evidence_refs.into_iter().collect(),
                representative_frame,
            })
        })
        .collect::<Vec<_>>();
    if visits.len() > MAX_SURFACE_VISITS {
        visits.drain(..visits.len() - MAX_SURFACE_VISITS);
    }
    for (index, visit) in visits.iter_mut().enumerate() {
        visit.sequence_index = index + 1;
    }
    visits
}

fn rect_overlap(left: Option<&PacketBoundsV2>, right: Option<&PacketBoundsV2>) -> bool {
    let (Some(left), Some(right)) = (left, right) else {
        return false;
    };
    let x_overlap = left.x < right.x + right.width && right.x < left.x + left.width;
    let y_overlap = left.y < right.y + right.height && right.y < left.y + left.height;
    x_overlap && y_overlap
}

fn role_for(hint: &str) -> RegionRoleV2 {
    let hint = hint.to_ascii_lowercase();
    if hint.contains("browser_chrome")
        || hint.contains("address_bar")
        || hint.contains("tab_chrome")
    {
        RegionRoleV2::BrowserChrome
    } else if hint.contains("navigation") || hint.contains("menu") || hint.contains("tab") {
        RegionRoleV2::Navigation
    } else if hint.contains("toolbar") {
        RegionRoleV2::Toolbar
    } else if hint.contains("button") || hint.contains("control") || hint.contains("picker") {
        RegionRoleV2::Control
    } else if hint.contains("composer") || hint.contains("editor") || hint.contains("text_field") {
        RegionRoleV2::ComposerEditor
    } else if hint.contains("terminal_input") {
        RegionRoleV2::TerminalInput
    } else if hint.contains("terminal_output") || hint.contains("terminal") {
        RegionRoleV2::TerminalOutput
    } else if hint.contains("user") || hint.contains("authored") {
        RegionRoleV2::UserAuthoredContent
    } else if hint.contains("assistant") || hint.contains("agent") || hint.contains("output") {
        RegionRoleV2::ApplicationAgentOutput
    } else if hint.contains("sidebar") {
        RegionRoleV2::Sidebar
    } else if hint.contains("modal") || hint.contains("dialog") {
        RegionRoleV2::Modal
    } else if hint.contains("notification") {
        RegionRoleV2::Notification
    } else if hint.contains("status") {
        RegionRoleV2::Status
    } else if hint.contains("document") || hint.contains("canvas") {
        RegionRoleV2::DocumentCanvas
    } else if hint.contains("content") {
        RegionRoleV2::PrimaryContent
    } else {
        RegionRoleV2::Unknown
    }
}

fn control_role(role: RegionRoleV2) -> bool {
    matches!(
        role,
        RegionRoleV2::Navigation
            | RegionRoleV2::Toolbar
            | RegionRoleV2::Control
            | RegionRoleV2::Status
            | RegionRoleV2::Notification
            | RegionRoleV2::Sidebar
            | RegionRoleV2::BrowserChrome
    )
}

fn foreground_ownership(
    frame: &EvidenceFrame,
    ownership_kind: Option<&str>,
    owner_window_id: Option<i64>,
    owner_bundle_id: Option<&str>,
    quality_flags: &[String],
) -> (bool, Vec<String>) {
    let mut reasons = Vec::new();
    if matches!(
        ownership_kind,
        Some("OtherWindowOwned" | "SameAppNonActiveWindow")
    ) {
        reasons.push("not_current_foreground_owner".into());
    }
    if frame.window_id.is_some() && owner_window_id.is_some() && frame.window_id != owner_window_id
    {
        reasons.push("owner_window_mismatch".into());
    }
    if frame.app_bundle_id.is_some()
        && owner_bundle_id.is_some()
        && frame.app_bundle_id.as_deref() != owner_bundle_id
    {
        reasons.push("owner_app_mismatch".into());
    }
    if quality_flags.iter().any(|flag| flag.contains("stale")) {
        reasons.push("stale_source_node".into());
    }
    if quality_flags
        .iter()
        .any(|flag| flag.contains("coordinate_transform_unverified"))
    {
        reasons.push("unverified_coordinate_space".into());
    }
    (reasons.is_empty(), reasons)
}

fn element_from_unit(frame: &EvidenceFrame, unit: &EvidenceContentUnit) -> CanonicalElementV2 {
    let hint = format!(
        "{} {} {}",
        unit.unit_type,
        unit.semantic_role.as_deref().unwrap_or(""),
        unit.ownership_kind.as_deref().unwrap_or("")
    );
    let (foreground_owned, mut rejection_reasons) = foreground_ownership(
        frame,
        unit.ownership_kind.as_deref(),
        unit.owner_window_id,
        unit.owner_bundle_id.as_deref(),
        &unit.quality_flags,
    );
    // Ownership is established before the element is allowed to contribute a
    // semantic role to the current foreground surface.
    let region_role = role_for(&hint);
    let task_eligible =
        foreground_owned && !control_role(region_role) && !is_categorical_control_hint(&hint);
    if control_role(region_role) || is_categorical_control_hint(&hint) {
        rejection_reasons.push("categorical_control_ineligible".into());
    }
    let text_reference = unit
        .text_hash
        .clone()
        .or_else(|| hash_optional(unit.text.as_deref()));
    CanonicalElementV2 {
        element_id: format!("element:{}:{}", frame.id, unit.id),
        frame_id: frame.id.clone(),
        bounds: unit.bounds.map(Into::into),
        display_id: frame.display_id.clone(),
        window_id: unit.owner_window_id.or(frame.window_id),
        owning_app_bundle: unit
            .owner_bundle_id
            .clone()
            .or_else(|| frame.app_bundle_id.clone()),
        source_scope: unit.source_scope.clone(),
        ownership_kind: unit.ownership_kind.clone(),
        ownership_confidence: unit.ownership_confidence,
        coordinate_space: if unit.source_scope.as_deref() == Some("active_window") {
            "active_window_pixels".into()
        } else {
            "captured_surface_pixels".into()
        },
        freshness: if unit.quality_flags.iter().any(|flag| flag.contains("stale")) {
            "stale".into()
        } else {
            "current_frame".into()
        },
        text_reference,
        visual_description: None,
        native_role: Some(unit.unit_type.clone()),
        native_subrole: unit.semantic_role.clone(),
        native_actionability: control_role(region_role) || is_categorical_control_hint(&hint),
        region_role,
        focused: frame.focused_node_evidence
            && unit
                .semantic_role
                .as_deref()
                .is_some_and(|role| role.to_ascii_lowercase().contains("focused")),
        editable: matches!(
            region_role,
            RegionRoleV2::ComposerEditor | RegionRoleV2::TerminalInput
        ),
        selected: frame.selected_text_present,
        interactive: control_role(region_role)
            || is_categorical_control_hint(&hint)
            || matches!(
                region_role,
                RegionRoleV2::ComposerEditor | RegionRoleV2::TerminalInput
            ),
        parent_element_id: None,
        child_element_ids: Vec::new(),
        source_votes: vec![unit.source.clone()],
        source_conflicts: Vec::new(),
        first_seen_at_ms: frame.captured_at,
        changed_at_ms: frame.captured_at,
        authorship_status: match region_role {
            RegionRoleV2::UserAuthoredContent | RegionRoleV2::TerminalInput => {
                AuthorshipStatusV2::User
            }
            RegionRoleV2::ApplicationAgentOutput | RegionRoleV2::TerminalOutput => {
                AuthorshipStatusV2::ApplicationOrAgent
            }
            _ => AuthorshipStatusV2::Unknown,
        },
        causal_evidence_refs: Vec::new(),
        task_eligible,
        rejection_reasons,
    }
}

fn merge_ocr(element: &mut CanonicalElementV2, frame: &EvidenceFrame, ocr: &EvidenceOcrSpan) {
    if !element.source_votes.iter().any(|vote| vote == "ocr") {
        element.source_votes.push("ocr".into());
    }
    let ocr_hash = stable_hash(normalize_text(&ocr.text).as_bytes());
    if element
        .text_reference
        .as_deref()
        .is_some_and(|hash| hash != ocr_hash)
    {
        element
            .source_conflicts
            .push("ax_ocr_text_disagreement".into());
    }
    element.changed_at_ms = element.changed_at_ms.max(frame.captured_at);
}

fn canonical_elements(
    frames: &[EvidenceFrame],
    partitions: &BTreeMap<String, EvidencePartitionV2>,
) -> (
    Vec<CanonicalElementV2>,
    BTreeMap<String, FrameCapacityAccountingV2>,
) {
    let mut all_elements = Vec::new();
    for frame in frames {
        for unit in &frame.content_units {
            all_elements.push(element_from_unit(frame, unit));
        }
        for ocr in &frame.ocr_spans {
            let normalized = normalize_text(&ocr.text);
            let ocr_bounds = Some(PacketBoundsV2::from(ocr.bounds));
            let matching = all_elements.iter_mut().find(|element| {
                element.frame_id == frame.id
                    && (element.text_reference.as_deref()
                        == Some(stable_hash(normalized.as_bytes()).as_str())
                        || rect_overlap(element.bounds.as_ref(), ocr_bounds.as_ref()))
            });
            if let Some(element) = matching {
                merge_ocr(element, frame, ocr);
            } else {
                all_elements.push(CanonicalElementV2 {
                    element_id: format!("element:{}:ocr:{}", frame.id, ocr.id),
                    frame_id: frame.id.clone(),
                    bounds: ocr_bounds,
                    display_id: frame.display_id.clone(),
                    window_id: ocr.owner_window_id.or(frame.window_id),
                    owning_app_bundle: ocr
                        .owner_bundle_id
                        .clone()
                        .or_else(|| frame.app_bundle_id.clone()),
                    source_scope: ocr.source_scope.clone(),
                    ownership_kind: ocr.ownership_kind.clone(),
                    ownership_confidence: ocr.ownership_confidence,
                    coordinate_space: if ocr.source_scope.as_deref() == Some("active_window") {
                        "active_window_pixels".into()
                    } else {
                        "captured_surface_pixels".into()
                    },
                    freshness: if ocr.quality_flags.iter().any(|flag| flag.contains("stale")) {
                        "stale".into()
                    } else {
                        "current_frame".into()
                    },
                    text_reference: Some(stable_hash(normalized.as_bytes())),
                    visual_description: None,
                    native_role: Some("ocr_span".into()),
                    native_subrole: None,
                    native_actionability: false,
                    region_role: RegionRoleV2::Unknown,
                    focused: false,
                    editable: false,
                    selected: false,
                    interactive: false,
                    parent_element_id: None,
                    child_element_ids: Vec::new(),
                    source_votes: vec!["ocr".into()],
                    source_conflicts: Vec::new(),
                    first_seen_at_ms: frame.captured_at,
                    changed_at_ms: frame.captured_at,
                    authorship_status: AuthorshipStatusV2::Unknown,
                    causal_evidence_refs: Vec::new(),
                    task_eligible: foreground_ownership(
                        frame,
                        ocr.ownership_kind.as_deref(),
                        ocr.owner_window_id,
                        ocr.owner_bundle_id.as_deref(),
                        &ocr.quality_flags,
                    )
                    .0,
                    rejection_reasons: foreground_ownership(
                        frame,
                        ocr.ownership_kind.as_deref(),
                        ocr.owner_window_id,
                        ocr.owner_bundle_id.as_deref(),
                        &ocr.quality_flags,
                    )
                    .1,
                });
            }
        }
    }
    let mut accounting = frames
        .iter()
        .enumerate()
        .map(|(index, frame)| {
            (
                frame.id.clone(),
                FrameCapacityAccountingV2 {
                    frame_id: frame.id.clone(),
                    partition: partitions
                        .get(&frame.id)
                        .copied()
                        .unwrap_or(EvidencePartitionV2::Background),
                    age_rank: frames.len().saturating_sub(index + 1),
                    retained_elements: 0,
                    dropped_elements: 0,
                    retained_events: 0,
                    dropped_events: 0,
                    retained_by_source: BTreeMap::new(),
                    dropped_by_source: BTreeMap::new(),
                    retained_by_role: BTreeMap::new(),
                    dropped_by_role: BTreeMap::new(),
                },
            )
        })
        .collect::<BTreeMap<_, _>>();
    let quotas = [
        (EvidencePartitionV2::Current, 64usize),
        (EvidencePartitionV2::Prior, 40usize),
        (EvidencePartitionV2::Support, 32usize),
        (EvidencePartitionV2::Background, 24usize),
    ];
    let mut retained = Vec::new();
    let mut chrome_count = 0usize;
    for (partition, quota) in quotas {
        let mut used = 0usize;
        for frame in frames
            .iter()
            .rev()
            .filter(|frame| partitions.get(&frame.id) == Some(&partition))
        {
            let mut frame_elements = all_elements
                .iter()
                .filter(|element| element.frame_id == frame.id)
                .cloned()
                .collect::<Vec<_>>();
            frame_elements.sort_by(|left, right| {
                right
                    .focused
                    .cmp(&left.focused)
                    .then_with(|| right.interactive.cmp(&left.interactive))
                    .then_with(|| left.element_id.cmp(&right.element_id))
            });
            for element in frame_elements {
                let role = format!("{:?}", element.region_role).to_ascii_lowercase();
                let source = element
                    .source_votes
                    .first()
                    .cloned()
                    .unwrap_or_else(|| "unknown".into());
                let chrome_allowed = element.region_role != RegionRoleV2::BrowserChrome
                    || chrome_count < MAX_BROWSER_CHROME_ELEMENTS;
                let keep = used < quota && retained.len() < MAX_ELEMENTS && chrome_allowed;
                let entry = accounting
                    .get_mut(&frame.id)
                    .expect("frame accounting exists");
                if keep {
                    used += 1;
                    if element.region_role == RegionRoleV2::BrowserChrome {
                        chrome_count += 1;
                    }
                    entry.retained_elements += 1;
                    *entry.retained_by_source.entry(source).or_default() += 1;
                    *entry.retained_by_role.entry(role).or_default() += 1;
                    retained.push(element);
                } else {
                    entry.dropped_elements += 1;
                    *entry.dropped_by_source.entry(source).or_default() += 1;
                    *entry.dropped_by_role.entry(role).or_default() += 1;
                }
            }
        }
    }
    let frame_times = frames
        .iter()
        .map(|frame| (frame.id.as_str(), frame.captured_at))
        .collect::<BTreeMap<_, _>>();
    retained.sort_by(|left, right| {
        frame_times
            .get(left.frame_id.as_str())
            .cmp(&frame_times.get(right.frame_id.as_str()))
            .then_with(|| left.element_id.cmp(&right.element_id))
    });
    (retained, accounting)
}

fn causal_events(
    frames: &[EvidenceFrame],
    partitions: &BTreeMap<String, EvidencePartitionV2>,
    elements: &[CanonicalElementV2],
    accounting: &mut BTreeMap<String, FrameCapacityAccountingV2>,
) -> Vec<CausalEventV2> {
    let mut all_events = Vec::new();
    for frame in frames {
        let partition = partitions
            .get(&frame.id)
            .copied()
            .unwrap_or(EvidencePartitionV2::Background);
        for event in &frame.ui_events {
            if !event_matches_frame_surface(frame, event) {
                continue;
            }
            let kind = event.event_type.to_ascii_lowercase();
            let point_target = event.x.zip(event.y).and_then(|(x, y)| {
                elements
                    .iter()
                    .filter(|element| {
                        element.frame_id == frame.id
                            && event
                                .window_id
                                .zip(element.window_id)
                                .map(|(a, b)| a == b)
                                .unwrap_or(true)
                            && element.bounds.as_ref().is_some_and(|bounds| {
                                x >= bounds.x
                                    && y >= bounds.y
                                    && x <= bounds.x + bounds.width
                                    && y <= bounds.y + bounds.height
                            })
                    })
                    .min_by(|left, right| {
                        let left_area = left
                            .bounds
                            .as_ref()
                            .map(|b| b.width * b.height)
                            .unwrap_or(f64::MAX);
                        let right_area = right
                            .bounds
                            .as_ref()
                            .map(|b| b.width * b.height)
                            .unwrap_or(f64::MAX);
                        left_area
                            .partial_cmp(&right_area)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
            });
            // Scroll events are causal even when the event tap cannot provide a
            // point in the same coordinate space as the captured window. Prefer
            // an owned content element, but retain region-only grounding when a
            // sparse Accessibility tree has no page element to cite.
            let region_target = kind
                .contains("scroll")
                .then(|| {
                    elements
                        .iter()
                        .filter(|element| {
                            element.frame_id == frame.id
                                && element.rejection_reasons.is_empty()
                                && event
                                    .window_id
                                    .zip(element.window_id)
                                    .map(|(a, b)| a == b)
                                    .unwrap_or(true)
                        })
                        .filter(|element| {
                            matches!(
                                element.region_role,
                                RegionRoleV2::PrimaryContent
                                    | RegionRoleV2::DocumentCanvas
                                    | RegionRoleV2::UserAuthoredContent
                                    | RegionRoleV2::ApplicationAgentOutput
                                    | RegionRoleV2::TerminalInput
                                    | RegionRoleV2::TerminalOutput
                                    | RegionRoleV2::ComposerEditor
                                    | RegionRoleV2::Unknown
                            )
                        })
                        .max_by(|left, right| {
                            let left_area = left
                                .bounds
                                .as_ref()
                                .map(|b| b.width * b.height)
                                .unwrap_or(0.0);
                            let right_area = right
                                .bounds
                                .as_ref()
                                .map(|b| b.width * b.height)
                                .unwrap_or(0.0);
                            left_area
                                .partial_cmp(&right_area)
                                .unwrap_or(std::cmp::Ordering::Equal)
                        })
                })
                .flatten();
            let target = point_target.or(region_target);
            let region_only_scroll = kind.contains("scroll") && target.is_none();
            let focused = elements
                .iter()
                .find(|element| element.frame_id == frame.id && element.focused);
            let missing = if kind.contains("click") && target.is_none() {
                vec!["no_owned_element_at_event_coordinates".into()]
            } else if region_only_scroll {
                vec!["owned_scroll_region_without_element".into()]
            } else {
                Vec::new()
            };
            all_events.push(CausalEventV2 {
                event_id: event.id.clone(),
                event_kind: event.event_type.clone(),
                observed_at_ms: event.ts_ms.unwrap_or(frame.captured_at),
                frame_id: frame.id.clone(),
                source_frame_id: frame
                    .transition
                    .as_ref()
                    .and_then(|transition| transition.pre_frame_id.clone())
                    .unwrap_or_else(|| frame.id.clone()),
                target_frame_id: frame
                    .transition
                    .as_ref()
                    .and_then(|transition| transition.post_frame_id.clone()),
                target_element_id: target.map(|element| element.element_id.clone()),
                target_region: target
                    .map(|element| element.region_role)
                    .or(region_only_scroll.then_some(RegionRoleV2::PrimaryContent)),
                focused_element_before: focused.map(|element| element.element_id.clone()),
                focused_element_after: None,
                window_id: event.window_id.or(frame.window_id),
                app_bundle_id: event
                    .app_bundle_id
                    .clone()
                    .or_else(|| frame.app_bundle_id.clone()),
                pointer_x: event.x,
                pointer_y: event.y,
                scroll_delta_x: event.scroll_delta_x,
                scroll_delta_y: event.scroll_delta_y,
                pre_state_reference: frame
                    .transition
                    .as_ref()
                    .and_then(|transition| transition.pre_frame_id.clone()),
                post_state_reference: frame
                    .transition
                    .as_ref()
                    .and_then(|transition| transition.post_frame_id.clone()),
                semantic_delta_reference: Some(format!("delta:{}", frame.id)),
                grounding_confidence: if point_target.is_some() {
                    0.88
                } else if target.is_some() {
                    0.68
                } else if region_only_scroll {
                    0.55
                } else if event.window_id.is_some() {
                    0.55
                } else {
                    0.2
                },
                missing_evidence: missing,
                conflicting_evidence: Vec::new(),
                partition,
                causal_parent_ids: Vec::new(),
                committed: None,
                source: "ui_event".into(),
            });
        }
        for burst in &frame.typing_bursts {
            let same_surface = typing_burst_matches_frame_surface(frame, burst);
            let focused = same_surface
                .then(|| {
                    elements.iter().find(|element| {
                        element.frame_id == frame.id && element.focused && element.editable
                    })
                })
                .flatten();
            let surface_grounded_commit = focused.is_none()
                && burst.committed
                && same_surface
                && (burst.char_count > 0 || burst.enter_count > 0 || burst.paste_count > 0);
            all_events.push(CausalEventV2 {
                event_id: burst.id.clone(),
                event_kind: burst
                    .commit_signal
                    .clone()
                    .unwrap_or_else(|| "typing_burst".into()),
                observed_at_ms: burst.ended_at_ms.max(burst.started_at_ms),
                frame_id: frame.id.clone(),
                source_frame_id: frame
                    .transition
                    .as_ref()
                    .and_then(|transition| transition.pre_frame_id.clone())
                    .unwrap_or_else(|| frame.id.clone()),
                target_frame_id: frame
                    .transition
                    .as_ref()
                    .and_then(|transition| transition.post_frame_id.clone()),
                target_element_id: focused.map(|element| element.element_id.clone()),
                target_region: focused.map(|element| element.region_role),
                focused_element_before: focused.map(|element| element.element_id.clone()),
                focused_element_after: focused.map(|element| element.element_id.clone()),
                window_id: focused
                    .and_then(|element| element.window_id)
                    .or(frame.window_id),
                app_bundle_id: focused
                    .and_then(|element| element.owning_app_bundle.clone())
                    .or_else(|| frame.app_bundle_id.clone()),
                pointer_x: None,
                pointer_y: None,
                scroll_delta_x: None,
                scroll_delta_y: None,
                pre_state_reference: frame
                    .transition
                    .as_ref()
                    .and_then(|transition| transition.pre_frame_id.clone()),
                post_state_reference: frame
                    .transition
                    .as_ref()
                    .and_then(|transition| transition.post_frame_id.clone()),
                semantic_delta_reference: Some(format!("delta:{}", frame.id)),
                grounding_confidence: if focused.is_some() {
                    0.9
                } else if surface_grounded_commit {
                    0.68
                } else {
                    0.35
                },
                missing_evidence: if focused.is_some() {
                    Vec::new()
                } else if surface_grounded_commit {
                    vec![
                        "focused_editable_element_missing".into(),
                        "typing_grounded_to_exact_app_and_window_only".into(),
                    ]
                } else {
                    vec!["focused_editable_element_missing".into()]
                },
                conflicting_evidence: Vec::new(),
                partition,
                causal_parent_ids: frame
                    .trigger
                    .as_ref()
                    .map(|trigger| trigger.caused_by_event_ids.clone())
                    .unwrap_or_default(),
                committed: Some(burst.committed),
                source: "typing_burst".into(),
            });
        }
    }
    let quotas = [
        (EvidencePartitionV2::Current, 32usize),
        (EvidencePartitionV2::Prior, 32usize),
        (EvidencePartitionV2::Support, 16usize),
        (EvidencePartitionV2::Background, 16usize),
    ];
    let mut events = Vec::new();
    for (partition, quota) in quotas {
        let mut candidates = all_events
            .iter()
            .filter(|event| event.partition == partition)
            .cloned()
            .collect::<Vec<_>>();
        candidates.sort_by(|left, right| {
            causal_event_priority(left)
                .cmp(&causal_event_priority(right))
                .then_with(|| right.observed_at_ms.cmp(&left.observed_at_ms))
                .then_with(|| left.event_id.cmp(&right.event_id))
        });
        for (index, event) in candidates.into_iter().enumerate() {
            let entry = accounting
                .get_mut(&event.frame_id)
                .expect("event frame accounting exists");
            if index < quota && events.len() < MAX_CAUSAL_EVENTS {
                entry.retained_events += 1;
                *entry
                    .retained_by_source
                    .entry(event.source.clone())
                    .or_default() += 1;
                events.push(event);
            } else {
                entry.dropped_events += 1;
                *entry
                    .dropped_by_source
                    .entry(event.source.clone())
                    .or_default() += 1;
            }
        }
    }
    events.sort_by(|left, right| {
        left.observed_at_ms
            .cmp(&right.observed_at_ms)
            .then_with(|| left.event_id.cmp(&right.event_id))
    });
    events
}

fn causal_event_priority(event: &CausalEventV2) -> u8 {
    let kind = event.event_kind.to_ascii_lowercase();
    if [
        "click",
        "scroll",
        "submit",
        "navigation",
        "app_switch",
        "focus",
        "terminal_command",
    ]
    .iter()
    .any(|signal| kind.contains(signal))
    {
        0
    } else if event.source == "typing_burst" || kind == "key_down" {
        1
    } else {
        2
    }
}

fn changed_regions(raw: Option<&str>) -> Vec<PacketBoundsV2> {
    let Some(value) = raw.and_then(|raw| serde_json::from_str::<serde_json::Value>(raw).ok())
    else {
        return Vec::new();
    };
    let candidates = value.as_array().cloned().unwrap_or_else(|| vec![value]);
    candidates
        .into_iter()
        .filter_map(|item| {
            let object = item.as_object()?;
            Some(PacketBoundsV2 {
                x: object.get("x").or_else(|| object.get("left"))?.as_f64()?,
                y: object.get("y").or_else(|| object.get("top"))?.as_f64()?,
                width: object.get("width").or_else(|| object.get("w"))?.as_f64()?,
                height: object.get("height").or_else(|| object.get("h"))?.as_f64()?,
            })
        })
        .collect()
}

fn json_list_has_values(raw: Option<&str>) -> bool {
    let Some(raw) = raw.map(str::trim).filter(|value| !value.is_empty()) else {
        return false;
    };
    match serde_json::from_str::<serde_json::Value>(raw) {
        Ok(serde_json::Value::Array(values)) => !values.is_empty(),
        Ok(serde_json::Value::Null) => false,
        Ok(serde_json::Value::String(value)) => !value.trim().is_empty(),
        Ok(_) => true,
        Err(_) => raw != "[]",
    }
}

fn no_change_diff_kind(kind: Option<&str>) -> bool {
    kind.is_some_and(|kind| {
        let normalized = kind.trim().to_ascii_lowercase();
        normalized == "same_screen_idle"
            || normalized == "no_change"
            || normalized == "unchanged"
            || normalized == "same_screen"
    })
}

fn semantic_deltas(frames: &[EvidenceFrame], events: &[CausalEventV2]) -> Vec<FrameChangeV2> {
    let mut deltas = frames
        .iter()
        .filter_map(|frame| {
            let diff = frame.frame_diff.as_ref()?;
            let aligned_transition = frame.transition.as_ref().filter(|transition| {
                transition.pre_frame_id.as_deref() == diff.from_frame_id.as_deref()
                    && transition.post_frame_id.as_deref() == diff.to_frame_id.as_deref()
            });
            let mut observable_changes = Vec::new();
            let added_content = json_list_has_values(diff.added_text_hashes.as_deref());
            let removed_content = json_list_has_values(diff.removed_text_hashes.as_deref());
            let explicit_no_change = no_change_diff_kind(diff.diff_type.as_deref());
            if added_content && !explicit_no_change {
                observable_changes.push("content_appeared".into());
            }
            if removed_content && !explicit_no_change {
                observable_changes.push("content_disappeared".into());
            }
            if let Some(kind) = diff
                .diff_type
                .as_deref()
                .filter(|kind| !kind.trim().is_empty() && !no_change_diff_kind(Some(kind)))
            {
                observable_changes.push(format!("visual_or_semantic_change:{kind}"));
            }
            if let Some(kind) =
                aligned_transition.and_then(|transition| transition.transition_type.as_deref())
            {
                observable_changes.push(format!("transition:{kind}"));
            }
            observable_changes.sort();
            observable_changes.dedup();
            let mut causal_event_ids = events
                .iter()
                .filter(|event| {
                    event.target_frame_id.as_deref() == diff.to_frame_id.as_deref()
                        || event.frame_id == frame.id
                })
                .map(|event| event.event_id.clone())
                .collect::<Vec<_>>();
            causal_event_ids.sort();
            causal_event_ids.dedup();
            let source_conflicts = if explicit_no_change && (added_content || removed_content) {
                vec!["diff_kind_no_change_conflicts_with_text_hash_delta".into()]
            } else {
                Vec::new()
            };
            Some(FrameChangeV2 {
                delta_id: format!("delta:{}", frame.id),
                frame_id: frame.id.clone(),
                prior_frame_id: diff
                    .from_frame_id
                    .clone()
                    .or_else(|| frame.previous_frame_id.clone()),
                next_frame_id: diff.to_frame_id.clone().unwrap_or_else(|| frame.id.clone()),
                diff_kind: diff.diff_type.clone(),
                changed_regions: changed_regions(diff.changed_region_json.as_deref().or_else(
                    || {
                        aligned_transition
                            .and_then(|transition| transition.changed_region_json.as_deref())
                    },
                )),
                no_observable_change: observable_changes.is_empty(),
                observable_changes,
                source_agreement: [
                    frame.frame_diff.as_ref().map(|_| "frame_diff".to_string()),
                    aligned_transition.map(|_| "event_transition".to_string()),
                ]
                .into_iter()
                .flatten()
                .collect(),
                source_conflicts,
                causal_event_ids,
                summary_hash: hash_optional(diff.summary.as_deref()),
                added_text_hashes: diff.added_text_hashes.clone(),
                removed_text_hashes: diff.removed_text_hashes.clone(),
            })
        })
        .collect::<Vec<_>>();
    for frame in frames.iter().filter(|frame| {
        frame.frame_diff.is_none()
            && (!frame.ui_events.is_empty()
                || !frame.typing_bursts.is_empty()
                || frame.transition.is_some())
    }) {
        let changed = frame
            .transition
            .as_ref()
            .and_then(|transition| transition.changed_region_json.as_deref());
        let regions = changed_regions(changed);
        let observable_changes = frame
            .transition
            .as_ref()
            .and_then(|transition| transition.transition_type.as_deref())
            .map(|kind| vec![format!("transition:{kind}")])
            .unwrap_or_default();
        deltas.push(FrameChangeV2 {
            delta_id: format!("delta:{}", frame.id),
            frame_id: frame.id.clone(),
            prior_frame_id: frame
                .transition
                .as_ref()
                .and_then(|transition| transition.pre_frame_id.clone())
                .or_else(|| frame.previous_frame_id.clone()),
            next_frame_id: frame
                .transition
                .as_ref()
                .and_then(|transition| transition.post_frame_id.clone())
                .unwrap_or_else(|| frame.id.clone()),
            diff_kind: None,
            changed_regions: regions,
            no_observable_change: observable_changes.is_empty(),
            observable_changes,
            source_agreement: frame
                .transition
                .as_ref()
                .map(|_| vec!["event_transition".into()])
                .unwrap_or_default(),
            source_conflicts: Vec::new(),
            causal_event_ids: events
                .iter()
                .filter(|event| event.frame_id == frame.id)
                .map(|event| event.event_id.clone())
                .collect(),
            summary_hash: frame
                .transition
                .as_ref()
                .and_then(|transition| hash_optional(transition.summary.as_deref())),
            added_text_hashes: None,
            removed_text_hashes: None,
        });
    }
    deltas.sort_by(|left, right| left.next_frame_id.cmp(&right.next_frame_id));
    deltas
}

fn image_candidate_audits(
    frames: &[EvidenceFrame],
    semantic_keyframes: &[KeyframeReferenceV2],
    surface_timeline: &[SurfaceVisitV2],
    task_relevance: &TaskRelevanceEvidenceV1,
) -> Vec<ImageCandidateAuditV1> {
    let representatives = surface_timeline
        .iter()
        .filter_map(|visit| {
            visit
                .representative_frame
                .as_ref()
                .map(|frame| (frame.frame_id.as_str(), visit))
        })
        .collect::<BTreeMap<_, _>>();
    let selected_by_id = semantic_keyframes
        .iter()
        .map(|frame| (frame.frame_id.as_str(), frame))
        .chain(surface_timeline.iter().filter_map(|visit| {
            visit
                .representative_frame
                .as_ref()
                .map(|frame| (frame.frame_id.as_str(), frame))
        }))
        .collect::<BTreeMap<_, _>>();
    let selected_duplicate_facts = selected_by_id
        .values()
        .filter_map(|frame| {
            frame.near_duplicate_group.as_ref().map(|group| {
                (
                    group.clone(),
                    frame
                        .task_evidence_role
                        .unwrap_or(TaskEvidenceRoleV1::Unknown),
                )
            })
        })
        .collect::<BTreeSet<_>>();
    let task_turn_id = task_relevance
        .current_task_turn
        .as_ref()
        .map(|turn| turn.task_turn_id.clone());
    let mut audits = frames
        .iter()
        .map(|frame| {
            let reference = selected_by_id
                .get(frame.id.as_str())
                .copied()
                .cloned()
                .unwrap_or_else(|| {
                    annotate_keyframe(
                        frame,
                        keyframe_reference(frame, EvidencePartitionV2::Background, Vec::new()),
                        task_relevance,
                    )
                });
            let role = reference
                .task_evidence_role
                .unwrap_or(TaskEvidenceRoleV1::Unknown);
            let selected = selected_by_id.contains_key(frame.id.as_str());
            let mut rejection_reasons = Vec::new();
            if !selected {
                if is_private_status(frame.privacy_status.as_deref()) {
                    rejection_reasons.push("privacy_blocked_before_transport".into());
                } else if let Some(reason) = reference.image_rejection_reason.clone() {
                    rejection_reasons.push(reason);
                }
                if reference
                    .near_duplicate_group
                    .as_ref()
                    .is_some_and(|group| selected_duplicate_facts.contains(&(group.clone(), role)))
                {
                    rejection_reasons.push("near_duplicate_same_task_fact".into());
                }
                if rejection_reasons.is_empty() && role == TaskEvidenceRoleV1::Unknown {
                    rejection_reasons.push("not_proven_task_relevant".into());
                } else if rejection_reasons.is_empty() {
                    rejection_reasons.push("bounded_image_budget_omitted".into());
                }
            }
            let (prepared_width, prepared_height) = reference
                .crop_pixels
                .as_ref()
                .map(|crop| {
                    (
                        Some(crop.width.round() as i64),
                        Some(crop.height.round() as i64),
                    )
                })
                .unwrap_or((reference.image_width, reference.image_height));
            let crop_policy = match reference.image_source_kind.as_str() {
                "native_active_window" => "privacy_safe_native_active_window",
                "derived_active_window_crop" => "verified_owned_window_crop",
                "full_display" => "bounded_full_window_equivalent",
                _ => "no_prepared_image",
            }
            .to_string();
            let preparation_reason = if selected {
                if role.authority_rank() >= TaskEvidenceRoleV1::PriorTaskBoundary.authority_rank() {
                    "task_state_transition_evidence"
                } else if reference.partition == EvidencePartitionV2::Current {
                    "factual_current_state"
                } else {
                    "bounded_supporting_context"
                }
            } else {
                "not_prepared"
            }
            .to_string();
            ImageCandidateAuditV1 {
                frame_id: frame.id.clone(),
                task_turn_id: task_turn_id.clone(),
                evidence_role: role,
                selected,
                selection_reasons: reference.selection_reasons.clone(),
                rejection_reasons,
                supports_latest_user_goal: role == TaskEvidenceRoleV1::LatestUserGoal,
                supports_current_agent_state: role == TaskEvidenceRoleV1::CurrentAgentState,
                supports_prior_task_boundary: role == TaskEvidenceRoleV1::PriorTaskBoundary,
                same_task_relation: reference.same_task_relation.clone(),
                cross_pane_ambiguity: reference.cross_pane_ambiguity,
                near_duplicate_group: reference.near_duplicate_group.clone(),
                engagement_used_as_same_task_tiebreaker: representatives
                    .get(frame.id.as_str())
                    .is_some_and(|visit| visit.engagement_used_as_same_task_tiebreaker),
                original_width: frame.pixel_width,
                original_height: frame.pixel_height,
                prepared_width,
                prepared_height,
                image_scope: reference.image_scope.clone(),
                crop_policy,
                redaction_status: reference.privacy_status.clone(),
                preparation_reason,
            }
        })
        .collect::<Vec<_>>();
    audits.sort_by_key(|audit| audit.frame_id.clone());
    audits
}

pub(super) fn build_observation_packet(
    input_frames: &[EvidenceFrame],
    evidence_watermark: &str,
    previous_valid_snapshot_id: Option<String>,
) -> Result<ObservationPacketV2, String> {
    build_observation_packet_with_task_relevance(
        input_frames,
        evidence_watermark,
        previous_valid_snapshot_id,
        TaskRelevanceEvidenceV1::default(),
    )
}

pub(super) fn build_observation_packet_with_task_relevance(
    input_frames: &[EvidenceFrame],
    evidence_watermark: &str,
    previous_valid_snapshot_id: Option<String>,
    task_relevance: TaskRelevanceEvidenceV1,
) -> Result<ObservationPacketV2, String> {
    let Some(input_current) = input_frames.last() else {
        return Err("observation packet requires at least one evidence frame".into());
    };
    if input_frames
        .iter()
        .any(|frame| frame.session_id != input_current.session_id)
    {
        return Err("observation packet rejected mixed_session_evidence".into());
    }
    let non_diagnostic_frames = input_frames
        .iter()
        .filter(|frame| !is_diagnostic_surface(frame))
        .cloned()
        .collect::<Vec<_>>();
    let diagnostic_frame_count = input_frames
        .len()
        .saturating_sub(non_diagnostic_frames.len());
    let packet_frames = if non_diagnostic_frames.is_empty() {
        input_frames.to_vec()
    } else {
        non_diagnostic_frames
    };
    let timeline_current = packet_frames
        .last()
        .expect("non-diagnostic packet frames are non-empty");
    let surface_timeline = build_surface_timeline(
        input_frames,
        &timeline_current.id,
        timeline_current.captured_at,
        &task_relevance,
    );
    let dropped_frame_count = packet_frames.len().saturating_sub(24);
    let frames = &packet_frames[dropped_frame_count..];
    let current = frames.last().expect("bounded frame window is non-empty");
    let partitions_by_frame = partition_frames(frames);
    let semantic_keyframes = select_keyframes(frames, &partitions_by_frame, &task_relevance);
    let current_frame = semantic_keyframes
        .iter()
        .find(|keyframe| keyframe.frame_id == current.id)
        .cloned()
        .unwrap_or_else(|| {
            annotate_keyframe(
                current,
                keyframe_reference(
                    current,
                    EvidencePartitionV2::Current,
                    vec!["current_frame_factual_state".into()],
                ),
                &task_relevance,
            )
        });
    let image_candidates = image_candidate_audits(
        &packet_frames,
        &semantic_keyframes,
        &surface_timeline,
        &task_relevance,
    );
    let (canonical_elements, mut frame_accounting) =
        canonical_elements(frames, &partitions_by_frame);
    let causal_events = causal_events(
        frames,
        &partitions_by_frame,
        &canonical_elements,
        &mut frame_accounting,
    );
    let frame_changes = semantic_deltas(frames, &causal_events);
    let action_surface_ownership_mismatch_count = frames
        .iter()
        .flat_map(|frame| {
            frame
                .ui_events
                .iter()
                .filter(|event| !event_matches_frame_surface(frame, event))
        })
        .count();
    let capture_trigger_ids = frames
        .iter()
        .filter_map(|frame| frame.trigger.as_ref().map(|trigger| trigger.id.clone()))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let transition_ids = frames
        .iter()
        .filter_map(|frame| {
            frame
                .transition
                .as_ref()
                .map(|transition| transition.id.clone())
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let focused_element_ids = canonical_elements
        .iter()
        .filter(|element| element.focused)
        .map(|element| element.element_id.clone())
        .collect();
    let editable_element_ids = canonical_elements
        .iter()
        .filter(|element| element.editable)
        .map(|element| element.element_id.clone())
        .collect();
    let selected_element_ids = canonical_elements
        .iter()
        .filter(|element| element.selected)
        .map(|element| element.element_id.clone())
        .collect();
    let mut partitions = BTreeMap::<EvidencePartitionV2, Vec<String>>::new();
    for (frame_id, partition) in &partitions_by_frame {
        partitions
            .entry(*partition)
            .or_default()
            .push(frame_id.clone());
    }
    let private_visit_count = surface_timeline
        .iter()
        .filter(|visit| visit.private)
        .count();
    let ownership_rejected_count = surface_timeline
        .iter()
        .filter_map(|visit| visit.representative_frame.as_ref())
        .filter(|frame| {
            frame
                .image_rejection_reason
                .as_deref()
                .is_some_and(|reason| {
                    reason.contains("ownership") || reason.contains("coordinate_mapping")
                })
        })
        .count();
    let missing_image_count = surface_timeline
        .iter()
        .filter(|visit| !visit.private)
        .filter(|visit| {
            visit.representative_frame.as_ref().is_none_or(|frame| {
                frame.image_rejection_reason.as_deref() == Some("no_readable_image_asset")
            })
        })
        .count();
    let mut missing_source_notes = Vec::new();
    if current.content_units.is_empty() {
        missing_source_notes.push("current_frame_missing_content_units".into());
    }
    if current.ocr_spans.is_empty() {
        missing_source_notes.push("current_frame_missing_ocr".into());
    }
    if !current_frame.model_eligible {
        missing_source_notes.push(format!(
            "current_frame_missing_readable_visual:{}",
            current_frame
                .image_rejection_reason
                .as_deref()
                .unwrap_or("unknown")
        ));
    }
    if current.trigger.is_none() {
        missing_source_notes.push("current_frame_missing_capture_trigger".into());
    }
    if private_visit_count > 0 {
        missing_source_notes.push(format!(
            "surface_images_private_excluded:{private_visit_count}"
        ));
    }
    if ownership_rejected_count > 0 {
        missing_source_notes.push(format!(
            "surface_images_ownership_rejected:{ownership_rejected_count}"
        ));
    }
    if missing_image_count > 0 {
        missing_source_notes.push(format!("surface_images_missing_crop:{missing_image_count}"));
    }
    if dropped_frame_count > 0 {
        missing_source_notes.push(format!(
            "frames_dropped_by_evidence_window_cap:{dropped_frame_count}"
        ));
    }
    if diagnostic_frame_count > 0 && diagnostic_frame_count < input_frames.len() {
        missing_source_notes.push(format!(
            "diagnostic_self_frames_excluded:{diagnostic_frame_count}"
        ));
    }
    if action_surface_ownership_mismatch_count > 0 {
        missing_source_notes.push(format!(
            "action_surface_ownership_mismatch_excluded:{action_surface_ownership_mismatch_count}"
        ));
    }
    missing_source_notes.truncate(MAX_NOTES);
    let conflicting_observations = canonical_elements
        .iter()
        .flat_map(|element| element.source_conflicts.iter().cloned())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .take(MAX_NOTES)
        .collect::<Vec<_>>();
    let packet_seed = format!(
        "{}:{}:{}",
        current.session_id.as_deref().unwrap_or("no_session"),
        evidence_watermark,
        current.id
    );
    let mut packet = ObservationPacketV2 {
        schema: OBSERVATION_PACKET_SCHEMA_V2.into(),
        packet_id: format!("packet-{}", stable_hash(packet_seed.as_bytes())),
        observed_at_ms: current.captured_at,
        session_id: current.session_id.clone(),
        evidence_watermark: evidence_watermark.into(),
        active_surface: ActiveSurfaceIdentityV2 {
            app_name: current.app_name.clone(),
            app_bundle_id: current.app_bundle_id.clone(),
            window_title_hash: hash_optional(current.window_name.as_deref()),
            window_id: current.window_id,
            browser_url_hash: hash_optional(current.browser_url.as_deref()),
            document_path_hash: hash_optional(current.document_path.as_deref()),
        },
        current_frame,
        semantic_keyframes,
        surface_timeline,
        task_relevance,
        image_candidates,
        canonical_elements,
        focused_element_ids,
        editable_element_ids,
        selected_element_ids,
        causal_events,
        frame_changes,
        capture_trigger_ids,
        transition_ids,
        return_anchor_facts: [
            ("browser_url", current.browser_url.as_deref()),
            ("document_path", current.document_path.as_deref()),
        ]
        .into_iter()
        .filter_map(|(kind, value)| {
            hash_optional(value).map(|content_hash| EvidenceHandleV2 {
                source_kind: format!("return_anchor_fact:{kind}"),
                record_id: format!("{}:{kind}", current.id),
                frame_id: Some(current.id.clone()),
                content_hash: Some(content_hash),
            })
        })
        .collect(),
        previous_valid_snapshot_id,
        evidence_quality: if private_visit_count > 0 {
            "privacy_limited".into()
        } else if frames.len() >= 2 {
            "bounded_multisource".into()
        } else {
            "thin".into()
        },
        missing_source_notes,
        conflicting_observations,
        partitions,
        size: PacketSizeAccountingV2 {
            frame_count: frames.len(),
            keyframe_count: 0,
            canonical_element_count: 0,
            causal_event_count: 0,
            serialized_bytes: 0,
            estimated_tokens: 0,
            truncated: dropped_frame_count > 0,
            frame_accounting: frame_accounting.into_values().collect(),
        },
    };
    packet.size.keyframe_count = packet.semantic_keyframes.len();
    packet.size.canonical_element_count = packet.canonical_elements.len();
    packet.size.causal_event_count = packet.causal_events.len();
    packet.size.truncated = packet.size.truncated
        || packet
            .size
            .frame_accounting
            .iter()
            .any(|entry| entry.dropped_elements > 0 || entry.dropped_events > 0);
    for _ in 0..3 {
        let bytes = serde_json::to_vec(&packet).map_err(|error| error.to_string())?;
        packet.size.serialized_bytes = bytes.len();
        packet.size.estimated_tokens = bytes.len().div_ceil(4);
    }
    if packet.size.serialized_bytes > MAX_PACKET_BYTES {
        return Err(format!(
            "observation packet exceeded byte cap: {} > {}",
            packet.size.serialized_bytes, MAX_PACKET_BYTES
        ));
    }
    Ok(packet)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::continuation::{
        EvidenceFrameDiff, EvidenceOcrSpan, EvidenceTransition, EvidenceTrigger,
        EvidenceTypingBurst, EvidenceUiEvent, EvidenceWindow,
    };

    fn frame(id: &str, observed_at_ms: i64, trigger: &str) -> EvidenceFrame {
        EvidenceFrame {
            id: id.into(),
            captured_at: observed_at_ms,
            app_name: Some("Test App".into()),
            window_name: Some("Window".into()),
            browser_url: None,
            document_path: None,
            capture_trigger: trigger.into(),
            text_source: Some("accessibility".into()),
            scope: Some("active_window".into()),
            display_id: Some("main".into()),
            window_id: Some(1),
            screen_scale: Some(1.0),
            pixel_width: Some(1200),
            pixel_height: Some(800),
            full_screenshot_path: None,
            active_window_crop_path: Some("/private/local/short-lived.jpg".into()),
            full_text: None,
            active_text: None,
            background_text: None,
            full_text_quality: Some("structured".into()),
            text_quality_flags: Vec::new(),
            structured_semantic_text: None,
            content_hash: Some(format!("hash-{id}")),
            image_hash: Some(format!("image-{id}")),
            privacy_status: Some("allowed".into()),
            app_bundle_id: Some("com.example.test".into()),
            previous_frame_id: None,
            session_id: Some("session-test".into()),
            app_contexts: Vec::new(),
            content_units: Vec::new(),
            ocr_spans: Vec::new(),
            visible_windows: Vec::new(),
            ui_events: Vec::new(),
            trigger: None,
            transition: None,
            frame_diff: None,
            typing_bursts: Vec::new(),
            clipboard_events: Vec::new(),
            focused_node_evidence: false,
            selected_text_present: false,
        }
    }

    fn content_unit(id: &str, role: &str, text: &str, bounds: Rect) -> EvidenceContentUnit {
        EvidenceContentUnit {
            id: id.into(),
            source: "ax".into(),
            unit_type: role.into(),
            semantic_role: Some(role.into()),
            text: Some(text.into()),
            text_hash: Some(stable_hash(normalize_text(text).as_bytes())),
            confidence: Some(0.9),
            ocr_span_ids: Vec::new(),
            bounds: Some(bounds),
            source_scope: Some("active".into()),
            ownership_kind: None,
            ownership_confidence: Some(0.8),
            active_artifact_match_confidence: Some(0.8),
            owner_window_id: Some(1),
            owner_bundle_id: Some("com.example.test".into()),
            quality_flags: Vec::new(),
        }
    }

    fn task_span(
        id: &str,
        frame_id: &str,
        role: TaskEvidenceRoleV1,
        pane_id: &str,
    ) -> TaskRelevantSpanV1 {
        TaskRelevantSpanV1 {
            span_id: id.into(),
            frame_id: frame_id.into(),
            surface_key_hash: Some("surface-hash".into()),
            pane_id: pane_id.into(),
            region_role: match role {
                TaskEvidenceRoleV1::LatestUserGoal => "user_message",
                TaskEvidenceRoleV1::CurrentUnsentDraft => "composer",
                TaskEvidenceRoleV1::CurrentAgentState => "agent_status",
                TaskEvidenceRoleV1::PriorTaskBoundary => "conversation_history",
                _ => "unknown",
            }
            .into(),
            conversational_role: match role {
                TaskEvidenceRoleV1::LatestUserGoal => "user",
                TaskEvidenceRoleV1::CurrentAgentState | TaskEvidenceRoleV1::PriorTaskBoundary => {
                    "assistant_or_agent"
                }
                _ => "unknown",
            }
            .into(),
            evidence_role: role,
            reading_order: 1,
            source_scope: "active_window".into(),
            ownership_kind: "active_window_owned".into(),
            owner_window_id: Some(1),
            owner_app_id: Some("com.example.test".into()),
            ownership_confidence: 0.95,
            region_confidence: 0.95,
            speaker_confidence: 0.95,
            order_confidence: 0.95,
            privacy_status: "allowed".into(),
            text_hash: format!("hash-{id}"),
            focused: role == TaskEvidenceRoleV1::CurrentUnsentDraft,
            selected: true,
            submitted: (role == TaskEvidenceRoleV1::CurrentUnsentDraft).then_some(false),
            geometry: Some(PacketBoundsV2 {
                x: 10.0,
                y: 10.0,
                width: 600.0,
                height: 80.0,
            }),
            quality_flags: Vec::new(),
            reason_codes: Vec::new(),
            missing_evidence: Vec::new(),
        }
    }

    fn structured_task_relevance(spans: Vec<TaskRelevantSpanV1>) -> TaskRelevanceEvidenceV1 {
        TaskRelevanceEvidenceV1 {
            schema: TASK_RELEVANCE_EVIDENCE_SCHEMA_V1.into(),
            source: "p6_role_region_task_turn".into(),
            current_task_turn: Some(PacketTaskTurnV1 {
                task_turn_id: "turn-current".into(),
                revision: 2,
                execution_state: "active".into(),
                current_actor: "assistant_or_agent".into(),
                waiting_on: "agent".into(),
                relation_to_prior: "new_task".into(),
                goal_confidence: 0.94,
                actor_state_confidence: 0.9,
                relation_confidence: 0.88,
                attribution_confidence: 0.93,
                missing_evidence: Vec::new(),
                quality_flags: Vec::new(),
            }),
            latest_user_goal_sample: Some("Verify the completed visual cue".into()),
            current_agent_state_sample: Some("Implementation is complete".into()),
            prior_task_boundary_sample: Some("Earlier backend work completed".into()),
            current_unsent_draft_present: spans
                .iter()
                .any(|span| span.evidence_role == TaskEvidenceRoleV1::CurrentUnsentDraft),
            spans,
            missing_evidence: Vec::new(),
            fallback_flags: Vec::new(),
            confidence_cap: 0.9,
        }
    }

    #[test]
    fn same_watermark_produces_byte_stable_packet_semantics() {
        let mut current = frame("2", 2_000, "submit");
        current.typing_bursts.push(EvidenceTypingBurst {
            id: "burst-1".into(),
            started_at_ms: 1_900,
            ended_at_ms: 1_950,
            app_bundle_id: Some("com.example.test".into()),
            app_name: Some("Test App".into()),
            window_id: Some(1),
            window_title: Some("Window".into()),
            char_count: 8,
            enter_count: 1,
            paste_count: 0,
            committed: true,
            commit_signal: Some("enter".into()),
        });
        let frames = vec![frame("1", 1_000, "periodic"), current];
        let first = build_observation_packet(&frames, "watermark-1", None).unwrap();
        let second = build_observation_packet(&frames, "watermark-1", None).unwrap();
        assert_eq!(
            serde_json::to_vec(&first).unwrap(),
            serde_json::to_vec(&second).unwrap()
        );
        assert_eq!(first.packet_id, second.packet_id);
    }

    #[test]
    fn semantic_boundaries_beat_unchanged_recency_for_keyframes() {
        let mut submit = frame("submit", 1_000, "submit");
        submit.trigger = Some(EvidenceTrigger {
            id: "trigger-submit".into(),
            trigger_type: "submit".into(),
            caused_by_event_ids: vec!["event-submit".into()],
        });
        let mut switched = frame("switch", 2_000, "window_switch");
        switched.ui_events.push(EvidenceUiEvent {
            id: "event-switch".into(),
            ts_ms: Some(2_000),
            event_type: "window_switch".into(),
            key_category: None,
            x: None,
            y: None,
            window_id: Some(1),
            app_bundle_id: Some("com.example.test".into()),
            scroll_delta_x: None,
            scroll_delta_y: None,
            button: None,
        });
        let mut errored = frame("error", 3_000, "visible_error");
        errored.transition = Some(EvidenceTransition {
            id: "transition-error".into(),
            primary_event_id: None,
            transition_type: Some("error".into()),
            summary: None,
            confidence: Some(0.9),
            pre_frame_id: Some("switch".into()),
            post_frame_id: Some("error".into()),
            changed_region_json: None,
        });
        let unchanged = frame("unchanged", 3_500, "periodic");
        let current = frame("current", 4_000, "periodic");
        let packet = build_observation_packet(
            &[submit, switched, errored, unchanged, current],
            "watermark",
            None,
        )
        .unwrap();
        let ids = packet
            .semantic_keyframes
            .iter()
            .map(|keyframe| keyframe.frame_id.as_str())
            .collect::<Vec<_>>();
        assert!(ids.contains(&"submit"));
        assert!(ids.contains(&"switch"));
        assert!(ids.contains(&"error"));
        assert!(ids.contains(&"current"));
        assert!(!ids.contains(&"unchanged"));
    }

    #[test]
    fn session_surface_timeline_preserves_live_shaped_context_and_returns() {
        let mut chatgpt = frame("489", 1_000, "surface_change");
        chatgpt.app_name = Some("ChatGPT".into());
        chatgpt.app_bundle_id = Some("com.openai.chat".into());
        chatgpt.window_name = Some("Private conversation title".into());

        let browser = |id: &str, at: i64, url: &str, title: &str| {
            let mut value = frame(id, at, "surface_change");
            value.app_name = Some("Helium".into());
            value.app_bundle_id = Some("net.imput.helium".into());
            value.browser_url = Some(url.into());
            value.window_name = Some(title.into());
            value
        };
        let devfolio = browser(
            "490",
            2_000,
            "https://devfolio.co/application/private-path?draft=secret",
            "Sensitive application title",
        );
        let google_first = browser(
            "492",
            3_000,
            "https://www.google.com/search?q=private+query",
            "Private query - Google Search",
        );
        let mut google_adjacent = browser(
            "493",
            4_000,
            "https://google.com/search?q=another+private+query",
            "Another private query",
        );
        google_adjacent
            .ui_events
            .push(ui_event("google-scroll", "scroll", 3_900, None, None));
        let devfolio_return = browser(
            "497",
            5_000,
            "https://devfolio.co/application/return",
            "Sensitive application title",
        );
        let logs_list = browser(
            "498",
            6_000,
            "https://platform.openai.com/logs?project=secret",
            "Logs - OpenAI Platform",
        );
        let blank_response = browser(
            "499",
            7_000,
            "https://platform.openai.com/logs/resp_secret",
            "Logs - resp_secret",
        );

        let packet = build_observation_packet(
            &[
                chatgpt,
                devfolio,
                google_first,
                google_adjacent,
                devfolio_return,
                logs_list,
                blank_response,
            ],
            "watermark-live-shaped",
            None,
        )
        .unwrap();

        let chronology = packet
            .surface_timeline
            .iter()
            .map(|visit| {
                (
                    visit.app_label.as_str(),
                    visit.site_hostname.as_deref(),
                    visit.revisited,
                    visit.is_current,
                    visit.frame_count,
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(
            chronology,
            vec![
                ("ChatGPT", None, false, false, 1),
                ("Helium", Some("devfolio.co"), false, false, 1),
                ("Helium", Some("google.com"), false, false, 2),
                ("Helium", Some("devfolio.co"), true, false, 1),
                ("Helium", Some("platform.openai.com"), false, true, 2),
            ]
        );
        let serialized = serde_json::to_string(&packet).unwrap();
        assert!(!serialized.contains("private-path"));
        assert!(!serialized.contains("private+query"));
        assert!(!serialized.contains("Sensitive application title"));
        assert!(!serialized.contains("resp_secret"));
    }

    #[test]
    fn codex_is_a_work_surface_and_preserves_browser_departure_and_return() {
        let browser = |id: &str, at: i64| {
            let mut value = frame(id, at, "surface_change");
            value.app_name = Some("Helium".into());
            value.app_bundle_id = Some("net.imput.helium".into());
            value.browser_url = Some("https://developers.openai.com/prompt-caching".into());
            value
        };
        let first_docs = browser("docs-first", 1_000);
        let mut codex = frame("codex-work", 2_000, "typing");
        codex.app_name = Some("Codex".into());
        codex.app_bundle_id = Some("com.openai.codex".into());
        codex.window_id = Some(2);
        let final_docs = browser("docs-return", 3_000);

        let packet = build_observation_packet(
            &[first_docs, codex, final_docs],
            "watermark-codex-work",
            None,
        )
        .unwrap();
        assert_eq!(packet.surface_timeline.len(), 3);
        assert_eq!(
            packet.surface_timeline[0].site_hostname.as_deref(),
            Some("developers.openai.com")
        );
        assert_eq!(packet.surface_timeline[1].app_label, "Codex");
        assert!(packet.surface_timeline[1].representative_frame.is_some());
        assert_eq!(
            packet.surface_timeline[2].site_hostname.as_deref(),
            Some("developers.openai.com")
        );
        assert!(packet.surface_timeline[2].revisited);
        assert!(packet.surface_timeline[2].is_current);
    }

    #[test]
    fn surface_timeline_records_neutral_hostname_mention_without_claiming_continuity() {
        let mut source = frame("source", 1_000, "app_switch");
        source.app_name = Some("Helium".into());
        source.window_name = Some("Inkling - Helium".into());
        source.app_bundle_id = Some("net.imput.helium".into());
        source.browser_url = Some("https://thinkingmachines.ai/news/introducing-inkling/".into());

        let mut chat = frame("chat", 2_000, "manual");
        chat.app_name = Some("Helium".into());
        chat.window_name = Some("ChatGPT - smalltalk - Helium".into());
        chat.app_bundle_id = Some("net.imput.helium".into());
        chat.browser_url = Some("https://chatgpt.com/project/smalltalk".into());
        chat.app_contexts
            .push(super::super::super::EvidenceAppContext {
                id: "chat-context".into(),
                adapter_id: "ai_chat_url_adapter".into(),
                object_type: "chat_conversation".into(),
                primary_id: None,
                title: Some("ChatGPT - smalltalk".into()),
                url: chat.browser_url.clone(),
                file_path: None,
                repo_path: None,
                selected_text: None,
                focused_object: None,
                confidence: Some(0.9),
            });
        chat.content_units.push(EvidenceContentUnit {
            id: "carried-source".into(),
            source: "ax".into(),
            unit_type: "unknown".into(),
            semantic_role: Some("main_content".into()),
            text: Some("https://thinkingmachines.ai/news/introducing-inkling/".into()),
            text_hash: Some("safe-hash".into()),
            confidence: Some(0.9),
            ocr_span_ids: Vec::new(),
            bounds: None,
            source_scope: Some("active_window".into()),
            ownership_kind: Some("active_window".into()),
            ownership_confidence: Some(0.9),
            active_artifact_match_confidence: Some(0.9),
            owner_window_id: Some(1),
            owner_bundle_id: Some("net.imput.helium".into()),
            quality_flags: Vec::new(),
        });
        chat.typing_bursts.push(EvidenceTypingBurst {
            id: "committed-chat-input".into(),
            started_at_ms: 1_800,
            ended_at_ms: 1_900,
            app_bundle_id: Some("net.imput.helium".into()),
            app_name: Some("Helium".into()),
            window_id: Some(1),
            window_title: Some("ChatGPT - smalltalk - Helium".into()),
            char_count: 12,
            enter_count: 1,
            paste_count: 0,
            committed: true,
            commit_signal: Some("enter".into()),
        });

        let packet = build_observation_packet(&[source, chat], "watermark", None).unwrap();
        assert_eq!(packet.surface_timeline.len(), 2);
        assert!(!packet.surface_timeline[0].carried_into_current_surface);
        assert!(packet.surface_timeline[0].hostname_mentioned_in_current_surface);
        assert!(packet.surface_timeline[1].committed_input);
        let serialized = serde_json::to_string(&packet).unwrap();
        assert!(serialized.contains("thinkingmachines.ai"));
        assert!(!serialized.contains("typed_characters"));
    }

    #[test]
    fn committed_input_from_another_window_cannot_promote_the_current_visit() {
        let mut chat = frame("chat", 2_000, "manual");
        chat.app_name = Some("Helium".into());
        chat.window_name = Some("ChatGPT - smalltalk - Helium".into());
        chat.app_bundle_id = Some("net.imput.helium".into());
        chat.window_id = Some(17);
        chat.browser_url = Some("https://chatgpt.com/project/smalltalk".into());
        chat.typing_bursts.push(EvidenceTypingBurst {
            id: "other-window-commit".into(),
            started_at_ms: 1_800,
            ended_at_ms: 1_900,
            app_bundle_id: Some("com.openai.codex".into()),
            app_name: Some("Codex".into()),
            window_id: Some(99),
            window_title: Some("Different task".into()),
            char_count: 24,
            enter_count: 1,
            paste_count: 0,
            committed: true,
            commit_signal: Some("enter".into()),
        });

        let packet = build_observation_packet(&[chat], "watermark", None).unwrap();

        assert!(!packet.surface_timeline[0].committed_input);
        let event = packet
            .causal_events
            .iter()
            .find(|event| event.event_id == "other-window-commit")
            .unwrap();
        assert_eq!(event.grounding_confidence, 0.35);
        assert!(event.target_element_id.is_none());
    }

    #[test]
    fn hidden_smalltalk_frame_is_a_chronology_separator() {
        let browser = |id: &str, at: i64| {
            let mut value = frame(id, at, "surface_change");
            value.app_name = Some("Helium".into());
            value.app_bundle_id = Some("net.imput.helium".into());
            value.browser_url = Some("https://developers.openai.com/prompt-caching".into());
            value
        };
        let first_docs = browser("docs-first", 1_000);
        let mut smalltalk = frame("smalltalk-self", 2_000, "manual");
        smalltalk.app_name = Some("Smalltalk".into());
        smalltalk.app_bundle_id = Some("com.smalltalk.app".into());
        smalltalk.window_id = Some(2);
        let final_docs = browser("docs-return", 3_000);

        let packet = build_observation_packet(
            &[first_docs, smalltalk, final_docs],
            "watermark-self-separator",
            None,
        )
        .unwrap();
        assert_eq!(packet.surface_timeline.len(), 2);
        assert!(!packet.surface_timeline[0].revisited);
        assert!(packet.surface_timeline[1].revisited);
        assert!(packet.surface_timeline[1].is_current);
    }

    #[test]
    fn cross_application_event_is_not_attributed_to_the_post_frame_surface() {
        let mut helium = frame("helium-post", 2_000, "manual");
        helium.app_name = Some("Helium".into());
        helium.app_bundle_id = Some("net.imput.helium".into());
        helium.window_id = Some(3);
        let mut codex_scroll = ui_event("codex-scroll", "scroll", 1_900, None, None);
        codex_scroll.app_bundle_id = Some("com.openai.codex".into());
        codex_scroll.window_id = Some(2);
        helium.ui_events.push(codex_scroll);

        let packet = build_observation_packet(&[helium], "watermark-event-owner", None).unwrap();
        assert_eq!(packet.surface_timeline[0].interaction_count, 0);
        assert!(packet.causal_events.is_empty());
        assert!(packet
            .missing_source_notes
            .iter()
            .any(|note| { note == "action_surface_ownership_mismatch_excluded:1" }));
    }

    #[test]
    fn surface_timeline_caps_visits_and_redacts_private_activity_deterministically() {
        let mut frames = (0..10)
            .map(|index| {
                let mut value = frame(
                    &format!("frame-{index}"),
                    index as i64 * 1_000,
                    "surface_change",
                );
                value.app_name = Some("Helium".into());
                value.app_bundle_id = Some("net.imput.helium".into());
                value.browser_url = Some(format!(
                    "https://site-{index}.example/private/path?token=secret"
                ));
                value
            })
            .collect::<Vec<_>>();
        frames[8].privacy_status = Some("private".into());

        let first = build_observation_packet(&frames, "watermark-capped", None).unwrap();
        let second = build_observation_packet(&frames, "watermark-capped", None).unwrap();
        assert_eq!(first.surface_timeline, second.surface_timeline);
        assert_eq!(first.surface_timeline.len(), MAX_SURFACE_VISITS);
        assert_eq!(
            first.surface_timeline[0].site_hostname.as_deref(),
            Some("site-2.example")
        );
        let private = first
            .surface_timeline
            .iter()
            .find(|visit| visit.private)
            .expect("private visit survives as a generic local fact");
        assert_eq!(private.app_label, "Private activity");
        assert!(private.site_hostname.is_none());
        assert!(private.representative_frame.is_none());
    }

    #[test]
    fn recent_structured_cross_app_surface_keeps_a_reserved_support_keyframe() {
        let mut code = frame("code", 1_000, "periodic");
        code.app_name = Some("Code".into());
        code.app_bundle_id = Some("com.microsoft.VSCode".into());
        code.document_path = Some("/private/project/observation_packet.rs".into());
        code.focused_node_evidence = true;
        let mut diagnostic_frames = (2..=5)
            .map(|index| {
                let mut frame = frame(
                    &format!("diagnostic-{index}"),
                    index * 1_000,
                    "window_switch",
                );
                frame.app_name = Some("ChatGPT".into());
                frame.app_bundle_id = Some("com.openai.codex".into());
                frame.window_id = Some(2);
                frame
            })
            .collect::<Vec<_>>();
        let mut current = frame("browser", 6_000, "manual");
        current.app_name = Some("Helium".into());
        current.app_bundle_id = Some("net.imput.helium".into());
        current.window_id = Some(3);
        let mut self_frame = frame("smalltalk", 7_000, "manual");
        self_frame.app_name = Some("Smalltalk".into());
        self_frame.app_bundle_id = Some("com.smalltalk.app".into());
        self_frame.window_id = Some(4);
        let mut frames = vec![code];
        frames.append(&mut diagnostic_frames);
        frames.push(current);
        frames.push(self_frame);

        let packet = build_observation_packet(&frames, "watermark", None).unwrap();
        assert_eq!(packet.current_frame.frame_id, "browser");
        assert!(packet
            .missing_source_notes
            .iter()
            .any(|note| note.starts_with("diagnostic_self_frames_excluded:")));
        let code_keyframe = packet
            .semantic_keyframes
            .iter()
            .find(|keyframe| keyframe.frame_id == "code")
            .expect("the recent structured code surface should be retained");
        assert_eq!(code_keyframe.partition, EvidencePartitionV2::Support);
        assert!(code_keyframe
            .selection_reasons
            .contains(&"reserved_recent_structured_support_surface".into()));
    }

    #[test]
    fn smalltalk_owned_window_is_excluded_even_when_frame_claims_helium() {
        let mut browser = frame("verified-browser", 1_000, "surface_change");
        browser.app_name = Some("Helium".into());
        browser.app_bundle_id = Some("net.imput.helium".into());
        browser.browser_url = Some("https://chatgpt.com/c/example".into());
        browser.window_id = Some(42);

        let mut poisoned = frame("poisoned-manual", 2_000, "manual");
        poisoned.app_name = Some("Helium".into());
        poisoned.app_bundle_id = Some("net.imput.helium".into());
        poisoned.window_name = Some("ChatGPT - smalltalk - Helium".into());
        poisoned.window_id = Some(7);
        poisoned.visible_windows = vec![EvidenceWindow {
            id: "window-smalltalk".into(),
            cg_window_id: Some(7),
            owner_name: Some("smalltalk".into()),
            bundle_id: None,
            window_title: Some("smalltalk".into()),
            layer: Some(0),
            alpha: Some(1.0),
            is_onscreen: true,
            is_active: true,
            bounds: Rect {
                x: 0.0,
                y: 0.0,
                w: 800.0,
                h: 600.0,
            },
        }];

        let packet =
            build_observation_packet(&[browser, poisoned], "watermark-owner-conflict", None)
                .unwrap();

        assert_eq!(packet.current_frame.frame_id, "verified-browser");
        assert!(packet
            .surface_timeline
            .iter()
            .all(|visit| !visit.evidence_refs.contains(&"poisoned-manual".to_string())));
        assert!(packet
            .missing_source_notes
            .iter()
            .any(|note| note == "diagnostic_self_frames_excluded:1"));
    }

    #[test]
    fn browser_detour_pressure_keeps_prior_origin_and_current_origin_entry() {
        let browser_frame = |id: &str, observed_at_ms: i64, url: &str, title: &str| {
            let mut value = frame(id, observed_at_ms, "surface_change");
            value.app_name = Some("Helium".into());
            value.app_bundle_id = Some("net.imput.helium".into());
            value.browser_url = Some(url.into());
            value.window_name = Some(title.into());
            value
        };
        let devfolio = browser_frame(
            "devfolio",
            1_000,
            "https://devfolio.co/application",
            "Hackathon application",
        );
        let x_entry = browser_frame("x-entry", 2_000, "https://x.com/home", "Home / X");
        let mut x_post = browser_frame("x-post", 3_000, "https://x.com/post/1", "Post / X");
        x_post.frame_diff = Some(EvidenceFrameDiff {
            from_frame_id: Some("x-entry".into()),
            to_frame_id: Some("x-post".into()),
            diff_type: Some("navigated_surface".into()),
            changed_region_json: None,
            added_text_hashes: Some("[\"post\"]".into()),
            removed_text_hashes: None,
            summary: None,
        });
        let mut x_profile =
            browser_frame("x-profile", 4_000, "https://x.com/profile", "Profile / X");
        x_profile.frame_diff = Some(EvidenceFrameDiff {
            from_frame_id: Some("x-post".into()),
            to_frame_id: Some("x-profile".into()),
            diff_type: Some("navigated_surface".into()),
            changed_region_json: None,
            added_text_hashes: Some("[\"profile\"]".into()),
            removed_text_hashes: None,
            summary: None,
        });
        let mut x_reply = browser_frame("x-reply", 5_000, "https://x.com/reply", "Reply / X");
        x_reply.frame_diff = Some(EvidenceFrameDiff {
            from_frame_id: Some("x-profile".into()),
            to_frame_id: Some("x-reply".into()),
            diff_type: Some("navigated_surface".into()),
            changed_region_json: None,
            added_text_hashes: Some("[\"reply\"]".into()),
            removed_text_hashes: None,
            summary: None,
        });
        let current = browser_frame("current", 6_000, "https://x.com/home", "Home / X");

        let packet = build_observation_packet(
            &[devfolio, x_entry, x_post, x_profile, x_reply, current],
            "watermark",
            None,
        )
        .unwrap();
        let ids = packet
            .semantic_keyframes
            .iter()
            .map(|keyframe| keyframe.frame_id.as_str())
            .collect::<BTreeSet<_>>();

        assert_eq!(packet.semantic_keyframes.len(), MAX_KEYFRAMES);
        assert!(ids.contains("current"));
        assert!(ids.contains("devfolio"));
        assert!(ids.contains("x-entry"));
        let context = packet
            .semantic_keyframes
            .iter()
            .find(|keyframe| keyframe.frame_id == "devfolio")
            .unwrap();
        assert_eq!(context.partition, EvidencePartitionV2::Support);
        assert!(context
            .selection_reasons
            .contains(&"reserved_recent_distinct_browser_origin".into()));
    }

    #[test]
    fn private_frames_never_become_model_eligible() {
        let mut private = frame("private", 1_000, "manual");
        private.privacy_status = Some("private".into());
        let packet = build_observation_packet(&[private], "watermark", None).unwrap();
        assert!(!packet.current_frame.model_eligible);
        assert!(packet.current_frame.local_image_handle_hash.is_none());
        assert!(packet
            .semantic_keyframes
            .iter()
            .all(|keyframe| !keyframe.model_eligible));
    }

    #[test]
    fn normal_browser_frame_remains_visible_and_model_eligible() {
        let image_path = std::env::temp_dir().join(format!(
            "smalltalk-browser-evidence-{}.jpg",
            std::process::id()
        ));
        std::fs::write(&image_path, b"browser-image").unwrap();
        let mut browser = frame("browser-current", 1_000, "manual");
        browser.app_name = Some("Helium".into());
        browser.app_bundle_id = Some("net.imput.helium".into());
        browser.window_name = Some("Home / X - Helium".into());
        browser.browser_url = Some("https://x.com/home".into());
        browser.active_window_crop_path = Some(image_path.to_string_lossy().to_string());
        browser.privacy_status = Some("normal".into());
        browser.content_units.push(content_unit(
            "page-body",
            "main_content",
            "Browser work remains visible",
            Rect {
                x: 100.0,
                y: 120.0,
                w: 600.0,
                h: 80.0,
            },
        ));

        let packet = build_observation_packet(&[browser], "browser-watermark", None).unwrap();

        assert!(packet.current_frame.model_eligible);
        assert_eq!(
            packet.current_frame.ephemeral_local_image_path.as_deref(),
            Some(image_path.to_string_lossy().as_ref())
        );
        assert!(packet.semantic_keyframes.iter().any(|keyframe| {
            keyframe.frame_id == "browser-current"
                && keyframe.model_eligible
                && keyframe.ephemeral_local_image_path.is_some()
        }));
        assert_eq!(packet.active_surface.app_name.as_deref(), Some("Helium"));
        assert_eq!(packet.surface_timeline.len(), 1);
        assert_eq!(packet.surface_timeline[0].app_label, "Helium");
        assert_eq!(
            packet.surface_timeline[0].site_hostname.as_deref(),
            Some("x.com")
        );
        assert!(!packet.surface_timeline[0].private);
        let _ = std::fs::remove_file(image_path);
    }

    #[test]
    fn ax_ocr_duplicates_merge_without_losing_conflict_or_provenance() {
        let bounds = Rect {
            x: 10.0,
            y: 10.0,
            w: 200.0,
            h: 40.0,
        };
        let mut current = frame("1", 1_000, "material_change");
        current
            .content_units
            .push(content_unit("ax-1", "content", "Run tests", bounds));
        current.ocr_spans.push(EvidenceOcrSpan {
            id: "ocr-1".into(),
            text: "Run all tests".into(),
            confidence: Some(0.9),
            bounds,
            source_scope: Some("active".into()),
            ownership_kind: None,
            ownership_confidence: Some(0.8),
            active_artifact_match_confidence: Some(0.8),
            owner_window_id: Some(1),
            owner_bundle_id: Some("com.example.test".into()),
            owner_app_name: Some("Test App".into()),
            owner_window_title: Some("Window".into()),
            quality_flags: Vec::new(),
        });
        current.frame_diff = Some(EvidenceFrameDiff {
            from_frame_id: None,
            to_frame_id: Some("1".into()),
            diff_type: Some("text_change".into()),
            changed_region_json: None,
            added_text_hashes: None,
            removed_text_hashes: None,
            summary: Some("changed".into()),
        });
        let packet = build_observation_packet(&[current], "watermark", None).unwrap();
        assert_eq!(packet.canonical_elements.len(), 1);
        let element = &packet.canonical_elements[0];
        assert_eq!(element.source_votes, vec!["ax", "ocr"]);
        assert!(element
            .source_conflicts
            .contains(&"ax_ocr_text_disagreement".into()));
    }

    #[test]
    fn controls_are_categorically_task_ineligible() {
        let mut current = frame("1", 1_000, "manual");
        current.content_units.push(content_unit(
            "button-1",
            "button",
            "Approve for me",
            Rect {
                x: 0.0,
                y: 0.0,
                w: 100.0,
                h: 30.0,
            },
        ));
        let packet = build_observation_packet(&[current], "watermark", None).unwrap();
        let control = &packet.canonical_elements[0];
        assert_eq!(control.region_role, RegionRoleV2::Control);
        assert!(!control.task_eligible);
        assert_eq!(
            control.rejection_reasons,
            vec!["categorical_control_ineligible"]
        );
    }

    fn ui_event(id: &str, kind: &str, ts: i64, x: Option<f64>, y: Option<f64>) -> EvidenceUiEvent {
        EvidenceUiEvent {
            id: id.into(),
            ts_ms: Some(ts),
            event_type: kind.into(),
            key_category: None,
            x,
            y,
            window_id: Some(1),
            app_bundle_id: Some("com.example.test".into()),
            scroll_delta_x: None,
            scroll_delta_y: None,
            button: None,
        }
    }

    #[test]
    fn current_frame_capacity_survives_an_oversized_old_frame() {
        let bounds = Rect {
            x: 0.0,
            y: 0.0,
            w: 100.0,
            h: 20.0,
        };
        let mut old = frame("old", 1_000, "periodic");
        for index in 0..200 {
            old.content_units.push(content_unit(
                &format!("old-{index}"),
                "content",
                "old",
                bounds,
            ));
        }
        let mut current = frame("current", 2_000, "manual");
        current.content_units.push(content_unit(
            "current-element",
            "content",
            "current",
            bounds,
        ));
        let packet = build_observation_packet(&[old, current], "watermark", None).unwrap();
        assert!(packet
            .canonical_elements
            .iter()
            .any(|element| element.frame_id == "current"));
        let current_audit = packet
            .size
            .frame_accounting
            .iter()
            .find(|entry| entry.frame_id == "current")
            .unwrap();
        assert_eq!(current_audit.retained_elements, 1);
        assert!(
            packet
                .size
                .frame_accounting
                .iter()
                .find(|entry| entry.frame_id == "old")
                .unwrap()
                .dropped_elements
                > 0
        );
    }

    #[test]
    fn current_causal_events_survive_old_event_pressure() {
        let mut old = frame("old", 1_000, "periodic");
        for index in 0..140 {
            old.ui_events.push(ui_event(
                &format!("old-event-{index}"),
                "ax_notification",
                1_000 + index,
                None,
                None,
            ));
        }
        let mut current = frame("current", 2_000, "manual");
        current.ui_events.push(ui_event(
            "current-click",
            "click",
            2_000,
            Some(10.0),
            Some(10.0),
        ));
        let packet = build_observation_packet(&[old, current], "watermark", None).unwrap();
        assert!(packet
            .causal_events
            .iter()
            .any(|event| event.event_id == "current-click"));
        assert!(
            packet
                .size
                .frame_accounting
                .iter()
                .find(|entry| entry.frame_id == "old")
                .unwrap()
                .dropped_events
                > 0
        );
    }

    #[test]
    fn prior_scroll_survives_newer_accessibility_notification_pressure() {
        let mut scrolled = frame("scrolled", 1_000, "periodic");
        let mut scroll = ui_event("prior-scroll", "scroll", 1_100, Some(20.0), Some(20.0));
        scroll.scroll_delta_y = Some(720.0);
        scrolled.ui_events.push(scroll);
        let mut noisy = frame("noisy", 2_000, "periodic");
        for index in 0..100 {
            noisy.ui_events.push(ui_event(
                &format!("notification-{index}"),
                "ax_notification",
                2_000 + index,
                None,
                None,
            ));
        }
        let current = frame("current", 3_000, "manual");
        let packet =
            build_observation_packet(&[scrolled, noisy, current], "watermark", None).unwrap();
        assert!(packet
            .causal_events
            .iter()
            .any(|event| event.event_id == "prior-scroll"));
    }

    #[test]
    fn missing_native_crop_derives_safe_crop_from_verified_bounds() {
        let path = std::env::temp_dir().join(format!("mfti-full-{}.jpg", std::process::id()));
        std::fs::write(&path, [0xff, 0xd8, 0xff, 0xd9]).unwrap();
        let mut current = frame("current", 1_000, "manual");
        current.active_window_crop_path = None;
        current.full_screenshot_path = Some(path.to_string_lossy().into_owned());
        current.visible_windows.push(EvidenceWindow {
            id: "window".into(),
            cg_window_id: Some(1),
            owner_name: Some("Test App".into()),
            bundle_id: Some("com.example.test".into()),
            window_title: Some("Window".into()),
            layer: Some(0),
            alpha: Some(1.0),
            is_onscreen: true,
            is_active: true,
            bounds: Rect {
                x: 10.0,
                y: 20.0,
                w: 600.0,
                h: 400.0,
            },
        });
        let packet = build_observation_packet(&[current], "watermark", None).unwrap();
        assert_eq!(
            packet.current_frame.image_source_kind,
            "derived_active_window_crop"
        );
        assert!(packet.current_frame.crop_pixels.is_some());
        std::fs::remove_file(path).ok();
    }

    #[test]
    fn uncertain_crop_has_typed_missing_image_reason() {
        let path = std::env::temp_dir().join(format!("mfti-unsafe-{}.jpg", std::process::id()));
        std::fs::write(&path, [0xff, 0xd8, 0xff, 0xd9]).unwrap();
        let mut current = frame("current", 1_000, "manual");
        current.active_window_crop_path = None;
        current.full_screenshot_path = Some(path.to_string_lossy().into_owned());
        current.scope = Some("active_display".into());
        let packet = build_observation_packet(&[current], "watermark", None).unwrap();
        assert!(!packet.current_frame.model_eligible);
        assert_eq!(
            packet.current_frame.image_rejection_reason.as_deref(),
            Some("full_display_ownership_not_permitted")
        );
        std::fs::remove_file(path).ok();
    }

    #[test]
    fn other_window_ocr_is_diagnostic_not_foreground_meaning() {
        let mut current = frame("current", 1_000, "manual");
        current.ocr_spans.push(EvidenceOcrSpan {
            id: "other".into(),
            text: "Tab search - pinned".into(),
            confidence: Some(0.9),
            bounds: Rect {
                x: 0.0,
                y: 0.0,
                w: 100.0,
                h: 20.0,
            },
            source_scope: Some("other_visible_window".into()),
            ownership_kind: Some("OtherWindowOwned".into()),
            ownership_confidence: Some(0.9),
            active_artifact_match_confidence: Some(0.0),
            owner_window_id: Some(99),
            owner_bundle_id: Some("other.app".into()),
            owner_app_name: Some("Other".into()),
            owner_window_title: Some("Other".into()),
            quality_flags: Vec::new(),
        });
        let packet = build_observation_packet(&[current], "watermark", None).unwrap();
        let element = packet
            .canonical_elements
            .iter()
            .find(|element| element.element_id.contains("other"))
            .unwrap();
        assert!(!element.task_eligible);
        assert!(element
            .rejection_reasons
            .contains(&"not_current_foreground_owner".into()));
    }

    #[test]
    fn browser_chrome_is_separate_and_cannot_consume_element_budget() {
        let bounds = Rect {
            x: 0.0,
            y: 0.0,
            w: 100.0,
            h: 20.0,
        };
        let mut current = frame("current", 1_000, "manual");
        for index in 0..100 {
            current.content_units.push(content_unit(
                &format!("chrome-{index}"),
                "browser_chrome",
                "tab",
                bounds,
            ));
        }
        current
            .content_units
            .push(content_unit("page-content", "content", "page", bounds));
        let packet = build_observation_packet(&[current], "watermark", None).unwrap();
        assert!(packet
            .canonical_elements
            .iter()
            .any(|element| element.element_id.contains("page-content")));
        assert!(
            packet
                .canonical_elements
                .iter()
                .filter(|element| element.region_role == RegionRoleV2::BrowserChrome)
                .count()
                <= MAX_BROWSER_CHROME_ELEMENTS
        );
    }

    #[test]
    fn click_and_scroll_ground_to_owned_element_and_region() {
        let bounds = Rect {
            x: 0.0,
            y: 0.0,
            w: 300.0,
            h: 300.0,
        };
        let mut current = frame("current", 1_000, "manual");
        current
            .content_units
            .push(content_unit("page", "content", "page", bounds));
        current
            .ui_events
            .push(ui_event("click", "click", 1_000, Some(20.0), Some(20.0)));
        let mut scroll = ui_event("scroll", "scroll", 1_001, Some(30.0), Some(30.0));
        scroll.scroll_delta_y = Some(120.0);
        current.ui_events.push(scroll);
        let packet = build_observation_packet(&[current], "watermark", None).unwrap();
        for id in ["click", "scroll"] {
            let event = packet
                .causal_events
                .iter()
                .find(|event| event.event_id == id)
                .unwrap();
            assert!(event
                .target_element_id
                .as_deref()
                .is_some_and(|target| target.contains("page")));
            assert_eq!(event.target_region, Some(RegionRoleV2::PrimaryContent));
        }
    }

    #[test]
    fn scroll_without_pointer_coordinates_keeps_owned_content_region_grounding() {
        let mut current = frame("current", 1_000, "manual");
        current.content_units.push(content_unit(
            "page",
            "content",
            "page",
            Rect {
                x: 0.0,
                y: 100.0,
                w: 900.0,
                h: 700.0,
            },
        ));
        let mut scroll = ui_event("scroll", "scroll", 1_001, None, None);
        scroll.scroll_delta_y = Some(120.0);
        current.ui_events.push(scroll);

        let packet = build_observation_packet(&[current], "watermark", None).unwrap();
        let event = packet
            .causal_events
            .iter()
            .find(|event| event.event_id == "scroll")
            .unwrap();
        assert_eq!(event.target_region, Some(RegionRoleV2::PrimaryContent));
        assert!(event.target_element_id.is_some());
        assert!(event.missing_evidence.is_empty());
        assert!(event.grounding_confidence >= 0.68);
    }

    #[test]
    fn no_change_diff_never_claims_content_appeared() {
        let mut current = frame("current", 1_000, "manual");
        current.frame_diff = Some(EvidenceFrameDiff {
            from_frame_id: Some("prior".into()),
            to_frame_id: Some("current".into()),
            diff_type: Some("same_screen_idle".into()),
            changed_region_json: None,
            added_text_hashes: Some("[\"stale-hash\"]".into()),
            removed_text_hashes: None,
            summary: None,
        });

        let packet = build_observation_packet(&[current], "watermark", None).unwrap();
        let delta = packet.frame_changes.first().unwrap();
        assert!(delta.no_observable_change);
        assert!(!delta
            .observable_changes
            .iter()
            .any(|change| change == "content_appeared"));
        assert!(delta
            .source_conflicts
            .iter()
            .any(|reason| reason == "diff_kind_no_change_conflicts_with_text_hash_delta"));
    }

    #[test]
    fn outgoing_transition_cannot_contaminate_the_incoming_frame_delta() {
        let prior = frame("prior", 1_000, "periodic");
        let mut current = frame("current", 2_000, "manual");
        current.previous_frame_id = Some("prior".into());
        current.frame_diff = Some(EvidenceFrameDiff {
            from_frame_id: Some("prior".into()),
            to_frame_id: Some("current".into()),
            diff_type: Some("content_changed".into()),
            changed_region_json: None,
            added_text_hashes: Some("[\"new-content\"]".into()),
            removed_text_hashes: None,
            summary: None,
        });
        current.transition = Some(EvidenceTransition {
            id: "outgoing-transition".into(),
            primary_event_id: None,
            transition_type: Some("switched_app".into()),
            summary: None,
            confidence: Some(0.9),
            pre_frame_id: Some("current".into()),
            post_frame_id: Some("future".into()),
            changed_region_json: None,
        });

        let packet = build_observation_packet(&[prior, current], "watermark", None).unwrap();
        let delta = packet
            .frame_changes
            .iter()
            .find(|delta| delta.next_frame_id == "current")
            .unwrap();

        assert!(delta
            .observable_changes
            .iter()
            .any(|change| change == "content_appeared"));
        assert!(!delta
            .observable_changes
            .iter()
            .any(|change| change == "transition:switched_app"));
        assert_eq!(delta.source_agreement, vec!["frame_diff"]);
    }

    #[test]
    fn privacy_safe_typing_links_focus_and_post_frame_without_text() {
        let mut current = frame("current", 1_000, "manual");
        current.focused_node_evidence = true;
        current.content_units.push(content_unit(
            "editor",
            "focused editor",
            "",
            Rect {
                x: 0.0,
                y: 0.0,
                w: 200.0,
                h: 40.0,
            },
        ));
        current.transition = Some(EvidenceTransition {
            id: "t".into(),
            primary_event_id: None,
            transition_type: Some("submit".into()),
            summary: None,
            confidence: Some(0.9),
            pre_frame_id: Some("pre".into()),
            post_frame_id: Some("post".into()),
            changed_region_json: None,
        });
        current.typing_bursts.push(EvidenceTypingBurst {
            id: "burst".into(),
            started_at_ms: 900,
            ended_at_ms: 950,
            app_bundle_id: Some("com.example.test".into()),
            app_name: Some("Test App".into()),
            window_id: Some(1),
            window_title: Some("Window".into()),
            char_count: 8,
            enter_count: 1,
            paste_count: 0,
            committed: true,
            commit_signal: Some("enter".into()),
        });
        let packet = build_observation_packet(&[current], "watermark", None).unwrap();
        let event = packet
            .causal_events
            .iter()
            .find(|event| event.event_id == "burst")
            .unwrap();
        assert!(event.target_element_id.is_some());
        assert_eq!(event.target_frame_id.as_deref(), Some("post"));
        assert!(!serde_json::to_string(event).unwrap().contains("raw_text"));
    }

    #[test]
    fn committed_typing_without_ax_focus_is_grounded_to_the_exact_surface() {
        let mut current = frame("current", 1_000, "manual");
        current.app_name = Some("Helium".into());
        current.app_bundle_id = Some("net.imput.helium".into());
        current.window_id = Some(17);
        current.window_name = Some("ChatGPT - smalltalk - Helium".into());
        current.typing_bursts.push(EvidenceTypingBurst {
            id: "surface-commit".into(),
            started_at_ms: 900,
            ended_at_ms: 950,
            app_bundle_id: Some("net.imput.helium".into()),
            app_name: Some("Helium".into()),
            window_id: Some(17),
            window_title: Some("ChatGPT - smalltalk - Helium".into()),
            char_count: 24,
            enter_count: 1,
            paste_count: 0,
            committed: true,
            commit_signal: Some("enter".into()),
        });

        let packet = build_observation_packet(&[current], "watermark", None).unwrap();
        let event = packet
            .causal_events
            .iter()
            .find(|event| event.event_id == "surface-commit")
            .unwrap();

        assert_eq!(event.grounding_confidence, 0.68);
        assert_eq!(event.committed, Some(true));
        assert_eq!(event.observed_at_ms, 950);
        assert!(event.target_element_id.is_none());
        assert!(event
            .missing_evidence
            .iter()
            .any(|reason| reason == "typing_grounded_to_exact_app_and_window_only"));
        let serialized = serde_json::to_string(event).unwrap();
        assert!(!serialized.contains("raw_text"));
        assert!(!serialized.contains("typed_characters"));
    }

    #[test]
    fn two_sessions_cannot_silently_mix() {
        let old = frame("old", 1_000, "manual");
        let mut current = frame("current", 2_000, "manual");
        current.session_id = Some("session-b".into());
        assert!(build_observation_packet(&[old, current], "watermark", None)
            .unwrap_err()
            .contains("mixed_session"));
    }

    #[test]
    fn multi_display_coordinate_mapping_uses_display_origin_and_scale() {
        let mapped = logical_rect_to_pixels(
            Rect {
                x: -1400.0,
                y: 100.0,
                w: 500.0,
                h: 300.0,
            },
            -1920.0,
            0.0,
            2.0,
            3840,
            2160,
        )
        .unwrap();
        assert_eq!(
            (mapped.x, mapped.y, mapped.width, mapped.height),
            (1040.0, 200.0, 1000.0, 600.0)
        );
    }

    #[test]
    fn packet_audit_totals_match_retained_contents() {
        let mut current = frame("current", 1_000, "manual");
        current.content_units.push(content_unit(
            "page",
            "content",
            "page",
            Rect {
                x: 0.0,
                y: 0.0,
                w: 100.0,
                h: 20.0,
            },
        ));
        current
            .ui_events
            .push(ui_event("click", "click", 1_000, Some(1.0), Some(1.0)));
        let packet = build_observation_packet(&[current], "watermark", None).unwrap();
        assert_eq!(
            packet.size.canonical_element_count,
            packet
                .size
                .frame_accounting
                .iter()
                .map(|entry| entry.retained_elements)
                .sum::<usize>()
        );
        assert_eq!(
            packet.size.causal_event_count,
            packet
                .size
                .frame_accounting
                .iter()
                .map(|entry| entry.retained_events)
                .sum::<usize>()
        );
    }

    #[test]
    fn p6_task_roles_outrank_busy_unrelated_surface_and_old_completion() {
        let old_completion = frame("old-completion", 1_000, "manual");
        let latest_goal = frame("latest-goal", 2_000, "submit");
        let mut busy_unrelated = frame("busy-unrelated", 3_000, "manual");
        busy_unrelated.app_name = Some("Adjacent Chat".into());
        busy_unrelated.app_bundle_id = Some("com.example.adjacent".into());
        busy_unrelated.window_id = Some(8);
        for index in 0..40 {
            let mut event = ui_event(
                &format!("busy-{index}"),
                "scroll",
                3_000 + index,
                None,
                None,
            );
            event.app_bundle_id = Some("com.example.adjacent".into());
            event.window_id = Some(8);
            busy_unrelated.ui_events.push(event);
        }
        let current = frame("current", 4_000, "manual");
        let task_relevance = structured_task_relevance(vec![
            task_span(
                "prior-span",
                "old-completion",
                TaskEvidenceRoleV1::PriorTaskBoundary,
                "pane-00",
            ),
            task_span(
                "goal-span",
                "latest-goal",
                TaskEvidenceRoleV1::LatestUserGoal,
                "pane-00",
            ),
        ]);

        let packet = build_observation_packet_with_task_relevance(
            &[old_completion, latest_goal, busy_unrelated, current],
            "task-ranked",
            None,
            task_relevance,
        )
        .unwrap();

        let goal = packet
            .semantic_keyframes
            .iter()
            .find(|frame| frame.frame_id == "latest-goal")
            .expect("latest attributed goal must survive image pressure");
        assert_eq!(
            goal.task_evidence_role,
            Some(TaskEvidenceRoleV1::LatestUserGoal)
        );
        assert!(packet.image_candidates.iter().any(|candidate| {
            candidate.frame_id == "latest-goal"
                && candidate.selected
                && candidate.supports_latest_user_goal
        }));
        let busy = packet
            .image_candidates
            .iter()
            .find(|candidate| candidate.frame_id == "busy-unrelated")
            .unwrap();
        assert_eq!(busy.evidence_role, TaskEvidenceRoleV1::Unknown);
        assert_ne!(busy.same_task_relation, "current_task_turn");
    }

    #[test]
    fn near_duplicate_same_task_fact_keeps_newer_image_only() {
        let mut older = frame("goal-older", 1_000, "submit");
        let mut newer = frame("goal-newer", 2_000, "submit");
        older.content_hash = Some("same-stable-content".into());
        newer.content_hash = Some("same-stable-content".into());
        let current = frame("current", 3_000, "manual");
        let task_relevance = structured_task_relevance(vec![
            task_span(
                "goal-old-span",
                "goal-older",
                TaskEvidenceRoleV1::LatestUserGoal,
                "pane-00",
            ),
            task_span(
                "goal-new-span",
                "goal-newer",
                TaskEvidenceRoleV1::LatestUserGoal,
                "pane-00",
            ),
        ]);

        let packet = build_observation_packet_with_task_relevance(
            &[older, newer, current],
            "near-duplicate",
            None,
            task_relevance,
        )
        .unwrap();

        assert!(packet
            .semantic_keyframes
            .iter()
            .any(|frame| frame.frame_id == "goal-newer"));
        assert!(!packet
            .semantic_keyframes
            .iter()
            .any(|frame| frame.frame_id == "goal-older"));
        let older_audit = packet
            .image_candidates
            .iter()
            .find(|candidate| candidate.frame_id == "goal-older")
            .unwrap();
        assert!(older_audit
            .rejection_reasons
            .contains(&"near_duplicate_same_task_fact".into()));
    }

    #[test]
    fn same_visual_content_is_preserved_when_task_state_changes() {
        let mut user = frame("user", 1_000, "submit");
        let mut agent = frame("agent", 2_000, "surface_change");
        user.content_hash = Some("same-stable-content".into());
        agent.content_hash = Some("same-stable-content".into());
        let current = frame("current", 3_000, "manual");
        let task_relevance = structured_task_relevance(vec![
            task_span(
                "user-span",
                "user",
                TaskEvidenceRoleV1::LatestUserGoal,
                "pane-00",
            ),
            task_span(
                "agent-span",
                "agent",
                TaskEvidenceRoleV1::CurrentAgentState,
                "pane-00",
            ),
        ]);

        let packet = build_observation_packet_with_task_relevance(
            &[user, agent, current],
            "state-change",
            None,
            task_relevance,
        )
        .unwrap();
        let ids = packet
            .semantic_keyframes
            .iter()
            .map(|frame| frame.frame_id.as_str())
            .collect::<BTreeSet<_>>();
        assert!(ids.contains("user"));
        assert!(ids.contains("agent"));
    }

    #[test]
    fn focused_uncommitted_composer_becomes_current_unsent_draft() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE continue_salient_turn_evidence (
               frame_id TEXT PRIMARY KEY, observed_at_ms INTEGER NOT NULL,
               salient_span_ids_json TEXT NOT NULL, latest_user_span_ids_json TEXT NOT NULL,
               current_agent_span_ids_json TEXT NOT NULL,
               prior_boundary_span_ids_json TEXT NOT NULL,
               salient_user_goal_sample TEXT, salient_agent_state_sample TEXT,
               prior_boundary_sample TEXT, sampling_confidence REAL NOT NULL,
               missing_roles_json TEXT NOT NULL, rejected_spans_json TEXT NOT NULL,
               fallback_flags_json TEXT NOT NULL, causal_typing_attribution_json TEXT
             );
             CREATE TABLE continue_ordered_evidence_spans (
               span_id TEXT PRIMARY KEY, frame_id TEXT NOT NULL, surface_key TEXT,
               pane_id TEXT NOT NULL, region_role TEXT NOT NULL,
               conversational_role TEXT NOT NULL, reading_order INTEGER NOT NULL,
               source_scope TEXT NOT NULL, ownership_kind TEXT NOT NULL,
               owner_window_id INTEGER, owner_app_id TEXT,
               ownership_confidence REAL NOT NULL, region_confidence REAL NOT NULL,
               speaker_confidence REAL NOT NULL, order_confidence REAL NOT NULL,
               privacy_status TEXT NOT NULL, text_hash TEXT NOT NULL,
               focused INTEGER NOT NULL, selected INTEGER NOT NULL,
               geometry_json TEXT, quality_flags_json TEXT NOT NULL,
               reason_codes_json TEXT NOT NULL
             );",
        )
        .unwrap();
        conn.execute(
            "INSERT INTO continue_salient_turn_evidence VALUES
             (?1,?2,?3,'[]','[]',?4,NULL,NULL,?5,0.88,'[]','[]','[]',NULL)",
            rusqlite::params![
                "draft-frame",
                2_000_i64,
                "[\"draft-span\",\"prior-span\"]",
                "[\"prior-span\"]",
                "Earlier implementation completed"
            ],
        )
        .unwrap();
        for (span_id, frame_id, region, role, focused, order) in [
            (
                "prior-span",
                "prior-frame",
                "conversation_history",
                "assistant_or_agent",
                0_i64,
                1_i64,
            ),
            (
                "draft-span",
                "draft-frame",
                "composer",
                "unknown",
                1_i64,
                2_i64,
            ),
        ] {
            conn.execute(
                "INSERT INTO continue_ordered_evidence_spans VALUES
                 (?1,?2,'surface', 'pane-00',?3,?4,?5,'active_window',
                  'active_window_owned',1,'com.example.test',0.95,0.95,0.8,0.95,
                  'allowed',?6,?7,1,'{\"x\":10.0,\"y\":10.0,\"width\":500.0,\"height\":80.0}',
                  '[]','[]')",
                rusqlite::params![
                    span_id,
                    frame_id,
                    region,
                    role,
                    order,
                    format!("hash-{span_id}"),
                    focused
                ],
            )
            .unwrap();
        }
        let frames = vec![
            frame("prior-frame", 1_000, "surface_change"),
            frame("draft-frame", 2_000, "manual"),
        ];

        let relevance = load_task_relevance_evidence(&conn, &frames).unwrap();

        assert_eq!(relevance.source, "p6_role_region_task_turn");
        assert!(relevance.current_unsent_draft_present);
        let draft = relevance
            .spans
            .iter()
            .find(|span| span.span_id == "draft-span")
            .unwrap();
        assert_eq!(draft.evidence_role, TaskEvidenceRoleV1::CurrentUnsentDraft);
        assert_eq!(draft.submitted, Some(false));
        assert!(draft.focused);
        assert_eq!(draft.owner_window_id, Some(1));
        assert!(draft.geometry.is_some());

        let packet =
            build_observation_packet_with_task_relevance(&frames, "unsent-draft", None, relevance)
                .unwrap();
        assert_eq!(
            packet.current_frame.task_evidence_role,
            Some(TaskEvidenceRoleV1::CurrentUnsentDraft)
        );
        assert!(!packet.surface_timeline[0].carried_into_current_surface);
    }
}
