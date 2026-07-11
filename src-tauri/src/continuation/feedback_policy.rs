use serde::{Deserialize, Serialize};

pub(crate) const FEEDBACK_POLICY_VERSION: &str = "feedback_scope_provenance_decay.v2";
pub(crate) const FEEDBACK_POLICY_FINGERPRINT: &str =
    "feedback_scope_provenance_decay.v2:orthogonal_scope:per_target_effects:expiry";
pub(crate) const FEEDBACK_MIGRATION_POLICY_VERSION: &str =
    "feedback_scope_provenance_decay.migration.v1";

const INFERRED_NAVIGATION_TTL_MS: i64 = 10 * 60 * 1_000;
const INFERRED_RETURN_TTL_MS: i64 = 20 * 60 * 1_000;
const INFERRED_TIMEOUT_TTL_MS: i64 = 10 * 60 * 1_000;
const EXPLICIT_OPEN_TTL_MS: i64 = 4 * 60 * 60 * 1_000;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FeedbackPolicyConfig {
    pub version: String,
    pub fingerprint: String,
    pub inferred_navigation_ttl_ms: i64,
    pub inferred_return_ttl_ms: i64,
    pub inferred_timeout_ttl_ms: i64,
    pub explicit_open_ttl_ms: i64,
    pub explicit_negative_reconfirmation_rule: String,
    pub explicit_positive_contradiction_rule: String,
}

pub(crate) fn feedback_policy_config() -> FeedbackPolicyConfig {
    FeedbackPolicyConfig {
        version: FEEDBACK_POLICY_VERSION.to_string(),
        fingerprint: FEEDBACK_POLICY_FINGERPRINT.to_string(),
        inferred_navigation_ttl_ms: INFERRED_NAVIGATION_TTL_MS,
        inferred_return_ttl_ms: INFERRED_RETURN_TTL_MS,
        inferred_timeout_ttl_ms: INFERRED_TIMEOUT_TTL_MS,
        explicit_open_ttl_ms: EXPLICIT_OPEN_TTL_MS,
        explicit_negative_reconfirmation_rule: "fresh_local_reconfirmation_after_negative"
            .to_string(),
        explicit_positive_contradiction_rule: "newer_feedback_or_incompatible_task_scope"
            .to_string(),
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum FeedbackProvenance {
    ExplicitUserAccept,
    ExplicitUserReject,
    ExplicitUserCorrect,
    ExplicitUserIgnore,
    ExplicitUserNextStep,
    ExplicitOpenAction,
    InferredNavigation,
    InferredReturn,
    InferredTimeout,
    SystemMigration,
    Unknown,
}

impl FeedbackProvenance {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::ExplicitUserAccept => "explicit_user_accept",
            Self::ExplicitUserReject => "explicit_user_reject",
            Self::ExplicitUserCorrect => "explicit_user_correct",
            Self::ExplicitUserIgnore => "explicit_user_ignore",
            Self::ExplicitUserNextStep => "explicit_user_next_step",
            Self::ExplicitOpenAction => "explicit_open_action",
            Self::InferredNavigation => "inferred_navigation",
            Self::InferredReturn => "inferred_return",
            Self::InferredTimeout => "inferred_timeout",
            Self::SystemMigration => "system_migration",
            Self::Unknown => "unknown",
        }
    }

    pub(crate) fn parse(value: Option<&str>) -> Self {
        match value.unwrap_or_default().trim() {
            "explicit_user_accept" => Self::ExplicitUserAccept,
            "explicit_user_reject" => Self::ExplicitUserReject,
            "explicit_user_correct" => Self::ExplicitUserCorrect,
            "explicit_user_ignore" => Self::ExplicitUserIgnore,
            "explicit_user_next_step" => Self::ExplicitUserNextStep,
            "explicit_open_action" => Self::ExplicitOpenAction,
            "inferred_navigation" => Self::InferredNavigation,
            "inferred_return" => Self::InferredReturn,
            "inferred_timeout" => Self::InferredTimeout,
            "system_migration" => Self::SystemMigration,
            _ => Self::Unknown,
        }
    }

    pub(crate) fn explicit_user(self) -> bool {
        matches!(
            self,
            Self::ExplicitUserAccept
                | Self::ExplicitUserReject
                | Self::ExplicitUserCorrect
                | Self::ExplicitUserIgnore
                | Self::ExplicitUserNextStep
        )
    }

    pub(crate) fn inferred(self) -> bool {
        matches!(
            self,
            Self::InferredNavigation | Self::InferredReturn | Self::InferredTimeout
        )
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FeedbackPolarity {
    Positive,
    Negative,
    Neutral,
    Informational,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum FeedbackEffect {
    Suppress,
    Rank,
    Promote,
    Annotate,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct FeedbackAllowedEffects {
    pub suppress: bool,
    pub rank: bool,
    pub promote: bool,
    pub annotate: bool,
}

impl FeedbackAllowedEffects {
    pub(crate) fn allows(self, effect: FeedbackEffect) -> bool {
        match effect {
            FeedbackEffect::Suppress => self.suppress,
            FeedbackEffect::Rank => self.rank,
            FeedbackEffect::Promote => self.promote,
            FeedbackEffect::Annotate => self.annotate,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FeedbackPolicyTargetScope {
    Candidate,
    TargetArtifact,
    ChosenArtifact,
    StableKey,
    Workstream,
    GlobalPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NormalizedFeedbackTarget {
    pub scope: FeedbackPolicyTargetScope,
    pub id_or_key: String,
    pub polarity: FeedbackPolarity,
    pub allowed_effects: FeedbackAllowedEffects,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct FeedbackPolicyEvent {
    pub event_id: String,
    pub event_kind: String,
    pub provenance: FeedbackProvenance,
    pub decision_id: Option<String>,
    pub selected_candidate_id: Option<String>,
    pub target_artifact_id: Option<String>,
    pub chosen_artifact_id: Option<String>,
    pub target_artifact_stable_key: Option<String>,
    pub chosen_artifact_stable_key: Option<String>,
    pub workstream_id: Option<String>,
    pub task_turn_id: Option<String>,
    pub session_id: Option<String>,
    pub branch_context_id: Option<String>,
    pub occurred_at_ms: i64,
    pub applies_from_ms: i64,
    pub expires_at_ms: Option<i64>,
    pub confidence: f64,
    pub reason: Option<String>,
    pub evidence_ids: Vec<String>,
    pub targets: Vec<NormalizedFeedbackTarget>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct FeedbackApplicabilityContext {
    pub decision_id: Option<String>,
    pub candidate_id: Option<String>,
    pub target_artifact_id: Option<String>,
    pub artifact_stable_key: Option<String>,
    pub workstream_id: Option<String>,
    pub task_turn_id: Option<String>,
    pub session_id: Option<String>,
    pub branch_context_id: Option<String>,
    pub branch_started_at_ms: Option<i64>,
    pub now_ms: i64,
    pub requested_effect: FeedbackEffect,
    pub contradicted_by_current_evidence: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct FeedbackApplicabilityAxes {
    pub target_match: bool,
    pub decision_match: bool,
    pub task_turn_match: bool,
    pub branch_match: bool,
    pub session_match: bool,
    pub workstream_match: bool,
    pub freshness_match: bool,
    pub provenance_match: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct FeedbackApplicabilityResult {
    pub event_id: String,
    pub target: NormalizedFeedbackTarget,
    pub provenance: FeedbackProvenance,
    pub eligible: bool,
    pub requested_effect: FeedbackEffect,
    pub event_age_ms: i64,
    pub cutoff_at_ms: Option<i64>,
    pub axes: FeedbackApplicabilityAxes,
    pub rejection_reason_codes: Vec<String>,
    pub confidence_contribution: f64,
    pub policy_version: String,
    pub policy_fingerprint: String,
}

pub(crate) fn provenance_for_explicit_kind(kind: &str) -> FeedbackProvenance {
    match kind {
        "accepted" => FeedbackProvenance::ExplicitUserAccept,
        "rejected" | "artifact_only_evidence" => FeedbackProvenance::ExplicitUserReject,
        "corrected" => FeedbackProvenance::ExplicitUserCorrect,
        "ignored" | "ignored_workstream" => FeedbackProvenance::ExplicitUserIgnore,
        "user_next_step_note" => FeedbackProvenance::ExplicitUserNextStep,
        _ => FeedbackProvenance::Unknown,
    }
}

pub(crate) fn provenance_for_inferred_kind(kind: &str) -> FeedbackProvenance {
    match kind {
        "accepted" | "auto_resumed" => FeedbackProvenance::InferredReturn,
        "corrected" | "rejected" => FeedbackProvenance::InferredNavigation,
        "ignored" => FeedbackProvenance::InferredTimeout,
        _ => FeedbackProvenance::Unknown,
    }
}

pub(crate) fn default_expiry_ms(
    provenance: FeedbackProvenance,
    occurred_at_ms: i64,
) -> Option<i64> {
    let policy = feedback_policy_config();
    let ttl = match provenance {
        FeedbackProvenance::InferredNavigation => Some(policy.inferred_navigation_ttl_ms),
        FeedbackProvenance::InferredReturn => Some(policy.inferred_return_ttl_ms),
        FeedbackProvenance::InferredTimeout => Some(policy.inferred_timeout_ttl_ms),
        FeedbackProvenance::ExplicitOpenAction => Some(policy.explicit_open_ttl_ms),
        _ => None,
    };
    ttl.map(|ttl| occurred_at_ms.saturating_add(ttl))
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn normalize_feedback_targets(
    event_kind: &str,
    provenance: FeedbackProvenance,
    selected_candidate_id: Option<&str>,
    target_artifact_id: Option<&str>,
    target_artifact_stable_key: Option<&str>,
    chosen_artifact_id: Option<&str>,
    chosen_artifact_stable_key: Option<&str>,
    workstream_id: Option<&str>,
) -> Vec<NormalizedFeedbackTarget> {
    let positive_effects = FeedbackAllowedEffects {
        rank: provenance.explicit_user() || provenance.inferred(),
        promote: matches!(
            provenance,
            FeedbackProvenance::ExplicitUserAccept | FeedbackProvenance::ExplicitUserCorrect
        ),
        annotate: true,
        ..FeedbackAllowedEffects::default()
    };
    let negative_effects = FeedbackAllowedEffects {
        suppress: true,
        rank: true,
        annotate: true,
        ..FeedbackAllowedEffects::default()
    };
    let neutral_effects = FeedbackAllowedEffects {
        annotate: true,
        ..FeedbackAllowedEffects::default()
    };
    let mut values = Vec::new();
    let selected_polarity = match event_kind {
        "accepted" | "auto_resumed" => FeedbackPolarity::Positive,
        "rejected" | "ignored" | "corrected" | "artifact_only_evidence" => {
            FeedbackPolarity::Negative
        }
        "ignored_workstream" => FeedbackPolarity::Negative,
        "user_next_step_note" => FeedbackPolarity::Neutral,
        _ => FeedbackPolarity::Informational,
    };
    let selected_effects = match selected_polarity {
        FeedbackPolarity::Positive => positive_effects,
        FeedbackPolarity::Negative => negative_effects,
        FeedbackPolarity::Neutral | FeedbackPolarity::Informational => neutral_effects,
    };
    if event_kind == "ignored_workstream" {
        push_target(
            &mut values,
            FeedbackPolicyTargetScope::Workstream,
            workstream_id,
            selected_polarity,
            selected_effects,
        );
        return values;
    }
    push_target(
        &mut values,
        FeedbackPolicyTargetScope::Candidate,
        selected_candidate_id,
        selected_polarity,
        selected_effects,
    );
    push_target(
        &mut values,
        FeedbackPolicyTargetScope::TargetArtifact,
        target_artifact_id,
        selected_polarity,
        selected_effects,
    );
    push_target(
        &mut values,
        FeedbackPolicyTargetScope::StableKey,
        target_artifact_stable_key,
        selected_polarity,
        selected_effects,
    );
    if event_kind == "corrected" {
        push_target(
            &mut values,
            FeedbackPolicyTargetScope::ChosenArtifact,
            chosen_artifact_id,
            FeedbackPolarity::Positive,
            positive_effects,
        );
        push_target(
            &mut values,
            FeedbackPolicyTargetScope::StableKey,
            chosen_artifact_stable_key,
            FeedbackPolarity::Positive,
            positive_effects,
        );
    }
    values
}

fn push_target(
    values: &mut Vec<NormalizedFeedbackTarget>,
    scope: FeedbackPolicyTargetScope,
    id_or_key: Option<&str>,
    polarity: FeedbackPolarity,
    allowed_effects: FeedbackAllowedEffects,
) {
    let Some(id_or_key) = id_or_key.map(str::trim).filter(|value| !value.is_empty()) else {
        return;
    };
    let target = NormalizedFeedbackTarget {
        scope,
        id_or_key: id_or_key.to_string(),
        polarity,
        allowed_effects,
    };
    if !values.contains(&target) {
        values.push(target);
    }
}

pub(crate) fn evaluate_feedback_applicability(
    event: &FeedbackPolicyEvent,
    target: &NormalizedFeedbackTarget,
    context: &FeedbackApplicabilityContext,
) -> FeedbackApplicabilityResult {
    let target_match = target_matches(target, context);
    let inferred_requires_decision =
        event.provenance.inferred() && matches!(target.scope, FeedbackPolicyTargetScope::Candidate);
    let decision_match = !inferred_requires_decision
        || event.decision_id.is_some()
            && event.decision_id.as_deref() == context.decision_id.as_deref();
    let task_turn_required =
        event.provenance.inferred() || context.requested_effect == FeedbackEffect::Promote;
    let task_turn_match = !task_turn_required
        || event.task_turn_id.is_some()
            && event.task_turn_id.as_deref() == context.task_turn_id.as_deref();
    let branch_match = event.branch_context_id.is_none()
        || event.branch_context_id.as_deref() == context.branch_context_id.as_deref();
    let session_required =
        event.provenance.inferred() || context.requested_effect == FeedbackEffect::Promote;
    let session_match = !session_required
        || event.session_id.is_some()
            && event.session_id.as_deref() == context.session_id.as_deref();
    let workstream_match = event.workstream_id.is_none()
        || event.workstream_id.as_deref() == context.workstream_id.as_deref();
    let freshness_match = context.now_ms >= event.applies_from_ms
        && event
            .expires_at_ms
            .is_none_or(|expires_at_ms| context.now_ms < expires_at_ms);
    let provenance_match = !matches!(
        event.provenance,
        FeedbackProvenance::Unknown | FeedbackProvenance::SystemMigration
    );
    let axes = FeedbackApplicabilityAxes {
        target_match,
        decision_match,
        task_turn_match,
        branch_match,
        session_match,
        workstream_match,
        freshness_match,
        provenance_match,
    };
    let mut reasons = Vec::new();
    if !target_match || !decision_match || !branch_match || !workstream_match {
        push_reason(&mut reasons, "scope_mismatch");
    }
    if !task_turn_match {
        push_reason(&mut reasons, "different_task_turn");
    }
    if !session_match {
        push_reason(&mut reasons, "different_session_without_durable_scope");
    }
    if context.now_ms < event.applies_from_ms
        || event
            .expires_at_ms
            .is_some_and(|expires_at_ms| context.now_ms >= expires_at_ms)
    {
        push_reason(&mut reasons, "expired");
    }
    if context
        .branch_started_at_ms
        .is_some_and(|started_at_ms| event.occurred_at_ms < started_at_ms)
    {
        push_reason(&mut reasons, "predates_branch");
    }
    if context.requested_effect == FeedbackEffect::Promote && event.provenance.inferred() {
        push_reason(&mut reasons, "inferred_source_cannot_promote");
    }
    if !provenance_match {
        push_reason(&mut reasons, "missing_provenance");
    }
    if !target.allowed_effects.allows(context.requested_effect) {
        if context.requested_effect == FeedbackEffect::Promote && event.provenance.inferred() {
            push_reason(&mut reasons, "inferred_source_cannot_promote");
        } else {
            push_reason(&mut reasons, "scope_mismatch");
        }
    }
    if context.contradicted_by_current_evidence {
        push_reason(&mut reasons, "contradicted_by_current_evidence");
    }
    let eligible = reasons.is_empty();
    let contribution_scale = match event.provenance {
        FeedbackProvenance::ExplicitUserAccept | FeedbackProvenance::ExplicitUserCorrect => 0.20,
        FeedbackProvenance::ExplicitOpenAction => 0.08,
        FeedbackProvenance::InferredNavigation | FeedbackProvenance::InferredReturn => 0.05,
        FeedbackProvenance::InferredTimeout => 0.03,
        _ => 0.0,
    };
    FeedbackApplicabilityResult {
        event_id: event.event_id.clone(),
        target: target.clone(),
        provenance: event.provenance,
        eligible,
        requested_effect: context.requested_effect,
        event_age_ms: context.now_ms.saturating_sub(event.occurred_at_ms),
        cutoff_at_ms: event.expires_at_ms,
        axes,
        rejection_reason_codes: reasons,
        confidence_contribution: if eligible {
            (event.confidence.clamp(0.0, 1.0) * contribution_scale * 10_000.0).round() / 10_000.0
        } else {
            0.0
        },
        policy_version: FEEDBACK_POLICY_VERSION.to_string(),
        policy_fingerprint: FEEDBACK_POLICY_FINGERPRINT.to_string(),
    }
}

fn target_matches(
    target: &NormalizedFeedbackTarget,
    context: &FeedbackApplicabilityContext,
) -> bool {
    match target.scope {
        FeedbackPolicyTargetScope::Candidate => {
            context.candidate_id.as_deref() == Some(target.id_or_key.as_str())
        }
        FeedbackPolicyTargetScope::TargetArtifact | FeedbackPolicyTargetScope::ChosenArtifact => {
            context.target_artifact_id.as_deref() == Some(target.id_or_key.as_str())
        }
        FeedbackPolicyTargetScope::StableKey => {
            context.artifact_stable_key.as_deref() == Some(target.id_or_key.as_str())
        }
        FeedbackPolicyTargetScope::Workstream => {
            context.workstream_id.as_deref() == Some(target.id_or_key.as_str())
        }
        FeedbackPolicyTargetScope::GlobalPolicy => true,
    }
}

fn push_reason(values: &mut Vec<String>, reason: &str) {
    if !values.iter().any(|value| value == reason) {
        values.push(reason.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(provenance: FeedbackProvenance) -> FeedbackPolicyEvent {
        let occurred_at_ms = 1_000;
        FeedbackPolicyEvent {
            event_id: "feedback-1".to_string(),
            event_kind: "corrected".to_string(),
            provenance,
            decision_id: Some("decision-a".to_string()),
            selected_candidate_id: Some("candidate-a".to_string()),
            target_artifact_id: Some("artifact-old".to_string()),
            chosen_artifact_id: Some("artifact-new".to_string()),
            target_artifact_stable_key: Some("stable-old".to_string()),
            chosen_artifact_stable_key: Some("stable-new".to_string()),
            workstream_id: Some("workstream-a".to_string()),
            task_turn_id: Some("turn-a".to_string()),
            session_id: Some("session-a".to_string()),
            branch_context_id: None,
            occurred_at_ms,
            applies_from_ms: occurred_at_ms,
            expires_at_ms: default_expiry_ms(provenance, occurred_at_ms),
            confidence: 0.9,
            reason: None,
            evidence_ids: Vec::new(),
            targets: normalize_feedback_targets(
                "corrected",
                provenance,
                Some("candidate-a"),
                Some("artifact-old"),
                Some("stable-old"),
                Some("artifact-new"),
                Some("stable-new"),
                Some("workstream-a"),
            ),
        }
    }

    fn context(effect: FeedbackEffect) -> FeedbackApplicabilityContext {
        FeedbackApplicabilityContext {
            decision_id: Some("decision-a".to_string()),
            candidate_id: None,
            target_artifact_id: Some("artifact-new".to_string()),
            artifact_stable_key: Some("stable-new".to_string()),
            workstream_id: Some("workstream-a".to_string()),
            task_turn_id: Some("turn-a".to_string()),
            session_id: Some("session-a".to_string()),
            branch_context_id: None,
            branch_started_at_ms: Some(900),
            now_ms: 2_000,
            requested_effect: effect,
            contradicted_by_current_evidence: false,
        }
    }

    #[test]
    fn provenance_is_not_derived_from_transport_source() {
        assert_eq!(
            provenance_for_inferred_kind("corrected"),
            FeedbackProvenance::InferredNavigation
        );
        assert_eq!(
            provenance_for_explicit_kind("corrected"),
            FeedbackProvenance::ExplicitUserCorrect
        );
        assert_eq!(
            FeedbackProvenance::parse(Some("island_primary")),
            FeedbackProvenance::Unknown
        );
    }

    #[test]
    fn corrected_polarity_is_target_specific_and_keeps_both_stable_keys() {
        let event = event(FeedbackProvenance::ExplicitUserCorrect);
        assert!(event.targets.iter().any(|target| {
            target.id_or_key == "artifact-old" && target.polarity == FeedbackPolarity::Negative
        }));
        assert!(event.targets.iter().any(|target| {
            target.id_or_key == "artifact-new" && target.polarity == FeedbackPolarity::Positive
        }));
        assert!(event.targets.iter().any(|target| {
            target.id_or_key == "stable-old" && target.polarity == FeedbackPolarity::Negative
        }));
        assert!(event.targets.iter().any(|target| {
            target.id_or_key == "stable-new" && target.polarity == FeedbackPolarity::Positive
        }));
        assert!(!event
            .targets
            .iter()
            .any(|target| target.scope == FeedbackPolicyTargetScope::Workstream));
    }

    #[test]
    fn inferred_positive_cannot_promote_even_inside_matching_scope() {
        let event = event(FeedbackProvenance::InferredNavigation);
        let target = event
            .targets
            .iter()
            .find(|target| target.id_or_key == "artifact-new")
            .unwrap();
        let result =
            evaluate_feedback_applicability(&event, target, &context(FeedbackEffect::Promote));
        assert!(!result.eligible);
        assert!(result
            .rejection_reason_codes
            .contains(&"inferred_source_cannot_promote".to_string()));
    }

    #[test]
    fn explicit_correction_promotes_only_after_branch_in_same_turn_and_session() {
        let event = event(FeedbackProvenance::ExplicitUserCorrect);
        let target = event
            .targets
            .iter()
            .find(|target| target.id_or_key == "artifact-new")
            .unwrap();
        assert!(
            evaluate_feedback_applicability(&event, target, &context(FeedbackEffect::Promote))
                .eligible
        );
        let mut wrong_turn = context(FeedbackEffect::Promote);
        wrong_turn.task_turn_id = Some("turn-b".to_string());
        let result = evaluate_feedback_applicability(&event, target, &wrong_turn);
        assert!(!result.eligible);
        assert!(result
            .rejection_reason_codes
            .contains(&"different_task_turn".to_string()));
        let mut wrong_session = context(FeedbackEffect::Promote);
        wrong_session.session_id = Some("session-b".to_string());
        let result = evaluate_feedback_applicability(&event, target, &wrong_session);
        assert!(result
            .rejection_reason_codes
            .contains(&"different_session_without_durable_scope".to_string()));
    }

    #[test]
    fn positive_before_branch_start_is_rejected() {
        let event = event(FeedbackProvenance::ExplicitUserCorrect);
        let target = event
            .targets
            .iter()
            .find(|target| target.id_or_key == "artifact-new")
            .unwrap();
        let mut context = context(FeedbackEffect::Promote);
        context.branch_started_at_ms = Some(1_001);
        let result = evaluate_feedback_applicability(&event, target, &context);
        assert!(!result.eligible);
        assert!(result
            .rejection_reason_codes
            .contains(&"predates_branch".to_string()));
    }

    #[test]
    fn freshness_boundary_is_exclusive_and_audited() {
        let event = event(FeedbackProvenance::InferredReturn);
        let target = event
            .targets
            .iter()
            .find(|target| target.id_or_key == "artifact-new")
            .unwrap();
        let expires = event.expires_at_ms.unwrap();
        let mut before = context(FeedbackEffect::Rank);
        before.now_ms = expires - 1;
        assert!(evaluate_feedback_applicability(&event, target, &before).eligible);
        let mut at = before.clone();
        at.now_ms = expires;
        let result = evaluate_feedback_applicability(&event, target, &at);
        assert!(!result.eligible);
        assert!(result
            .rejection_reason_codes
            .contains(&"expired".to_string()));
    }

    #[test]
    fn legacy_unknown_positive_is_conservative() {
        let event = event(FeedbackProvenance::Unknown);
        let target = event
            .targets
            .iter()
            .find(|target| target.id_or_key == "artifact-new")
            .unwrap();
        let result =
            evaluate_feedback_applicability(&event, target, &context(FeedbackEffect::Promote));
        assert!(!result.eligible);
        assert!(result
            .rejection_reason_codes
            .contains(&"missing_provenance".to_string()));
    }

    #[test]
    fn applicability_dimensions_are_conjunctive_not_interchangeable() {
        let mut event = event(FeedbackProvenance::InferredReturn);
        event.branch_context_id = Some("branch-a".to_string());
        let target = event
            .targets
            .iter()
            .find(|target| target.scope == FeedbackPolicyTargetScope::Candidate)
            .unwrap();
        let mut context = context(FeedbackEffect::Rank);
        context.candidate_id = Some("candidate-a".to_string());
        context.branch_context_id = Some("branch-b".to_string());
        context.decision_id = Some("decision-b".to_string());
        context.workstream_id = Some("workstream-b".to_string());
        context.contradicted_by_current_evidence = true;

        let result = evaluate_feedback_applicability(&event, target, &context);

        assert!(!result.eligible);
        assert!(result
            .rejection_reason_codes
            .contains(&"scope_mismatch".to_string()));
        assert!(result
            .rejection_reason_codes
            .contains(&"contradicted_by_current_evidence".to_string()));
        assert!(!result.axes.decision_match);
        assert!(!result.axes.branch_match);
        assert!(!result.axes.workstream_match);
    }
}
