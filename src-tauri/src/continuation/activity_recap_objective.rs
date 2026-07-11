use super::activity_recap::{
    sanitize_public_text, ActivityConfidence, ActivityEvidenceAnchorType,
    ActivityEvidenceConfidence, ActivityEvidenceSource, ActivityEvidenceSpan,
    ActivityRecapValidationStatus, ContinueActivityRecap,
};
use super::activity_recap_inputs::{
    ActivityRecapInputs, OpenLoopFact, SurfaceSnapshotFact, TaskActionFact,
};
use super::activity_recap_segments::{
    ActivitySegmentRole, StitchedActivitySegment, StitchedActivityTimeline,
};
use serde::{Deserialize, Serialize};
use std::cmp::Reverse;

const MAX_OBJECTIVE_TERMS: usize = 8;
const MAX_REJECTED_TERMS: usize = 16;

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ActivityWorkLabelKind {
    Compose,
    Review,
    Debug,
    Research,
    Browse,
    FileBrowse,
    Communicate,
    Code,
    Terminal,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct RejectedObjectiveTerm {
    pub term: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ActivityWorkLabelResult {
    pub primary_work_label: Option<String>,
    pub primary_work_label_kind: ActivityWorkLabelKind,
    pub primary_work_summary_seed: Option<String>,
    pub where_label: Option<String>,
    pub object_label: Option<String>,
    pub objective_terms: Vec<String>,
    pub selected_objective_source: Option<ObjectiveTermAudit>,
    pub objective_candidate_audit: Vec<ObjectiveTermAudit>,
    pub rejected_terms: Vec<RejectedObjectiveTerm>,
    pub confidence: ActivityConfidence,
    pub evidence_spans: Vec<ActivityEvidenceSpan>,
    pub missing_evidence: Vec<String>,
}

impl Default for ActivityWorkLabelResult {
    fn default() -> Self {
        Self {
            primary_work_label: None,
            primary_work_label_kind: ActivityWorkLabelKind::Unknown,
            primary_work_summary_seed: None,
            where_label: None,
            object_label: None,
            objective_terms: Vec::new(),
            selected_objective_source: None,
            objective_candidate_audit: Vec::new(),
            rejected_terms: Vec::new(),
            confidence: ActivityConfidence::None,
            evidence_spans: Vec::new(),
            missing_evidence: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TermSource {
    CurrentTaskGoal,
    OpenLoop,
    ActionSubject,
    Workstream,
    Memory,
    SurfaceTask,
    RelativeFile,
    SurfaceTitle,
    TargetTitle,
}

impl TermSource {
    fn as_str(self) -> &'static str {
        match self {
            Self::CurrentTaskGoal => "current_task_goal",
            Self::OpenLoop => "open_loop",
            Self::ActionSubject => "current_task_action_subject",
            Self::Workstream => "selected_workstream",
            Self::Memory => "memory",
            Self::SurfaceTask => "surface_task",
            Self::RelativeFile => "relative_file",
            Self::SurfaceTitle => "surface_title",
            Self::TargetTitle => "target_title",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ObjectiveTermAudit {
    pub term: String,
    pub source_kind: String,
    pub source_id: Option<String>,
    pub task_turn_id: Option<String>,
    pub workstream_id: Option<String>,
    pub eligible: bool,
    pub eligibility_reason_codes: Vec<String>,
    pub attribution_confidence: ActivityEvidenceConfidence,
    pub freshness: String,
}

#[derive(Debug, Clone)]
struct TermCandidate {
    value: String,
    source: TermSource,
    priority: u8,
    confidence: ActivityEvidenceConfidence,
    anchor_type: ActivityEvidenceAnchorType,
    anchor_ids: Vec<String>,
    task_turn_id: Option<String>,
    workstream_id: Option<String>,
    eligibility_reason_codes: Vec<String>,
    freshness: String,
}

#[derive(Debug, Clone, Copy, Default)]
struct KindSignals {
    composing: bool,
    planning: bool,
    reviewing: bool,
    debugging: bool,
    reading_docs: bool,
    browsing: bool,
    file_browsing: bool,
    communicating: bool,
    coding: bool,
    terminal: bool,
    has_action: bool,
    has_error: bool,
}

/// Infers a short, deterministic description of the primary work represented by
/// the stitched timeline. Only productized, privacy-scrubbed fields from
/// `ActivityRecapInputs` are considered. Branch and detour roles remain
/// authoritative: a support or detour surface cannot become the primary label.
pub(crate) fn infer_activity_work_labels(
    inputs: &ActivityRecapInputs,
    timeline: &StitchedActivityTimeline,
) -> ActivityWorkLabelResult {
    let mut result = ActivityWorkLabelResult::default();
    let Some(primary) = timeline.primary_segment.as_ref() else {
        audit_non_primary_terms(timeline, &mut result.rejected_terms);
        result
            .missing_evidence
            .push("No primary activity segment was grounded.".to_string());
        return result;
    };

    if !primary_role_is_eligible(primary.role) {
        audit_rejected(
            &mut result.rejected_terms,
            primary.surface_title.as_deref(),
            "non_primary_activity_role",
        );
        result
            .missing_evidence
            .push("The available activity is support or a detour, not primary work.".to_string());
        return result;
    }

    audit_non_primary_terms(timeline, &mut result.rejected_terms);
    let actions = matching_primary_actions(inputs, primary);
    let snapshots = matching_primary_snapshots(inputs, primary);
    let signals = classify_kind_signals(inputs, primary, &actions, &snapshots);
    let kind = select_kind(signals);
    let where_label = infer_where_label(inputs, primary, &snapshots);
    let mut candidates = collect_term_candidates(
        inputs,
        primary,
        &actions,
        &snapshots,
        &mut result.rejected_terms,
    );
    candidates.sort_by_key(|candidate| Reverse(candidate.priority));
    dedupe_candidates(&mut candidates);

    let object_candidate = candidates.first().cloned();
    result.objective_candidate_audit = candidates
        .iter()
        .take(MAX_OBJECTIVE_TERMS)
        .map(objective_term_audit)
        .collect();
    result.selected_objective_source = object_candidate.as_ref().map(objective_term_audit);
    let object_label = object_candidate
        .as_ref()
        .and_then(|candidate| normalize_object_label(&candidate.value));
    let planning = signals.planning
        || object_label
            .as_deref()
            .is_some_and(contains_planning_language);
    let label = build_work_label(
        kind,
        planning,
        object_label.as_deref(),
        where_label.as_deref(),
    );
    let contradictions = has_primary_contradiction(inputs, primary);
    let confidence = label_confidence(
        primary,
        signals,
        object_candidate.as_ref(),
        where_label.as_deref(),
        contradictions,
    );

    result.primary_work_label_kind = kind;
    result.primary_work_label = label;
    result.where_label = where_label;
    result.object_label = object_label;
    result.objective_terms = candidates
        .iter()
        .take(MAX_OBJECTIVE_TERMS)
        .map(|candidate| candidate.value.clone())
        .collect();
    result.confidence = confidence;

    if let Some(label) = result.primary_work_label.as_deref() {
        result.primary_work_summary_seed = Some(summary_seed(
            label,
            result.where_label.as_deref(),
            result.object_label.as_deref(),
        ));
        result.evidence_spans = label_evidence_spans(
            label,
            confidence,
            &actions,
            object_candidate.as_ref(),
            &snapshots,
        );
    }

    if !signals.has_action {
        result
            .missing_evidence
            .push("No direct action kind grounds the activity label.".to_string());
    }
    if result.object_label.is_none() {
        result
            .missing_evidence
            .push("The exact work subject is not grounded.".to_string());
    }
    if result.where_label.is_none() {
        result
            .missing_evidence
            .push("The work surface is not identified.".to_string());
    }
    if contradictions {
        result
            .missing_evidence
            .push("Local memory contains contradictory objective evidence.".to_string());
    }
    dedupe_strings(&mut result.missing_evidence);
    result
}

fn objective_term_audit(candidate: &TermCandidate) -> ObjectiveTermAudit {
    ObjectiveTermAudit {
        term: candidate.value.clone(),
        source_kind: candidate.source.as_str().to_string(),
        source_id: candidate.anchor_ids.first().cloned(),
        task_turn_id: candidate.task_turn_id.clone(),
        workstream_id: candidate.workstream_id.clone(),
        eligible: true,
        eligibility_reason_codes: candidate.eligibility_reason_codes.clone(),
        attribution_confidence: candidate.confidence,
        freshness: candidate.freshness.clone(),
    }
}

/// Applies only the P5-04 fields to the existing recap contract. Target
/// confidence, open-loop state, detours, and target explanations belong to
/// later recap stages and intentionally remain untouched.
pub(crate) fn recap_with_activity_work_label(
    result: ActivityWorkLabelResult,
) -> ContinueActivityRecap {
    if result.primary_work_label.is_none() {
        return ContinueActivityRecap::default();
    }

    ContinueActivityRecap {
        primary_work_summary: result.primary_work_summary_seed,
        primary_work_label: result.primary_work_label,
        primary_where_summary: result.where_label,
        activity_confidence: result.confidence,
        missing_evidence: result.missing_evidence,
        evidence_spans: result.evidence_spans,
        validation_status: if matches!(
            result.confidence,
            ActivityConfidence::Medium | ActivityConfidence::High
        ) {
            ActivityRecapValidationStatus::Valid
        } else {
            ActivityRecapValidationStatus::Thin
        },
        ..ContinueActivityRecap::default()
    }
    .sanitized()
}

fn primary_role_is_eligible(role: ActivitySegmentRole) -> bool {
    matches!(
        role,
        ActivitySegmentRole::Primary
            | ActivitySegmentRole::PromotedPrimary
            | ActivitySegmentRole::Return
    )
}

fn matching_primary_actions<'a>(
    inputs: &'a ActivityRecapInputs,
    primary: &StitchedActivitySegment,
) -> Vec<&'a TaskActionFact> {
    let Some(artifact_id) = primary.artifact_id.as_deref() else {
        return Vec::new();
    };
    inputs
        .recent_actions
        .iter()
        .filter(|action| {
            let current_turn_match = inputs.current_task_turn.as_ref().is_none_or(|turn| {
                action.task_turn_id.as_deref() == Some(turn.task_turn_id.as_str())
            });
            (action.artifact_id.as_deref() == Some(artifact_id)
                || action.secondary_artifact_id.as_deref() == Some(artifact_id))
                && current_turn_match
                && primary
                    .start_ms
                    .is_none_or(|start| action.created_at_ms >= start)
                && primary.end_ms.is_none_or(|end| action.created_at_ms <= end)
                && !action_is_support_or_interrupt(action)
        })
        .collect()
}

fn matching_primary_snapshots<'a>(
    inputs: &'a ActivityRecapInputs,
    primary: &StitchedActivitySegment,
) -> Vec<&'a SurfaceSnapshotFact> {
    inputs
        .surface_snapshots
        .iter()
        .filter(|snapshot| {
            snapshot.artifact_id.is_some()
                && snapshot.artifact_id.as_deref() == primary.artifact_id.as_deref()
        })
        .collect()
}

fn classify_kind_signals(
    inputs: &ActivityRecapInputs,
    primary: &StitchedActivitySegment,
    actions: &[&TaskActionFact],
    snapshots: &[&SurfaceSnapshotFact],
) -> KindSignals {
    let mut signals = KindSignals::default();
    let mut values = primary.activity_kinds.clone();
    values.extend(actions.iter().map(|action| action.action_kind.clone()));
    for snapshot in snapshots {
        values.extend(
            [
                snapshot.activity_state.as_ref(),
                snapshot.task_state.as_ref(),
                snapshot.command_state.as_ref(),
            ]
            .into_iter()
            .flatten()
            .cloned(),
        );
        signals.has_error |= snapshot.has_error_markers;
    }
    if let Some(current) = inputs.current_surface.as_ref().filter(|current| {
        current.artifact_id.is_some()
            && current.artifact_id.as_deref() == primary.artifact_id.as_deref()
    }) {
        values.extend(
            [current.activity_state.as_ref(), current.task_state.as_ref()]
                .into_iter()
                .flatten()
                .cloned(),
        );
    }

    for value in &values {
        let value = value.to_ascii_lowercase();
        signals.composing |= contains_any(
            &value,
            &[
                "compos",
                "writing",
                "editing_text",
                "filling_form",
                "chat_reasoning",
            ],
        );
        signals.planning |= contains_any(&value, &["planning", "plan", "outline"]);
        signals.reviewing |= contains_any(&value, &["review", "diff", "inspecting_output"]);
        signals.debugging |= contains_any(
            &value,
            &[
                "debug",
                "encountering_error",
                "failed",
                "blocked",
                "unresolved_error",
            ],
        );
        signals.reading_docs |= contains_any(
            &value,
            &[
                "reading_docs",
                "documentation",
                "reading",
                "searching_reference",
            ],
        );
        signals.file_browsing |= contains_any(&value, &["file_brows", "finder"]);
        signals.browsing |= contains_any(&value, &["brows", "searching"]);
        signals.communicating |= contains_any(&value, &["message", "reply", "communicat"]);
        signals.coding |= contains_any(
            &value,
            &[
                "editing_code",
                "implement",
                "coding",
                "code_edit",
                "actively_editing",
            ],
        );
        signals.terminal |= contains_any(
            &value,
            &[
                "running_command",
                "command_running",
                "running_tests",
                "observing_command_output",
            ],
        );
    }
    signals.has_action = !actions.is_empty();
    signals.has_error |= actions.iter().any(|action| {
        contains_any(
            &action.action_kind.to_ascii_lowercase(),
            &["error", "fail", "block", "debug"],
        )
    });

    let surface_blob = format!(
        "{} {} {}",
        primary.app_name.as_deref().unwrap_or_default(),
        primary.artifact_kind.as_deref().unwrap_or_default(),
        primary.surface_title.as_deref().unwrap_or_default()
    )
    .to_ascii_lowercase();
    signals.file_browsing |= contains_any(&surface_blob, &["finder", "file browser"]);
    signals.coding |= contains_any(
        &surface_blob,
        &["code_editor", "visual studio code", "vscode", "xcode"],
    );
    signals.terminal |= contains_any(&surface_blob, &["terminal", "iterm", "warp"]);
    signals.communicating |= contains_any(&surface_blob, &["gmail", "mail", "messages", "slack"]);
    signals.browsing |= contains_any(&surface_blob, &["browser", "chrome", "safari", "firefox"]);
    signals.debugging |= signals.has_error;
    signals
}

fn select_kind(signals: KindSignals) -> ActivityWorkLabelKind {
    if signals.debugging {
        ActivityWorkLabelKind::Debug
    } else if signals.reviewing {
        ActivityWorkLabelKind::Review
    } else if signals.coding {
        ActivityWorkLabelKind::Code
    } else if signals.composing || signals.planning {
        ActivityWorkLabelKind::Compose
    } else if signals.reading_docs {
        ActivityWorkLabelKind::Research
    } else if signals.file_browsing {
        ActivityWorkLabelKind::FileBrowse
    } else if signals.communicating {
        ActivityWorkLabelKind::Communicate
    } else if signals.terminal {
        ActivityWorkLabelKind::Terminal
    } else if signals.browsing {
        ActivityWorkLabelKind::Browse
    } else {
        ActivityWorkLabelKind::Unknown
    }
}

fn collect_term_candidates(
    inputs: &ActivityRecapInputs,
    primary: &StitchedActivitySegment,
    actions: &[&TaskActionFact],
    snapshots: &[&SurfaceSnapshotFact],
    rejected: &mut Vec<RejectedObjectiveTerm>,
) -> Vec<TermCandidate> {
    let mut candidates = Vec::new();

    if let Some(turn) = inputs.current_task_turn.as_ref() {
        if turn.goal_confidence >= 0.60 && turn.attribution_confidence >= 0.55 {
            if let Some(raw) = turn
                .latest_user_goal_summary
                .as_deref()
                .or(turn.task_object.as_deref())
            {
                match sanitize_objective_term(raw) {
                    Ok(value) => candidates.push(TermCandidate {
                        value,
                        source: TermSource::CurrentTaskGoal,
                        priority: 120,
                        confidence: evidence_confidence(
                            turn.goal_confidence.min(turn.attribution_confidence),
                        ),
                        anchor_type: ActivityEvidenceAnchorType::Action,
                        anchor_ids: turn.evidence_ids.clone(),
                        task_turn_id: Some(turn.task_turn_id.clone()),
                        workstream_id: turn.workstream_id.clone(),
                        eligibility_reason_codes: vec![
                            "current_task_goal_high_attribution".to_string()
                        ],
                        freshness: "current_turn".to_string(),
                    }),
                    Err(reason) => audit_rejected(rejected, Some(raw), reason),
                }
            }
        }
    }

    for open_loop in &inputs.open_loops {
        if !open_loop.eligible_for_objective {
            audit_rejected(
                rejected,
                open_loop.objective_hint.as_deref(),
                "open_loop_ineligible_for_current_task",
            );
            continue;
        }
        if !open_loop_matches_primary(open_loop, primary) {
            continue;
        }
        if open_loop.confidence < 0.45 || open_loop.quality == "thin" {
            audit_rejected(
                rejected,
                open_loop.objective_hint.as_deref(),
                "thin_open_loop",
            );
            continue;
        }
        add_candidate(
            &mut candidates,
            rejected,
            open_loop.objective_hint.as_deref(),
            TermSource::OpenLoop,
            100,
            evidence_confidence(open_loop.confidence),
            ActivityEvidenceAnchorType::OpenLoop,
            vec![open_loop.open_loop_id.clone()],
            None,
            open_loop.task_turn_id.as_deref(),
            Some(&open_loop.workstream_id),
            &open_loop.freshness,
        );
    }

    for action in actions {
        let blocked_source_reason = action_source_rejection_reason(action);
        add_candidate(
            &mut candidates,
            rejected,
            action.semantic_subject.as_deref(),
            TermSource::ActionSubject,
            110,
            evidence_confidence(
                action
                    .attribution_confidence
                    .unwrap_or(action.confidence)
                    .min(action.confidence),
            ),
            ActivityEvidenceAnchorType::Action,
            vec![action.action_id.clone()],
            blocked_source_reason,
            action.task_turn_id.as_deref(),
            primary.workstream_id.as_deref(),
            "current_turn",
        );
    }

    if let Some(workstream) = inputs.selected_workstream.as_ref().filter(|workstream| {
        primary.workstream_id.as_deref() == Some(&workstream.workstream_id)
            || primary.artifact_id.as_deref() == workstream.primary_artifact_id.as_deref()
    }) {
        add_candidate(
            &mut candidates,
            rejected,
            workstream.title.as_deref(),
            TermSource::Workstream,
            90,
            evidence_confidence(workstream.confidence),
            ActivityEvidenceAnchorType::Workstream,
            vec![workstream.workstream_id.clone()],
            (workstream.confidence < 0.45).then_some("thin_workstream_title"),
            inputs
                .current_task_turn
                .as_ref()
                .map(|turn| turn.task_turn_id.as_str()),
            Some(&workstream.workstream_id),
            "current_turn",
        );
    }

    for memory in inputs.memory_facts.iter().filter(|memory| {
        memory.relation == "support"
            && memory.confidence >= 0.65
            && memory.feedback_score >= 0.0
            && matches!(
                memory.memory_type.as_str(),
                "origin_intent" | "active_edit" | "draft_state" | "resume_instruction"
            )
            && (memory.artifact_id.as_deref() == primary.artifact_id.as_deref()
                || memory.workstream_id.as_deref() == primary.workstream_id.as_deref())
    }) {
        add_candidate(
            &mut candidates,
            rejected,
            memory.summary.as_deref(),
            TermSource::Memory,
            70,
            evidence_confidence(memory.confidence),
            ActivityEvidenceAnchorType::MemoryCell,
            vec![memory.memory_id.clone()],
            None,
            None,
            memory.workstream_id.as_deref(),
            "historical",
        );
    }

    for snapshot in snapshots {
        let snapshot_confidence = confidence_from_quality(&snapshot.evidence_quality);
        add_candidate(
            &mut candidates,
            rejected,
            snapshot.relative_file_name.as_deref(),
            TermSource::RelativeFile,
            85,
            snapshot_confidence,
            ActivityEvidenceAnchorType::SurfaceSnapshot,
            vec![snapshot.snapshot_id.clone()],
            None,
            None,
            primary.workstream_id.as_deref(),
            "current_turn",
        );
        add_candidate(
            &mut candidates,
            rejected,
            snapshot.task_state.as_deref(),
            TermSource::SurfaceTask,
            75,
            snapshot_confidence,
            ActivityEvidenceAnchorType::SurfaceSnapshot,
            vec![snapshot.snapshot_id.clone()],
            None,
            None,
            primary.workstream_id.as_deref(),
            "current_turn",
        );
        add_candidate(
            &mut candidates,
            rejected,
            snapshot.display_title.as_deref(),
            TermSource::SurfaceTitle,
            55,
            snapshot_confidence,
            ActivityEvidenceAnchorType::SurfaceSnapshot,
            vec![snapshot.snapshot_id.clone()],
            None,
            None,
            primary.workstream_id.as_deref(),
            "current_turn",
        );
    }

    for target in [
        inputs.return_target.as_ref(),
        inputs.resume_work_target.as_ref(),
    ]
    .into_iter()
    .flatten()
    .filter(|target| Some(target.artifact_id.as_str()) == primary.artifact_id.as_deref())
    {
        add_candidate(
            &mut candidates,
            rejected,
            target.display_title.as_deref(),
            TermSource::TargetTitle,
            60,
            evidence_confidence(target.identity_confidence),
            ActivityEvidenceAnchorType::Frame,
            target.evidence_frame_id.iter().cloned().collect(),
            None,
            None,
            primary.workstream_id.as_deref(),
            "current_turn",
        );
    }

    add_candidate(
        &mut candidates,
        rejected,
        primary.surface_title.as_deref(),
        TermSource::SurfaceTitle,
        45,
        primary.confidence,
        ActivityEvidenceAnchorType::Episode,
        Vec::new(),
        None,
        None,
        primary.workstream_id.as_deref(),
        "current_turn",
    );
    candidates
}

#[allow(clippy::too_many_arguments)]
fn add_candidate(
    candidates: &mut Vec<TermCandidate>,
    rejected: &mut Vec<RejectedObjectiveTerm>,
    raw: Option<&str>,
    source: TermSource,
    priority: u8,
    confidence: ActivityEvidenceConfidence,
    anchor_type: ActivityEvidenceAnchorType,
    anchor_ids: Vec<String>,
    blocked_reason: Option<&str>,
    task_turn_id: Option<&str>,
    workstream_id: Option<&str>,
    freshness: &str,
) {
    let Some(raw) = raw.map(str::trim).filter(|value| !value.is_empty()) else {
        return;
    };
    if let Some(reason) = blocked_reason {
        audit_rejected(rejected, Some(raw), reason);
        return;
    }
    match sanitize_objective_term(raw) {
        Ok(value) => candidates.push(TermCandidate {
            value,
            source,
            priority,
            confidence,
            anchor_type,
            anchor_ids,
            task_turn_id: task_turn_id.map(str::to_string),
            workstream_id: workstream_id.map(str::to_string),
            eligibility_reason_codes: vec!["eligible_after_task_relation_filter".to_string()],
            freshness: freshness.to_string(),
        }),
        Err(reason) => audit_rejected(rejected, Some(raw), reason),
    }
}

fn sanitize_objective_term(raw: &str) -> Result<String, &'static str> {
    if raw.contains("://")
        || raw.contains("/Users/")
        || raw.contains("/private/")
        || raw.contains("\\")
        || raw
            .split_whitespace()
            .any(|token| token.starts_with('/') || token.starts_with("~/"))
    {
        return Err("raw_locator");
    }
    let Some(mut value) = sanitize_public_text(raw.to_string(), 120) else {
        return Err("unsafe_or_internal_term");
    };
    value = strip_product_suffixes(&value);
    value = value
        .trim_matches(|ch: char| ch.is_whitespace() || matches!(ch, '-' | '—' | '|' | ':' | ','))
        .to_string();
    if value.is_empty() {
        return Err("empty_after_normalization");
    }
    if looks_like_browser_chrome(&value) {
        return Err("browser_chrome_or_toolbar");
    }
    if generic_app_only(&value) {
        return Err("generic_app_only");
    }
    if looks_like_sensitive_field(&value) {
        return Err("sensitive_field");
    }
    Ok(value)
}

fn normalize_object_label(value: &str) -> Option<String> {
    let mut output = value.trim().to_string();
    let lower = output.to_ascii_lowercase();
    for prefix in [
        "origin work target was ",
        "active work continued on ",
        "draft state remained for ",
        "resume instruction was ",
        "working on ",
        "writing ",
        "planning ",
        "plan for ",
        "reviewing ",
        "review ",
        "debugging ",
        "editing ",
        "implementing ",
        "reading about ",
        "reading ",
        "browsing ",
    ] {
        if lower.starts_with(prefix) {
            output = output[prefix.len()..].trim().to_string();
            break;
        }
    }
    output = output.trim_end_matches('.').trim().to_string();
    if output.is_empty() || generic_app_only(&output) || generic_object_state(&output) {
        None
    } else {
        Some(output)
    }
}

fn infer_where_label(
    inputs: &ActivityRecapInputs,
    primary: &StitchedActivitySegment,
    snapshots: &[&SurfaceSnapshotFact],
) -> Option<String> {
    let title = snapshots
        .iter()
        .find_map(|snapshot| snapshot.display_title.as_deref())
        .or(primary.surface_title.as_deref())
        .and_then(|value| sanitize_public_text(value.to_string(), 120))
        .filter(|value| !looks_like_browser_chrome(value));
    let app = primary
        .app_name
        .as_deref()
        .or_else(|| {
            inputs.current_surface.as_ref().and_then(|current| {
                (current.artifact_id.as_deref() == primary.artifact_id.as_deref())
                    .then_some(current.app_name.as_deref())
                    .flatten()
            })
        })
        .and_then(|value| sanitize_public_text(value.to_string(), 80));

    match (title, app) {
        (Some(title), Some(app))
            if !generic_app_only(&title) && !contains_case_insensitive(&title, &app) =>
        {
            Some(format!("{title} in {app}"))
        }
        (Some(title), _) if !generic_surface_title(&title) => Some(title),
        (_, Some(app)) => Some(app),
        _ => None,
    }
}

fn build_work_label(
    kind: ActivityWorkLabelKind,
    planning: bool,
    object: Option<&str>,
    where_label: Option<&str>,
) -> Option<String> {
    let with_object =
        |verb: &str| object.map(|object| format!("{verb} {}", article_object(object)));
    let in_where = |verb: &str| where_label.map(|where_label| format!("{verb} in {where_label}"));
    match kind {
        ActivityWorkLabelKind::Compose if planning => {
            with_object("planning").or_else(|| in_where("planning"))
        }
        ActivityWorkLabelKind::Compose => with_object("writing").or_else(|| in_where("writing")),
        ActivityWorkLabelKind::Review => with_object("reviewing").or_else(|| in_where("reviewing")),
        ActivityWorkLabelKind::Debug => {
            with_object("debugging").or_else(|| Some("debugging a terminal error".to_string()))
        }
        ActivityWorkLabelKind::Research => with_object("reading documentation about")
            .or_else(|| Some("reading documentation".to_string())),
        ActivityWorkLabelKind::Browse => with_object("browsing").or_else(|| in_where("browsing")),
        ActivityWorkLabelKind::FileBrowse => {
            if object.is_some_and(is_photo_term) {
                Some("browsing photos".to_string())
            } else {
                with_object("browsing").or_else(|| Some("browsing files".to_string()))
            }
        }
        ActivityWorkLabelKind::Communicate => {
            with_object("writing").or_else(|| in_where("writing a message"))
        }
        ActivityWorkLabelKind::Code => with_object("editing").or_else(|| in_where("coding")),
        ActivityWorkLabelKind::Terminal => {
            with_object("running").or_else(|| Some("working in a terminal".to_string()))
        }
        ActivityWorkLabelKind::Unknown => in_where("working"),
    }
}

fn summary_seed(label: &str, where_label: Option<&str>, _object_label: Option<&str>) -> String {
    where_label
        .filter(|where_label| !contains_case_insensitive(label, where_label))
        .map(|where_label| format!("{label} in {where_label}"))
        .unwrap_or_else(|| label.to_string())
}

fn label_confidence(
    primary: &StitchedActivitySegment,
    signals: KindSignals,
    object: Option<&TermCandidate>,
    where_label: Option<&str>,
    contradictions: bool,
) -> ActivityConfidence {
    if contradictions {
        return if signals.has_action || where_label.is_some() {
            ActivityConfidence::Low
        } else {
            ActivityConfidence::None
        };
    }
    let primary_role = primary_role_is_eligible(primary.role);
    let strong_object = object.is_some_and(|candidate| {
        matches!(
            candidate.source,
            TermSource::CurrentTaskGoal
                | TermSource::OpenLoop
                | TermSource::ActionSubject
                | TermSource::Workstream
                | TermSource::RelativeFile
        ) && candidate.confidence != ActivityEvidenceConfidence::Low
    });
    if primary_role
        && signals.has_action
        && strong_object
        && primary.confidence != ActivityEvidenceConfidence::Low
    {
        ActivityConfidence::High
    } else if primary_role && signals.has_action && (object.is_some() || where_label.is_some()) {
        ActivityConfidence::Medium
    } else if object.is_some() || where_label.is_some() {
        ActivityConfidence::Low
    } else {
        ActivityConfidence::None
    }
}

fn label_evidence_spans(
    label: &str,
    confidence: ActivityConfidence,
    actions: &[&TaskActionFact],
    object: Option<&TermCandidate>,
    snapshots: &[&SurfaceSnapshotFact],
) -> Vec<ActivityEvidenceSpan> {
    let evidence_confidence = match confidence {
        ActivityConfidence::High => ActivityEvidenceConfidence::High,
        ActivityConfidence::Medium => ActivityEvidenceConfidence::Medium,
        ActivityConfidence::Low | ActivityConfidence::None => ActivityEvidenceConfidence::Low,
    };
    let mut spans = Vec::new();
    if let Some(action) = actions.first() {
        spans.push(ActivityEvidenceSpan {
            claim_key: "primary_work_label".to_string(),
            claim_text: label.to_string(),
            anchor_type: ActivityEvidenceAnchorType::Action,
            anchor_ids: vec![action.action_id.clone()],
            confidence: evidence_confidence,
            source: ActivityEvidenceSource::Local,
        });
    }
    if let Some(object) = object.filter(|candidate| !candidate.anchor_ids.is_empty()) {
        spans.push(ActivityEvidenceSpan {
            claim_key: "primary_work_object".to_string(),
            claim_text: object.value.clone(),
            anchor_type: object.anchor_type,
            anchor_ids: object.anchor_ids.clone(),
            confidence: object.confidence,
            source: ActivityEvidenceSource::Local,
        });
    }
    if spans.is_empty() {
        if let Some(snapshot) = snapshots.first() {
            spans.push(ActivityEvidenceSpan {
                claim_key: "primary_work_label".to_string(),
                claim_text: label.to_string(),
                anchor_type: ActivityEvidenceAnchorType::SurfaceSnapshot,
                anchor_ids: vec![snapshot.snapshot_id.clone()],
                confidence: evidence_confidence,
                source: ActivityEvidenceSource::Local,
            });
        }
    }
    spans
}

fn action_source_rejection_reason(action: &TaskActionFact) -> Option<&'static str> {
    let source = action
        .evidence_source_kind
        .as_deref()
        .unwrap_or_default()
        .to_ascii_lowercase();
    if contains_any(
        &source,
        &["background_ocr", "ocr_background", "unattributed_ocr"],
    ) {
        Some("background_ocr")
    } else if contains_any(&source, &["model", "cloud", "llm", "micro_inference"]) {
        Some("unsupported_model_phrase")
    } else if action.quality_flags.iter().any(|flag| {
        contains_any(
            &flag.to_ascii_lowercase(),
            &["background", "unattributed", "private"],
        )
    }) {
        Some("unattributed_or_private_text")
    } else if action.confidence < 0.45
        || action
            .attribution_confidence
            .is_some_and(|confidence| confidence < 0.45)
    {
        Some("thin_action_attribution")
    } else {
        None
    }
}

fn action_is_support_or_interrupt(action: &TaskActionFact) -> bool {
    [
        action.action_role.as_str(),
        action.branch_action_role.as_deref().unwrap_or_default(),
    ]
    .into_iter()
    .any(|role| {
        matches!(
            role,
            "support" | "branch" | "interrupt" | "diagnostic" | "current_focus_only"
        )
    })
}

fn open_loop_matches_primary(open_loop: &OpenLoopFact, primary: &StitchedActivitySegment) -> bool {
    primary.workstream_id.as_deref() == Some(&open_loop.workstream_id)
        || primary.artifact_id.as_deref().is_some_and(|artifact_id| {
            [
                open_loop.origin_artifact_id.as_deref(),
                open_loop.primary_return_artifact_id.as_deref(),
                open_loop.resume_work_artifact_id.as_deref(),
            ]
            .contains(&Some(artifact_id))
        })
}

fn has_primary_contradiction(
    inputs: &ActivityRecapInputs,
    primary: &StitchedActivitySegment,
) -> bool {
    let candidate_id = inputs
        .selected_candidate
        .as_ref()
        .map(|candidate| candidate.candidate_id.as_str());
    inputs.memory_facts.iter().any(|memory| {
        let related = memory.artifact_id.as_deref() == primary.artifact_id.as_deref()
            || memory.workstream_id.as_deref() == primary.workstream_id.as_deref();
        related
            && (memory.relation.to_ascii_lowercase().contains("contradict")
                || candidate_id.is_some_and(|candidate_id| {
                    memory
                        .contradicts_candidate_ids
                        .iter()
                        .any(|value| value == candidate_id)
                }))
    })
}

fn audit_non_primary_terms(
    timeline: &StitchedActivityTimeline,
    rejected: &mut Vec<RejectedObjectiveTerm>,
) {
    for segment in &timeline.ordered_segments {
        if primary_role_is_eligible(segment.role) {
            continue;
        }
        let reason = match segment.role {
            ActivitySegmentRole::Support => "support_not_promoted",
            ActivitySegmentRole::Detour => "detour_not_promoted",
            ActivitySegmentRole::Interrupt => "interrupt_not_primary",
            ActivitySegmentRole::CurrentFocusOnly => "current_focus_only",
            ActivitySegmentRole::Unclear => "unclear_activity_role",
            _ => "non_primary_activity_role",
        };
        audit_rejected(rejected, segment.surface_title.as_deref(), reason);
    }
}

fn audit_rejected(rejected: &mut Vec<RejectedObjectiveTerm>, raw: Option<&str>, reason: &str) {
    if rejected.len() >= MAX_REJECTED_TERMS {
        return;
    }
    let Some(raw) = raw.map(str::trim).filter(|value| !value.is_empty()) else {
        return;
    };
    let term = sanitize_public_text(raw.to_string(), 120)
        .unwrap_or_else(|| "[unsafe term removed]".to_string());
    if !rejected
        .iter()
        .any(|existing| existing.term.eq_ignore_ascii_case(&term) && existing.reason == reason)
    {
        rejected.push(RejectedObjectiveTerm {
            term,
            reason: reason.to_string(),
        });
    }
}

fn dedupe_candidates(candidates: &mut Vec<TermCandidate>) {
    let mut output: Vec<TermCandidate> = Vec::new();
    for candidate in candidates.drain(..) {
        if let Some(existing) = output
            .iter_mut()
            .find(|existing| existing.value.eq_ignore_ascii_case(&candidate.value))
        {
            for anchor in candidate.anchor_ids {
                if !existing.anchor_ids.contains(&anchor) {
                    existing.anchor_ids.push(anchor);
                }
            }
            continue;
        }
        output.push(candidate);
    }
    *candidates = output;
}

fn strip_product_suffixes(value: &str) -> String {
    let mut output = value.trim().to_string();
    for separator in [" — ", " - ", " | "] {
        if let Some((left, right)) = output.rsplit_once(separator) {
            if generic_app_only(right) {
                output = left.trim().to_string();
                break;
            }
        }
    }
    output
}

fn looks_like_browser_chrome(value: &str) -> bool {
    let value = value.to_ascii_lowercase();
    contains_any(
        &value,
        &[
            "address and search bar",
            "back forward reload",
            "browser toolbar",
            "tab search",
            "bookmarks bar",
            "new tab",
            "untitled window",
        ],
    )
}

fn generic_app_only(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "chatgpt"
            | "codex"
            | "helium"
            | "google chrome"
            | "chrome"
            | "safari"
            | "firefox"
            | "finder"
            | "terminal"
            | "iterm"
            | "warp"
            | "visual studio code"
            | "vscode"
            | "xcode"
            | "gmail"
            | "mail"
            | "messages"
            | "slack"
    )
}

fn generic_surface_title(value: &str) -> bool {
    generic_app_only(value)
        || matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "untitled" | "home" | "workspace" | "new chat" | "inbox"
        )
}

fn generic_object_state(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "editing"
            | "composing"
            | "writing"
            | "planning"
            | "reading"
            | "reviewing"
            | "running command"
            | "actively working"
            | "in progress"
    )
}

fn looks_like_sensitive_field(value: &str) -> bool {
    let value = value.to_ascii_lowercase();
    contains_any(
        &value,
        &[
            "password",
            "passcode",
            "credit card",
            "security code",
            "api key",
            "secret key",
        ],
    )
}

fn article_object(value: &str) -> String {
    let lower = value.to_ascii_lowercase();
    if lower.starts_with("a ")
        || lower.starts_with("an ")
        || lower.starts_with("the ")
        || value.ends_with(".rs")
        || value.ends_with(".ts")
        || value.ends_with(".tsx")
        || value.ends_with(".md")
        || value.ends_with(".py")
    {
        value.to_string()
    } else {
        format!("the {value}")
    }
}

fn contains_planning_language(value: &str) -> bool {
    let value = value.to_ascii_lowercase();
    contains_any(&value, &["plan", "planning", "outline", "roadmap"])
}

fn is_photo_term(value: &str) -> bool {
    let value = value.to_ascii_lowercase();
    contains_any(&value, &["photo", "image", "picture"])
}

fn confidence_from_quality(value: &str) -> ActivityEvidenceConfidence {
    match value {
        "strong" | "high" => ActivityEvidenceConfidence::High,
        "medium" | "sufficient" => ActivityEvidenceConfidence::Medium,
        _ => ActivityEvidenceConfidence::Low,
    }
}

fn evidence_confidence(value: f64) -> ActivityEvidenceConfidence {
    if value >= 0.78 {
        ActivityEvidenceConfidence::High
    } else if value >= 0.50 {
        ActivityEvidenceConfidence::Medium
    } else {
        ActivityEvidenceConfidence::Low
    }
}

fn contains_any(value: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| value.contains(needle))
}

fn contains_case_insensitive(value: &str, needle: &str) -> bool {
    value
        .to_ascii_lowercase()
        .contains(&needle.to_ascii_lowercase())
}

fn dedupe_strings(values: &mut Vec<String>) {
    let mut output = Vec::new();
    for value in values.drain(..) {
        if !output.contains(&value) {
            output.push(value);
        }
    }
    *values = output;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::continuation::activity_recap_inputs::{
        ActivityRecapDecisionContext, CurrentTaskTurnFact, ExistingQualityFacts, MemoryFact,
        OpenLoopFact, SurfaceSnapshotFact, WorkstreamFact,
    };
    use crate::continuation::activity_recap_segments::ActivitySegmentPromotionState;

    fn empty_inputs() -> ActivityRecapInputs {
        ActivityRecapInputs {
            schema: "smalltalk.activity_recap_inputs.v1".to_string(),
            current_task_turn: None,
            decision_context: ActivityRecapDecisionContext {
                decision_id_seed: None,
                mode: "normal".to_string(),
                lookback_ms: 60_000,
                evidence_watermark: None,
                output_mode: None,
            },
            current_surface: None,
            selected_workstream: None,
            selected_candidate: None,
            return_target: None,
            resume_work_target: None,
            recent_segments: Vec::new(),
            recent_actions: Vec::new(),
            recent_moments: Vec::new(),
            open_loops: Vec::new(),
            workstream_states: Vec::new(),
            branch_contexts: Vec::new(),
            surface_snapshots: Vec::new(),
            support_evidence: Vec::new(),
            memory_facts: Vec::new(),
            existing_quality: ExistingQualityFacts {
                p0_quality_signals: None,
                current_surface_resolution: None,
                evidence_freshness_ledger: None,
                app_activity_summary: None,
                quality_gate: None,
            },
            input_warnings: Vec::new(),
        }
    }

    fn segment(
        id: &str,
        artifact_id: &str,
        app: &str,
        title: &str,
        kind: &str,
        role: ActivitySegmentRole,
        activity_kinds: &[&str],
    ) -> StitchedActivitySegment {
        StitchedActivitySegment {
            segment_id: id.to_string(),
            start_ms: Some(0),
            end_ms: Some(100),
            app_name: Some(app.to_string()),
            surface_title: Some(title.to_string()),
            artifact_kind: Some(kind.to_string()),
            workstream_id: Some("workstream-primary".to_string()),
            artifact_id: Some(artifact_id.to_string()),
            role,
            activity_kinds: activity_kinds
                .iter()
                .map(|value| (*value).to_string())
                .collect(),
            local_reason: "test fixture".to_string(),
            promotion_state: ActivitySegmentPromotionState::NotApplicable,
            confidence: ActivityEvidenceConfidence::High,
            evidence_anchor_ids: vec![id.to_string()],
        }
    }

    fn timeline(primary: StitchedActivitySegment) -> StitchedActivityTimeline {
        StitchedActivityTimeline {
            primary_segment: Some(primary.clone()),
            current_segment: Some(primary.clone()),
            ordered_segments: vec![primary],
            confidence: ActivityConfidence::High,
            ..StitchedActivityTimeline::default()
        }
    }

    fn action(
        id: &str,
        artifact_id: &str,
        kind: &str,
        subject: Option<&str>,
        source: &str,
    ) -> TaskActionFact {
        TaskActionFact {
            action_id: id.to_string(),
            task_turn_id: None,
            frame_id: "frame-safe".to_string(),
            artifact_id: Some(artifact_id.to_string()),
            secondary_artifact_id: None,
            action_kind: kind.to_string(),
            action_role: "primary".to_string(),
            confidence: 0.92,
            created_at_ms: 50,
            semantic_delta_kind: None,
            semantic_subject: subject.map(str::to_string),
            semantic_after_hint: None,
            evidence_source_kind: Some(source.to_string()),
            evidence_span_ids: Vec::new(),
            attribution_confidence: Some(0.9),
            quality_flags: Vec::new(),
            branch_kind: None,
            branch_action_role: None,
            branch_confidence: None,
            branch_reason_code: None,
        }
    }

    fn snapshot(
        id: &str,
        artifact_id: &str,
        domain: &str,
        title: &str,
        relative_file: Option<&str>,
    ) -> SurfaceSnapshotFact {
        SurfaceSnapshotFact {
            snapshot_id: id.to_string(),
            artifact_id: Some(artifact_id.to_string()),
            frame_id: Some(42),
            domain: domain.to_string(),
            app_name: None,
            display_title: Some(title.to_string()),
            relative_file_name: relative_file.map(str::to_string),
            git_branch: None,
            activity_state: None,
            task_state: None,
            command_state: None,
            has_error_markers: false,
            identity_confidence: "high".to_string(),
            evidence_quality: "strong".to_string(),
            openability: "unknown".to_string(),
            missing_evidence: Vec::new(),
            evidence_sources: vec!["native_metadata".to_string()],
            observed_at_ms: 90,
        }
    }

    fn current_capture_task() -> CurrentTaskTurnFact {
        CurrentTaskTurnFact {
            task_turn_id: "task-capture".to_string(),
            revision: 1,
            workstream_id: Some("workstream-primary".to_string()),
            parent_task_turn_id: None,
            prior_task_turn_id: None,
            supersedes_task_turn_id: None,
            latest_user_goal_summary: Some(
                "Understand what the island Capture button does".to_string(),
            ),
            task_object: Some("island_capture_button".to_string()),
            task_kind: "investigation".to_string(),
            execution_state: "active".to_string(),
            current_actor: "assistant_or_agent".to_string(),
            waiting_on: "agent".to_string(),
            relation_to_prior: "new_task".to_string(),
            started_at_ms: 100,
            last_observed_at_ms: 200,
            goal_confidence: 0.95,
            task_object_confidence: 0.93,
            actor_state_confidence: 0.92,
            execution_state_confidence: 0.92,
            waiting_on_confidence: 0.92,
            relation_confidence: 0.90,
            attribution_confidence: 0.94,
            task_claim_confidence: 0.93,
            state_claim_confidence: 0.92,
            missing_evidence: Vec::new(),
            evidence_ids: vec!["span-capture".to_string()],
            reason_codes: vec!["explicit_user_goal".to_string()],
        }
    }

    fn historical_open_loop(eligible_for_objective: bool) -> OpenLoopFact {
        OpenLoopFact {
            open_loop_id: "loop-stremio".to_string(),
            workstream_id: "workstream-primary".to_string(),
            task_turn_id: Some("task-prior".to_string()),
            parent_task_turn_id: None,
            origin_task_turn_id: Some("task-prior".to_string()),
            state: "open".to_string(),
            boundary_kind: "unfinished_edit".to_string(),
            quality: "strong".to_string(),
            confidence: 0.99,
            origin_artifact_id: Some("artifact-primary".to_string()),
            current_focus_artifact_id: Some("artifact-primary".to_string()),
            primary_return_artifact_id: Some("artifact-primary".to_string()),
            resume_work_artifact_id: Some("artifact-primary".to_string()),
            blocker_artifact_id: None,
            verification_artifact_id: None,
            objective_hint: Some("Continue Stremio".to_string()),
            last_concrete_progress: None,
            unfinished_state: Some("editing".to_string()),
            next_evidence_backed_action: None,
            current_focus_relation: None,
            relation_to_current_task: "unrelated".to_string(),
            freshness: "stale".to_string(),
            eligible_for_primary: false,
            eligible_for_objective,
            eligible_for_last_state: false,
            eligibility_reason_codes: vec!["different_task_turn".to_string()],
            missing_evidence: Vec::new(),
            evidence_spans: Vec::new(),
            last_updated_at_ms: 90,
        }
    }

    #[test]
    fn current_task_goal_outranks_generic_artifact_open_loop() {
        let mut inputs = empty_inputs();
        inputs.current_task_turn = Some(current_capture_task());
        inputs.open_loops = vec![historical_open_loop(true)];
        let primary = segment(
            "segment-primary",
            "artifact-primary",
            "AgentChat",
            "Smalltalk task",
            "agent_chat",
            ActivitySegmentRole::Primary,
            &["planning"],
        );

        let result = infer_activity_work_labels(&inputs, &timeline(primary));

        assert_eq!(
            result
                .selected_objective_source
                .as_ref()
                .map(|audit| audit.source_kind.as_str()),
            Some("current_task_goal")
        );
        assert_eq!(
            result
                .selected_objective_source
                .as_ref()
                .and_then(|audit| audit.task_turn_id.as_deref()),
            Some("task-capture")
        );
    }

    #[test]
    fn ineligible_open_loop_receives_no_objective_score() {
        let mut inputs = empty_inputs();
        inputs.current_task_turn = Some(current_capture_task());
        inputs.open_loops = vec![historical_open_loop(false)];
        let primary = segment(
            "segment-primary",
            "artifact-primary",
            "AgentChat",
            "Smalltalk task",
            "agent_chat",
            ActivitySegmentRole::Primary,
            &["planning"],
        );

        let result = infer_activity_work_labels(&inputs, &timeline(primary));

        assert!(result
            .objective_candidate_audit
            .iter()
            .all(|candidate| candidate.source_id.as_deref() != Some("loop-stremio")));
        assert!(result.rejected_terms.iter().any(|term| {
            term.term == "Continue Stremio"
                && term.reason == "open_loop_ineligible_for_current_task"
        }));
    }

    #[test]
    fn smalltalk_p5_chat_infers_a_grounded_planning_label() {
        let mut inputs = empty_inputs();
        inputs.selected_workstream = Some(WorkstreamFact {
            workstream_id: "workstream-primary".to_string(),
            state: "active".to_string(),
            title: Some("Smalltalk P5 activity recap plan".to_string()),
            primary_artifact_id: Some("chat".to_string()),
            last_active_timestamp_ms: 100,
            confidence: 0.9,
            unresolved_signal: None,
        });
        inputs.recent_actions = vec![action(
            "action-compose",
            "chat",
            "composing",
            Some("planning the Smalltalk P5 activity recap"),
            "active_attributed_text",
        )];
        let result = infer_activity_work_labels(
            &inputs,
            &timeline(segment(
                "chat-segment",
                "chat",
                "ChatGPT",
                "Smalltalk research chat",
                "browser_tab",
                ActivitySegmentRole::Primary,
                &["composing", "planning"],
            )),
        );

        assert_eq!(
            result.primary_work_label_kind,
            ActivityWorkLabelKind::Compose
        );
        assert_eq!(
            result.primary_work_label.as_deref(),
            Some("planning the Smalltalk P5 activity recap")
        );
        assert_eq!(result.confidence, ActivityConfidence::High);
        assert!(result
            .evidence_spans
            .iter()
            .any(|span| span.anchor_type == ActivityEvidenceAnchorType::Action));
    }

    #[test]
    fn generic_chat_degrades_to_writing_in_a_known_surface() {
        let mut inputs = empty_inputs();
        inputs.recent_actions = vec![action(
            "action-compose",
            "chat",
            "composing",
            None,
            "native_action",
        )];
        let result = infer_activity_work_labels(
            &inputs,
            &timeline(segment(
                "chat-segment",
                "chat",
                "ChatGPT",
                "ChatGPT",
                "browser_tab",
                ActivitySegmentRole::Primary,
                &["composing"],
            )),
        );

        assert_eq!(
            result.primary_work_label.as_deref(),
            Some("writing in ChatGPT")
        );
        assert_eq!(result.object_label, None);
        assert_eq!(result.confidence, ActivityConfidence::Medium);
        assert!(result
            .missing_evidence
            .iter()
            .any(|value| value.contains("exact work subject")));
    }

    #[test]
    fn finder_photo_detour_is_audited_but_does_not_replace_chat_work() {
        let mut inputs = empty_inputs();
        inputs.recent_actions = vec![action(
            "action-compose",
            "chat",
            "composing",
            Some("Smalltalk P5 plan"),
            "active_attributed_text",
        )];
        let chat = segment(
            "chat-segment",
            "chat",
            "ChatGPT",
            "Smalltalk chat",
            "browser_tab",
            ActivitySegmentRole::Primary,
            &["composing", "planning"],
        );
        let finder = segment(
            "finder-segment",
            "finder",
            "Finder",
            "Photos",
            "finder",
            ActivitySegmentRole::Detour,
            &["file_browsing"],
        );
        let mut stitched = timeline(chat.clone());
        stitched.current_segment = Some(finder.clone());
        stitched.ordered_segments = vec![chat, finder];

        let result = infer_activity_work_labels(&inputs, &stitched);

        assert_eq!(
            result.primary_work_label.as_deref(),
            Some("planning the Smalltalk P5 plan")
        );
        assert!(!result.objective_terms.iter().any(|term| term == "Photos"));
        assert!(result
            .rejected_terms
            .iter()
            .any(|term| { term.term == "Photos" && term.reason == "detour_not_promoted" }));
    }

    #[test]
    fn finder_photos_becomes_a_label_only_after_explicit_promotion() {
        let inputs = empty_inputs();
        let result = infer_activity_work_labels(
            &inputs,
            &timeline(segment(
                "finder-segment",
                "finder",
                "Finder",
                "Photos",
                "finder",
                ActivitySegmentRole::PromotedPrimary,
                &["file_browsing"],
            )),
        );

        assert_eq!(
            result.primary_work_label_kind,
            ActivityWorkLabelKind::FileBrowse
        );
        assert_eq!(
            result.primary_work_label.as_deref(),
            Some("browsing photos")
        );
        assert_eq!(result.confidence, ActivityConfidence::Low);
    }

    #[test]
    fn active_editor_file_typing_infers_code_without_a_raw_path() {
        let mut inputs = empty_inputs();
        inputs.recent_actions = vec![action(
            "action-edit",
            "editor",
            "editing",
            None,
            "native_action",
        )];
        inputs.surface_snapshots = vec![snapshot(
            "snapshot-editor",
            "editor",
            "code_editor",
            "Smalltalk — Visual Studio Code",
            Some("continuation.rs"),
        )];
        let result = infer_activity_work_labels(
            &inputs,
            &timeline(segment(
                "editor-segment",
                "editor",
                "Visual Studio Code",
                "Smalltalk — Visual Studio Code",
                "code_editor",
                ActivitySegmentRole::Primary,
                &["actively_editing"],
            )),
        );

        assert_eq!(result.primary_work_label_kind, ActivityWorkLabelKind::Code);
        assert_eq!(
            result.primary_work_label.as_deref(),
            Some("editing continuation.rs")
        );
        assert!(!result
            .primary_work_label
            .as_deref()
            .unwrap_or_default()
            .contains('/'));
    }

    #[test]
    fn terminal_error_infers_debugging_without_inventing_command_output() {
        let mut inputs = empty_inputs();
        inputs.recent_actions = vec![action(
            "action-error",
            "terminal",
            "encountering_error",
            None,
            "native_action",
        )];
        let mut terminal_snapshot = snapshot(
            "snapshot-terminal",
            "terminal",
            "terminal",
            "Terminal",
            None,
        );
        terminal_snapshot.has_error_markers = true;
        terminal_snapshot.command_state = Some("visible_error_unresolved".to_string());
        inputs.surface_snapshots = vec![terminal_snapshot];
        let result = infer_activity_work_labels(
            &inputs,
            &timeline(segment(
                "terminal-segment",
                "terminal",
                "Terminal",
                "Terminal",
                "terminal",
                ActivitySegmentRole::Primary,
                &["encountering_error"],
            )),
        );

        assert_eq!(result.primary_work_label_kind, ActivityWorkLabelKind::Debug);
        assert_eq!(
            result.primary_work_label.as_deref(),
            Some("debugging a terminal error")
        );
        assert_eq!(result.object_label, None);
    }

    #[test]
    fn background_ocr_subject_is_rejected_and_never_leaks_into_label() {
        let mut inputs = empty_inputs();
        inputs.recent_actions = vec![action(
            "action-compose",
            "chat",
            "composing",
            Some("Confidential acquisition roadmap"),
            "background_ocr",
        )];
        let result = infer_activity_work_labels(
            &inputs,
            &timeline(segment(
                "chat-segment",
                "chat",
                "ChatGPT",
                "ChatGPT",
                "browser_tab",
                ActivitySegmentRole::Primary,
                &["composing"],
            )),
        );

        assert_eq!(
            result.primary_work_label.as_deref(),
            Some("writing in ChatGPT")
        );
        assert!(!result
            .primary_work_label
            .as_deref()
            .unwrap_or_default()
            .contains("acquisition"));
        assert!(result
            .rejected_terms
            .iter()
            .any(|term| { term.reason == "background_ocr" && term.term.contains("acquisition") }));
    }

    #[test]
    fn support_only_activity_cannot_become_a_primary_research_label() {
        let inputs = empty_inputs();
        let docs = segment(
            "docs-segment",
            "docs",
            "Safari",
            "Rust documentation",
            "browser_tab",
            ActivitySegmentRole::Support,
            &["reading_docs"],
        );
        let result = infer_activity_work_labels(&inputs, &timeline(docs));

        assert_eq!(result.primary_work_label, None);
        assert_eq!(result.confidence, ActivityConfidence::None);
        assert!(result.rejected_terms.iter().any(|term| {
            term.term == "Rust documentation" && term.reason == "non_primary_activity_role"
        }));
    }

    #[test]
    fn contradictory_memory_downgrades_an_otherwise_strong_label() {
        let mut inputs = empty_inputs();
        inputs.recent_actions = vec![action(
            "action-edit",
            "editor",
            "editing",
            Some("continuation engine"),
            "native_action",
        )];
        inputs.memory_facts = vec![MemoryFact {
            memory_id: "memory-contradiction".to_string(),
            workstream_id: Some("workstream-primary".to_string()),
            artifact_id: Some("editor".to_string()),
            episode_id: None,
            action_id: None,
            memory_type: "correction".to_string(),
            relation: "contradicts_objective".to_string(),
            summary: None,
            source_anchor: serde_json::Value::Null,
            last_seen_at_ms: 100,
            confidence: 0.9,
            importance: 0.9,
            retrieval_score: 0.9,
            feedback_score: -1.0,
            retrieval_reasons: Vec::new(),
            supports_candidate_ids: Vec::new(),
            contradicts_candidate_ids: Vec::new(),
        }];
        let result = infer_activity_work_labels(
            &inputs,
            &timeline(segment(
                "editor-segment",
                "editor",
                "Visual Studio Code",
                "Smalltalk",
                "code_editor",
                ActivitySegmentRole::Primary,
                &["editing_code"],
            )),
        );

        assert_eq!(result.confidence, ActivityConfidence::Low);
        assert!(result
            .missing_evidence
            .iter()
            .any(|value| value.contains("contradictory")));
    }

    #[test]
    fn trusted_origin_memory_can_supply_a_missing_object_at_low_confidence() {
        let mut inputs = empty_inputs();
        inputs.memory_facts = vec![MemoryFact {
            memory_id: "memory-origin".to_string(),
            workstream_id: Some("workstream-primary".to_string()),
            artifact_id: Some("chat".to_string()),
            episode_id: None,
            action_id: None,
            memory_type: "origin_intent".to_string(),
            relation: "support".to_string(),
            summary: Some("Origin work target was Smalltalk activity recap.".to_string()),
            source_anchor: serde_json::Value::Null,
            last_seen_at_ms: 100,
            confidence: 0.8,
            importance: 0.8,
            retrieval_score: 0.8,
            feedback_score: 0.0,
            retrieval_reasons: vec!["graph_match".to_string()],
            supports_candidate_ids: Vec::new(),
            contradicts_candidate_ids: Vec::new(),
        }];
        let result = infer_activity_work_labels(
            &inputs,
            &timeline(segment(
                "chat-segment",
                "chat",
                "ChatGPT",
                "ChatGPT",
                "browser_tab",
                ActivitySegmentRole::Primary,
                &["composing"],
            )),
        );

        assert_eq!(
            result.primary_work_label.as_deref(),
            Some("writing the Smalltalk activity recap")
        );
        assert_eq!(result.confidence, ActivityConfidence::Low);
        assert!(result.evidence_spans.iter().any(|span| {
            span.anchor_type == ActivityEvidenceAnchorType::MemoryCell
                && span.anchor_ids == vec!["memory-origin"]
        }));
    }

    #[test]
    fn raw_paths_and_model_subjects_are_rejected_from_objective_terms() {
        let mut inputs = empty_inputs();
        inputs.recent_actions = vec![
            action(
                "action-path",
                "editor",
                "editing",
                Some("/Users/example/private/secret.rs"),
                "native_action",
            ),
            action(
                "action-model",
                "editor",
                "editing",
                Some("invented launch strategy"),
                "cloud_model",
            ),
        ];
        let result = infer_activity_work_labels(
            &inputs,
            &timeline(segment(
                "editor-segment",
                "editor",
                "Visual Studio Code",
                "Visual Studio Code",
                "code_editor",
                ActivitySegmentRole::Primary,
                &["editing_code"],
            )),
        );

        assert!(result.objective_terms.is_empty());
        assert!(result
            .rejected_terms
            .iter()
            .any(|term| term.reason == "raw_locator"));
        assert!(result
            .rejected_terms
            .iter()
            .any(|term| term.reason == "unsupported_model_phrase"));
        assert!(!result
            .primary_work_label
            .as_deref()
            .unwrap_or_default()
            .contains("strategy"));
    }

    #[test]
    fn inferred_fields_map_to_recap_without_changing_target_confidence() {
        let result = ActivityWorkLabelResult {
            primary_work_label: Some("reviewing the continuation diff".to_string()),
            primary_work_label_kind: ActivityWorkLabelKind::Review,
            primary_work_summary_seed: Some("reviewing the continuation diff".to_string()),
            where_label: Some("Smalltalk in Visual Studio Code".to_string()),
            object_label: Some("continuation diff".to_string()),
            confidence: ActivityConfidence::High,
            evidence_spans: vec![ActivityEvidenceSpan {
                claim_key: "primary_work_label".to_string(),
                claim_text: "reviewing the continuation diff".to_string(),
                anchor_type: ActivityEvidenceAnchorType::Action,
                anchor_ids: vec!["action-review".to_string()],
                confidence: ActivityEvidenceConfidence::High,
                source: ActivityEvidenceSource::Local,
            }],
            ..ActivityWorkLabelResult::default()
        };
        let recap = recap_with_activity_work_label(result);

        assert_eq!(recap.activity_confidence, ActivityConfidence::High);
        assert_eq!(recap.target_confidence, ActivityConfidence::None);
        assert_eq!(
            recap.validation_status,
            ActivityRecapValidationStatus::Valid
        );
        assert_eq!(
            recap.primary_work_label.as_deref(),
            Some("reviewing the continuation diff")
        );
    }
}
