use super::activity_recap::sanitize_public_text;
use super::{
    is_generated_debug_surface, is_smalltalk_self_surface, parse_string_array,
    privacy_status_is_limited, table_exists, ContinueActivitySummary,
    ContinueAppActivityIntelligence, ContinueDecisionQualityGate, ContinueMemoryRetrievalReport,
    ContinueSupportEvidenceItem, CurrentSurfaceResolutionAudit, EvidenceFreshnessLedger,
    P0QualitySignals, ResolvedCurrentSurface, ScoredContinueCandidate, ScorerArtifact,
    ScorerWorkstream,
};
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use serde_json::{json, Value};

pub(crate) const ACTIVITY_RECAP_INPUTS_SCHEMA: &str = "smalltalk.activity_recap_inputs.v1";
const MAX_SELECTED_RELEVANCE_MS: i64 = 7 * 24 * 60 * 60 * 1000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ActivityRecapInputCaps {
    pub max_activity_segments: usize,
    pub max_task_actions: usize,
    pub max_semantic_moments: usize,
    pub max_open_loops: usize,
    pub max_workstream_states: usize,
    pub max_branch_contexts: usize,
    pub max_surface_snapshots: usize,
    pub max_memory_cells: usize,
    pub max_support_items: usize,
}

impl Default for ActivityRecapInputCaps {
    fn default() -> Self {
        Self {
            max_activity_segments: 12,
            max_task_actions: 40,
            max_semantic_moments: 30,
            max_open_loops: 8,
            max_workstream_states: 8,
            max_branch_contexts: 8,
            max_surface_snapshots: 8,
            max_memory_cells: 12,
            max_support_items: 12,
        }
    }
}

pub(super) struct ActivityRecapBuildContext<'a> {
    pub decision_id_seed: Option<&'a str>,
    pub session_id: Option<&'a str>,
    pub mode: &'a str,
    pub lookback_ms: i64,
    pub evidence_watermark: Option<&'a str>,
    pub output_mode: Option<&'a str>,
    pub requested_at_ms: i64,
    pub current_surface: Option<&'a ResolvedCurrentSurface>,
    pub current_surface_resolution: Option<&'a CurrentSurfaceResolutionAudit>,
    pub selected_workstream: Option<&'a ScorerWorkstream>,
    pub selected_candidate: Option<&'a ScoredContinueCandidate>,
    pub public_return_candidate: Option<&'a ScoredContinueCandidate>,
    pub app_activity: &'a ContinueAppActivityIntelligence,
    pub support_evidence: &'a [ContinueSupportEvidenceItem],
    pub memory_retrieval: &'a ContinueMemoryRetrievalReport,
    pub p0_quality_signals: Option<&'a P0QualitySignals>,
    pub evidence_freshness_ledger: Option<&'a EvidenceFreshnessLedger>,
    pub app_activity_summary: Option<&'a ContinueActivitySummary>,
    pub quality_gate: Option<&'a ContinueDecisionQualityGate>,
    pub caps: ActivityRecapInputCaps,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub(crate) struct ActivityRecapInputs {
    pub schema: String,
    pub decision_context: ActivityRecapDecisionContext,
    pub current_surface: Option<CurrentSurfaceFact>,
    pub selected_workstream: Option<WorkstreamFact>,
    pub selected_candidate: Option<CandidateFact>,
    pub return_target: Option<TargetFact>,
    pub resume_work_target: Option<TargetFact>,
    pub recent_segments: Vec<ActivitySegmentFact>,
    pub recent_actions: Vec<TaskActionFact>,
    pub recent_moments: Vec<SemanticMomentFact>,
    pub open_loops: Vec<OpenLoopFact>,
    pub workstream_states: Vec<WorkstreamStateFact>,
    pub branch_contexts: Vec<BranchContextFact>,
    pub surface_snapshots: Vec<SurfaceSnapshotFact>,
    pub support_evidence: Vec<SupportEvidenceFact>,
    pub memory_facts: Vec<MemoryFact>,
    pub existing_quality: ExistingQualityFacts,
    pub input_warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct ActivityRecapDecisionContext {
    pub decision_id_seed: Option<String>,
    pub mode: String,
    pub lookback_ms: i64,
    pub evidence_watermark: Option<String>,
    pub output_mode: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub(crate) struct CurrentSurfaceFact {
    pub surface_id: String,
    pub artifact_id: Option<String>,
    pub app_name: Option<String>,
    pub display_title: Option<String>,
    pub domain: Option<String>,
    pub activity_state: Option<String>,
    pub task_state: Option<String>,
    pub observed_at_ms: i64,
    pub evidence_quality: String,
    pub openability: String,
    pub focus_confidence: f64,
    pub identity_confidence: Option<f64>,
    pub snapshot_id: Option<String>,
    pub evidence_ids: Vec<String>,
    pub missing_evidence: Vec<String>,
    pub claim_eligible: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub(crate) struct WorkstreamFact {
    pub workstream_id: String,
    pub state: String,
    pub title: Option<String>,
    pub primary_artifact_id: Option<String>,
    pub last_active_timestamp_ms: i64,
    pub confidence: f64,
    pub unresolved_signal: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub(crate) struct CandidateFact {
    pub candidate_id: String,
    pub workstream_id: String,
    pub candidate_kind: String,
    pub target_artifact_id: Option<String>,
    pub last_meaningful_action_id: Option<String>,
    pub open_loop_id: Option<String>,
    pub activity_segment_id: Option<String>,
    pub activity_intent: Option<String>,
    pub task_phase: Option<String>,
    pub continuation_role: Option<String>,
    pub score: f64,
    pub evidence_sufficiency_score: f64,
    pub missing_evidence: Vec<String>,
    pub branch_promotion_state: Option<String>,
    pub branch_public_return_eligible: Option<bool>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub(crate) struct TargetFact {
    pub artifact_id: String,
    pub artifact_kind: String,
    pub display_title: Option<String>,
    pub openability: String,
    pub evidence_quality: String,
    pub identity_confidence: f64,
    pub evidence_frame_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub(crate) struct ActivitySegmentFact {
    pub segment_id: String,
    pub artifact_id: Option<String>,
    pub workstream_id: Option<String>,
    pub episode_id: Option<String>,
    pub app_name: Option<String>,
    pub display_title: Option<String>,
    pub app_family: String,
    pub surface_type: String,
    pub started_at_ms: i64,
    pub ended_at_ms: i64,
    pub activity_intent: Option<String>,
    pub task_phase: Option<String>,
    pub continuation_role: Option<String>,
    pub work_value_score: Option<f64>,
    pub support_score: Option<f64>,
    pub divergence_score: Option<f64>,
    pub evidence_sufficiency_score: Option<f64>,
    pub reason: Option<String>,
    pub missing_evidence: Vec<String>,
    pub evidence_kinds: Vec<String>,
    pub has_heavy_frame: bool,
    pub has_direct_url: bool,
    pub has_document_path: bool,
    pub has_visible_text: bool,
    pub is_event_backed_only: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub(crate) struct TaskActionFact {
    pub action_id: String,
    pub frame_id: String,
    pub artifact_id: Option<String>,
    pub secondary_artifact_id: Option<String>,
    pub action_kind: String,
    pub action_role: String,
    pub confidence: f64,
    pub created_at_ms: i64,
    pub semantic_delta_kind: Option<String>,
    pub semantic_subject: Option<String>,
    pub semantic_after_hint: Option<String>,
    pub evidence_source_kind: Option<String>,
    pub evidence_span_ids: Vec<String>,
    pub attribution_confidence: Option<f64>,
    pub quality_flags: Vec<String>,
    pub branch_kind: Option<String>,
    pub branch_action_role: Option<String>,
    pub branch_confidence: Option<f64>,
    pub branch_reason_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub(crate) struct SemanticMomentFact {
    pub moment_id: String,
    pub started_at_ms: i64,
    pub ended_at_ms: i64,
    pub pre_frame_id: Option<i64>,
    pub post_frame_id: Option<i64>,
    pub pre_artifact_id: Option<String>,
    pub post_artifact_id: Option<String>,
    pub dominant_artifact_id: Option<String>,
    pub dominant_event_type: String,
    pub ordered_event_ids: Vec<String>,
    pub boundary_kind: String,
    pub current_focus_relation: Option<String>,
    pub semantic_summary: Option<String>,
    pub evidence_quality: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub(crate) struct EvidenceSpanFact {
    pub evidence_kind: String,
    pub evidence_id: String,
    pub role: String,
    pub summary: Option<String>,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub(crate) struct OpenLoopFact {
    pub open_loop_id: String,
    pub workstream_id: String,
    pub state: String,
    pub boundary_kind: String,
    pub quality: String,
    pub confidence: f64,
    pub origin_artifact_id: Option<String>,
    pub current_focus_artifact_id: Option<String>,
    pub primary_return_artifact_id: Option<String>,
    pub resume_work_artifact_id: Option<String>,
    pub blocker_artifact_id: Option<String>,
    pub verification_artifact_id: Option<String>,
    pub objective_hint: Option<String>,
    pub last_concrete_progress: Option<String>,
    pub unfinished_state: Option<String>,
    pub next_evidence_backed_action: Option<String>,
    pub current_focus_relation: Option<String>,
    pub missing_evidence: Vec<String>,
    pub evidence_spans: Vec<EvidenceSpanFact>,
    pub last_updated_at_ms: i64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub(crate) struct WorkstreamStateFact {
    pub snapshot_id: String,
    pub workstream_id: String,
    pub observed_at_ms: i64,
    pub state: String,
    pub previous_state: Option<String>,
    pub confidence: f64,
    pub transition_kind: Option<String>,
    pub has_blocker: bool,
    pub evidence_action_ids: Vec<String>,
    pub missing_evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub(crate) struct BranchContextFact {
    pub branch_id: String,
    pub branch_action_id: String,
    pub origin_artifact_id: Option<String>,
    pub origin_workstream_id: Option<String>,
    pub branch_artifact_id: String,
    pub branch_kind: String,
    pub branch_started_at_ms: i64,
    pub last_branch_seen_at_ms: i64,
    pub returned_to_origin_at_ms: Option<i64>,
    pub promotion_state: String,
    pub promotion_reason: Option<String>,
    pub confidence: f64,
    pub reason_code: Option<String>,
    pub evidence_action_ids: Vec<String>,
    pub updated_at_ms: i64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub(crate) struct SurfaceSnapshotFact {
    pub snapshot_id: String,
    pub artifact_id: Option<String>,
    pub frame_id: Option<i64>,
    pub domain: String,
    pub app_name: Option<String>,
    pub display_title: Option<String>,
    pub relative_file_name: Option<String>,
    pub git_branch: Option<String>,
    pub activity_state: Option<String>,
    pub task_state: Option<String>,
    pub command_state: Option<String>,
    pub has_error_markers: bool,
    pub identity_confidence: String,
    pub evidence_quality: String,
    pub openability: String,
    pub missing_evidence: Vec<String>,
    pub evidence_sources: Vec<String>,
    pub observed_at_ms: i64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct SupportEvidenceFact {
    pub artifact_id: Option<String>,
    pub artifact_kind: Option<String>,
    pub display_title: Option<String>,
    pub branch_kind: String,
    pub origin_artifact_id: Option<String>,
    pub role: String,
    pub public_return_eligible: bool,
    pub reason: Option<String>,
    pub evidence_anchor_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub(crate) struct MemoryFact {
    pub memory_id: String,
    pub workstream_id: Option<String>,
    pub artifact_id: Option<String>,
    pub episode_id: Option<String>,
    pub action_id: Option<String>,
    pub memory_type: String,
    pub relation: String,
    pub summary: Option<String>,
    pub last_seen_at_ms: i64,
    pub confidence: f64,
    pub importance: f64,
    pub retrieval_score: f64,
    pub feedback_score: f64,
    pub retrieval_reasons: Vec<String>,
    pub supports_candidate_ids: Vec<String>,
    pub contradicts_candidate_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub(crate) struct ExistingQualityFacts {
    pub p0_quality_signals: Option<Value>,
    pub current_surface_resolution: Option<Value>,
    pub evidence_freshness_ledger: Option<Value>,
    pub app_activity_summary: Option<Value>,
    pub quality_gate: Option<Value>,
}

pub(super) fn build_activity_recap_inputs(
    conn: &Connection,
    context: ActivityRecapBuildContext<'_>,
) -> ActivityRecapInputs {
    let since_ms = context
        .requested_at_ms
        .saturating_sub(context.lookback_ms.max(1));
    let selected_relevance_since_ms = context
        .requested_at_ms
        .saturating_sub(context.lookback_ms.clamp(1, MAX_SELECTED_RELEVANCE_MS));
    let selected_workstream_id = context.selected_workstream.map(|value| value.id.as_str());
    let selected_artifact_id = context
        .selected_candidate
        .and_then(|candidate| candidate.target_artifact.as_ref())
        .map(|artifact| artifact.id.as_str());
    let mut input_warnings = Vec::new();

    let recent_segments = map_recent_segments(conn, &context, since_ms);
    let recent_actions = section_or_warning(
        load_recent_actions(
            conn,
            context.session_id,
            since_ms,
            context.caps.max_task_actions,
            selected_artifact_id,
        ),
        "task_actions",
        &mut input_warnings,
    );
    let recent_moments = section_or_warning(
        load_recent_moments(
            conn,
            since_ms,
            context.caps.max_semantic_moments,
            selected_artifact_id,
        ),
        "semantic_moments",
        &mut input_warnings,
    );
    let open_loops = section_or_warning(
        load_open_loops(
            conn,
            since_ms,
            selected_relevance_since_ms,
            context.caps.max_open_loops,
            selected_workstream_id,
            selected_artifact_id,
        ),
        "open_loops",
        &mut input_warnings,
    );
    let workstream_states = section_or_warning(
        load_workstream_states(
            conn,
            since_ms,
            selected_relevance_since_ms,
            context.caps.max_workstream_states,
            selected_workstream_id,
            selected_artifact_id,
        ),
        "workstream_states",
        &mut input_warnings,
    );
    let branch_contexts = section_or_warning(
        load_branch_contexts(
            conn,
            since_ms,
            selected_relevance_since_ms,
            context.caps.max_branch_contexts,
            selected_workstream_id,
            selected_artifact_id,
        ),
        "branch_contexts",
        &mut input_warnings,
    );
    let surface_snapshots = section_or_warning(
        load_surface_snapshots(
            conn,
            since_ms,
            context.caps.max_surface_snapshots,
            selected_artifact_id,
        ),
        "surface_snapshots",
        &mut input_warnings,
    );

    ActivityRecapInputs {
        schema: ACTIVITY_RECAP_INPUTS_SCHEMA.to_string(),
        decision_context: ActivityRecapDecisionContext {
            decision_id_seed: context.decision_id_seed.map(str::to_string),
            mode: context.mode.to_string(),
            lookback_ms: context.lookback_ms,
            evidence_watermark: context.evidence_watermark.map(str::to_string),
            output_mode: context.output_mode.map(str::to_string),
        },
        current_surface: context.current_surface.map(|surface| {
            current_surface_fact(
                surface,
                selected_artifact_matches(&context, surface.artifact_id.as_deref()),
            )
        }),
        selected_workstream: context.selected_workstream.map(workstream_fact),
        selected_candidate: context.selected_candidate.map(candidate_fact),
        return_target: context
            .public_return_candidate
            .and_then(|candidate| candidate.target_artifact.as_ref())
            .and_then(|artifact| target_fact(conn, artifact, context.public_return_candidate)),
        resume_work_target: context.public_return_candidate.and_then(|candidate| {
            candidate
                .resume_work_target
                .as_ref()
                .or(candidate.target_artifact.as_ref())
                .and_then(|artifact| target_fact(conn, artifact, Some(candidate)))
        }),
        recent_segments,
        recent_actions,
        recent_moments,
        open_loops,
        workstream_states,
        branch_contexts,
        surface_snapshots,
        support_evidence: map_support_evidence(
            conn,
            context.support_evidence,
            context.caps.max_support_items,
        ),
        memory_facts: map_memory_facts(
            conn,
            context.memory_retrieval,
            context.caps.max_memory_cells,
        ),
        existing_quality: existing_quality_facts(&context),
        input_warnings,
    }
}

fn section_or_warning<T>(
    result: Result<Vec<T>, String>,
    section: &str,
    warnings: &mut Vec<String>,
) -> Vec<T> {
    match result {
        Ok(values) => values,
        Err(_) => {
            warnings.push(format!("activity_recap_inputs:{}_unavailable", section));
            Vec::new()
        }
    }
}

fn current_surface_fact(
    surface: &ResolvedCurrentSurface,
    selected_artifact_match: bool,
) -> CurrentSurfaceFact {
    CurrentSurfaceFact {
        surface_id: surface.surface_id.clone(),
        artifact_id: surface.artifact_id.clone(),
        app_name: safe_optional(surface.app_name.as_deref(), 80),
        display_title: safe_optional(surface.window_title.as_deref(), 160)
            .or_else(|| safe_optional(surface.app_name.as_deref(), 80)),
        domain: safe_optional(surface.domain.as_deref(), 80),
        activity_state: safe_optional(surface.activity_state.as_deref(), 100),
        task_state: safe_optional(surface.task_state.as_deref(), 100),
        observed_at_ms: surface.observed_at_ms,
        evidence_quality: surface.evidence_quality.clone(),
        openability: surface.openability.clone(),
        focus_confidence: surface.focus_confidence,
        identity_confidence: surface.identity_confidence,
        snapshot_id: surface.snapshot_id.clone(),
        evidence_ids: surface.evidence_ids.iter().take(12).cloned().collect(),
        missing_evidence: safe_string_list(&surface.missing_fields, 12, 120),
        claim_eligible: (!surface.is_self_surface || selected_artifact_match)
            && !surface.is_generated_debug_surface,
    }
}

fn workstream_fact(workstream: &ScorerWorkstream) -> WorkstreamFact {
    WorkstreamFact {
        workstream_id: workstream.id.clone(),
        state: workstream.state.clone(),
        title: safe_optional(workstream.title_candidate.as_deref(), 160),
        primary_artifact_id: workstream.primary_artifact_id.clone(),
        last_active_timestamp_ms: workstream.last_active_timestamp_ms,
        confidence: workstream.confidence,
        unresolved_signal: safe_optional(workstream.unresolved_signal.as_deref(), 120),
    }
}

fn candidate_fact(candidate: &ScoredContinueCandidate) -> CandidateFact {
    CandidateFact {
        candidate_id: candidate.id.clone(),
        workstream_id: candidate.workstream_id.clone(),
        candidate_kind: candidate.candidate_kind.clone(),
        target_artifact_id: candidate
            .target_artifact
            .as_ref()
            .map(|artifact| artifact.id.clone()),
        last_meaningful_action_id: candidate
            .last_meaningful_action
            .as_ref()
            .map(|action| action.id.clone()),
        open_loop_id: candidate
            .open_loop
            .as_ref()
            .map(|open_loop| open_loop.id.clone()),
        activity_segment_id: candidate.app_activity_segment_id.clone(),
        activity_intent: safe_optional(candidate.activity_intent.as_deref(), 100),
        task_phase: safe_optional(candidate.task_phase.as_deref(), 100),
        continuation_role: safe_optional(candidate.continuation_role.as_deref(), 100),
        score: candidate.score,
        evidence_sufficiency_score: candidate.evidence_sufficiency_score,
        missing_evidence: safe_string_list(&candidate.missing_evidence, 16, 120),
        branch_promotion_state: candidate.branch_promotion_state.clone(),
        branch_public_return_eligible: candidate.branch_public_return_eligible,
    }
}

fn target_fact(
    conn: &Connection,
    artifact: &ScorerArtifact,
    candidate: Option<&ScoredContinueCandidate>,
) -> Option<TargetFact> {
    if artifact_privacy_blocked(conn, Some(&artifact.id))
        || artifact
            .privacy_status
            .as_deref()
            .is_some_and(privacy_blocked)
    {
        return None;
    }
    Some(TargetFact {
        artifact_id: artifact.id.clone(),
        artifact_kind: artifact.artifact_kind.clone(),
        display_title: safe_optional(artifact.display_title.as_deref(), 160),
        openability: artifact.openability.clone(),
        evidence_quality: artifact.evidence_quality.clone(),
        identity_confidence: artifact.identity_confidence,
        evidence_frame_id: candidate.and_then(|value| value.evidence_frame_id.clone()),
    })
}

fn map_recent_segments(
    conn: &Connection,
    context: &ActivityRecapBuildContext<'_>,
    since_ms: i64,
) -> Vec<ActivitySegmentFact> {
    let mut segments = context.app_activity.segments.iter().collect::<Vec<_>>();
    segments.sort_by(|left, right| {
        right
            .ended_at_ms
            .cmp(&left.ended_at_ms)
            .then_with(|| left.id.cmp(&right.id))
    });
    let mut output = Vec::new();
    for segment in segments {
        if output.len() >= context.caps.max_activity_segments {
            break;
        }
        if segment.ended_at_ms < since_ms
            || context
                .session_id
                .is_some_and(|session| segment.session_id.as_deref() != Some(session))
            || artifact_claim_excluded(
                conn,
                segment.artifact_id.as_deref(),
                context
                    .selected_candidate
                    .and_then(|candidate| candidate.target_artifact.as_ref())
                    .map(|artifact| artifact.id.as_str()),
            )
        {
            continue;
        }
        let selected_match = selected_artifact_matches(context, segment.artifact_id.as_deref());
        if segment.is_self_or_diagnostic && !selected_match {
            continue;
        }
        let classification = context
            .app_activity
            .classifications
            .iter()
            .filter(|value| value.segment_id == segment.id)
            .max_by_key(|value| value.updated_at_ms);
        output.push(ActivitySegmentFact {
            segment_id: segment.id.clone(),
            artifact_id: segment.artifact_id.clone(),
            workstream_id: segment.workstream_id.clone(),
            episode_id: segment.episode_id.clone(),
            app_name: safe_optional(segment.app_name.as_deref(), 80),
            display_title: safe_optional(segment.window_title.as_deref(), 160)
                .or_else(|| safe_optional(segment.app_name.as_deref(), 80)),
            app_family: segment.app_family.clone(),
            surface_type: segment.surface_type.clone(),
            started_at_ms: segment.started_at_ms,
            ended_at_ms: segment.ended_at_ms,
            activity_intent: classification
                .and_then(|value| safe_optional(Some(&value.activity_intent), 100)),
            task_phase: classification
                .and_then(|value| safe_optional(Some(&value.task_phase), 100)),
            continuation_role: classification
                .and_then(|value| safe_optional(Some(&value.continuation_role), 100)),
            work_value_score: classification.map(|value| value.work_value_score),
            support_score: classification.map(|value| value.support_score),
            divergence_score: classification.map(|value| value.divergence_score),
            evidence_sufficiency_score: classification
                .map(|value| value.evidence_sufficiency_score),
            reason: classification.and_then(|value| safe_optional(Some(&value.reason), 180)),
            missing_evidence: classification
                .map(|value| safe_string_list(&value.missing_evidence, 12, 120))
                .unwrap_or_default(),
            evidence_kinds: safe_string_list(&segment.evidence_kinds, 12, 80),
            has_heavy_frame: segment.has_heavy_frame,
            has_direct_url: segment.has_direct_url,
            has_document_path: segment.has_document_path,
            has_visible_text: segment.has_visible_text,
            is_event_backed_only: segment.is_event_backed_only,
        });
    }
    output
}

fn selected_artifact_matches(
    context: &ActivityRecapBuildContext<'_>,
    artifact_id: Option<&str>,
) -> bool {
    let Some(artifact_id) = artifact_id else {
        return false;
    };
    context.selected_candidate.is_some_and(|candidate| {
        candidate
            .target_artifact
            .as_ref()
            .is_some_and(|artifact| artifact.id == artifact_id)
            || candidate
                .resume_work_target
                .as_ref()
                .is_some_and(|artifact| artifact.id == artifact_id)
    }) || context
        .selected_workstream
        .is_some_and(|workstream| workstream.primary_artifact_id.as_deref() == Some(artifact_id))
}

fn load_recent_actions(
    conn: &Connection,
    session_id: Option<&str>,
    since_ms: i64,
    limit: usize,
    selected_artifact_id: Option<&str>,
) -> Result<Vec<TaskActionFact>, String> {
    if limit == 0 || !table_exists(conn, "continue_task_actions")? {
        return Ok(Vec::new());
    }
    let has_frames = table_exists(conn, "frames")?;
    let frame_columns = if has_frames {
        "f.privacy_status, f.app_name, f.app_bundle_id, f.window_name, f.browser_url, f.document_path"
    } else {
        "NULL, NULL, NULL, NULL, NULL, NULL"
    };
    let frame_join = if has_frames {
        "LEFT JOIN frames f ON CAST(f.id AS TEXT) = ta.frame_id"
    } else {
        ""
    };
    let session_clause = if has_frames && session_id.is_some() {
        "AND (f.session_id = ?3 OR ta.frame_id LIKE 'event-surface-%')"
    } else {
        ""
    };
    let sql = format!(
        "SELECT ta.id, ta.frame_id, ta.artifact_id, ta.secondary_artifact_id,
                ta.action_kind, ta.action_role, ta.confidence, ta.created_at_ms,
                ta.semantic_delta_kind, ta.semantic_subject, ta.semantic_after_hint,
                ta.evidence_source_kind, ta.evidence_span_ids_json,
                ta.attribution_confidence, ta.quality_flags_json, ta.branch_kind,
                ta.branch_action_role, ta.branch_confidence, ta.branch_reason_code,
                a.privacy_status, secondary.privacy_status, {frame_columns}
         FROM continue_task_actions ta
         LEFT JOIN continue_artifacts a ON a.id = ta.artifact_id
         LEFT JOIN continue_artifacts secondary ON secondary.id = ta.secondary_artifact_id
         {frame_join}
         WHERE ta.created_at_ms >= ?1 {session_clause}
         ORDER BY ta.created_at_ms DESC, ta.id DESC
         LIMIT ?2"
    );
    let mut stmt = conn.prepare(&sql).map_err(|error| error.to_string())?;
    let map_row = |row: &rusqlite::Row<'_>| {
        Ok((
            TaskActionFact {
                action_id: row.get(0)?,
                frame_id: row.get(1)?,
                artifact_id: row.get(2)?,
                secondary_artifact_id: row.get(3)?,
                action_kind: row.get(4)?,
                action_role: row.get(5)?,
                confidence: row.get(6)?,
                created_at_ms: row.get(7)?,
                semantic_delta_kind: safe_optional(
                    row.get::<_, Option<String>>(8)?.as_deref(),
                    100,
                ),
                semantic_subject: safe_optional(row.get::<_, Option<String>>(9)?.as_deref(), 120),
                semantic_after_hint: safe_optional(
                    row.get::<_, Option<String>>(10)?.as_deref(),
                    180,
                ),
                evidence_source_kind: row.get(11)?,
                evidence_span_ids: parse_string_array(&row.get::<_, String>(12)?),
                attribution_confidence: row.get(13)?,
                quality_flags: safe_string_list(
                    &parse_string_array(&row.get::<_, String>(14)?),
                    12,
                    100,
                ),
                branch_kind: row.get(15)?,
                branch_action_role: row.get(16)?,
                branch_confidence: row.get(17)?,
                branch_reason_code: row.get(18)?,
            },
            row.get::<_, Option<String>>(19)?,
            row.get::<_, Option<String>>(20)?,
            row.get::<_, Option<String>>(21)?,
            row.get::<_, Option<String>>(22)?,
            row.get::<_, Option<String>>(23)?,
            row.get::<_, Option<String>>(24)?,
            row.get::<_, Option<String>>(25)?,
            row.get::<_, Option<String>>(26)?,
        ))
    };
    let rows = if has_frames && session_id.is_some() {
        stmt.query_map(params![since_ms, limit as i64, session_id], map_row)
    } else {
        stmt.query_map(params![since_ms, limit as i64], map_row)
    }
    .map_err(|error| error.to_string())?;
    let mut output = Vec::new();
    for row in rows {
        let (
            fact,
            primary_privacy,
            secondary_privacy,
            frame_privacy,
            app,
            bundle,
            title,
            url,
            path,
        ) = row.map_err(|error| error.to_string())?;
        if [primary_privacy, secondary_privacy, frame_privacy]
            .iter()
            .flatten()
            .any(|status| privacy_blocked(status))
        {
            continue;
        }
        let self_surface = is_smalltalk_self_surface(
            app.as_deref(),
            bundle.as_deref(),
            title.as_deref(),
            url.as_deref(),
            path.as_deref(),
        );
        if (self_surface && fact.artifact_id.as_deref() != selected_artifact_id)
            || is_generated_debug_surface(
                app.as_deref(),
                title.as_deref(),
                url.as_deref(),
                path.as_deref(),
            )
        {
            continue;
        }
        output.push(fact);
    }
    Ok(output)
}

fn load_recent_moments(
    conn: &Connection,
    since_ms: i64,
    limit: usize,
    selected_artifact_id: Option<&str>,
) -> Result<Vec<SemanticMomentFact>, String> {
    if limit == 0 || !table_exists(conn, "continue_semantic_moments")? {
        return Ok(Vec::new());
    }
    let mut stmt = conn
        .prepare(
            "SELECT sm.id, sm.started_at_ms, sm.ended_at_ms, sm.pre_frame_id,
                    sm.post_frame_id, sm.pre_artifact_id, sm.post_artifact_id,
                    sm.dominant_artifact_id, sm.dominant_event_type,
                    sm.ordered_event_ids_json, sm.boundary_kind,
                    sm.current_focus_relation, sm.semantic_summary, sm.evidence_quality,
                    pre.privacy_status, post.privacy_status, dominant.privacy_status
             FROM continue_semantic_moments sm
             LEFT JOIN continue_artifacts pre ON pre.id = sm.pre_artifact_id
             LEFT JOIN continue_artifacts post ON post.id = sm.post_artifact_id
             LEFT JOIN continue_artifacts dominant ON dominant.id = sm.dominant_artifact_id
             WHERE sm.ended_at_ms >= ?1
             ORDER BY sm.ended_at_ms DESC, sm.id DESC
             LIMIT ?2",
        )
        .map_err(|error| error.to_string())?;
    let rows = stmt
        .query_map(params![since_ms, limit as i64], |row| {
            Ok((
                SemanticMomentFact {
                    moment_id: row.get(0)?,
                    started_at_ms: row.get(1)?,
                    ended_at_ms: row.get(2)?,
                    pre_frame_id: row.get(3)?,
                    post_frame_id: row.get(4)?,
                    pre_artifact_id: row.get(5)?,
                    post_artifact_id: row.get(6)?,
                    dominant_artifact_id: row.get(7)?,
                    dominant_event_type: row.get(8)?,
                    ordered_event_ids: parse_string_array(&row.get::<_, String>(9)?),
                    boundary_kind: row.get(10)?,
                    current_focus_relation: row.get(11)?,
                    semantic_summary: safe_optional(
                        row.get::<_, Option<String>>(12)?.as_deref(),
                        180,
                    ),
                    evidence_quality: row.get(13)?,
                },
                row.get::<_, Option<String>>(14)?,
                row.get::<_, Option<String>>(15)?,
                row.get::<_, Option<String>>(16)?,
            ))
        })
        .map_err(|error| error.to_string())?;
    let mut output = Vec::new();
    for row in rows {
        let (fact, pre_privacy, post_privacy, dominant_privacy) =
            row.map_err(|error| error.to_string())?;
        if [pre_privacy, post_privacy, dominant_privacy]
            .iter()
            .flatten()
            .any(|status| privacy_blocked(status))
            || [
                fact.pre_artifact_id.as_deref(),
                fact.post_artifact_id.as_deref(),
                fact.dominant_artifact_id.as_deref(),
            ]
            .into_iter()
            .flatten()
            .any(|artifact_id| {
                artifact_claim_excluded(conn, Some(artifact_id), selected_artifact_id)
            })
        {
            continue;
        }
        output.push(fact);
    }
    Ok(output)
}

fn load_open_loops(
    conn: &Connection,
    since_ms: i64,
    selected_relevance_since_ms: i64,
    limit: usize,
    selected_workstream_id: Option<&str>,
    selected_artifact_id: Option<&str>,
) -> Result<Vec<OpenLoopFact>, String> {
    if limit == 0 || !table_exists(conn, "continue_open_loops")? {
        return Ok(Vec::new());
    }
    let mut stmt = conn
        .prepare(
            "SELECT id, workstream_id, state, boundary_kind, quality, confidence,
                    origin_artifact_id, current_focus_artifact_id,
                    primary_return_artifact_id, resume_work_artifact_id,
                    blocker_artifact_id, verification_artifact_id, objective_hint,
                    last_concrete_progress, unfinished_state,
                    next_evidence_backed_action, current_focus_relation,
                    missing_fields_json, evidence_spans_json, last_updated_at_ms
             FROM continue_open_loops
             WHERE last_updated_at_ms >= ?1
                OR (?2 IS NOT NULL AND workstream_id = ?2 AND last_updated_at_ms >= ?3)
             ORDER BY CASE WHEN workstream_id = ?2 THEN 0 ELSE 1 END,
                      last_updated_at_ms DESC, confidence DESC, id DESC
             LIMIT ?4",
        )
        .map_err(|error| error.to_string())?;
    let rows = stmt
        .query_map(
            params![
                since_ms,
                selected_workstream_id,
                selected_relevance_since_ms,
                limit as i64
            ],
            |row| {
                let spans = serde_json::from_str::<Vec<super::ContinueEvidenceSpan>>(
                    &row.get::<_, String>(18)?,
                )
                .unwrap_or_default()
                .into_iter()
                .take(12)
                .map(|span| EvidenceSpanFact {
                    evidence_kind: span.evidence_kind,
                    evidence_id: span.evidence_id,
                    role: span.role,
                    summary: safe_optional(span.summary.as_deref(), 160),
                    confidence: span.confidence,
                })
                .collect();
                Ok(OpenLoopFact {
                    open_loop_id: row.get(0)?,
                    workstream_id: row.get(1)?,
                    state: row.get(2)?,
                    boundary_kind: row.get(3)?,
                    quality: row.get(4)?,
                    confidence: row.get(5)?,
                    origin_artifact_id: row.get(6)?,
                    current_focus_artifact_id: row.get(7)?,
                    primary_return_artifact_id: row.get(8)?,
                    resume_work_artifact_id: row.get(9)?,
                    blocker_artifact_id: row.get(10)?,
                    verification_artifact_id: row.get(11)?,
                    objective_hint: safe_optional(
                        row.get::<_, Option<String>>(12)?.as_deref(),
                        180,
                    ),
                    last_concrete_progress: safe_optional(
                        row.get::<_, Option<String>>(13)?.as_deref(),
                        220,
                    ),
                    unfinished_state: safe_optional(
                        row.get::<_, Option<String>>(14)?.as_deref(),
                        220,
                    ),
                    next_evidence_backed_action: safe_optional(
                        row.get::<_, Option<String>>(15)?.as_deref(),
                        220,
                    ),
                    current_focus_relation: row.get(16)?,
                    missing_evidence: safe_string_list(
                        &parse_string_array(&row.get::<_, String>(17)?),
                        16,
                        120,
                    ),
                    evidence_spans: spans,
                    last_updated_at_ms: row.get(19)?,
                })
            },
        )
        .map_err(|error| error.to_string())?;
    let rows = rows
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    Ok(rows
        .into_iter()
        .filter(|open_loop| {
            [
                open_loop.origin_artifact_id.as_deref(),
                open_loop.current_focus_artifact_id.as_deref(),
                open_loop.primary_return_artifact_id.as_deref(),
                open_loop.resume_work_artifact_id.as_deref(),
                open_loop.blocker_artifact_id.as_deref(),
                open_loop.verification_artifact_id.as_deref(),
            ]
            .into_iter()
            .flatten()
            .all(|artifact_id| {
                !artifact_claim_excluded(conn, Some(artifact_id), selected_artifact_id)
            })
        })
        .collect())
}

fn load_workstream_states(
    conn: &Connection,
    since_ms: i64,
    selected_relevance_since_ms: i64,
    limit: usize,
    selected_workstream_id: Option<&str>,
    selected_artifact_id: Option<&str>,
) -> Result<Vec<WorkstreamStateFact>, String> {
    if limit == 0 || !table_exists(conn, "continue_workstream_state_snapshots")? {
        return Ok(Vec::new());
    }
    let mut stmt = conn
        .prepare(
            "SELECT id, workstream_id, observed_at_ms, state, previous_state,
                    origin_artifact_id, active_artifact_id,
                    resume_work_target_artifact_id, blocker_artifact_id,
                    confidence, transition_reason, evidence_action_ids_json,
                    missing_evidence_json
             FROM continue_workstream_state_snapshots
             WHERE observed_at_ms >= ?1
                OR (?2 IS NOT NULL AND workstream_id = ?2 AND observed_at_ms >= ?3)
             ORDER BY CASE WHEN workstream_id = ?2 THEN 0 ELSE 1 END,
                      observed_at_ms DESC, confidence DESC, id DESC
             LIMIT ?4",
        )
        .map_err(|error| error.to_string())?;
    let rows = stmt
        .query_map(
            params![
                since_ms,
                selected_workstream_id,
                selected_relevance_since_ms,
                limit as i64
            ],
            |row| {
                let origin_artifact_id = row.get::<_, Option<String>>(5)?;
                let active_artifact_id = row.get::<_, Option<String>>(6)?;
                let resume_artifact_id = row.get::<_, Option<String>>(7)?;
                let blocker_artifact_id = row.get::<_, Option<String>>(8)?;
                Ok((
                    WorkstreamStateFact {
                        snapshot_id: row.get(0)?,
                        workstream_id: row.get(1)?,
                        observed_at_ms: row.get(2)?,
                        state: row.get(3)?,
                        previous_state: row.get(4)?,
                        confidence: row.get(9)?,
                        transition_kind: safe_optional(
                            row.get::<_, Option<String>>(10)?.as_deref(),
                            100,
                        ),
                        has_blocker: blocker_artifact_id.is_some(),
                        evidence_action_ids: parse_string_array(&row.get::<_, String>(11)?)
                            .into_iter()
                            .take(12)
                            .collect(),
                        missing_evidence: safe_string_list(
                            &parse_string_array(&row.get::<_, String>(12)?),
                            12,
                            100,
                        ),
                    },
                    [
                        origin_artifact_id,
                        active_artifact_id,
                        resume_artifact_id,
                        blocker_artifact_id,
                    ],
                ))
            },
        )
        .map_err(|error| error.to_string())?;
    let rows = rows
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    Ok(rows
        .into_iter()
        .filter_map(|(fact, artifact_ids)| {
            artifact_ids
                .iter()
                .flatten()
                .all(|artifact_id| {
                    !artifact_claim_excluded(conn, Some(artifact_id), selected_artifact_id)
                })
                .then_some(fact)
        })
        .collect())
}

fn load_branch_contexts(
    conn: &Connection,
    since_ms: i64,
    selected_relevance_since_ms: i64,
    limit: usize,
    selected_workstream_id: Option<&str>,
    selected_artifact_id: Option<&str>,
) -> Result<Vec<BranchContextFact>, String> {
    if limit == 0 || !table_exists(conn, "continue_branch_contexts")? {
        return Ok(Vec::new());
    }
    let mut stmt = conn
        .prepare(
            "SELECT bc.id, bc.branch_action_id, bc.origin_artifact_id,
                    bc.origin_workstream_id, bc.branch_artifact_id, bc.branch_kind,
                    bc.branch_started_at_ms, bc.last_branch_seen_at_ms,
                    bc.returned_to_origin_at_ms, bc.promotion_state,
                    bc.promotion_reason, bc.confidence, bc.reason_code,
                    bc.evidence_action_ids_json, bc.updated_at_ms,
                    branch.privacy_status, origin.privacy_status
             FROM continue_branch_contexts bc
             LEFT JOIN continue_artifacts branch ON branch.id = bc.branch_artifact_id
             LEFT JOIN continue_artifacts origin ON origin.id = bc.origin_artifact_id
             WHERE bc.last_branch_seen_at_ms >= ?1
                OR (?2 IS NOT NULL AND bc.origin_workstream_id = ?2
                    AND bc.last_branch_seen_at_ms >= ?4)
                OR (?3 IS NOT NULL
                    AND (bc.origin_artifact_id = ?3 OR bc.branch_artifact_id = ?3)
                    AND bc.last_branch_seen_at_ms >= ?4)
             ORDER BY CASE
                        WHEN bc.origin_workstream_id = ?2 OR bc.origin_artifact_id = ?3 THEN 0
                        ELSE 1
                      END,
                      bc.last_branch_seen_at_ms DESC, bc.updated_at_ms DESC, bc.id DESC
             LIMIT ?5",
        )
        .map_err(|error| error.to_string())?;
    let rows = stmt
        .query_map(
            params![
                since_ms,
                selected_workstream_id,
                selected_artifact_id,
                selected_relevance_since_ms,
                limit as i64
            ],
            |row| {
                Ok((
                    BranchContextFact {
                        branch_id: row.get(0)?,
                        branch_action_id: row.get(1)?,
                        origin_artifact_id: row.get(2)?,
                        origin_workstream_id: row.get(3)?,
                        branch_artifact_id: row.get(4)?,
                        branch_kind: row.get(5)?,
                        branch_started_at_ms: row.get(6)?,
                        last_branch_seen_at_ms: row.get(7)?,
                        returned_to_origin_at_ms: row.get(8)?,
                        promotion_state: row.get(9)?,
                        promotion_reason: safe_optional(
                            row.get::<_, Option<String>>(10)?.as_deref(),
                            180,
                        ),
                        confidence: row.get(11)?,
                        reason_code: row.get(12)?,
                        evidence_action_ids: parse_string_array(&row.get::<_, String>(13)?),
                        updated_at_ms: row.get(14)?,
                    },
                    row.get::<_, Option<String>>(15)?,
                    row.get::<_, Option<String>>(16)?,
                ))
            },
        )
        .map_err(|error| error.to_string())?;
    let mut output = Vec::new();
    for row in rows {
        let (fact, branch_privacy, origin_privacy) = row.map_err(|error| error.to_string())?;
        if [branch_privacy, origin_privacy]
            .iter()
            .flatten()
            .any(|status| privacy_blocked(status))
            || artifact_claim_excluded(conn, Some(&fact.branch_artifact_id), selected_artifact_id)
            || artifact_claim_excluded(
                conn,
                fact.origin_artifact_id.as_deref(),
                selected_artifact_id,
            )
        {
            continue;
        }
        output.push(fact);
    }
    Ok(output)
}

fn load_surface_snapshots(
    conn: &Connection,
    since_ms: i64,
    limit: usize,
    selected_artifact_id: Option<&str>,
) -> Result<Vec<SurfaceSnapshotFact>, String> {
    if limit == 0 || !table_exists(conn, "continue_surface_snapshots")? {
        return Ok(Vec::new());
    }
    let mut stmt = conn
        .prepare(
            "SELECT id, artifact_id, frame_id, domain, app_name, bundle_id,
                    window_title, thread_title, active_relative_file, git_branch,
                    activity_state, task_state, command_state, error_markers_json,
                    identity_confidence, evidence_quality, openability, privacy_status,
                    missing_fields_json, evidence_sources_json, observed_at_ms
             FROM continue_surface_snapshots
             WHERE observed_at_ms >= ?1
             ORDER BY observed_at_ms DESC, updated_at_ms DESC, id DESC
             LIMIT ?2",
        )
        .map_err(|error| error.to_string())?;
    let rows = stmt
        .query_map(params![since_ms, limit as i64], |row| {
            let window_title = row.get::<_, Option<String>>(6)?;
            let thread_title = row.get::<_, Option<String>>(7)?;
            Ok((
                SurfaceSnapshotFact {
                    snapshot_id: row.get(0)?,
                    artifact_id: row.get(1)?,
                    frame_id: row.get(2)?,
                    domain: row.get(3)?,
                    app_name: safe_optional(row.get::<_, Option<String>>(4)?.as_deref(), 80),
                    display_title: safe_optional(thread_title.as_deref(), 160)
                        .or_else(|| safe_optional(window_title.as_deref(), 160)),
                    relative_file_name: safe_file_name(row.get::<_, Option<String>>(8)?.as_deref()),
                    git_branch: safe_optional(row.get::<_, Option<String>>(9)?.as_deref(), 100),
                    activity_state: safe_optional(
                        row.get::<_, Option<String>>(10)?.as_deref(),
                        100,
                    ),
                    task_state: safe_optional(row.get::<_, Option<String>>(11)?.as_deref(), 100),
                    command_state: safe_optional(row.get::<_, Option<String>>(12)?.as_deref(), 100),
                    has_error_markers: !parse_string_array(
                        row.get::<_, Option<String>>(13)?.as_deref().unwrap_or("[]"),
                    )
                    .is_empty(),
                    identity_confidence: row.get(14)?,
                    evidence_quality: row.get(15)?,
                    openability: row.get(16)?,
                    missing_evidence: safe_string_list(
                        &parse_string_array(
                            row.get::<_, Option<String>>(18)?.as_deref().unwrap_or("[]"),
                        ),
                        16,
                        120,
                    ),
                    evidence_sources: safe_string_list(
                        &parse_string_array(&row.get::<_, String>(19)?),
                        12,
                        100,
                    ),
                    observed_at_ms: row.get(20)?,
                },
                row.get::<_, Option<String>>(5)?,
                window_title,
                row.get::<_, Option<String>>(17)?,
            ))
        })
        .map_err(|error| error.to_string())?;
    let mut output = Vec::new();
    for row in rows {
        let (fact, bundle_id, raw_window_title, privacy_status) =
            row.map_err(|error| error.to_string())?;
        if privacy_status.as_deref().is_some_and(privacy_blocked) {
            continue;
        }
        let selected_match = fact.artifact_id.as_deref() == selected_artifact_id;
        if (is_smalltalk_self_surface(
            fact.app_name.as_deref(),
            bundle_id.as_deref(),
            raw_window_title.as_deref(),
            None,
            None,
        ) && !selected_match)
            || is_generated_debug_surface(
                fact.app_name.as_deref(),
                raw_window_title.as_deref(),
                None,
                None,
            )
        {
            continue;
        }
        output.push(fact);
    }
    Ok(output)
}

fn map_support_evidence(
    conn: &Connection,
    values: &[ContinueSupportEvidenceItem],
    limit: usize,
) -> Vec<SupportEvidenceFact> {
    values
        .iter()
        .filter(|value| !artifact_privacy_blocked(conn, value.artifact_id.as_deref()))
        .take(limit)
        .map(|value| SupportEvidenceFact {
            artifact_id: value.artifact_id.clone(),
            artifact_kind: value.artifact_kind.clone(),
            display_title: safe_optional(value.title.as_deref(), 160),
            branch_kind: value.branch_kind.clone(),
            origin_artifact_id: value.origin_artifact_id.clone(),
            role: value.role.clone(),
            public_return_eligible: value.public_return_eligible,
            reason: safe_optional(Some(&value.reason), 180),
            evidence_anchor_ids: safe_string_list(&value.evidence_anchor_ids, 16, 120),
        })
        .collect()
}

fn map_memory_facts(
    conn: &Connection,
    report: &ContinueMemoryRetrievalReport,
    limit: usize,
) -> Vec<MemoryFact> {
    let primary = report
        .retrieved_cells
        .iter()
        .map(|value| (value, "support"));
    let counter = report
        .counter_evidence
        .iter()
        .map(|value| (value, "contradiction"));
    primary
        .chain(counter)
        .filter(|(value, _)| {
            value.model_visible
                && !privacy_blocked(value.privacy_status.as_deref().unwrap_or(""))
                && value.redaction_level != "private"
                && !artifact_privacy_blocked(conn, value.artifact_id.as_deref())
        })
        .take(limit)
        .map(|(value, relation)| MemoryFact {
            memory_id: value.id.clone(),
            workstream_id: value.workstream_id.clone(),
            artifact_id: value.artifact_id.clone(),
            episode_id: value.episode_id.clone(),
            action_id: value.action_id.clone(),
            memory_type: value.memory_type.clone(),
            relation: relation.to_string(),
            summary: safe_optional(Some(&value.summary), 220),
            last_seen_at_ms: value.last_seen_at_ms,
            confidence: value.confidence,
            importance: value.importance,
            retrieval_score: value.retrieval_score,
            feedback_score: value.feedback_score,
            retrieval_reasons: safe_string_list(&value.retrieval_reasons, 12, 100),
            supports_candidate_ids: value
                .supports_candidate_ids
                .iter()
                .take(12)
                .cloned()
                .collect(),
            contradicts_candidate_ids: value
                .contradicts_candidate_ids
                .iter()
                .take(12)
                .cloned()
                .collect(),
        })
        .collect()
}

fn existing_quality_facts(context: &ActivityRecapBuildContext<'_>) -> ExistingQualityFacts {
    ExistingQualityFacts {
        p0_quality_signals: context
            .p0_quality_signals
            .and_then(|value| serde_json::to_value(value).ok()),
        current_surface_resolution: context.current_surface_resolution.map(|value| {
            json!({
                "schema": value.schema,
                "selected_surface_id": value.selected.surface_id,
                "row_count": value.row_count,
                "rejected_count": value.top_rejected.len(),
                "latest_any_evidence_ms": value.latest_any_evidence_ms,
                "latest_non_self_evidence_ms": value.latest_non_self_evidence_ms,
                "latest_heavy_frame_ms": value.latest_heavy_frame_ms,
                "latest_event_ms": value.latest_event_ms,
                "warnings": safe_string_list(&value.warnings, 12, 120),
            })
        }),
        evidence_freshness_ledger: context.evidence_freshness_ledger.map(|value| {
            json!({
                "decision_watermark_ms": value.decision_watermark_ms,
                "latest_any_evidence_ms": value.latest_any_evidence_ms,
                "latest_non_self_evidence_ms": value.latest_non_self_evidence_ms,
                "latest_heavy_frame_ms": value.latest_heavy_frame_ms,
                "latest_event_ms": value.latest_event_ms,
                "latest_fresh_openable_ms": value.latest_fresh_openable_ms,
                "selected_candidate_evidence_ms": value.selected_candidate_evidence_ms,
                "selected_candidate_age_ms": value.selected_candidate_age_ms,
                "has_newer_non_self_than_selected": value.has_newer_non_self_than_selected,
                "stale_selected_target": value.stale_selected_target,
                "freshness_reason": safe_optional(Some(&value.freshness_reason), 160),
                "warnings": safe_string_list(&value.warnings, 16, 120),
            })
        }),
        app_activity_summary: context.app_activity_summary.map(|value| {
            json!({
                "main_work": safe_optional(value.main_work.as_deref(), 180),
                "support_context": safe_string_list(&value.support_context, 6, 160),
                "recent_divergence": safe_string_list(&value.recent_divergence, 6, 160),
                "diagnostic_surfaces": safe_string_list(&value.diagnostic_surfaces, 6, 160),
                "missing_for_current_focus": safe_string_list(&value.missing_for_current_focus, 8, 120),
            })
        }),
        quality_gate: context
            .quality_gate
            .and_then(|value| serde_json::to_value(value).ok()),
    }
}

fn artifact_privacy_blocked(conn: &Connection, artifact_id: Option<&str>) -> bool {
    let Some(artifact_id) = artifact_id else {
        return false;
    };
    conn.query_row(
        "SELECT privacy_status FROM continue_artifacts WHERE id = ?1",
        params![artifact_id],
        |row| row.get::<_, Option<String>>(0),
    )
    .optional()
    .ok()
    .flatten()
    .flatten()
    .as_deref()
    .is_some_and(privacy_blocked)
}

fn artifact_claim_excluded(
    conn: &Connection,
    artifact_id: Option<&str>,
    selected_artifact_id: Option<&str>,
) -> bool {
    let Some(artifact_id) = artifact_id else {
        return false;
    };
    let row = conn
        .query_row(
            "SELECT privacy_status, app_name, bundle_id, window_title, browser_url, document_path
             FROM continue_artifacts WHERE id = ?1",
            params![artifact_id],
            |row| {
                Ok((
                    row.get::<_, Option<String>>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, Option<String>>(5)?,
                ))
            },
        )
        .optional()
        .ok()
        .flatten();
    let Some((privacy_status, app_name, bundle_id, window_title, browser_url, document_path)) = row
    else {
        return false;
    };
    privacy_status.as_deref().is_some_and(privacy_blocked)
        || is_generated_debug_surface(
            app_name.as_deref(),
            window_title.as_deref(),
            browser_url.as_deref(),
            document_path.as_deref(),
        )
        || (artifact_id != selected_artifact_id.unwrap_or_default()
            && is_smalltalk_self_surface(
                app_name.as_deref(),
                bundle_id.as_deref(),
                window_title.as_deref(),
                browser_url.as_deref(),
                document_path.as_deref(),
            ))
}

fn privacy_blocked(status: &str) -> bool {
    let normalized = status.to_ascii_lowercase();
    privacy_status_is_limited(&normalized)
        || [
            "never_send_to_ai",
            "privacy_skip",
            "exclude",
            "blocked",
            "sensitive",
        ]
        .iter()
        .any(|needle| normalized.contains(needle))
}

fn safe_optional(value: Option<&str>, max_chars: usize) -> Option<String> {
    value.and_then(|value| sanitize_public_text(value.to_string(), max_chars))
}

fn safe_string_list(values: &[String], limit: usize, max_chars: usize) -> Vec<String> {
    let mut output = Vec::new();
    for value in values {
        if output.len() >= limit {
            break;
        }
        if let Some(value) = safe_optional(Some(value), max_chars) {
            if !output.contains(&value) {
                output.push(value);
            }
        }
    }
    output
}

fn safe_file_name(value: Option<&str>) -> Option<String> {
    value
        .and_then(|value| value.rsplit(['/', '\\']).next())
        .and_then(|value| safe_optional(Some(value), 120))
}

#[cfg(test)]
mod tests {
    use super::super::enrichment::{EnrichmentNeed, WeakSurfaceClassification, WeakSurfaceDomain};
    use super::super::{
        ensure_continue_schema, AppActivityFeatures, ContinueActivityClassification,
        ContinueAppActivitySegment, ContinueMemoryCellSummary, ContinueMemoryRetrievalQuerySummary,
    };
    use super::*;
    use rusqlite::params;

    const NOW_MS: i64 = 100_000;
    const LOOKBACK_MS: i64 = 10_000;

    fn setup() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        ensure_continue_schema(&conn).unwrap();
        conn
    }

    fn empty_memory() -> ContinueMemoryRetrievalReport {
        ContinueMemoryRetrievalReport {
            schema: "smalltalk.continue_memory_retrieval.v1".to_string(),
            generated_at_ms: NOW_MS,
            query: ContinueMemoryRetrievalQuerySummary {
                active_workstream_ids: Vec::new(),
                candidate_workstream_ids: Vec::new(),
                candidate_artifact_ids: Vec::new(),
                current_artifact_id: None,
                current_activity_kind: None,
                unresolved_state_kinds: Vec::new(),
                recent_keywords: Vec::new(),
                lookback_ms: LOOKBACK_MS,
                max_cells: 12,
            },
            retrieved_cells: Vec::new(),
            counter_evidence: Vec::new(),
            missing_evidence: Vec::new(),
            privacy_notes: Vec::new(),
            warnings: Vec::new(),
        }
    }

    fn build_context<'a>(
        app_activity: &'a ContinueAppActivityIntelligence,
        memory: &'a ContinueMemoryRetrievalReport,
    ) -> ActivityRecapBuildContext<'a> {
        ActivityRecapBuildContext {
            decision_id_seed: Some("decision-test"),
            session_id: None,
            mode: "normal",
            lookback_ms: LOOKBACK_MS,
            evidence_watermark: Some("watermark-test"),
            output_mode: Some("thin_continue"),
            requested_at_ms: NOW_MS,
            current_surface: None,
            current_surface_resolution: None,
            selected_workstream: None,
            selected_candidate: None,
            public_return_candidate: None,
            app_activity,
            support_evidence: &[],
            memory_retrieval: memory,
            p0_quality_signals: None,
            evidence_freshness_ledger: None,
            app_activity_summary: Some(&app_activity.summary),
            quality_gate: None,
            caps: ActivityRecapInputCaps::default(),
        }
    }

    fn segment(
        id: &str,
        app_name: &str,
        title: &str,
        artifact_id: Option<&str>,
        started_at_ms: i64,
        ended_at_ms: i64,
    ) -> ContinueAppActivitySegment {
        ContinueAppActivitySegment {
            id: id.to_string(),
            session_id: None,
            app_name: Some(app_name.to_string()),
            bundle_id: None,
            app_family: app_name.to_ascii_lowercase(),
            surface_type: "window".to_string(),
            window_title: Some(title.to_string()),
            artifact_id: artifact_id.map(str::to_string),
            workstream_id: Some("ws-primary".to_string()),
            episode_id: None,
            started_at_ms,
            ended_at_ms,
            evidence_kinds: vec!["window_snapshot".to_string()],
            evidence_anchor: json!({"segment_id": id}),
            features: AppActivityFeatures::default(),
            has_heavy_frame: true,
            has_direct_url: false,
            has_document_path: false,
            has_visible_text: true,
            is_event_backed_only: false,
            is_self_or_diagnostic: false,
        }
    }

    fn classification(
        segment_id: &str,
        intent: &str,
        role: &str,
        updated_at_ms: i64,
    ) -> ContinueActivityClassification {
        ContinueActivityClassification {
            id: format!("classification-{segment_id}"),
            segment_id: segment_id.to_string(),
            session_id: None,
            artifact_id: None,
            workstream_id: Some("ws-primary".to_string()),
            app_family: "test".to_string(),
            surface_type: "window".to_string(),
            activity_intent: intent.to_string(),
            task_phase: "in_progress".to_string(),
            continuation_role: role.to_string(),
            work_value_score: 0.8,
            resume_likelihood_score: 0.8,
            support_score: if role == "support_context" { 0.9 } else { 0.1 },
            divergence_score: if role == "divergence" { 0.9 } else { 0.1 },
            diagnostic_score: 0.0,
            interaction_depth_score: 0.8,
            objective_relation_score: 0.8,
            evidence_sufficiency_score: 0.8,
            reason: format!("{intent} evidence"),
            why_not_primary: None,
            missing_evidence: Vec::new(),
            feature_json: json!({}),
            evidence_anchor_json: json!({"segment_id": segment_id}),
            created_by: "test".to_string(),
            created_at_ms: updated_at_ms,
            updated_at_ms,
        }
    }

    fn app_activity(
        segments: Vec<ContinueAppActivitySegment>,
        classifications: Vec<ContinueActivityClassification>,
    ) -> ContinueAppActivityIntelligence {
        ContinueAppActivityIntelligence {
            schema: "smalltalk.continue_app_activity.v1".to_string(),
            generated_at_ms: NOW_MS,
            segments,
            classifications,
            summary: ContinueActivitySummary::default(),
            warnings: Vec::new(),
        }
    }

    fn current_surface(app_name: &str, title: &str) -> ResolvedCurrentSurface {
        ResolvedCurrentSurface {
            surface_id: "surface-current".to_string(),
            artifact_id: None,
            app_name: Some(app_name.to_string()),
            bundle_id: None,
            window_title: Some(title.to_string()),
            artifact_kind: "unknown".to_string(),
            browser_url: None,
            document_path: None,
            evidence_ids: vec!["window-current".to_string()],
            evidence_kinds: vec!["window_snapshot".to_string()],
            observed_at_ms: NOW_MS,
            latest_non_self_at_ms: Some(NOW_MS),
            latest_heavy_frame_at_ms: None,
            latest_event_at_ms: Some(NOW_MS),
            is_self_surface: false,
            is_generated_debug_surface: false,
            focus_confidence: 0.6,
            evidence_quality: "thin".to_string(),
            openability: "unknown".to_string(),
            domain: None,
            activity_state: None,
            task_state: None,
            identity_confidence: None,
            snapshot_id: None,
            missing_fields: vec!["missing_active_content".to_string()],
            weak_surface_classification: WeakSurfaceClassification {
                domain: WeakSurfaceDomain::NotWeakSurface,
                enrichment_need: EnrichmentNeed::None,
                confidence: 0.6,
                reasons: Vec::new(),
                adapter_key: None,
                privacy_tier: "normal".to_string(),
                observed_app_name: Some(app_name.to_string()),
                observed_bundle_id: None,
                observed_window_title: Some(title.to_string()),
            },
            reason: "latest active window".to_string(),
            warnings: Vec::new(),
        }
    }

    fn insert_artifact(
        conn: &Connection,
        id: &str,
        kind: &str,
        title: &str,
        privacy_status: Option<&str>,
    ) {
        conn.execute(
            "INSERT INTO continue_artifacts (
                id, artifact_kind, stable_key, display_title,
                first_seen_timestamp, last_seen_timestamp, identity_confidence,
                evidence_quality, privacy_status, openability, created_at_ms, updated_at_ms
             ) VALUES (?1, ?2, ?1, ?3, ?4, ?4, 0.9, 'strong', ?5, 'inspectable', ?4, ?4)",
            params![id, kind, title, NOW_MS, privacy_status],
        )
        .unwrap();
    }

    fn insert_workstream(conn: &Connection, id: &str) {
        conn.execute(
            "INSERT OR IGNORE INTO continue_workstreams (
                id, state, title_candidate, created_at_ms, last_active_timestamp_ms,
                confidence, unresolved_signal, source
             ) VALUES (?1, 'active', 'Primary work', ?2, ?2, 0.9, 'unfinished', 'test')",
            params![id, NOW_MS],
        )
        .unwrap();
    }

    fn insert_action(
        conn: &Connection,
        id: &str,
        artifact_id: Option<&str>,
        action_kind: &str,
        action_role: &str,
        created_at_ms: i64,
        subject: Option<&str>,
        after_hint: Option<&str>,
    ) {
        conn.execute(
            "INSERT INTO continue_task_actions (
                id, frame_id, artifact_id, action_kind, action_role,
                evidence_event_ids_json, confidence, created_at_ms,
                semantic_subject, semantic_after_hint, semantic_evidence_quote,
                evidence_span_ids_json, quality_flags_json
             ) VALUES (?1, ?1, ?2, ?3, ?4, '[]', 0.9, ?5, ?6, ?7,
                       'raw output sk-should-not-leak /Users/private/file', '[]', '[]')",
            params![
                id,
                artifact_id,
                action_kind,
                action_role,
                created_at_ms,
                subject,
                after_hint
            ],
        )
        .unwrap();
    }

    fn insert_moment(
        conn: &Connection,
        id: &str,
        artifact_id: Option<&str>,
        ended_at_ms: i64,
        boundary_kind: &str,
        summary: Option<&str>,
    ) {
        conn.execute(
            "INSERT INTO continue_semantic_moments (
                id, started_at_ms, ended_at_ms, dominant_artifact_id,
                dominant_event_type, ordered_event_ids_json, causal_chain_json,
                boundary_kind, semantic_summary, evidence_quality, created_at_ms
             ) VALUES (?1, ?2, ?3, ?4, 'content_change', '[]', '[]', ?5, ?6, 'strong', ?3)",
            params![
                id,
                ended_at_ms - 100,
                ended_at_ms,
                artifact_id,
                boundary_kind,
                summary
            ],
        )
        .unwrap();
    }

    fn insert_open_loop(
        conn: &Connection,
        id: &str,
        workstream_id: &str,
        artifact_id: Option<&str>,
        blocker_artifact_id: Option<&str>,
        last_updated_at_ms: i64,
    ) {
        insert_workstream(conn, workstream_id);
        conn.execute(
            "INSERT INTO continue_open_loops (
                id, workstream_id, state, boundary_kind, quality, confidence,
                origin_artifact_id, primary_return_artifact_id, blocker_artifact_id,
                objective_hint, last_concrete_progress, unfinished_state,
                next_evidence_backed_action, current_focus_relation,
                missing_fields_json, evidence_spans_json, last_updated_at_ms, revision
             ) VALUES (?1, ?2, 'open', 'unfinished_progress', 'strong', 0.9,
                       ?3, ?3, ?4, 'Finish the bounded input layer',
                       'Added graph facts', 'Tests are unfinished',
                       'Run the focused tests', 'same_workstream', '[]', '[]', ?5, 1)",
            params![
                id,
                workstream_id,
                artifact_id,
                blocker_artifact_id,
                last_updated_at_ms
            ],
        )
        .unwrap();
    }

    fn insert_branch(
        conn: &Connection,
        id: &str,
        action_id: &str,
        origin_artifact_id: Option<&str>,
        branch_artifact_id: &str,
        last_seen_at_ms: i64,
    ) {
        conn.execute(
            "INSERT OR IGNORE INTO continue_artifacts (
                id, artifact_kind, stable_key, display_title,
                first_seen_timestamp, last_seen_timestamp, identity_confidence,
                evidence_quality, openability, created_at_ms, updated_at_ms
             ) VALUES (?1, 'browser_tab', ?1, 'Support surface', ?2, ?2,
                       0.8, 'medium', 'inspectable', ?2, ?2)",
            params![branch_artifact_id, last_seen_at_ms],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO continue_branch_contexts (
                id, branch_action_id, origin_artifact_id, origin_workstream_id,
                branch_artifact_id, branch_kind, branch_started_at_ms,
                last_branch_seen_at_ms, promotion_state, confidence,
                evidence_action_ids_json, created_at_ms, updated_at_ms
             ) VALUES (?1, ?2, ?3, 'ws-primary', ?4, 'support_lookup', ?5, ?5,
                       'unpromoted', 0.8, '[]', ?5, ?5)",
            params![
                id,
                action_id,
                origin_artifact_id,
                branch_artifact_id,
                last_seen_at_ms
            ],
        )
        .unwrap();
    }

    fn insert_snapshot(
        conn: &Connection,
        id: &str,
        artifact_id: Option<&str>,
        observed_at_ms: i64,
        privacy_status: Option<&str>,
        missing: &[&str],
    ) {
        conn.execute(
            "INSERT INTO continue_surface_snapshots (
                id, surface_key, domain, adapter_version, observed_at_ms,
                artifact_id, app_name, window_title, workspace_path,
                repo_root_path, thread_title, active_file_path, active_relative_file,
                visible_text_sample, activity_state, task_state, command_state,
                error_markers_json, identity_confidence, evidence_quality, openability,
                privacy_status, missing_fields_json, evidence_sources_json,
                created_at_ms, updated_at_ms
             ) VALUES (?1, ?1, 'codex_desktop_app', 'test.v1', ?2, ?3, 'Codex',
                       'Smalltalk work', '/Users/private/workspace', '/Users/private/repo',
                       'P5 recap inputs', '/Users/private/src/continuation.rs',
                       'src/continuation.rs', 'raw visible secret', 'composing',
                       'implementation', 'idle', '[\"error\"]', 'medium', 'medium',
                       'inspectable', ?4, ?5, '[\"accessibility\"]', ?2, ?2)",
            params![
                id,
                observed_at_ms,
                artifact_id,
                privacy_status,
                json!(missing).to_string()
            ],
        )
        .unwrap();
    }

    fn memory_cell(id: &str, relation: &str) -> ContinueMemoryCellSummary {
        ContinueMemoryCellSummary {
            id: id.to_string(),
            workstream_id: Some("ws-primary".to_string()),
            artifact_id: None,
            episode_id: None,
            action_id: None,
            memory_type: relation.to_string(),
            summary: "Evidence-backed prior context".to_string(),
            keywords: Vec::new(),
            tags: Vec::new(),
            source_anchor: json!({}),
            last_seen_at_ms: NOW_MS,
            confidence: 0.8,
            importance: 0.8,
            decay_score: 0.9,
            privacy_status: None,
            redaction_level: "safe".to_string(),
            feedback_score: 0.0,
            retrieval_score: 0.8,
            retrieval_reasons: vec!["graph_match".to_string()],
            supports_candidate_ids: Vec::new(),
            contradicts_candidate_ids: Vec::new(),
            model_visible: true,
        }
    }

    #[test]
    fn inputs_chat_composing_keeps_finder_as_detour() {
        let conn = setup();
        insert_artifact(
            &conn,
            "chat",
            "chat_thread",
            "Smalltalk research chat",
            None,
        );
        insert_artifact(&conn, "finder", "finder", "Photos", None);
        insert_action(
            &conn,
            "compose-action",
            Some("chat"),
            "composing",
            "primary_progress",
            NOW_MS - 500,
            Some("P5 evidence graph plan"),
            Some("Writing the bounded input layer"),
        );
        insert_action(
            &conn,
            "finder-action",
            Some("finder"),
            "navigating",
            "support",
            NOW_MS - 200,
            Some("Photos"),
            None,
        );
        insert_open_loop(
            &conn,
            "loop-chat",
            "ws-primary",
            Some("chat"),
            None,
            NOW_MS - 300,
        );
        let activity = app_activity(
            vec![
                segment(
                    "chat-segment",
                    "ChatGPT",
                    "Smalltalk research chat",
                    Some("chat"),
                    NOW_MS - 2_000,
                    NOW_MS - 600,
                ),
                segment(
                    "finder-segment",
                    "Finder",
                    "Photos",
                    Some("finder"),
                    NOW_MS - 500,
                    NOW_MS,
                ),
            ],
            vec![
                classification("chat-segment", "composing", "resume_target", NOW_MS - 500),
                classification("finder-segment", "browsing", "divergence", NOW_MS),
            ],
        );
        let memory = empty_memory();

        let inputs = build_activity_recap_inputs(&conn, build_context(&activity, &memory));

        assert_eq!(inputs.recent_segments.len(), 2);
        assert!(inputs
            .recent_segments
            .iter()
            .any(|segment| segment.segment_id == "chat-segment"
                && segment.activity_intent.as_deref() == Some("composing")));
        assert!(inputs
            .recent_segments
            .iter()
            .any(|segment| segment.segment_id == "finder-segment"
                && segment.continuation_role.as_deref() == Some("divergence")));
        assert_eq!(inputs.recent_actions.len(), 2);
        assert_eq!(inputs.open_loops[0].open_loop_id, "loop-chat");
    }

    #[test]
    fn inputs_editor_primary_includes_unpromoted_docs_search_support() {
        let conn = setup();
        for (id, kind, title) in [
            ("editor", "code_editor", "continuation.rs"),
            ("docs", "browser_tab", "Rust documentation"),
            ("search", "browser_tab", "Search results"),
        ] {
            insert_artifact(&conn, id, kind, title, None);
        }
        insert_workstream(&conn, "ws-primary");
        insert_action(
            &conn,
            "edit-action",
            Some("editor"),
            "editing",
            "primary_progress",
            NOW_MS - 900,
            Some("recap inputs"),
            None,
        );
        insert_action(
            &conn,
            "docs-action",
            Some("docs"),
            "reading",
            "support",
            NOW_MS - 600,
            None,
            None,
        );
        insert_action(
            &conn,
            "search-action",
            Some("search"),
            "searching",
            "support",
            NOW_MS - 300,
            None,
            None,
        );
        insert_branch(
            &conn,
            "branch-docs",
            "docs-action",
            Some("editor"),
            "docs",
            NOW_MS - 600,
        );
        insert_branch(
            &conn,
            "branch-search",
            "search-action",
            Some("editor"),
            "search",
            NOW_MS - 300,
        );
        insert_open_loop(
            &conn,
            "loop-editor",
            "ws-primary",
            Some("editor"),
            None,
            NOW_MS - 200,
        );
        let activity = app_activity(
            vec![
                segment(
                    "editor-segment",
                    "Code",
                    "continuation.rs",
                    Some("editor"),
                    NOW_MS - 2_000,
                    NOW_MS - 1_000,
                ),
                segment(
                    "docs-segment",
                    "Browser",
                    "Rust documentation",
                    Some("docs"),
                    NOW_MS - 900,
                    NOW_MS - 600,
                ),
                segment(
                    "search-segment",
                    "Browser",
                    "Search results",
                    Some("search"),
                    NOW_MS - 500,
                    NOW_MS - 200,
                ),
            ],
            vec![
                classification("editor-segment", "editing", "resume_target", NOW_MS - 1_000),
                classification("docs-segment", "reading", "support_context", NOW_MS - 600),
                classification(
                    "search-segment",
                    "searching",
                    "support_context",
                    NOW_MS - 200,
                ),
            ],
        );
        let memory = empty_memory();

        let inputs = build_activity_recap_inputs(&conn, build_context(&activity, &memory));

        assert_eq!(inputs.recent_segments.len(), 3);
        assert_eq!(inputs.recent_actions.len(), 3);
        assert_eq!(inputs.branch_contexts.len(), 2);
        assert!(inputs
            .branch_contexts
            .iter()
            .all(|branch| branch.promotion_state == "unpromoted"));
        assert_eq!(
            inputs.open_loops[0].primary_return_artifact_id.as_deref(),
            Some("editor")
        );
    }

    #[test]
    fn inputs_terminal_error_preserves_blocker_without_raw_output() {
        let conn = setup();
        insert_artifact(&conn, "terminal", "terminal", "Smalltalk tests", None);
        insert_action(
            &conn,
            "command-action",
            Some("terminal"),
            "running_command",
            "primary_progress",
            NOW_MS - 700,
            Some("cargo test"),
            None,
        );
        insert_action(
            &conn,
            "error-action",
            Some("terminal"),
            "encountering_error",
            "blocker",
            NOW_MS - 400,
            Some("compile error"),
            Some("token sk-should-not-leak"),
        );
        insert_moment(
            &conn,
            "error-moment",
            Some("terminal"),
            NOW_MS - 300,
            "error_without_resolution",
            Some("Build stopped at a compile error"),
        );
        insert_open_loop(
            &conn,
            "loop-terminal",
            "ws-primary",
            Some("terminal"),
            Some("terminal"),
            NOW_MS - 200,
        );
        let activity = app_activity(
            vec![segment(
                "terminal-segment",
                "Terminal",
                "Smalltalk tests",
                Some("terminal"),
                NOW_MS - 1_000,
                NOW_MS,
            )],
            vec![classification(
                "terminal-segment",
                "running_command",
                "resume_target",
                NOW_MS,
            )],
        );
        let memory = empty_memory();

        let inputs = build_activity_recap_inputs(&conn, build_context(&activity, &memory));
        let serialized = serde_json::to_string(&inputs).unwrap();

        assert!(inputs
            .recent_actions
            .iter()
            .any(|action| action.action_id == "error-action"));
        assert_eq!(inputs.recent_moments[0].moment_id, "error-moment");
        assert_eq!(
            inputs.open_loops[0].blocker_artifact_id.as_deref(),
            Some("terminal")
        );
        assert!(!serialized.contains("sk-should-not-leak"));
        assert!(!serialized.contains("/Users/private"));
        assert!(!serialized.contains("raw output"));
    }

    #[test]
    fn inputs_weak_surface_uses_snapshot_and_missing_evidence() {
        let conn = setup();
        insert_artifact(&conn, "codex", "code_editor", "Codex task", None);
        insert_snapshot(
            &conn,
            "snapshot-safe",
            Some("codex"),
            NOW_MS - 100,
            None,
            &["exact_thread_target", "active_file"],
        );
        insert_snapshot(
            &conn,
            "snapshot-private",
            Some("codex"),
            NOW_MS,
            Some("privacy_skip"),
            &["privacy_blocked_text"],
        );
        let mut event_only = segment(
            "codex-segment",
            "Codex",
            "Codex task",
            Some("codex"),
            NOW_MS - 500,
            NOW_MS,
        );
        event_only.has_heavy_frame = false;
        event_only.has_visible_text = false;
        event_only.is_event_backed_only = true;
        let activity = app_activity(
            vec![event_only],
            vec![classification(
                "codex-segment",
                "composing",
                "needs_fresh_capture",
                NOW_MS,
            )],
        );
        let memory = empty_memory();

        let inputs = build_activity_recap_inputs(&conn, build_context(&activity, &memory));
        let serialized = serde_json::to_string(&inputs).unwrap();

        assert_eq!(inputs.surface_snapshots.len(), 1);
        assert_eq!(inputs.surface_snapshots[0].snapshot_id, "snapshot-safe");
        assert_eq!(
            inputs.surface_snapshots[0].relative_file_name.as_deref(),
            Some("continuation.rs")
        );
        assert!(inputs.surface_snapshots[0]
            .missing_evidence
            .contains(&"exact_thread_target".to_string()));
        assert!(!serialized.contains("workspace_path"));
        assert!(!serialized.contains("/Users/private"));
        assert!(!serialized.contains("raw visible secret"));
    }

    #[test]
    fn inputs_generic_events_remain_thin() {
        let conn = setup();
        let activity = app_activity(
            vec![{
                let mut value = segment(
                    "generic-segment",
                    "Unknown App",
                    "Untitled",
                    None,
                    NOW_MS - 100,
                    NOW_MS,
                );
                value.has_heavy_frame = false;
                value.has_visible_text = false;
                value.is_event_backed_only = true;
                value
            }],
            vec![classification(
                "generic-segment",
                "unknown",
                "needs_fresh_capture",
                NOW_MS,
            )],
        );
        let memory = empty_memory();
        let surface = current_surface("Unknown App", "Untitled");
        let mut context = build_context(&activity, &memory);
        context.current_surface = Some(&surface);

        let inputs = build_activity_recap_inputs(&conn, context);

        assert_eq!(inputs.recent_segments.len(), 1);
        assert!(inputs.recent_actions.is_empty());
        assert!(inputs.recent_moments.is_empty());
        assert!(inputs.open_loops.is_empty());
        assert!(inputs.branch_contexts.is_empty());
        assert!(inputs.surface_snapshots.is_empty());
        assert!(inputs.memory_facts.is_empty());
        assert!(inputs.selected_candidate.is_none());
        assert!(inputs.return_target.is_none());
        assert!(inputs.resume_work_target.is_none());
        assert_eq!(
            inputs
                .current_surface
                .as_ref()
                .and_then(|surface| surface.display_title.as_deref()),
            Some("Untitled")
        );
    }

    #[test]
    fn inputs_enforce_every_cap_lookback_and_read_only_contract() {
        let conn = setup();
        let caps = ActivityRecapInputCaps::default();
        assert_eq!(caps.max_activity_segments, 12);
        assert_eq!(caps.max_task_actions, 40);
        assert_eq!(caps.max_semantic_moments, 30);
        assert_eq!(caps.max_open_loops, 8);
        assert_eq!(caps.max_workstream_states, 8);
        assert_eq!(caps.max_branch_contexts, 8);
        assert_eq!(caps.max_surface_snapshots, 8);
        assert_eq!(caps.max_memory_cells, 12);
        assert_eq!(caps.max_support_items, 12);

        insert_workstream(&conn, "ws-primary");
        let mut segments = Vec::new();
        let mut classifications = Vec::new();
        for index in 0..15 {
            let id = format!("segment-{index:02}");
            segments.push(segment(
                &id,
                "Code",
                "Project",
                None,
                NOW_MS - 500 + index,
                NOW_MS - 400 + index,
            ));
            classifications.push(classification(
                &id,
                "editing",
                "resume_target",
                NOW_MS + index,
            ));
        }
        segments.push(segment(
            "segment-before-window",
            "Code",
            "Old project",
            None,
            NOW_MS - LOOKBACK_MS - 200,
            NOW_MS - LOOKBACK_MS - 100,
        ));
        classifications.push(classification(
            "segment-before-window",
            "editing",
            "resume_target",
            NOW_MS - LOOKBACK_MS - 100,
        ));

        for index in 0..43 {
            insert_action(
                &conn,
                &format!("action-{index:02}"),
                None,
                "editing",
                "primary_progress",
                NOW_MS - 500 + index,
                None,
                None,
            );
        }
        insert_action(
            &conn,
            "action-before-window",
            None,
            "editing",
            "primary_progress",
            NOW_MS - LOOKBACK_MS - 1,
            None,
            None,
        );
        for index in 0..33 {
            insert_moment(
                &conn,
                &format!("moment-{index:02}"),
                None,
                NOW_MS - 500 + index,
                "content_change",
                None,
            );
        }
        insert_moment(
            &conn,
            "moment-before-window",
            None,
            NOW_MS - LOOKBACK_MS - 1,
            "content_change",
            None,
        );
        for index in 0..11 {
            insert_open_loop(
                &conn,
                &format!("loop-{index:02}"),
                &format!("ws-{index:02}"),
                None,
                None,
                NOW_MS - 500 + index,
            );
            insert_branch(
                &conn,
                &format!("branch-{index:02}"),
                &format!("action-{index:02}"),
                None,
                &format!("branch-artifact-{index:02}"),
                NOW_MS - 500 + index,
            );
            insert_snapshot(
                &conn,
                &format!("snapshot-{index:02}"),
                None,
                NOW_MS - 500 + index,
                None,
                &[],
            );
            conn.execute(
                "INSERT INTO continue_workstream_state_snapshots (
                    id, workstream_id, observed_at_ms, state, previous_state,
                    origin_artifact_id, active_artifact_id,
                    resume_work_target_artifact_id, last_support_artifact_id,
                    blocker_artifact_id, confidence, transition_reason,
                    evidence_action_ids_json, evidence_artifact_ids_json,
                    missing_evidence_json, warnings_json, created_at_ms
                 ) VALUES (?1, ?2, ?3, 'editing', NULL, NULL, NULL, NULL, NULL,
                           NULL, 0.9, 'latest_action_editing', '[]', '[]', '[]', '[]', ?3)",
                params![
                    format!("workstream-state-{index:02}"),
                    format!("ws-{index:02}"),
                    NOW_MS - 500 + index,
                ],
            )
            .unwrap();
        }

        let activity = app_activity(segments, classifications);
        let mut memory = empty_memory();
        memory.retrieved_cells = (0..15)
            .map(|index| memory_cell(&format!("memory-{index:02}"), "origin_intent"))
            .collect();
        let support = (0..15)
            .map(|index| ContinueSupportEvidenceItem {
                artifact_id: None,
                artifact_kind: Some("browser_tab".to_string()),
                title: Some(format!("Support {index:02}")),
                branch_kind: "support_lookup".to_string(),
                origin_artifact_id: None,
                role: "support".to_string(),
                public_return_eligible: false,
                reason: "bounded support evidence".to_string(),
                evidence_anchor_ids: vec![format!("support-anchor-{index:02}")],
            })
            .collect::<Vec<_>>();
        let mut context = build_context(&activity, &memory);
        context.support_evidence = &support;
        let changes_before = conn.total_changes();

        let inputs = build_activity_recap_inputs(&conn, context);

        assert_eq!(conn.total_changes(), changes_before);
        assert_eq!(inputs.recent_segments.len(), caps.max_activity_segments);
        assert_eq!(inputs.recent_actions.len(), caps.max_task_actions);
        assert_eq!(inputs.recent_moments.len(), caps.max_semantic_moments);
        assert_eq!(inputs.open_loops.len(), caps.max_open_loops);
        assert_eq!(inputs.workstream_states.len(), caps.max_workstream_states);
        assert_eq!(inputs.branch_contexts.len(), caps.max_branch_contexts);
        assert_eq!(inputs.surface_snapshots.len(), caps.max_surface_snapshots);
        assert_eq!(inputs.memory_facts.len(), caps.max_memory_cells);
        assert_eq!(inputs.support_evidence.len(), caps.max_support_items);
        let serialized = serde_json::to_string(&inputs).unwrap();
        assert!(!serialized.contains("before-window"));
    }

    #[test]
    fn inputs_exclude_private_and_diagnostic_evidence() {
        let conn = setup();
        insert_artifact(
            &conn,
            "private-artifact",
            "code_editor",
            "Private work",
            Some("never_send_to_ai"),
        );
        insert_action(
            &conn,
            "private-action",
            Some("private-artifact"),
            "editing",
            "primary_progress",
            NOW_MS,
            Some("private material"),
            None,
        );
        insert_moment(
            &conn,
            "private-moment",
            Some("private-artifact"),
            NOW_MS,
            "content_change",
            Some("private material"),
        );
        insert_open_loop(
            &conn,
            "private-loop",
            "ws-private",
            Some("private-artifact"),
            None,
            NOW_MS,
        );
        insert_snapshot(
            &conn,
            "private-snapshot",
            Some("private-artifact"),
            NOW_MS,
            Some("sensitive"),
            &[],
        );
        let mut diagnostic = segment(
            "diagnostic-segment",
            "Smalltalk",
            "Smalltalk Continue",
            None,
            NOW_MS - 100,
            NOW_MS,
        );
        diagnostic.is_self_or_diagnostic = true;
        let activity = app_activity(
            vec![diagnostic],
            vec![classification(
                "diagnostic-segment",
                "diagnostic",
                "diagnostic_only",
                NOW_MS,
            )],
        );
        let mut memory = empty_memory();
        let mut private_memory = memory_cell("private-memory", "origin_intent");
        private_memory.privacy_status = Some("privacy_skip".to_string());
        private_memory.artifact_id = Some("private-artifact".to_string());
        memory.retrieved_cells.push(private_memory);

        let inputs = build_activity_recap_inputs(&conn, build_context(&activity, &memory));

        assert!(inputs.recent_segments.is_empty());
        assert!(inputs.recent_actions.is_empty());
        assert!(inputs.recent_moments.is_empty());
        assert!(inputs.open_loops.is_empty());
        assert!(inputs.surface_snapshots.is_empty());
        assert!(inputs.memory_facts.is_empty());
    }
}
