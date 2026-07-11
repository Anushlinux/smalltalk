use super::activity_recap::{
    sanitize_public_text, ActivityConfidence, ActivityCurrentState, ActivityEvidenceAnchorType,
    ActivityEvidenceConfidence, ActivityEvidenceSource, ActivityEvidenceSpan,
    ActivityRecapValidationStatus, ContinueActivityRecap,
};
use super::activity_recap_inputs::ActivityRecapInputs;
use super::activity_recap_segments::StitchedActivityTimeline;
use super::confidence::{ConfidenceClaim, ContinueConfidenceVector};
use super::semantic_consistency::{
    ConsistencyAgreementStatus, CrossLayerConsistencyResult, DirectTargetPolicyResult,
};
use super::stable_hash;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) const ACTIVITY_RECAP_TASK_TRUTH_SCHEMA: &str = "smalltalk.activity_recap_task_truth.v1";
pub(crate) const ACTIVITY_RECAP_VALIDATOR_POLICY_VERSION: &str = "task_truth_recap_validation.v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ActivityRecapTaskIdentity {
    pub task_turn_id: String,
    pub task_turn_revision: i64,
    pub task_identity_key: String,
    pub bounded_semantic_label: Option<String>,
    pub execution_state: String,
    pub current_actor: String,
    pub waiting_on: String,
    pub relation_to_prior: String,
    pub workstream_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ActivityRecapTaskTruth {
    pub schema: String,
    pub validator_policy_version: String,
    pub identity: ActivityRecapTaskIdentity,
    pub latest_user_goal: Option<String>,
    pub task_object: Option<String>,
    pub prior_task_turn_id: Option<String>,
    pub prior_context_role: Option<String>,
    pub consistency_status: String,
    pub consistency_policy_version: String,
    pub selected_primary_segment_id: Option<String>,
    pub selected_open_loop_id: Option<String>,
    pub direct_target_allowed: bool,
    pub direct_target_policy_version: String,
    pub direct_target_locator_kind: String,
    pub claim_confidence_caps: BTreeMap<String, f64>,
    pub claim_evidence_handles: BTreeMap<String, Vec<String>>,
    pub missing_evidence: Vec<String>,
    pub allowed_context_roles: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ActivityRecapLocalGuard {
    pub forbidden_primary_terms: Vec<String>,
    pub rejected_source_hashes: Vec<String>,
    pub rejection_reason_codes: Vec<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct ActivityRecapTruthBundle {
    pub model_truth: ActivityRecapTaskTruth,
    pub local_guard: ActivityRecapLocalGuard,
}

pub(crate) fn build_activity_recap_truth_bundle(
    inputs: &ActivityRecapInputs,
    timeline: &StitchedActivityTimeline,
    consistency: &CrossLayerConsistencyResult,
    confidence: &ContinueConfidenceVector,
    direct_target: &DirectTargetPolicyResult,
) -> Option<ActivityRecapTruthBundle> {
    let task = inputs.current_task_turn.as_ref()?;
    let semantic_label = task
        .task_object
        .clone()
        .or_else(|| task.latest_user_goal_summary.clone())
        .and_then(|value| sanitize_public_text(value, 160));
    let identity_seed = serde_json::to_vec(&(
        &task.task_turn_id,
        &semantic_label,
        &task.execution_state,
        &task.current_actor,
        &task.waiting_on,
        &task.workstream_id,
    ))
    .unwrap_or_default();
    let identity = ActivityRecapTaskIdentity {
        task_turn_id: task.task_turn_id.clone(),
        task_turn_revision: task.revision,
        task_identity_key: stable_hash(&identity_seed),
        bounded_semantic_label: semantic_label,
        execution_state: task.execution_state.clone(),
        current_actor: task.current_actor.clone(),
        waiting_on: task.waiting_on.clone(),
        relation_to_prior: task.relation_to_prior.clone(),
        workstream_id: task.workstream_id.clone(),
    };
    let mut evidence = task.evidence_ids.clone();
    evidence.sort();
    evidence.dedup();
    let handles = evidence
        .iter()
        .enumerate()
        .map(|(index, _)| format!("t{}", index + 1))
        .collect::<Vec<_>>();
    let mut claim_evidence_handles = BTreeMap::new();
    for claim in [
        "primary_work_summary",
        "primary_where_summary",
        "last_meaningful_state",
        "unfinished_state",
        "next_action_summary",
    ] {
        claim_evidence_handles.insert(claim.to_string(), handles.clone());
    }
    if !direct_target.supporting_evidence_ids.is_empty() {
        let target_handles = direct_target
            .supporting_evidence_ids
            .iter()
            .enumerate()
            .map(|(index, _)| format!("p{}", index + 1))
            .collect::<Vec<_>>();
        claim_evidence_handles.insert("why_this_target".to_string(), target_handles.clone());
        claim_evidence_handles.insert("why_no_safe_target".to_string(), target_handles);
    } else {
        claim_evidence_handles.insert("why_no_safe_target".to_string(), handles.clone());
    }
    let mut claim_confidence_caps = BTreeMap::new();
    for (key, claim) in [
        ("primary_work_summary", ConfidenceClaim::CurrentTask),
        ("primary_where_summary", ConfidenceClaim::SurfaceIdentity),
        ("last_meaningful_state", ConfidenceClaim::ActorActivity),
        ("unfinished_state", ConfidenceClaim::ActorActivity),
        ("next_action_summary", ConfidenceClaim::NextAction),
        ("why_this_target", ConfidenceClaim::DirectTarget),
        ("why_no_safe_target", ConfidenceClaim::CurrentTask),
    ] {
        claim_confidence_caps.insert(key.to_string(), confidence.claim(claim).score);
    }

    let mut forbidden = BTreeSet::new();
    let mut rejected_source_hashes = BTreeSet::new();
    let mut rejection_reason_codes = BTreeSet::new();
    for loop_fact in &inputs.open_loops {
        if loop_fact.eligible_for_objective && loop_fact.relation_to_current_task == "same_task" {
            continue;
        }
        collect_forbidden(loop_fact.objective_hint.as_deref(), &mut forbidden);
        rejected_source_hashes.insert(stable_hash(loop_fact.open_loop_id.as_bytes()));
        rejection_reason_codes.extend(loop_fact.eligibility_reason_codes.iter().cloned());
    }
    for branch in &inputs.branch_contexts {
        if branch.task_turn_id.as_deref() == Some(task.task_turn_id.as_str())
            && branch.promotion_state.starts_with("promoted_")
            && branch.feedback_rejection_reasons.is_empty()
        {
            continue;
        }
        collect_forbidden(branch.promotion_reason.as_deref(), &mut forbidden);
        collect_forbidden(Some(&branch.branch_kind), &mut forbidden);
        rejected_source_hashes.insert(stable_hash(branch.branch_artifact_id.as_bytes()));
        rejection_reason_codes.extend(branch.feedback_rejection_reasons.iter().cloned());
    }
    for memory in &inputs.memory_facts {
        if memory.relation != "support" || memory.feedback_score < 0.0 {
            collect_forbidden(memory.summary.as_deref(), &mut forbidden);
            rejected_source_hashes.insert(stable_hash(memory.memory_id.as_bytes()));
            rejection_reason_codes.insert("memory_not_eligible_for_current_task".to_string());
        }
    }
    if let Some(primary) = timeline.primary_segment.as_ref() {
        if primary.workstream_id.as_deref() != task.workstream_id.as_deref()
            && !matches!(
                primary.role,
                super::activity_recap_segments::ActivitySegmentRole::Support
                    | super::activity_recap_segments::ActivitySegmentRole::Detour
                    | super::activity_recap_segments::ActivitySegmentRole::Return
            )
        {
            collect_forbidden(primary.surface_title.as_deref(), &mut forbidden);
            rejected_source_hashes.insert(stable_hash(primary.segment_id.as_bytes()));
            rejection_reason_codes.insert("primary_segment_not_current_task".to_string());
        }
    }
    let mut missing_evidence = task.missing_evidence.clone();
    missing_evidence.extend(consistency.missing_evidence.iter().cloned());
    missing_evidence.sort();
    missing_evidence.dedup();
    Some(ActivityRecapTruthBundle {
        model_truth: ActivityRecapTaskTruth {
            schema: ACTIVITY_RECAP_TASK_TRUTH_SCHEMA.to_string(),
            validator_policy_version: ACTIVITY_RECAP_VALIDATOR_POLICY_VERSION.to_string(),
            identity,
            latest_user_goal: task.latest_user_goal_summary.clone(),
            task_object: task.task_object.clone(),
            prior_task_turn_id: task.prior_task_turn_id.clone(),
            prior_context_role: task.prior_task_turn_id.as_ref().map(|_| {
                if task.relation_to_prior == "supersedes" {
                    "superseded".to_string()
                } else {
                    "same_workstream_prior_turn".to_string()
                }
            }),
            consistency_status: consistency_status(consistency.agreement_status).to_string(),
            consistency_policy_version: consistency.policy_version.clone(),
            selected_primary_segment_id: consistency.primary_segment_id.clone(),
            selected_open_loop_id: consistency.selected_open_loop_id.clone(),
            direct_target_allowed: direct_target.direct_target_allowed,
            direct_target_policy_version: direct_target.policy_version.clone(),
            direct_target_locator_kind: direct_target.locator_kind.clone(),
            claim_confidence_caps,
            claim_evidence_handles,
            missing_evidence,
            allowed_context_roles: vec![
                "same_task".to_string(),
                "prior_completed".to_string(),
                "child_support".to_string(),
                "detour".to_string(),
                "returned_support".to_string(),
            ],
        },
        local_guard: ActivityRecapLocalGuard {
            forbidden_primary_terms: forbidden.into_iter().collect(),
            rejected_source_hashes: rejected_source_hashes.into_iter().collect(),
            rejection_reason_codes: rejection_reason_codes.into_iter().collect(),
        },
    })
}

pub(crate) fn rebase_local_recap_on_task_truth(
    mut recap: ContinueActivityRecap,
    truth: &ActivityRecapTaskTruth,
    inputs: &ActivityRecapInputs,
) -> ContinueActivityRecap {
    let task_evidence = inputs
        .current_task_turn
        .as_ref()
        .map(|task| task.evidence_ids.clone())
        .unwrap_or_default();
    let summary = truth
        .latest_user_goal
        .clone()
        .or_else(|| truth.task_object.clone())
        .and_then(|value| sanitize_public_text(value, 280));
    if let Some(summary) = summary {
        recap.primary_work_summary = Some(summary.clone());
        recap.primary_work_label = truth.identity.bounded_semantic_label.clone();
        replace_claim_span(
            &mut recap,
            "primary_work_summary",
            summary,
            task_evidence.clone(),
            confidence_for_cap(cap(truth, "primary_work_summary")),
        );
    }
    if let Some(surface) = inputs.current_surface.as_ref() {
        recap.primary_where_summary = surface
            .display_title
            .clone()
            .or_else(|| surface.app_name.clone())
            .and_then(|value| sanitize_public_text(value, 180));
        if let Some(where_summary) = recap.primary_where_summary.clone() {
            replace_claim_span(
                &mut recap,
                "primary_where_summary",
                where_summary,
                surface.evidence_ids.clone(),
                confidence_for_cap(cap(truth, "primary_where_summary")),
            );
        }
    }
    recap.current_state = current_state(&truth.identity.execution_state);
    recap.last_meaningful_state = state_summary(truth);
    recap.unfinished_state = unfinished_summary(truth);
    recap.next_action_summary = next_action_summary(truth);
    for (key, value) in [
        ("last_meaningful_state", recap.last_meaningful_state.clone()),
        ("unfinished_state", recap.unfinished_state.clone()),
        ("next_action_summary", recap.next_action_summary.clone()),
    ] {
        if let Some(value) = value {
            replace_claim_span(
                &mut recap,
                key,
                value,
                task_evidence.clone(),
                confidence_for_cap(cap(truth, key)),
            );
        }
    }
    if truth.direct_target_allowed {
        recap.why_no_safe_target = None;
    } else {
        recap.why_this_target = None;
        let explanation =
            "The task is clear, but there is no verified URL or document path to reopen directly."
                .to_string();
        recap.why_no_safe_target = Some(explanation.clone());
        replace_claim_span(
            &mut recap,
            "why_no_safe_target",
            explanation,
            task_evidence,
            confidence_for_cap(cap(truth, "why_no_safe_target")),
        );
    }
    recap.activity_confidence = activity_confidence(cap(truth, "primary_work_summary"));
    recap.target_confidence = activity_confidence(cap(truth, "why_this_target"));
    recap.validation_status = if truth.consistency_status == "conflicting" {
        recap
            .warnings
            .push("activity_recap:semantic_consistency_conflict".to_string());
        ActivityRecapValidationStatus::Thin
    } else if recap.primary_work_summary.is_some() && cap(truth, "primary_work_summary") >= 0.5 {
        ActivityRecapValidationStatus::Valid
    } else {
        ActivityRecapValidationStatus::Thin
    };
    for missing in &truth.missing_evidence {
        if !recap.missing_evidence.contains(missing) {
            recap.missing_evidence.push(missing.clone());
        }
    }
    recap.sanitized()
}

fn consistency_status(status: ConsistencyAgreementStatus) -> &'static str {
    match status {
        ConsistencyAgreementStatus::Consistent => "consistent",
        ConsistencyAgreementStatus::ConsistentWithSupportRelation => {
            "consistent_with_support_relation"
        }
        ConsistencyAgreementStatus::ThinButNonConflicting => "thin_but_non_conflicting",
        ConsistencyAgreementStatus::Conflicting => "conflicting",
        ConsistencyAgreementStatus::Unresolved => "unresolved",
    }
}

fn collect_forbidden(value: Option<&str>, output: &mut BTreeSet<String>) {
    let Some(value) = value else {
        return;
    };
    for token in super::activity_recap_model::normalized_tokens(value) {
        if token.len() > 2 {
            output.insert(token);
        }
    }
}

fn cap(truth: &ActivityRecapTaskTruth, key: &str) -> f64 {
    truth.claim_confidence_caps.get(key).copied().unwrap_or(0.0)
}

fn activity_confidence(score: f64) -> ActivityConfidence {
    if score >= 0.75 {
        ActivityConfidence::High
    } else if score >= 0.5 {
        ActivityConfidence::Medium
    } else if score > 0.0 {
        ActivityConfidence::Low
    } else {
        ActivityConfidence::None
    }
}

fn confidence_for_cap(score: f64) -> ActivityEvidenceConfidence {
    if score >= 0.75 {
        ActivityEvidenceConfidence::High
    } else if score >= 0.5 {
        ActivityEvidenceConfidence::Medium
    } else {
        ActivityEvidenceConfidence::Low
    }
}

fn current_state(state: &str) -> ActivityCurrentState {
    match state {
        "active" => ActivityCurrentState::ActivelyWorking,
        "blocked" => ActivityCurrentState::Blocked,
        "completed" | "superseded" => ActivityCurrentState::CompleteOrIdle,
        "idle_after_progress" | "suspended" => ActivityCurrentState::PausedAfterProgress,
        _ => ActivityCurrentState::Unclear,
    }
}

fn state_summary(truth: &ActivityRecapTaskTruth) -> Option<String> {
    let object = truth
        .identity
        .bounded_semantic_label
        .as_deref()
        .unwrap_or("the current task");
    let text = match (
        truth.identity.execution_state.as_str(),
        truth.identity.current_actor.as_str(),
    ) {
        ("completed", _) => format!("The current task, {object}, is complete."),
        ("blocked", _) => format!("The current task, {object}, is blocked."),
        ("active", "assistant_or_agent") => format!("The agent is working on {object}."),
        ("active", "user") => format!("You were actively working on {object}."),
        ("active", _) => format!("Work is active on {object}."),
        _ => format!("The latest known state concerns {object}."),
    };
    sanitize_public_text(text, 240)
}

fn unfinished_summary(truth: &ActivityRecapTaskTruth) -> Option<String> {
    if matches!(
        truth.identity.execution_state.as_str(),
        "completed" | "superseded"
    ) {
        None
    } else {
        truth
            .identity
            .bounded_semantic_label
            .as_ref()
            .and_then(|object| {
                sanitize_public_text(format!("{object} is not yet confirmed complete."), 240)
            })
    }
}

fn next_action_summary(truth: &ActivityRecapTaskTruth) -> Option<String> {
    let text = match truth.identity.waiting_on.as_str() {
        "agent" => "Wait for the agent's current work, then review the result.",
        "user" => "Provide the input needed to continue the current task.",
        "external" => "Check whether the external dependency is ready before continuing.",
        _ if truth.identity.execution_state == "active" => {
            "Continue the current task from its latest supported state."
        }
        _ => return None,
    };
    sanitize_public_text(text.to_string(), 240)
}

fn replace_claim_span(
    recap: &mut ContinueActivityRecap,
    key: &str,
    text: String,
    anchor_ids: Vec<String>,
    confidence: ActivityEvidenceConfidence,
) {
    recap.evidence_spans.retain(|span| span.claim_key != key);
    recap.evidence_spans.push(ActivityEvidenceSpan {
        claim_key: key.to_string(),
        claim_text: text,
        anchor_type: ActivityEvidenceAnchorType::Action,
        anchor_ids,
        confidence,
        source: ActivityEvidenceSource::Local,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::continuation::activity_recap_inputs::{
        ActivityRecapDecisionContext, CurrentTaskTurnFact, ExistingQualityFacts,
    };
    use crate::continuation::semantic_consistency::CrossLayerConsistencyResult;

    fn inputs() -> ActivityRecapInputs {
        ActivityRecapInputs {
            schema: "smalltalk.activity_recap_inputs.v2".to_string(),
            decision_context: ActivityRecapDecisionContext {
                decision_id_seed: None,
                mode: "normal".to_string(),
                lookback_ms: 1,
                evidence_watermark: None,
                output_mode: None,
            },
            current_task_turn: Some(CurrentTaskTurnFact {
                task_turn_id: "turn-capture".to_string(),
                revision: 2,
                workstream_id: Some("smalltalk".to_string()),
                parent_task_turn_id: None,
                prior_task_turn_id: Some("turn-prior".to_string()),
                supersedes_task_turn_id: Some("turn-prior".to_string()),
                latest_user_goal_summary: Some(
                    "Investigate what the island Capture button does".to_string(),
                ),
                task_object: Some("Capture button".to_string()),
                task_kind: "investigate".to_string(),
                execution_state: "active".to_string(),
                current_actor: "assistant_or_agent".to_string(),
                waiting_on: "agent".to_string(),
                relation_to_prior: "new_task".to_string(),
                started_at_ms: 10,
                last_observed_at_ms: 20,
                goal_confidence: 0.9,
                task_object_confidence: 0.9,
                actor_state_confidence: 0.9,
                execution_state_confidence: 0.9,
                waiting_on_confidence: 0.9,
                relation_confidence: 0.9,
                attribution_confidence: 0.9,
                task_claim_confidence: 0.9,
                state_claim_confidence: 0.9,
                missing_evidence: vec![],
                evidence_ids: vec!["span-current".to_string()],
                reason_codes: vec![],
            }),
            current_surface: None,
            selected_workstream: None,
            selected_candidate: None,
            return_target: None,
            resume_work_target: None,
            recent_segments: vec![],
            recent_actions: vec![],
            recent_moments: vec![],
            open_loops: vec![],
            workstream_states: vec![],
            branch_contexts: vec![],
            surface_snapshots: vec![],
            support_evidence: vec![],
            memory_facts: vec![],
            existing_quality: ExistingQualityFacts {
                p0_quality_signals: None,
                current_surface_resolution: None,
                evidence_freshness_ledger: None,
                app_activity_summary: None,
                quality_gate: None,
            },
            input_warnings: vec![],
        }
    }

    #[test]
    fn local_recap_uses_current_task_and_null_target_honestly() {
        let mut consistency = CrossLayerConsistencyResult::default();
        consistency.current_task_turn_id = Some("turn-capture".to_string());
        let truth = build_activity_recap_truth_bundle(
            &inputs(),
            &StitchedActivityTimeline::default(),
            &consistency,
            &ContinueConfidenceVector::default(),
            &DirectTargetPolicyResult::default(),
        )
        .unwrap();
        let recap = rebase_local_recap_on_task_truth(
            ContinueActivityRecap::default(),
            &truth.model_truth,
            &inputs(),
        );
        assert!(recap
            .primary_work_summary
            .as_deref()
            .unwrap()
            .contains("Capture button"));
        assert!(recap.why_this_target.is_none());
        assert!(recap.why_no_safe_target.is_some());
    }
}
