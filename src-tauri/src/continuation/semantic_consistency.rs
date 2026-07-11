use serde::{Deserialize, Serialize};

pub(crate) const SEMANTIC_GRAPH_POLICY_VERSION: &str = "workstream_open_loop_consistency.v1";
pub(crate) const SEMANTIC_GRAPH_POLICY_FINGERPRINT: &str =
    "workstream_open_loop_consistency.v1:task_first:relation_before_quality:decision_scoped_repair";
pub(crate) const CROSS_LAYER_CONSISTENCY_SCHEMA: &str = "smalltalk.cross_layer_consistency.v1";
pub(crate) const DIRECT_TARGET_POLICY_SCHEMA: &str = "smalltalk.direct_target_policy.v1";
pub(crate) const DIRECT_TARGET_POLICY_VERSION: &str = "direct_target_url_or_document_path.v1";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RelationToCurrentTask {
    SameTask,
    SameWorkstreamPriorTurn,
    ChildSupport,
    Detour,
    Interruption,
    ReturnedSupport,
    SupersededTask,
    Unrelated,
    Unknown,
}

impl RelationToCurrentTask {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::SameTask => "same_task",
            Self::SameWorkstreamPriorTurn => "same_workstream_prior_turn",
            Self::ChildSupport => "child_support",
            Self::Detour => "detour",
            Self::Interruption => "interruption",
            Self::ReturnedSupport => "returned_support",
            Self::SupersededTask => "superseded_task",
            Self::Unrelated => "unrelated",
            Self::Unknown => "unknown",
        }
    }

    pub(crate) fn supports_current_task(self) -> bool {
        matches!(
            self,
            Self::SameTask | Self::ChildSupport | Self::Detour | Self::ReturnedSupport
        )
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SemanticFreshness {
    CurrentTurn,
    RecentPriorTurn,
    Historical,
    Unknown,
}

impl SemanticFreshness {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::CurrentTurn => "current_turn",
            Self::RecentPriorTurn => "recent_prior_turn",
            Self::Historical => "historical",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct SemanticEligibilityInput {
    pub entity_kind: String,
    pub entity_id: String,
    pub current_task_turn_id: Option<String>,
    pub current_workstream_id: Option<String>,
    pub owner_task_turn_id: Option<String>,
    pub owner_workstream_id: Option<String>,
    pub parent_task_turn_id: Option<String>,
    pub origin_task_turn_id: Option<String>,
    pub relation_hint: Option<RelationToCurrentTask>,
    pub lifecycle_state: Option<String>,
    pub current_task_started_at_ms: Option<i64>,
    pub entity_updated_at_ms: Option<i64>,
    pub supporting_evidence_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct SemanticEligibilityDecision {
    pub schema: String,
    pub policy_version: String,
    pub entity_kind: String,
    pub entity_id: String,
    pub current_task_turn_id: Option<String>,
    pub current_workstream_id: Option<String>,
    pub owner_task_turn_id: Option<String>,
    pub owner_workstream_id: Option<String>,
    pub parent_task_turn_id: Option<String>,
    pub origin_task_turn_id: Option<String>,
    pub relation_to_current_task: RelationToCurrentTask,
    pub freshness: SemanticFreshness,
    pub eligible_for_primary: bool,
    pub eligible_for_objective: bool,
    pub eligible_for_last_state: bool,
    pub eligible_as_support_evidence: bool,
    pub reason_codes: Vec<String>,
    pub supporting_evidence_ids: Vec<String>,
}

pub(crate) fn evaluate_semantic_eligibility(
    input: &SemanticEligibilityInput,
) -> SemanticEligibilityDecision {
    let relation = input.relation_hint.unwrap_or_else(|| relation_for(input));
    let freshness = freshness_for(input, relation);
    let terminal = input
        .lifecycle_state
        .as_deref()
        .is_some_and(|state| matches!(state, "completed" | "superseded" | "closed" | "resolved"));
    let is_workstream = input.entity_kind == "workstream";
    let same_selected_workstream = input.owner_workstream_id.is_some()
        && input.owner_workstream_id == input.current_workstream_id;
    let mut eligible_for_primary = false;
    let mut eligible_for_objective = false;
    let mut eligible_for_last_state = false;
    let mut eligible_as_support_evidence = false;
    let mut reasons = Vec::new();

    match relation {
        RelationToCurrentTask::SameTask => {
            eligible_for_primary = !terminal;
            eligible_for_objective = !terminal;
            eligible_for_last_state = true;
            eligible_as_support_evidence = true;
            reasons.push("owned_by_current_task_turn".to_string());
        }
        RelationToCurrentTask::SameWorkstreamPriorTurn => {
            eligible_for_primary = is_workstream && same_selected_workstream;
            eligible_as_support_evidence = true;
            reasons.push("same_workstream_prior_turn_history_only".to_string());
        }
        RelationToCurrentTask::ChildSupport
        | RelationToCurrentTask::Detour
        | RelationToCurrentTask::ReturnedSupport => {
            eligible_as_support_evidence = true;
            reasons.push(format!("{}_evidence_only", relation.as_str()));
        }
        RelationToCurrentTask::Interruption => {
            reasons.push("interruption_not_current_task_authority".to_string());
        }
        RelationToCurrentTask::SupersededTask => {
            eligible_as_support_evidence = true;
            reasons.push("superseded_task_history_only".to_string());
        }
        RelationToCurrentTask::Unrelated => {
            reasons.push("unrelated_to_current_task".to_string());
        }
        RelationToCurrentTask::Unknown => {
            reasons.push("task_relation_unknown".to_string());
        }
    }

    if terminal {
        eligible_for_primary = relation == RelationToCurrentTask::SameTask;
        eligible_for_objective = false;
        if relation != RelationToCurrentTask::SameTask {
            eligible_for_last_state = false;
        }
        reasons.push(format!(
            "lifecycle_{}_not_current_objective",
            input.lifecycle_state.as_deref().unwrap_or("terminal")
        ));
    }
    if matches!(freshness, SemanticFreshness::Historical)
        && relation != RelationToCurrentTask::SameTask
    {
        eligible_for_primary = false;
        eligible_for_objective = false;
        eligible_for_last_state = false;
        reasons.push("outside_current_turn_freshness_policy".to_string());
    }
    reasons.sort();
    reasons.dedup();

    SemanticEligibilityDecision {
        schema: "smalltalk.semantic_eligibility.v1".to_string(),
        policy_version: SEMANTIC_GRAPH_POLICY_VERSION.to_string(),
        entity_kind: input.entity_kind.clone(),
        entity_id: input.entity_id.clone(),
        current_task_turn_id: input.current_task_turn_id.clone(),
        current_workstream_id: input.current_workstream_id.clone(),
        owner_task_turn_id: input.owner_task_turn_id.clone(),
        owner_workstream_id: input.owner_workstream_id.clone(),
        parent_task_turn_id: input.parent_task_turn_id.clone(),
        origin_task_turn_id: input.origin_task_turn_id.clone(),
        relation_to_current_task: relation,
        freshness,
        eligible_for_primary,
        eligible_for_objective,
        eligible_for_last_state,
        eligible_as_support_evidence,
        reason_codes: reasons,
        supporting_evidence_ids: input.supporting_evidence_ids.clone(),
    }
}

fn relation_for(input: &SemanticEligibilityInput) -> RelationToCurrentTask {
    let Some(current_task_turn_id) = input.current_task_turn_id.as_deref() else {
        return RelationToCurrentTask::Unknown;
    };
    if input.owner_task_turn_id.as_deref() == Some(current_task_turn_id) {
        return RelationToCurrentTask::SameTask;
    }
    if input.parent_task_turn_id.as_deref() == Some(current_task_turn_id)
        || input.origin_task_turn_id.as_deref() == Some(current_task_turn_id)
    {
        return RelationToCurrentTask::ChildSupport;
    }
    if input
        .lifecycle_state
        .as_deref()
        .is_some_and(|state| matches!(state, "superseded" | "completed" | "closed" | "resolved"))
        && input.owner_task_turn_id.is_some()
    {
        return RelationToCurrentTask::SupersededTask;
    }
    match (
        input.current_workstream_id.as_deref(),
        input.owner_workstream_id.as_deref(),
    ) {
        (Some(current), Some(owner)) if current == owner => {
            RelationToCurrentTask::SameWorkstreamPriorTurn
        }
        (Some(_), Some(_)) => RelationToCurrentTask::Unrelated,
        _ => RelationToCurrentTask::Unknown,
    }
}

fn freshness_for(
    input: &SemanticEligibilityInput,
    relation: RelationToCurrentTask,
) -> SemanticFreshness {
    if relation == RelationToCurrentTask::SameTask {
        return SemanticFreshness::CurrentTurn;
    }
    match (input.current_task_started_at_ms, input.entity_updated_at_ms) {
        (Some(started), Some(updated)) if updated >= started => SemanticFreshness::RecentPriorTurn,
        (Some(started), Some(updated)) if started.saturating_sub(updated) <= 45 * 60 * 1_000 => {
            SemanticFreshness::RecentPriorTurn
        }
        (Some(_), Some(_)) => SemanticFreshness::Historical,
        _ => SemanticFreshness::Unknown,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct DirectTargetPolicyInput {
    pub candidate_id: Option<String>,
    pub artifact_id: Option<String>,
    pub task_turn_eligible: bool,
    pub workstream_eligible: bool,
    pub feedback_eligible: bool,
    pub branch_eligible: bool,
    pub freshness_eligible: bool,
    pub target_identity_confidence: Option<f64>,
    pub openability: Option<String>,
    pub browser_url: Option<String>,
    pub document_path: Option<String>,
    pub evidence_preview_id: Option<String>,
    pub supporting_evidence_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DirectTargetPolicyResult {
    pub schema: String,
    pub policy_version: String,
    pub candidate_id: Option<String>,
    pub artifact_id: Option<String>,
    pub task_turn_eligible: bool,
    pub workstream_eligible: bool,
    pub feedback_eligible: bool,
    pub branch_eligible: bool,
    pub freshness_eligible: bool,
    #[serde(default)]
    pub target_identity_confidence: Option<f64>,
    #[serde(default)]
    pub target_identity_confident: bool,
    pub openable: bool,
    pub locator_kind: String,
    pub validated_direct_locator_present: bool,
    pub evidence_preview_available: bool,
    pub direct_target_allowed: bool,
    pub reason_codes: Vec<String>,
    pub supporting_evidence_ids: Vec<String>,
}

impl Default for DirectTargetPolicyResult {
    fn default() -> Self {
        evaluate_direct_target_policy(&DirectTargetPolicyInput {
            candidate_id: None,
            artifact_id: None,
            task_turn_eligible: false,
            workstream_eligible: false,
            feedback_eligible: false,
            branch_eligible: false,
            freshness_eligible: false,
            target_identity_confidence: None,
            openability: None,
            browser_url: None,
            document_path: None,
            evidence_preview_id: None,
            supporting_evidence_ids: Vec::new(),
        })
    }
}

pub(crate) fn evaluate_direct_target_policy(
    input: &DirectTargetPolicyInput,
) -> DirectTargetPolicyResult {
    let valid_url = input.browser_url.as_deref().is_some_and(valid_browser_url);
    let valid_path = input
        .document_path
        .as_deref()
        .is_some_and(valid_document_path);
    let locator_kind = if valid_url {
        "browser_url"
    } else if valid_path {
        "document_path"
    } else if input.evidence_preview_id.is_some() {
        "evidence_preview"
    } else {
        "none"
    };
    let validated_direct_locator_present = valid_url || valid_path;
    let target_identity_confident = input
        .target_identity_confidence
        .is_some_and(|confidence| confidence >= 0.55);
    let openable =
        validated_direct_locator_present && input.openability.as_deref() == Some("openable");
    let mut reasons = Vec::new();
    for (eligible, reason) in [
        (input.task_turn_eligible, "task_turn_ineligible"),
        (input.workstream_eligible, "workstream_ineligible"),
        (input.feedback_eligible, "feedback_ineligible"),
        (input.branch_eligible, "branch_ineligible"),
        (input.freshness_eligible, "freshness_ineligible"),
    ] {
        if !eligible {
            reasons.push(reason.to_string());
        }
    }
    if !validated_direct_locator_present {
        reasons.push("no_validated_direct_locator".to_string());
    }
    if !target_identity_confident {
        reasons.push("target_identity_thin".to_string());
    }
    if input.evidence_preview_id.is_some() && !validated_direct_locator_present {
        reasons.push("evidence_preview_is_not_direct_locator".to_string());
    }
    if input.openability.as_deref() == Some("openable") && !validated_direct_locator_present {
        reasons.push("openability_label_without_locator".to_string());
    }
    if validated_direct_locator_present && !openable {
        reasons.push("locator_not_openable_under_strict_policy".to_string());
    }
    reasons.sort();
    reasons.dedup();
    let direct_target_allowed = input.task_turn_eligible
        && input.workstream_eligible
        && input.feedback_eligible
        && input.branch_eligible
        && input.freshness_eligible
        && target_identity_confident
        && openable;

    DirectTargetPolicyResult {
        schema: DIRECT_TARGET_POLICY_SCHEMA.to_string(),
        policy_version: DIRECT_TARGET_POLICY_VERSION.to_string(),
        candidate_id: input.candidate_id.clone(),
        artifact_id: input.artifact_id.clone(),
        task_turn_eligible: input.task_turn_eligible,
        workstream_eligible: input.workstream_eligible,
        feedback_eligible: input.feedback_eligible,
        branch_eligible: input.branch_eligible,
        freshness_eligible: input.freshness_eligible,
        target_identity_confidence: input.target_identity_confidence,
        target_identity_confident,
        openable,
        locator_kind: locator_kind.to_string(),
        validated_direct_locator_present,
        evidence_preview_available: input.evidence_preview_id.is_some(),
        direct_target_allowed,
        reason_codes: reasons,
        supporting_evidence_ids: input.supporting_evidence_ids.clone(),
    }
}

fn valid_browser_url(value: &str) -> bool {
    let value = value.trim();
    !value.chars().any(char::is_whitespace)
        && (value.starts_with("https://") || value.starts_with("http://"))
        && value
            .split_once("://")
            .is_some_and(|(_, rest)| !rest.is_empty() && !rest.starts_with('/'))
}

fn valid_document_path(value: &str) -> bool {
    let value = value.trim();
    value.starts_with('/')
        && value.len() > 1
        && !value.contains('\0')
        && !value.to_ascii_lowercase().contains("continue_outputs")
        && !value.to_ascii_lowercase().contains("resume_query_exports")
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConsistencyAgreementStatus {
    Consistent,
    ConsistentWithSupportRelation,
    ThinButNonConflicting,
    Conflicting,
    Unresolved,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub(crate) struct CrossLayerConsistencyInput {
    pub current_task_turn_id: Option<String>,
    pub current_task_high_confidence: bool,
    pub selected_workstream_id: Option<String>,
    pub current_task_workstream_id: Option<String>,
    pub primary_segment_id: Option<String>,
    pub primary_segment_relation: Option<RelationToCurrentTask>,
    pub selected_open_loop_id: Option<String>,
    pub selected_open_loop_eligible: Option<bool>,
    pub selected_objective_source: Option<String>,
    pub selected_objective_eligible: Option<bool>,
    pub selected_candidate_id: Option<String>,
    pub selected_candidate_workstream_id: Option<String>,
    pub public_target_artifact_id: Option<String>,
    pub public_target_eligible: Option<bool>,
    pub branch_feedback_applicable: Option<bool>,
    pub last_state_from_completed_prior_task: bool,
    pub target_explanation_matches_current_task: Option<bool>,
    pub missing_evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CrossLayerConsistencyResult {
    pub schema: String,
    pub policy_version: String,
    pub current_task_turn_id: Option<String>,
    pub selected_workstream_id: Option<String>,
    pub primary_segment_id: Option<String>,
    pub selected_open_loop_id: Option<String>,
    pub selected_objective_source: Option<String>,
    pub selected_candidate_id: Option<String>,
    pub public_target_artifact_id: Option<String>,
    pub agreement_status: ConsistencyAgreementStatus,
    pub conflicts: Vec<String>,
    pub repairs_or_downgrades: Vec<String>,
    pub missing_evidence: Vec<String>,
}

impl Default for CrossLayerConsistencyResult {
    fn default() -> Self {
        evaluate_cross_layer_consistency(&CrossLayerConsistencyInput::default())
    }
}

pub(crate) fn evaluate_cross_layer_consistency(
    input: &CrossLayerConsistencyInput,
) -> CrossLayerConsistencyResult {
    let mut conflicts = Vec::new();
    let mut repairs = Vec::new();
    if input.current_task_turn_id.is_none() {
        repairs.push("downgrade_no_current_task_turn".to_string());
    }
    if let (Some(selected), Some(current)) = (
        input.selected_workstream_id.as_deref(),
        input.current_task_workstream_id.as_deref(),
    ) {
        if selected != current {
            conflicts.push("selected_workstream_differs_from_current_task".to_string());
            repairs.push("filter_non_current_workstream_projection".to_string());
        }
    }
    if input.primary_segment_relation.is_some_and(|relation| {
        !matches!(
            relation,
            RelationToCurrentTask::SameTask
                | RelationToCurrentTask::ChildSupport
                | RelationToCurrentTask::Detour
                | RelationToCurrentTask::ReturnedSupport
        )
    }) {
        conflicts.push("primary_segment_unrelated_to_current_task".to_string());
        repairs.push("replace_or_remove_primary_segment".to_string());
    }
    if input.selected_open_loop_eligible == Some(false) {
        conflicts.push("objective_open_loop_ineligible_for_current_task".to_string());
        repairs.push("filter_ineligible_open_loop".to_string());
    }
    if input.selected_objective_eligible == Some(false) {
        conflicts.push("objective_source_ineligible_for_current_task".to_string());
        repairs.push("replace_objective_with_current_task_truth".to_string());
    }
    if let (Some(candidate_workstream), Some(current_workstream)) = (
        input.selected_candidate_workstream_id.as_deref(),
        input.current_task_workstream_id.as_deref(),
    ) {
        if candidate_workstream != current_workstream {
            conflicts.push("selected_candidate_wrong_workstream".to_string());
            repairs.push("filter_non_current_candidate".to_string());
        }
    }
    if input.branch_feedback_applicable == Some(false) {
        conflicts.push("branch_promoted_by_inapplicable_feedback".to_string());
        repairs.push("remove_branch_from_current_projection".to_string());
    }
    if input.last_state_from_completed_prior_task {
        conflicts.push("last_state_from_completed_prior_task".to_string());
        repairs.push("remove_prior_completed_last_state".to_string());
    }
    if input.target_explanation_matches_current_task == Some(false) {
        conflicts.push("target_explanation_names_different_task".to_string());
        repairs.push("suppress_mismatched_target_explanation".to_string());
    }
    if input.public_target_eligible == Some(false) && input.public_target_artifact_id.is_some() {
        conflicts.push("public_target_ineligible_for_current_task".to_string());
        repairs.push("suppress_public_target".to_string());
    }
    conflicts.sort();
    conflicts.dedup();
    repairs.sort();
    repairs.dedup();

    let has_support = input.primary_segment_relation.is_some_and(|relation| {
        matches!(
            relation,
            RelationToCurrentTask::ChildSupport
                | RelationToCurrentTask::Detour
                | RelationToCurrentTask::ReturnedSupport
        )
    });
    let agreement_status = if !conflicts.is_empty() {
        ConsistencyAgreementStatus::Conflicting
    } else if input.current_task_turn_id.is_none() {
        ConsistencyAgreementStatus::Unresolved
    } else if input.selected_workstream_id.is_none()
        || (input.primary_segment_id.is_none() && !input.current_task_high_confidence)
    {
        ConsistencyAgreementStatus::ThinButNonConflicting
    } else if has_support {
        ConsistencyAgreementStatus::ConsistentWithSupportRelation
    } else {
        ConsistencyAgreementStatus::Consistent
    };

    CrossLayerConsistencyResult {
        schema: CROSS_LAYER_CONSISTENCY_SCHEMA.to_string(),
        policy_version: SEMANTIC_GRAPH_POLICY_VERSION.to_string(),
        current_task_turn_id: input.current_task_turn_id.clone(),
        selected_workstream_id: input.selected_workstream_id.clone(),
        primary_segment_id: input.primary_segment_id.clone(),
        selected_open_loop_id: input.selected_open_loop_id.clone(),
        selected_objective_source: input.selected_objective_source.clone(),
        selected_candidate_id: input.selected_candidate_id.clone(),
        public_target_artifact_id: input.public_target_artifact_id.clone(),
        agreement_status,
        conflicts,
        repairs_or_downgrades: repairs,
        missing_evidence: input.missing_evidence.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eligibility(
        entity_kind: &str,
        owner_turn: Option<&str>,
        owner_workstream: Option<&str>,
        state: Option<&str>,
    ) -> SemanticEligibilityDecision {
        evaluate_semantic_eligibility(&SemanticEligibilityInput {
            entity_kind: entity_kind.to_string(),
            entity_id: "entity".to_string(),
            current_task_turn_id: Some("turn-current".to_string()),
            current_workstream_id: Some("workstream-smalltalk".to_string()),
            owner_task_turn_id: owner_turn.map(str::to_string),
            owner_workstream_id: owner_workstream.map(str::to_string),
            parent_task_turn_id: None,
            origin_task_turn_id: None,
            relation_hint: None,
            lifecycle_state: state.map(str::to_string),
            current_task_started_at_ms: Some(10_000),
            entity_updated_at_ms: Some(10_100),
            supporting_evidence_ids: vec!["action-1".to_string()],
        })
    }

    #[test]
    fn same_project_new_task_workstream_continuity_does_not_inherit_state() {
        let workstream = eligibility(
            "workstream",
            Some("turn-prior"),
            Some("workstream-smalltalk"),
            Some("active"),
        );
        assert!(workstream.eligible_for_primary);
        assert!(!workstream.eligible_for_objective);
        let prior_loop = eligibility(
            "open_loop",
            Some("turn-prior"),
            Some("workstream-smalltalk"),
            Some("completed"),
        );
        assert!(!prior_loop.eligible_for_primary);
        assert!(!prior_loop.eligible_for_objective);
        assert!(!prior_loop.eligible_for_last_state);
    }

    #[test]
    fn unrelated_old_loop_is_ineligible_before_quality() {
        let result = eligibility(
            "open_loop",
            Some("turn-stremio"),
            Some("workstream-stremio"),
            Some("open"),
        );
        assert_eq!(
            result.relation_to_current_task,
            RelationToCurrentTask::Unrelated
        );
        assert!(!result.eligible_for_objective);
        assert!(result
            .reason_codes
            .contains(&"unrelated_to_current_task".to_string()));
    }

    #[test]
    fn completed_prior_loop_is_history_not_current_state() {
        let result = eligibility(
            "open_loop",
            Some("turn-prior"),
            Some("workstream-smalltalk"),
            Some("completed"),
        );
        assert_eq!(
            result.relation_to_current_task,
            RelationToCurrentTask::SupersededTask
        );
        assert!(!result.eligible_for_last_state);
    }

    #[test]
    fn child_support_can_supply_evidence_but_not_primary_objective() {
        let mut input = SemanticEligibilityInput {
            entity_kind: "open_loop".to_string(),
            entity_id: "support-loop".to_string(),
            current_task_turn_id: Some("turn-parent".to_string()),
            current_workstream_id: Some("workstream-smalltalk".to_string()),
            owner_task_turn_id: Some("turn-docs".to_string()),
            owner_workstream_id: Some("workstream-smalltalk".to_string()),
            parent_task_turn_id: Some("turn-parent".to_string()),
            origin_task_turn_id: Some("turn-parent".to_string()),
            relation_hint: None,
            lifecycle_state: Some("blocked".to_string()),
            current_task_started_at_ms: Some(1_000),
            entity_updated_at_ms: Some(1_100),
            supporting_evidence_ids: vec!["docs-action".to_string()],
        };
        let result = evaluate_semantic_eligibility(&input);
        assert_eq!(
            result.relation_to_current_task,
            RelationToCurrentTask::ChildSupport
        );
        assert!(result.eligible_as_support_evidence);
        assert!(!result.eligible_for_primary);
        assert!(!result.eligible_for_objective);
        input.relation_hint = Some(RelationToCurrentTask::ReturnedSupport);
        assert!(evaluate_semantic_eligibility(&input).eligible_as_support_evidence);
    }

    #[test]
    fn direct_target_policy_rejects_frame_and_label_only_but_keeps_preview() {
        let result = evaluate_direct_target_policy(&DirectTargetPolicyInput {
            candidate_id: Some("candidate".to_string()),
            artifact_id: Some("artifact".to_string()),
            task_turn_eligible: true,
            workstream_eligible: true,
            feedback_eligible: true,
            branch_eligible: true,
            freshness_eligible: true,
            target_identity_confidence: Some(0.9),
            openability: Some("openable".to_string()),
            browser_url: None,
            document_path: None,
            evidence_preview_id: Some("frame-1".to_string()),
            supporting_evidence_ids: vec!["frame-1".to_string()],
        });
        assert!(!result.direct_target_allowed);
        assert!(result.evidence_preview_available);
        assert_eq!(result.locator_kind, "evidence_preview");
        assert!(result
            .reason_codes
            .contains(&"openability_label_without_locator".to_string()));
    }

    #[test]
    fn direct_target_policy_allows_valid_url_and_document_path() {
        for (url, path, kind) in [
            (Some("https://example.test/task"), None, "browser_url"),
            (None, Some("/tmp/example.txt"), "document_path"),
        ] {
            let result = evaluate_direct_target_policy(&DirectTargetPolicyInput {
                candidate_id: Some("candidate".to_string()),
                artifact_id: Some("artifact".to_string()),
                task_turn_eligible: true,
                workstream_eligible: true,
                feedback_eligible: true,
                branch_eligible: true,
                freshness_eligible: true,
                target_identity_confidence: Some(0.9),
                openability: Some("openable".to_string()),
                browser_url: url.map(str::to_string),
                document_path: path.map(str::to_string),
                evidence_preview_id: Some("frame-1".to_string()),
                supporting_evidence_ids: vec![],
            });
            assert!(result.direct_target_allowed);
            assert_eq!(result.locator_kind, kind);
        }
    }

    #[test]
    fn p6_08_direct_target_requires_openability_and_identity_confidence() {
        for (openability, identity_confidence, expected_reason) in [
            (
                "frame_fallback",
                0.9,
                "locator_not_openable_under_strict_policy",
            ),
            ("openable", 0.4, "target_identity_thin"),
        ] {
            let result = evaluate_direct_target_policy(&DirectTargetPolicyInput {
                candidate_id: Some("candidate".to_string()),
                artifact_id: Some("artifact".to_string()),
                task_turn_eligible: true,
                workstream_eligible: true,
                feedback_eligible: true,
                branch_eligible: true,
                freshness_eligible: true,
                target_identity_confidence: Some(identity_confidence),
                openability: Some(openability.to_string()),
                browser_url: Some("https://example.test/task".to_string()),
                document_path: None,
                evidence_preview_id: Some("frame-1".to_string()),
                supporting_evidence_ids: vec!["frame-1".to_string()],
            });
            assert!(!result.direct_target_allowed);
            assert!(result.reason_codes.contains(&expected_reason.to_string()));
        }
    }

    #[test]
    fn consistency_reports_all_stable_hard_conflict_codes() {
        let result = evaluate_cross_layer_consistency(&CrossLayerConsistencyInput {
            current_task_turn_id: Some("turn-current".to_string()),
            current_task_high_confidence: true,
            selected_workstream_id: Some("workstream-old".to_string()),
            current_task_workstream_id: Some("workstream-current".to_string()),
            primary_segment_id: Some("segment-old".to_string()),
            primary_segment_relation: Some(RelationToCurrentTask::Unrelated),
            selected_open_loop_id: Some("loop-old".to_string()),
            selected_open_loop_eligible: Some(false),
            selected_objective_source: Some("open_loop:loop-old".to_string()),
            selected_objective_eligible: Some(false),
            selected_candidate_id: Some("candidate-old".to_string()),
            selected_candidate_workstream_id: Some("workstream-old".to_string()),
            public_target_artifact_id: Some("artifact-old".to_string()),
            public_target_eligible: Some(false),
            branch_feedback_applicable: Some(false),
            last_state_from_completed_prior_task: true,
            target_explanation_matches_current_task: Some(false),
            missing_evidence: vec![],
        });
        assert_eq!(
            result.agreement_status,
            ConsistencyAgreementStatus::Conflicting
        );
        for code in [
            "primary_segment_unrelated_to_current_task",
            "objective_open_loop_ineligible_for_current_task",
            "branch_promoted_by_inapplicable_feedback",
            "last_state_from_completed_prior_task",
            "target_explanation_names_different_task",
        ] {
            assert!(
                result.conflicts.contains(&code.to_string()),
                "missing {code}"
            );
        }
    }

    #[test]
    fn consistency_distinguishes_support_thin_and_unresolved_states() {
        let consistent = evaluate_cross_layer_consistency(&CrossLayerConsistencyInput {
            current_task_turn_id: Some("turn".to_string()),
            selected_workstream_id: Some("workstream".to_string()),
            current_task_workstream_id: Some("workstream".to_string()),
            primary_segment_id: Some("segment".to_string()),
            primary_segment_relation: Some(RelationToCurrentTask::SameTask),
            ..Default::default()
        });
        assert_eq!(
            consistent.agreement_status,
            ConsistencyAgreementStatus::Consistent
        );
        let support = evaluate_cross_layer_consistency(&CrossLayerConsistencyInput {
            primary_segment_relation: Some(RelationToCurrentTask::ChildSupport),
            ..CrossLayerConsistencyInput {
                current_task_turn_id: Some("turn".to_string()),
                selected_workstream_id: Some("workstream".to_string()),
                current_task_workstream_id: Some("workstream".to_string()),
                primary_segment_id: Some("segment".to_string()),
                ..Default::default()
            }
        });
        assert_eq!(
            support.agreement_status,
            ConsistencyAgreementStatus::ConsistentWithSupportRelation
        );
        let thin = evaluate_cross_layer_consistency(&CrossLayerConsistencyInput {
            current_task_turn_id: Some("turn".to_string()),
            ..Default::default()
        });
        assert_eq!(
            thin.agreement_status,
            ConsistencyAgreementStatus::ThinButNonConflicting
        );
        let unresolved = evaluate_cross_layer_consistency(&CrossLayerConsistencyInput::default());
        assert_eq!(
            unresolved.agreement_status,
            ConsistencyAgreementStatus::Unresolved
        );
    }

    #[test]
    fn conflict_repairs_are_decision_scoped_and_deterministic() {
        let input = CrossLayerConsistencyInput {
            current_task_turn_id: Some("turn-current".to_string()),
            current_task_high_confidence: true,
            selected_open_loop_id: Some("loop-history".to_string()),
            selected_open_loop_eligible: Some(false),
            ..Default::default()
        };
        let first = evaluate_cross_layer_consistency(&input);
        let second = evaluate_cross_layer_consistency(&input);
        assert_eq!(first, second);
        assert!(first
            .repairs_or_downgrades
            .contains(&"filter_ineligible_open_loop".to_string()));
        assert_eq!(input.selected_open_loop_id.as_deref(), Some("loop-history"));
    }
}
