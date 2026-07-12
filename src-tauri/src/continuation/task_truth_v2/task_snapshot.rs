use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use super::super::{stable_hash, CurrentTaskTurn};
use super::observation_packet::{EvidenceHandleV2, ObservationPacketV2};

pub(crate) const TASK_SNAPSHOT_SCHEMA_V2: &str = "smalltalk.task_snapshot.v2";

fn default_task_basis() -> String {
    "unresolved".into()
}

fn default_semantic_source() -> String {
    "unresolved".into()
}

fn default_wording_source() -> String {
    "deterministic".into()
}

fn default_thread_status() -> String {
    "unresolved".into()
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SnapshotSelectionStatusV2 {
    Selected,
    Alternative,
    Unresolved,
    Superseded,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ReturnAnchorStatusV2 {
    Unresolved,
    CandidateOnly,
    Safe,
    Suppressed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct ClaimEvidenceV2 {
    pub(crate) claim: String,
    pub(crate) evidence_refs: Vec<EvidenceHandleV2>,
    pub(crate) confidence: f64,
    pub(crate) source_confidence: BTreeMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct SnapshotHypothesisV2 {
    #[serde(default)]
    pub(crate) hypothesis_id: String,
    pub(crate) summary: String,
    pub(crate) relation: String,
    pub(crate) confidence: f64,
    pub(crate) evidence_refs: Vec<EvidenceHandleV2>,
    #[serde(default)]
    pub(crate) contradicting_evidence_refs: Vec<EvidenceHandleV2>,
    #[serde(default)]
    pub(crate) task_thread_id: Option<String>,
    #[serde(default)]
    pub(crate) task_thread_revision: Option<i64>,
    #[serde(default)]
    pub(crate) last_supported_at_ms: Option<i64>,
    #[serde(default)]
    pub(crate) disposition: String,
    #[serde(default)]
    pub(crate) reason_codes: Vec<String>,
    #[serde(default)]
    pub(crate) semantic_payload: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct TaskSnapshotV2 {
    pub(crate) schema: String,
    pub(crate) snapshot_id: String,
    pub(crate) revision: i64,
    pub(crate) prior_snapshot_id: Option<String>,
    pub(crate) supersedes_snapshot_id: Option<String>,
    pub(crate) observed_at_ms: i64,
    pub(crate) evidence_watermark: String,
    pub(crate) packet_id: String,
    #[serde(default)]
    pub(crate) session_id: Option<String>,
    #[serde(default)]
    pub(crate) task_thread_id: Option<String>,
    #[serde(default)]
    pub(crate) task_thread_revision: Option<i64>,
    #[serde(default = "default_thread_status")]
    pub(crate) thread_status: String,
    #[serde(default)]
    pub(crate) continuity_thread_id: Option<String>,
    #[serde(default)]
    pub(crate) continuity_thread_revision: Option<i64>,
    #[serde(default)]
    pub(crate) continuity_identity_token: Option<String>,
    #[serde(default)]
    pub(crate) supersedes_thread_id: Option<String>,
    pub(crate) legacy_task_turn_id: Option<String>,
    #[serde(default = "default_task_basis")]
    pub(crate) task_basis: String,
    #[serde(default)]
    pub(crate) observed_surface: Option<String>,
    #[serde(default)]
    pub(crate) immediate_user_operation: Option<String>,
    #[serde(default)]
    pub(crate) semantic_effect_of_operation: Option<String>,
    #[serde(default)]
    pub(crate) current_subtask: Option<String>,
    pub(crate) task_summary: Option<String>,
    pub(crate) task_kind: String,
    pub(crate) task_object: Option<String>,
    pub(crate) user_goal: Option<String>,
    pub(crate) app_identity: Option<String>,
    pub(crate) surface_identity_hash: Option<String>,
    pub(crate) document_or_thread_identity_hash: Option<String>,
    pub(crate) execution_state: String,
    pub(crate) current_actor: String,
    pub(crate) waiting_on: String,
    pub(crate) last_meaningful_progress: Option<String>,
    pub(crate) unfinished_step: Option<String>,
    pub(crate) next_action: Option<String>,
    pub(crate) relation_to_prior: String,
    pub(crate) selection_status: SnapshotSelectionStatusV2,
    pub(crate) claim_evidence: Vec<ClaimEvidenceV2>,
    pub(crate) alternative_hypotheses: Vec<SnapshotHypothesisV2>,
    pub(crate) contradictions: Vec<String>,
    pub(crate) confidence_by_field: BTreeMap<String, f64>,
    pub(crate) confidence_by_source: BTreeMap<String, f64>,
    pub(crate) return_anchor_candidate_id: Option<String>,
    pub(crate) return_anchor_status: ReturnAnchorStatusV2,
    pub(crate) resolver_version: String,
    pub(crate) provenance: Vec<String>,
    pub(crate) continuity_confidence_decay: f64,
    #[serde(default = "default_semantic_source")]
    pub(crate) semantic_source: String,
    #[serde(default)]
    pub(crate) provider_name: Option<String>,
    #[serde(default)]
    pub(crate) provider_model: Option<String>,
    #[serde(default)]
    pub(crate) provider_request_id: Option<String>,
    #[serde(default)]
    pub(crate) provider_response_id: Option<String>,
    #[serde(default)]
    pub(crate) selected_hypothesis_id: Option<String>,
    #[serde(default = "default_wording_source")]
    pub(crate) wording_source: String,
    #[serde(default)]
    pub(crate) inference_status: String,
}

fn evidence_refs(turn: &CurrentTaskTurn, packet: &ObservationPacketV2) -> Vec<EvidenceHandleV2> {
    let mut refs = turn
        .latest_user_span_ids
        .iter()
        .chain(turn.current_state_span_ids.iter())
        .map(|id| EvidenceHandleV2 {
            source_kind: "ordered_evidence_span".into(),
            record_id: id.clone(),
            frame_id: Some(packet.current_frame.frame_id.clone()),
            content_hash: None,
        })
        .collect::<Vec<_>>();
    refs.extend(turn.supporting_event_ids.iter().map(|id| EvidenceHandleV2 {
        source_kind: "causal_event".into(),
        record_id: id.clone(),
        frame_id: None,
        content_hash: None,
    }));
    refs.sort_by(|left, right| {
        left.source_kind
            .cmp(&right.source_kind)
            .then_with(|| left.record_id.cmp(&right.record_id))
    });
    refs.dedup_by(|left, right| {
        left.source_kind == right.source_kind && left.record_id == right.record_id
    });
    refs
}

fn snapshot_id(turn_id: &str, revision: i64) -> String {
    format!(
        "snapshot-{}",
        stable_hash(format!("{turn_id}:{revision}").as_bytes())
    )
}

pub(crate) fn project_current_task_turn(
    turn: &CurrentTaskTurn,
    packet: &ObservationPacketV2,
    prior_snapshot: Option<&TaskSnapshotV2>,
) -> TaskSnapshotV2 {
    let revision = prior_snapshot
        .filter(|prior| prior.legacy_task_turn_id.as_deref() == Some(turn.task_turn_id.as_str()))
        .map(|prior| prior.revision + 1)
        .unwrap_or_else(|| turn.revision.max(1));
    let refs = evidence_refs(turn, packet);
    let mut confidence_by_field = BTreeMap::new();
    confidence_by_field.insert("task_summary".into(), turn.goal_confidence);
    confidence_by_field.insert("task_object".into(), turn.task_object_confidence);
    confidence_by_field.insert("execution_state".into(), turn.execution_state_confidence);
    confidence_by_field.insert("current_actor".into(), turn.actor_state_confidence);
    confidence_by_field.insert("waiting_on".into(), turn.waiting_on_confidence);
    confidence_by_field.insert("relation_to_prior".into(), turn.relation_confidence);
    let mut confidence_by_source = BTreeMap::new();
    confidence_by_source.insert(
        "legacy_current_task_turn_projection".into(),
        turn.attribution_confidence,
    );
    confidence_by_source.insert(
        "observation_packet".into(),
        if packet.evidence_quality == "bounded_multisource" {
            0.8
        } else {
            0.45
        },
    );
    let task_summary = turn.latest_user_goal_summary.clone();
    let claim_evidence = [
        ("task_summary", turn.goal_confidence),
        ("task_object", turn.task_object_confidence),
        ("execution_state", turn.execution_state_confidence),
        ("current_actor", turn.actor_state_confidence),
        ("waiting_on", turn.waiting_on_confidence),
    ]
    .into_iter()
    .map(|(claim, confidence)| ClaimEvidenceV2 {
        claim: claim.into(),
        evidence_refs: refs.clone(),
        confidence,
        source_confidence: confidence_by_source.clone(),
    })
    .collect();
    TaskSnapshotV2 {
        schema: TASK_SNAPSHOT_SCHEMA_V2.into(),
        snapshot_id: snapshot_id(&turn.task_turn_id, revision),
        revision,
        prior_snapshot_id: prior_snapshot.map(|prior| prior.snapshot_id.clone()),
        supersedes_snapshot_id: prior_snapshot.map(|prior| prior.snapshot_id.clone()),
        observed_at_ms: packet.observed_at_ms,
        evidence_watermark: packet.evidence_watermark.clone(),
        packet_id: packet.packet_id.clone(),
        session_id: packet.session_id.clone(),
        task_thread_id: None,
        task_thread_revision: None,
        thread_status: "unresolved".into(),
        continuity_thread_id: None,
        continuity_thread_revision: None,
        continuity_identity_token: None,
        supersedes_thread_id: None,
        legacy_task_turn_id: Some(turn.task_turn_id.clone()),
        task_basis: "explicit_goal".into(),
        observed_surface: None,
        immediate_user_operation: None,
        semantic_effect_of_operation: None,
        current_subtask: None,
        task_summary: task_summary.clone(),
        task_kind: turn.task_kind.clone(),
        task_object: turn.task_object.clone(),
        user_goal: turn.latest_user_goal_summary.clone(),
        app_identity: packet.active_surface.app_name.clone(),
        surface_identity_hash: turn.surface_key_hash.clone(),
        document_or_thread_identity_hash: packet
            .active_surface
            .document_path_hash
            .clone()
            .or_else(|| packet.active_surface.browser_url_hash.clone()),
        execution_state: turn.execution_state.label().into(),
        current_actor: turn.current_actor.label().into(),
        waiting_on: turn.waiting_on.label().into(),
        last_meaningful_progress: turn.actor_activity_state.clone(),
        unfinished_step: match turn.execution_state.label() {
            "completed" | "superseded" => None,
            _ => turn
                .actor_activity_state
                .clone()
                .or_else(|| task_summary.clone()),
        },
        next_action: None,
        relation_to_prior: turn.relation_to_prior.label().into(),
        selection_status: SnapshotSelectionStatusV2::Selected,
        claim_evidence,
        alternative_hypotheses: Vec::new(),
        contradictions: turn.quality_flags.clone(),
        confidence_by_field,
        confidence_by_source,
        return_anchor_candidate_id: None,
        return_anchor_status: ReturnAnchorStatusV2::Unresolved,
        resolver_version: "task_truth_v2.local_projection.v1".into(),
        provenance: vec![
            "p6_current_task_turn_compatibility_projection".into(),
            "target_features_excluded_from_task_confidence".into(),
        ],
        continuity_confidence_decay: 0.0,
        semantic_source: "unresolved".into(),
        provider_name: None,
        provider_model: None,
        provider_request_id: None,
        provider_response_id: None,
        selected_hypothesis_id: None,
        wording_source: "deterministic".into(),
        inference_status: "no_inference".into(),
    }
}

pub(crate) fn unresolved_snapshot(
    packet: &ObservationPacketV2,
    prior_snapshot: Option<&TaskSnapshotV2>,
    reason: &str,
) -> TaskSnapshotV2 {
    let revision = prior_snapshot.map(|prior| prior.revision + 1).unwrap_or(1);
    let seed = format!("unresolved:{}:{}", packet.packet_id, revision);
    TaskSnapshotV2 {
        schema: TASK_SNAPSHOT_SCHEMA_V2.into(),
        snapshot_id: format!("snapshot-{}", stable_hash(seed.as_bytes())),
        revision,
        prior_snapshot_id: prior_snapshot.map(|prior| prior.snapshot_id.clone()),
        supersedes_snapshot_id: None,
        observed_at_ms: packet.observed_at_ms,
        evidence_watermark: packet.evidence_watermark.clone(),
        packet_id: packet.packet_id.clone(),
        session_id: packet.session_id.clone(),
        task_thread_id: None,
        task_thread_revision: None,
        thread_status: "unresolved".into(),
        continuity_thread_id: None,
        continuity_thread_revision: None,
        continuity_identity_token: None,
        supersedes_thread_id: None,
        legacy_task_turn_id: None,
        task_basis: "unresolved".into(),
        observed_surface: None,
        immediate_user_operation: None,
        semantic_effect_of_operation: None,
        current_subtask: None,
        task_summary: None,
        task_kind: "unknown".into(),
        task_object: None,
        user_goal: None,
        app_identity: packet.active_surface.app_name.clone(),
        surface_identity_hash: None,
        document_or_thread_identity_hash: None,
        execution_state: "unclear".into(),
        current_actor: "unknown".into(),
        waiting_on: "unknown".into(),
        last_meaningful_progress: None,
        unfinished_step: None,
        next_action: None,
        relation_to_prior: if prior_snapshot.is_some() {
            "continuity_unproven".into()
        } else {
            "unknown".into()
        },
        selection_status: SnapshotSelectionStatusV2::Unresolved,
        claim_evidence: Vec::new(),
        alternative_hypotheses: Vec::new(),
        contradictions: vec![reason.into()],
        confidence_by_field: BTreeMap::new(),
        confidence_by_source: BTreeMap::from([("observation_packet".into(), 0.0)]),
        return_anchor_candidate_id: None,
        return_anchor_status: ReturnAnchorStatusV2::Unresolved,
        resolver_version: "task_truth_v2.local_projection.v1".into(),
        provenance: vec!["uncertainty_persisted_without_stale_task_carry_forward".into()],
        continuity_confidence_decay: if prior_snapshot.is_some() { 0.2 } else { 0.0 },
        semantic_source: "unresolved".into(),
        provider_name: None,
        provider_model: None,
        provider_request_id: None,
        provider_response_id: None,
        selected_hypothesis_id: None,
        wording_source: "deterministic".into(),
        inference_status: reason.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::continuation::task_truth_v2::observation_packet::{
        ActiveSurfaceIdentityV2, EvidencePartitionV2, KeyframeReferenceV2, PacketSizeAccountingV2,
    };
    use crate::continuation::task_truth_v2::selection::select_snapshot;
    use crate::continuation::task_turn::{
        TaskExecutionState, TaskTurnActor, TaskTurnRelation, TaskTurnWaitingOn,
        CURRENT_TASK_TURN_SCHEMA_V2,
    };

    fn packet(observed_at_ms: i64) -> ObservationPacketV2 {
        let current_frame = KeyframeReferenceV2 {
            frame_id: format!("frame-{observed_at_ms}"),
            observed_at_ms,
            partition: EvidencePartitionV2::Current,
            surface_identity: ActiveSurfaceIdentityV2 {
                app_name: Some("Test App".into()),
                app_bundle_id: Some("com.example.test".into()),
                window_title_hash: Some("window-hash".into()),
                window_id: Some(1),
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
            local_image_handle_hash: None,
            ephemeral_local_image_path: None,
            selection_reasons: vec!["current_frame".into()],
        };
        ObservationPacketV2 {
            schema: "smalltalk.observation_packet.v2".into(),
            packet_id: format!("packet-{observed_at_ms}"),
            observed_at_ms,
            session_id: Some("session-test".into()),
            evidence_watermark: format!("watermark-{observed_at_ms}"),
            active_surface: ActiveSurfaceIdentityV2 {
                app_name: Some("Test App".into()),
                app_bundle_id: Some("com.example.test".into()),
                window_title_hash: Some("window-hash".into()),
                window_id: Some(1),
                browser_url_hash: None,
                document_path_hash: Some("document-hash".into()),
            },
            current_frame: current_frame.clone(),
            semantic_keyframes: vec![current_frame],
            canonical_elements: Vec::new(),
            focused_element_ids: Vec::new(),
            editable_element_ids: Vec::new(),
            selected_element_ids: Vec::new(),
            causal_events: Vec::new(),
            frame_changes: Vec::new(),
            capture_trigger_ids: Vec::new(),
            transition_ids: Vec::new(),
            return_anchor_facts: Vec::new(),
            previous_valid_snapshot_id: None,
            evidence_quality: "bounded_multisource".into(),
            missing_source_notes: Vec::new(),
            conflicting_observations: Vec::new(),
            partitions: BTreeMap::from([(
                EvidencePartitionV2::Current,
                vec![format!("frame-{observed_at_ms}")],
            )]),
            size: PacketSizeAccountingV2 {
                frame_count: 1,
                keyframe_count: 1,
                canonical_element_count: 0,
                causal_event_count: 0,
                serialized_bytes: 100,
                estimated_tokens: 25,
                truncated: false,
                frame_accounting: Vec::new(),
            },
        }
    }

    fn turn(state: TaskExecutionState, actor: TaskTurnActor) -> CurrentTaskTurn {
        CurrentTaskTurn {
            schema_version: CURRENT_TASK_TURN_SCHEMA_V2.into(),
            task_turn_id: "turn-1".into(),
            session_id: Some("session-test".into()),
            surface_key_hash: Some("surface-hash".into()),
            artifact_id: None,
            workstream_id: None,
            started_at_ms: 1_000,
            last_observed_at_ms: 1_000,
            latest_user_goal_summary: Some("Implement observation packets".into()),
            latest_user_goal_hash: Some("goal-hash".into()),
            latest_user_goal_redaction_status: "derived_redacted".into(),
            task_object: Some("observation packets".into()),
            task_object_hash: Some("object-hash".into()),
            task_object_redaction_status: "derived_redacted".into(),
            task_kind: "implementation".into(),
            current_actor: actor,
            actor_activity_state: Some("Building the local shadow model".into()),
            actor_activity_state_hash: Some("state-hash".into()),
            actor_activity_state_redaction_status: "derived_redacted".into(),
            execution_state: state,
            waiting_on: if actor == TaskTurnActor::User {
                TaskTurnWaitingOn::User
            } else {
                TaskTurnWaitingOn::Agent
            },
            relation_to_prior: TaskTurnRelation::Continuation,
            prior_task_turn_id: None,
            supersedes_task_turn_id: None,
            parent_task_turn_id: None,
            latest_user_span_ids: vec!["span-user".into()],
            current_state_span_ids: vec!["span-state".into()],
            prior_boundary_span_ids: Vec::new(),
            supporting_action_ids: vec!["action-1".into()],
            supporting_event_ids: vec!["event-submit".into()],
            evidence_quality: "strong".into(),
            goal_confidence: 0.9,
            task_object_confidence: 0.85,
            actor_state_confidence: 0.8,
            execution_state_confidence: 0.85,
            waiting_on_confidence: 0.8,
            relation_confidence: 0.8,
            attribution_confidence: 0.9,
            missing_evidence: Vec::new(),
            quality_flags: Vec::new(),
            reason_codes: vec!["causal_user_action".into()],
            revision: 1,
            selected: true,
            updated_at_ms: 1_000,
        }
    }

    #[test]
    fn revisions_preserve_submit_waiting_response_review_lineage() {
        let submit = project_current_task_turn(
            &turn(TaskExecutionState::Active, TaskTurnActor::User),
            &packet(1_000),
            None,
        );
        let waiting = project_current_task_turn(
            &turn(TaskExecutionState::Active, TaskTurnActor::AssistantOrAgent),
            &packet(2_000),
            Some(&submit),
        );
        let response = project_current_task_turn(
            &turn(
                TaskExecutionState::IdleAfterProgress,
                TaskTurnActor::AssistantOrAgent,
            ),
            &packet(3_000),
            Some(&waiting),
        );
        let review = project_current_task_turn(
            &turn(TaskExecutionState::Active, TaskTurnActor::User),
            &packet(4_000),
            Some(&response),
        );
        assert_eq!(
            [
                submit.revision,
                waiting.revision,
                response.revision,
                review.revision
            ],
            [1, 2, 3, 4]
        );
        assert_eq!(
            waiting.prior_snapshot_id.as_deref(),
            Some(submit.snapshot_id.as_str())
        );
        assert_eq!(
            response.prior_snapshot_id.as_deref(),
            Some(waiting.snapshot_id.as_str())
        );
        assert_eq!(
            review.prior_snapshot_id.as_deref(),
            Some(response.snapshot_id.as_str())
        );
    }

    #[test]
    fn support_activity_does_not_supersede_primary_task() {
        let primary = project_current_task_turn(
            &turn(TaskExecutionState::Active, TaskTurnActor::AssistantOrAgent),
            &packet(1_000),
            None,
        );
        let mut support = primary.clone();
        support.snapshot_id = "snapshot-support".into();
        support.observed_at_ms = 2_000;
        support.relation_to_prior = "child_support_step".into();
        let selected = select_snapshot(&[primary.clone(), support]);
        assert!(selected.selected_snapshot_id.is_none());
        assert!(selected.unresolved);
    }

    #[test]
    fn openable_stale_target_cannot_change_snapshot_selection() {
        let primary = project_current_task_turn(
            &turn(TaskExecutionState::Active, TaskTurnActor::AssistantOrAgent),
            &packet(1_000),
            None,
        );
        let mut stale = primary.clone();
        stale.snapshot_id = "snapshot-stale".into();
        stale.observed_at_ms = 500;
        stale.execution_state = "completed".into();
        stale.return_anchor_candidate_id = Some("openable-stale-url".into());
        stale.return_anchor_status = ReturnAnchorStatusV2::Safe;
        let baseline = select_snapshot(&[primary.clone(), stale.clone()]);
        stale.return_anchor_candidate_id = None;
        stale.return_anchor_status = ReturnAnchorStatusV2::Unresolved;
        let without_openability = select_snapshot(&[primary.clone(), stale]);
        assert_eq!(
            baseline.selected_snapshot_id,
            without_openability.selected_snapshot_id
        );
        assert!(baseline.selected_snapshot_id.is_none());
    }

    #[test]
    fn no_evidence_checkpoint_is_unresolved_not_stale_precise_truth() {
        let prior = project_current_task_turn(
            &turn(TaskExecutionState::Active, TaskTurnActor::AssistantOrAgent),
            &packet(1_000),
            None,
        );
        let unresolved = unresolved_snapshot(&packet(2_000), Some(&prior), "no_current_evidence");
        assert_eq!(
            unresolved.selection_status,
            SnapshotSelectionStatusV2::Unresolved
        );
        assert!(unresolved.task_summary.is_none());
        assert!(unresolved.user_goal.is_none());
        assert_eq!(unresolved.execution_state, "unclear");
        assert!(unresolved.continuity_confidence_decay > 0.0);
    }

    #[test]
    fn provider_failure_cannot_select_a_local_semantic_fallback_label() {
        let mut local = project_current_task_turn(
            &turn(TaskExecutionState::Active, TaskTurnActor::User),
            &packet(1_000),
            None,
        );
        local.task_thread_id = Some("thread-prior".into());
        local.task_thread_revision = Some(4);
        local.thread_status = "active".into();
        let unresolved = unresolved_snapshot(
            &packet(2_000),
            Some(&local),
            "cloud_inference_unresolved:provider_failure:provider_error",
        );
        let selection = select_snapshot(&[local.clone(), unresolved.clone()]);
        assert!(selection.selected_snapshot_id.is_none());
        assert!(selection.unresolved);
        assert!(unresolved.task_summary.is_none());
        assert_eq!(unresolved.semantic_source, "unresolved");
        assert_eq!(
            unresolved.prior_snapshot_id.as_deref(),
            Some(local.snapshot_id.as_str())
        );
        assert!(unresolved.task_thread_id.is_none());
        assert!(unresolved.task_thread_revision.is_none());
        assert!(unresolved.continuity_thread_id.is_none());
        assert_eq!(unresolved.relation_to_prior, "continuity_unproven");
    }
}
