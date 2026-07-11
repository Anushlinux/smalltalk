use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

use super::task_snapshot::{SnapshotSelectionStatusV2, TaskSnapshotV2};

pub(crate) const SNAPSHOT_SELECTION_POLICY_V1: &str =
    "smalltalk.task_truth_v2.snapshot_selection.v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct SnapshotSelectionScoreV2 {
    pub(crate) snapshot_id: String,
    pub(crate) temporal_continuity: f64,
    pub(crate) causal_user_action: f64,
    pub(crate) unfinished_state: f64,
    pub(crate) explicit_correction: f64,
    pub(crate) surface_continuity: f64,
    pub(crate) prior_relation: f64,
    pub(crate) confidence: f64,
    pub(crate) contradiction_penalty: f64,
    pub(crate) total: f64,
    pub(crate) reason_codes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct SnapshotSelectionResultV2 {
    pub(crate) policy_version: String,
    pub(crate) selected_snapshot_id: Option<String>,
    pub(crate) unresolved: bool,
    pub(crate) scores: Vec<SnapshotSelectionScoreV2>,
    pub(crate) excluded_feature_families: Vec<String>,
}

fn average_confidence(snapshot: &TaskSnapshotV2) -> f64 {
    if snapshot.confidence_by_field.is_empty() {
        0.0
    } else {
        snapshot.confidence_by_field.values().sum::<f64>()
            / snapshot.confidence_by_field.len() as f64
    }
}

fn score(snapshot: &TaskSnapshotV2, newest_at_ms: i64) -> SnapshotSelectionScoreV2 {
    let age_ms = newest_at_ms.saturating_sub(snapshot.observed_at_ms).max(0) as f64;
    let temporal_continuity = (1.0 - age_ms / (45.0 * 60.0 * 1000.0)).clamp(0.0, 1.0);
    let causal_user_action = snapshot
        .claim_evidence
        .iter()
        .flat_map(|claim| claim.evidence_refs.iter())
        .any(|evidence| evidence.source_kind == "causal_event")
        .then_some(1.0)
        .unwrap_or(0.0);
    let unfinished_state = match snapshot.execution_state.as_str() {
        "active" | "blocked" | "idle_after_progress" => 1.0,
        "suspended" => 0.55,
        "completed" | "superseded" => 0.0,
        _ => 0.25,
    };
    let explicit_correction = matches!(
        snapshot.relation_to_prior.as_str(),
        "correction" | "supersedes"
    )
    .then_some(1.0)
    .unwrap_or(0.0);
    let surface_continuity = snapshot
        .surface_identity_hash
        .as_ref()
        .map(|_| 0.8)
        .unwrap_or(0.25);
    let prior_relation = match snapshot.relation_to_prior.as_str() {
        "continuation" | "clarification" | "correction" => 1.0,
        "new_task" | "supersedes" => 0.7,
        "child_support_step" => 0.15,
        _ => 0.25,
    };
    let confidence = average_confidence(snapshot);
    let contradiction_penalty = (snapshot.contradictions.len() as f64 * 0.12).min(0.6);
    let total = (temporal_continuity * 0.17
        + causal_user_action * 0.2
        + unfinished_state * 0.2
        + explicit_correction * 0.12
        + surface_continuity * 0.1
        + prior_relation * 0.09
        + confidence * 0.12
        - contradiction_penalty)
        .clamp(0.0, 1.0);
    let mut reason_codes = vec!["task_evidence_only".into()];
    if snapshot.relation_to_prior == "child_support_step" {
        reason_codes.push("support_activity_cannot_silently_supersede_primary".into());
    }
    if matches!(
        snapshot.execution_state.as_str(),
        "completed" | "superseded"
    ) {
        reason_codes.push("finished_snapshot_deprioritized".into());
    }
    SnapshotSelectionScoreV2 {
        snapshot_id: snapshot.snapshot_id.clone(),
        temporal_continuity,
        causal_user_action,
        unfinished_state,
        explicit_correction,
        surface_continuity,
        prior_relation,
        confidence,
        contradiction_penalty,
        total,
        reason_codes,
    }
}

pub(crate) fn select_snapshot(snapshots: &[TaskSnapshotV2]) -> SnapshotSelectionResultV2 {
    let newest_at_ms = snapshots
        .iter()
        .map(|snapshot| snapshot.observed_at_ms)
        .max()
        .unwrap_or(0);
    let mut scores = snapshots
        .iter()
        .filter(|snapshot| snapshot.selection_status != SnapshotSelectionStatusV2::Unresolved)
        .map(|snapshot| score(snapshot, newest_at_ms))
        .collect::<Vec<_>>();
    scores.sort_by(|left, right| {
        right
            .total
            .partial_cmp(&left.total)
            .unwrap_or(Ordering::Equal)
            .then_with(|| left.snapshot_id.cmp(&right.snapshot_id))
    });
    let selected_snapshot_id = scores
        .first()
        .filter(|score| score.total >= 0.35)
        .map(|score| score.snapshot_id.clone());
    SnapshotSelectionResultV2 {
        policy_version: SNAPSHOT_SELECTION_POLICY_V1.into(),
        unresolved: selected_snapshot_id.is_none(),
        selected_snapshot_id,
        scores,
        excluded_feature_families: vec![
            "url_existence".into(),
            "path_existence".into(),
            "openability".into(),
            "artifact_richness".into(),
            "legacy_candidate_score".into(),
        ],
    }
}
