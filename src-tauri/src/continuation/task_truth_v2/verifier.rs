use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

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

const MATERIAL_FIELDS: [&str; 16] = [
    "observed_surface",
    "immediate_user_operation",
    "semantic_effect_of_operation",
    "current_subtask",
    "likely_primary_task",
    "task_object",
    "app_identity",
    "surface_identity_hash",
    "document_or_thread_identity_hash",
    "execution_state",
    "current_actor",
    "waiting_on",
    "last_meaningful_progress",
    "unfinished_state",
    "possible_next_action",
    "relationship_to_prior",
];

const MAX_CLOSE_HYPOTHESIS_CONFIDENCE_GAP: f64 = 0.10;

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
            ResolutionStatusV1::PrivacyBlocked
                | ResolutionStatusV1::ModelUnavailable
                | ResolutionStatusV1::ProviderFailure
                | ResolutionStatusV1::InvalidResponse
                | ResolutionStatusV1::VerificationRejected
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
        if ranked.get(1).is_some_and(|next| {
            (selected.confidence - next.confidence).abs() <= MAX_CLOSE_HYPOTHESIS_CONFIDENCE_GAP
        }) {
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
                .get(field)
                .and_then(Option::as_ref)
                .into_iter()
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
            let mut hash_normalized = 0usize;
            let original_ref_count = refs.len();
            refs = refs
                .iter()
                .filter_map(|reference| {
                    normalize_reference(packet, prior, reference).map(|(normalized, changed)| {
                        hash_normalized += usize::from(changed);
                        normalized
                    })
                })
                .collect();
            let invalid = original_ref_count.saturating_sub(refs.len());
            if claims.is_empty() || refs.is_empty() {
                reasons.push("material_claim_without_evidence".into());
            }
            if invalid > 0 {
                reasons.push("referenced_id_not_in_request".into());
            }
            if hash_normalized > 0 {
                reasons.push("evidence_hash_normalized_to_request".into());
            }
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
            if matches!(
                field,
                "likely_primary_task" | "task_object" | "current_subtask"
            ) && !refs.is_empty()
                && !refs
                    .iter()
                    .any(|reference| reference_supports_user_authorship(packet, reference))
            {
                reasons.push("user_authorship_not_causally_supported".into());
            }
            if field == "immediate_user_operation"
                && !refs
                    .iter()
                    .any(|reference| reference.source_kind == "causal_event")
            {
                reasons.push("operation_without_causal_event".into());
            }
            if field == "semantic_effect_of_operation"
                && !refs
                    .iter()
                    .any(|reference| reference.source_kind == "semantic_delta")
            {
                reasons.push("semantic_effect_without_delta".into());
            }
            if field == "task_object"
                && !refs.iter().any(|reference| {
                    reference.source_kind == "canonical_element" && reference.content_hash.is_some()
                })
            {
                reasons.push("task_object_without_hashed_object_evidence".into());
            }
            if is_task_identity_field(field)
                && refs
                    .iter()
                    .any(|reference| reference.source_kind == "prior_snapshot")
                && packet.causal_events.iter().any(|event| {
                    event.partition == EvidencePartitionV2::Current
                        && event.grounding_confidence >= 0.55
                })
                && !matches!(
                    selected.relationship_to_prior,
                    super::model::TaskRelationshipV1::Continuation
                        | super::model::TaskRelationshipV1::ReturnToPriorTask
                )
            {
                reasons.push("prior_hypothesis_overridden_by_newer_causal_evidence".into());
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
            if field == "possible_next_action" {
                if selected
                    .unfinished_state
                    .as_deref()
                    .is_none_or(str::is_empty)
                {
                    reasons.push("next_action_without_unfinished_state".into());
                }
                if value.is_some_and(generic_next_action) {
                    reasons.push("generic_invented_next_action".into());
                }
                if value.is_some_and(unsafe_or_invented_next_action) {
                    reasons.push("unsafe_or_invented_next_action".into());
                }
                if !refs.iter().any(|reference| {
                    reference.source_kind == "canonical_element"
                        && reference.content_hash.is_some()
                        && packet
                            .canonical_elements
                            .iter()
                            .find(|element| element.element_id == reference.record_id)
                            .is_some_and(|element| {
                                element.authorship_status == AuthorshipStatusV2::User
                            })
                }) {
                    reasons.push("next_action_without_user_authored_plan".into());
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
            if field == "unfinished_state"
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
                        | "unsafe_or_invented_next_action"
                        | "next_action_without_unfinished_state"
                        | "lifecycle_value_not_supported_by_schema"
                        | "unfinished_step_conflicts_with_completed_state"
                        | "operation_without_causal_event"
                        | "semantic_effect_without_delta"
                        | "task_object_without_hashed_object_evidence"
                        | "prior_hypothesis_overridden_by_newer_causal_evidence"
                        | "next_action_without_user_authored_plan"
                        | "user_authorship_not_causally_supported"
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
                && matches!(
                    field,
                    "likely_primary_task" | "current_subtask" | "execution_state"
                )
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
            ResolutionStatusV1::VerificationRejected
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
        "observed_surface" => hypothesis.observed_surface.as_deref(),
        "immediate_user_operation" => hypothesis.immediate_user_operation.as_deref(),
        "semantic_effect_of_operation" => hypothesis.semantic_effect_of_operation.as_deref(),
        "current_subtask" => hypothesis.current_subtask.as_deref(),
        "likely_primary_task" => hypothesis.likely_primary_task.as_deref(),
        "task_object" => hypothesis.task_object.as_deref(),
        "app_identity" => hypothesis.app_identity.as_deref(),
        "surface_identity_hash" => hypothesis.surface_identity_hash.as_deref(),
        "document_or_thread_identity_hash" => {
            hypothesis.document_or_thread_identity_hash.as_deref()
        }
        "execution_state" => hypothesis.execution_state.as_deref(),
        "current_actor" => hypothesis.current_actor.as_deref(),
        "waiting_on" => hypothesis.waiting_on.as_deref(),
        "last_meaningful_progress" => hypothesis.last_meaningful_progress.as_deref(),
        "unfinished_state" => hypothesis.unfinished_state.as_deref(),
        "possible_next_action" => hypothesis.possible_next_action.as_deref(),
        "relationship_to_prior" => Some(hypothesis.relationship_to_prior.label()),
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
    let task_summary = accepted(fields, "likely_primary_task")
        .then(|| selected.likely_primary_task.clone())
        .flatten();
    let user_goal = accepted(fields, "current_subtask")
        .then(|| selected.current_subtask.clone())
        .flatten();
    if task_summary.is_none() {
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
    let mut ranked_alternatives = hypotheses
        .iter()
        .filter(|item| {
            item.hypothesis_id != selected.hypothesis_id
                && item.likely_primary_task.is_some()
                && (selected.confidence - item.confidence).abs()
                    <= MAX_CLOSE_HYPOTHESIS_CONFIDENCE_GAP
        })
        .collect::<Vec<_>>();
    ranked_alternatives.sort_by(|left, right| {
        right
            .confidence
            .partial_cmp(&left.confidence)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.hypothesis_id.cmp(&right.hypothesis_id))
    });
    let alternatives = ranked_alternatives
        .into_iter()
        .take(2)
        .map(|item| SnapshotHypothesisV2 {
            hypothesis_id: item.hypothesis_id.clone(),
            summary: item
                .likely_primary_task
                .clone()
                .unwrap_or_else(|| "Alternative interpretation".into()),
            relation: item.relationship_to_prior.label().into(),
            confidence: item.confidence,
            evidence_refs: item
                .claim_evidence
                .values()
                .filter_map(Option::as_ref)
                .flat_map(|claim| claim.evidence_refs.iter().cloned())
                .collect(),
            contradicting_evidence_refs: item
                .contradictions
                .iter()
                .flat_map(|contradiction| contradiction.evidence_refs.iter().cloned())
                .collect(),
            task_thread_id: item.continuity_thread_id.clone(),
            task_thread_revision: item.continuity_thread_revision,
            last_supported_at_ms: Some(packet.observed_at_ms),
            disposition: "retained_close_alternative".into(),
            reason_codes: vec![
                "model_hypothesis_retained_without_merging".into(),
                "confidence_within_0_10_of_selected".into(),
            ],
            semantic_payload: serde_json::to_value(item).ok(),
        })
        .collect::<Vec<_>>();
    let has_close_alternatives = !alternatives.is_empty();
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
        session_id: packet.session_id.clone(),
        task_thread_id: selected.continuity_thread_id.clone(),
        task_thread_revision: selected.continuity_thread_revision,
        thread_status: "unresolved".into(),
        continuity_thread_id: selected.continuity_thread_id.clone(),
        continuity_thread_revision: selected.continuity_thread_revision,
        continuity_identity_token: selected.continuity_identity_token.clone(),
        supersedes_thread_id: selected.supersedes_thread_id.clone(),
        legacy_task_turn_id: None,
        task_basis: "explicit_goal".into(),
        observed_surface: accepted(fields, "observed_surface")
            .then(|| selected.observed_surface.clone())
            .flatten(),
        immediate_user_operation: accepted(fields, "immediate_user_operation")
            .then(|| selected.immediate_user_operation.clone())
            .flatten(),
        semantic_effect_of_operation: accepted(fields, "semantic_effect_of_operation")
            .then(|| selected.semantic_effect_of_operation.clone())
            .flatten(),
        current_subtask: accepted(fields, "current_subtask")
            .then(|| selected.current_subtask.clone())
            .flatten(),
        task_summary,
        task_kind: "model_inferred".into(),
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
        unfinished_step: accepted(fields, "unfinished_state")
            .then(|| selected.unfinished_state.clone())
            .flatten(),
        next_action: accepted(fields, "possible_next_action")
            .then(|| selected.possible_next_action.clone())
            .flatten(),
        relation_to_prior: if accepted(fields, "relationship_to_prior") {
            selected.relationship_to_prior.label().into()
        } else {
            "unknown".into()
        },
        selection_status: if has_close_alternatives {
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
        semantic_source: "cloud_multimodal_model".into(),
        provider_name: None,
        provider_model: None,
        provider_request_id: None,
        provider_response_id: None,
        selected_hypothesis_id: Some(selected.hypothesis_id.clone()),
        wording_source: "deterministic".into(),
        inference_status: "verified".into(),
    })
}

fn normalize_reference(
    packet: &ObservationPacketV2,
    prior: Option<&TaskSnapshotV2>,
    reference: &EvidenceHandleV2,
) -> Option<(EvidenceHandleV2, bool)> {
    let frame_matches = |actual: Option<&str>| {
        reference
            .frame_id
            .as_deref()
            .is_none_or(|expected| actual == Some(expected))
    };
    let normalized = |frame_id: Option<&str>, content_hash: Option<&str>| {
        let supplied_hash = reference
            .content_hash
            .as_deref()
            .filter(|value| !value.trim().is_empty());
        let changed = supplied_hash != content_hash;
        (
            EvidenceHandleV2 {
                source_kind: reference.source_kind.clone(),
                record_id: reference.record_id.clone(),
                frame_id: frame_id.map(str::to_string),
                content_hash: content_hash.map(str::to_string),
            },
            changed,
        )
    };
    match reference.source_kind.as_str() {
        "canonical_element" => packet
            .canonical_elements
            .iter()
            .find(|item| item.element_id == reference.record_id)
            .filter(|item| frame_matches(Some(&item.frame_id)))
            .map(|item| normalized(Some(&item.frame_id), item.text_reference.as_deref())),
        "causal_event" => packet
            .causal_events
            .iter()
            .find(|item| item.event_id == reference.record_id)
            .filter(|item| frame_matches(Some(&item.frame_id)))
            .map(|item| normalized(Some(&item.frame_id), None)),
        "keyframe" => packet
            .semantic_keyframes
            .iter()
            .find(|item| item.frame_id == reference.record_id)
            .filter(|item| frame_matches(Some(&item.frame_id)))
            .map(|item| {
                normalized(
                    Some(&item.frame_id),
                    item.local_image_handle_hash.as_deref(),
                )
            }),
        "semantic_delta" => packet
            .frame_changes
            .iter()
            .find(|item| item.delta_id == reference.record_id)
            .filter(|item| frame_matches(Some(&item.frame_id)))
            .map(|item| normalized(Some(&item.frame_id), None)),
        "transition" => packet
            .transition_ids
            .contains(&reference.record_id)
            .then(|| normalized(reference.frame_id.as_deref(), None)),
        "capture_trigger" => packet
            .capture_trigger_ids
            .contains(&reference.record_id)
            .then(|| normalized(reference.frame_id.as_deref(), None)),
        "return_anchor_fact" => packet
            .return_anchor_facts
            .iter()
            .find(|item| item.record_id == reference.record_id)
            .filter(|item| frame_matches(item.frame_id.as_deref()))
            .map(|item| normalized(item.frame_id.as_deref(), item.content_hash.as_deref())),
        "prior_snapshot" => prior
            .filter(|item| {
                item.snapshot_id == reference.record_id
                    && packet.previous_valid_snapshot_id.as_deref()
                        == Some(item.snapshot_id.as_str())
                    && frame_matches(None)
            })
            .map(|_| normalized(None, None)),
        "prior_thread_revision" if reference.frame_id.is_none() => Some((reference.clone(), false)),
        _ => None,
    }
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
        })
}

fn is_task_identity_field(field: &str) -> bool {
    matches!(
        field,
        "likely_primary_task" | "task_object" | "current_subtask"
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
                | "interrupted"
                | "suspended"
                | "completed"
                | "superseded"
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

fn unsafe_or_invented_next_action(value: &str) -> bool {
    let normalized = value.trim().to_ascii_lowercase();
    [
        "delete ",
        "erase ",
        "drop ",
        "overwrite ",
        "reset ",
        "remove ",
        "send ",
        "submit ",
        "post ",
        "publish ",
        "force push ",
        "git reset ",
        "rm ",
        "sudo ",
    ]
    .iter()
    .any(|prefix| normalized.starts_with(prefix))
        || normalized.contains("http://")
        || normalized.contains("https://")
        || normalized.contains("file://")
        || normalized.contains("../")
        || normalized.contains("~/")
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
        FrameChangeV2, KeyframeReferenceV2, PacketBoundsV2, PacketSizeAccountingV2, RegionRoleV2,
    };
    use crate::continuation::task_truth_v2::task_snapshot::unresolved_snapshot;

    fn reference(record_id: &str, frame_id: Option<&str>) -> EvidenceHandleV2 {
        let source_kind = if record_id.starts_with("event-") {
            "causal_event"
        } else if record_id.starts_with("anchor-") {
            "return_anchor_fact"
        } else if record_id.starts_with("delta:") {
            "semantic_delta"
        } else {
            "canonical_element"
        };
        EvidenceHandleV2 {
            source_kind: source_kind.into(),
            record_id: record_id.into(),
            frame_id: frame_id.map(str::to_string),
            content_hash: (source_kind == "canonical_element").then(|| format!("hash-{record_id}")),
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
            display_id: Some("main".into()),
            window_id: Some(7),
            owning_app_bundle: Some("com.fixture.canvas".into()),
            source_scope: Some("active_window".into()),
            ownership_kind: Some("ActiveWindowOwned".into()),
            ownership_confidence: Some(0.95),
            coordinate_space: "active_window_pixels".into(),
            freshness: "current_frame".into(),
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
            surface_identity: ActiveSurfaceIdentityV2 {
                app_name: Some("Unfamiliar Canvas".into()),
                app_bundle_id: Some("com.fixture.canvas".into()),
                window_title_hash: Some("surface-hash".into()),
                window_id: Some(7),
                browser_url_hash: None,
                document_path_hash: Some("document-hash".into()),
            },
            surface_ownership_confidence: 0.95,
            privacy_status: "allowed".into(),
            model_eligible: true,
            image_source_kind: "native_active_window".into(),
            image_scope: "active_window".into(),
            image_width: None,
            image_height: None,
            image_rejection_reason: None,
            crop_pixels: None,
            local_image_handle_hash: Some("image-hash".into()),
            ephemeral_local_image_path: None,
            selection_reasons: vec!["manual_continue_boundary".into()],
            task_evidence_role: None,
            task_turn_id: None,
            same_task_relation: "unknown".into(),
            cross_pane_ambiguity: false,
            near_duplicate_group: None,
        };
        let old = KeyframeReferenceV2 {
            frame_id: "frame-old".into(),
            observed_at_ms: 500,
            partition: EvidencePartitionV2::Prior,
            surface_identity: ActiveSurfaceIdentityV2 {
                app_name: Some("Unfamiliar Canvas".into()),
                app_bundle_id: Some("com.fixture.canvas".into()),
                window_title_hash: Some("old-surface-hash".into()),
                window_id: Some(7),
                browser_url_hash: None,
                document_path_hash: Some("document-hash".into()),
            },
            surface_ownership_confidence: 0.95,
            privacy_status: "allowed".into(),
            model_eligible: true,
            image_source_kind: "native_active_window".into(),
            image_scope: "active_window".into(),
            image_width: None,
            image_height: None,
            image_rejection_reason: None,
            crop_pixels: None,
            local_image_handle_hash: Some("old-image-hash".into()),
            ephemeral_local_image_path: None,
            selection_reasons: vec!["causal_baseline".into()],
            task_evidence_role: None,
            task_turn_id: None,
            same_task_relation: "unknown".into(),
            cross_pane_ambiguity: false,
            near_duplicate_group: None,
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
            surface_timeline: Vec::new(),
            task_relevance: Default::default(),
            image_candidates: Vec::new(),
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
                source_frame_id: "frame-current".into(),
                target_frame_id: Some("frame-current".into()),
                target_element_id: Some("element-user".into()),
                target_region: Some(RegionRoleV2::UserAuthoredContent),
                focused_element_before: None,
                focused_element_after: None,
                window_id: Some(7),
                app_bundle_id: Some("com.fixture.canvas".into()),
                pointer_x: None,
                pointer_y: None,
                scroll_delta_x: None,
                scroll_delta_y: None,
                pre_state_reference: None,
                post_state_reference: Some("frame-current".into()),
                semantic_delta_reference: None,
                grounding_confidence: 0.9,
                missing_evidence: Vec::new(),
                conflicting_evidence: Vec::new(),
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
                frame_accounting: Vec::new(),
            },
        }
    }

    fn hypothesis(task_ref: &str) -> ModelTaskHypothesisV1 {
        let task = "Implement live corpus shadow evaluation";
        let evidence_frame = if task_ref == "element-old" {
            "frame-old"
        } else {
            "frame-current"
        };
        ModelTaskHypothesisV1 {
            hypothesis_id: "hypothesis-1".into(),
            observed_surface: Some("An implementation workspace".into()),
            immediate_user_operation: Some("Committed an implementation change".into()),
            semantic_effect_of_operation: Some("The evaluator implementation changed".into()),
            current_subtask: Some("Complete the shadow evaluator".into()),
            likely_primary_task: Some(task.into()),
            task_object: Some("live corpus".into()),
            app_identity: Some("Unfamiliar Canvas".into()),
            surface_identity_hash: Some("surface-hash".into()),
            document_or_thread_identity_hash: Some("document-hash".into()),
            execution_state: Some("active".into()),
            current_actor: Some("assistant_or_agent".into()),
            waiting_on: Some("assistant_or_agent".into()),
            last_meaningful_progress: Some("The submission was committed".into()),
            unfinished_state: Some("Complete the shadow evaluator".into()),
            possible_next_action: Some("Run the frozen development evaluator".into()),
            relationship_to_prior: super::super::model::TaskRelationshipV1::NewTask,
            continuity_thread_id: None,
            continuity_thread_revision: None,
            continuity_identity_token: None,
            supersedes_thread_id: None,
            return_anchor_record_id: None,
            claim_evidence: MATERIAL_FIELDS
                .iter()
                .map(|field| {
                    (
                        (*field).into(),
                        Some(ModelClaimEvidenceV1 {
                            claim: field_value_placeholder(field),
                            evidence_refs: vec![reference(task_ref, Some(evidence_frame))],
                            confidence: 0.84,
                        }),
                    )
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
        assert_eq!(
            verdict(&result, "likely_primary_task"),
            FieldVerdictV1::Removed
        );
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
        assert_eq!(
            verdict(&result, "likely_primary_task"),
            FieldVerdictV1::Removed
        );
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
        hypothesis.possible_next_action = Some("Continue".into());
        let result = LocalEvidenceVerifier.verify(
            &packet(),
            None,
            &[hypothesis],
            ResolutionStatusV1::Resolved,
        );
        assert_eq!(
            verdict(&result, "possible_next_action"),
            FieldVerdictV1::Removed
        );
        assert!(result.snapshot.unwrap().next_action.is_none());
    }

    #[test]
    fn destructive_or_locator_like_next_action_is_removed_field_locally() {
        for proposed in [
            "Delete the repository and start again",
            "Open https://invented.invalid/task/123",
        ] {
            let mut hypothesis = hypothesis("element-user");
            hypothesis.possible_next_action = Some(proposed.into());
            let result = LocalEvidenceVerifier.verify(
                &packet(),
                None,
                &[hypothesis],
                ResolutionStatusV1::Resolved,
            );
            assert_eq!(
                verdict(&result, "possible_next_action"),
                FieldVerdictV1::Removed
            );
            let snapshot = result.snapshot.expect("other admitted fields survive");
            assert!(snapshot.next_action.is_none());
            assert!(snapshot.task_summary.is_some());
        }
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
        assert_eq!(snapshot.confidence_by_field["likely_primary_task"], 0.84);
    }

    #[test]
    fn nonexistent_evidence_ids_remove_material_claims() {
        let result = LocalEvidenceVerifier.verify(
            &packet(),
            None,
            &[hypothesis("missing-id")],
            ResolutionStatusV1::Resolved,
        );
        assert_eq!(
            verdict(&result, "likely_primary_task"),
            FieldVerdictV1::Removed
        );
        assert!(result.snapshot.is_none());
    }

    #[test]
    fn known_evidence_id_with_drifted_hash_is_normalized_and_downgraded() {
        let mut hypothesis = hypothesis("element-user");
        hypothesis
            .claim_evidence
            .get_mut("observed_surface")
            .and_then(Option::as_mut)
            .unwrap()
            .evidence_refs[0]
            .content_hash = Some("hash-from-another-record".into());
        let result = LocalEvidenceVerifier.verify(
            &packet(),
            None,
            &[hypothesis],
            ResolutionStatusV1::Resolved,
        );
        let field = result
            .fields
            .iter()
            .find(|field| field.field == "observed_surface")
            .unwrap();
        assert_eq!(field.verdict, FieldVerdictV1::Downgraded);
        assert!(field
            .reasons
            .contains(&"evidence_hash_normalized_to_request".into()));
        assert_eq!(
            field.accepted_evidence_refs[0].content_hash.as_deref(),
            Some("hash-element-user")
        );
    }

    #[test]
    fn close_hypotheses_preserve_ambiguity_and_request_second_pass() {
        let first = hypothesis("element-user");
        let mut second = first.clone();
        second.hypothesis_id = "hypothesis-2".into();
        second.likely_primary_task = Some("Review the corpus measurements".into());
        second.relationship_to_prior = super::super::model::TaskRelationshipV1::UnrelatedOrUnknown;
        second.confidence = 0.80;
        let mut third = first.clone();
        third.hypothesis_id = "hypothesis-3".into();
        third.likely_primary_task = Some("Verify the evaluator output".into());
        third.relationship_to_prior = super::super::model::TaskRelationshipV1::Verification;
        third.confidence = 0.72;
        // Provider order is intentionally not confidence order. The verifier
        // must retain the actually close hypothesis and discard the far one.
        let result = LocalEvidenceVerifier.verify(
            &packet(),
            None,
            &[first, third, second],
            ResolutionStatusV1::Ambiguous,
        );
        assert_eq!(result.status, ResolutionStatusV1::Ambiguous);
        assert!(result
            .second_pass_reasons
            .contains(&"top_hypotheses_close".into()));
        let alternatives = result.snapshot.unwrap().alternative_hypotheses;
        assert_eq!(alternatives.len(), 1);
        assert_eq!(alternatives[0].hypothesis_id, "hypothesis-2");
        assert_eq!(alternatives[0].relation, "unrelated_or_unknown");
        assert!(alternatives[0]
            .reason_codes
            .contains(&"confidence_within_0_10_of_selected".into()));
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
        assert_eq!(
            verdict(&result, "likely_primary_task"),
            FieldVerdictV1::Accepted
        );
    }

    #[test]
    fn passive_visible_browser_page_is_observation_not_primary_task() {
        let mut packet = packet();
        packet.canonical_elements[0].region_role = RegionRoleV2::PrimaryContent;
        packet.canonical_elements[0].authorship_status = AuthorshipStatusV2::Unknown;
        packet.canonical_elements[0].causal_evidence_refs.clear();
        let result = LocalEvidenceVerifier.verify(
            &packet,
            None,
            &[hypothesis("element-user")],
            ResolutionStatusV1::Resolved,
        );
        assert_eq!(
            verdict(&result, "likely_primary_task"),
            FieldVerdictV1::Removed
        );
        assert!(result.snapshot.is_none());
    }

    #[test]
    fn agent_or_third_party_output_cannot_establish_user_task() {
        let mut packet = packet();
        packet.canonical_elements[0].region_role = RegionRoleV2::ApplicationAgentOutput;
        packet.canonical_elements[0].authorship_status = AuthorshipStatusV2::ApplicationOrAgent;
        let result = LocalEvidenceVerifier.verify(
            &packet,
            None,
            &[hypothesis("element-user")],
            ResolutionStatusV1::Resolved,
        );
        assert!(result.snapshot.is_none());
        assert!(result.fields.iter().any(|field| field
            .reasons
            .contains(&"user_authorship_not_causally_supported".into())));
    }

    #[test]
    fn grounded_cross_app_supporting_research_relationship_is_preserved() {
        let mut hypothesis = hypothesis("element-user");
        hypothesis.relationship_to_prior =
            super::super::model::TaskRelationshipV1::SupportingResearch;
        let result = LocalEvidenceVerifier.verify(
            &packet(),
            None,
            &[hypothesis],
            ResolutionStatusV1::Resolved,
        );
        let snapshot = result.snapshot.unwrap();
        assert_eq!(snapshot.relation_to_prior, "supporting_research");
        assert_eq!(snapshot.semantic_source, "cloud_multimodal_model");
    }

    #[test]
    fn invented_task_object_and_specific_next_action_are_removed_independently() {
        let mut hypothesis = hypothesis("element-user");
        hypothesis.task_object = Some("invented private repository".into());
        hypothesis.possible_next_action = Some("Delete the invented repository".into());
        for field in ["task_object", "possible_next_action"] {
            hypothesis
                .claim_evidence
                .get_mut(field)
                .and_then(Option::as_mut)
                .unwrap()
                .evidence_refs = vec![reference("event-submit", Some("frame-current"))];
        }
        let result = LocalEvidenceVerifier.verify(
            &packet(),
            None,
            &[hypothesis],
            ResolutionStatusV1::Resolved,
        );
        assert_eq!(verdict(&result, "task_object"), FieldVerdictV1::Removed);
        assert_eq!(
            verdict(&result, "possible_next_action"),
            FieldVerdictV1::Removed
        );
        let snapshot = result.snapshot.unwrap();
        assert!(snapshot.task_object.is_none());
        assert!(snapshot.next_action.is_none());
    }

    #[test]
    fn newer_causal_evidence_prevents_prior_hypothesis_override() {
        let mut packet = packet();
        let prior = unresolved_snapshot(&packet, None, "prior_fixture");
        packet.previous_valid_snapshot_id = Some(prior.snapshot_id.clone());
        let mut hypothesis = hypothesis("element-user");
        hypothesis.relationship_to_prior = super::super::model::TaskRelationshipV1::NewTask;
        let prior_ref = EvidenceHandleV2 {
            source_kind: "prior_snapshot".into(),
            record_id: prior.snapshot_id.clone(),
            frame_id: None,
            content_hash: None,
        };
        hypothesis
            .claim_evidence
            .get_mut("likely_primary_task")
            .and_then(Option::as_mut)
            .unwrap()
            .evidence_refs = vec![prior_ref.clone()];
        let result = LocalEvidenceVerifier.verify(
            &packet,
            Some(&prior),
            &[hypothesis],
            ResolutionStatusV1::Resolved,
        );
        assert_eq!(
            verdict(&result, "likely_primary_task"),
            FieldVerdictV1::Removed
        );
        assert!(result.fields.iter().any(|field| field
            .reasons
            .contains(&"prior_hypothesis_overridden_by_newer_causal_evidence".into())));
    }

    #[test]
    fn semantic_effect_requires_a_real_delta_reference() {
        let mut packet = packet();
        packet.frame_changes.push(FrameChangeV2 {
            delta_id: "delta:frame-current".into(),
            frame_id: "frame-current".into(),
            prior_frame_id: Some("frame-old".into()),
            next_frame_id: "frame-current".into(),
            diff_kind: Some("content_changed".into()),
            changed_regions: Vec::new(),
            observable_changes: vec!["content_appeared".into()],
            no_observable_change: false,
            source_agreement: vec!["frame_diff".into()],
            source_conflicts: Vec::new(),
            causal_event_ids: vec!["event-submit".into()],
            summary_hash: None,
            added_text_hashes: Some("[\"hash\"]".into()),
            removed_text_hashes: None,
        });
        let mut hypothesis = hypothesis("element-user");
        hypothesis
            .claim_evidence
            .get_mut("semantic_effect_of_operation")
            .and_then(Option::as_mut)
            .unwrap()
            .evidence_refs = vec![reference("delta:frame-current", Some("frame-current"))];
        let result = LocalEvidenceVerifier.verify(
            &packet,
            None,
            &[hypothesis],
            ResolutionStatusV1::Resolved,
        );
        assert_eq!(
            verdict(&result, "semantic_effect_of_operation"),
            FieldVerdictV1::Accepted
        );
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
        let identity_before = snapshot.selected_hypothesis_id.clone();
        let relation_before = snapshot.relation_to_prior.clone();
        let wording = deterministic_first_screen_wording(&snapshot);
        assert!(wording.contains(snapshot.task_summary.as_deref().unwrap()));
        assert!(!wording.contains("anchor-current"));
        assert_eq!(snapshot.selected_hypothesis_id, identity_before);
        assert_eq!(snapshot.relation_to_prior, relation_before);
    }
}
