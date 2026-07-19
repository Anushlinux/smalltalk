use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs;
use std::path::{Component, Path, PathBuf};

pub(crate) const ACCURACY_FIXTURE_SCHEMA_V1: &str = "smalltalk.continue_accuracy_fixture.v1";
pub(crate) const ACCURACY_EVAL_POLICY_SCHEMA_V1: &str =
    "smalltalk.continue_accuracy_eval_policy.v1";
pub(crate) const ACCURACY_MILESTONE_SCHEMA_V1: &str = "smalltalk.continue_accuracy_milestones.v1";
pub(crate) const AUDIT_IMPORT_CANDIDATE_SCHEMA_V1: &str =
    "smalltalk.continue_accuracy_import_candidate.v1";
pub(crate) const LCA_COMPACT_MODEL_OUTPUT_SCHEMA_V1: &str =
    "smalltalk.lca_02.semantic_probe_response.v3";
pub(crate) const LCA_REPLAY_MANIFEST_SCHEMA_V1: &str = "smalltalk.lca_05.replay_manifest.v1";
pub(crate) const LCA_SEMANTIC_FIELDS_V1: [&str; 6] = [
    "unfinished_task",
    "task_state",
    "resume_point",
    "next_supported_action",
    "completed_context",
    "where_summary",
];

/// Text limits are intentionally small. Fixtures are semantic probes, not capture archives.
pub(crate) const MAX_CASE_ID_CHARS: usize = 96;
pub(crate) const MAX_DESCRIPTION_CHARS: usize = 512;
pub(crate) const MAX_SOURCE_TEXT_CHARS: usize = 512;
pub(crate) const MAX_EXPECTED_TEXT_CHARS: usize = 512;
pub(crate) const MAX_NOTE_TEXT_CHARS: usize = 256;
pub(crate) const MAX_LABEL_CHARS: usize = 128;
const MAX_IMPORT_FILE_BYTES: u64 = 4 * 1024 * 1024;

#[derive(Debug)]
pub(crate) enum AccuracyFixtureError {
    Json(serde_json::Error),
    Io(std::io::Error),
    InvalidSchema {
        expected: &'static str,
        actual: String,
    },
    InvalidContract(String),
    Privacy(Vec<PrivacyLintViolationV1>),
    HoldoutDenied,
}

impl fmt::Display for AccuracyFixtureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json(error) => write!(f, "invalid accuracy JSON: {error}"),
            Self::Io(error) => write!(f, "accuracy fixture I/O failed: {error}"),
            Self::InvalidSchema { expected, actual } => {
                write!(f, "unsupported schema {actual:?}; expected {expected:?}")
            }
            Self::InvalidContract(message) => f.write_str(message),
            Self::Privacy(violations) => {
                write!(
                    f,
                    "fixture privacy lint failed with {} violation(s)",
                    violations.len()
                )
            }
            Self::HoldoutDenied => f.write_str("locked holdout access is denied in this mode"),
        }
    }
}

impl std::error::Error for AccuracyFixtureError {}

impl From<serde_json::Error> for AccuracyFixtureError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}

impl From<std::io::Error> for AccuracyFixtureError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CaptureAccuracyScenarioV1 {
    FreshTaskOnly,
    OldCompletionVisible,
    CausalControlContainment,
    StaleInferredFeedback,
    UnrelatedOpenLoop,
    AllContaminants,
    #[serde(rename = "adjacent_5318852")]
    Adjacent5318852,
    #[serde(rename = "adjacent_5478796")]
    Adjacent5478796,
    #[serde(rename = "lca_05cd_product_need_review")]
    Lca05cdProductNeedReview,
    #[serde(rename = "lca_0d1c_visual_cue_request")]
    Lca0d1cVisualCueRequest,
    #[serde(rename = "lca_0056_visual_cue_verification")]
    Lca0056VisualCueVerification,
    #[serde(rename = "lca_0e34_unsent_regression_draft")]
    Lca0e34UnsentRegressionDraft,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub(crate) enum FixturePartitionV1 {
    Development,
    Validation,
    LockedHoldout,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum InjectionBoundaryV1 {
    CaptureRecords,
    FrameTextResolution,
    HistoricalState,
    SemanticCheckpoint,
    ModelTransport,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PrivacyReviewStatusV1 {
    Pending,
    Approved,
    Rejected,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct PrivacyReviewV1 {
    pub(crate) status: PrivacyReviewStatusV1,
    pub(crate) reviewed_at_ms: Option<i64>,
    pub(crate) reviewer_role: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum FixtureTextStorageClassV1 {
    Synthetic,
    DerivedRedacted,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum FixtureTextPurposeV1 {
    SourceSemanticText,
    ExpectedSemanticText,
    Note,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct FixtureTextV1 {
    pub(crate) text: String,
    pub(crate) storage_class: FixtureTextStorageClassV1,
    pub(crate) purpose: FixtureTextPurposeV1,
    #[serde(default)]
    pub(crate) human_privacy_approved: bool,
    pub(crate) source_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum FixtureScalarV1 {
    Label { value: String },
    Text { value: FixtureTextV1 },
    Integer { value: i64 },
    Number { value: f64 },
    Boolean { value: bool },
    Null,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct FixtureBoundsV1 {
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) width: f64,
    pub(crate) height: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct FixtureSourceRecordV1 {
    pub(crate) record_id: String,
    pub(crate) frame_id: Option<String>,
    pub(crate) observed_at_ms: i64,
    pub(crate) parent_record_id: Option<String>,
    pub(crate) source_role: Option<String>,
    pub(crate) source_order: Option<i64>,
    pub(crate) bounds: Option<FixtureBoundsV1>,
    pub(crate) owner_id_hash: Option<String>,
    pub(crate) confidence: Option<f64>,
    pub(crate) text: Option<FixtureTextV1>,
    #[serde(default)]
    pub(crate) metadata: BTreeMap<String, FixtureScalarV1>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct RedactedSourceRecordsV1 {
    #[serde(default)]
    pub(crate) frames: Vec<FixtureSourceRecordV1>,
    #[serde(default)]
    pub(crate) ax_nodes: Vec<FixtureSourceRecordV1>,
    #[serde(default)]
    pub(crate) ocr_spans: Vec<FixtureSourceRecordV1>,
    #[serde(default)]
    pub(crate) content_units: Vec<FixtureSourceRecordV1>,
    #[serde(default)]
    pub(crate) frame_text_resolution: Vec<FixtureSourceRecordV1>,
    #[serde(default)]
    pub(crate) app_window_context: Vec<FixtureSourceRecordV1>,
    #[serde(default)]
    pub(crate) ui_events: Vec<FixtureSourceRecordV1>,
    #[serde(default)]
    pub(crate) transitions: Vec<FixtureSourceRecordV1>,
    #[serde(default)]
    pub(crate) typing_metadata: Vec<FixtureSourceRecordV1>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct HistoricalRecordV1 {
    pub(crate) record_id: String,
    pub(crate) occurred_at_ms: i64,
    pub(crate) injection_boundary: InjectionBoundaryV1,
    #[serde(default)]
    pub(crate) fields: BTreeMap<String, FixtureScalarV1>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct InjectedHistoricalStateV1 {
    #[serde(default)]
    pub(crate) feedback_events: Vec<HistoricalRecordV1>,
    #[serde(default)]
    pub(crate) branch_contexts: Vec<HistoricalRecordV1>,
    #[serde(default)]
    pub(crate) workstreams: Vec<HistoricalRecordV1>,
    #[serde(default)]
    pub(crate) open_loops: Vec<HistoricalRecordV1>,
    #[serde(default)]
    pub(crate) memory_cells: Vec<HistoricalRecordV1>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AccuracyCheckpointV1 {
    ResolvedText,
    RegionRoles,
    ConversationalRoles,
    OrderedTurnSpans,
    LatestTaskTurn,
    TaskAction,
    SemanticDelta,
    EligibleFeedback,
    SelectedWorkstream,
    EligibleOpenLoop,
    PrimaryRecapSegment,
    LocalRecap,
    ValidatedRecap,
    PublicTarget,
    ProductAnswer,
    CurrentSurface,
    SelectedCandidate,
    TaskRelevantEvidencePacket,
    CompactSemanticRequest,
    CompactSemanticOutput,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ExpectedCheckpointStatusV1 {
    Expected,
    ExpectedAbstention,
    NotImplemented,
    Missing,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum FixtureProbeTaskStateV1 {
    Active,
    WaitingForResult,
    NeedsUserVerification,
    Blocked,
    Superseded,
    Completed,
    Unclear,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum FixtureProbeResolutionStatusV1 {
    Resolved,
    PartlyResolved,
    Unresolved,
    Refused,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum FixtureProbeFieldVerifierResultV1 {
    Pending,
    Admitted,
    Rejected,
    NotProposed,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum FixtureProbeSurfaceRoleV1 {
    PrimaryWork,
    SupportingWork,
    DetourOrUnrelated,
    Unclear,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct FixtureProbeVisitRoleV1 {
    pub(crate) role: FixtureProbeSurfaceRoleV1,
    pub(crate) confidence: f64,
    #[serde(default)]
    pub(crate) support_slots: Vec<String>,
    pub(crate) relationship_to_primary_task: String,
}

/// A synthetic provider response used only at the deterministic transport seam.
/// It mirrors the LCA-02 response contract, but carries no provider identifiers,
/// raw captures, paths, or URLs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct FixtureCompactModelOutputV1 {
    pub(crate) schema: String,
    pub(crate) unfinished_task: Option<String>,
    pub(crate) task_state: FixtureProbeTaskStateV1,
    pub(crate) resume_point: Option<String>,
    pub(crate) next_supported_action: Option<String>,
    pub(crate) completed_context: Option<String>,
    pub(crate) where_summary: Option<String>,
    #[serde(default)]
    pub(crate) visit_roles: BTreeMap<String, FixtureProbeVisitRoleV1>,
    pub(crate) support_slots_by_field: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    pub(crate) missing_evidence: Vec<String>,
    pub(crate) missing_evidence_by_field: BTreeMap<String, Vec<String>>,
    pub(crate) confidence_by_field: BTreeMap<String, f64>,
    pub(crate) verifier_result_by_field: BTreeMap<String, FixtureProbeFieldVerifierResultV1>,
    pub(crate) status: FixtureProbeResolutionStatusV1,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExpectedCheckpointV1 {
    pub(crate) checkpoint: AccuracyCheckpointV1,
    pub(crate) status: ExpectedCheckpointStatusV1,
    #[serde(default)]
    pub(crate) slots: BTreeMap<String, FixtureScalarV1>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ForbiddenClaimV1 {
    pub(crate) term: FixtureTextV1,
    #[serde(default)]
    pub(crate) checkpoints: Vec<AccuracyCheckpointV1>,
    #[serde(default = "default_true")]
    pub(crate) primary_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct AllowedUncertaintyV1 {
    pub(crate) checkpoint: AccuracyCheckpointV1,
    pub(crate) slot: String,
    #[serde(default)]
    pub(crate) allowed_labels: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExpectedModelParityV1 {
    pub(crate) required: bool,
    #[serde(default)]
    pub(crate) identity_slots: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ContinueAccuracyFixtureV1 {
    pub(crate) schema: String,
    pub(crate) case_id: String,
    pub(crate) scenario: CaptureAccuracyScenarioV1,
    pub(crate) description: String,
    pub(crate) privacy_review: PrivacyReviewV1,
    pub(crate) fixture_partition: FixturePartitionV1,
    pub(crate) injection_boundary: InjectionBoundaryV1,
    pub(crate) redacted_source_records: RedactedSourceRecordsV1,
    pub(crate) injected_historical_state: InjectedHistoricalStateV1,
    #[serde(default)]
    pub(crate) expected_checkpoints: Vec<ExpectedCheckpointV1>,
    #[serde(default)]
    pub(crate) forbidden_claims: Vec<ForbiddenClaimV1>,
    #[serde(default)]
    pub(crate) allowed_uncertainty: Vec<AllowedUncertaintyV1>,
    #[serde(default)]
    pub(crate) deterministic_model_output: Option<FixtureCompactModelOutputV1>,
    pub(crate) expected_model_parity: ExpectedModelParityV1,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub(crate) enum LcaReplayCaseKindV1 {
    Critical,
    Adversarial,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub(crate) enum LcaResponseVariantV1 {
    ValidResolved,
    ValidPartlyResolved,
    UnsupportedSemanticField,
    WrongTaskRealSlot,
    PriorCompletion,
    GenericNextAction,
    ConfidenceInflating,
    InvalidInlineCitation,
    InvalidStructuredOutput,
    ProviderIncompleteEmpty,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct LcaReplayCaseV1 {
    pub(crate) case_id: String,
    pub(crate) fixture_case_id: String,
    pub(crate) kind: LcaReplayCaseKindV1,
    pub(crate) failure_class: String,
    #[serde(default)]
    pub(crate) source_log_label: Option<String>,
    #[serde(default)]
    pub(crate) source_sha256: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct LcaReplayManifestV1 {
    pub(crate) schema: String,
    #[serde(default)]
    pub(crate) cases: Vec<LcaReplayCaseV1>,
    #[serde(default)]
    pub(crate) response_variants: Vec<LcaResponseVariantV1>,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ConfidenceBoundariesV1 {
    pub(crate) none_max: f64,
    pub(crate) low_max: f64,
    pub(crate) medium_max: f64,
    pub(crate) high_min: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct EvalMinimumSamplesV1 {
    pub(crate) calibration_predictions: usize,
    pub(crate) calibration_cases_per_bin: usize,
    pub(crate) worst_slice_cases: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct EvalBaselineV1 {
    pub(crate) model_off_p95_ms: f64,
    pub(crate) sample_count: usize,
    pub(crate) warmup_count: usize,
    pub(crate) environment_label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ContinueAccuracyEvalPolicyV1 {
    pub(crate) schema: String,
    pub(crate) policy_version: String,
    #[serde(default)]
    pub(crate) semantic_slot_rubric: BTreeMap<String, String>,
    #[serde(default)]
    pub(crate) materially_wrong_slots: Vec<String>,
    pub(crate) confidence_boundaries: ConfidenceBoundariesV1,
    pub(crate) wrong_confident_threshold: f64,
    pub(crate) minimum_samples: EvalMinimumSamplesV1,
    #[serde(default)]
    pub(crate) partitions: Vec<FixturePartitionV1>,
    #[serde(default)]
    pub(crate) aggregation_modes: Vec<String>,
    pub(crate) calibration_ece_max: f64,
    pub(crate) baseline: EvalBaselineV1,
    pub(crate) model_off_p95_regression_factor: f64,
    pub(crate) absolute_model_off_p95_budget_ms: Option<f64>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub(crate) enum P6PhaseV1 {
    P6_01,
    P6_02,
    P6_03,
    P6_04,
    P6_05,
    P6_06,
    P6_07,
    P6_08,
    P6_09,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum MilestoneExpectedStatusV1 {
    Pass,
    KnownFailure,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct MilestoneCaseV1 {
    pub(crate) case_id: String,
    pub(crate) expected_status: MilestoneExpectedStatusV1,
    pub(crate) expected_first_divergence: Option<AccuracyCheckpointV1>,
    pub(crate) must_pass_by_phase: P6PhaseV1,
    pub(crate) owner_checkpoint: AccuracyCheckpointV1,
    #[serde(default)]
    pub(crate) expires_at_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct AccuracyMilestoneManifestV1 {
    pub(crate) schema: String,
    pub(crate) manifest_version: String,
    #[serde(default)]
    pub(crate) cases: Vec<MilestoneCaseV1>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ObservedMilestoneStatusV1 {
    Pass,
    Fail,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ObservedMilestoneV1 {
    pub(crate) case_id: String,
    pub(crate) status: ObservedMilestoneStatusV1,
    pub(crate) first_divergence: Option<AccuracyCheckpointV1>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MilestoneViolationV1 {
    pub(crate) case_id: String,
    pub(crate) reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HoldoutAccessModeV1 {
    Development,
    Validation,
    ReleaseEvaluation,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AuditSourceKindV1 {
    FrameTextResolution,
    LinkedTaskActions,
    SurfaceSnapshots,
    FeedbackEvents,
    BranchContexts,
    OpenLoops,
    StitchedTimeline,
    WorkLabels,
    ModelPack,
    ModelValidation,
    ContinueDecisionResult,
}

#[derive(Debug, Clone)]
pub(crate) struct LocalAuditSourceRequestV1 {
    pub(crate) kind: AuditSourceKindV1,
    pub(crate) relative_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct AuditImportSourceCandidateV1 {
    pub(crate) kind: AuditSourceKindV1,
    pub(crate) path_hash: String,
    pub(crate) content_hash: String,
    pub(crate) byte_count: u64,
    pub(crate) line_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct AuditImportCandidateV1 {
    pub(crate) schema: String,
    pub(crate) case_id: String,
    pub(crate) review_required: bool,
    pub(crate) contains_private_text: bool,
    #[serde(default)]
    pub(crate) sources: Vec<AuditImportSourceCandidateV1>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PrivacyLintCodeV1 {
    OversizedText,
    HomeDirectoryPath,
    RawPath,
    RawUrl,
    UrlQueryString,
    ScreenshotPath,
    SecretLikeToken,
    LongOpaqueToken,
    InvalidHash,
    DerivedTextNotApproved,
    DerivedTextMissingHash,
    InvalidMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct PrivacyLintViolationV1 {
    pub(crate) path: String,
    pub(crate) code: PrivacyLintCodeV1,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct PrivacyLintReportV1 {
    pub(crate) passed: bool,
    #[serde(default)]
    pub(crate) violations: Vec<PrivacyLintViolationV1>,
}

pub(crate) fn parse_accuracy_fixture_json(
    bytes: &[u8],
) -> Result<ContinueAccuracyFixtureV1, AccuracyFixtureError> {
    let fixture: ContinueAccuracyFixtureV1 = serde_json::from_slice(bytes)?;
    require_schema(&fixture.schema, ACCURACY_FIXTURE_SCHEMA_V1)?;
    validate_fixture_contract(&fixture)?;
    Ok(fixture)
}

pub(crate) fn parse_lca_replay_manifest_json(
    bytes: &[u8],
) -> Result<LcaReplayManifestV1, AccuracyFixtureError> {
    let manifest: LcaReplayManifestV1 = serde_json::from_slice(bytes)?;
    require_schema(&manifest.schema, LCA_REPLAY_MANIFEST_SCHEMA_V1)?;
    let critical = manifest
        .cases
        .iter()
        .filter(|case| case.kind == LcaReplayCaseKindV1::Critical)
        .count();
    let adversarial = manifest
        .cases
        .iter()
        .filter(|case| case.kind == LcaReplayCaseKindV1::Adversarial)
        .count();
    if critical != 4 || adversarial != 4 {
        return Err(AccuracyFixtureError::InvalidContract(format!(
            "LCA replay manifest requires 4 critical and 4 adversarial cases; found {critical} and {adversarial}"
        )));
    }
    let case_ids = manifest
        .cases
        .iter()
        .map(|case| case.case_id.as_str())
        .collect::<BTreeSet<_>>();
    let fixture_ids = manifest
        .cases
        .iter()
        .map(|case| case.fixture_case_id.as_str())
        .collect::<BTreeSet<_>>();
    if case_ids.len() != 8 || fixture_ids.len() != 8 {
        return Err(AccuracyFixtureError::InvalidContract(
            "LCA replay case and fixture references must be unique".to_string(),
        ));
    }
    for case in &manifest.cases {
        validate_short_identifier("lca case_id", &case.case_id, MAX_CASE_ID_CHARS)?;
        validate_short_identifier(
            "lca fixture_case_id",
            &case.fixture_case_id,
            MAX_CASE_ID_CHARS,
        )?;
        validate_short_identifier("lca failure_class", &case.failure_class, MAX_LABEL_CHARS)?;
        match case.kind {
            LcaReplayCaseKindV1::Critical => {
                let label = case.source_log_label.as_deref().ok_or_else(|| {
                    AccuracyFixtureError::InvalidContract(format!(
                        "critical LCA replay case {} requires source_log_label",
                        case.case_id
                    ))
                })?;
                validate_short_identifier("lca source_log_label", label, MAX_LABEL_CHARS)?;
                let hash = case.source_sha256.as_deref().ok_or_else(|| {
                    AccuracyFixtureError::InvalidContract(format!(
                        "critical LCA replay case {} requires source_sha256",
                        case.case_id
                    ))
                })?;
                if hash.len() != 64 || !hash.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                    return Err(AccuracyFixtureError::InvalidContract(format!(
                        "critical LCA replay case {} has an invalid source_sha256",
                        case.case_id
                    )));
                }
            }
            LcaReplayCaseKindV1::Adversarial => {
                if case.source_log_label.is_some() || case.source_sha256.is_some() {
                    return Err(AccuracyFixtureError::InvalidContract(format!(
                        "adversarial LCA replay case {} must not claim source-log provenance",
                        case.case_id
                    )));
                }
            }
        }
    }
    let required_variants = [
        LcaResponseVariantV1::ValidResolved,
        LcaResponseVariantV1::ValidPartlyResolved,
        LcaResponseVariantV1::UnsupportedSemanticField,
        LcaResponseVariantV1::WrongTaskRealSlot,
        LcaResponseVariantV1::PriorCompletion,
        LcaResponseVariantV1::GenericNextAction,
        LcaResponseVariantV1::ConfidenceInflating,
        LcaResponseVariantV1::InvalidInlineCitation,
        LcaResponseVariantV1::InvalidStructuredOutput,
        LcaResponseVariantV1::ProviderIncompleteEmpty,
    ]
    .into_iter()
    .collect::<BTreeSet<_>>();
    let actual_variants = manifest
        .response_variants
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    if actual_variants != required_variants || manifest.response_variants.len() != 10 {
        return Err(AccuracyFixtureError::InvalidContract(
            "LCA replay manifest must enumerate each required response variant exactly once"
                .to_string(),
        ));
    }
    Ok(manifest)
}

/// Parse a fixture and enforce partition access in one operation. Evaluators should prefer this
/// entry point so a locked holdout cannot be loaded and then accidentally evaluated in a
/// development or validation run.
pub(crate) fn parse_accuracy_fixture_json_for_access(
    bytes: &[u8],
    mode: HoldoutAccessModeV1,
) -> Result<ContinueAccuracyFixtureV1, AccuracyFixtureError> {
    let fixture = parse_accuracy_fixture_json(bytes)?;
    enforce_holdout_access(fixture.fixture_partition, mode)?;
    Ok(fixture)
}

pub(crate) fn parse_eval_policy_json(
    bytes: &[u8],
) -> Result<ContinueAccuracyEvalPolicyV1, AccuracyFixtureError> {
    let policy: ContinueAccuracyEvalPolicyV1 = serde_json::from_slice(bytes)?;
    require_schema(&policy.schema, ACCURACY_EVAL_POLICY_SCHEMA_V1)?;
    validate_eval_policy(&policy)?;
    Ok(policy)
}

pub(crate) fn parse_milestone_manifest_json(
    bytes: &[u8],
) -> Result<AccuracyMilestoneManifestV1, AccuracyFixtureError> {
    let manifest: AccuracyMilestoneManifestV1 = serde_json::from_slice(bytes)?;
    require_schema(&manifest.schema, ACCURACY_MILESTONE_SCHEMA_V1)?;
    validate_short_identifier(
        "manifest_version",
        &manifest.manifest_version,
        MAX_LABEL_CHARS,
    )?;
    let mut ids = BTreeSet::new();
    for case in &manifest.cases {
        validate_short_identifier("milestone.case_id", &case.case_id, MAX_CASE_ID_CHARS)?;
        if !ids.insert(&case.case_id) {
            return Err(AccuracyFixtureError::InvalidContract(format!(
                "duplicate milestone case_id {:?}",
                case.case_id
            )));
        }
        if case.expires_at_ms.is_some_and(|value| value <= 0) {
            return Err(AccuracyFixtureError::InvalidContract(format!(
                "milestone {:?} has a non-positive expires_at_ms",
                case.case_id
            )));
        }
    }
    Ok(manifest)
}

pub(crate) fn parse_audit_import_candidate_json(
    bytes: &[u8],
) -> Result<AuditImportCandidateV1, AccuracyFixtureError> {
    let candidate: AuditImportCandidateV1 = serde_json::from_slice(bytes)?;
    require_schema(&candidate.schema, AUDIT_IMPORT_CANDIDATE_SCHEMA_V1)?;
    validate_short_identifier("case_id", &candidate.case_id, MAX_CASE_ID_CHARS)?;
    if !candidate.review_required || candidate.contains_private_text {
        return Err(AccuracyFixtureError::InvalidContract(
            "import candidates must require review and contain no private text".to_string(),
        ));
    }
    for source in &candidate.sources {
        validate_hash("import source path_hash", &source.path_hash)?;
        validate_hash("import source content_hash", &source.content_hash)?;
    }
    Ok(candidate)
}

fn require_schema(actual: &str, expected: &'static str) -> Result<(), AccuracyFixtureError> {
    if actual == expected {
        Ok(())
    } else {
        Err(AccuracyFixtureError::InvalidSchema {
            expected,
            actual: actual.to_string(),
        })
    }
}

fn validate_fixture_contract(
    fixture: &ContinueAccuracyFixtureV1,
) -> Result<(), AccuracyFixtureError> {
    validate_short_identifier("case_id", &fixture.case_id, MAX_CASE_ID_CHARS)?;
    if fixture.privacy_review.status != PrivacyReviewStatusV1::Approved
        || fixture.privacy_review.reviewed_at_ms.is_none()
        || fixture
            .privacy_review
            .reviewer_role
            .as_deref()
            .is_none_or(str::is_empty)
    {
        return Err(AccuracyFixtureError::InvalidContract(
            "committed fixtures require completed human privacy review metadata".to_string(),
        ));
    }
    if fixture.description.chars().count() > MAX_DESCRIPTION_CHARS {
        return Err(AccuracyFixtureError::InvalidContract(
            "fixture description exceeds its documented cap".to_string(),
        ));
    }
    if fixture.injection_boundary == InjectionBoundaryV1::CaptureRecords
        && !fixture
            .redacted_source_records
            .frame_text_resolution
            .is_empty()
    {
        return Err(AccuracyFixtureError::InvalidContract(
            "capture-record replay cannot inject pre-derived frame_text_resolution rows"
                .to_string(),
        ));
    }
    let mut checkpoints = BTreeSet::new();
    for checkpoint in &fixture.expected_checkpoints {
        if !checkpoints.insert(checkpoint.checkpoint) {
            return Err(AccuracyFixtureError::InvalidContract(format!(
                "duplicate checkpoint {:?}",
                checkpoint.checkpoint
            )));
        }
        let slots_must_be_empty = matches!(
            checkpoint.status,
            ExpectedCheckpointStatusV1::NotImplemented | ExpectedCheckpointStatusV1::Missing
        );
        if slots_must_be_empty != checkpoint.slots.is_empty() {
            return Err(AccuracyFixtureError::InvalidContract(format!(
                "checkpoint {:?} must {} typed slots",
                checkpoint.checkpoint,
                if slots_must_be_empty {
                    "not contain"
                } else {
                    "contain"
                }
            )));
        }
    }
    let is_lca = matches!(
        fixture.scenario,
        CaptureAccuracyScenarioV1::Lca05cdProductNeedReview
            | CaptureAccuracyScenarioV1::Lca0d1cVisualCueRequest
            | CaptureAccuracyScenarioV1::Lca0056VisualCueVerification
            | CaptureAccuracyScenarioV1::Lca0e34UnsentRegressionDraft
    );
    if is_lca != fixture.deterministic_model_output.is_some() {
        return Err(AccuracyFixtureError::InvalidContract(
            "exactly the four LCA fixtures must provide deterministic_model_output".to_string(),
        ));
    }
    if let Some(output) = &fixture.deterministic_model_output {
        validate_compact_model_output(output)?;
    }
    let privacy = lint_accuracy_fixture(fixture);
    if !privacy.passed {
        return Err(AccuracyFixtureError::Privacy(privacy.violations));
    }
    Ok(())
}

fn validate_compact_model_output(
    output: &FixtureCompactModelOutputV1,
) -> Result<(), AccuracyFixtureError> {
    if output.schema != LCA_COMPACT_MODEL_OUTPUT_SCHEMA_V1 {
        return Err(AccuracyFixtureError::InvalidContract(format!(
            "deterministic_model_output uses unsupported schema {:?}",
            output.schema
        )));
    }
    let expected_fields = LCA_SEMANTIC_FIELDS_V1.into_iter().collect::<BTreeSet<_>>();
    for (name, actual) in [
        (
            "support_slots_by_field",
            output
                .support_slots_by_field
                .keys()
                .map(String::as_str)
                .collect::<BTreeSet<_>>(),
        ),
        (
            "missing_evidence_by_field",
            output
                .missing_evidence_by_field
                .keys()
                .map(String::as_str)
                .collect::<BTreeSet<_>>(),
        ),
        (
            "confidence_by_field",
            output
                .confidence_by_field
                .keys()
                .map(String::as_str)
                .collect::<BTreeSet<_>>(),
        ),
        (
            "verifier_result_by_field",
            output
                .verifier_result_by_field
                .keys()
                .map(String::as_str)
                .collect::<BTreeSet<_>>(),
        ),
    ] {
        if actual != expected_fields {
            return Err(AccuracyFixtureError::InvalidContract(format!(
                "deterministic_model_output.{name} must contain the exact LCA semantic field set"
            )));
        }
    }
    for (field, value, cap) in [
        ("unfinished_task", output.unfinished_task.as_deref(), 220),
        ("resume_point", output.resume_point.as_deref(), 260),
        (
            "next_supported_action",
            output.next_supported_action.as_deref(),
            180,
        ),
        (
            "completed_context",
            output.completed_context.as_deref(),
            180,
        ),
        ("where_summary", output.where_summary.as_deref(), 220),
    ] {
        if value.is_some_and(|value| value.trim().is_empty() || value.chars().count() > cap) {
            return Err(AccuracyFixtureError::InvalidContract(format!(
                "deterministic_model_output.{field} is empty or exceeds its public cap"
            )));
        }
        let supports = &output.support_slots_by_field[field];
        if value.is_some() == supports.is_empty() {
            return Err(AccuracyFixtureError::InvalidContract(format!(
                "deterministic_model_output.{field} must cite support exactly when non-null"
            )));
        }
    }
    if (output.task_state != FixtureProbeTaskStateV1::Unclear)
        == output.support_slots_by_field["task_state"].is_empty()
    {
        return Err(AccuracyFixtureError::InvalidContract(
            "deterministic_model_output.task_state must cite support unless it is unclear"
                .to_string(),
        ));
    }
    if output
        .confidence_by_field
        .values()
        .any(|value| !value.is_finite() || !(0.0..=1.0).contains(value))
    {
        return Err(AccuracyFixtureError::InvalidContract(
            "deterministic_model_output confidences must be finite values in [0, 1]".to_string(),
        ));
    }
    if output
        .verifier_result_by_field
        .values()
        .any(|result| *result != FixtureProbeFieldVerifierResultV1::Pending)
    {
        return Err(AccuracyFixtureError::InvalidContract(
            "fixture provider responses must leave every verifier result pending".to_string(),
        ));
    }
    Ok(())
}

fn validate_eval_policy(policy: &ContinueAccuracyEvalPolicyV1) -> Result<(), AccuracyFixtureError> {
    validate_short_identifier("policy_version", &policy.policy_version, MAX_LABEL_CHARS)?;
    validate_public_safe_string(
        "baseline.environment_label",
        &policy.baseline.environment_label,
        MAX_LABEL_CHARS,
    )?;
    for (slot, rubric) in &policy.semantic_slot_rubric {
        validate_short_identifier("semantic_slot_rubric key", slot, MAX_LABEL_CHARS)?;
        validate_public_safe_string("semantic_slot_rubric value", rubric, MAX_DESCRIPTION_CHARS)?;
    }
    for slot in &policy.materially_wrong_slots {
        validate_short_identifier("materially_wrong_slot", slot, MAX_LABEL_CHARS)?;
    }
    for mode in &policy.aggregation_modes {
        validate_short_identifier("aggregation_mode", mode, MAX_LABEL_CHARS)?;
    }
    let values = [
        policy.confidence_boundaries.none_max,
        policy.confidence_boundaries.low_max,
        policy.confidence_boundaries.medium_max,
        policy.confidence_boundaries.high_min,
        policy.wrong_confident_threshold,
        policy.calibration_ece_max,
    ];
    if values
        .iter()
        .any(|value| !value.is_finite() || !(0.0..=1.0).contains(value))
        || policy.confidence_boundaries.none_max > policy.confidence_boundaries.low_max
        || policy.confidence_boundaries.low_max > policy.confidence_boundaries.medium_max
        || policy.confidence_boundaries.medium_max > policy.confidence_boundaries.high_min
    {
        return Err(AccuracyFixtureError::InvalidContract(
            "confidence and calibration boundaries must be ordered values in [0, 1]".to_string(),
        ));
    }
    if policy.minimum_samples.calibration_predictions == 0
        || policy.minimum_samples.calibration_cases_per_bin == 0
        || policy.minimum_samples.worst_slice_cases == 0
        || policy.baseline.sample_count == 0
        || policy.baseline.model_off_p95_ms <= 0.0
        || policy.model_off_p95_regression_factor < 1.0
        || policy
            .absolute_model_off_p95_budget_ms
            .is_some_and(|value| !value.is_finite() || value <= 0.0)
    {
        return Err(AccuracyFixtureError::InvalidContract(
            "eval policy sample and performance values must be positive".to_string(),
        ));
    }
    let partitions: BTreeSet<_> = policy.partitions.iter().copied().collect();
    if partitions.len() != 3
        || !partitions.contains(&FixturePartitionV1::Development)
        || !partitions.contains(&FixturePartitionV1::Validation)
        || !partitions.contains(&FixturePartitionV1::LockedHoldout)
    {
        return Err(AccuracyFixtureError::InvalidContract(
            "eval policy must freeze development, validation, and locked_holdout partitions"
                .to_string(),
        ));
    }
    Ok(())
}

pub(crate) fn validate_initial_capture_case_set(
    fixtures: &[ContinueAccuracyFixtureV1],
) -> Result<(), AccuracyFixtureError> {
    let ids: BTreeSet<_> = fixtures.iter().map(|fixture| &fixture.case_id).collect();
    if ids.len() != fixtures.len() {
        return Err(AccuracyFixtureError::InvalidContract(
            "accuracy corpus contains duplicate case ids".to_string(),
        ));
    }
    let required_ids = [
        "capture_button_fresh_task_only",
        "capture_button_old_completion_visible",
        "capture_button_stale_inferred_feedback",
        "capture_button_unrelated_open_loop",
        "capture_button_all_contaminants",
        "capture_button_adjacent_before_new_task",
        "capture_button_adjacent_after_support_detour",
    ];
    let missing = required_ids
        .iter()
        .filter(|required| !ids.iter().any(|case_id| case_id.as_str() == **required))
        .copied()
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        return Err(AccuracyFixtureError::InvalidContract(format!(
            "accuracy corpus is missing required initial cases: {}",
            missing.join(",")
        )));
    }
    Ok(())
}

pub(crate) fn enforce_holdout_access(
    partition: FixturePartitionV1,
    mode: HoldoutAccessModeV1,
) -> Result<(), AccuracyFixtureError> {
    if partition == FixturePartitionV1::LockedHoldout
        && mode != HoldoutAccessModeV1::ReleaseEvaluation
    {
        Err(AccuracyFixtureError::HoldoutDenied)
    } else {
        Ok(())
    }
}

pub(crate) fn enforce_milestones(
    manifest: &AccuracyMilestoneManifestV1,
    observed: &[ObservedMilestoneV1],
    current_phase: P6PhaseV1,
    now_ms: i64,
) -> Vec<MilestoneViolationV1> {
    let mut violations = Vec::new();
    let mut observed_by_id = BTreeMap::new();
    for result in observed {
        if observed_by_id
            .insert(result.case_id.as_str(), result)
            .is_some()
        {
            violations.push(MilestoneViolationV1 {
                case_id: result.case_id.clone(),
                reason: "duplicate_observation".to_string(),
            });
        }
    }
    for expected in &manifest.cases {
        let Some(actual) = observed_by_id.get(expected.case_id.as_str()) else {
            violations.push(MilestoneViolationV1 {
                case_id: expected.case_id.clone(),
                reason: "missing_observation".to_string(),
            });
            continue;
        };
        let expected_observed = match expected.expected_status {
            MilestoneExpectedStatusV1::Pass => ObservedMilestoneStatusV1::Pass,
            MilestoneExpectedStatusV1::KnownFailure => ObservedMilestoneStatusV1::Fail,
        };
        if actual.status != expected_observed {
            violations.push(MilestoneViolationV1 {
                case_id: expected.case_id.clone(),
                reason: if actual.status == ObservedMilestoneStatusV1::Pass {
                    "unexpected_improvement_requires_manifest_review".to_string()
                } else {
                    "unexpected_regression".to_string()
                },
            });
        }
        if actual.first_divergence != expected.expected_first_divergence {
            violations.push(MilestoneViolationV1 {
                case_id: expected.case_id.clone(),
                reason: "first_divergence_changed".to_string(),
            });
        }
        if expected.expected_status == MilestoneExpectedStatusV1::KnownFailure
            && (current_phase >= expected.must_pass_by_phase
                || expected
                    .expires_at_ms
                    .is_some_and(|expires| now_ms >= expires))
        {
            violations.push(MilestoneViolationV1 {
                case_id: expected.case_id.clone(),
                reason: "known_failure_marker_expired".to_string(),
            });
        }
    }
    violations
}

pub(crate) fn lint_accuracy_fixture(fixture: &ContinueAccuracyFixtureV1) -> PrivacyLintReportV1 {
    let mut violations = Vec::new();
    lint_metadata_string(
        "case_id",
        &fixture.case_id,
        MAX_CASE_ID_CHARS,
        &mut violations,
    );
    lint_metadata_string(
        "description",
        &fixture.description,
        MAX_DESCRIPTION_CHARS,
        &mut violations,
    );
    if let Some(role) = &fixture.privacy_review.reviewer_role {
        lint_metadata_string(
            "privacy_review.reviewer_role",
            role,
            MAX_LABEL_CHARS,
            &mut violations,
        );
    }
    for (path, record) in source_records_with_paths(&fixture.redacted_source_records) {
        lint_source_record(&path, record, fixture, &mut violations);
    }
    for (group, records) in historical_records_with_groups(&fixture.injected_historical_state) {
        for (index, record) in records.iter().enumerate() {
            let path = format!("injected_historical_state.{group}[{index}]");
            lint_metadata_string(
                &format!("{path}.record_id"),
                &record.record_id,
                MAX_LABEL_CHARS,
                &mut violations,
            );
            lint_scalar_map(&path, &record.fields, fixture, &mut violations);
        }
    }
    for (index, checkpoint) in fixture.expected_checkpoints.iter().enumerate() {
        lint_scalar_map(
            &format!("expected_checkpoints[{index}].slots"),
            &checkpoint.slots,
            fixture,
            &mut violations,
        );
    }
    for (index, claim) in fixture.forbidden_claims.iter().enumerate() {
        lint_fixture_text(
            &format!("forbidden_claims[{index}].term"),
            &claim.term,
            fixture,
            &mut violations,
        );
    }
    for (index, uncertainty) in fixture.allowed_uncertainty.iter().enumerate() {
        lint_metadata_string(
            &format!("allowed_uncertainty[{index}].slot"),
            &uncertainty.slot,
            MAX_LABEL_CHARS,
            &mut violations,
        );
        for (label_index, label) in uncertainty.allowed_labels.iter().enumerate() {
            lint_metadata_string(
                &format!("allowed_uncertainty[{index}].allowed_labels[{label_index}]"),
                label,
                MAX_LABEL_CHARS,
                &mut violations,
            );
        }
    }
    if let Some(output) = &fixture.deterministic_model_output {
        lint_metadata_string(
            "deterministic_model_output.schema",
            &output.schema,
            MAX_LABEL_CHARS,
            &mut violations,
        );
        for (field, value, cap) in [
            ("unfinished_task", output.unfinished_task.as_deref(), 220),
            ("resume_point", output.resume_point.as_deref(), 260),
            (
                "next_supported_action",
                output.next_supported_action.as_deref(),
                180,
            ),
            (
                "completed_context",
                output.completed_context.as_deref(),
                180,
            ),
            ("where_summary", output.where_summary.as_deref(), 220),
        ] {
            if let Some(value) = value {
                lint_sensitive_string(
                    &format!("deterministic_model_output.{field}"),
                    value,
                    cap,
                    &mut violations,
                );
            }
        }
        for (field, supports) in &output.support_slots_by_field {
            lint_metadata_string(
                "deterministic_model_output.support_slots_by_field.key",
                field,
                MAX_LABEL_CHARS,
                &mut violations,
            );
            for (index, support) in supports.iter().enumerate() {
                lint_metadata_string(
                    &format!("deterministic_model_output.support_slots_by_field.{field}[{index}]"),
                    support,
                    MAX_LABEL_CHARS,
                    &mut violations,
                );
            }
        }
        for (field, notes) in &output.missing_evidence_by_field {
            lint_metadata_string(
                "deterministic_model_output.missing_evidence_by_field.key",
                field,
                MAX_LABEL_CHARS,
                &mut violations,
            );
            for (index, note) in notes.iter().enumerate() {
                lint_sensitive_string(
                    &format!(
                        "deterministic_model_output.missing_evidence_by_field.{field}[{index}]"
                    ),
                    note,
                    MAX_NOTE_TEXT_CHARS,
                    &mut violations,
                );
            }
        }
        for (index, note) in output.missing_evidence.iter().enumerate() {
            lint_sensitive_string(
                &format!("deterministic_model_output.missing_evidence[{index}]"),
                note,
                MAX_NOTE_TEXT_CHARS,
                &mut violations,
            );
        }
        for (visit_id, visit) in &output.visit_roles {
            lint_metadata_string(
                "deterministic_model_output.visit_roles.key",
                visit_id,
                MAX_LABEL_CHARS,
                &mut violations,
            );
            lint_sensitive_string(
                &format!(
                    "deterministic_model_output.visit_roles.{visit_id}.relationship_to_primary_task"
                ),
                &visit.relationship_to_primary_task,
                MAX_NOTE_TEXT_CHARS,
                &mut violations,
            );
            for (index, support) in visit.support_slots.iter().enumerate() {
                lint_metadata_string(
                    &format!(
                        "deterministic_model_output.visit_roles.{visit_id}.support_slots[{index}]"
                    ),
                    support,
                    MAX_LABEL_CHARS,
                    &mut violations,
                );
            }
        }
    }
    for (index, slot) in fixture
        .expected_model_parity
        .identity_slots
        .iter()
        .enumerate()
    {
        lint_metadata_string(
            &format!("expected_model_parity.identity_slots[{index}]"),
            slot,
            MAX_LABEL_CHARS,
            &mut violations,
        );
    }
    PrivacyLintReportV1 {
        passed: violations.is_empty(),
        violations,
    }
}

fn lint_source_record(
    path: &str,
    record: &FixtureSourceRecordV1,
    fixture: &ContinueAccuracyFixtureV1,
    violations: &mut Vec<PrivacyLintViolationV1>,
) {
    for (name, value) in [
        ("record_id", Some(&record.record_id)),
        ("frame_id", record.frame_id.as_ref()),
        ("parent_record_id", record.parent_record_id.as_ref()),
        ("source_role", record.source_role.as_ref()),
    ] {
        if let Some(value) = value {
            lint_metadata_string(
                &format!("{path}.{name}"),
                value,
                MAX_LABEL_CHARS,
                violations,
            );
        }
    }
    if let Some(hash) = &record.owner_id_hash {
        lint_hash(&format!("{path}.owner_id_hash"), hash, violations);
    }
    if let Some(text) = &record.text {
        lint_fixture_text(&format!("{path}.text"), text, fixture, violations);
    }
    lint_scalar_map(
        &format!("{path}.metadata"),
        &record.metadata,
        fixture,
        violations,
    );
}

fn lint_scalar_map(
    path: &str,
    fields: &BTreeMap<String, FixtureScalarV1>,
    fixture: &ContinueAccuracyFixtureV1,
    violations: &mut Vec<PrivacyLintViolationV1>,
) {
    for (key, value) in fields {
        lint_metadata_string(&format!("{path}.key"), key, MAX_LABEL_CHARS, violations);
        match value {
            FixtureScalarV1::Label { value } => {
                lint_metadata_string(&format!("{path}.{key}"), value, MAX_LABEL_CHARS, violations)
            }
            FixtureScalarV1::Text { value } => {
                lint_fixture_text(&format!("{path}.{key}"), value, fixture, violations)
            }
            _ => {}
        }
    }
}

fn lint_fixture_text(
    path: &str,
    text: &FixtureTextV1,
    fixture: &ContinueAccuracyFixtureV1,
    violations: &mut Vec<PrivacyLintViolationV1>,
) {
    let cap = match text.purpose {
        FixtureTextPurposeV1::SourceSemanticText => MAX_SOURCE_TEXT_CHARS,
        FixtureTextPurposeV1::ExpectedSemanticText => MAX_EXPECTED_TEXT_CHARS,
        FixtureTextPurposeV1::Note => MAX_NOTE_TEXT_CHARS,
    };
    lint_sensitive_string(path, &text.text, cap, violations);
    if text.storage_class == FixtureTextStorageClassV1::DerivedRedacted {
        if !text.human_privacy_approved
            || fixture.privacy_review.status != PrivacyReviewStatusV1::Approved
        {
            violations.push(PrivacyLintViolationV1 {
                path: path.to_string(),
                code: PrivacyLintCodeV1::DerivedTextNotApproved,
            });
        }
        match &text.source_hash {
            Some(hash) => lint_hash(&format!("{path}.source_hash"), hash, violations),
            None => violations.push(PrivacyLintViolationV1 {
                path: path.to_string(),
                code: PrivacyLintCodeV1::DerivedTextMissingHash,
            }),
        }
    } else if let Some(hash) = &text.source_hash {
        lint_hash(&format!("{path}.source_hash"), hash, violations);
    }
}

fn lint_metadata_string(
    path: &str,
    value: &str,
    cap: usize,
    violations: &mut Vec<PrivacyLintViolationV1>,
) {
    lint_sensitive_string(path, value, cap, violations);
}

fn lint_sensitive_string(
    path: &str,
    value: &str,
    cap: usize,
    violations: &mut Vec<PrivacyLintViolationV1>,
) {
    let lower = value.to_ascii_lowercase();
    if value.chars().count() > cap {
        push_violation(path, PrivacyLintCodeV1::OversizedText, violations);
    }
    if lower.contains("/users/")
        || lower.contains("/home/")
        || lower.contains("c:\\users\\")
        || lower.starts_with("~/")
    {
        push_violation(path, PrivacyLintCodeV1::HomeDirectoryPath, violations);
    }
    if lower.contains("http://") || lower.contains("https://") {
        push_violation(path, PrivacyLintCodeV1::RawUrl, violations);
        if value
            .split_whitespace()
            .any(|part| part.contains("://") && part.contains('?'))
        {
            push_violation(path, PrivacyLintCodeV1::UrlQueryString, violations);
        }
    }
    if looks_like_path(value) {
        push_violation(path, PrivacyLintCodeV1::RawPath, violations);
    }
    if [".png", ".jpg", ".jpeg", ".heic", ".webp", ".tiff"]
        .iter()
        .any(|suffix| lower.contains(suffix))
    {
        push_violation(path, PrivacyLintCodeV1::ScreenshotPath, violations);
    }
    if [
        "sk-",
        "ghp_",
        "github_pat_",
        "xoxb-",
        "xoxp-",
        "bearer ",
        "aiza",
    ]
    .iter()
    .any(|prefix| lower.contains(prefix))
    {
        push_violation(path, PrivacyLintCodeV1::SecretLikeToken, violations);
    }
    if value
        .split(|character: char| {
            !character.is_ascii_alphanumeric() && character != '_' && character != '-'
        })
        .any(looks_like_opaque_token)
    {
        push_violation(path, PrivacyLintCodeV1::LongOpaqueToken, violations);
    }
}

fn looks_like_path(value: &str) -> bool {
    value.starts_with('/')
        || value.starts_with("./")
        || value.starts_with("../")
        || value.contains("\\Users\\")
        || value.contains("/Documents/")
        || value.contains("/Library/")
}

fn looks_like_opaque_token(value: &str) -> bool {
    value.len() >= 32
        && value.bytes().any(|byte| byte.is_ascii_alphabetic())
        && value.bytes().any(|byte| byte.is_ascii_digit())
}

fn lint_hash(path: &str, hash: &str, violations: &mut Vec<PrivacyLintViolationV1>) {
    if hash == "fixture-owner" {
        return;
    }
    if hash.len() != 64
        || !hash
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        push_violation(path, PrivacyLintCodeV1::InvalidHash, violations);
    }
}

fn push_violation(
    path: &str,
    code: PrivacyLintCodeV1,
    violations: &mut Vec<PrivacyLintViolationV1>,
) {
    violations.push(PrivacyLintViolationV1 {
        path: path.to_string(),
        code,
    });
}

fn source_records_with_paths(
    source: &RedactedSourceRecordsV1,
) -> Vec<(String, &FixtureSourceRecordV1)> {
    let groups: [(&str, &Vec<FixtureSourceRecordV1>); 9] = [
        ("frames", &source.frames),
        ("ax_nodes", &source.ax_nodes),
        ("ocr_spans", &source.ocr_spans),
        ("content_units", &source.content_units),
        ("frame_text_resolution", &source.frame_text_resolution),
        ("app_window_context", &source.app_window_context),
        ("ui_events", &source.ui_events),
        ("transitions", &source.transitions),
        ("typing_metadata", &source.typing_metadata),
    ];
    groups
        .into_iter()
        .flat_map(|(group, records)| {
            records.iter().enumerate().map(move |(index, record)| {
                (format!("redacted_source_records.{group}[{index}]"), record)
            })
        })
        .collect()
}

fn historical_records_with_groups(
    state: &InjectedHistoricalStateV1,
) -> [(&'static str, &Vec<HistoricalRecordV1>); 5] {
    [
        ("feedback_events", &state.feedback_events),
        ("branch_contexts", &state.branch_contexts),
        ("workstreams", &state.workstreams),
        ("open_loops", &state.open_loops),
        ("memory_cells", &state.memory_cells),
    ]
}

fn validate_short_identifier(
    field: &str,
    value: &str,
    max_chars: usize,
) -> Result<(), AccuracyFixtureError> {
    if value.is_empty()
        || value.chars().count() > max_chars
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    {
        return Err(AccuracyFixtureError::InvalidContract(format!(
            "{field} must be a bounded ASCII identifier"
        )));
    }
    Ok(())
}

fn validate_public_safe_string(
    field: &str,
    value: &str,
    max_chars: usize,
) -> Result<(), AccuracyFixtureError> {
    let mut violations = Vec::new();
    lint_sensitive_string(field, value, max_chars, &mut violations);
    if violations.is_empty() {
        Ok(())
    } else {
        Err(AccuracyFixtureError::Privacy(violations))
    }
}

fn validate_hash(field: &str, hash: &str) -> Result<(), AccuracyFixtureError> {
    if hash.len() == 64
        && hash
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        Ok(())
    } else {
        Err(AccuracyFixtureError::InvalidContract(format!(
            "{field} must be a lowercase SHA-256 digest"
        )))
    }
}

pub(crate) fn import_local_audit_candidate(
    audit_root: &Path,
    case_id: &str,
    requests: &[LocalAuditSourceRequestV1],
) -> Result<AuditImportCandidateV1, AccuracyFixtureError> {
    validate_short_identifier("case_id", case_id, MAX_CASE_ID_CHARS)?;
    let canonical_root = audit_root.canonicalize()?;
    let mut sources = Vec::with_capacity(requests.len());
    for request in requests {
        validate_allowlisted_relative_path(&request.kind, &request.relative_path)?;
        let path = audit_root.join(&request.relative_path);
        let canonical_path = path.canonicalize()?;
        if !canonical_path.starts_with(&canonical_root) {
            return Err(AccuracyFixtureError::InvalidContract(
                "audit source escapes the selected audit root".to_string(),
            ));
        }
        let metadata = fs::metadata(&canonical_path)?;
        if !metadata.is_file() || metadata.len() > MAX_IMPORT_FILE_BYTES {
            return Err(AccuracyFixtureError::InvalidContract(
                "audit source is not a bounded regular file".to_string(),
            ));
        }
        let bytes = fs::read(&canonical_path)?;
        sources.push(AuditImportSourceCandidateV1 {
            kind: request.kind.clone(),
            path_hash: sha256_hex(request.relative_path.to_string_lossy().as_bytes()),
            content_hash: sha256_hex(&bytes),
            byte_count: bytes.len() as u64,
            line_count: bytes.split(|byte| *byte == b'\n').count(),
        });
    }
    Ok(AuditImportCandidateV1 {
        schema: AUDIT_IMPORT_CANDIDATE_SCHEMA_V1.to_string(),
        case_id: case_id.to_string(),
        review_required: true,
        contains_private_text: false,
        sources,
    })
}

fn validate_allowlisted_relative_path(
    kind: &AuditSourceKindV1,
    path: &Path,
) -> Result<(), AccuracyFixtureError> {
    if path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(AccuracyFixtureError::InvalidContract(
            "audit importer accepts normal relative paths only".to_string(),
        ));
    }
    let components: Vec<_> = path
        .components()
        .filter_map(|component| match component {
            Component::Normal(value) => value.to_str(),
            _ => None,
        })
        .collect();
    let allowed = match kind {
        AuditSourceKindV1::FrameTextResolution => {
            matches!(
                components.as_slice(),
                ["evidence", "frames", _, "frame_text_resolution.json"]
            )
        }
        AuditSourceKindV1::LinkedTaskActions => {
            matches!(
                components.as_slice(),
                ["evidence", "frames", _, "linked_task_actions.json"]
            )
        }
        AuditSourceKindV1::SurfaceSnapshots => components == ["decision", "surface-snapshots.json"],
        AuditSourceKindV1::FeedbackEvents => {
            components == ["continue", "layers", "continue_feedback_events.ndjson"]
        }
        AuditSourceKindV1::BranchContexts => {
            components == ["continue", "layers", "continue_branch_contexts.ndjson"]
        }
        AuditSourceKindV1::OpenLoops => {
            components == ["continue", "layers", "continue_open_loops.ndjson"]
        }
        AuditSourceKindV1::StitchedTimeline => {
            components == ["activity_recap", "stitched_timeline.json"]
        }
        AuditSourceKindV1::WorkLabels => components == ["activity_recap", "work_labels.json"],
        AuditSourceKindV1::ModelPack => components == ["activity_recap", "model_pack.json"],
        AuditSourceKindV1::ModelValidation => {
            components == ["activity_recap", "model_validation.json"]
        }
        AuditSourceKindV1::ContinueDecisionResult => {
            components == ["final", "continue_decision_result.json"]
        }
    };
    if !allowed {
        return Err(AccuracyFixtureError::InvalidContract(format!(
            "audit source kind {kind:?} does not permit this relative path"
        )));
    }
    Ok(())
}

pub(crate) fn stable_json_sha256<T: Serialize>(value: &T) -> Result<String, AccuracyFixtureError> {
    Ok(sha256_hex(&serde_json::to_vec(value)?))
}

pub(crate) fn sha256_hex(input: &[u8]) -> String {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let bit_len = (input.len() as u64).wrapping_mul(8);
    let mut message = input.to_vec();
    message.push(0x80);
    while message.len() % 64 != 56 {
        message.push(0);
    }
    message.extend_from_slice(&bit_len.to_be_bytes());
    let mut state = [
        0x6a09e667u32,
        0xbb67ae85,
        0x3c6ef372,
        0xa54ff53a,
        0x510e527f,
        0x9b05688c,
        0x1f83d9ab,
        0x5be0cd19,
    ];
    for chunk in message.chunks_exact(64) {
        let mut words = [0u32; 64];
        for (index, word) in words.iter_mut().take(16).enumerate() {
            let offset = index * 4;
            *word = u32::from_be_bytes(chunk[offset..offset + 4].try_into().unwrap());
        }
        for index in 16..64 {
            let s0 = words[index - 15].rotate_right(7)
                ^ words[index - 15].rotate_right(18)
                ^ (words[index - 15] >> 3);
            let s1 = words[index - 2].rotate_right(17)
                ^ words[index - 2].rotate_right(19)
                ^ (words[index - 2] >> 10);
            words[index] = words[index - 16]
                .wrapping_add(s0)
                .wrapping_add(words[index - 7])
                .wrapping_add(s1);
        }
        let mut work = state;
        for index in 0..64 {
            let s1 = work[4].rotate_right(6) ^ work[4].rotate_right(11) ^ work[4].rotate_right(25);
            let choice = (work[4] & work[5]) ^ ((!work[4]) & work[6]);
            let temp1 = work[7]
                .wrapping_add(s1)
                .wrapping_add(choice)
                .wrapping_add(K[index])
                .wrapping_add(words[index]);
            let s0 = work[0].rotate_right(2) ^ work[0].rotate_right(13) ^ work[0].rotate_right(22);
            let majority = (work[0] & work[1]) ^ (work[0] & work[2]) ^ (work[1] & work[2]);
            let temp2 = s0.wrapping_add(majority);
            work = [
                temp1.wrapping_add(temp2),
                work[0],
                work[1],
                work[2],
                work[3].wrapping_add(temp1),
                work[4],
                work[5],
                work[6],
            ];
        }
        for (value, addition) in state.iter_mut().zip(work) {
            *value = value.wrapping_add(addition);
        }
    }
    state.iter().map(|value| format!("{value:08x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn synthetic_text(text: &str, purpose: FixtureTextPurposeV1) -> FixtureTextV1 {
        FixtureTextV1 {
            text: text.to_string(),
            storage_class: FixtureTextStorageClassV1::Synthetic,
            purpose,
            human_privacy_approved: false,
            source_hash: None,
        }
    }

    fn fixture() -> ContinueAccuracyFixtureV1 {
        ContinueAccuracyFixtureV1 {
            schema: ACCURACY_FIXTURE_SCHEMA_V1.to_string(),
            case_id: "capture-fresh".to_string(),
            scenario: CaptureAccuracyScenarioV1::FreshTaskOnly,
            description: "Synthetic Capture-button task".to_string(),
            privacy_review: PrivacyReviewV1 {
                status: PrivacyReviewStatusV1::Approved,
                reviewed_at_ms: Some(1),
                reviewer_role: Some("fixture_owner".to_string()),
            },
            fixture_partition: FixturePartitionV1::Development,
            injection_boundary: InjectionBoundaryV1::CaptureRecords,
            redacted_source_records: RedactedSourceRecordsV1 {
                content_units: vec![FixtureSourceRecordV1 {
                    record_id: "content-1".to_string(),
                    frame_id: Some("frame-1".to_string()),
                    observed_at_ms: 1,
                    parent_record_id: None,
                    source_role: Some("static_text".to_string()),
                    source_order: Some(1),
                    bounds: None,
                    owner_id_hash: None,
                    confidence: Some(1.0),
                    text: Some(synthetic_text(
                        "What does the island Capture button do?",
                        FixtureTextPurposeV1::SourceSemanticText,
                    )),
                    metadata: BTreeMap::new(),
                }],
                ..Default::default()
            },
            injected_historical_state: InjectedHistoricalStateV1::default(),
            expected_checkpoints: vec![ExpectedCheckpointV1 {
                checkpoint: AccuracyCheckpointV1::LatestTaskTurn,
                status: ExpectedCheckpointStatusV1::Expected,
                slots: BTreeMap::from([(
                    "execution_state".to_string(),
                    FixtureScalarV1::Label {
                        value: "active".to_string(),
                    },
                )]),
            }],
            forbidden_claims: vec![],
            allowed_uncertainty: vec![],
            deterministic_model_output: None,
            expected_model_parity: ExpectedModelParityV1 {
                required: true,
                identity_slots: vec!["task_object".to_string()],
            },
        }
    }

    #[test]
    fn strict_fixture_parse_rejects_unknown_schema_and_field() {
        let fixture = fixture();
        let bytes = serde_json::to_vec(&fixture).unwrap();
        assert!(parse_accuracy_fixture_json(&bytes).is_ok());

        let mut value = serde_json::to_value(&fixture).unwrap();
        value["schema"] = serde_json::Value::String("future.schema".to_string());
        assert!(matches!(
            parse_accuracy_fixture_json(&serde_json::to_vec(&value).unwrap()),
            Err(AccuracyFixtureError::InvalidSchema { .. })
        ));

        value["schema"] = serde_json::Value::String(ACCURACY_FIXTURE_SCHEMA_V1.to_string());
        value["unknown"] = serde_json::json!(true);
        assert!(matches!(
            parse_accuracy_fixture_json(&serde_json::to_vec(&value).unwrap()),
            Err(AccuracyFixtureError::Json(_))
        ));
    }

    #[test]
    fn privacy_linter_rejects_required_sensitive_shapes() {
        let samples = [
            (
                "/Users/example/private.txt",
                PrivacyLintCodeV1::HomeDirectoryPath,
            ),
            (
                "sk-abcdefghijklmnopqrstuvwxyz123456",
                PrivacyLintCodeV1::SecretLikeToken,
            ),
            (
                "https://example.test/path?token=secret",
                PrivacyLintCodeV1::UrlQueryString,
            ),
            (
                "captures/private-shot.png",
                PrivacyLintCodeV1::ScreenshotPath,
            ),
        ];
        for (sample, expected) in samples {
            let mut fixture = fixture();
            fixture.redacted_source_records.content_units[0].text = Some(synthetic_text(
                sample,
                FixtureTextPurposeV1::SourceSemanticText,
            ));
            let report = lint_accuracy_fixture(&fixture);
            assert!(report.violations.iter().any(|item| item.code == expected));
        }
        let mut oversized = fixture();
        oversized.redacted_source_records.content_units[0].text = Some(synthetic_text(
            &"x".repeat(MAX_SOURCE_TEXT_CHARS + 1),
            FixtureTextPurposeV1::SourceSemanticText,
        ));
        assert!(lint_accuracy_fixture(&oversized)
            .violations
            .iter()
            .any(|item| item.code == PrivacyLintCodeV1::OversizedText));
    }

    #[test]
    fn holdout_is_default_denied() {
        assert!(enforce_holdout_access(
            FixturePartitionV1::LockedHoldout,
            HoldoutAccessModeV1::Development
        )
        .is_err());
        assert!(enforce_holdout_access(
            FixturePartitionV1::LockedHoldout,
            HoldoutAccessModeV1::ReleaseEvaluation
        )
        .is_ok());

        let mut locked = fixture();
        locked.fixture_partition = FixturePartitionV1::LockedHoldout;
        let bytes = serde_json::to_vec(&locked).unwrap();
        assert!(matches!(
            parse_accuracy_fixture_json_for_access(&bytes, HoldoutAccessModeV1::Validation),
            Err(AccuracyFixtureError::HoldoutDenied)
        ));
        assert!(parse_accuracy_fixture_json_for_access(
            &bytes,
            HoldoutAccessModeV1::ReleaseEvaluation
        )
        .is_ok());
    }

    #[test]
    fn committed_fixture_requires_completed_human_review() {
        let mut pending = fixture();
        pending.privacy_review.status = PrivacyReviewStatusV1::Pending;
        assert!(matches!(
            parse_accuracy_fixture_json(&serde_json::to_vec(&pending).unwrap()),
            Err(AccuracyFixtureError::InvalidContract(_))
        ));

        let mut derived = fixture();
        derived.redacted_source_records.content_units[0].text = Some(FixtureTextV1 {
            text: "Bounded reviewed excerpt".to_string(),
            storage_class: FixtureTextStorageClassV1::DerivedRedacted,
            purpose: FixtureTextPurposeV1::SourceSemanticText,
            human_privacy_approved: false,
            source_hash: Some(sha256_hex(b"private source")),
        });
        assert!(lint_accuracy_fixture(&derived)
            .violations
            .iter()
            .any(|item| item.code == PrivacyLintCodeV1::DerivedTextNotApproved));
    }

    #[test]
    fn eval_policy_is_strict_and_freezes_all_partitions() {
        let policy = ContinueAccuracyEvalPolicyV1 {
            schema: ACCURACY_EVAL_POLICY_SCHEMA_V1.to_string(),
            policy_version: "p6.01-v1".to_string(),
            semantic_slot_rubric: BTreeMap::from([(
                "latest_user_goal".to_string(),
                "typed exact semantic slot".to_string(),
            )]),
            materially_wrong_slots: vec!["latest_user_goal".to_string()],
            confidence_boundaries: ConfidenceBoundariesV1 {
                none_max: 0.0,
                low_max: 0.45,
                medium_max: 0.75,
                high_min: 0.75,
            },
            wrong_confident_threshold: 0.65,
            minimum_samples: EvalMinimumSamplesV1 {
                calibration_predictions: 100,
                calibration_cases_per_bin: 5,
                worst_slice_cases: 5,
            },
            partitions: vec![
                FixturePartitionV1::Development,
                FixturePartitionV1::Validation,
                FixturePartitionV1::LockedHoldout,
            ],
            aggregation_modes: vec!["case_macro".to_string(), "worst_slice".to_string()],
            calibration_ece_max: 0.1,
            baseline: EvalBaselineV1 {
                model_off_p95_ms: 1.0,
                sample_count: 10,
                warmup_count: 1,
                environment_label: "local-test".to_string(),
            },
            model_off_p95_regression_factor: 1.25,
            absolute_model_off_p95_budget_ms: None,
        };
        assert!(parse_eval_policy_json(&serde_json::to_vec(&policy).unwrap()).is_ok());
        let mut unknown = serde_json::to_value(&policy).unwrap();
        unknown["unfrozen"] = serde_json::json!(true);
        assert!(matches!(
            parse_eval_policy_json(&serde_json::to_vec(&unknown).unwrap()),
            Err(AccuracyFixtureError::Json(_))
        ));
    }

    #[test]
    fn milestone_known_failure_expires_and_improvement_requires_review() {
        let manifest = AccuracyMilestoneManifestV1 {
            schema: ACCURACY_MILESTONE_SCHEMA_V1.to_string(),
            manifest_version: "v1".to_string(),
            cases: vec![MilestoneCaseV1 {
                case_id: "capture-fresh".to_string(),
                expected_status: MilestoneExpectedStatusV1::KnownFailure,
                expected_first_divergence: Some(AccuracyCheckpointV1::LatestTaskTurn),
                must_pass_by_phase: P6PhaseV1::P6_03,
                owner_checkpoint: AccuracyCheckpointV1::LatestTaskTurn,
                expires_at_ms: None,
            }],
        };
        let passing = [ObservedMilestoneV1 {
            case_id: "capture-fresh".to_string(),
            status: ObservedMilestoneStatusV1::Pass,
            first_divergence: None,
        }];
        let violations = enforce_milestones(&manifest, &passing, P6PhaseV1::P6_01, 0);
        assert!(violations
            .iter()
            .any(|item| item.reason.contains("improvement")));

        let failing = [ObservedMilestoneV1 {
            case_id: "capture-fresh".to_string(),
            status: ObservedMilestoneStatusV1::Fail,
            first_divergence: Some(AccuracyCheckpointV1::LatestTaskTurn),
        }];
        let violations = enforce_milestones(&manifest, &failing, P6PhaseV1::P6_03, 0);
        assert!(violations
            .iter()
            .any(|item| item.reason == "known_failure_marker_expired"));
    }

    #[test]
    fn audit_importer_emits_hashes_without_private_text() {
        let directory = std::env::temp_dir().join(format!(
            "smalltalk-accuracy-import-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(directory.join("final")).unwrap();
        let source = directory.join("final/continue_decision_result.json");
        fs::write(&source, br#"{"private":"must not escape"}"#).unwrap();
        let candidate = import_local_audit_candidate(
            &directory,
            "capture-import",
            &[LocalAuditSourceRequestV1 {
                kind: AuditSourceKindV1::ContinueDecisionResult,
                relative_path: PathBuf::from("final/continue_decision_result.json"),
            }],
        )
        .unwrap();
        let output = serde_json::to_string(&candidate).unwrap();
        assert!(candidate.review_required);
        assert!(!candidate.contains_private_text);
        assert!(!output.contains("must not escape"));
        assert_eq!(candidate.sources[0].content_hash.len(), 64);

        fs::create_dir_all(directory.join("misleading")).unwrap();
        fs::write(
            directory.join("misleading/continue_decision_result.json"),
            b"{}",
        )
        .unwrap();
        assert!(import_local_audit_candidate(
            &directory,
            "capture-import",
            &[LocalAuditSourceRequestV1 {
                kind: AuditSourceKindV1::ContinueDecisionResult,
                relative_path: PathBuf::from("misleading/continue_decision_result.json"),
            }],
        )
        .is_err());
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn sha256_is_stable_and_standard() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn committed_corpus_parses_lints_and_contains_initial_case_set() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/continue_accuracy");
        let policy = parse_eval_policy_json(&fs::read(root.join("eval-policy.v1.json")).unwrap())
            .expect("frozen policy must parse");
        let manifest =
            parse_milestone_manifest_json(&fs::read(root.join("known-failures.v1.json")).unwrap())
                .expect("milestone manifest must parse");
        let mut fixtures = fs::read_dir(root.join("cases"))
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("json"))
            .map(|path| {
                parse_accuracy_fixture_json_for_access(
                    &fs::read(path).unwrap(),
                    HoldoutAccessModeV1::Development,
                )
                .unwrap()
            })
            .collect::<Vec<_>>();
        fixtures.sort_by(|left, right| left.case_id.cmp(&right.case_id));
        validate_initial_capture_case_set(&fixtures).unwrap();
        assert!(manifest.cases.iter().all(|milestone| fixtures
            .iter()
            .any(|fixture| fixture.case_id == milestone.case_id)));
        assert!(fixtures
            .iter()
            .all(|fixture| lint_accuracy_fixture(fixture).passed));
        assert_eq!(policy.partitions.len(), 3);
    }

    #[test]
    fn lca_06_live_runtime_manifest_is_complete_and_privacy_safe() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/continue_accuracy");
        let raw = fs::read_to_string(root.join("lca-06-live-runtime-replay.v1.json")).unwrap();
        let manifest: serde_json::Value = serde_json::from_str(&raw).unwrap();
        let cases = manifest["cases"].as_array().unwrap();
        let ids = cases
            .iter()
            .filter_map(|case| case["case_id"].as_str())
            .collect::<BTreeSet<_>>();
        let expected = (1..=11)
            .map(|index| format!("LCA-06-{index:02}"))
            .collect::<BTreeSet<_>>();
        assert_eq!(
            ids.into_iter().map(str::to_string).collect::<BTreeSet<_>>(),
            expected
        );
        assert_eq!(
            manifest["privacy_class"].as_str(),
            Some("synthetic_causal_shape_only")
        );
        for forbidden in [
            "/Users/",
            "https://",
            "api_key",
            "screenshot_path",
            "raw_text",
        ] {
            assert!(
                !raw.contains(forbidden),
                "private fixture marker: {forbidden}"
            );
        }
    }
}
