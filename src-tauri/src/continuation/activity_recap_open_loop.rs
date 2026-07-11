use super::activity_recap::{
    sanitize_public_text, ActivityConfidence, ActivityCurrentState, ActivityEvidenceAnchorType,
    ActivityEvidenceConfidence, ActivityEvidenceSource, ActivityEvidenceSpan,
    ActivityRecapValidationStatus, ContinueActivityRecap,
};
use super::activity_recap_detours::DetourRecapResult;
use super::activity_recap_inputs::{
    ActivityRecapInputs, OpenLoopFact, SurfaceSnapshotFact, TargetFact, TaskActionFact,
    WorkstreamStateFact,
};
use super::activity_recap_objective::ActivityWorkLabelResult;
use super::activity_recap_segments::{
    ActivityPrimaryContinuity, ActivitySegmentRole, StitchedActivitySegment,
    StitchedActivityTimeline,
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

const CLAIM_PRIMARY_WORK: &str = "primary_work_summary";
const CLAIM_LAST_STATE: &str = "last_meaningful_state";
const CLAIM_UNFINISHED: &str = "unfinished_state";
const CLAIM_NEXT_ACTION: &str = "next_action_summary";
const CLAIM_WHY_TARGET: &str = "why_this_target";
const CLAIM_WHY_NO_TARGET: &str = "why_no_safe_target";

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct LastStateRecapResult {
    pub current_state: ActivityCurrentState,
    pub last_meaningful_state: Option<String>,
    pub unfinished_state: Option<String>,
    pub next_action_summary: Option<String>,
    pub why_this_target: Option<String>,
    pub why_no_safe_target: Option<String>,
    pub target_confidence: ActivityConfidence,
    pub missing_evidence: Vec<String>,
    pub warnings: Vec<String>,
    pub evidence_spans: Vec<ActivityEvidenceSpan>,
    pub validation_status: ActivityRecapValidationStatus,
}

#[derive(Debug, Clone)]
struct GroundedClaim {
    text: String,
    span: ActivityEvidenceSpan,
}

/// Produces the P5-06 last-state layer from normalized local evidence. This is
/// a pure transform: it does not open, promote, rank, persist, or model-infer a
/// target. Existing P0-P4 decisions remain authoritative.
pub(crate) fn synthesize_last_state(
    inputs: &ActivityRecapInputs,
    timeline: &StitchedActivityTimeline,
    work_label: &ActivityWorkLabelResult,
    detours: &DetourRecapResult,
) -> LastStateRecapResult {
    let primary = timeline.primary_segment.as_ref();
    let open_loop = select_open_loop(inputs, primary);
    let workstream_state = select_workstream_state(inputs, primary);
    let primary_actions = matching_primary_actions(inputs, primary);
    let primary_snapshots = matching_primary_snapshots(inputs, primary);
    let complete_or_idle = completion_is_latest(
        open_loop,
        workstream_state,
        &primary_actions,
        &primary_snapshots,
    );
    let blocker =
        !complete_or_idle && unresolved_blocker(open_loop, workstream_state, &primary_snapshots);
    let active_action = primary_actions
        .iter()
        .copied()
        .find(|action| action_is_active(action));

    let mut result = LastStateRecapResult {
        current_state: classify_current_state(
            inputs,
            timeline,
            detours,
            open_loop,
            workstream_state,
            active_action,
            blocker,
            complete_or_idle,
        ),
        target_confidence: target_confidence(inputs),
        validation_status: ActivityRecapValidationStatus::Thin,
        ..LastStateRecapResult::default()
    };

    let primary_claim = primary_work_claim(work_label);
    let last_claim = last_meaningful_claim(
        inputs,
        timeline,
        work_label,
        open_loop,
        workstream_state,
        &primary_actions,
        &primary_snapshots,
    );
    let unfinished_claim = unfinished_claim(
        timeline,
        open_loop,
        workstream_state,
        active_action,
        blocker,
        &primary_snapshots,
        complete_or_idle,
    );
    let next_claim = next_action_claim(
        inputs,
        timeline,
        work_label,
        open_loop,
        workstream_state,
        active_action,
        blocker,
        complete_or_idle,
    );
    let (why_target_claim, why_no_target_claim) = target_explanation_claims(inputs, timeline);

    push_claim_span(&mut result.evidence_spans, primary_claim);
    assign_claim(
        &mut result.last_meaningful_state,
        &mut result.evidence_spans,
        last_claim,
    );
    assign_claim(
        &mut result.unfinished_state,
        &mut result.evidence_spans,
        unfinished_claim,
    );
    assign_claim(
        &mut result.next_action_summary,
        &mut result.evidence_spans,
        next_claim,
    );
    assign_claim(
        &mut result.why_this_target,
        &mut result.evidence_spans,
        why_target_claim,
    );
    assign_claim(
        &mut result.why_no_safe_target,
        &mut result.evidence_spans,
        why_no_target_claim,
    );

    result
        .missing_evidence
        .extend(work_label.missing_evidence.iter().cloned());
    result
        .missing_evidence
        .extend(detours.missing_evidence.iter().cloned());
    if let Some(open_loop) = open_loop {
        result
            .missing_evidence
            .extend(open_loop.missing_evidence.iter().cloned());
    } else {
        result
            .missing_evidence
            .push("No grounded open loop describes unfinished work.".to_string());
    }
    if let Some(workstream_state) = workstream_state {
        result
            .missing_evidence
            .extend(workstream_state.missing_evidence.iter().cloned());
    }
    if result.last_meaningful_state.is_none() {
        result
            .missing_evidence
            .push("The last meaningful task state is not visible.".to_string());
    }
    if result.next_action_summary.is_none() {
        result
            .missing_evidence
            .push("No evidence-backed next action is visible.".to_string());
    }
    if inputs.return_target.is_none() && inputs.resume_work_target.is_none() {
        result
            .missing_evidence
            .push("No safely openable return target is grounded.".to_string());
    }
    result.warnings.extend(timeline.warnings.iter().cloned());
    result.warnings.extend(detours.warnings.iter().cloned());
    dedupe_strings(&mut result.missing_evidence);
    dedupe_strings(&mut result.warnings);
    dedupe_spans(&mut result.evidence_spans);

    let state_claims_valid = [
        (CLAIM_LAST_STATE, result.last_meaning_state_ref()),
        (CLAIM_UNFINISHED, result.unfinished_state.as_deref()),
        (CLAIM_NEXT_ACTION, result.next_action_summary.as_deref()),
        (CLAIM_WHY_TARGET, result.why_this_target.as_deref()),
        (CLAIM_WHY_NO_TARGET, result.why_no_safe_target.as_deref()),
    ]
    .into_iter()
    .all(|(key, value)| {
        value.is_none() || claim_has_span(&result.evidence_spans, key, value.unwrap_or_default())
    });
    let useful = result.last_meaningful_state.is_some()
        || result.unfinished_state.is_some()
        || result.next_action_summary.is_some();
    result.validation_status = if state_claims_valid
        && useful
        && !matches!(result.current_state, ActivityCurrentState::Unclear)
    {
        ActivityRecapValidationStatus::Valid
    } else {
        ActivityRecapValidationStatus::Thin
    };
    result
}

impl LastStateRecapResult {
    fn last_meaning_state_ref(&self) -> Option<&str> {
        self.last_meaningful_state.as_deref()
    }
}

/// Applies only P5-06-owned fields and then revalidates the public contract
/// after the existing privacy scrubber has run.
pub(crate) fn apply_last_state_recap(
    mut recap: ContinueActivityRecap,
    result: LastStateRecapResult,
) -> ContinueActivityRecap {
    recap.current_state = result.current_state;
    recap.last_meaningful_state = result.last_meaningful_state;
    recap.unfinished_state = result.unfinished_state;
    recap.next_action_summary = result.next_action_summary;
    recap.why_this_target = result.why_this_target;
    recap.why_no_safe_target = result.why_no_safe_target;
    recap.target_confidence = result.target_confidence;
    recap.missing_evidence.extend(result.missing_evidence);
    recap.warnings.extend(result.warnings);
    recap.evidence_spans.extend(result.evidence_spans);
    recap.validation_status = max_validation(recap.validation_status, result.validation_status);
    dedupe_strings(&mut recap.missing_evidence);
    dedupe_strings(&mut recap.warnings);
    dedupe_spans(&mut recap.evidence_spans);
    let mut recap = recap.sanitized();
    enforce_public_claim_grounding(&mut recap);
    recap
}

fn select_open_loop<'a>(
    inputs: &'a ActivityRecapInputs,
    primary: Option<&StitchedActivitySegment>,
) -> Option<&'a OpenLoopFact> {
    let selected_workstream_id = inputs
        .selected_workstream
        .as_ref()
        .map(|workstream| workstream.workstream_id.as_str());
    let selected_loop_id = inputs
        .selected_candidate
        .as_ref()
        .and_then(|candidate| candidate.open_loop_id.as_deref());
    let mut candidates = inputs
        .open_loops
        .iter()
        .filter(|open_loop| open_loop.eligible_for_last_state)
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        open_loop_priority(right, selected_loop_id, selected_workstream_id, primary)
            .cmp(&open_loop_priority(
                left,
                selected_loop_id,
                selected_workstream_id,
                primary,
            ))
            .then_with(|| right.last_updated_at_ms.cmp(&left.last_updated_at_ms))
            .then_with(|| right.confidence.total_cmp(&left.confidence))
            .then_with(|| left.open_loop_id.cmp(&right.open_loop_id))
    });
    candidates.into_iter().next()
}

fn open_loop_priority(
    open_loop: &OpenLoopFact,
    selected_loop_id: Option<&str>,
    selected_workstream_id: Option<&str>,
    primary: Option<&StitchedActivitySegment>,
) -> u8 {
    let mut priority = 0;
    if selected_loop_id == Some(open_loop.open_loop_id.as_str()) {
        priority += 8;
    }
    if selected_workstream_id == Some(open_loop.workstream_id.as_str()) {
        priority += 4;
    }
    if primary.is_some_and(|segment| open_loop_matches_primary(open_loop, segment)) {
        priority += 2;
    }
    if open_loop.quality != "thin" && open_loop.confidence >= 0.55 {
        priority += 1;
    }
    priority
}

fn open_loop_matches_primary(open_loop: &OpenLoopFact, primary: &StitchedActivitySegment) -> bool {
    primary.workstream_id.as_deref() == Some(open_loop.workstream_id.as_str())
        || primary.artifact_id.as_deref().is_some_and(|artifact_id| {
            [
                open_loop.origin_artifact_id.as_deref(),
                open_loop.primary_return_artifact_id.as_deref(),
                open_loop.resume_work_artifact_id.as_deref(),
                open_loop.current_focus_artifact_id.as_deref(),
            ]
            .contains(&Some(artifact_id))
        })
}

fn select_workstream_state<'a>(
    inputs: &'a ActivityRecapInputs,
    primary: Option<&StitchedActivitySegment>,
) -> Option<&'a WorkstreamStateFact> {
    let selected_workstream_id = inputs
        .selected_workstream
        .as_ref()
        .map(|workstream| workstream.workstream_id.as_str());
    let primary_workstream_id = primary.and_then(|segment| segment.workstream_id.as_deref());
    inputs
        .workstream_states
        .iter()
        .filter(|state| {
            let current_turn_match = inputs.current_task_turn.as_ref().is_none_or(|turn| {
                state.task_turn_id.as_deref() == Some(turn.task_turn_id.as_str())
            });
            current_turn_match
                && (selected_workstream_id == Some(state.workstream_id.as_str())
                    || primary_workstream_id == Some(state.workstream_id.as_str())
                    || (selected_workstream_id.is_none() && primary_workstream_id.is_none()))
        })
        .max_by(|left, right| {
            workstream_state_priority(left, selected_workstream_id, primary_workstream_id)
                .cmp(&workstream_state_priority(
                    right,
                    selected_workstream_id,
                    primary_workstream_id,
                ))
                .then_with(|| left.observed_at_ms.cmp(&right.observed_at_ms))
                .then_with(|| left.confidence.total_cmp(&right.confidence))
                .then_with(|| right.snapshot_id.cmp(&left.snapshot_id))
        })
}

fn workstream_state_priority(
    state: &WorkstreamStateFact,
    selected_workstream_id: Option<&str>,
    primary_workstream_id: Option<&str>,
) -> u8 {
    if selected_workstream_id == Some(state.workstream_id.as_str()) {
        2
    } else if primary_workstream_id == Some(state.workstream_id.as_str()) {
        1
    } else {
        0
    }
}

fn matching_primary_actions<'a>(
    inputs: &'a ActivityRecapInputs,
    primary: Option<&StitchedActivitySegment>,
) -> Vec<&'a TaskActionFact> {
    let mut actions = inputs
        .recent_actions
        .iter()
        .filter(|action| action_is_publicly_grounded(action))
        .filter(|action| {
            let Some(primary) = primary else {
                return false;
            };
            let artifact_match = primary.artifact_id.as_deref().is_some_and(|artifact_id| {
                action.artifact_id.as_deref() == Some(artifact_id)
                    || action.secondary_artifact_id.as_deref() == Some(artifact_id)
            });
            let within_segment = primary
                .start_ms
                .is_none_or(|start| action.created_at_ms >= start)
                && primary.end_ms.is_none_or(|end| action.created_at_ms <= end);
            artifact_match && within_segment && !action_is_support_or_interrupt(action)
        })
        .collect::<Vec<_>>();
    actions.sort_by(|left, right| {
        right
            .created_at_ms
            .cmp(&left.created_at_ms)
            .then_with(|| left.action_id.cmp(&right.action_id))
    });
    actions
}

fn matching_primary_snapshots<'a>(
    inputs: &'a ActivityRecapInputs,
    primary: Option<&StitchedActivitySegment>,
) -> Vec<&'a SurfaceSnapshotFact> {
    let mut snapshots = inputs
        .surface_snapshots
        .iter()
        .filter(|snapshot| {
            primary.is_some_and(|primary| {
                primary.artifact_id.is_some()
                    && snapshot.artifact_id.as_deref() == primary.artifact_id.as_deref()
            })
        })
        .collect::<Vec<_>>();
    snapshots.sort_by(|left, right| {
        right
            .observed_at_ms
            .cmp(&left.observed_at_ms)
            .then_with(|| left.snapshot_id.cmp(&right.snapshot_id))
    });
    snapshots
}

fn classify_current_state(
    inputs: &ActivityRecapInputs,
    timeline: &StitchedActivityTimeline,
    detours: &DetourRecapResult,
    open_loop: Option<&OpenLoopFact>,
    workstream_state: Option<&WorkstreamStateFact>,
    active_action: Option<&TaskActionFact>,
    blocker: bool,
    complete_or_idle: bool,
) -> ActivityCurrentState {
    if complete_or_idle {
        return ActivityCurrentState::CompleteOrIdle;
    }
    if blocker {
        return ActivityCurrentState::Blocked;
    }
    if detours.current_state == Some(ActivityCurrentState::RecentlyDetoured)
        || current_differs_from_primary(timeline)
    {
        return ActivityCurrentState::RecentlyDetoured;
    }
    if active_action.is_some()
        || workstream_state.is_some_and(workstream_state_is_active)
        || timeline.current_segment.as_ref().is_some_and(|segment| {
            primary_role(segment.role)
                && segment.activity_kinds.iter().any(|kind| active_kind(kind))
        })
        || inputs.current_surface.as_ref().is_some_and(|surface| {
            surface.activity_state.as_deref().is_some_and(active_kind)
                || surface.task_state.as_deref().is_some_and(active_kind)
        })
    {
        return ActivityCurrentState::ActivelyWorking;
    }
    if open_loop.is_some_and(open_loop_is_unfinished)
        || workstream_state.is_some_and(workstream_state_is_unfinished)
        || (timeline.primary_segment.is_some()
            && inputs
                .selected_workstream
                .as_ref()
                .and_then(|workstream| workstream.unresolved_signal.as_ref())
                .is_some())
    {
        return ActivityCurrentState::PausedAfterProgress;
    }
    ActivityCurrentState::Unclear
}

fn last_meaningful_claim(
    inputs: &ActivityRecapInputs,
    timeline: &StitchedActivityTimeline,
    work_label: &ActivityWorkLabelResult,
    open_loop: Option<&OpenLoopFact>,
    workstream_state: Option<&WorkstreamStateFact>,
    actions: &[&TaskActionFact],
    snapshots: &[&SurfaceSnapshotFact],
) -> Option<GroundedClaim> {
    if let Some(open_loop) = open_loop {
        if let Some(text) = open_loop
            .last_concrete_progress
            .as_deref()
            .and_then(|text| public_sentence(text, 240))
        {
            return open_loop_claim(CLAIM_LAST_STATE, text, open_loop);
        }
    }
    if let Some(action) = actions.first().copied() {
        if let Some(text) =
            action_state_sentence(action, work_label, timeline.primary_segment.as_ref())
        {
            return claim(
                CLAIM_LAST_STATE,
                text,
                ActivityEvidenceAnchorType::Action,
                vec![action.action_id.clone()],
                evidence_confidence(action.confidence),
            );
        }
    }
    if let Some(moment) = inputs
        .recent_moments
        .iter()
        .filter(|moment| {
            timeline.primary_segment.as_ref().is_some_and(|primary| {
                primary.artifact_id.as_deref().is_some_and(|artifact_id| {
                    moment.dominant_artifact_id.as_deref() == Some(artifact_id)
                        || moment.post_artifact_id.as_deref() == Some(artifact_id)
                })
            })
        })
        .filter(|moment| moment.evidence_quality != "thin")
        .filter(|moment| moment.semantic_summary.is_some())
        .max_by_key(|moment| moment.ended_at_ms)
    {
        if let Some(text) = moment
            .semantic_summary
            .as_deref()
            .and_then(|value| public_sentence(value, 240))
        {
            let anchors = if moment.ordered_event_ids.is_empty() {
                vec![moment.moment_id.clone()]
            } else {
                moment.ordered_event_ids.clone()
            };
            return claim(
                CLAIM_LAST_STATE,
                text,
                ActivityEvidenceAnchorType::Event,
                anchors,
                confidence_from_quality(&moment.evidence_quality),
            );
        }
    }
    if let Some(snapshot) = snapshots.first().copied() {
        if let Some(text) = snapshot_state_sentence(snapshot, work_label) {
            return claim(
                CLAIM_LAST_STATE,
                text,
                ActivityEvidenceAnchorType::SurfaceSnapshot,
                vec![snapshot.snapshot_id.clone()],
                confidence_from_quality(&snapshot.evidence_quality),
            );
        }
    }
    if let Some(workstream_state) = workstream_state {
        if let Some(text) = workstream_state_sentence(workstream_state) {
            return workstream_state_claim(CLAIM_LAST_STATE, text, workstream_state);
        }
    }
    if let Some(workstream) = inputs.selected_workstream.as_ref() {
        if let Some(text) = workstream
            .unresolved_signal
            .as_deref()
            .and_then(|value| public_sentence(value, 240))
        {
            return claim(
                CLAIM_LAST_STATE,
                text,
                ActivityEvidenceAnchorType::Workstream,
                vec![workstream.workstream_id.clone()],
                evidence_confidence(workstream.confidence),
            );
        }
    }
    let current = inputs.current_surface.as_ref()?;
    if !current.claim_eligible || current.evidence_ids.is_empty() {
        return None;
    }
    let app = current
        .app_name
        .as_deref()
        .and_then(|value| sanitize_public_text(value.to_string(), 80))
        .unwrap_or_else(|| "the latest surface".to_string());
    claim(
        CLAIM_LAST_STATE,
        format!("I saw recent work in {app}, but the exact task state was not visible."),
        ActivityEvidenceAnchorType::Event,
        current.evidence_ids.clone(),
        ActivityEvidenceConfidence::Low,
    )
}

fn unfinished_claim(
    timeline: &StitchedActivityTimeline,
    open_loop: Option<&OpenLoopFact>,
    workstream_state: Option<&WorkstreamStateFact>,
    active_action: Option<&TaskActionFact>,
    blocker: bool,
    snapshots: &[&SurfaceSnapshotFact],
    complete_or_idle: bool,
) -> Option<GroundedClaim> {
    if complete_or_idle {
        return None;
    }
    if let Some(open_loop) = open_loop.filter(|open_loop| !open_loop_is_complete(open_loop)) {
        if let Some(text) = open_loop
            .unfinished_state
            .as_deref()
            .and_then(|text| public_sentence(text, 240))
        {
            return open_loop_claim(CLAIM_UNFINISHED, text, open_loop);
        }
        if blocker {
            return open_loop_claim(
                CLAIM_UNFINISHED,
                "A visible error or blocker remained unresolved.".to_string(),
                open_loop,
            );
        }
    }
    if blocker {
        if let Some(snapshot) = snapshots
            .iter()
            .copied()
            .find(|value| value.has_error_markers)
        {
            return claim(
                CLAIM_UNFINISHED,
                "A visible error or blocker remained unresolved.".to_string(),
                ActivityEvidenceAnchorType::SurfaceSnapshot,
                vec![snapshot.snapshot_id.clone()],
                confidence_from_quality(&snapshot.evidence_quality),
            );
        }
        if let Some(workstream_state) = workstream_state.filter(|state| {
            state.has_blocker || normalized_signal(&state.state).contains("blocked")
        }) {
            return workstream_state_claim(
                CLAIM_UNFINISHED,
                "A visible error or blocker remained unresolved.".to_string(),
                workstream_state,
            );
        }
    }
    if let Some(action) = active_action.filter(|action| action_is_composing(action)) {
        return claim(
            CLAIM_UNFINISHED,
            "A draft or edit was still active.".to_string(),
            ActivityEvidenceAnchorType::Action,
            vec![action.action_id.clone()],
            evidence_confidence(action.confidence),
        );
    }
    if timeline.primary_continuity == ActivityPrimaryContinuity::BranchedWithoutReturn {
        let branch = timeline
            .current_segment
            .as_ref()
            .filter(|segment| !primary_role(segment.role))?;
        return segment_claim(
            CLAIM_UNFINISHED,
            "A recent branch had not returned to the primary work.".to_string(),
            branch,
        );
    }
    None
}

fn next_action_claim(
    inputs: &ActivityRecapInputs,
    timeline: &StitchedActivityTimeline,
    work_label: &ActivityWorkLabelResult,
    open_loop: Option<&OpenLoopFact>,
    workstream_state: Option<&WorkstreamStateFact>,
    active_action: Option<&TaskActionFact>,
    blocker: bool,
    complete_or_idle: bool,
) -> Option<GroundedClaim> {
    if complete_or_idle {
        return None;
    }
    if let Some(open_loop) = open_loop.filter(|open_loop| !open_loop_is_complete(open_loop)) {
        if let Some(text) = open_loop
            .next_evidence_backed_action
            .as_deref()
            .and_then(|text| public_sentence(text, 240))
        {
            return open_loop_claim(CLAIM_NEXT_ACTION, text, open_loop);
        }
    }
    if blocker {
        if let Some(open_loop) = open_loop {
            return open_loop_claim(
                CLAIM_NEXT_ACTION,
                "Inspect the visible error before rerunning or continuing.".to_string(),
                open_loop,
            );
        }
        if let Some(snapshot) = inputs
            .surface_snapshots
            .iter()
            .filter(|snapshot| snapshot.has_error_markers)
            .max_by_key(|snapshot| snapshot.observed_at_ms)
        {
            return claim(
                CLAIM_NEXT_ACTION,
                "Inspect the visible error before rerunning or continuing.".to_string(),
                ActivityEvidenceAnchorType::SurfaceSnapshot,
                vec![snapshot.snapshot_id.clone()],
                confidence_from_quality(&snapshot.evidence_quality),
            );
        }
        if let Some(workstream_state) = workstream_state.filter(|state| {
            state.has_blocker || normalized_signal(&state.state).contains("blocked")
        }) {
            return workstream_state_claim(
                CLAIM_NEXT_ACTION,
                "Inspect the visible error before rerunning or continuing.".to_string(),
                workstream_state,
            );
        }
    }
    if let Some(action) = active_action.filter(|action| action_is_composing(action)) {
        let text = work_label
            .where_label
            .as_deref()
            .and_then(|where_label| sanitize_public_text(where_label.to_string(), 100))
            .map(|where_label| format!("Continue writing or editing in {where_label}."))
            .unwrap_or_else(|| "Continue the active draft or edit.".to_string());
        return claim(
            CLAIM_NEXT_ACTION,
            text,
            ActivityEvidenceAnchorType::Action,
            vec![action.action_id.clone()],
            evidence_confidence(action.confidence),
        );
    }
    if timeline.primary_continuity == ActivityPrimaryContinuity::BranchedWithoutReturn {
        let branch = timeline
            .current_segment
            .as_ref()
            .filter(|segment| !primary_role(segment.role))?;
        let text = work_label
            .where_label
            .as_deref()
            .and_then(|where_label| sanitize_public_text(where_label.to_string(), 100))
            .map(|where_label| {
                format!("Return to {where_label}; the exact next task step is not visible.")
            })
            .unwrap_or_else(|| {
                "Return to the grounded primary work; the exact next task step is not visible."
                    .to_string()
            });
        return segment_claim(CLAIM_NEXT_ACTION, text, branch);
    }
    if let Some(current) = inputs.current_surface.as_ref().filter(|current| {
        current.claim_eligible
            && !current.evidence_ids.is_empty()
            && (current.identity_confidence.unwrap_or(0.0) < 0.55
                || current.evidence_quality == "thin")
    }) {
        let surface = current
            .app_name
            .as_deref()
            .and_then(|value| sanitize_public_text(value.to_string(), 80))
            .unwrap_or_else(|| "surface".to_string());
        return claim(
            CLAIM_NEXT_ACTION,
            format!("Review the latest {surface} surface; the exact task identity is thin."),
            ActivityEvidenceAnchorType::Event,
            current.evidence_ids.clone(),
            ActivityEvidenceConfidence::Low,
        );
    }
    if inputs.return_target.is_some() || inputs.resume_work_target.is_some() {
        let target = inputs
            .resume_work_target
            .as_ref()
            .or(inputs.return_target.as_ref())?;
        let where_label = work_label
            .where_label
            .as_deref()
            .and_then(|value| sanitize_public_text(value.to_string(), 100));
        let text = where_label
            .map(|where_label| {
                format!("Return to {where_label} to continue the grounded primary work.")
            })
            .unwrap_or_else(|| "Return to the grounded primary work.".to_string());
        return target_claim(CLAIM_NEXT_ACTION, text, target, inputs);
    }
    if timeline.primary_segment.is_some() {
        return segment_claim(
            CLAIM_NEXT_ACTION,
            "Capture evidence again if you need a precise return target.".to_string(),
            timeline.primary_segment.as_ref()?,
        );
    }
    None
}

fn target_explanation_claims(
    inputs: &ActivityRecapInputs,
    timeline: &StitchedActivityTimeline,
) -> (Option<GroundedClaim>, Option<GroundedClaim>) {
    if let Some(target) = inputs
        .resume_work_target
        .as_ref()
        .or(inputs.return_target.as_ref())
    {
        return (
            target_claim(
                CLAIM_WHY_TARGET,
                "This is the safest return point because it matches the grounded primary work."
                    .to_string(),
                target,
                inputs,
            ),
            None,
        );
    }
    let Some(primary) = timeline
        .primary_segment
        .as_ref()
        .or(timeline.current_segment.as_ref())
    else {
        return (None, None);
    };
    (
        None,
        segment_claim(
            CLAIM_WHY_NO_TARGET,
            "Recent activity is visible, but no safely openable return target is grounded."
                .to_string(),
            primary,
        ),
    )
}

fn primary_work_claim(work_label: &ActivityWorkLabelResult) -> Option<GroundedClaim> {
    let text = work_label.primary_work_summary_seed.clone()?;
    let source = work_label.evidence_spans.first()?;
    claim(
        CLAIM_PRIMARY_WORK,
        text,
        source.anchor_type,
        source.anchor_ids.clone(),
        source.confidence,
    )
}

fn action_state_sentence(
    action: &TaskActionFact,
    work_label: &ActivityWorkLabelResult,
    primary: Option<&StitchedActivitySegment>,
) -> Option<String> {
    let kind = normalized_signal(&action.action_kind);
    if contains_any(&kind, &["error", "block", "fail"]) {
        return Some("You had a visible error or blocker in the active work surface.".to_string());
    }
    if contains_any(&kind, &["run", "execute", "command"]) {
        return Some("You were reviewing the result of a recent command or run.".to_string());
    }
    if action_is_composing(action) {
        if let Some(summary) = work_label
            .primary_work_summary_seed
            .as_deref()
            .and_then(|value| sanitize_public_text(value.to_string(), 200))
        {
            return Some(format!("You were {}.", lowercase_first(&summary)));
        }
        return Some("You were actively writing or editing.".to_string());
    }
    if contains_any(&kind, &["review", "read", "inspect", "verify"]) {
        let where_label = work_label
            .where_label
            .as_deref()
            .and_then(|value| sanitize_public_text(value.to_string(), 100))
            .or_else(|| {
                primary
                    .and_then(|segment| segment.app_name.as_deref())
                    .and_then(|value| sanitize_public_text(value.to_string(), 80))
            });
        return Some(
            where_label
                .map(|where_label| format!("You were reviewing work in {where_label}."))
                .unwrap_or_else(|| "You were reviewing the latest work state.".to_string()),
        );
    }
    work_label
        .primary_work_summary_seed
        .as_deref()
        .and_then(|value| sanitize_public_text(value.to_string(), 200))
        .map(|summary| format!("You were {}.", lowercase_first(&summary)))
}

fn snapshot_state_sentence(
    snapshot: &SurfaceSnapshotFact,
    work_label: &ActivityWorkLabelResult,
) -> Option<String> {
    if snapshot.has_error_markers {
        return Some("You had a visible error or blocker in the active work surface.".to_string());
    }
    let state = snapshot
        .task_state
        .as_deref()
        .or(snapshot.activity_state.as_deref())
        .or(snapshot.command_state.as_deref())?;
    let state = sanitize_public_text(state.to_string(), 100)?;
    let where_label = work_label
        .where_label
        .as_deref()
        .and_then(|value| sanitize_public_text(value.to_string(), 100));
    Some(
        where_label
            .map(|where_label| format!("The last visible state was {state} in {where_label}."))
            .unwrap_or_else(|| format!("The last visible task state was {state}.")),
    )
}

fn unresolved_blocker(
    open_loop: Option<&OpenLoopFact>,
    workstream_state: Option<&WorkstreamStateFact>,
    snapshots: &[&SurfaceSnapshotFact],
) -> bool {
    open_loop.is_some_and(|open_loop| {
        !open_loop_is_complete(open_loop)
            && (open_loop.blocker_artifact_id.is_some()
                || contains_any(
                    &normalized_signal(&format!(
                        "{} {} {}",
                        open_loop.state,
                        open_loop.boundary_kind,
                        open_loop.unfinished_state.as_deref().unwrap_or_default()
                    )),
                    &["block", "error", "fail"],
                ))
    }) || workstream_state.is_some_and(|state| {
        state.has_blocker
            || contains_any(
                &normalized_signal(&format!(
                    "{} {}",
                    state.state,
                    state.transition_kind.as_deref().unwrap_or_default()
                )),
                &["block", "error", "fail"],
            )
    }) || snapshots.iter().any(|snapshot| snapshot.has_error_markers)
}

fn completion_is_latest(
    open_loop: Option<&OpenLoopFact>,
    workstream_state: Option<&WorkstreamStateFact>,
    actions: &[&TaskActionFact],
    snapshots: &[&SurfaceSnapshotFact],
) -> bool {
    let complete_at = [
        open_loop
            .filter(|open_loop| open_loop_is_complete(open_loop))
            .map(|open_loop| open_loop.last_updated_at_ms),
        workstream_state
            .filter(|state| workstream_state_is_complete(state))
            .map(|state| state.observed_at_ms),
    ]
    .into_iter()
    .flatten()
    .max();
    let Some(complete_at) = complete_at else {
        return false;
    };
    let competing_at = [
        open_loop
            .filter(|open_loop| !open_loop_is_complete(open_loop))
            .map(|open_loop| open_loop.last_updated_at_ms),
        workstream_state
            .filter(|state| !workstream_state_is_complete(state))
            .map(|state| state.observed_at_ms),
        actions.first().map(|action| action.created_at_ms),
        snapshots
            .iter()
            .filter(|snapshot| snapshot.has_error_markers)
            .map(|snapshot| snapshot.observed_at_ms)
            .max(),
    ]
    .into_iter()
    .flatten()
    .max();
    competing_at.is_none_or(|competing_at| complete_at >= competing_at)
}

fn open_loop_is_complete(open_loop: &OpenLoopFact) -> bool {
    contains_any(
        &normalized_signal(&format!("{} {}", open_loop.state, open_loop.boundary_kind)),
        &["complete", "completed", "closed", "resolved", "idle"],
    )
}

fn open_loop_is_unfinished(open_loop: &OpenLoopFact) -> bool {
    !open_loop_is_complete(open_loop)
        && (open_loop.unfinished_state.is_some()
            || open_loop.next_evidence_backed_action.is_some()
            || contains_any(
                &normalized_signal(&format!("{} {}", open_loop.state, open_loop.boundary_kind)),
                &[
                    "open",
                    "active",
                    "unfinished",
                    "blocked",
                    "pending",
                    "paused",
                ],
            ))
}

fn workstream_state_is_complete(state: &WorkstreamStateFact) -> bool {
    contains_any(
        &normalized_signal(&state.state),
        &["resolved", "complete", "completed", "abandoned", "idle"],
    )
}

fn workstream_state_is_active(state: &WorkstreamStateFact) -> bool {
    contains_any(
        &normalized_signal(&state.state),
        &[
            "editing",
            "composing",
            "waiting for output",
            "verifying",
            "exploring",
        ],
    )
}

fn workstream_state_is_unfinished(state: &WorkstreamStateFact) -> bool {
    !workstream_state_is_complete(state)
        && contains_any(
            &normalized_signal(&state.state),
            &[
                "active",
                "blocked",
                "ready to resume",
                "branching for evidence",
                "suspended",
                "paused",
            ],
        )
}

fn workstream_state_sentence(state: &WorkstreamStateFact) -> Option<String> {
    let normalized = normalized_signal(&state.state);
    let text = if contains_any(&normalized, &["blocked"]) || state.has_blocker {
        "The selected workstream still had a visible blocker."
    } else if contains_any(&normalized, &["ready to resume", "suspended", "paused"]) {
        "The selected workstream was paused after recent progress."
    } else if contains_any(&normalized, &["resolved", "complete", "completed", "idle"]) {
        "The selected workstream was complete or idle."
    } else if contains_any(&normalized, &["composing"]) {
        "The selected workstream still had active composing work."
    } else if contains_any(&normalized, &["editing"]) {
        "The selected workstream still had active editing work."
    } else if contains_any(&normalized, &["verifying", "waiting for output"]) {
        "The selected workstream was waiting on or reviewing output."
    } else if contains_any(&normalized, &["branching for evidence"]) {
        "The selected workstream was in a support branch without a recorded return."
    } else {
        return None;
    };
    Some(text.to_string())
}

fn workstream_state_claim(
    key: &str,
    text: String,
    state: &WorkstreamStateFact,
) -> Option<GroundedClaim> {
    let anchors = if state.evidence_action_ids.is_empty() {
        vec![state.snapshot_id.clone()]
    } else {
        state.evidence_action_ids.clone()
    };
    claim(
        key,
        text,
        ActivityEvidenceAnchorType::Workstream,
        anchors,
        evidence_confidence(state.confidence),
    )
}

fn current_differs_from_primary(timeline: &StitchedActivityTimeline) -> bool {
    let (Some(primary), Some(current)) = (
        timeline.primary_segment.as_ref(),
        timeline.current_segment.as_ref(),
    ) else {
        return false;
    };
    primary.segment_id != current.segment_id && !primary_role(current.role)
}

fn action_is_publicly_grounded(action: &TaskActionFact) -> bool {
    let source = normalized_signal(action.evidence_source_kind.as_deref().unwrap_or_default());
    let private_flag = action.quality_flags.iter().any(|flag| {
        contains_any(
            &normalized_signal(flag),
            &["private", "background", "unattributed"],
        )
    });
    action.confidence >= 0.45
        && action.attribution_confidence.unwrap_or(action.confidence) >= 0.45
        && !private_flag
        && !contains_any(
            &source,
            &[
                "background ocr",
                "unattributed ocr",
                "model",
                "cloud",
                "llm",
            ],
        )
}

fn action_is_support_or_interrupt(action: &TaskActionFact) -> bool {
    [
        action.action_role.as_str(),
        action.branch_action_role.as_deref().unwrap_or_default(),
    ]
    .into_iter()
    .any(|value| {
        contains_any(
            &normalized_signal(value),
            &[
                "support",
                "branch",
                "interrupt",
                "diagnostic",
                "current focus only",
            ],
        )
    })
}

fn action_is_active(action: &TaskActionFact) -> bool {
    active_kind(&action.action_kind)
        || action
            .semantic_delta_kind
            .as_deref()
            .is_some_and(active_kind)
}

fn action_is_composing(action: &TaskActionFact) -> bool {
    let signal = normalized_signal(&format!(
        "{} {}",
        action.action_kind,
        action.semantic_delta_kind.as_deref().unwrap_or_default()
    ));
    contains_any(
        &signal,
        &[
            "compos",
            "typing",
            "writ",
            "edit",
            "content change",
            "draft",
        ],
    )
}

fn active_kind(value: &str) -> bool {
    contains_any(
        &normalized_signal(value),
        &[
            "compos",
            "typing",
            "writ",
            "edit",
            "run",
            "execut",
            "review",
            "inspect",
            "debug",
            "coding",
            "content change",
        ],
    )
}

fn primary_role(role: ActivitySegmentRole) -> bool {
    matches!(
        role,
        ActivitySegmentRole::Primary
            | ActivitySegmentRole::PromotedPrimary
            | ActivitySegmentRole::Return
    )
}

fn target_confidence(inputs: &ActivityRecapInputs) -> ActivityConfidence {
    let Some(target) = inputs
        .resume_work_target
        .as_ref()
        .or(inputs.return_target.as_ref())
    else {
        return ActivityConfidence::None;
    };
    let sufficiency = inputs
        .selected_candidate
        .as_ref()
        .map(|candidate| candidate.evidence_sufficiency_score)
        .unwrap_or_default();
    if target.evidence_quality == "strong"
        && target.identity_confidence >= 0.8
        && target.evidence_frame_id.is_some()
        && sufficiency >= 0.75
    {
        ActivityConfidence::High
    } else if target.identity_confidence >= 0.55 && sufficiency >= 0.5 {
        ActivityConfidence::Medium
    } else {
        ActivityConfidence::Low
    }
}

fn target_claim(
    key: &str,
    text: String,
    target: &TargetFact,
    inputs: &ActivityRecapInputs,
) -> Option<GroundedClaim> {
    if let Some(frame_id) = target.evidence_frame_id.as_ref() {
        return claim(
            key,
            text,
            ActivityEvidenceAnchorType::Frame,
            vec![frame_id.clone()],
            confidence_from_quality(&target.evidence_quality),
        );
    }
    if let Some(action_id) = inputs
        .selected_candidate
        .as_ref()
        .and_then(|candidate| candidate.last_meaningful_action_id.as_ref())
    {
        return claim(
            key,
            text,
            ActivityEvidenceAnchorType::Action,
            vec![action_id.clone()],
            evidence_confidence(target.identity_confidence),
        );
    }
    if let Some(open_loop_id) = inputs
        .selected_candidate
        .as_ref()
        .and_then(|candidate| candidate.open_loop_id.as_ref())
    {
        return claim(
            key,
            text,
            ActivityEvidenceAnchorType::OpenLoop,
            vec![open_loop_id.clone()],
            evidence_confidence(target.identity_confidence),
        );
    }
    None
}

fn open_loop_claim(key: &str, text: String, open_loop: &OpenLoopFact) -> Option<GroundedClaim> {
    let confidence = evidence_confidence(open_loop.confidence);
    if let Some(span) = open_loop
        .evidence_spans
        .iter()
        .filter(|span| !span.evidence_id.trim().is_empty())
        .max_by(|left, right| left.confidence.total_cmp(&right.confidence))
    {
        return claim(
            key,
            text,
            anchor_type_for_kind(&span.evidence_kind),
            vec![span.evidence_id.clone()],
            evidence_confidence(span.confidence),
        );
    }
    claim(
        key,
        text,
        ActivityEvidenceAnchorType::OpenLoop,
        vec![open_loop.open_loop_id.clone()],
        confidence,
    )
}

fn segment_claim(
    key: &str,
    text: String,
    segment: &StitchedActivitySegment,
) -> Option<GroundedClaim> {
    let anchor = segment.evidence_anchor_ids.first()?.clone();
    claim(
        key,
        text,
        ActivityEvidenceAnchorType::Event,
        vec![anchor],
        segment.confidence,
    )
}

fn claim(
    key: &str,
    text: String,
    anchor_type: ActivityEvidenceAnchorType,
    anchor_ids: Vec<String>,
    confidence: ActivityEvidenceConfidence,
) -> Option<GroundedClaim> {
    let text = sanitize_public_text(text, 280)?;
    let anchor_ids = bounded_anchors(anchor_ids);
    if anchor_ids.is_empty() {
        return None;
    }
    Some(GroundedClaim {
        span: ActivityEvidenceSpan {
            claim_key: key.to_string(),
            claim_text: text.clone(),
            anchor_type,
            anchor_ids,
            confidence,
            source: ActivityEvidenceSource::Local,
        },
        text,
    })
}

fn assign_claim(
    destination: &mut Option<String>,
    spans: &mut Vec<ActivityEvidenceSpan>,
    claim: Option<GroundedClaim>,
) {
    let Some(claim) = claim else {
        return;
    };
    *destination = Some(claim.text);
    spans.push(claim.span);
}

fn push_claim_span(spans: &mut Vec<ActivityEvidenceSpan>, claim: Option<GroundedClaim>) {
    if let Some(claim) = claim {
        spans.push(claim.span);
    }
}

fn enforce_public_claim_grounding(recap: &mut ContinueActivityRecap) {
    if recap
        .primary_work_summary
        .as_deref()
        .is_some_and(|value| !claim_has_span(&recap.evidence_spans, CLAIM_PRIMARY_WORK, value))
    {
        recap.primary_work_summary = None;
        recap
            .missing_evidence
            .push("The primary work summary lacked a public evidence span.".to_string());
    }
    clear_unanchored(
        &recap.evidence_spans,
        CLAIM_LAST_STATE,
        &mut recap.last_meaningful_state,
    );
    clear_unanchored(
        &recap.evidence_spans,
        CLAIM_UNFINISHED,
        &mut recap.unfinished_state,
    );
    clear_unanchored(
        &recap.evidence_spans,
        CLAIM_NEXT_ACTION,
        &mut recap.next_action_summary,
    );
    clear_unanchored(
        &recap.evidence_spans,
        CLAIM_WHY_TARGET,
        &mut recap.why_this_target,
    );
    clear_unanchored(
        &recap.evidence_spans,
        CLAIM_WHY_NO_TARGET,
        &mut recap.why_no_safe_target,
    );
    dedupe_strings(&mut recap.missing_evidence);
    let has_useful_state = recap.last_meaningful_state.is_some()
        || recap.unfinished_state.is_some()
        || recap.next_action_summary.is_some();
    if !has_useful_state || recap.current_state == ActivityCurrentState::Unclear {
        recap.validation_status = ActivityRecapValidationStatus::Thin;
    }
}

fn clear_unanchored(spans: &[ActivityEvidenceSpan], key: &str, value: &mut Option<String>) {
    if value
        .as_deref()
        .is_some_and(|text| !claim_has_span(spans, key, text))
    {
        *value = None;
    }
}

fn claim_has_span(spans: &[ActivityEvidenceSpan], key: &str, text: &str) -> bool {
    spans
        .iter()
        .any(|span| span.claim_key == key && span.claim_text == text && !span.anchor_ids.is_empty())
}

fn max_validation(
    left: ActivityRecapValidationStatus,
    right: ActivityRecapValidationStatus,
) -> ActivityRecapValidationStatus {
    if left == ActivityRecapValidationStatus::Rejected
        || right == ActivityRecapValidationStatus::Rejected
    {
        ActivityRecapValidationStatus::Rejected
    } else if left == ActivityRecapValidationStatus::Fallback
        || right == ActivityRecapValidationStatus::Fallback
    {
        ActivityRecapValidationStatus::Fallback
    } else if left == ActivityRecapValidationStatus::Valid
        || right == ActivityRecapValidationStatus::Valid
    {
        ActivityRecapValidationStatus::Valid
    } else {
        ActivityRecapValidationStatus::Thin
    }
}

fn public_sentence(value: &str, max_chars: usize) -> Option<String> {
    let mut text = sanitize_public_text(value.to_string(), max_chars)?;
    if !text.ends_with(['.', '!', '?']) {
        text.push('.');
    }
    Some(text)
}

fn anchor_type_for_kind(value: &str) -> ActivityEvidenceAnchorType {
    let value = normalized_signal(value);
    if value.contains("frame") {
        ActivityEvidenceAnchorType::Frame
    } else if value.contains("action") {
        ActivityEvidenceAnchorType::Action
    } else if value.contains("episode") {
        ActivityEvidenceAnchorType::Episode
    } else if value.contains("workstream") {
        ActivityEvidenceAnchorType::Workstream
    } else if value.contains("branch") {
        ActivityEvidenceAnchorType::Branch
    } else if value.contains("snapshot") {
        ActivityEvidenceAnchorType::SurfaceSnapshot
    } else if value.contains("memory") {
        ActivityEvidenceAnchorType::MemoryCell
    } else if value.contains("open loop") {
        ActivityEvidenceAnchorType::OpenLoop
    } else {
        ActivityEvidenceAnchorType::Event
    }
}

fn confidence_from_quality(value: &str) -> ActivityEvidenceConfidence {
    match normalized_signal(value).as_str() {
        "strong" | "high" | "complete" => ActivityEvidenceConfidence::High,
        "usable" | "medium" | "moderate" => ActivityEvidenceConfidence::Medium,
        _ => ActivityEvidenceConfidence::Low,
    }
}

fn evidence_confidence(value: f64) -> ActivityEvidenceConfidence {
    if value >= 0.8 {
        ActivityEvidenceConfidence::High
    } else if value >= 0.55 {
        ActivityEvidenceConfidence::Medium
    } else {
        ActivityEvidenceConfidence::Low
    }
}

fn bounded_anchors(values: Vec<String>) -> Vec<String> {
    let mut output = Vec::new();
    for value in values {
        let value = value.trim();
        if !value.is_empty() && !output.iter().any(|existing| existing == value) {
            output.push(value.to_string());
        }
        if output.len() >= 16 {
            break;
        }
    }
    output
}

fn normalized_signal(value: &str) -> String {
    value
        .to_ascii_lowercase()
        .replace(['_', '-', ':'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn contains_any(value: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| value.contains(needle))
}

fn lowercase_first(value: &str) -> String {
    let mut chars = value.chars();
    match chars.next() {
        Some(first) => first.to_lowercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

fn dedupe_strings(values: &mut Vec<String>) {
    let mut seen = HashSet::new();
    values.retain(|value| seen.insert(value.clone()));
}

fn dedupe_spans(spans: &mut Vec<ActivityEvidenceSpan>) {
    let mut seen = HashSet::new();
    spans.retain(|span| {
        seen.insert(format!(
            "{}:{:?}:{}:{}",
            span.claim_key,
            span.anchor_type,
            span.anchor_ids.join("|"),
            span.claim_text
        ))
    });
}

#[cfg(test)]
mod tests {
    use super::super::activity_recap::{ActivityDetourRole, ActivityDetourSummary};
    use super::super::activity_recap_inputs::{
        ActivityRecapDecisionContext, BranchContextFact, CurrentSurfaceFact, ExistingQualityFacts,
        WorkstreamFact, ACTIVITY_RECAP_INPUTS_SCHEMA,
    };
    use super::super::activity_recap_segments::ActivitySegmentPromotionState;
    use super::*;

    fn empty_inputs() -> ActivityRecapInputs {
        ActivityRecapInputs {
            schema: ACTIVITY_RECAP_INPUTS_SCHEMA.to_string(),
            current_task_turn: None,
            decision_context: ActivityRecapDecisionContext {
                decision_id_seed: Some("decision-last-state".to_string()),
                mode: "normal".to_string(),
                lookback_ms: 60_000,
                evidence_watermark: Some("watermark".to_string()),
                output_mode: Some("continue_ready".to_string()),
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

    fn stitched_segment(
        id: &str,
        app: &str,
        title: &str,
        artifact_id: &str,
        role: ActivitySegmentRole,
        kinds: &[&str],
        at_ms: i64,
    ) -> StitchedActivitySegment {
        StitchedActivitySegment {
            segment_id: id.to_string(),
            start_ms: Some(at_ms),
            end_ms: Some(at_ms + 100),
            app_name: Some(app.to_string()),
            surface_title: Some(title.to_string()),
            artifact_kind: Some("window".to_string()),
            workstream_id: Some("ws-primary".to_string()),
            artifact_id: Some(artifact_id.to_string()),
            role,
            activity_kinds: kinds.iter().map(|value| value.to_string()).collect(),
            local_reason: "grounded test activity".to_string(),
            promotion_state: if role == ActivitySegmentRole::PromotedPrimary {
                ActivitySegmentPromotionState::Promoted
            } else {
                ActivitySegmentPromotionState::NotApplicable
            },
            confidence: ActivityEvidenceConfidence::High,
            evidence_anchor_ids: vec![format!("event-{id}")],
        }
    }

    fn timeline(
        primary: StitchedActivitySegment,
        current: StitchedActivitySegment,
        continuity: ActivityPrimaryContinuity,
    ) -> StitchedActivityTimeline {
        let ordered = if primary.segment_id == current.segment_id {
            vec![primary.clone()]
        } else {
            vec![primary.clone(), current.clone()]
        };
        StitchedActivityTimeline {
            primary_segment: Some(primary),
            current_segment: Some(current),
            ordered_segments: ordered,
            primary_continuity: continuity,
            confidence: ActivityConfidence::High,
            ..StitchedActivityTimeline::default()
        }
    }

    fn action(id: &str, artifact_id: &str, kind: &str, at_ms: i64) -> TaskActionFact {
        TaskActionFact {
            action_id: id.to_string(),
            task_turn_id: None,
            frame_id: format!("frame-{id}"),
            artifact_id: Some(artifact_id.to_string()),
            secondary_artifact_id: None,
            action_kind: kind.to_string(),
            action_role: "primary".to_string(),
            confidence: 0.92,
            created_at_ms: at_ms,
            semantic_delta_kind: Some("content_change".to_string()),
            semantic_subject: None,
            semantic_after_hint: None,
            evidence_source_kind: Some("local".to_string()),
            evidence_span_ids: vec![format!("span-{id}")],
            attribution_confidence: Some(0.9),
            quality_flags: Vec::new(),
            branch_kind: None,
            branch_action_role: None,
            branch_confidence: None,
            branch_reason_code: None,
        }
    }

    fn open_loop(state: &str) -> OpenLoopFact {
        OpenLoopFact {
            open_loop_id: "open-loop-primary".to_string(),
            workstream_id: "ws-primary".to_string(),
            task_turn_id: None,
            parent_task_turn_id: None,
            origin_task_turn_id: None,
            state: state.to_string(),
            boundary_kind: "continuation".to_string(),
            quality: "strong".to_string(),
            confidence: 0.9,
            origin_artifact_id: Some("primary-artifact".to_string()),
            current_focus_artifact_id: Some("primary-artifact".to_string()),
            primary_return_artifact_id: Some("primary-artifact".to_string()),
            resume_work_artifact_id: Some("primary-artifact".to_string()),
            blocker_artifact_id: None,
            verification_artifact_id: None,
            objective_hint: Some("Smalltalk P5 plan".to_string()),
            last_concrete_progress: Some("You outlined the last-state synthesis rules".to_string()),
            unfinished_state: None,
            next_evidence_backed_action: None,
            current_focus_relation: Some("same_as_origin".to_string()),
            relation_to_current_task: "same_task".to_string(),
            freshness: "current_turn".to_string(),
            eligible_for_primary: true,
            eligible_for_objective: true,
            eligible_for_last_state: true,
            eligibility_reason_codes: Vec::new(),
            missing_evidence: Vec::new(),
            evidence_spans: Vec::new(),
            last_updated_at_ms: 100,
        }
    }

    fn work_label() -> ActivityWorkLabelResult {
        ActivityWorkLabelResult {
            primary_work_label: Some("Writing the Smalltalk P5 plan".to_string()),
            primary_work_summary_seed: Some("Writing the Smalltalk P5 plan in Codex".to_string()),
            where_label: Some("Codex".to_string()),
            confidence: ActivityConfidence::High,
            evidence_spans: vec![ActivityEvidenceSpan {
                claim_key: "primary_work_label".to_string(),
                claim_text: "Writing the Smalltalk P5 plan".to_string(),
                anchor_type: ActivityEvidenceAnchorType::Action,
                anchor_ids: vec!["action-compose".to_string()],
                confidence: ActivityEvidenceConfidence::High,
                source: ActivityEvidenceSource::Local,
            }],
            ..ActivityWorkLabelResult::default()
        }
    }

    fn assert_claims_are_anchored(result: &LastStateRecapResult) {
        for (key, value) in [
            (CLAIM_LAST_STATE, result.last_meaningful_state.as_deref()),
            (CLAIM_UNFINISHED, result.unfinished_state.as_deref()),
            (CLAIM_NEXT_ACTION, result.next_action_summary.as_deref()),
            (CLAIM_WHY_TARGET, result.why_this_target.as_deref()),
            (CLAIM_WHY_NO_TARGET, result.why_no_safe_target.as_deref()),
        ] {
            if let Some(value) = value {
                assert!(claim_has_span(&result.evidence_spans, key, value));
            }
        }
    }

    #[test]
    fn active_composer_produces_grounded_active_draft_and_next_action() {
        let mut inputs = empty_inputs();
        inputs.recent_actions.push(action(
            "action-compose",
            "primary-artifact",
            "composing",
            50,
        ));
        inputs.open_loops.push(open_loop("active"));
        let primary = stitched_segment(
            "primary",
            "Codex",
            "Smalltalk research chat",
            "primary-artifact",
            ActivitySegmentRole::Primary,
            &["composing"],
            0,
        );
        let result = synthesize_last_state(
            &inputs,
            &timeline(
                primary.clone(),
                primary,
                ActivityPrimaryContinuity::Continuous,
            ),
            &work_label(),
            &DetourRecapResult::default(),
        );

        assert_eq!(result.current_state, ActivityCurrentState::ActivelyWorking);
        assert_eq!(
            result.unfinished_state.as_deref(),
            Some("A draft or edit was still active.")
        );
        assert_eq!(
            result.next_action_summary.as_deref(),
            Some("Continue writing or editing in Codex.")
        );
        assert_claims_are_anchored(&result);
    }

    #[test]
    fn finder_detour_preserves_primary_last_state_and_reports_recent_detour() {
        let mut inputs = empty_inputs();
        inputs.open_loops.push(open_loop("open"));
        let primary = stitched_segment(
            "primary",
            "Codex",
            "Smalltalk research chat",
            "primary-artifact",
            ActivitySegmentRole::Primary,
            &["planning"],
            0,
        );
        let finder = stitched_segment(
            "finder",
            "Finder",
            "Photos",
            "photo-artifact",
            ActivitySegmentRole::Detour,
            &["browsing"],
            120,
        );
        let detours = DetourRecapResult {
            current_state: Some(ActivityCurrentState::RecentlyDetoured),
            recent_detours: vec![ActivityDetourSummary {
                surface_title: Some("Photos".to_string()),
                app_name: Some("Finder".to_string()),
                role: ActivityDetourRole::Detour,
                activity_label: Some("file browsing".to_string()),
                reason: "Brief file browsing without unfinished file work.".to_string(),
                start_ms: Some(120),
                end_ms: Some(140),
                confidence: ActivityEvidenceConfidence::High,
                evidence_anchor_ids: vec!["event-finder".to_string()],
            }],
            ..DetourRecapResult::default()
        };
        let result = synthesize_last_state(
            &inputs,
            &timeline(
                primary,
                finder,
                ActivityPrimaryContinuity::BranchedWithoutReturn,
            ),
            &work_label(),
            &detours,
        );

        assert_eq!(result.current_state, ActivityCurrentState::RecentlyDetoured);
        assert_eq!(
            result.last_meaningful_state.as_deref(),
            Some("You outlined the last-state synthesis rules.")
        );
        assert!(result
            .next_action_summary
            .as_deref()
            .is_some_and(|value| value.starts_with("Return to Codex")));
        assert_claims_are_anchored(&result);
    }

    #[test]
    fn terminal_error_becomes_blocked_with_a_safe_inspection_action() {
        let mut inputs = empty_inputs();
        let mut loop_fact = open_loop("blocked");
        loop_fact.blocker_artifact_id = Some("terminal-artifact".to_string());
        loop_fact.last_concrete_progress = Some("You ran the latest local check".to_string());
        inputs.open_loops.push(loop_fact);
        let terminal = stitched_segment(
            "terminal",
            "Terminal",
            "Build output",
            "terminal-artifact",
            ActivitySegmentRole::PromotedPrimary,
            &["error"],
            0,
        );
        let result = synthesize_last_state(
            &inputs,
            &timeline(
                terminal.clone(),
                terminal,
                ActivityPrimaryContinuity::NewPrimary,
            ),
            &ActivityWorkLabelResult::default(),
            &DetourRecapResult::default(),
        );

        assert_eq!(result.current_state, ActivityCurrentState::Blocked);
        assert_eq!(
            result.unfinished_state.as_deref(),
            Some("A visible error or blocker remained unresolved.")
        );
        assert_eq!(
            result.next_action_summary.as_deref(),
            Some("Inspect the visible error before rerunning or continuing.")
        );
        assert_claims_are_anchored(&result);
    }

    #[test]
    fn docs_support_branch_returns_to_editor_without_promoting_docs() {
        let mut inputs = empty_inputs();
        inputs.open_loops.push(open_loop("open"));
        inputs.branch_contexts.push(BranchContextFact {
            branch_id: "branch-docs".to_string(),
            branch_action_id: "action-docs".to_string(),
            task_turn_id: None,
            origin_task_turn_id: None,
            promotion_task_turn_id: None,
            promoted_at_ms: None,
            origin_artifact_id: Some("primary-artifact".to_string()),
            origin_workstream_id: Some("ws-primary".to_string()),
            branch_artifact_id: "docs-artifact".to_string(),
            branch_kind: "docs_support".to_string(),
            branch_started_at_ms: 120,
            last_branch_seen_at_ms: 140,
            returned_to_origin_at_ms: None,
            promotion_state: "unpromoted".to_string(),
            promotion_reason: None,
            confidence: 0.9,
            reason_code: None,
            evidence_action_ids: vec!["action-docs".to_string()],
            promotion_evidence_action_ids: Vec::new(),
            eligible_feedback_event_ids: Vec::new(),
            feedback_rejection_reasons: Vec::new(),
            updated_at_ms: 140,
        });
        let primary = stitched_segment(
            "editor",
            "Code",
            "Smalltalk source",
            "primary-artifact",
            ActivitySegmentRole::Primary,
            &["editing"],
            0,
        );
        let docs = stitched_segment(
            "docs",
            "Browser",
            "Documentation",
            "docs-artifact",
            ActivitySegmentRole::Support,
            &["reading"],
            120,
        );
        let result = synthesize_last_state(
            &inputs,
            &timeline(
                primary,
                docs,
                ActivityPrimaryContinuity::BranchedWithoutReturn,
            ),
            &ActivityWorkLabelResult {
                where_label: Some("the editor".to_string()),
                ..work_label()
            },
            &DetourRecapResult {
                current_state: Some(ActivityCurrentState::RecentlyDetoured),
                ..DetourRecapResult::default()
            },
        );

        assert_eq!(result.current_state, ActivityCurrentState::RecentlyDetoured);
        assert_eq!(
            result.next_action_summary.as_deref(),
            Some("Return to the editor; the exact next task step is not visible.")
        );
        assert!(inputs.return_target.is_none());
        assert_claims_are_anchored(&result);
    }

    #[test]
    fn weak_codex_surface_keeps_identity_thin_and_invents_no_target() {
        let mut inputs = empty_inputs();
        inputs.current_surface = Some(CurrentSurfaceFact {
            surface_id: "surface-codex".to_string(),
            artifact_id: None,
            app_name: Some("Codex".to_string()),
            display_title: None,
            domain: Some("codex".to_string()),
            activity_state: None,
            task_state: None,
            observed_at_ms: 100,
            evidence_quality: "thin".to_string(),
            openability: "unknown".to_string(),
            focus_confidence: 0.6,
            identity_confidence: Some(0.3),
            snapshot_id: None,
            evidence_ids: vec!["event-codex".to_string()],
            missing_evidence: vec!["task_identity".to_string()],
            claim_eligible: true,
        });
        let current = stitched_segment(
            "codex",
            "Codex",
            "Codex",
            "weak-artifact",
            ActivitySegmentRole::CurrentFocusOnly,
            &[],
            100,
        );
        let timeline = StitchedActivityTimeline {
            current_segment: Some(current.clone()),
            ordered_segments: vec![current],
            confidence: ActivityConfidence::Low,
            ..StitchedActivityTimeline::default()
        };
        let result = synthesize_last_state(
            &inputs,
            &timeline,
            &ActivityWorkLabelResult::default(),
            &DetourRecapResult::default(),
        );

        assert_eq!(result.current_state, ActivityCurrentState::Unclear);
        assert_eq!(
            result.last_meaningful_state.as_deref(),
            Some("I saw recent work in Codex, but the exact task state was not visible.")
        );
        assert_eq!(
            result.next_action_summary.as_deref(),
            Some("Review the latest Codex surface; the exact task identity is thin.")
        );
        assert!(result.why_this_target.is_none());
        assert_claims_are_anchored(&result);
    }

    #[test]
    fn no_open_loop_or_action_does_not_invent_unfinished_work() {
        let inputs = empty_inputs();
        let result = synthesize_last_state(
            &inputs,
            &StitchedActivityTimeline::default(),
            &ActivityWorkLabelResult::default(),
            &DetourRecapResult::default(),
        );

        assert_eq!(result.current_state, ActivityCurrentState::Unclear);
        assert!(result.last_meaningful_state.is_none());
        assert!(result.unfinished_state.is_none());
        assert!(result.next_action_summary.is_none());
        assert!(result.why_this_target.is_none());
        assert!(result.why_no_safe_target.is_none());
        assert_eq!(
            result.validation_status,
            ActivityRecapValidationStatus::Thin
        );
    }

    #[test]
    fn apply_stage_preserves_detours_and_drops_unanchored_public_claims() {
        let recap = ContinueActivityRecap {
            primary_work_summary: Some("Unanchored primary summary".to_string()),
            recent_detours: vec![ActivityDetourSummary {
                surface_title: Some("Finder".to_string()),
                app_name: Some("Finder".to_string()),
                role: ActivityDetourRole::Detour,
                activity_label: None,
                reason: "Brief file browsing remained a detour.".to_string(),
                start_ms: None,
                end_ms: None,
                confidence: ActivityEvidenceConfidence::Medium,
                evidence_anchor_ids: vec!["event-finder".to_string()],
            }],
            ..ContinueActivityRecap::default()
        };
        let applied = apply_last_state_recap(
            recap,
            LastStateRecapResult {
                current_state: ActivityCurrentState::PausedAfterProgress,
                last_meaningful_state: Some("Grounded state.".to_string()),
                evidence_spans: vec![ActivityEvidenceSpan {
                    claim_key: CLAIM_LAST_STATE.to_string(),
                    claim_text: "Grounded state.".to_string(),
                    anchor_type: ActivityEvidenceAnchorType::Action,
                    anchor_ids: vec!["action-grounded".to_string()],
                    confidence: ActivityEvidenceConfidence::High,
                    source: ActivityEvidenceSource::Local,
                }],
                validation_status: ActivityRecapValidationStatus::Valid,
                ..LastStateRecapResult::default()
            },
        );

        assert!(applied.primary_work_summary.is_none());
        assert_eq!(applied.recent_detours.len(), 1);
        assert_eq!(
            applied.last_meaningful_state.as_deref(),
            Some("Grounded state.")
        );
        assert_eq!(
            applied.validation_status,
            ActivityRecapValidationStatus::Valid
        );
    }

    #[test]
    fn complete_open_loop_maps_to_complete_without_unfinished_state() {
        let mut inputs = empty_inputs();
        let mut loop_fact = open_loop("completed");
        loop_fact.unfinished_state = Some("This stale field must not appear".to_string());
        loop_fact.next_evidence_backed_action =
            Some("This stale action must not appear".to_string());
        inputs.open_loops.push(loop_fact);
        inputs.surface_snapshots.push(SurfaceSnapshotFact {
            snapshot_id: "snapshot-old-error".to_string(),
            artifact_id: Some("primary-artifact".to_string()),
            frame_id: Some(42),
            domain: "terminal".to_string(),
            app_name: Some("Terminal".to_string()),
            display_title: Some("Build output".to_string()),
            relative_file_name: None,
            git_branch: None,
            activity_state: Some("reviewing".to_string()),
            task_state: Some("error".to_string()),
            command_state: Some("failed".to_string()),
            has_error_markers: true,
            identity_confidence: "high".to_string(),
            evidence_quality: "strong".to_string(),
            openability: "unknown".to_string(),
            missing_evidence: Vec::new(),
            evidence_sources: vec!["native_metadata".to_string()],
            observed_at_ms: 50,
        });
        let primary = stitched_segment(
            "primary",
            "Codex",
            "Smalltalk research chat",
            "primary-artifact",
            ActivitySegmentRole::Primary,
            &[],
            0,
        );
        let result = synthesize_last_state(
            &inputs,
            &timeline(
                primary.clone(),
                primary,
                ActivityPrimaryContinuity::Continuous,
            ),
            &work_label(),
            &DetourRecapResult::default(),
        );

        assert_eq!(result.current_state, ActivityCurrentState::CompleteOrIdle);
        assert!(result.unfinished_state.is_none());
        assert!(result.next_action_summary.is_none());
    }

    #[test]
    fn workstream_state_snapshot_can_ground_paused_after_progress() {
        let mut inputs = empty_inputs();
        inputs.workstream_states.push(WorkstreamStateFact {
            snapshot_id: "state-primary".to_string(),
            workstream_id: "ws-primary".to_string(),
            task_turn_id: None,
            observed_at_ms: 90,
            state: "ready_to_resume".to_string(),
            previous_state: Some("editing".to_string()),
            confidence: 0.88,
            transition_kind: Some("idle_after_progress".to_string()),
            has_blocker: false,
            evidence_action_ids: vec!["action-progress".to_string()],
            missing_evidence: vec!["exact_next_action".to_string()],
        });
        let primary = stitched_segment(
            "primary",
            "Code",
            "Smalltalk source",
            "primary-artifact",
            ActivitySegmentRole::Primary,
            &[],
            0,
        );
        let result = synthesize_last_state(
            &inputs,
            &timeline(
                primary.clone(),
                primary,
                ActivityPrimaryContinuity::Continuous,
            ),
            &work_label(),
            &DetourRecapResult::default(),
        );

        assert_eq!(
            result.current_state,
            ActivityCurrentState::PausedAfterProgress
        );
        assert_eq!(
            result.last_meaningful_state.as_deref(),
            Some("The selected workstream was paused after recent progress.")
        );
        assert!(result.evidence_spans.iter().any(|span| {
            span.claim_key == CLAIM_LAST_STATE
                && span.anchor_type == ActivityEvidenceAnchorType::Workstream
                && span.anchor_ids == vec!["action-progress"]
        }));
    }

    #[test]
    fn target_reason_requires_an_existing_grounded_target_anchor() {
        let mut inputs = empty_inputs();
        inputs.resume_work_target = Some(TargetFact {
            artifact_id: "primary-artifact".to_string(),
            artifact_kind: "conversation".to_string(),
            display_title: Some("Smalltalk research chat".to_string()),
            openability: "openable".to_string(),
            evidence_quality: "strong".to_string(),
            identity_confidence: 0.9,
            evidence_frame_id: Some("frame-target".to_string()),
        });
        let primary = stitched_segment(
            "primary",
            "Codex",
            "Smalltalk research chat",
            "primary-artifact",
            ActivitySegmentRole::Primary,
            &["reviewing"],
            0,
        );
        let result = synthesize_last_state(
            &inputs,
            &timeline(
                primary.clone(),
                primary,
                ActivityPrimaryContinuity::Continuous,
            ),
            &work_label(),
            &DetourRecapResult::default(),
        );

        assert!(result.why_this_target.is_some());
        assert!(result.why_no_safe_target.is_none());
        assert!(result.evidence_spans.iter().any(|span| {
            span.claim_key == CLAIM_WHY_TARGET
                && span.anchor_type == ActivityEvidenceAnchorType::Frame
                && span.anchor_ids == vec!["frame-target"]
        }));
    }

    #[test]
    fn selected_workstream_open_loop_wins_over_a_newer_unrelated_loop() {
        let mut inputs = empty_inputs();
        inputs.selected_workstream = Some(WorkstreamFact {
            workstream_id: "ws-primary".to_string(),
            state: "active".to_string(),
            title: Some("Smalltalk".to_string()),
            primary_artifact_id: Some("primary-artifact".to_string()),
            last_active_timestamp_ms: 100,
            confidence: 0.9,
            unresolved_signal: Some("Work remains open".to_string()),
        });
        let selected = open_loop("open");
        let mut unrelated = open_loop("blocked");
        unrelated.open_loop_id = "open-loop-unrelated".to_string();
        unrelated.workstream_id = "ws-unrelated".to_string();
        unrelated.origin_artifact_id = Some("unrelated-artifact".to_string());
        unrelated.current_focus_artifact_id = Some("unrelated-artifact".to_string());
        unrelated.last_concrete_progress = Some("Unrelated progress".to_string());
        unrelated.last_updated_at_ms = 200;
        inputs.open_loops = vec![unrelated, selected];
        let primary = stitched_segment(
            "primary",
            "Codex",
            "Smalltalk",
            "primary-artifact",
            ActivitySegmentRole::Primary,
            &[],
            0,
        );
        let result = synthesize_last_state(
            &inputs,
            &timeline(
                primary.clone(),
                primary,
                ActivityPrimaryContinuity::Continuous,
            ),
            &work_label(),
            &DetourRecapResult::default(),
        );

        assert_eq!(
            result.last_meaningful_state.as_deref(),
            Some("You outlined the last-state synthesis rules.")
        );
        assert_ne!(result.current_state, ActivityCurrentState::Blocked);
    }
}
