use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

use super::model::{is_control_role, ModelTaskHypothesisV1, ResolutionStatusV1};
use super::observation_packet::{
    AuthorshipStatusV2, EvidenceHandleV2, EvidencePartitionV2, ObservationPacketV2,
};
use super::task_snapshot::{
    ClaimEvidenceV2, ReturnAnchorStatusV2, SnapshotHypothesisV2, SnapshotSelectionStatusV2,
    TaskSnapshotV2, TASK_SNAPSHOT_SCHEMA_V2,
};

pub(crate) const TASK_TRUTH_VERIFIER_VERSION: &str = "task_truth_v2.evidence_verifier.v1";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum FieldVerdictV1 {
    Accepted,
    Downgraded,
    Removed,
    Ambiguous,
    Unsupported,
    Contradicted,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct FieldVerificationV1 {
    pub(crate) field: String,
    pub(crate) verdict: FieldVerdictV1,
    pub(crate) confidence_before: f64,
    pub(crate) confidence_after: f64,
    pub(crate) reasons: Vec<String>,
    pub(crate) accepted_evidence_refs: Vec<EvidenceHandleV2>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct VerificationResultV1 {
    pub(crate) status: ResolutionStatusV1,
    pub(crate) snapshot: Option<TaskSnapshotV2>,
    pub(crate) fields: Vec<FieldVerificationV1>,
    pub(crate) second_pass_reasons: Vec<String>,
    pub(crate) unsupported_claim_count: usize,
}

pub(crate) trait TaskTruthVerifier {
    fn verify(
        &self,
        packet: &ObservationPacketV2,
        prior: Option<&TaskSnapshotV2>,
        hypotheses: &[ModelTaskHypothesisV1],
        model_status: ResolutionStatusV1,
    ) -> VerificationResultV1;
}

pub(crate) struct LocalEvidenceVerifier;

const MATERIAL_FIELDS: [&str; 14] = [
    "task_summary",
    "task_kind",
    "task_object",
    "user_goal",
    "app_identity",
    "surface_identity_hash",
    "document_or_thread_identity_hash",
    "execution_state",
    "current_actor",
    "waiting_on",
    "last_meaningful_progress",
    "unfinished_step",
    "next_action",
    "relation_to_prior",
];

impl TaskTruthVerifier for LocalEvidenceVerifier {
    fn verify(
        &self,
        packet: &ObservationPacketV2,
        prior: Option<&TaskSnapshotV2>,
        hypotheses: &[ModelTaskHypothesisV1],
        model_status: ResolutionStatusV1,
    ) -> VerificationResultV1 {
        if matches!(
            model_status,
            ResolutionStatusV1::PrivacyBlocked | ResolutionStatusV1::ModelUnavailable
        ) || hypotheses.is_empty()
        {
            return VerificationResultV1 {
                status: model_status,
                snapshot: None,
                fields: Vec::new(),
                second_pass_reasons: Vec::new(),
                unsupported_claim_count: 0,
            };
        }
        let mut ranked = hypotheses.iter().collect::<Vec<_>>();
        ranked.sort_by(|left, right| {
            right
                .confidence
                .partial_cmp(&left.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| left.hypothesis_id.cmp(&right.hypothesis_id))
        });
        let selected = ranked[0];
        let mut second_pass_reasons = Vec::new();
        if ranked
            .get(1)
            .is_some_and(|next| (selected.confidence - next.confidence).abs() <= 0.10)
        {
            second_pass_reasons.push("top_hypotheses_close".into());
        }
        if packet
            .canonical_elements
            .iter()
            .any(|element| !element.source_conflicts.is_empty())
        {
            second_pass_reasons.push("ax_ocr_visual_conflict".into());
        }
        let mut fields = Vec::new();
        for field in MATERIAL_FIELDS {
            let value = field_value(selected, field);
            if value.is_none() {
                continue;
            }
            let before = selected
                .confidence_by_field
                .get(field)
                .copied()
                .unwrap_or(selected.confidence)
                .clamp(0.0, 1.0);
            let claims = selected
                .claim_evidence
                .iter()
                .filter(|claim| claim.field == field)
                .collect::<Vec<_>>();
            let mut reasons: Vec<String> = Vec::new();
            let mut refs = claims
                .iter()
                .flat_map(|claim| claim.evidence_refs.iter().cloned())
                .collect::<Vec<_>>();
            refs.sort_by(|left, right| {
                left.source_kind
                    .cmp(&right.source_kind)
                    .then_with(|| left.record_id.cmp(&right.record_id))
            });
            refs.dedup_by(|left, right| {
                left.source_kind == right.source_kind && left.record_id == right.record_id
            });
            let invalid = refs
                .iter()
                .filter(|reference| !reference_exists(packet, prior, reference))
                .count();
            if claims.is_empty() || refs.is_empty() {
                reasons.push("material_claim_without_evidence".into());
            }
            if invalid > 0 {
                reasons.push("referenced_id_not_in_request".into());
            }
            refs.retain(|reference| reference_exists(packet, prior, reference));
            if refs
                .iter()
                .any(|reference| reference_is_private(packet, reference))
            {
                reasons.push("privacy_blocked_evidence".into());
                refs.retain(|reference| !reference_is_private(packet, reference));
            }
            if is_task_identity_field(field)
                && !refs.is_empty()
                && refs
                    .iter()
                    .all(|reference| reference_is_control(packet, reference))
            {
                reasons.push("control_navigation_cannot_be_authored_goal".into());
                second_pass_reasons.push("claim_references_control_region".into());
            }
            if is_task_identity_field(field)
                && refs
                    .iter()
                    .any(|reference| reference_is_historical_without_continuity(packet, reference))
            {
                reasons.push("historical_evidence_without_temporal_continuity".into());
            }
            if matches!(field, "task_summary" | "task_object" | "user_goal")
                && !refs.is_empty()
                && !refs
                    .iter()
                    .any(|reference| reference_supports_user_authorship(packet, reference))
            {
                reasons.push("user_authorship_not_causally_supported".into());
            }
            if field == "app_identity"
                && value != packet.active_surface.app_name.as_deref()
                && value != packet.active_surface.app_bundle_id.as_deref()
            {
                reasons.push("invented_app_identity".into());
            }
            if field == "document_or_thread_identity_hash"
                && value != packet.active_surface.document_path_hash.as_deref()
                && value != packet.active_surface.browser_url_hash.as_deref()
            {
                reasons.push("invented_document_or_thread_identity".into());
            }
            if field == "surface_identity_hash"
                && !packet
                    .active_surface
                    .window_title_hash
                    .as_deref()
                    .is_some_and(|identity| Some(identity) == value)
            {
                reasons.push("invented_surface_identity".into());
            }
            if field == "next_action" {
                if selected
                    .unfinished_step
                    .as_deref()
                    .is_none_or(str::is_empty)
                {
                    reasons.push("next_action_without_unfinished_state".into());
                }
                if value.is_some_and(generic_next_action) {
                    reasons.push("generic_invented_next_action".into());
                }
            }
            if selected
                .contradictions
                .iter()
                .any(|item| item.field == field)
            {
                reasons.push("model_reported_field_contradiction".into());
            }
            if matches!(field, "execution_state" | "current_actor" | "waiting_on")
                && !lifecycle_value_allowed(field, value.unwrap_or_default())
            {
                reasons.push("lifecycle_value_not_supported_by_schema".into());
            }
            if field == "unfinished_step"
                && selected.execution_state.as_deref() == Some("completed")
            {
                reasons.push("unfinished_step_conflicts_with_completed_state".into());
            }
            let hard = reasons.iter().any(|reason| {
                matches!(
                    reason.as_str(),
                    "material_claim_without_evidence"
                        | "referenced_id_not_in_request"
                        | "privacy_blocked_evidence"
                        | "control_navigation_cannot_be_authored_goal"
                        | "invented_app_identity"
                        | "invented_document_or_thread_identity"
                        | "invented_surface_identity"
                        | "generic_invented_next_action"
                        | "next_action_without_unfinished_state"
                        | "lifecycle_value_not_supported_by_schema"
                        | "unfinished_step_conflicts_with_completed_state"
                )
            });
            let contradicted = reasons
                .iter()
                .any(|reason| reason == "model_reported_field_contradiction");
            let verdict = if hard {
                FieldVerdictV1::Removed
            } else if contradicted {
                FieldVerdictV1::Contradicted
            } else if !reasons.is_empty() {
                FieldVerdictV1::Downgraded
            } else if model_status == ResolutionStatusV1::Ambiguous {
                FieldVerdictV1::Ambiguous
            } else {
                FieldVerdictV1::Accepted
            };
            let after = match verdict {
                FieldVerdictV1::Accepted => before,
                FieldVerdictV1::Ambiguous => before.min(0.59),
                FieldVerdictV1::Downgraded => (before - 0.20).max(0.0),
                FieldVerdictV1::Contradicted => (before - 0.35).max(0.0),
                _ => 0.0,
            };
            if (0.45..=0.65).contains(&before)
                && matches!(field, "task_summary" | "user_goal" | "execution_state")
            {
                second_pass_reasons.push("critical_field_near_threshold".into());
            }
            fields.push(FieldVerificationV1 {
                field: field.into(),
                verdict,
                confidence_before: before,
                confidence_after: after,
                reasons,
                accepted_evidence_refs: refs,
            });
        }
        if let Some(prior) = prior {
            if packet.observed_at_ms < prior.observed_at_ms {
                second_pass_reasons.push("temporal_ordering_contradictory".into());
            }
        }
        if selected.return_anchor_record_id.is_some()
            && selected
                .return_anchor_record_id
                .as_ref()
                .is_none_or(|anchor| {
                    !packet
                        .return_anchor_facts
                        .iter()
                        .any(|item| &item.record_id == anchor)
                })
        {
            second_pass_reasons.push("task_return_anchor_disagree".into());
        }
        second_pass_reasons.sort();
        second_pass_reasons.dedup();
        let unsupported_claim_count = fields
            .iter()
            .filter(|field| {
                matches!(
                    field.verdict,
                    FieldVerdictV1::Removed | FieldVerdictV1::Unsupported
                )
            })
            .count();
        let snapshot = build_verified_snapshot(packet, prior, selected, hypotheses, &fields);
        let status = if snapshot
            .as_ref()
            .and_then(|snapshot| snapshot.task_summary.as_ref())
            .is_none()
        {
            ResolutionStatusV1::InsufficientEvidence
        } else if model_status == ResolutionStatusV1::Ambiguous || hypotheses.len() > 1 {
            ResolutionStatusV1::Ambiguous
        } else {
            ResolutionStatusV1::Resolved
        };
        VerificationResultV1 {
            status,
            snapshot,
            fields,
            second_pass_reasons,
            unsupported_claim_count,
        }
    }
}

fn field_value<'a>(hypothesis: &'a ModelTaskHypothesisV1, field: &str) -> Option<&'a str> {
    match field {
        "task_summary" => hypothesis.task_summary.as_deref(),
        "task_kind" => hypothesis.task_kind.as_deref(),
        "task_object" => hypothesis.task_object.as_deref(),
        "user_goal" => hypothesis.user_goal.as_deref(),
        "app_identity" => hypothesis.app_identity.as_deref(),
        "surface_identity_hash" => hypothesis.surface_identity_hash.as_deref(),
        "document_or_thread_identity_hash" => {
            hypothesis.document_or_thread_identity_hash.as_deref()
        }
        "execution_state" => hypothesis.execution_state.as_deref(),
        "current_actor" => hypothesis.current_actor.as_deref(),
        "waiting_on" => hypothesis.waiting_on.as_deref(),
        "last_meaningful_progress" => hypothesis.last_meaningful_progress.as_deref(),
        "unfinished_step" => hypothesis.unfinished_step.as_deref(),
        "next_action" => hypothesis.next_action.as_deref(),
        "relation_to_prior" => hypothesis.relation_to_prior.as_deref(),
        _ => None,
    }
}

fn accepted(fields: &[FieldVerificationV1], field: &str) -> bool {
    fields.iter().any(|item| {
        item.field == field
            && matches!(
                item.verdict,
                FieldVerdictV1::Accepted | FieldVerdictV1::Downgraded | FieldVerdictV1::Ambiguous
            )
    })
}

fn build_verified_snapshot(
    packet: &ObservationPacketV2,
    prior: Option<&TaskSnapshotV2>,
    selected: &ModelTaskHypothesisV1,
    hypotheses: &[ModelTaskHypothesisV1],
    fields: &[FieldVerificationV1],
) -> Option<TaskSnapshotV2> {
    let task_summary = accepted(fields, "task_summary")
        .then(|| selected.task_summary.clone())
        .flatten();
    let user_goal = accepted(fields, "user_goal")
        .then(|| selected.user_goal.clone())
        .flatten();
    if task_summary.is_none() && user_goal.is_none() {
        return None;
    }
    let revision = prior.map(|item| item.revision + 1).unwrap_or(1);
    let seed = format!(
        "{}:{}:{}",
        packet.packet_id, selected.hypothesis_id, revision
    );
    let confidence_by_field = fields
        .iter()
        .filter(|field| field.confidence_after > 0.0)
        .map(|field| (field.field.clone(), field.confidence_after))
        .collect::<BTreeMap<_, _>>();
    let claim_evidence = fields
        .iter()
        .filter(|field| field.confidence_after > 0.0 && !field.accepted_evidence_refs.is_empty())
        .map(|field| ClaimEvidenceV2 {
            claim: field.field.clone(),
            evidence_refs: field.accepted_evidence_refs.clone(),
            confidence: field.confidence_after,
            source_confidence: BTreeMap::from([(
                "multimodal_model".into(),
                field.confidence_before,
            )]),
        })
        .collect();
    let mut alternatives = hypotheses
        .iter()
        .filter(|item| item.hypothesis_id != selected.hypothesis_id)
        .take(1)
        .map(|item| SnapshotHypothesisV2 {
            summary: item
                .task_summary
                .clone()
                .unwrap_or_else(|| "Alternative interpretation".into()),
            relation: item
                .relation_to_prior
                .clone()
                .unwrap_or_else(|| "unknown".into()),
            confidence: item.confidence,
            evidence_refs: item
                .claim_evidence
                .iter()
                .flat_map(|claim| claim.evidence_refs.iter().cloned())
                .collect(),
        })
        .collect::<Vec<_>>();
    alternatives.truncate(1);
    let contradictions = selected
        .contradictions
        .iter()
        .map(|item| format!("{}:{}", item.field, item.reason))
        .collect::<Vec<_>>();
    let anchor_valid = selected
        .return_anchor_record_id
        .as_ref()
        .is_some_and(|anchor| {
            packet
                .return_anchor_facts
                .iter()
                .any(|item| &item.record_id == anchor)
        });
    Some(TaskSnapshotV2 {
        schema: TASK_SNAPSHOT_SCHEMA_V2.into(),
        snapshot_id: format!("snapshot-{}", super::super::stable_hash(seed.as_bytes())),
        revision,
        prior_snapshot_id: prior.map(|item| item.snapshot_id.clone()),
        supersedes_snapshot_id: prior.map(|item| item.snapshot_id.clone()),
        observed_at_ms: packet.observed_at_ms,
        evidence_watermark: packet.evidence_watermark.clone(),
        packet_id: packet.packet_id.clone(),
        legacy_task_turn_id: None,
        task_basis: "explicit_goal".into(),
        task_summary,
        task_kind: if accepted(fields, "task_kind") {
            selected
                .task_kind
                .clone()
                .unwrap_or_else(|| "unknown".into())
        } else {
            "unknown".into()
        },
        task_object: accepted(fields, "task_object")
            .then(|| selected.task_object.clone())
            .flatten(),
        user_goal,
        app_identity: accepted(fields, "app_identity")
            .then(|| selected.app_identity.clone())
            .flatten(),
        surface_identity_hash: accepted(fields, "surface_identity_hash")
            .then(|| selected.surface_identity_hash.clone())
            .flatten(),
        document_or_thread_identity_hash: accepted(fields, "document_or_thread_identity_hash")
            .then(|| selected.document_or_thread_identity_hash.clone())
            .flatten(),
        execution_state: if accepted(fields, "execution_state") {
            selected
                .execution_state
                .clone()
                .unwrap_or_else(|| "unclear".into())
        } else {
            "unclear".into()
        },
        current_actor: if accepted(fields, "current_actor") {
            selected
                .current_actor
                .clone()
                .unwrap_or_else(|| "unknown".into())
        } else {
            "unknown".into()
        },
        waiting_on: if accepted(fields, "waiting_on") {
            selected
                .waiting_on
                .clone()
                .unwrap_or_else(|| "unknown".into())
        } else {
            "unknown".into()
        },
        last_meaningful_progress: accepted(fields, "last_meaningful_progress")
            .then(|| selected.last_meaningful_progress.clone())
            .flatten(),
        unfinished_step: accepted(fields, "unfinished_step")
            .then(|| selected.unfinished_step.clone())
            .flatten(),
        next_action: accepted(fields, "next_action")
            .then(|| selected.next_action.clone())
            .flatten(),
        relation_to_prior: if accepted(fields, "relation_to_prior") {
            selected
                .relation_to_prior
                .clone()
                .unwrap_or_else(|| "unknown".into())
        } else {
            "unknown".into()
        },
        selection_status: if hypotheses.len() > 1 {
            SnapshotSelectionStatusV2::Alternative
        } else {
            SnapshotSelectionStatusV2::Selected
        },
        claim_evidence,
        alternative_hypotheses: alternatives,
        contradictions,
        confidence_by_field,
        confidence_by_source: BTreeMap::from([(
            "multimodal_model_verified".into(),
            selected.confidence,
        )]),
        return_anchor_candidate_id: anchor_valid
            .then(|| selected.return_anchor_record_id.clone())
            .flatten(),
        return_anchor_status: if anchor_valid {
            ReturnAnchorStatusV2::CandidateOnly
        } else {
            ReturnAnchorStatusV2::Unresolved
        },
        resolver_version: super::model::TASK_TRUTH_RESOLVER_VERSION.into(),
        provenance: vec![
            "multimodal_model_hypothesis".into(),
            "local_evidence_verification".into(),
            "target_features_excluded_from_task_confidence".into(),
        ],
        continuity_confidence_decay: 0.0,
    })
}

fn reference_exists(
    packet: &ObservationPacketV2,
    prior: Option<&TaskSnapshotV2>,
    reference: &EvidenceHandleV2,
) -> bool {
    packet
        .canonical_elements
        .iter()
        .any(|item| item.element_id == reference.record_id)
        || packet
            .causal_events
            .iter()
            .any(|item| item.event_id == reference.record_id)
        || packet
            .semantic_keyframes
            .iter()
            .any(|item| item.frame_id == reference.record_id)
        || packet
            .transition_ids
            .iter()
            .any(|id| id == &reference.record_id)
        || packet
            .capture_trigger_ids
            .iter()
            .any(|id| id == &reference.record_id)
        || packet
            .return_anchor_facts
            .iter()
            .any(|item| item.record_id == reference.record_id)
        || prior.is_some_and(|item| item.snapshot_id == reference.record_id)
}

fn reference_is_private(packet: &ObservationPacketV2, reference: &EvidenceHandleV2) -> bool {
    let frame_id = reference.frame_id.as_deref().or_else(|| {
        packet
            .canonical_elements
            .iter()
            .find(|item| item.element_id == reference.record_id)
            .map(|item| item.frame_id.as_str())
    });
    frame_id.is_some_and(|frame_id| {
        packet
            .semantic_keyframes
            .iter()
            .find(|frame| frame.frame_id == frame_id)
            .is_some_and(|frame| !frame.model_eligible)
    })
}

fn reference_is_control(packet: &ObservationPacketV2, reference: &EvidenceHandleV2) -> bool {
    packet
        .canonical_elements
        .iter()
        .find(|item| item.element_id == reference.record_id)
        .is_some_and(|item| is_control_role(item.region_role) || !item.task_eligible)
}

fn reference_is_historical_without_continuity(
    packet: &ObservationPacketV2,
    reference: &EvidenceHandleV2,
) -> bool {
    let Some(element) = packet
        .canonical_elements
        .iter()
        .find(|item| item.element_id == reference.record_id)
    else {
        return false;
    };
    if element.frame_id == packet.current_frame.frame_id {
        return false;
    }
    let prior_partition = packet
        .partitions
        .get(&EvidencePartitionV2::Prior)
        .is_some_and(|frames| frames.contains(&element.frame_id));
    prior_partition
        && !packet.causal_events.iter().any(|event| {
            event.frame_id == packet.current_frame.frame_id
                && event.causal_parent_ids.contains(&reference.record_id)
        })
}

fn reference_supports_user_authorship(
    packet: &ObservationPacketV2,
    reference: &EvidenceHandleV2,
) -> bool {
    if packet
        .causal_events
        .iter()
        .find(|item| item.event_id == reference.record_id)
        .is_some_and(|item| item.committed == Some(true) || item.source == "ui_event")
    {
        return true;
    }
    packet
        .canonical_elements
        .iter()
        .find(|item| item.element_id == reference.record_id)
        .is_some_and(|item| {
            item.authorship_status == AuthorshipStatusV2::User
                || !item.causal_evidence_refs.is_empty()
                || item.source_votes.iter().collect::<BTreeSet<_>>().len() >= 2
        })
}

fn is_task_identity_field(field: &str) -> bool {
    matches!(
        field,
        "task_summary" | "task_kind" | "task_object" | "user_goal"
    )
}

fn lifecycle_value_allowed(field: &str, value: &str) -> bool {
    match field {
        "execution_state" => matches!(
            value,
            "active"
                | "composing"
                | "editing"
                | "reviewing"
                | "waiting"
                | "debugging"
                | "searching"
                | "comparing"
                | "blocked"
                | "completed"
                | "idle_after_progress"
                | "unclear"
        ),
        "current_actor" => matches!(
            value,
            "user" | "assistant_or_agent" | "application" | "unknown"
        ),
        "waiting_on" => matches!(
            value,
            "user" | "assistant_or_agent" | "application" | "external" | "nothing" | "unknown"
        ),
        _ => true,
    }
}

fn generic_next_action(value: &str) -> bool {
    let normalized = value.trim().to_ascii_lowercase();
    normalized.len() < 8
        || matches!(
            normalized.as_str(),
            "continue" | "keep going" | "finish it" | "review" | "try again" | "proceed"
        )
}

pub(crate) fn deterministic_first_screen_wording(snapshot: &TaskSnapshotV2) -> String {
    let task = snapshot
        .task_summary
        .as_deref()
        .or(snapshot.user_goal.as_deref())
        .unwrap_or("Current task is unclear");
    let state = match snapshot.execution_state.as_str() {
        "unclear" => None,
        value => Some(value.replace('_', " ")),
    };
    let mut lines = vec![task.to_string()];
    if let Some(state) = state {
        lines.push(format!("State: {state}."));
    }
    if let Some(progress) = snapshot.last_meaningful_progress.as_deref() {
        lines.push(format!("Last progress: {progress}"));
    }
    if let Some(next) = snapshot.next_action.as_deref() {
        lines.push(format!("Next: {next}"));
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::continuation::task_truth_v2::model::{
        FieldContradictionV1, ModelClaimEvidenceV1, ModelTaskHypothesisV1,
    };
    use crate::continuation::task_truth_v2::observation_packet::{
        ActiveSurfaceIdentityV2, CanonicalElementV2, CausalEventV2, EvidencePartitionV2,
        KeyframeReferenceV2, PacketBoundsV2, PacketSizeAccountingV2, RegionRoleV2,
    };

    fn reference(record_id: &str, frame_id: Option<&str>) -> EvidenceHandleV2 {
        EvidenceHandleV2 {
            source_kind: "fixture".into(),
            record_id: record_id.into(),
            frame_id: frame_id.map(str::to_string),
            content_hash: None,
        }
    }

    fn element(
        id: &str,
        frame: &str,
        role: RegionRoleV2,
        authorship: AuthorshipStatusV2,
    ) -> CanonicalElementV2 {
        CanonicalElementV2 {
            element_id: id.into(),
            frame_id: frame.into(),
            bounds: Some(PacketBoundsV2 {
                x: 10.0,
                y: 10.0,
                width: 200.0,
                height: 40.0,
            }),
            text_reference: Some(format!("hash-{id}")),
            visual_description: None,
            native_role: Some(if role == RegionRoleV2::Control {
                "AXButton".into()
            } else {
                "AXStaticText".into()
            }),
            native_subrole: None,
            native_actionability: role == RegionRoleV2::Control,
            region_role: role,
            focused: false,
            editable: false,
            selected: false,
            interactive: role == RegionRoleV2::Control,
            parent_element_id: None,
            child_element_ids: Vec::new(),
            source_votes: vec!["ax".into(), "visual".into()],
            source_conflicts: Vec::new(),
            first_seen_at_ms: if frame == "frame-old" { 500 } else { 1_000 },
            changed_at_ms: if frame == "frame-old" { 500 } else { 1_000 },
            authorship_status: authorship,
            causal_evidence_refs: Vec::new(),
            task_eligible: role != RegionRoleV2::Control,
            rejection_reasons: (role == RegionRoleV2::Control)
                .then(|| vec!["categorical_control_ineligible".into()])
                .unwrap_or_default(),
        }
    }

    fn packet() -> ObservationPacketV2 {
        let current = KeyframeReferenceV2 {
            frame_id: "frame-current".into(),
            observed_at_ms: 1_000,
            partition: EvidencePartitionV2::Current,
            privacy_status: "allowed".into(),
            model_eligible: true,
            local_image_handle_hash: Some("image-hash".into()),
            ephemeral_local_image_path: None,
            selection_reasons: vec!["manual_continue_boundary".into()],
        };
        let old = KeyframeReferenceV2 {
            frame_id: "frame-old".into(),
            observed_at_ms: 500,
            partition: EvidencePartitionV2::Prior,
            privacy_status: "allowed".into(),
            model_eligible: true,
            local_image_handle_hash: Some("old-image-hash".into()),
            ephemeral_local_image_path: None,
            selection_reasons: vec!["causal_baseline".into()],
        };
        ObservationPacketV2 {
            schema: "smalltalk.observation_packet.v2".into(),
            packet_id: "packet-fixture".into(),
            observed_at_ms: 1_000,
            session_id: Some("session-fixture".into()),
            evidence_watermark: "watermark".into(),
            active_surface: ActiveSurfaceIdentityV2 {
                app_name: Some("Unfamiliar Canvas".into()),
                app_bundle_id: Some("com.fixture.canvas".into()),
                window_title_hash: Some("surface-hash".into()),
                window_id: Some(7),
                browser_url_hash: None,
                document_path_hash: Some("document-hash".into()),
            },
            current_frame: current.clone(),
            semantic_keyframes: vec![old, current],
            canonical_elements: vec![
                element(
                    "element-user",
                    "frame-current",
                    RegionRoleV2::UserAuthoredContent,
                    AuthorshipStatusV2::User,
                ),
                element(
                    "element-control",
                    "frame-current",
                    RegionRoleV2::Control,
                    AuthorshipStatusV2::ApplicationOrAgent,
                ),
                element(
                    "element-old",
                    "frame-old",
                    RegionRoleV2::PrimaryContent,
                    AuthorshipStatusV2::Unknown,
                ),
            ],
            focused_element_ids: Vec::new(),
            editable_element_ids: Vec::new(),
            selected_element_ids: Vec::new(),
            causal_events: vec![CausalEventV2 {
                event_id: "event-submit".into(),
                event_kind: "enter".into(),
                observed_at_ms: 990,
                frame_id: "frame-current".into(),
                partition: EvidencePartitionV2::Current,
                causal_parent_ids: Vec::new(),
                committed: Some(true),
                source: "typing_burst".into(),
            }],
            frame_changes: Vec::new(),
            capture_trigger_ids: vec!["trigger-manual".into()],
            transition_ids: vec!["transition-submit".into()],
            return_anchor_facts: vec![reference("anchor-current", Some("frame-current"))],
            previous_valid_snapshot_id: None,
            evidence_quality: "bounded_multisource".into(),
            missing_source_notes: vec!["thin_ax".into()],
            conflicting_observations: Vec::new(),
            partitions: BTreeMap::from([
                (EvidencePartitionV2::Current, vec!["frame-current".into()]),
                (EvidencePartitionV2::Prior, vec!["frame-old".into()]),
            ]),
            size: PacketSizeAccountingV2 {
                frame_count: 2,
                keyframe_count: 2,
                canonical_element_count: 3,
                causal_event_count: 1,
                serialized_bytes: 500,
                estimated_tokens: 125,
                truncated: false,
            },
        }
    }

    fn hypothesis(task_ref: &str) -> ModelTaskHypothesisV1 {
        let task = "Implement live corpus shadow evaluation";
        ModelTaskHypothesisV1 {
            hypothesis_id: "hypothesis-1".into(),
            task_summary: Some(task.into()),
            task_kind: Some("implementation".into()),
            task_object: Some("live corpus".into()),
            user_goal: Some(task.into()),
            app_identity: Some("Unfamiliar Canvas".into()),
            surface_identity_hash: Some("surface-hash".into()),
            document_or_thread_identity_hash: Some("document-hash".into()),
            execution_state: Some("active".into()),
            current_actor: Some("assistant_or_agent".into()),
            waiting_on: Some("assistant_or_agent".into()),
            last_meaningful_progress: Some("The submission was committed".into()),
            unfinished_step: Some("Complete the shadow evaluator".into()),
            next_action: Some("Run the frozen development evaluator".into()),
            relation_to_prior: Some("new_task".into()),
            return_anchor_record_id: None,
            claim_evidence: MATERIAL_FIELDS
                .iter()
                .map(|field| ModelClaimEvidenceV1 {
                    field: (*field).into(),
                    claim: field_value_placeholder(field),
                    evidence_refs: vec![reference(task_ref, Some("frame-current"))],
                    confidence: 0.84,
                })
                .collect(),
            contradictions: Vec::new(),
            confidence_by_field: MATERIAL_FIELDS
                .iter()
                .map(|field| ((*field).into(), 0.84))
                .collect(),
            confidence: 0.84,
        }
    }

    fn field_value_placeholder(field: &str) -> String {
        format!("fixture claim for {field}")
    }

    fn verdict(result: &VerificationResultV1, field: &str) -> FieldVerdictV1 {
        result
            .fields
            .iter()
            .find(|item| item.field == field)
            .map(|item| item.verdict)
            .unwrap()
    }

    #[test]
    fn model_button_label_cannot_be_the_goal_and_session_013_control_is_contained() {
        let result = LocalEvidenceVerifier.verify(
            &packet(),
            None,
            &[hypothesis("element-control")],
            ResolutionStatusV1::Resolved,
        );
        assert_eq!(verdict(&result, "task_summary"), FieldVerdictV1::Removed);
        assert!(result.snapshot.is_none());
        assert!(result
            .second_pass_reasons
            .contains(&"claim_references_control_region".into()));
    }

    #[test]
    fn old_completed_text_needs_a_temporal_continuity_edge() {
        let result = LocalEvidenceVerifier.verify(
            &packet(),
            None,
            &[hypothesis("element-old")],
            ResolutionStatusV1::Resolved,
        );
        assert_eq!(verdict(&result, "task_summary"), FieldVerdictV1::Downgraded);
        assert!(result.fields.iter().any(|field| field
            .reasons
            .contains(&"historical_evidence_without_temporal_continuity".into())));
    }

    #[test]
    fn invented_file_or_thread_identity_is_removed() {
        let mut hypothesis = hypothesis("element-user");
        hypothesis.document_or_thread_identity_hash = Some("invented-file".into());
        let result = LocalEvidenceVerifier.verify(
            &packet(),
            None,
            &[hypothesis],
            ResolutionStatusV1::Resolved,
        );
        assert_eq!(
            verdict(&result, "document_or_thread_identity_hash"),
            FieldVerdictV1::Removed
        );
        assert!(result
            .snapshot
            .unwrap()
            .document_or_thread_identity_hash
            .is_none());
    }

    #[test]
    fn plausible_but_generic_next_step_is_removed() {
        let mut hypothesis = hypothesis("element-user");
        hypothesis.next_action = Some("Continue".into());
        let result = LocalEvidenceVerifier.verify(
            &packet(),
            None,
            &[hypothesis],
            ResolutionStatusV1::Resolved,
        );
        assert_eq!(verdict(&result, "next_action"), FieldVerdictV1::Removed);
        assert!(result.snapshot.unwrap().next_action.is_none());
    }

    #[test]
    fn target_for_another_task_triggers_reconciliation_but_cannot_raise_confidence() {
        let mut hypothesis = hypothesis("element-user");
        hypothesis.return_anchor_record_id = Some("anchor-other-task".into());
        let result = LocalEvidenceVerifier.verify(
            &packet(),
            None,
            &[hypothesis],
            ResolutionStatusV1::Resolved,
        );
        assert!(result
            .second_pass_reasons
            .contains(&"task_return_anchor_disagree".into()));
        let snapshot = result.snapshot.unwrap();
        assert!(snapshot.return_anchor_candidate_id.is_none());
        assert_eq!(snapshot.confidence_by_field["task_summary"], 0.84);
    }

    #[test]
    fn nonexistent_evidence_ids_remove_material_claims() {
        let result = LocalEvidenceVerifier.verify(
            &packet(),
            None,
            &[hypothesis("missing-id")],
            ResolutionStatusV1::Resolved,
        );
        assert_eq!(verdict(&result, "task_summary"), FieldVerdictV1::Removed);
        assert!(result.snapshot.is_none());
    }

    #[test]
    fn close_hypotheses_preserve_ambiguity_and_request_second_pass() {
        let first = hypothesis("element-user");
        let mut second = first.clone();
        second.hypothesis_id = "hypothesis-2".into();
        second.task_summary = Some("Review the corpus measurements".into());
        second.confidence = 0.80;
        let result = LocalEvidenceVerifier.verify(
            &packet(),
            None,
            &[first, second],
            ResolutionStatusV1::Ambiguous,
        );
        assert_eq!(result.status, ResolutionStatusV1::Ambiguous);
        assert!(result
            .second_pass_reasons
            .contains(&"top_hypotheses_close".into()));
        assert_eq!(result.snapshot.unwrap().alternative_hypotheses.len(), 1);
    }

    #[test]
    fn ax_visual_conflict_is_a_bounded_second_pass_trigger() {
        let mut packet = packet();
        packet.canonical_elements[0]
            .source_conflicts
            .push("ax_ocr_text_disagreement".into());
        let result = LocalEvidenceVerifier.verify(
            &packet,
            None,
            &[hypothesis("element-user")],
            ResolutionStatusV1::Resolved,
        );
        assert!(result
            .second_pass_reasons
            .contains(&"ax_ocr_visual_conflict".into()));
    }

    #[test]
    fn correct_task_without_direct_target_stays_valid() {
        let result = LocalEvidenceVerifier.verify(
            &packet(),
            None,
            &[hypothesis("element-user")],
            ResolutionStatusV1::Resolved,
        );
        let snapshot = result.snapshot.unwrap();
        assert!(snapshot.task_summary.is_some());
        assert_eq!(
            snapshot.return_anchor_status,
            ReturnAnchorStatusV2::Unresolved
        );
    }

    #[test]
    fn unfamiliar_app_with_pixels_events_and_thin_ax_can_still_resolve() {
        let result = LocalEvidenceVerifier.verify(
            &packet(),
            None,
            &[hypothesis("element-user")],
            ResolutionStatusV1::Resolved,
        );
        assert_eq!(result.status, ResolutionStatusV1::Resolved);
        assert_eq!(
            result.snapshot.unwrap().app_identity.as_deref(),
            Some("Unfamiliar Canvas")
        );
    }

    #[test]
    fn contradiction_only_downgrades_affected_field() {
        let mut hypothesis = hypothesis("element-user");
        hypothesis.contradictions.push(FieldContradictionV1 {
            field: "waiting_on".into(),
            reason: "visible output and spinner disagree".into(),
            evidence_refs: vec![reference("element-user", Some("frame-current"))],
        });
        let result = LocalEvidenceVerifier.verify(
            &packet(),
            None,
            &[hypothesis],
            ResolutionStatusV1::Resolved,
        );
        assert_eq!(verdict(&result, "waiting_on"), FieldVerdictV1::Contradicted);
        assert_eq!(verdict(&result, "task_summary"), FieldVerdictV1::Accepted);
    }

    #[test]
    fn wording_preserves_verified_identity_and_nullable_target() {
        let snapshot = LocalEvidenceVerifier
            .verify(
                &packet(),
                None,
                &[hypothesis("element-user")],
                ResolutionStatusV1::Resolved,
            )
            .snapshot
            .unwrap();
        let wording = deterministic_first_screen_wording(&snapshot);
        assert!(wording.contains(snapshot.task_summary.as_deref().unwrap()));
        assert!(!wording.contains("anchor-current"));
    }
}
