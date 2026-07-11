use super::{
    activity_recap::ActivityConfidence, CrossLayerConsistencyResult, CurrentTaskTurn,
    DirectTargetPolicyResult,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const CONTINUE_CONFIDENCE_SCHEMA_V2: &str = "smalltalk.continue_confidence.v2";
pub const CONTINUE_CONFIDENCE_POLICY_VERSION: &str = "p6.06.v1";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ConfidenceDimension {
    SurfaceIdentity,
    ActiveWindowOwnership,
    RegionSegmentation,
    SpeakerAttribution,
    TurnOrder,
    LatestUserGoal,
    TaskObject,
    CurrentActorState,
    ExecutionState,
    CurrentActor,
    WaitingOn,
    RelationToPrior,
    WorkstreamAlignment,
    BranchRole,
    OpenLoopRelevance,
    RecapClaimSupport,
    TargetIdentity,
    TargetOpenability,
    DirectTargetPolicy,
}

impl ConfidenceDimension {
    pub const ALL: [Self; 19] = [
        Self::SurfaceIdentity,
        Self::ActiveWindowOwnership,
        Self::RegionSegmentation,
        Self::SpeakerAttribution,
        Self::TurnOrder,
        Self::LatestUserGoal,
        Self::TaskObject,
        Self::CurrentActorState,
        Self::ExecutionState,
        Self::CurrentActor,
        Self::WaitingOn,
        Self::RelationToPrior,
        Self::WorkstreamAlignment,
        Self::BranchRole,
        Self::OpenLoopRelevance,
        Self::RecapClaimSupport,
        Self::TargetIdentity,
        Self::TargetOpenability,
        Self::DirectTargetPolicy,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::SurfaceIdentity => "surface_identity",
            Self::ActiveWindowOwnership => "active_window_ownership",
            Self::RegionSegmentation => "region_segmentation",
            Self::SpeakerAttribution => "speaker_attribution",
            Self::TurnOrder => "turn_order",
            Self::LatestUserGoal => "latest_user_goal",
            Self::TaskObject => "task_object",
            Self::CurrentActorState => "current_actor_state",
            Self::ExecutionState => "execution_state",
            Self::CurrentActor => "current_actor",
            Self::WaitingOn => "waiting_on",
            Self::RelationToPrior => "relation_to_prior",
            Self::WorkstreamAlignment => "workstream_alignment",
            Self::BranchRole => "branch_role",
            Self::OpenLoopRelevance => "open_loop_relevance",
            Self::RecapClaimSupport => "recap_claim_support",
            Self::TargetIdentity => "target_identity",
            Self::TargetOpenability => "target_openability",
            Self::DirectTargetPolicy => "direct_target_policy",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum ConfidenceLabel {
    None,
    Low,
    Medium,
    High,
}

impl ConfidenceLabel {
    pub fn from_score(score: f64) -> Self {
        match score.clamp(0.0, 1.0) {
            value if value < 0.20 => Self::None,
            value if value < 0.50 => Self::Low,
            value if value < 0.75 => Self::Medium,
            _ => Self::High,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
        }
    }

    pub fn activity(self) -> ActivityConfidence {
        match self {
            Self::None => ActivityConfidence::None,
            Self::Low => ActivityConfidence::Low,
            Self::Medium => ActivityConfidence::Medium,
            Self::High => ActivityConfidence::High,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConfidenceDimensionValue {
    pub score: f64,
    pub label: ConfidenceLabel,
    pub supporting_evidence_ids: Vec<String>,
    pub missing_evidence: Vec<String>,
    pub quality_flags: Vec<String>,
    pub calculation_reason: String,
}

impl ConfidenceDimensionValue {
    fn new(
        score: f64,
        supporting_evidence_ids: Vec<String>,
        missing_evidence: Vec<String>,
        quality_flags: Vec<String>,
        calculation_reason: impl Into<String>,
    ) -> Self {
        let score = round(score);
        Self {
            score,
            label: ConfidenceLabel::from_score(score),
            supporting_evidence_ids: dedup(supporting_evidence_ids),
            missing_evidence: dedup(missing_evidence),
            quality_flags: dedup(quality_flags),
            calculation_reason: calculation_reason.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum ConfidenceClaim {
    SurfaceIdentity,
    CurrentTask,
    ActorActivity,
    PriorTaskCompleted,
    NextAction,
    ActivityRecap,
    DirectTarget,
}

impl ConfidenceClaim {
    pub const ALL: [Self; 7] = [
        Self::SurfaceIdentity,
        Self::CurrentTask,
        Self::ActorActivity,
        Self::PriorTaskCompleted,
        Self::NextAction,
        Self::ActivityRecap,
        Self::DirectTarget,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::SurfaceIdentity => "surface_identity",
            Self::CurrentTask => "current_task",
            Self::ActorActivity => "actor_activity",
            Self::PriorTaskCompleted => "prior_task_completed",
            Self::NextAction => "next_action",
            Self::ActivityRecap => "activity_recap",
            Self::DirectTarget => "direct_target",
        }
    }
}

pub fn claim_dependencies(claim: ConfidenceClaim) -> &'static [ConfidenceDimension] {
    use ConfidenceDimension as D;
    match claim {
        ConfidenceClaim::SurfaceIdentity => &[D::SurfaceIdentity, D::ActiveWindowOwnership],
        ConfidenceClaim::CurrentTask => &[
            D::SpeakerAttribution,
            D::TurnOrder,
            D::LatestUserGoal,
            D::TaskObject,
        ],
        ConfidenceClaim::ActorActivity => &[
            D::SpeakerAttribution,
            D::CurrentActorState,
            D::CurrentActor,
            D::RelationToPrior,
        ],
        ConfidenceClaim::PriorTaskCompleted => &[D::RelationToPrior, D::ExecutionState],
        ConfidenceClaim::NextAction => &[
            D::LatestUserGoal,
            D::TaskObject,
            D::ExecutionState,
            D::CurrentActor,
            D::WaitingOn,
            D::OpenLoopRelevance,
        ],
        ConfidenceClaim::ActivityRecap => &[
            D::SpeakerAttribution,
            D::TurnOrder,
            D::LatestUserGoal,
            D::TaskObject,
            D::ExecutionState,
            D::CurrentActor,
            D::WaitingOn,
            D::WorkstreamAlignment,
            D::RecapClaimSupport,
        ],
        ConfidenceClaim::DirectTarget => &[
            D::TargetIdentity,
            D::TargetOpenability,
            D::DirectTargetPolicy,
        ],
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClaimConfidence {
    pub claim: ConfidenceClaim,
    pub score: f64,
    pub label: ConfidenceLabel,
    pub critical_dimensions: Vec<ConfidenceDimension>,
    pub bounding_dimensions: Vec<ConfidenceDimension>,
    pub missing_evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContinueConfidenceVector {
    pub schema: String,
    pub policy_version: String,
    pub revision: String,
    pub dimensions: BTreeMap<ConfidenceDimension, ConfidenceDimensionValue>,
    pub claims: BTreeMap<ConfidenceClaim, ClaimConfidence>,
}

impl Default for ContinueConfidenceVector {
    fn default() -> Self {
        let dimensions = ConfidenceDimension::ALL
            .into_iter()
            .map(|dimension| {
                (
                    dimension,
                    ConfidenceDimensionValue::new(
                        0.0,
                        Vec::new(),
                        vec![dimension.as_str().to_string()],
                        vec!["not_evaluated".to_string()],
                        "default unevaluated confidence",
                    ),
                )
            })
            .collect::<BTreeMap<_, _>>();
        let claims = ConfidenceClaim::ALL
            .into_iter()
            .map(|claim| (claim, bound_claim(claim, &dimensions)))
            .collect();
        Self {
            schema: CONTINUE_CONFIDENCE_SCHEMA_V2.to_string(),
            policy_version: CONTINUE_CONFIDENCE_POLICY_VERSION.to_string(),
            revision: stable_hash(b"default_unevaluated"),
            dimensions,
            claims,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LegacyConfidenceDerivation {
    pub confidence: f64,
    pub confidence_label: String,
    pub activity_confidence: ActivityConfidence,
    pub target_confidence: ActivityConfidence,
    pub confidence_claim: ConfidenceClaim,
    pub activity_claim: ConfidenceClaim,
    pub target_claim: ConfidenceClaim,
    pub reason: String,
}

impl Default for LegacyConfidenceDerivation {
    fn default() -> Self {
        ContinueConfidenceVector::default().legacy_derivation()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CompactConfidenceSummary {
    pub schema: String,
    pub task: ClaimConfidence,
    pub state: ClaimConfidence,
    pub recap: ClaimConfidence,
    pub target: ClaimConfidence,
    pub material_revision: String,
}

impl Default for CompactConfidenceSummary {
    fn default() -> Self {
        ContinueConfidenceVector::default().compact_summary()
    }
}

pub(crate) struct ConfidenceBuildInput<'a> {
    pub current_task_turn: Option<&'a CurrentTaskTurn>,
    pub cross_layer_consistency: &'a CrossLayerConsistencyResult,
    pub direct_target_policy: &'a DirectTargetPolicyResult,
    pub surface_identity_score: f64,
    pub surface_evidence_ids: Vec<String>,
    pub surface_missing_evidence: Vec<String>,
    pub recap_support_score: f64,
}

pub(crate) fn build_confidence_vector(input: ConfidenceBuildInput<'_>) -> ContinueConfidenceVector {
    use ConfidenceDimension as D;
    let mut dimensions = BTreeMap::new();
    let surface_score = input.surface_identity_score.clamp(0.0, 1.0);
    dimensions.insert(
        D::SurfaceIdentity,
        ConfidenceDimensionValue::new(
            surface_score,
            input.surface_evidence_ids.clone(),
            input.surface_missing_evidence.clone(),
            Vec::new(),
            "bounded surface snapshot identity only",
        ),
    );
    dimensions.insert(
        D::ActiveWindowOwnership,
        ConfidenceDimensionValue::new(
            surface_score.min(
                input
                    .current_task_turn
                    .map(|turn| turn.attribution_confidence)
                    .unwrap_or(0.0),
            ),
            input.surface_evidence_ids.clone(),
            missing_if(
                input.current_task_turn.is_none(),
                "active_window_owned_task_evidence",
            ),
            Vec::new(),
            "minimum of surface identity and attributed current-task ownership",
        ),
    );

    let (task_evidence, task_missing, task_flags) = input
        .current_task_turn
        .map(|turn| {
            let mut evidence = turn.latest_user_span_ids.clone();
            evidence.extend(turn.current_state_span_ids.clone());
            evidence.extend(turn.supporting_action_ids.clone());
            (
                evidence,
                turn.missing_evidence.clone(),
                turn.quality_flags.clone(),
            )
        })
        .unwrap_or_else(|| {
            (
                Vec::new(),
                vec!["current_task_turn".to_string()],
                vec!["current_task_turn_missing".to_string()],
            )
        });
    let task = input.current_task_turn;
    let attribution = task.map(|turn| turn.attribution_confidence).unwrap_or(0.0);
    for (dimension, score, reason) in [
        (
            D::RegionSegmentation,
            attribution,
            "ordered role-region evidence quality",
        ),
        (
            D::SpeakerAttribution,
            attribution,
            "current task-turn attribution confidence",
        ),
        (
            D::TurnOrder,
            attribution,
            "current task-turn ordered-span confidence",
        ),
        (
            D::LatestUserGoal,
            task.map(|turn| turn.goal_confidence).unwrap_or(0.0),
            "current task-turn goal confidence",
        ),
        (
            D::TaskObject,
            task.map(|turn| turn.task_object_confidence).unwrap_or(0.0),
            "current task-turn task-object confidence",
        ),
        (
            D::CurrentActorState,
            task.map(|turn| turn.actor_state_confidence).unwrap_or(0.0),
            "current task-turn actor-state confidence",
        ),
        (
            D::ExecutionState,
            task.map(|turn| turn.execution_state_confidence)
                .unwrap_or(0.0),
            "current task-turn execution-state confidence",
        ),
        (
            D::CurrentActor,
            task.map(|turn| turn.actor_state_confidence).unwrap_or(0.0),
            "current task-turn current-actor confidence",
        ),
        (
            D::WaitingOn,
            task.map(|turn| turn.waiting_on_confidence).unwrap_or(0.0),
            "current task-turn waiting-on confidence",
        ),
        (
            D::RelationToPrior,
            task.map(|turn| turn.relation_confidence).unwrap_or(0.0),
            "current task-turn prior-relation confidence",
        ),
    ] {
        dimensions.insert(
            dimension,
            ConfidenceDimensionValue::new(
                score,
                task_evidence.clone(),
                task_missing.clone(),
                task_flags.clone(),
                reason,
            ),
        );
    }

    let consistency_score = match input.cross_layer_consistency.agreement_status {
        super::semantic_consistency::ConsistencyAgreementStatus::Consistent => 0.90,
        super::semantic_consistency::ConsistencyAgreementStatus::ConsistentWithSupportRelation => {
            0.82
        }
        super::semantic_consistency::ConsistencyAgreementStatus::ThinButNonConflicting => 0.52,
        super::semantic_consistency::ConsistencyAgreementStatus::Conflicting => 0.20,
        super::semantic_consistency::ConsistencyAgreementStatus::Unresolved => 0.10,
    };
    dimensions.insert(
        D::WorkstreamAlignment,
        ConfidenceDimensionValue::new(
            consistency_score,
            Vec::new(),
            input.cross_layer_consistency.missing_evidence.clone(),
            input.cross_layer_consistency.conflicts.clone(),
            "cross-layer consistency agreement bounds workstream alignment",
        ),
    );
    dimensions.insert(
        D::BranchRole,
        ConfidenceDimensionValue::new(
            if input
                .cross_layer_consistency
                .conflicts
                .iter()
                .any(|value| value.contains("branch"))
            {
                0.20
            } else {
                consistency_score
            },
            Vec::new(),
            Vec::new(),
            input.cross_layer_consistency.conflicts.clone(),
            "branch eligibility and semantic consistency",
        ),
    );
    dimensions.insert(
        D::OpenLoopRelevance,
        ConfidenceDimensionValue::new(
            if input
                .cross_layer_consistency
                .selected_open_loop_id
                .is_none()
            {
                task.map(|turn| turn.goal_confidence)
                    .unwrap_or(0.0)
                    .min(0.70)
            } else {
                consistency_score
            },
            Vec::new(),
            missing_if(
                input
                    .cross_layer_consistency
                    .selected_open_loop_id
                    .is_none(),
                "task_linked_open_loop_or_explicit_next_action",
            ),
            input.cross_layer_consistency.conflicts.clone(),
            "eligible task-linked open loop or bounded current task goal",
        ),
    );
    dimensions.insert(
        D::RecapClaimSupport,
        ConfidenceDimensionValue::new(
            input.recap_support_score.min(consistency_score),
            task_evidence.clone(),
            input.cross_layer_consistency.missing_evidence.clone(),
            input.cross_layer_consistency.conflicts.clone(),
            "recap support bounded by semantic-center consistency",
        ),
    );

    let target_evidence = input.direct_target_policy.supporting_evidence_ids.clone();
    let target_identity = if input.direct_target_policy.target_identity_confident {
        input
            .direct_target_policy
            .target_identity_confidence
            .unwrap_or(0.0)
            .clamp(0.0, 1.0)
    } else {
        0.0
    };
    let target_openability = if input.direct_target_policy.openable {
        0.90
    } else {
        0.0
    };
    let target_policy = if input.direct_target_policy.direct_target_allowed {
        0.90
    } else {
        0.0
    };
    let target_missing = input
        .direct_target_policy
        .reason_codes
        .iter()
        .filter(|reason| {
            matches!(
                reason.as_str(),
                "no_validated_direct_locator"
                    | "locator_not_openable_under_strict_policy"
                    | "openability_label_without_locator"
            )
        })
        .cloned()
        .collect::<Vec<_>>();
    for (dimension, score, reason) in [
        (
            D::TargetIdentity,
            target_identity,
            "validated direct locator identity",
        ),
        (
            D::TargetOpenability,
            target_openability,
            "strict direct-locator openability",
        ),
        (
            D::DirectTargetPolicy,
            target_policy,
            "canonical direct-target policy eligibility",
        ),
    ] {
        dimensions.insert(
            dimension,
            ConfidenceDimensionValue::new(
                score,
                target_evidence.clone(),
                target_missing.clone(),
                input.direct_target_policy.reason_codes.clone(),
                reason,
            ),
        );
    }

    let claims = ConfidenceClaim::ALL
        .into_iter()
        .map(|claim| (claim, bound_claim(claim, &dimensions)))
        .collect::<BTreeMap<_, _>>();
    let revision_seed =
        serde_json::to_vec(&(&dimensions, &claims, CONTINUE_CONFIDENCE_POLICY_VERSION))
            .unwrap_or_default();
    ContinueConfidenceVector {
        schema: CONTINUE_CONFIDENCE_SCHEMA_V2.to_string(),
        policy_version: CONTINUE_CONFIDENCE_POLICY_VERSION.to_string(),
        revision: stable_hash(&revision_seed),
        dimensions,
        claims,
    }
}

pub fn bound_claim(
    claim: ConfidenceClaim,
    dimensions: &BTreeMap<ConfidenceDimension, ConfidenceDimensionValue>,
) -> ClaimConfidence {
    let critical_dimensions = claim_dependencies(claim).to_vec();
    let score = critical_dimensions
        .iter()
        .filter_map(|dimension| dimensions.get(dimension).map(|value| value.score))
        .reduce(f64::min)
        .unwrap_or(0.0);
    let bounding_dimensions = critical_dimensions
        .iter()
        .filter(|dimension| {
            dimensions
                .get(dimension)
                .is_some_and(|value| (value.score - score).abs() < 0.000_001)
        })
        .copied()
        .collect();
    let missing_evidence = critical_dimensions
        .iter()
        .filter_map(|dimension| dimensions.get(dimension))
        .flat_map(|value| value.missing_evidence.clone())
        .collect();
    ClaimConfidence {
        claim,
        score: round(score),
        label: ConfidenceLabel::from_score(score),
        critical_dimensions,
        bounding_dimensions,
        missing_evidence: dedup(missing_evidence),
    }
}

impl ContinueConfidenceVector {
    pub fn claim(&self, claim: ConfidenceClaim) -> ClaimConfidence {
        self.claims
            .get(&claim)
            .cloned()
            .unwrap_or_else(|| bound_claim(claim, &self.dimensions))
    }

    pub fn legacy_derivation(&self) -> LegacyConfidenceDerivation {
        let activity = self.claim(ConfidenceClaim::ActivityRecap);
        let target = self.claim(ConfidenceClaim::DirectTarget);
        LegacyConfidenceDerivation {
            confidence: activity.score,
            confidence_label: match activity.label {
                ConfidenceLabel::None => "thin",
                other => other.as_str(),
            }
            .to_string(),
            activity_confidence: activity.label.activity(),
            target_confidence: target.label.activity(),
            confidence_claim: ConfidenceClaim::ActivityRecap,
            activity_claim: ConfidenceClaim::ActivityRecap,
            target_claim: ConfidenceClaim::DirectTarget,
            reason: "legacy confidence is the bounded activity-recap claim; target confidence is derived independently from the direct-target claim".to_string(),
        }
    }

    pub fn compact_summary(&self) -> CompactConfidenceSummary {
        CompactConfidenceSummary {
            schema: "smalltalk.continue_confidence_summary.v1".to_string(),
            task: self.claim(ConfidenceClaim::CurrentTask),
            state: self.claim(ConfidenceClaim::ActorActivity),
            recap: self.claim(ConfidenceClaim::ActivityRecap),
            target: self.claim(ConfidenceClaim::DirectTarget),
            material_revision: self.revision.clone(),
        }
    }
}

fn missing_if(condition: bool, value: &str) -> Vec<String> {
    condition.then(|| value.to_string()).into_iter().collect()
}

fn round(value: f64) -> f64 {
    (value.clamp(0.0, 1.0) * 10_000.0).round() / 10_000.0
}

fn dedup(mut values: Vec<String>) -> Vec<String> {
    values.sort();
    values.dedup();
    values
}

fn stable_hash(bytes: &[u8]) -> String {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn value(score: f64) -> ConfidenceDimensionValue {
        ConfidenceDimensionValue::new(score, vec![], vec![], vec![], "test")
    }

    #[test]
    fn claim_dependency_mapping_is_stable_and_complete() {
        assert_eq!(
            claim_dependencies(ConfidenceClaim::DirectTarget),
            &[
                ConfidenceDimension::TargetIdentity,
                ConfidenceDimension::TargetOpenability,
                ConfidenceDimension::DirectTargetPolicy,
            ]
        );
        assert!(claim_dependencies(ConfidenceClaim::CurrentTask)
            .contains(&ConfidenceDimension::LatestUserGoal));
    }

    #[test]
    fn weakest_critical_dimension_bounds_claim() {
        let dimensions = BTreeMap::from([
            (ConfidenceDimension::SpeakerAttribution, value(0.9)),
            (ConfidenceDimension::TurnOrder, value(0.8)),
            (ConfidenceDimension::LatestUserGoal, value(0.7)),
            (ConfidenceDimension::TaskObject, value(0.2)),
        ]);
        let claim = bound_claim(ConfidenceClaim::CurrentTask, &dimensions);
        assert_eq!(claim.score, 0.2);
        assert_eq!(claim.label, ConfidenceLabel::Low);
        assert_eq!(
            claim.bounding_dimensions,
            vec![ConfidenceDimension::TaskObject]
        );
    }

    #[test]
    fn strong_identity_does_not_raise_task_confidence() {
        let dimensions = BTreeMap::from([
            (ConfidenceDimension::SurfaceIdentity, value(0.95)),
            (ConfidenceDimension::ActiveWindowOwnership, value(0.9)),
            (ConfidenceDimension::SpeakerAttribution, value(0.1)),
            (ConfidenceDimension::TurnOrder, value(0.1)),
            (ConfidenceDimension::LatestUserGoal, value(0.0)),
            (ConfidenceDimension::TaskObject, value(0.0)),
        ]);
        assert_eq!(
            bound_claim(ConfidenceClaim::SurfaceIdentity, &dimensions).label,
            ConfidenceLabel::High
        );
        assert_eq!(
            bound_claim(ConfidenceClaim::CurrentTask, &dimensions).label,
            ConfidenceLabel::None
        );
    }

    #[test]
    fn strong_task_does_not_raise_target_openability() {
        let mut dimensions = BTreeMap::new();
        for dimension in claim_dependencies(ConfidenceClaim::CurrentTask) {
            dimensions.insert(*dimension, value(0.9));
        }
        dimensions.insert(ConfidenceDimension::TargetIdentity, value(0.0));
        dimensions.insert(ConfidenceDimension::TargetOpenability, value(0.0));
        dimensions.insert(ConfidenceDimension::DirectTargetPolicy, value(0.0));
        assert_eq!(
            bound_claim(ConfidenceClaim::CurrentTask, &dimensions).label,
            ConfidenceLabel::High
        );
        assert_eq!(
            bound_claim(ConfidenceClaim::DirectTarget, &dimensions).label,
            ConfidenceLabel::None
        );
    }

    #[test]
    fn legacy_fields_derive_from_separate_activity_and_target_claims() {
        let mut vector = ContinueConfidenceVector::default();
        for dimension in claim_dependencies(ConfidenceClaim::ActivityRecap) {
            vector.dimensions.insert(*dimension, value(0.8));
        }
        for dimension in claim_dependencies(ConfidenceClaim::DirectTarget) {
            vector.dimensions.insert(*dimension, value(0.0));
        }
        vector.claims = ConfidenceClaim::ALL
            .into_iter()
            .map(|claim| (claim, bound_claim(claim, &vector.dimensions)))
            .collect();
        let legacy = vector.legacy_derivation();
        assert_eq!(legacy.activity_confidence, ActivityConfidence::High);
        assert_eq!(legacy.target_confidence, ActivityConfidence::None);
        assert_eq!(legacy.confidence, 0.8);
    }
}
