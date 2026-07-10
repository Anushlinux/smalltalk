use super::activity_recap::{ActivityConfidence, ActivityEvidenceConfidence};
use super::activity_recap_inputs::{
    ActivityRecapInputs, ActivitySegmentFact, BranchContextFact, CurrentSurfaceFact,
    SemanticMomentFact,
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

const MAX_STITCHED_SEGMENTS: usize = 6;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ActivitySegmentRole {
    Primary,
    Support,
    Detour,
    Interrupt,
    Return,
    CurrentFocusOnly,
    PromotedPrimary,
    Unclear,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ActivitySegmentPromotionState {
    NotApplicable,
    NotPromoted,
    Promoted,
    Blocked,
    Unclear,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ActivityPrimaryContinuity {
    Continuous,
    InterruptedThenReturned,
    BranchedWithoutReturn,
    NewPrimary,
    Unclear,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct StitchedActivitySegment {
    pub segment_id: String,
    pub start_ms: Option<i64>,
    pub end_ms: Option<i64>,
    pub app_name: Option<String>,
    pub surface_title: Option<String>,
    pub artifact_kind: Option<String>,
    pub workstream_id: Option<String>,
    pub artifact_id: Option<String>,
    pub role: ActivitySegmentRole,
    pub activity_kinds: Vec<String>,
    pub local_reason: String,
    pub promotion_state: ActivitySegmentPromotionState,
    pub confidence: ActivityEvidenceConfidence,
    pub evidence_anchor_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct StitchedActivityTimeline {
    pub primary_segment: Option<StitchedActivitySegment>,
    pub current_segment: Option<StitchedActivitySegment>,
    pub ordered_segments: Vec<StitchedActivitySegment>,
    pub recent_detours: Vec<StitchedActivitySegment>,
    pub support_segments: Vec<StitchedActivitySegment>,
    pub interruptions: Vec<StitchedActivitySegment>,
    pub returned_to_primary: bool,
    pub primary_continuity: ActivityPrimaryContinuity,
    pub confidence: ActivityConfidence,
    pub warnings: Vec<String>,
}

impl Default for StitchedActivityTimeline {
    fn default() -> Self {
        Self {
            primary_segment: None,
            current_segment: None,
            ordered_segments: Vec::new(),
            recent_detours: Vec::new(),
            support_segments: Vec::new(),
            interruptions: Vec::new(),
            returned_to_primary: false,
            primary_continuity: ActivityPrimaryContinuity::Unclear,
            confidence: ActivityConfidence::None,
            warnings: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct SegmentSignals {
    selected_primary: bool,
    selected_candidate: bool,
    open_loop_primary: bool,
    direct_primary_action: bool,
    branch_origin: bool,
    branch_related: bool,
    structured_classification: bool,
    grounded_current_work: bool,
    surface_snapshot: bool,
}

impl SegmentSignals {
    fn independent_count(self) -> usize {
        [
            self.selected_primary,
            self.selected_candidate,
            self.open_loop_primary,
            self.direct_primary_action,
            self.branch_origin,
            self.branch_related,
            self.structured_classification,
            self.grounded_current_work,
            self.surface_snapshot,
        ]
        .into_iter()
        .filter(|value| *value)
        .count()
    }

    fn has_primary_signal(self) -> bool {
        self.selected_primary
            || self.selected_candidate
            || self.open_loop_primary
            || self.direct_primary_action
            || self.branch_origin
            || self.grounded_current_work
    }
}

/// Converts bounded graph facts into a compact, chronological activity timeline.
///
/// This is deliberately a pure transform. Branch promotion state is treated as
/// authoritative, current focus is kept separate from primary work, and no raw
/// database content or public recap prose is introduced here.
pub(crate) fn stitch_activity_segments(inputs: &ActivityRecapInputs) -> StitchedActivityTimeline {
    let mut warnings = inputs.input_warnings.clone();
    dedupe_strings(&mut warnings);

    let mut facts = inputs.recent_segments.iter().collect::<Vec<_>>();
    facts.sort_by(|left, right| {
        left.started_at_ms
            .cmp(&right.started_at_ms)
            .then_with(|| left.ended_at_ms.cmp(&right.ended_at_ms))
            .then_with(|| left.segment_id.cmp(&right.segment_id))
    });

    let mut seen_segment_ids = HashSet::new();
    let mut segments = facts
        .into_iter()
        .filter(|segment| seen_segment_ids.insert(segment.segment_id.clone()))
        .map(|segment| classify_segment(inputs, segment, &mut warnings))
        .collect::<Vec<_>>();

    ensure_current_surface_segment(inputs, &mut segments, &mut warnings);
    segments.sort_by(stitched_segment_order);
    mark_explicit_returns(inputs, &mut segments);
    let mut merged = merge_adjacent_segments(inputs, segments);
    merged.sort_by(stitched_segment_order);

    let uncapped_primary_index = select_primary_index(&merged);
    let uncapped_current_index = select_current_index(inputs, &merged);
    let returned_to_primary = merged
        .iter()
        .any(|segment| segment.role == ActivitySegmentRole::Return);
    if !returned_to_primary
        && inputs
            .branch_contexts
            .iter()
            .any(|branch| branch.returned_to_origin_at_ms.is_some())
    {
        warnings.push("activity_segments:return_without_grounded_segment".to_string());
    }
    let primary_continuity =
        determine_primary_continuity(&merged, uncapped_primary_index, returned_to_primary);

    if merged.len() > MAX_STITCHED_SEGMENTS {
        warnings.push("activity_segments:timeline_capped".to_string());
        merged = cap_relevant_segments(
            merged,
            uncapped_primary_index,
            uncapped_current_index,
            MAX_STITCHED_SEGMENTS,
        );
    }

    let primary_index = select_primary_index(&merged);
    let current_index = select_current_index(inputs, &merged);
    let primary_segment = primary_index.and_then(|index| merged.get(index).cloned());
    let current_segment = current_index.and_then(|index| merged.get(index).cloned());

    if primary_segment.is_none() && !merged.is_empty() {
        warnings.push("activity_segments:no_primary_segment".to_string());
    }
    if current_segment
        .as_ref()
        .is_some_and(|segment| segment.role == ActivitySegmentRole::CurrentFocusOnly)
    {
        warnings.push("activity_segments:current_focus_only".to_string());
    }
    dedupe_strings(&mut warnings);

    let recent_detours = role_subset(&merged, ActivitySegmentRole::Detour);
    let support_segments = role_subset(&merged, ActivitySegmentRole::Support);
    let interruptions = role_subset(&merged, ActivitySegmentRole::Interrupt);
    let confidence = timeline_confidence(primary_segment.as_ref(), current_segment.as_ref());

    StitchedActivityTimeline {
        primary_segment,
        current_segment,
        ordered_segments: merged,
        recent_detours,
        support_segments,
        interruptions,
        returned_to_primary,
        primary_continuity,
        confidence,
        warnings,
    }
}

fn classify_segment(
    inputs: &ActivityRecapInputs,
    segment: &ActivitySegmentFact,
    warnings: &mut Vec<String>,
) -> StitchedActivitySegment {
    let branch = latest_branch_for_artifact(inputs, segment.artifact_id.as_deref());
    let candidate_branch_policy = candidate_branch_policy_for_segment(inputs, segment);
    let promotion_state = branch
        .map(|value| promotion_state(&value.promotion_state, warnings))
        .or_else(|| {
            candidate_branch_policy.map(|value| {
                value
                    .branch_promotion_state
                    .as_deref()
                    .map(|state| promotion_state(state, warnings))
                    .unwrap_or(ActivitySegmentPromotionState::NotPromoted)
            })
        })
        .unwrap_or(ActivitySegmentPromotionState::NotApplicable);
    let signals = segment_signals(inputs, segment, branch);
    let role = classify_role(
        inputs,
        segment,
        branch,
        candidate_branch_policy.is_some(),
        promotion_state,
        signals,
    );
    let confidence = segment_confidence(segment, branch, promotion_state, signals);
    let mut activity_kinds = Vec::new();
    push_optional_string(&mut activity_kinds, segment.activity_intent.as_deref());
    for action in matching_actions(inputs, segment) {
        push_string_once(&mut activity_kinds, &action.action_kind);
    }
    for snapshot in inputs
        .surface_snapshots
        .iter()
        .filter(|snapshot| snapshot_matches_segment(snapshot.artifact_id.as_deref(), segment))
    {
        push_optional_string(&mut activity_kinds, snapshot.activity_state.as_deref());
        push_optional_string(&mut activity_kinds, snapshot.task_state.as_deref());
        push_optional_string(&mut activity_kinds, snapshot.command_state.as_deref());
    }
    activity_kinds.sort();
    activity_kinds.dedup();

    let mut evidence_anchor_ids = vec![segment.segment_id.clone()];
    push_optional_string(&mut evidence_anchor_ids, segment.episode_id.as_deref());
    collect_segment_anchors(inputs, segment, branch, &mut evidence_anchor_ids);
    evidence_anchor_ids.sort();
    evidence_anchor_ids.dedup();

    StitchedActivitySegment {
        segment_id: segment.segment_id.clone(),
        start_ms: Some(segment.started_at_ms),
        end_ms: Some(segment.ended_at_ms),
        app_name: segment.app_name.clone(),
        surface_title: segment.display_title.clone(),
        artifact_kind: artifact_kind_for_artifact(inputs, segment.artifact_id.as_deref()),
        workstream_id: segment.workstream_id.clone(),
        artifact_id: segment.artifact_id.clone(),
        role,
        activity_kinds,
        local_reason: local_reason(role, promotion_state, signals),
        promotion_state,
        confidence,
        evidence_anchor_ids,
    }
}

fn classify_role(
    inputs: &ActivityRecapInputs,
    segment: &ActivitySegmentFact,
    branch: Option<&BranchContextFact>,
    candidate_branch_policy: bool,
    promotion: ActivitySegmentPromotionState,
    signals: SegmentSignals,
) -> ActivitySegmentRole {
    if promotion == ActivitySegmentPromotionState::Promoted {
        return ActivitySegmentRole::PromotedPrimary;
    }

    if let Some(branch) = branch {
        if branch_is_interrupt(branch) || is_messaging_segment(segment) {
            return ActivitySegmentRole::Interrupt;
        }
        if branch.branch_kind == "current_focus_only" {
            return ActivitySegmentRole::CurrentFocusOnly;
        }
        if promotion == ActivitySegmentPromotionState::Blocked {
            return if branch_has_origin(branch) && branch_is_support(branch) {
                ActivitySegmentRole::Support
            } else {
                ActivitySegmentRole::CurrentFocusOnly
            };
        }
        if promotion == ActivitySegmentPromotionState::NotPromoted {
            return if branch_has_origin(branch) && branch_is_support(branch) {
                ActivitySegmentRole::Support
            } else if role_hint_is_interrupt(segment) {
                ActivitySegmentRole::Interrupt
            } else {
                ActivitySegmentRole::Detour
            };
        }
    }

    if candidate_branch_policy
        && matches!(
            promotion,
            ActivitySegmentPromotionState::NotPromoted | ActivitySegmentPromotionState::Blocked
        )
    {
        return if is_messaging_segment(segment) || role_hint_is_interrupt(segment) {
            ActivitySegmentRole::Interrupt
        } else if matches!(
            segment.continuation_role.as_deref(),
            Some("current_focus_only" | "needs_fresh_capture")
        ) {
            ActivitySegmentRole::CurrentFocusOnly
        } else {
            ActivitySegmentRole::Support
        };
    }

    if is_messaging_segment(segment) || role_hint_is_interrupt(segment) {
        return ActivitySegmentRole::Interrupt;
    }
    if matches!(
        segment.continuation_role.as_deref(),
        Some("divergence" | "background_consumption")
    ) {
        return ActivitySegmentRole::Detour;
    }
    if segment.continuation_role.as_deref() == Some("diagnostic_only") {
        return if signals.branch_related {
            ActivitySegmentRole::Support
        } else {
            ActivitySegmentRole::Detour
        };
    }
    if segment.continuation_role.as_deref() == Some("support_context") {
        return if signals.branch_related
            || segment.support_score.is_some_and(|score| score >= 0.45)
            || segment
                .workstream_id
                .as_deref()
                .is_some_and(|id| selected_workstream_matches(inputs, id))
        {
            ActivitySegmentRole::Support
        } else {
            ActivitySegmentRole::Detour
        };
    }
    if signals.has_primary_signal()
        && role_hint_allows_primary(segment.continuation_role.as_deref())
        && !weak_event_only_segment(segment)
    {
        return ActivitySegmentRole::Primary;
    }
    if signals.grounded_current_work
        && signals.has_primary_signal()
        && !matches!(
            segment.continuation_role.as_deref(),
            Some(
                "support_context"
                    | "divergence"
                    | "background_consumption"
                    | "interruption"
                    | "diagnostic_only"
            )
        )
    {
        return ActivitySegmentRole::Primary;
    }
    if signals.has_primary_signal()
        && matches!(
            segment.continuation_role.as_deref(),
            Some("resume_target" | "resume_target_if_unfinished")
        )
    {
        return ActivitySegmentRole::Primary;
    }
    if segment.app_family == "finder"
        && (inputs.selected_workstream.is_some() || timeline_has_primary_fact(inputs))
    {
        return ActivitySegmentRole::Detour;
    }
    if matches!(
        segment.continuation_role.as_deref(),
        Some("current_focus_only" | "needs_fresh_capture")
    ) || current_surface_matches_segment(inputs.current_surface.as_ref(), segment)
    {
        return ActivitySegmentRole::CurrentFocusOnly;
    }
    ActivitySegmentRole::Unclear
}

fn segment_signals(
    inputs: &ActivityRecapInputs,
    segment: &ActivitySegmentFact,
    branch: Option<&BranchContextFact>,
) -> SegmentSignals {
    let selected_primary = segment.artifact_id.as_deref().is_some_and(|artifact_id| {
        inputs
            .selected_workstream
            .as_ref()
            .is_some_and(|workstream| {
                workstream.primary_artifact_id.as_deref() == Some(artifact_id)
            })
    });
    let selected_candidate = inputs.selected_candidate.as_ref().is_some_and(|candidate| {
        candidate.activity_segment_id.as_deref() == Some(&segment.segment_id)
            || segment.artifact_id.as_deref().is_some_and(|artifact_id| {
                candidate.target_artifact_id.as_deref() == Some(artifact_id)
            })
    });
    let open_loop_primary = inputs.open_loops.iter().any(|open_loop| {
        let artifact_id = segment.artifact_id.as_deref();
        artifact_id.is_some()
            && [
                open_loop.origin_artifact_id.as_deref(),
                open_loop.primary_return_artifact_id.as_deref(),
                open_loop.resume_work_artifact_id.as_deref(),
            ]
            .contains(&artifact_id)
    });
    let direct_primary_action = matching_actions(inputs, segment).into_iter().any(|action| {
        direct_primary_action_kind(&action.action_kind)
            && action.confidence >= 0.55
            && !action_role_is_support_or_interrupt(
                action
                    .branch_action_role
                    .as_deref()
                    .unwrap_or(&action.action_role),
            )
    });
    let branch_origin = branch.is_none()
        && inputs.branch_contexts.iter().any(|value| {
            value.origin_artifact_id.as_deref() == segment.artifact_id.as_deref()
                && value.origin_artifact_id.is_some()
        });
    let branch_related = branch.is_some()
        || inputs.support_evidence.iter().any(|support| {
            support.artifact_id.as_deref() == segment.artifact_id.as_deref()
                || support.origin_artifact_id.as_deref() == segment.artifact_id.as_deref()
        });
    let structured_classification = !segment.is_event_backed_only
        && segment
            .evidence_sufficiency_score
            .is_some_and(|score| score >= 0.60)
        && segment.continuation_role.is_some();
    let grounded_current_work =
        current_surface_matches_segment(inputs.current_surface.as_ref(), segment)
            && inputs
                .current_surface
                .as_ref()
                .is_some_and(grounded_current_surface);
    let surface_snapshot = inputs.surface_snapshots.iter().any(|snapshot| {
        snapshot_matches_segment(snapshot.artifact_id.as_deref(), segment)
            && snapshot.evidence_quality != "thin"
            && (snapshot.activity_state.is_some()
                || snapshot.task_state.is_some()
                || snapshot.command_state.is_some()
                || snapshot.has_error_markers)
    });

    SegmentSignals {
        selected_primary,
        selected_candidate,
        open_loop_primary,
        direct_primary_action,
        branch_origin,
        branch_related,
        structured_classification,
        grounded_current_work,
        surface_snapshot,
    }
}

fn segment_confidence(
    segment: &ActivitySegmentFact,
    branch: Option<&BranchContextFact>,
    promotion: ActivitySegmentPromotionState,
    signals: SegmentSignals,
) -> ActivityEvidenceConfidence {
    let independent = signals.independent_count();
    let explicit_promotion = promotion == ActivitySegmentPromotionState::Promoted
        && branch.is_some_and(|value| value.confidence >= 0.70);
    let has_strong_structured_signal = signals.open_loop_primary
        || signals.direct_primary_action
        || signals.surface_snapshot
        || explicit_promotion;
    let missing_evidence = !segment.missing_evidence.is_empty();

    if weak_event_only_segment(segment) && !has_strong_structured_signal {
        return ActivityEvidenceConfidence::Low;
    }
    if independent >= 3 && !missing_evidence && !segment.is_event_backed_only {
        ActivityEvidenceConfidence::High
    } else if independent >= 1
        || has_strong_structured_signal
        || segment
            .evidence_sufficiency_score
            .is_some_and(|score| score >= 0.60)
    {
        ActivityEvidenceConfidence::Medium
    } else {
        ActivityEvidenceConfidence::Low
    }
}

fn ensure_current_surface_segment(
    inputs: &ActivityRecapInputs,
    segments: &mut Vec<StitchedActivitySegment>,
    warnings: &mut Vec<String>,
) {
    let Some(current) = inputs.current_surface.as_ref() else {
        return;
    };
    if segments
        .iter()
        .any(|segment| current_surface_matches_stitched(current, segment))
    {
        return;
    }

    let branch = latest_branch_for_artifact(inputs, current.artifact_id.as_deref());
    let candidate_branch_policy =
        candidate_branch_policy_for_artifact(inputs, current.artifact_id.as_deref(), None);
    let promotion = branch
        .map(|branch| promotion_state(&branch.promotion_state, warnings))
        .or_else(|| {
            candidate_branch_policy.map(|candidate| {
                candidate
                    .branch_promotion_state
                    .as_deref()
                    .map(|state| promotion_state(state, warnings))
                    .unwrap_or(ActivitySegmentPromotionState::NotPromoted)
            })
        })
        .unwrap_or(ActivitySegmentPromotionState::NotApplicable);
    let selected_primary = current.artifact_id.as_deref().is_some_and(|artifact_id| {
        inputs
            .selected_workstream
            .as_ref()
            .is_some_and(|workstream| {
                workstream.primary_artifact_id.as_deref() == Some(artifact_id)
            })
    });
    let role = if promotion == ActivitySegmentPromotionState::Promoted {
        ActivitySegmentRole::PromotedPrimary
    } else if current_app_is_messaging(current) || branch.is_some_and(branch_is_interrupt) {
        ActivitySegmentRole::Interrupt
    } else if candidate_branch_policy.is_some()
        && matches!(
            promotion,
            ActivitySegmentPromotionState::NotPromoted | ActivitySegmentPromotionState::Blocked
        )
    {
        ActivitySegmentRole::CurrentFocusOnly
    } else if selected_primary && grounded_current_surface(current) {
        ActivitySegmentRole::Primary
    } else if current_app_is_finder(current) && timeline_has_primary_fact(inputs) {
        ActivitySegmentRole::Detour
    } else {
        ActivitySegmentRole::CurrentFocusOnly
    };
    let mut anchors = current.evidence_ids.clone();
    push_optional_string(&mut anchors, current.snapshot_id.as_deref());
    anchors.push(current.surface_id.clone());
    if let Some(branch) = branch {
        anchors.push(branch.branch_id.clone());
        for action_id in &branch.evidence_action_ids {
            push_string_once(&mut anchors, action_id);
        }
    }
    anchors.sort();
    anchors.dedup();

    let confidence = if grounded_current_surface(current)
        && selected_primary
        && current.missing_evidence.is_empty()
        && current.evidence_ids.len() >= 2
    {
        ActivityEvidenceConfidence::High
    } else if grounded_current_surface(current)
        || promotion == ActivitySegmentPromotionState::Promoted
    {
        ActivityEvidenceConfidence::Medium
    } else {
        ActivityEvidenceConfidence::Low
    };
    let mut activity_kinds = Vec::new();
    push_optional_string(&mut activity_kinds, current.activity_state.as_deref());
    push_optional_string(&mut activity_kinds, current.task_state.as_deref());
    activity_kinds.sort();
    activity_kinds.dedup();

    segments.push(StitchedActivitySegment {
        segment_id: format!("current-surface:{}", current.surface_id),
        start_ms: Some(current.observed_at_ms),
        end_ms: Some(current.observed_at_ms),
        app_name: current.app_name.clone(),
        surface_title: current.display_title.clone(),
        artifact_kind: artifact_kind_for_artifact(inputs, current.artifact_id.as_deref()),
        workstream_id: inputs
            .selected_workstream
            .as_ref()
            .filter(|workstream| {
                current.artifact_id.as_deref().is_some_and(|artifact_id| {
                    workstream.primary_artifact_id.as_deref() == Some(artifact_id)
                })
            })
            .map(|workstream| workstream.workstream_id.clone()),
        artifact_id: current.artifact_id.clone(),
        role,
        activity_kinds,
        local_reason: match role {
            ActivitySegmentRole::Primary => {
                "Grounded current-work evidence matches the selected primary artifact.".to_string()
            }
            ActivitySegmentRole::PromotedPrimary => {
                "Explicit branch promotion makes the current surface primary work.".to_string()
            }
            ActivitySegmentRole::Detour => {
                "The factual current surface is unrelated to the known primary work.".to_string()
            }
            ActivitySegmentRole::Interrupt => {
                "Messaging or interrupt evidence marks the current surface as an interruption."
                    .to_string()
            }
            _ => "The latest factual focus lacks enough evidence to classify as work or support."
                .to_string(),
        },
        promotion_state: promotion,
        confidence,
        evidence_anchor_ids: anchors,
    });
}

fn mark_explicit_returns(inputs: &ActivityRecapInputs, segments: &mut [StitchedActivitySegment]) {
    for index in 0..segments.len() {
        if segments[index].role != ActivitySegmentRole::Primary {
            continue;
        }
        let prior_same_primary = segments[..index].iter().any(|prior| {
            matches!(
                prior.role,
                ActivitySegmentRole::Primary | ActivitySegmentRole::Return
            ) && same_primary_identity(prior, &segments[index])
        });
        let prior_branch = segments[..index].iter().any(|prior| {
            matches!(
                prior.role,
                ActivitySegmentRole::Support
                    | ActivitySegmentRole::Detour
                    | ActivitySegmentRole::Interrupt
                    | ActivitySegmentRole::CurrentFocusOnly
            )
        });
        if prior_same_primary
            && prior_branch
            && has_explicit_return_evidence(inputs, &segments[index])
        {
            segments[index].role = ActivitySegmentRole::Return;
            segments[index].local_reason =
                "Explicit return evidence reconnects this segment to the primary work.".to_string();
        }
    }
}

fn has_explicit_return_evidence(
    inputs: &ActivityRecapInputs,
    segment: &StitchedActivitySegment,
) -> bool {
    let start_ms = segment.start_ms.unwrap_or(i64::MIN);
    let end_ms = segment.end_ms.unwrap_or(i64::MAX);
    let branch_return = inputs.branch_contexts.iter().any(|branch| {
        branch.origin_artifact_id.as_deref() == segment.artifact_id.as_deref()
            && branch.returned_to_origin_at_ms.is_some_and(|returned_at| {
                returned_at <= end_ms && returned_at >= branch.branch_started_at_ms
            })
    });
    let action_return = inputs.recent_actions.iter().any(|action| {
        action.action_kind == "returning_to_origin"
            && action.artifact_id.as_deref() == segment.artifact_id.as_deref()
            && action.created_at_ms >= start_ms
            && action.created_at_ms <= end_ms
    });
    let moment_return = inputs.recent_moments.iter().any(|moment| {
        moment.boundary_kind.to_ascii_lowercase().contains("return")
            && moment.started_at_ms <= end_ms
            && moment.ended_at_ms >= start_ms
            && moment_artifact_matches(moment, segment.artifact_id.as_deref())
    });
    branch_return || action_return || moment_return
}

fn merge_adjacent_segments(
    inputs: &ActivityRecapInputs,
    segments: Vec<StitchedActivitySegment>,
) -> Vec<StitchedActivitySegment> {
    let mut merged: Vec<StitchedActivitySegment> = Vec::new();
    for segment in segments {
        let can_merge = merged.last().is_some_and(|previous| {
            previous.role == segment.role
                && same_merge_identity(previous, &segment)
                && !has_meaningful_boundary(inputs, previous, &segment)
        });
        if can_merge {
            let previous = merged.last_mut().expect("checked above");
            previous.start_ms = min_optional(previous.start_ms, segment.start_ms);
            previous.end_ms = max_optional(previous.end_ms, segment.end_ms);
            for kind in segment.activity_kinds {
                push_string_once(&mut previous.activity_kinds, &kind);
            }
            previous.activity_kinds.sort();
            for anchor in segment.evidence_anchor_ids {
                push_string_once(&mut previous.evidence_anchor_ids, &anchor);
            }
            previous.evidence_anchor_ids.sort();
            previous.confidence = max_evidence_confidence(previous.confidence, segment.confidence);
            if previous.local_reason != segment.local_reason {
                previous.local_reason = format!(
                    "{} {}",
                    previous.local_reason.trim_end_matches('.'),
                    segment.local_reason
                );
            }
        } else {
            merged.push(segment);
        }
    }
    merged
}

fn has_meaningful_boundary(
    inputs: &ActivityRecapInputs,
    left: &StitchedActivitySegment,
    right: &StitchedActivitySegment,
) -> bool {
    let left_end = left.end_ms.unwrap_or(i64::MIN);
    let right_start = right.start_ms.unwrap_or(i64::MAX);
    inputs.recent_moments.iter().any(|moment| {
        moment.started_at_ms <= right_start
            && moment.ended_at_ms >= left_end
            && (meaningful_boundary_kind(&moment.boundary_kind)
                || moment.pre_artifact_id != moment.post_artifact_id)
    })
}

fn meaningful_boundary_kind(value: &str) -> bool {
    !matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "" | "unknown" | "none"
    )
}

fn select_primary_index(segments: &[StitchedActivitySegment]) -> Option<usize> {
    segments
        .iter()
        .rposition(|segment| segment.role == ActivitySegmentRole::PromotedPrimary)
        .or_else(|| {
            segments
                .iter()
                .position(|segment| segment.role == ActivitySegmentRole::Primary)
        })
        .or_else(|| {
            segments
                .iter()
                .rposition(|segment| segment.role == ActivitySegmentRole::Return)
        })
}

fn select_current_index(
    inputs: &ActivityRecapInputs,
    segments: &[StitchedActivitySegment],
) -> Option<usize> {
    if let Some(current) = inputs.current_surface.as_ref() {
        if let Some(index) = segments
            .iter()
            .rposition(|segment| current_surface_matches_stitched(current, segment))
        {
            return Some(index);
        }
    }
    (!segments.is_empty()).then_some(segments.len().saturating_sub(1))
}

fn cap_relevant_segments(
    segments: Vec<StitchedActivitySegment>,
    primary_index: Option<usize>,
    current_index: Option<usize>,
    limit: usize,
) -> Vec<StitchedActivitySegment> {
    if segments.len() <= limit {
        return segments;
    }
    let mut keep = HashSet::new();
    if let Some(index) = primary_index {
        keep.insert(index);
    }
    if let Some(index) = current_index {
        keep.insert(index);
    }
    if let Some(index) = segments
        .iter()
        .rposition(|segment| segment.role == ActivitySegmentRole::Return)
    {
        keep.insert(index);
    }

    let mut ranked = (0..segments.len()).collect::<Vec<_>>();
    ranked.sort_by(|left, right| {
        segment_relevance(&segments[*right])
            .cmp(&segment_relevance(&segments[*left]))
            .then_with(|| right.cmp(left))
    });
    for index in ranked {
        if keep.len() >= limit {
            break;
        }
        keep.insert(index);
    }
    let mut indices = keep.into_iter().collect::<Vec<_>>();
    indices.sort_unstable();
    indices
        .into_iter()
        .filter_map(|index| segments.get(index).cloned())
        .collect()
}

fn segment_relevance(segment: &StitchedActivitySegment) -> u8 {
    match segment.role {
        ActivitySegmentRole::Return | ActivitySegmentRole::PromotedPrimary => 7,
        ActivitySegmentRole::Primary => 6,
        ActivitySegmentRole::Interrupt => 5,
        ActivitySegmentRole::Support => 4,
        ActivitySegmentRole::Detour => 3,
        ActivitySegmentRole::CurrentFocusOnly => 2,
        ActivitySegmentRole::Unclear => 1,
    }
}

fn determine_primary_continuity(
    segments: &[StitchedActivitySegment],
    primary_index: Option<usize>,
    returned_to_primary: bool,
) -> ActivityPrimaryContinuity {
    if returned_to_primary {
        return ActivityPrimaryContinuity::InterruptedThenReturned;
    }
    if segments
        .iter()
        .any(|segment| segment.role == ActivitySegmentRole::PromotedPrimary)
    {
        return ActivityPrimaryContinuity::NewPrimary;
    }
    let Some(primary_index) = primary_index else {
        return ActivityPrimaryContinuity::Unclear;
    };
    if segments[primary_index + 1..].iter().any(|segment| {
        matches!(
            segment.role,
            ActivitySegmentRole::Support
                | ActivitySegmentRole::Detour
                | ActivitySegmentRole::Interrupt
                | ActivitySegmentRole::CurrentFocusOnly
                | ActivitySegmentRole::Unclear
        )
    }) {
        ActivityPrimaryContinuity::BranchedWithoutReturn
    } else {
        ActivityPrimaryContinuity::Continuous
    }
}

fn timeline_confidence(
    primary: Option<&StitchedActivitySegment>,
    current: Option<&StitchedActivitySegment>,
) -> ActivityConfidence {
    match (primary, current) {
        (None, None) => ActivityConfidence::None,
        (None, Some(_)) => ActivityConfidence::Low,
        (Some(primary), Some(current)) => match (primary.confidence, current.confidence) {
            (ActivityEvidenceConfidence::High, ActivityEvidenceConfidence::High) => {
                ActivityConfidence::High
            }
            (ActivityEvidenceConfidence::Low, _) | (_, ActivityEvidenceConfidence::Low) => {
                ActivityConfidence::Low
            }
            _ => ActivityConfidence::Medium,
        },
        (Some(primary), None) => match primary.confidence {
            ActivityEvidenceConfidence::High => ActivityConfidence::High,
            ActivityEvidenceConfidence::Medium => ActivityConfidence::Medium,
            ActivityEvidenceConfidence::Low => ActivityConfidence::Low,
        },
    }
}

fn collect_segment_anchors(
    inputs: &ActivityRecapInputs,
    segment: &ActivitySegmentFact,
    branch: Option<&BranchContextFact>,
    anchors: &mut Vec<String>,
) {
    for action in matching_actions(inputs, segment) {
        push_string_once(anchors, &action.action_id);
        for span_id in &action.evidence_span_ids {
            push_string_once(anchors, span_id);
        }
    }
    for moment in inputs.recent_moments.iter().filter(|moment| {
        moment.started_at_ms <= segment.ended_at_ms
            && moment.ended_at_ms >= segment.started_at_ms
            && moment_artifact_matches(moment, segment.artifact_id.as_deref())
    }) {
        push_string_once(anchors, &moment.moment_id);
        for event_id in &moment.ordered_event_ids {
            push_string_once(anchors, event_id);
        }
    }
    for open_loop in inputs.open_loops.iter().filter(|open_loop| {
        let workstream_match = segment
            .workstream_id
            .as_deref()
            .is_some_and(|workstream_id| open_loop.workstream_id == workstream_id);
        let artifact_match = segment.artifact_id.as_deref().is_some_and(|artifact_id| {
            [
                open_loop.origin_artifact_id.as_deref(),
                open_loop.current_focus_artifact_id.as_deref(),
                open_loop.primary_return_artifact_id.as_deref(),
                open_loop.resume_work_artifact_id.as_deref(),
                open_loop.blocker_artifact_id.as_deref(),
                open_loop.verification_artifact_id.as_deref(),
            ]
            .contains(&Some(artifact_id))
        });
        workstream_match || artifact_match
    }) {
        push_string_once(anchors, &open_loop.open_loop_id);
        for span in &open_loop.evidence_spans {
            push_string_once(anchors, &span.evidence_id);
        }
    }
    if let Some(branch) = branch {
        push_string_once(anchors, &branch.branch_id);
        push_string_once(anchors, &branch.branch_action_id);
        for action_id in &branch.evidence_action_ids {
            push_string_once(anchors, action_id);
        }
    }
    for snapshot in inputs
        .surface_snapshots
        .iter()
        .filter(|snapshot| snapshot_matches_segment(snapshot.artifact_id.as_deref(), segment))
    {
        push_string_once(anchors, &snapshot.snapshot_id);
    }
    if current_surface_matches_segment(inputs.current_surface.as_ref(), segment) {
        if let Some(current) = inputs.current_surface.as_ref() {
            push_string_once(anchors, &current.surface_id);
            for evidence_id in &current.evidence_ids {
                push_string_once(anchors, evidence_id);
            }
            push_optional_string(anchors, current.snapshot_id.as_deref());
        }
    }
}

fn matching_actions<'a>(
    inputs: &'a ActivityRecapInputs,
    segment: &ActivitySegmentFact,
) -> Vec<&'a super::activity_recap_inputs::TaskActionFact> {
    let Some(artifact_id) = segment.artifact_id.as_deref() else {
        return Vec::new();
    };
    inputs
        .recent_actions
        .iter()
        .filter(|action| {
            (action.artifact_id.as_deref() == Some(artifact_id)
                || action.secondary_artifact_id.as_deref() == Some(artifact_id))
                && action.created_at_ms >= segment.started_at_ms
                && action.created_at_ms <= segment.ended_at_ms
        })
        .collect()
}

fn latest_branch_for_artifact<'a>(
    inputs: &'a ActivityRecapInputs,
    artifact_id: Option<&str>,
) -> Option<&'a BranchContextFact> {
    let artifact_id = artifact_id?;
    inputs
        .branch_contexts
        .iter()
        .filter(|branch| branch.branch_artifact_id == artifact_id)
        .max_by(|left, right| {
            left.last_branch_seen_at_ms
                .cmp(&right.last_branch_seen_at_ms)
                .then_with(|| left.updated_at_ms.cmp(&right.updated_at_ms))
                .then_with(|| left.branch_id.cmp(&right.branch_id))
        })
}

fn candidate_branch_policy_for_segment<'a>(
    inputs: &'a ActivityRecapInputs,
    segment: &ActivitySegmentFact,
) -> Option<&'a super::activity_recap_inputs::CandidateFact> {
    candidate_branch_policy_for_artifact(
        inputs,
        segment.artifact_id.as_deref(),
        Some(&segment.segment_id),
    )
}

fn candidate_branch_policy_for_artifact<'a>(
    inputs: &'a ActivityRecapInputs,
    artifact_id: Option<&str>,
    segment_id: Option<&str>,
) -> Option<&'a super::activity_recap_inputs::CandidateFact> {
    inputs.selected_candidate.as_ref().filter(|candidate| {
        let matches_segment =
            segment_id.is_some() && candidate.activity_segment_id.as_deref() == segment_id;
        let matches_artifact =
            artifact_id.is_some() && candidate.target_artifact_id.as_deref() == artifact_id;
        (matches_segment || matches_artifact)
            && (candidate.branch_promotion_state.is_some()
                || candidate.branch_public_return_eligible == Some(false))
    })
}

fn promotion_state(value: &str, warnings: &mut Vec<String>) -> ActivitySegmentPromotionState {
    if value == "unpromoted" {
        ActivitySegmentPromotionState::NotPromoted
    } else if matches!(
        value,
        "promoted_primary"
            | "promoted_blocker"
            | "promoted_user_corrected"
            | "promoted_user_accepted"
            | "promoted_sustained_work"
    ) {
        ActivitySegmentPromotionState::Promoted
    } else if matches!(
        value,
        "blocked_diagnostic_self" | "blocked_feedback_suppressed" | "blocked_thin_current_focus"
    ) {
        ActivitySegmentPromotionState::Blocked
    } else {
        warnings.push("activity_segments:unknown_promotion_state".to_string());
        ActivitySegmentPromotionState::Unclear
    }
}

fn local_reason(
    role: ActivitySegmentRole,
    promotion: ActivitySegmentPromotionState,
    signals: SegmentSignals,
) -> String {
    match role {
        ActivitySegmentRole::PromotedPrimary => {
            "Explicit branch promotion makes this segment primary work.".to_string()
        }
        ActivitySegmentRole::Primary
            if signals.open_loop_primary && signals.direct_primary_action =>
        {
            "An open loop and direct work identify this as primary work.".to_string()
        }
        ActivitySegmentRole::Primary if signals.selected_primary => {
            "This matches the selected workstream's primary artifact.".to_string()
        }
        ActivitySegmentRole::Primary if signals.direct_primary_action => {
            "Direct editing, composing, command, or review evidence identifies primary work."
                .to_string()
        }
        ActivitySegmentRole::Primary => {
            "Structured local evidence identifies this as primary work.".to_string()
        }
        ActivitySegmentRole::Support if promotion == ActivitySegmentPromotionState::Blocked => {
            "Branch policy blocks promotion, so this remains supporting context.".to_string()
        }
        ActivitySegmentRole::Support => {
            "Unpromoted branch or support evidence links this to the primary work.".to_string()
        }
        ActivitySegmentRole::Detour => {
            "This activity has no strong primary-work relation or promotion evidence.".to_string()
        }
        ActivitySegmentRole::Interrupt => {
            "Messaging or interrupt evidence marks this as an interruption.".to_string()
        }
        ActivitySegmentRole::Return => {
            "Explicit return evidence reconnects this segment to the primary work.".to_string()
        }
        ActivitySegmentRole::CurrentFocusOnly => {
            "The latest focus is factual but too thin to classify as primary or support."
                .to_string()
        }
        ActivitySegmentRole::Unclear => {
            "Available local evidence does not support a more specific activity role.".to_string()
        }
    }
}

fn artifact_kind_for_artifact(
    inputs: &ActivityRecapInputs,
    artifact_id: Option<&str>,
) -> Option<String> {
    let artifact_id = artifact_id?;
    inputs
        .return_target
        .as_ref()
        .filter(|target| target.artifact_id == artifact_id)
        .or_else(|| {
            inputs
                .resume_work_target
                .as_ref()
                .filter(|target| target.artifact_id == artifact_id)
        })
        .map(|target| target.artifact_kind.clone())
        .or_else(|| {
            inputs
                .support_evidence
                .iter()
                .find(|support| support.artifact_id.as_deref() == Some(artifact_id))
                .and_then(|support| support.artifact_kind.clone())
        })
}

fn current_surface_matches_segment(
    current: Option<&CurrentSurfaceFact>,
    segment: &ActivitySegmentFact,
) -> bool {
    current.is_some_and(|current| {
        (current.artifact_id.is_some()
            && current.artifact_id.as_deref() == segment.artifact_id.as_deref())
            || (current.app_name.as_deref() == segment.app_name.as_deref()
                && current.display_title.as_deref() == segment.display_title.as_deref())
    })
}

fn current_surface_matches_stitched(
    current: &CurrentSurfaceFact,
    segment: &StitchedActivitySegment,
) -> bool {
    (current.artifact_id.is_some()
        && current.artifact_id.as_deref() == segment.artifact_id.as_deref())
        || (current.app_name.as_deref() == segment.app_name.as_deref()
            && current.display_title.as_deref() == segment.surface_title.as_deref())
}

fn grounded_current_surface(current: &CurrentSurfaceFact) -> bool {
    if !current.claim_eligible || current.evidence_quality == "thin" {
        return false;
    }
    [
        current.activity_state.as_deref(),
        current.task_state.as_deref(),
    ]
    .into_iter()
    .flatten()
    .any(|state| {
        matches!(
            state.trim().to_ascii_lowercase().as_str(),
            "actively_working"
                | "actively_editing"
                | "editing"
                | "composing"
                | "composing_prompt"
                | "running_command"
                | "command_running"
                | "reviewing_output"
                | "reviewing_diff"
                | "encountering_error"
                | "visible_error_unresolved"
        )
    })
}

fn role_hint_allows_primary(role: Option<&str>) -> bool {
    matches!(role, Some("resume_target" | "resume_target_if_unfinished")) || role.is_none()
}

fn role_hint_is_interrupt(segment: &ActivitySegmentFact) -> bool {
    segment.continuation_role.as_deref() == Some("interruption")
}

fn weak_event_only_segment(segment: &ActivitySegmentFact) -> bool {
    segment.is_event_backed_only
        && !segment.has_heavy_frame
        && !segment.has_visible_text
        && segment
            .evidence_sufficiency_score
            .is_none_or(|score| score < 0.55)
}

fn direct_primary_action_kind(value: &str) -> bool {
    matches!(
        value,
        "editing"
            | "composing"
            | "running_command"
            | "observing_command_output"
            | "reviewing_output"
            | "encountering_error"
            | "returning_to_origin"
    )
}

fn action_role_is_support_or_interrupt(value: &str) -> bool {
    matches!(
        value,
        "support" | "branch" | "interrupt" | "diagnostic" | "current_focus_only"
    )
}

fn selected_workstream_matches(inputs: &ActivityRecapInputs, workstream_id: &str) -> bool {
    inputs
        .selected_workstream
        .as_ref()
        .is_some_and(|workstream| workstream.workstream_id == workstream_id)
}

fn timeline_has_primary_fact(inputs: &ActivityRecapInputs) -> bool {
    inputs
        .selected_workstream
        .as_ref()
        .and_then(|workstream| workstream.primary_artifact_id.as_ref())
        .is_some()
        || inputs.open_loops.iter().any(|open_loop| {
            open_loop.origin_artifact_id.is_some()
                || open_loop.primary_return_artifact_id.is_some()
                || open_loop.resume_work_artifact_id.is_some()
        })
}

fn branch_is_interrupt(branch: &BranchContextFact) -> bool {
    matches!(
        branch.branch_kind.as_str(),
        "message_interrupt" | "messaging_interrupt" | "interrupt"
    )
}

fn branch_is_support(branch: &BranchContextFact) -> bool {
    matches!(
        branch.branch_kind.as_str(),
        "support"
            | "search_branch"
            | "documentation_reference"
            | "source_evidence"
            | "terminal_support_output"
            | "tool_or_agent_output"
            | "verification_branch"
            | "diagnostic_self"
            | "unknown_support"
    )
}

fn branch_has_origin(branch: &BranchContextFact) -> bool {
    branch.origin_artifact_id.is_some() && branch.reason_code.as_deref() != Some("branch:no_origin")
}

fn is_messaging_segment(segment: &ActivitySegmentFact) -> bool {
    segment.app_family == "messaging"
        || segment
            .app_name
            .as_deref()
            .is_some_and(|name| name.to_ascii_lowercase().contains("gmail"))
}

fn current_app_is_messaging(current: &CurrentSurfaceFact) -> bool {
    current.app_name.as_deref().is_some_and(|name| {
        let name = name.to_ascii_lowercase();
        name.contains("gmail") || name.contains("mail") || name.contains("messages")
    })
}

fn current_app_is_finder(current: &CurrentSurfaceFact) -> bool {
    current
        .app_name
        .as_deref()
        .is_some_and(|name| name.eq_ignore_ascii_case("finder"))
}

fn snapshot_matches_segment(
    snapshot_artifact_id: Option<&str>,
    segment: &ActivitySegmentFact,
) -> bool {
    snapshot_artifact_id.is_some() && snapshot_artifact_id == segment.artifact_id.as_deref()
}

fn moment_artifact_matches(moment: &SemanticMomentFact, artifact_id: Option<&str>) -> bool {
    artifact_id.is_some()
        && [
            moment.pre_artifact_id.as_deref(),
            moment.post_artifact_id.as_deref(),
            moment.dominant_artifact_id.as_deref(),
        ]
        .contains(&artifact_id)
}

fn same_primary_identity(left: &StitchedActivitySegment, right: &StitchedActivitySegment) -> bool {
    if left.artifact_id.is_some() || right.artifact_id.is_some() {
        left.artifact_id == right.artifact_id
    } else if left.workstream_id.is_some() || right.workstream_id.is_some() {
        left.workstream_id == right.workstream_id
    } else {
        left.app_name == right.app_name && left.surface_title == right.surface_title
    }
}

fn same_merge_identity(left: &StitchedActivitySegment, right: &StitchedActivitySegment) -> bool {
    left.artifact_id == right.artifact_id
        && left.workstream_id == right.workstream_id
        && ((left.artifact_id.is_some() || left.workstream_id.is_some())
            || (left.app_name == right.app_name && left.surface_title == right.surface_title))
}

fn stitched_segment_order(
    left: &StitchedActivitySegment,
    right: &StitchedActivitySegment,
) -> std::cmp::Ordering {
    left.start_ms
        .unwrap_or(i64::MIN)
        .cmp(&right.start_ms.unwrap_or(i64::MIN))
        .then_with(|| {
            left.end_ms
                .unwrap_or(i64::MIN)
                .cmp(&right.end_ms.unwrap_or(i64::MIN))
        })
        .then_with(|| left.segment_id.cmp(&right.segment_id))
}

fn role_subset(
    segments: &[StitchedActivitySegment],
    role: ActivitySegmentRole,
) -> Vec<StitchedActivitySegment> {
    segments
        .iter()
        .filter(|segment| segment.role == role)
        .cloned()
        .collect()
}

fn max_evidence_confidence(
    left: ActivityEvidenceConfidence,
    right: ActivityEvidenceConfidence,
) -> ActivityEvidenceConfidence {
    match (left, right) {
        (ActivityEvidenceConfidence::High, _) | (_, ActivityEvidenceConfidence::High) => {
            ActivityEvidenceConfidence::High
        }
        (ActivityEvidenceConfidence::Medium, _) | (_, ActivityEvidenceConfidence::Medium) => {
            ActivityEvidenceConfidence::Medium
        }
        _ => ActivityEvidenceConfidence::Low,
    }
}

fn min_optional(left: Option<i64>, right: Option<i64>) -> Option<i64> {
    match (left, right) {
        (Some(left), Some(right)) => Some(left.min(right)),
        (Some(value), None) | (None, Some(value)) => Some(value),
        (None, None) => None,
    }
}

fn max_optional(left: Option<i64>, right: Option<i64>) -> Option<i64> {
    match (left, right) {
        (Some(left), Some(right)) => Some(left.max(right)),
        (Some(value), None) | (None, Some(value)) => Some(value),
        (None, None) => None,
    }
}

fn push_optional_string(values: &mut Vec<String>, value: Option<&str>) {
    if let Some(value) = value {
        push_string_once(values, value);
    }
}

fn push_string_once(values: &mut Vec<String>, value: &str) {
    if !value.is_empty() && !values.iter().any(|existing| existing == value) {
        values.push(value.to_string());
    }
}

fn dedupe_strings(values: &mut Vec<String>) {
    let mut seen = HashSet::new();
    values.retain(|value| seen.insert(value.clone()));
}

#[cfg(test)]
mod tests {
    use super::super::activity_recap_inputs::{
        ActivityRecapDecisionContext, ActivitySegmentFact, BranchContextFact, CandidateFact,
        CurrentSurfaceFact, ExistingQualityFacts, OpenLoopFact, SemanticMomentFact,
        SurfaceSnapshotFact, TaskActionFact, WorkstreamFact, ACTIVITY_RECAP_INPUTS_SCHEMA,
    };
    use super::*;
    use serde_json::json;

    fn empty_inputs() -> ActivityRecapInputs {
        ActivityRecapInputs {
            schema: ACTIVITY_RECAP_INPUTS_SCHEMA.to_string(),
            decision_context: ActivityRecapDecisionContext {
                decision_id_seed: Some("decision-test".to_string()),
                mode: "normal".to_string(),
                lookback_ms: 10_000,
                evidence_watermark: Some("watermark-test".to_string()),
                output_mode: Some("thin_continue".to_string()),
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
        app_family: &str,
        artifact_id: Option<&str>,
        workstream_id: Option<&str>,
        start_ms: i64,
        end_ms: i64,
        continuation_role: &str,
    ) -> ActivitySegmentFact {
        ActivitySegmentFact {
            segment_id: id.to_string(),
            artifact_id: artifact_id.map(str::to_string),
            workstream_id: workstream_id.map(str::to_string),
            episode_id: Some(format!("episode-{id}")),
            app_name: Some(app_family.to_string()),
            display_title: Some(format!("{app_family} surface")),
            app_family: app_family.to_string(),
            surface_type: "window".to_string(),
            started_at_ms: start_ms,
            ended_at_ms: end_ms,
            activity_intent: Some("editing".to_string()),
            task_phase: Some("in_progress".to_string()),
            continuation_role: Some(continuation_role.to_string()),
            work_value_score: Some(0.8),
            support_score: Some(if continuation_role == "support_context" {
                0.8
            } else {
                0.1
            }),
            divergence_score: Some(
                if matches!(continuation_role, "divergence" | "background_consumption") {
                    0.9
                } else {
                    0.1
                },
            ),
            evidence_sufficiency_score: Some(0.8),
            reason: Some("bounded local classification".to_string()),
            missing_evidence: Vec::new(),
            evidence_kinds: vec!["window_snapshot".to_string()],
            has_heavy_frame: true,
            has_direct_url: false,
            has_document_path: false,
            has_visible_text: true,
            is_event_backed_only: false,
        }
    }

    fn selected_workstream(primary_artifact_id: &str) -> WorkstreamFact {
        WorkstreamFact {
            workstream_id: "ws-primary".to_string(),
            state: "active".to_string(),
            title: Some("Primary work".to_string()),
            primary_artifact_id: Some(primary_artifact_id.to_string()),
            last_active_timestamp_ms: 100,
            confidence: 0.9,
            unresolved_signal: Some("unfinished".to_string()),
        }
    }

    fn action(
        id: &str,
        artifact_id: &str,
        action_kind: &str,
        action_role: &str,
        created_at_ms: i64,
    ) -> TaskActionFact {
        TaskActionFact {
            action_id: id.to_string(),
            frame_id: format!("frame-{id}"),
            artifact_id: Some(artifact_id.to_string()),
            secondary_artifact_id: None,
            action_kind: action_kind.to_string(),
            action_role: action_role.to_string(),
            confidence: 0.9,
            created_at_ms,
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

    fn branch(
        id: &str,
        origin_artifact_id: Option<&str>,
        branch_artifact_id: &str,
        branch_kind: &str,
        promotion_state: &str,
        returned_to_origin_at_ms: Option<i64>,
    ) -> BranchContextFact {
        BranchContextFact {
            branch_id: id.to_string(),
            branch_action_id: format!("action-{id}"),
            origin_artifact_id: origin_artifact_id.map(str::to_string),
            origin_workstream_id: origin_artifact_id.map(|_| "ws-primary".to_string()),
            branch_artifact_id: branch_artifact_id.to_string(),
            branch_kind: branch_kind.to_string(),
            branch_started_at_ms: 20,
            last_branch_seen_at_ms: 30,
            returned_to_origin_at_ms,
            promotion_state: promotion_state.to_string(),
            promotion_reason: Some("bounded branch policy".to_string()),
            confidence: 0.9,
            reason_code: Some("branch:test".to_string()),
            evidence_action_ids: vec![format!("evidence-{id}")],
            updated_at_ms: 100,
        }
    }

    fn current_surface(
        artifact_id: Option<&str>,
        app_name: &str,
        title: &str,
        observed_at_ms: i64,
    ) -> CurrentSurfaceFact {
        CurrentSurfaceFact {
            surface_id: "surface-current".to_string(),
            artifact_id: artifact_id.map(str::to_string),
            app_name: Some(app_name.to_string()),
            display_title: Some(title.to_string()),
            domain: None,
            activity_state: None,
            task_state: None,
            observed_at_ms,
            evidence_quality: "thin".to_string(),
            openability: "unknown".to_string(),
            focus_confidence: 0.6,
            identity_confidence: None,
            snapshot_id: None,
            evidence_ids: vec!["window-current".to_string()],
            missing_evidence: vec!["active_content".to_string()],
            claim_eligible: true,
        }
    }

    fn moment(id: &str, artifact_id: &str, at_ms: i64, boundary_kind: &str) -> SemanticMomentFact {
        SemanticMomentFact {
            moment_id: id.to_string(),
            started_at_ms: at_ms,
            ended_at_ms: at_ms,
            pre_frame_id: None,
            post_frame_id: None,
            pre_artifact_id: Some(artifact_id.to_string()),
            post_artifact_id: Some(artifact_id.to_string()),
            dominant_artifact_id: Some(artifact_id.to_string()),
            dominant_event_type: "content_change".to_string(),
            ordered_event_ids: vec![format!("event-{id}")],
            boundary_kind: boundary_kind.to_string(),
            current_focus_relation: None,
            semantic_summary: None,
            evidence_quality: "strong".to_string(),
        }
    }

    #[test]
    fn empty_inputs_produce_an_empty_none_confidence_timeline() {
        let timeline = stitch_activity_segments(&empty_inputs());

        assert_eq!(timeline, StitchedActivityTimeline::default());
    }

    #[test]
    fn chat_primary_finder_detour_and_explicit_chat_return_are_preserved() {
        let mut inputs = empty_inputs();
        inputs.selected_workstream = Some(selected_workstream("chat"));
        inputs.recent_segments = vec![
            segment(
                "chat-return",
                "browser",
                Some("chat"),
                Some("ws-primary"),
                32,
                40,
                "resume_target",
            ),
            segment(
                "finder",
                "finder",
                Some("photos"),
                None,
                20,
                30,
                "current_focus_only",
            ),
            segment(
                "chat-primary",
                "browser",
                Some("chat"),
                Some("ws-primary"),
                0,
                15,
                "resume_target",
            ),
        ];
        inputs.recent_actions = vec![
            action("compose", "chat", "composing", "primary", 5),
            action("return", "chat", "returning_to_origin", "primary", 35),
        ];
        inputs.branch_contexts = vec![branch(
            "finder-detour",
            Some("chat"),
            "photos",
            "unrelated_browsing",
            "unpromoted",
            Some(32),
        )];
        inputs.current_surface = Some(current_surface(
            Some("chat"),
            "browser",
            "browser surface",
            40,
        ));

        let timeline = stitch_activity_segments(&inputs);

        assert_eq!(timeline.ordered_segments.len(), 3);
        assert_eq!(
            timeline.ordered_segments[0].role,
            ActivitySegmentRole::Primary
        );
        assert_eq!(
            timeline.ordered_segments[1].role,
            ActivitySegmentRole::Detour
        );
        assert_eq!(
            timeline.ordered_segments[2].role,
            ActivitySegmentRole::Return
        );
        assert_eq!(
            timeline.primary_segment.as_ref().unwrap().segment_id,
            "chat-primary"
        );
        assert_eq!(
            timeline.current_segment.as_ref().unwrap().segment_id,
            "chat-return"
        );
        assert_eq!(timeline.recent_detours.len(), 1);
        assert!(timeline.returned_to_primary);
        assert_eq!(
            timeline.primary_continuity,
            ActivityPrimaryContinuity::InterruptedThenReturned
        );
        assert!(timeline.ordered_segments[2]
            .evidence_anchor_ids
            .contains(&"return".to_string()));
    }

    #[test]
    fn editor_primary_and_unpromoted_docs_branch_remain_separate() {
        let mut inputs = empty_inputs();
        inputs.selected_workstream = Some(selected_workstream("editor"));
        inputs.recent_segments = vec![
            segment(
                "docs",
                "browser",
                Some("docs"),
                Some("ws-primary"),
                20,
                30,
                "support_context",
            ),
            segment(
                "editor",
                "code_editor",
                Some("editor"),
                Some("ws-primary"),
                0,
                15,
                "resume_target",
            ),
        ];
        inputs.recent_actions = vec![action("edit", "editor", "editing", "primary", 5)];
        inputs.branch_contexts = vec![branch(
            "docs-branch",
            Some("editor"),
            "docs",
            "documentation_reference",
            "unpromoted",
            None,
        )];

        let timeline = stitch_activity_segments(&inputs);

        assert_eq!(
            timeline
                .primary_segment
                .as_ref()
                .unwrap()
                .artifact_id
                .as_deref(),
            Some("editor")
        );
        assert_eq!(timeline.support_segments.len(), 1);
        assert_eq!(
            timeline.support_segments[0].artifact_id.as_deref(),
            Some("docs")
        );
        assert_eq!(
            timeline.support_segments[0].promotion_state,
            ActivitySegmentPromotionState::NotPromoted
        );
        assert_eq!(
            timeline.primary_continuity,
            ActivityPrimaryContinuity::BranchedWithoutReturn
        );

        inputs.branch_contexts[0].origin_artifact_id = None;
        inputs.branch_contexts[0].origin_workstream_id = Some("ws-primary".to_string());
        inputs.branch_contexts[0].reason_code = Some("branch:no_origin".to_string());
        let no_origin = stitch_activity_segments(&inputs);
        assert_eq!(
            no_origin.ordered_segments[1].role,
            ActivitySegmentRole::Detour
        );
    }

    #[test]
    fn terminal_branch_requires_explicit_promotion_even_when_classifier_says_resume_target() {
        let mut inputs = empty_inputs();
        inputs.selected_workstream = Some(selected_workstream("editor"));
        inputs.recent_segments = vec![segment(
            "terminal",
            "terminal",
            Some("terminal"),
            Some("ws-primary"),
            20,
            100,
            "resume_target",
        )];
        inputs.recent_actions = vec![
            action("command", "terminal", "running_command", "primary", 30),
            action("error", "terminal", "encountering_error", "blocker", 90),
        ];
        inputs.branch_contexts = vec![branch(
            "terminal-branch",
            Some("editor"),
            "terminal",
            "terminal_support_output",
            "unpromoted",
            None,
        )];

        let unpromoted = stitch_activity_segments(&inputs);
        assert_eq!(
            unpromoted.ordered_segments[0].role,
            ActivitySegmentRole::Support
        );
        assert!(unpromoted.primary_segment.is_none());

        inputs.branch_contexts[0].promotion_state = "promoted_sustained_work".to_string();
        let promoted = stitch_activity_segments(&inputs);
        assert_eq!(
            promoted.ordered_segments[0].role,
            ActivitySegmentRole::PromotedPrimary
        );
        assert_eq!(
            promoted.ordered_segments[0].promotion_state,
            ActivitySegmentPromotionState::Promoted
        );
        assert_eq!(
            promoted.primary_continuity,
            ActivityPrimaryContinuity::NewPrimary
        );
    }

    #[test]
    fn gmail_compose_is_an_interrupt_without_branch_promotion() {
        let mut inputs = empty_inputs();
        inputs.selected_workstream = Some(selected_workstream("editor"));
        inputs.recent_segments = vec![segment(
            "gmail",
            "messaging",
            Some("gmail"),
            Some("ws-primary"),
            20,
            30,
            "resume_target_if_unfinished",
        )];
        inputs.recent_actions = vec![action("reply", "gmail", "composing", "primary", 25)];
        inputs.branch_contexts = vec![branch(
            "gmail-branch",
            Some("editor"),
            "gmail",
            "message_interrupt",
            "unpromoted",
            None,
        )];

        let timeline = stitch_activity_segments(&inputs);

        assert_eq!(timeline.interruptions.len(), 1);
        assert_eq!(
            timeline.interruptions[0].role,
            ActivitySegmentRole::Interrupt
        );
        assert!(timeline.primary_segment.is_none());
    }

    #[test]
    fn latest_current_focus_only_surface_does_not_replace_primary_work() {
        let mut inputs = empty_inputs();
        inputs.selected_workstream = Some(selected_workstream("editor"));
        inputs.recent_segments = vec![segment(
            "editor",
            "code_editor",
            Some("editor"),
            Some("ws-primary"),
            0,
            10,
            "resume_target",
        )];
        inputs.recent_actions = vec![action("edit", "editor", "editing", "primary", 5)];
        inputs.current_surface = Some(current_surface(None, "Unknown App", "Untitled", 20));

        let timeline = stitch_activity_segments(&inputs);

        assert_eq!(
            timeline
                .primary_segment
                .as_ref()
                .unwrap()
                .artifact_id
                .as_deref(),
            Some("editor")
        );
        assert_eq!(
            timeline.current_segment.as_ref().unwrap().role,
            ActivitySegmentRole::CurrentFocusOnly
        );
        assert_ne!(
            timeline.primary_segment.as_ref().unwrap().segment_id,
            timeline.current_segment.as_ref().unwrap().segment_id
        );
        assert_eq!(timeline.confidence, ActivityConfidence::Low);
    }

    #[test]
    fn unknown_event_only_surface_stays_low_confidence_current_focus_only() {
        let mut inputs = empty_inputs();
        let mut weak = segment("weak", "unknown", None, None, 0, 10, "needs_fresh_capture");
        weak.has_heavy_frame = false;
        weak.has_visible_text = false;
        weak.is_event_backed_only = true;
        weak.evidence_sufficiency_score = Some(0.3);
        weak.missing_evidence = vec!["active_content".to_string()];
        inputs.recent_segments = vec![weak];
        inputs.current_surface = Some(current_surface(None, "unknown", "unknown surface", 10));

        let timeline = stitch_activity_segments(&inputs);

        assert!(timeline.primary_segment.is_none());
        assert_eq!(
            timeline.current_segment.as_ref().unwrap().role,
            ActivitySegmentRole::CurrentFocusOnly
        );
        assert_eq!(
            timeline.current_segment.as_ref().unwrap().confidence,
            ActivityEvidenceConfidence::Low
        );
        assert_eq!(timeline.confidence, ActivityConfidence::Low);
    }

    #[test]
    fn grounded_p3_current_work_can_be_primary_without_inventing_a_target() {
        let mut inputs = empty_inputs();
        inputs.selected_workstream = Some(selected_workstream("codex"));
        let mut current = current_surface(Some("codex"), "Codex", "Implementation task", 50);
        current.activity_state = Some("composing_prompt".to_string());
        current.evidence_quality = "strong".to_string();
        current.evidence_ids = vec!["window-current".to_string(), "snapshot-current".to_string()];
        current.missing_evidence.clear();
        inputs.current_surface = Some(current);

        let timeline = stitch_activity_segments(&inputs);

        assert_eq!(timeline.ordered_segments.len(), 1);
        assert_eq!(
            timeline.ordered_segments[0].role,
            ActivitySegmentRole::Primary
        );
        assert_eq!(timeline.confidence, ActivityConfidence::High);
        assert!(
            timeline.ordered_segments[0].surface_title.as_deref() == Some("Implementation task")
        );
    }

    #[test]
    fn merge_requires_same_role_and_no_meaningful_semantic_boundary() {
        let mut inputs = empty_inputs();
        inputs.selected_workstream = Some(selected_workstream("editor"));
        inputs.recent_segments = vec![
            segment(
                "editor-two",
                "code_editor",
                Some("editor"),
                Some("ws-primary"),
                11,
                20,
                "resume_target",
            ),
            segment(
                "editor-one",
                "code_editor",
                Some("editor"),
                Some("ws-primary"),
                0,
                10,
                "resume_target",
            ),
        ];
        inputs.recent_moments = vec![moment("unknown", "editor", 10, "unknown")];

        let merged = stitch_activity_segments(&inputs);
        assert_eq!(merged.ordered_segments.len(), 1);
        assert_eq!(merged.ordered_segments[0].start_ms, Some(0));
        assert_eq!(merged.ordered_segments[0].end_ms, Some(20));
        assert!(merged.ordered_segments[0]
            .evidence_anchor_ids
            .contains(&"editor-two".to_string()));

        inputs.recent_moments = vec![moment(
            "error-boundary",
            "editor",
            10,
            "error_without_resolution",
        )];
        let separated = stitch_activity_segments(&inputs);
        assert_eq!(separated.ordered_segments.len(), 2);
    }

    #[test]
    fn output_cap_preserves_primary_and_factual_current_segment() {
        let mut inputs = empty_inputs();
        inputs.selected_workstream = Some(selected_workstream("primary"));
        inputs.recent_segments.push(segment(
            "primary",
            "code_editor",
            Some("primary"),
            Some("ws-primary"),
            0,
            5,
            "resume_target",
        ));
        inputs.recent_actions = vec![action("edit", "primary", "editing", "primary", 2)];
        for index in 0..7 {
            inputs.recent_segments.push(segment(
                &format!("detour-{index}"),
                "browser",
                Some(&format!("detour-artifact-{index}")),
                None,
                10 + index * 10,
                15 + index * 10,
                "divergence",
            ));
        }
        inputs.current_surface = Some(current_surface(
            Some("detour-artifact-6"),
            "browser",
            "browser surface",
            75,
        ));

        let first = stitch_activity_segments(&inputs);
        let second = stitch_activity_segments(&inputs);

        assert_eq!(first, second);
        assert_eq!(first.ordered_segments.len(), MAX_STITCHED_SEGMENTS);
        assert!(first
            .ordered_segments
            .iter()
            .any(|segment| segment.segment_id == "primary"));
        assert_eq!(
            first
                .current_segment
                .as_ref()
                .unwrap()
                .artifact_id
                .as_deref(),
            Some("detour-artifact-6")
        );
        assert!(first
            .warnings
            .contains(&"activity_segments:timeline_capped".to_string()));
    }

    #[test]
    fn url_or_event_only_selection_cannot_create_high_activity_confidence() {
        let mut inputs = empty_inputs();
        inputs.selected_workstream = Some(selected_workstream("tab"));
        let mut selected = segment(
            "tab",
            "browser",
            Some("tab"),
            Some("ws-primary"),
            0,
            10,
            "resume_target",
        );
        selected.has_direct_url = true;
        selected.has_heavy_frame = false;
        selected.has_visible_text = false;
        selected.is_event_backed_only = true;
        selected.evidence_sufficiency_score = Some(0.2);
        selected.missing_evidence = vec!["active_content".to_string()];
        inputs.recent_segments = vec![selected];

        let timeline = stitch_activity_segments(&inputs);

        assert_eq!(
            timeline.primary_segment.as_ref().unwrap().confidence,
            ActivityEvidenceConfidence::Low
        );
        assert_eq!(timeline.confidence, ActivityConfidence::Low);
    }

    #[test]
    fn promotion_mapping_and_serialized_shape_are_stable() {
        let mut inputs = empty_inputs();
        inputs.recent_segments = vec![segment(
            "branch",
            "terminal",
            Some("branch"),
            None,
            0,
            10,
            "resume_target",
        )];
        inputs.branch_contexts = vec![branch(
            "branch-context",
            Some("origin"),
            "branch",
            "terminal_support_output",
            "blocked_feedback_suppressed",
            None,
        )];

        let blocked = stitch_activity_segments(&inputs);
        assert_eq!(
            blocked.ordered_segments[0].promotion_state,
            ActivitySegmentPromotionState::Blocked
        );
        assert_eq!(
            blocked.ordered_segments[0].role,
            ActivitySegmentRole::Support
        );

        inputs.branch_contexts[0].promotion_state = "promoted_preview_only".to_string();
        let unclear = stitch_activity_segments(&inputs);
        assert_eq!(
            unclear.ordered_segments[0].promotion_state,
            ActivitySegmentPromotionState::Unclear
        );
        assert!(unclear
            .warnings
            .contains(&"activity_segments:unknown_promotion_state".to_string()));

        let value = serde_json::to_value(&unclear).unwrap();
        assert_eq!(value["primary_continuity"], json!("unclear"));
        assert_eq!(
            value["ordered_segments"][0]["promotion_state"],
            json!("unclear")
        );
        assert!(value.get("primary_segment").is_some());
        assert!(value.get("current_segment").is_some());
    }

    #[test]
    fn selected_candidate_segment_is_a_primary_signal_but_not_a_branch_override() {
        let mut inputs = empty_inputs();
        inputs.recent_segments = vec![segment(
            "candidate-segment",
            "ai_coding_agent",
            Some("candidate-artifact"),
            Some("ws-candidate"),
            0,
            10,
            "resume_target",
        )];
        inputs.selected_candidate = Some(CandidateFact {
            candidate_id: "candidate".to_string(),
            workstream_id: "ws-candidate".to_string(),
            candidate_kind: "artifact".to_string(),
            target_artifact_id: Some("candidate-artifact".to_string()),
            last_meaningful_action_id: None,
            open_loop_id: None,
            activity_segment_id: Some("candidate-segment".to_string()),
            activity_intent: Some("implementing".to_string()),
            task_phase: Some("in_progress".to_string()),
            continuation_role: Some("resume_target".to_string()),
            score: 0.8,
            evidence_sufficiency_score: 0.8,
            missing_evidence: Vec::new(),
            branch_promotion_state: None,
            branch_public_return_eligible: Some(true),
        });

        let selected = stitch_activity_segments(&inputs);
        assert_eq!(
            selected.ordered_segments[0].role,
            ActivitySegmentRole::Primary
        );

        let candidate = inputs.selected_candidate.as_mut().unwrap();
        candidate.branch_promotion_state = Some("unpromoted".to_string());
        candidate.branch_public_return_eligible = Some(false);
        let candidate_guarded = stitch_activity_segments(&inputs);
        assert_eq!(
            candidate_guarded.ordered_segments[0].role,
            ActivitySegmentRole::Support
        );

        inputs.branch_contexts = vec![branch(
            "unpromoted",
            Some("origin"),
            "candidate-artifact",
            "tool_or_agent_output",
            "unpromoted",
            None,
        )];
        let guarded = stitch_activity_segments(&inputs);
        assert_eq!(
            guarded.ordered_segments[0].role,
            ActivitySegmentRole::Support
        );
    }

    #[test]
    fn open_loop_origin_can_identify_primary_but_blocker_alone_cannot() {
        let mut inputs = empty_inputs();
        inputs.recent_segments = vec![segment(
            "origin",
            "code_editor",
            Some("origin"),
            Some("ws-primary"),
            0,
            10,
            "resume_target",
        )];
        inputs.open_loops = vec![OpenLoopFact {
            open_loop_id: "loop".to_string(),
            workstream_id: "ws-primary".to_string(),
            state: "open".to_string(),
            boundary_kind: "unfinished_progress".to_string(),
            quality: "strong".to_string(),
            confidence: 0.9,
            origin_artifact_id: Some("origin".to_string()),
            current_focus_artifact_id: None,
            primary_return_artifact_id: Some("origin".to_string()),
            resume_work_artifact_id: Some("origin".to_string()),
            blocker_artifact_id: None,
            verification_artifact_id: None,
            objective_hint: None,
            last_concrete_progress: None,
            unfinished_state: None,
            next_evidence_backed_action: None,
            current_focus_relation: None,
            missing_evidence: Vec::new(),
            evidence_spans: Vec::new(),
            last_updated_at_ms: 10,
        }];

        let origin = stitch_activity_segments(&inputs);
        assert_eq!(
            origin.ordered_segments[0].role,
            ActivitySegmentRole::Primary
        );

        inputs.recent_segments[0].artifact_id = Some("blocker".to_string());
        inputs.open_loops[0].origin_artifact_id = None;
        inputs.open_loops[0].primary_return_artifact_id = None;
        inputs.open_loops[0].resume_work_artifact_id = None;
        inputs.open_loops[0].blocker_artifact_id = Some("blocker".to_string());
        let blocker_only = stitch_activity_segments(&inputs);
        assert!(blocker_only.primary_segment.is_none());
    }

    #[test]
    fn snapshot_signal_adds_grounding_without_exposing_snapshot_payloads() {
        let mut inputs = empty_inputs();
        inputs.selected_workstream = Some(selected_workstream("editor"));
        inputs.recent_segments = vec![segment(
            "editor",
            "code_editor",
            Some("editor"),
            Some("ws-primary"),
            0,
            10,
            "resume_target",
        )];
        inputs.surface_snapshots = vec![SurfaceSnapshotFact {
            snapshot_id: "snapshot-editor".to_string(),
            artifact_id: Some("editor".to_string()),
            frame_id: None,
            domain: "code_editor".to_string(),
            app_name: Some("Code".to_string()),
            display_title: Some("Project".to_string()),
            relative_file_name: Some("continuation.rs".to_string()),
            git_branch: Some("feature".to_string()),
            activity_state: Some("actively_editing".to_string()),
            task_state: Some("in_progress".to_string()),
            command_state: None,
            has_error_markers: false,
            identity_confidence: "high".to_string(),
            evidence_quality: "strong".to_string(),
            openability: "inspectable".to_string(),
            missing_evidence: Vec::new(),
            evidence_sources: vec!["accessibility".to_string()],
            observed_at_ms: 5,
        }];

        let timeline = stitch_activity_segments(&inputs);

        assert!(timeline.ordered_segments[0]
            .activity_kinds
            .contains(&"actively_editing".to_string()));
        assert!(timeline.ordered_segments[0]
            .evidence_anchor_ids
            .contains(&"snapshot-editor".to_string()));
        let serialized = serde_json::to_string(&timeline).unwrap();
        assert!(!serialized.contains("git_branch"));
        assert!(!serialized.contains("relative_file_name"));
    }
}
