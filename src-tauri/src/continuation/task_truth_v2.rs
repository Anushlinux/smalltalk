use super::accuracy_eval::run_fixture_once;
use super::accuracy_fixture::{
    sha256_hex, stable_json_sha256, CaptureAccuracyScenarioV1, ContinueAccuracyFixtureV1,
    ExpectedModelParityV1, FixtureBoundsV1, FixturePartitionV1, FixtureScalarV1,
    FixtureSourceRecordV1, FixtureTextPurposeV1, FixtureTextStorageClassV1, FixtureTextV1,
    InjectedHistoricalStateV1, InjectionBoundaryV1, PrivacyReviewStatusV1, PrivacyReviewV1,
    RedactedSourceRecordsV1, ACCURACY_FIXTURE_SCHEMA_V1,
};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
#[cfg(test)]
use std::path::PathBuf;

pub(crate) mod audit;
pub(crate) mod checkpoint;
pub(crate) mod model;
pub(crate) mod observation_packet;
pub(crate) mod production;
pub(crate) mod selection;
pub(crate) mod task_snapshot;
pub(crate) mod verifier;

use self::observation_packet::build_observation_packet;
use self::task_snapshot::{project_current_task_turn, unresolved_snapshot};
use self::verifier::TaskTruthVerifier;

pub(crate) const TASK_TRUTH_FIXTURE_SCHEMA_V2: &str = "smalltalk.task_truth_fixture.v2";
pub(crate) const TASK_TRUTH_POLICY_SCHEMA_V1: &str = "smalltalk.task_truth_v2.eval_policy.v1";
pub(crate) const TASK_TRUTH_REPORT_SCHEMA_V1: &str = "smalltalk.task_truth_v2.report.v1";
pub(crate) const TASK_TRUTH_BUILDER_SCHEMA_V1: &str = "smalltalk.task_truth_v2.builder_input.v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct MultimodalShadowAuditV1 {
    pub(crate) schema: String,
    pub(crate) provider_enabled: bool,
    pub(crate) provider_model: String,
    pub(crate) resolver: model::ResolverAttemptV1,
    pub(crate) verification: verifier::VerificationResultV1,
    pub(crate) second_pass_ran: bool,
    pub(crate) second_pass_latency_ms: i64,
    pub(crate) second_pass_changed_fields: bool,
    pub(crate) estimated_request_cost_usd: Option<f64>,
    pub(crate) deterministic_wording: Option<String>,
    pub(crate) production_authority_changed: bool,
}

fn multimodal_provider_enabled() -> bool {
    std::env::var("SMALLTALK_TASK_TRUTH_MULTIMODAL_ENABLED")
        .ok()
        .is_some_and(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes"
            )
        })
}

fn run_multimodal_shadow(
    packet: &observation_packet::ObservationPacketV2,
    prior: Option<&task_snapshot::TaskSnapshotV2>,
) -> (
    Option<task_snapshot::TaskSnapshotV2>,
    MultimodalShadowAuditV1,
) {
    use self::model::{TaskTruthModelClient, TaskTruthResolver};
    let enabled = multimodal_provider_enabled();
    let mut config = super::continue_openai_config(None).ok();
    let task_truth_model = std::env::var("SMALLTALK_TASK_TRUTH_MODEL")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            super::project_dotenv_values()
                .ok()
                .and_then(|values| values.get("SMALLTALK_TASK_TRUTH_MODEL").cloned())
                .filter(|value| !value.trim().is_empty())
        });
    if let (Some(config), Some(task_truth_model)) = (config.as_mut(), task_truth_model) {
        config.model = task_truth_model;
    }
    let model_name = config
        .as_ref()
        .map(|config| config.model.clone())
        .unwrap_or_else(|| "unconfigured".into());
    let client: Box<dyn TaskTruthModelClient> = if enabled {
        match config.and_then(|config| config.api_key.map(|api_key| (config.model, api_key))) {
            Some((model, api_key)) => {
                Box::new(model::OpenAiTaskTruthModelClient { model, api_key })
            }
            None => Box::new(model::UnavailableModelClient {
                model: model_name.clone(),
                reason: "credentials_unavailable".into(),
            }),
        }
    } else {
        Box::new(model::UnavailableModelClient {
            model: model_name.clone(),
            reason: "multimodal_shadow_disabled".into(),
        })
    };
    let resolver = model::MultimodalTaskTruthResolver.resolve(packet, prior, client.as_ref());
    let verifier = verifier::LocalEvidenceVerifier;
    let mut verification = resolver
        .output
        .as_ref()
        .map(|output| verifier.verify(packet, prior, &output.hypotheses, output.resolution_status))
        .unwrap_or(verifier::VerificationResultV1 {
            status: resolver.status,
            snapshot: None,
            fields: Vec::new(),
            second_pass_reasons: Vec::new(),
            unsupported_claim_count: 0,
        });
    let mut second_pass_ran = false;
    let mut second_pass_latency_ms = 0;
    let mut second_pass_changed_fields = false;
    if enabled
        && !verification.second_pass_reasons.is_empty()
        && resolver.output.is_some()
        && !matches!(
            resolver.status,
            model::ResolutionStatusV1::PrivacyBlocked | model::ResolutionStatusV1::ModelUnavailable
        )
    {
        let started = std::time::Instant::now();
        let conflict = json!({
            "reason": "bounded_conflict_reconciliation",
            "triggers": verification.second_pass_reasons,
            "competing_hypotheses": resolver.output.as_ref().map(|output| &output.hypotheses),
            "first_pass_verdicts": verification.fields,
            "legacy_p6_answer_included": false,
        });
        if let Ok(request) =
            model::build_multimodal_request(packet, prior, client.model_name(), Some(&conflict))
        {
            second_pass_ran = true;
            if let Ok(output) = client.infer(&request) {
                let candidate =
                    verifier.verify(packet, prior, &output.hypotheses, output.resolution_status);
                let candidate_better = candidate.snapshot.is_some()
                    && (verification.snapshot.is_none()
                        || candidate.unsupported_claim_count
                            < verification.unsupported_claim_count);
                if candidate_better {
                    second_pass_changed_fields = candidate.snapshot != verification.snapshot;
                    verification = candidate;
                }
            }
        }
        second_pass_latency_ms = started.elapsed().as_millis() as i64;
    }
    let snapshot = verification.snapshot.clone();
    let deterministic_wording = snapshot
        .as_ref()
        .map(verifier::deterministic_first_screen_wording);
    let estimated_request_cost_usd = resolver.request_audit.as_ref().map(|audit| {
        // Audit-only rough estimate. Model selection remains based on locked task-truth results.
        ((audit.estimated_tokens as f64 * 0.000_000_5) + (audit.image_count as f64 * 0.000_85))
            * if second_pass_ran { 2.0 } else { 1.0 }
    });
    (
        snapshot,
        MultimodalShadowAuditV1 {
            schema: "smalltalk.task_truth_multimodal_shadow_audit.v1".into(),
            provider_enabled: enabled,
            provider_model: model_name,
            resolver,
            verification,
            second_pass_ran,
            second_pass_latency_ms,
            second_pass_changed_fields,
            estimated_request_cost_usd,
            deterministic_wording,
            production_authority_changed: false,
        },
    )
}

pub(super) fn ensure_shadow_schema(conn: &Connection) -> Result<(), String> {
    checkpoint::ensure_schema(conn)
}

pub(super) fn checkpoint_observation_frames(
    conn: &Connection,
    frames: &[super::EvidenceFrame],
    evidence_watermark: &str,
) -> Result<Option<checkpoint::SemanticCheckpointV2>, String> {
    if frames.is_empty() {
        return Ok(None);
    }
    let prior = checkpoint::load_latest_snapshot(
        conn,
        frames.last().and_then(|frame| frame.session_id.as_deref()),
    )?;
    let packet = build_observation_packet(
        frames,
        evidence_watermark,
        prior.as_ref().map(|snapshot| snapshot.snapshot_id.clone()),
    )?;
    let snapshot = match super::task_turn::selected_current_task_turn(conn)? {
        Some(turn)
            if turn
                .latest_user_goal_summary
                .as_deref()
                .is_some_and(|goal| !goal.trim().is_empty()) =>
        {
            project_current_task_turn(&turn, &packet, prior.as_ref())
        }
        _ => unresolved_snapshot(
            &packet,
            prior.as_ref(),
            "no_supported_current_task_evidence",
        ),
    };
    checkpoint::persist_checkpoint(conn, &packet, &snapshot).map(Some)
}

pub(super) fn record_manual_continue_shadow(
    conn: &Connection,
    decision_id: &str,
    session_id: Option<&str>,
    legacy_selected_candidate_id: Option<&str>,
) -> Result<audit::ShadowAuditSummaryV2, String> {
    let started = std::time::Instant::now();
    let watermark = super::build_continue_evidence_watermark(conn, session_id)?;
    let mut frames = super::load_evidence_frames(
        conn,
        &super::ContinueSecondLayerRebuildRequest {
            session_id: session_id.map(str::to_string),
            lookback_ms: Some(45 * 60 * 1000),
            limit: Some(24),
            ..Default::default()
        },
    )?;
    if let Some(current) = frames.last_mut() {
        current.capture_trigger = "manual_continue".into();
    }
    let prior = checkpoint::load_latest_snapshot(conn, session_id)?;
    let packet = build_observation_packet(
        &frames,
        &watermark.hash,
        prior.as_ref().map(|snapshot| snapshot.snapshot_id.clone()),
    )?;
    let legacy_turn = super::task_turn::selected_current_task_turn(conn)?;
    let local_fallback_snapshot = match legacy_turn.as_ref() {
        Some(turn)
            if turn
                .latest_user_goal_summary
                .as_deref()
                .is_some_and(|goal| !goal.trim().is_empty()) =>
        {
            project_current_task_turn(turn, &packet, prior.as_ref())
        }
        _ => unresolved_snapshot(
            &packet,
            prior.as_ref(),
            "manual_continue_without_supported_task",
        ),
    };
    let (verified_snapshot, multimodal_audit) = run_multimodal_shadow(&packet, prior.as_ref());
    let snapshot = verified_snapshot.unwrap_or_else(|| {
        let mut fallback = local_fallback_snapshot;
        fallback.provenance.push(
            format!(
                "multimodal_shadow_fallback:{:?}",
                multimodal_audit.resolver.status
            )
            .to_ascii_lowercase(),
        );
        fallback
    });
    checkpoint::persist_checkpoint(conn, &packet, &snapshot)?;
    let snapshots = checkpoint::load_recent_snapshots(conn, session_id, 24)?;
    let selection = selection::select_snapshot(&snapshots);
    audit::persist_shadow_audit(
        conn,
        decision_id,
        &packet,
        &snapshots,
        &selection,
        legacy_turn.as_ref().map(|turn| turn.task_turn_id.as_str()),
        legacy_selected_candidate_id,
        started.elapsed().as_millis() as i64,
        Some(&multimodal_audit),
    )
}

const MAX_TEXT_CHARS: usize = 512;
const REQUIRED_LIVE_CASES: usize = 200;
const REQUIRED_HOLDOUT_CASES: usize = 50;
const REQUIRED_SURFACE_CASES: usize = 15;
const REQUIRED_INTERRUPTION_SEQUENCES: usize = 30;
const REQUIRED_AMBIGUOUS_OR_BLOCKED: usize = 20;
const REQUIRED_WAITING: usize = 20;
const REQUIRED_BOUNDARIES: usize = 20;

const REQUIRED_SURFACES: [&str; 10] = [
    "agent_chat",
    "editor_ide",
    "terminal",
    "browser_research_search",
    "documents",
    "spreadsheets",
    "email_messaging",
    "pdf_file_manager",
    "custom_rendered_canvas",
    "mixed_window_thin_unknown",
];

const COMPARISON_FIELDS: [&str; 13] = [
    "observation_construction",
    "causal_interaction_association",
    "control_region_eligibility",
    "authorship",
    "task_selection",
    "task_object",
    "lifecycle_state",
    "last_progress",
    "unfinished_step",
    "next_action",
    "where",
    "target_resolution",
    "answer_composition",
];

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SourceKindV2 {
    LiveRedacted,
    SyntheticCounterfactual,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TaskTruthPartitionV2 {
    Development,
    Validation,
    LockedHoldout,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ReviewStatusV2 {
    Pending,
    Approved,
    Rejected,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TextProvenanceV2 {
    DerivedRedacted,
    HumanParaphrase,
    Synthetic,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct TaskTruthTextV2 {
    pub(crate) text: String,
    pub(crate) provenance: TextProvenanceV2,
    pub(crate) content_hash: String,
    #[serde(default)]
    pub(crate) copied_from_private_text: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct SourceRecordV2 {
    pub(crate) record_id: String,
    #[serde(default)]
    pub(crate) frame_id: Option<String>,
    pub(crate) observed_at_ms: i64,
    #[serde(default)]
    pub(crate) parent_record_id: Option<String>,
    #[serde(default)]
    pub(crate) native_role: Option<String>,
    #[serde(default)]
    pub(crate) native_subrole: Option<String>,
    #[serde(default)]
    pub(crate) source_order: Option<i64>,
    #[serde(default)]
    pub(crate) bounds: Option<FixtureBoundsV1>,
    #[serde(default)]
    pub(crate) text: Option<TaskTruthTextV2>,
    #[serde(default)]
    pub(crate) actions: Vec<String>,
    #[serde(default)]
    pub(crate) focused: Option<bool>,
    #[serde(default)]
    pub(crate) editable: Option<bool>,
    #[serde(default)]
    pub(crate) owner_hash: Option<String>,
    #[serde(default)]
    pub(crate) metadata: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ProductionEvidenceV2 {
    #[serde(default)]
    pub(crate) frames: Vec<SourceRecordV2>,
    #[serde(default)]
    pub(crate) private_replay_refs: Vec<SourceRecordV2>,
    #[serde(default)]
    pub(crate) redacted_screenshot_regions: Vec<SourceRecordV2>,
    #[serde(default)]
    pub(crate) ax_nodes: Vec<SourceRecordV2>,
    #[serde(default)]
    pub(crate) ocr_spans: Vec<SourceRecordV2>,
    #[serde(default)]
    pub(crate) content_units: Vec<SourceRecordV2>,
    #[serde(default)]
    pub(crate) app_window_surface: Vec<SourceRecordV2>,
    #[serde(default)]
    pub(crate) ui_events: Vec<SourceRecordV2>,
    #[serde(default)]
    pub(crate) typing_bursts: Vec<SourceRecordV2>,
    #[serde(default)]
    pub(crate) capture_triggers: Vec<SourceRecordV2>,
    #[serde(default)]
    pub(crate) event_transitions: Vec<SourceRecordV2>,
    #[serde(default)]
    pub(crate) frame_diffs: Vec<SourceRecordV2>,
    #[serde(default)]
    pub(crate) change_regions: Vec<SourceRecordV2>,
    #[serde(default)]
    pub(crate) prior_valid_task_snapshot: Option<SourceRecordV2>,
    #[serde(default)]
    pub(crate) return_anchor_facts: Vec<SourceRecordV2>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct CaseReviewV2 {
    pub(crate) status: ReviewStatusV2,
    #[serde(default)]
    pub(crate) reviewer_ids: Vec<String>,
    #[serde(default)]
    pub(crate) blinded_before_product_output: bool,
    #[serde(default)]
    pub(crate) independently_human_reviewed: bool,
    pub(crate) reviewed_at_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct HumanAdjudicationV2 {
    pub(crate) resolution: String,
    pub(crate) expected_observation_status: String,
    pub(crate) expected_causal_association: bool,
    pub(crate) expected_control_as_task: bool,
    pub(crate) expected_authorship: bool,
    pub(crate) primary_task_summary: TaskTruthTextV2,
    pub(crate) task_object: TaskTruthTextV2,
    pub(crate) user_goal: TaskTruthTextV2,
    pub(crate) last_meaningful_progress: TaskTruthTextV2,
    pub(crate) unfinished_step: TaskTruthTextV2,
    pub(crate) execution_state: String,
    pub(crate) current_actor: String,
    pub(crate) waiting_on: String,
    pub(crate) next_supported_action: TaskTruthTextV2,
    pub(crate) where_identity: TaskTruthTextV2,
    pub(crate) relation_to_prior_task: String,
    #[serde(default)]
    pub(crate) support_detour_surfaces: Vec<TaskTruthTextV2>,
    pub(crate) direct_return_anchor: Option<TaskTruthTextV2>,
    #[serde(default)]
    pub(crate) acceptable_alternative_hypotheses: Vec<TaskTruthTextV2>,
    #[serde(default)]
    pub(crate) required_abstention_fields: Vec<String>,
    #[serde(default)]
    pub(crate) forbidden_claims: Vec<TaskTruthTextV2>,
    pub(crate) immediately_useful: bool,
    pub(crate) reviewer_notes: TaskTruthTextV2,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PathStatusV2 {
    RecordedProductionOutput,
    ReplayedProductionOutput,
    Implemented,
    NotImplemented,
    MissingEvidence,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct PathOutputV2 {
    pub(crate) status: PathStatusV2,
    pub(crate) evidence_hash: String,
    #[serde(default)]
    pub(crate) recorded_private_output_hash: Option<String>,
    #[serde(default)]
    pub(crate) consumed_evidence_groups: Vec<String>,
    #[serde(default)]
    pub(crate) unsupported_evidence_groups: Vec<String>,
    #[serde(default)]
    pub(crate) claim_confidence: Option<f64>,
    #[serde(default)]
    pub(crate) checkpoints: BTreeMap<String, Value>,
    pub(crate) task_identity: Option<TaskTruthTextV2>,
    pub(crate) answer_class: Option<String>,
    #[serde(default)]
    pub(crate) unsupported_claims: Vec<TaskTruthTextV2>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct RecordedOutputsV2 {
    pub(crate) path_a_legacy_p6: PathOutputV2,
    pub(crate) path_c_task_truth_shadow: PathOutputV2,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ModelParityReviewV1 {
    pub(crate) critical_local_solvable: bool,
    pub(crate) model_on_task_identity: Option<TaskTruthTextV2>,
    pub(crate) model_off_task_identity: Option<TaskTruthTextV2>,
    pub(crate) disagreement_explained: bool,
    pub(crate) explanation_independently_reviewed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct TaskTruthCaseV2 {
    pub(crate) case_id: String,
    pub(crate) source_kind: SourceKindV2,
    pub(crate) capture_pipeline_version: String,
    pub(crate) injection_boundary: String,
    pub(crate) surface_family: String,
    pub(crate) application_identity_bucket: String,
    pub(crate) layout_workflow_bucket: String,
    pub(crate) privacy_review: CaseReviewV2,
    pub(crate) label_review: CaseReviewV2,
    pub(crate) partition: TaskTruthPartitionV2,
    pub(crate) decision_mode: String,
    pub(crate) decision_offset_ms: i64,
    pub(crate) model_request_status: String,
    pub(crate) model_validation_status: String,
    #[serde(default)]
    pub(crate) interruption_resumption_sequence: bool,
    #[serde(default)]
    pub(crate) ambiguous_or_privacy_blocked: bool,
    #[serde(default)]
    pub(crate) waiting_on_agent_or_application: bool,
    #[serde(default)]
    pub(crate) completed_vs_new_task_boundary: bool,
    pub(crate) source: ProductionEvidenceV2,
    pub(crate) adjudication: Option<HumanAdjudicationV2>,
    pub(crate) recorded_outputs: RecordedOutputsV2,
    #[serde(default)]
    pub(crate) model_parity: Option<ModelParityReviewV1>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct TaskTruthCorpusV2 {
    pub(crate) schema: String,
    pub(crate) corpus_version: String,
    #[serde(default)]
    pub(crate) cases: Vec<TaskTruthCaseV2>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TaskTruthPolicyV1 {
    pub(crate) schema: String,
    pub(crate) policy_version: String,
    pub(crate) frozen_before_holdout_access: bool,
    pub(crate) holdout_accessed_at_freeze: bool,
    pub(crate) post_holdout_change_justification: Option<String>,
    pub(crate) metrics: BTreeMap<String, MetricPolicyV1>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct MetricPolicyV1 {
    pub(crate) definition: String,
    pub(crate) threshold: f64,
    pub(crate) higher_is_better: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct BuilderInputV1 {
    pub(crate) schema: String,
    pub(crate) case_id: String,
    pub(crate) source_root: String,
    #[serde(default)]
    pub(crate) records: Vec<BuilderRecordV1>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct BuilderRecordV1 {
    pub(crate) evidence_group: String,
    pub(crate) record: SourceRecordV2,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct PrivacyManifestV1 {
    pub(crate) schema: String,
    pub(crate) case_id: String,
    pub(crate) dry_run: bool,
    pub(crate) review_required: bool,
    pub(crate) retained_record_count: usize,
    pub(crate) rejected_fields: Vec<String>,
    pub(crate) source_root_hash: String,
    pub(crate) content_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct BuilderResultV1 {
    pub(crate) schema: String,
    pub(crate) review_status: ReviewStatusV2,
    pub(crate) privacy_manifest: PrivacyManifestV1,
    pub(crate) retained_records: Vec<BuilderRecordV1>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct PathCaseResultV1 {
    pub(crate) status: PathStatusV2,
    pub(crate) source_fingerprint: String,
    pub(crate) recorded_private_output_hash: Option<String>,
    pub(crate) first_divergence: Option<String>,
    pub(crate) field_results: BTreeMap<String, String>,
    pub(crate) task_identity: Option<String>,
    pub(crate) answer_class: String,
    pub(crate) unsupported_claim_count: usize,
    pub(crate) control_navigation_as_task: bool,
    pub(crate) human_immediately_useful: Option<bool>,
    pub(crate) next_action_label_present: bool,
    pub(crate) next_action_claim_present: bool,
    pub(crate) return_target_claim_present: bool,
    pub(crate) critical_local_solvable: bool,
    pub(crate) model_on_off_disagreement_unexplained: bool,
    pub(crate) consumed_evidence_groups: Vec<String>,
    pub(crate) unsupported_evidence_groups: Vec<String>,
    pub(crate) claim_confidence: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct TaskTruthCaseResultV1 {
    pub(crate) case_id: String,
    pub(crate) source_kind: SourceKindV2,
    pub(crate) release_eligible: bool,
    pub(crate) requires_task_selection_abstention: bool,
    pub(crate) application_identity_bucket: String,
    pub(crate) surface_family: String,
    pub(crate) partition: TaskTruthPartitionV2,
    pub(crate) human_label_status: String,
    pub(crate) paths: BTreeMap<String, PathCaseResultV1>,
    pub(crate) deterministic_replay_match: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct MetricAssessmentV1 {
    pub(crate) numerator: usize,
    pub(crate) denominator: usize,
    pub(crate) rate: Option<f64>,
    pub(crate) threshold: f64,
    pub(crate) higher_is_better: bool,
    pub(crate) passed: bool,
    pub(crate) status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ConfidenceIntervalV1 {
    pub(crate) method: String,
    pub(crate) confidence_level: f64,
    pub(crate) denominator: usize,
    pub(crate) lower: Option<f64>,
    pub(crate) upper: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct TaskTruthReportV1 {
    pub(crate) schema: String,
    pub(crate) policy_version: String,
    pub(crate) corpus_counts: BTreeMap<String, usize>,
    pub(crate) partition_counts: BTreeMap<String, usize>,
    pub(crate) human_review_counts: BTreeMap<String, usize>,
    pub(crate) privacy_review_counts: BTreeMap<String, usize>,
    pub(crate) surface_denominators: BTreeMap<String, usize>,
    pub(crate) reviewed_live_surface_denominators: BTreeMap<String, usize>,
    pub(crate) slice_denominators: BTreeMap<String, usize>,
    pub(crate) reviewed_live_slice_denominators: BTreeMap<String, usize>,
    pub(crate) per_path_field_metrics: BTreeMap<String, BTreeMap<String, BTreeMap<String, usize>>>,
    pub(crate) reviewed_live_per_path_field_metrics:
        BTreeMap<String, BTreeMap<String, BTreeMap<String, usize>>>,
    pub(crate) path_macro_results: BTreeMap<String, BTreeMap<String, usize>>,
    pub(crate) reviewed_live_path_macro_results: BTreeMap<String, BTreeMap<String, usize>>,
    pub(crate) synthetic_counterfactual_path_macro_results:
        BTreeMap<String, BTreeMap<String, usize>>,
    pub(crate) surface_family_macro_results:
        BTreeMap<String, BTreeMap<String, BTreeMap<String, usize>>>,
    pub(crate) reviewed_live_surface_family_macro_results:
        BTreeMap<String, BTreeMap<String, BTreeMap<String, usize>>>,
    pub(crate) worst_surface_family_slice_by_path: BTreeMap<String, Option<String>>,
    pub(crate) frozen_policy_metric_results: BTreeMap<String, MetricAssessmentV1>,
    pub(crate) tt2_05_metric_results: BTreeMap<String, MetricAssessmentV1>,
    pub(crate) tt2_05_surface_wrong_task_results: BTreeMap<String, MetricAssessmentV1>,
    pub(crate) tt2_05_confidence_intervals: BTreeMap<String, ConfidenceIntervalV1>,
    pub(crate) wrong_task_examples: BTreeMap<String, Vec<String>>,
    pub(crate) reviewed_live_wrong_task_examples: BTreeMap<String, Vec<String>>,
    pub(crate) first_divergence_histogram: BTreeMap<String, usize>,
    pub(crate) reviewed_live_first_divergence_histogram: BTreeMap<String, usize>,
    pub(crate) worst_surface_family_slice: Option<String>,
    pub(crate) manual_background_downgrade_count: usize,
    pub(crate) manual_background_downgrade_evaluable_count: usize,
    pub(crate) unsupported_claim_count: usize,
    pub(crate) path_b_partial_evidence_case_count: usize,
    pub(crate) reviewed_live_path_b_partial_evidence_case_count: usize,
    pub(crate) locked_holdout_access_status: String,
    pub(crate) release_gate_passed: bool,
    pub(crate) release_gate_violations: Vec<String>,
    pub(crate) cases: Vec<TaskTruthCaseResultV1>,
}

pub(crate) fn parse_corpus(bytes: &[u8]) -> Result<TaskTruthCorpusV2, String> {
    let mut corpus: TaskTruthCorpusV2 = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    if corpus.schema != TASK_TRUTH_FIXTURE_SCHEMA_V2 {
        return Err(format!(
            "unsupported Task Truth fixture schema {:?}",
            corpus.schema
        ));
    }
    for case in &mut corpus.cases {
        let source_hash = stable_json_sha256(&case.source).map_err(|e| e.to_string())?;
        for output in [
            &mut case.recorded_outputs.path_a_legacy_p6,
            &mut case.recorded_outputs.path_c_task_truth_shadow,
        ] {
            if output.evidence_hash == "same_source" {
                output.evidence_hash = source_hash.clone();
            }
        }
    }
    validate_corpus(&corpus)?;
    Ok(corpus)
}

pub(crate) fn parse_policy(bytes: &[u8]) -> Result<TaskTruthPolicyV1, String> {
    let policy: TaskTruthPolicyV1 = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    if policy.schema != TASK_TRUTH_POLICY_SCHEMA_V1 || !policy.frozen_before_holdout_access {
        return Err(
            "Task Truth policy must use the v1 schema and be frozen before holdout access".into(),
        );
    }
    let required = [
        "wrong_primary_task",
        "useful_non_generic_summary",
        "task_object_accuracy",
        "execution_state_accuracy",
        "supported_next_action_accuracy",
        "unsupported_specific_claim",
        "control_navigation_as_task",
        "return_target_precision",
        "stronger_manual_result_downgraded",
        "unseen_application_useful_summary",
        "human_immediately_useful_rating",
        "precise_abstention",
        "generic_non_answer",
    ];
    for metric in required {
        let Some(metric_policy) = policy.metrics.get(metric) else {
            return Err(format!("frozen policy is missing metric {metric}"));
        };
        if !metric_policy.threshold.is_finite()
            || !(0.0..=1.0).contains(&metric_policy.threshold)
            || metric_policy.definition.trim().is_empty()
        {
            return Err(format!(
                "frozen policy has invalid rubric or threshold for {metric}"
            ));
        }
    }
    if policy.holdout_accessed_at_freeze
        && policy
            .post_holdout_change_justification
            .as_deref()
            .is_none_or(str::is_empty)
    {
        return Err("post-holdout policy changes require a written justification".into());
    }
    Ok(policy)
}

fn all_source_records(source: &ProductionEvidenceV2) -> Vec<&SourceRecordV2> {
    let mut records = Vec::new();
    for group in [
        &source.frames,
        &source.private_replay_refs,
        &source.redacted_screenshot_regions,
        &source.ax_nodes,
        &source.ocr_spans,
        &source.content_units,
        &source.app_window_surface,
        &source.ui_events,
        &source.typing_bursts,
        &source.capture_triggers,
        &source.event_transitions,
        &source.frame_diffs,
        &source.change_regions,
        &source.return_anchor_facts,
    ] {
        records.extend(group.iter());
    }
    if let Some(record) = &source.prior_valid_task_snapshot {
        records.push(record);
    }
    records
}

fn validate_corpus(corpus: &TaskTruthCorpusV2) -> Result<(), String> {
    let mut ids = BTreeSet::new();
    let mut partition_by_bucket = BTreeMap::new();
    for case in &corpus.cases {
        if !ids.insert(&case.case_id) {
            return Err(format!("duplicate case id {}", case.case_id));
        }
        if !REQUIRED_SURFACES.contains(&case.surface_family.as_str()) {
            return Err(format!(
                "case {} has unsupported surface family",
                case.case_id
            ));
        }
        let group = format!(
            "{}::{}",
            case.application_identity_bucket, case.layout_workflow_bucket
        );
        if let Some(previous) = partition_by_bucket.insert(group.clone(), case.partition) {
            if previous != case.partition {
                return Err(format!("application-level partition leakage for {group}"));
            }
        }
        if case.source_kind == SourceKindV2::LiveRedacted && case.source.frames.is_empty() {
            return Err(format!("live case {} has no frames", case.case_id));
        }
        if case.source_kind == SourceKindV2::LiveRedacted
            && case
                .source
                .frames
                .iter()
                .any(|frame| !frame.metadata.contains_key("privacy_status"))
        {
            return Err(format!(
                "live case {} has a frame without privacy status",
                case.case_id
            ));
        }
        if !case.source.private_replay_refs.is_empty() {
            return Err(format!(
                "committed case {} contains private replay references",
                case.case_id
            ));
        }
        for record in all_source_records(&case.source) {
            lint_record(case, record)?;
        }
        if let Some(adjudication) = &case.adjudication {
            lint_adjudication(&case.case_id, adjudication)?;
        }
        if let Some(parity) = &case.model_parity {
            for (label, text) in [
                (
                    "model_on_task_identity",
                    parity.model_on_task_identity.as_ref(),
                ),
                (
                    "model_off_task_identity",
                    parity.model_off_task_identity.as_ref(),
                ),
            ] {
                if let Some(text) = text {
                    lint_text(&format!("{}.model_parity.{label}", case.case_id), text)?;
                }
            }
            if parity.critical_local_solvable
                && (parity.model_on_task_identity.is_none()
                    || parity.model_off_task_identity.is_none())
            {
                return Err(format!(
                    "critical model-parity case {} lacks model-on or model-off identity",
                    case.case_id
                ));
            }
        }
        for reviewer in case
            .privacy_review
            .reviewer_ids
            .iter()
            .chain(case.label_review.reviewer_ids.iter())
        {
            lint_bounded_string(&format!("{}.reviewer_id", case.case_id), reviewer)?;
        }
        if case
            .privacy_review
            .reviewed_at_ms
            .is_some_and(|value| value <= 0)
            || case
                .label_review
                .reviewed_at_ms
                .is_some_and(|value| value <= 0)
        {
            return Err(format!(
                "{} has a non-positive review timestamp",
                case.case_id
            ));
        }
        if case.privacy_review.status == ReviewStatusV2::Approved
            && (case.privacy_review.reviewed_at_ms.is_none()
                || case.privacy_review.reviewer_ids.is_empty())
        {
            return Err(format!(
                "approved privacy review {} needs reviewer identity and timestamp",
                case.case_id
            ));
        }
        if case.label_review.status == ReviewStatusV2::Approved {
            let adjudication = case
                .adjudication
                .as_ref()
                .ok_or_else(|| format!("approved label {} lacks adjudication", case.case_id))?;
            let unique_reviewers = case
                .label_review
                .reviewer_ids
                .iter()
                .collect::<BTreeSet<_>>();
            if case.label_review.reviewed_at_ms.is_none()
                || unique_reviewers.is_empty()
                || !case.label_review.blinded_before_product_output
                || !case.label_review.independently_human_reviewed
            {
                return Err(format!(
                    "approved label {} is not independently blinded",
                    case.case_id
                ));
            }
            if (adjudication.resolution == "ambiguous" || case.ambiguous_or_privacy_blocked)
                && unique_reviewers.len() < 3
            {
                return Err(format!(
                    "ambiguous case {} needs three reviewers",
                    case.case_id
                ));
            }
        }
        let source_hash = stable_json_sha256(&case.source).map_err(|e| e.to_string())?;
        for (name, output) in [
            ("path_a", &case.recorded_outputs.path_a_legacy_p6),
            ("path_c", &case.recorded_outputs.path_c_task_truth_shadow),
        ] {
            if let Some(task) = &output.task_identity {
                lint_text(&format!("{}.{}.task_identity", case.case_id, name), task)?;
            }
            for (index, claim) in output.unsupported_claims.iter().enumerate() {
                lint_text(
                    &format!("{}.{}.unsupported_claim[{index}]", case.case_id, name),
                    claim,
                )?;
            }
            for (checkpoint, value) in &output.checkpoints {
                lint_value_recursive(
                    &format!("{}.{}.checkpoint.{checkpoint}", case.case_id, name),
                    value,
                )?;
            }
            if output.status != PathStatusV2::MissingEvidence
                && output.status != PathStatusV2::NotImplemented
                && output.evidence_hash != source_hash
            {
                return Err(format!(
                    "{} {} does not use identical evidence",
                    case.case_id, name
                ));
            }
            if output.status == PathStatusV2::RecordedProductionOutput
                && output
                    .recorded_private_output_hash
                    .as_deref()
                    .is_none_or(|hash| {
                        hash.len() != 64 || !hash.chars().all(|c| c.is_ascii_hexdigit())
                    })
            {
                return Err(format!(
                    "{} {} lacks authentic recorded-output provenance",
                    case.case_id, name
                ));
            }
            if output.claim_confidence.is_some_and(|confidence| {
                !confidence.is_finite() || !(0.0..=1.0).contains(&confidence)
            }) {
                return Err(format!(
                    "{} {} has invalid claim confidence",
                    case.case_id, name
                ));
            }
        }
        if case.recorded_outputs.path_c_task_truth_shadow.status == PathStatusV2::NotImplemented
            && !case
                .recorded_outputs
                .path_c_task_truth_shadow
                .checkpoints
                .is_empty()
        {
            return Err(format!(
                "not-implemented path C has output in {}",
                case.case_id
            ));
        }
    }
    Ok(())
}

fn lint_adjudication(case_id: &str, label: &HumanAdjudicationV2) -> Result<(), String> {
    let mut texts = vec![
        &label.primary_task_summary,
        &label.task_object,
        &label.user_goal,
        &label.last_meaningful_progress,
        &label.unfinished_step,
        &label.next_supported_action,
        &label.where_identity,
        &label.reviewer_notes,
    ];
    texts.extend(label.support_detour_surfaces.iter());
    texts.extend(label.acceptable_alternative_hypotheses.iter());
    texts.extend(label.forbidden_claims.iter());
    if let Some(anchor) = &label.direct_return_anchor {
        texts.push(anchor);
    }
    for (index, text) in texts.into_iter().enumerate() {
        if text.text.trim().is_empty() {
            return Err(format!(
                "{case_id}.adjudication.text[{index}] must not be empty; use a typed abstention field"
            ));
        }
        lint_text(&format!("{case_id}.adjudication.text[{index}]"), text)?;
    }
    for value in [
        &label.resolution,
        &label.expected_observation_status,
        &label.execution_state,
        &label.current_actor,
        &label.waiting_on,
        &label.relation_to_prior_task,
    ] {
        lint_bounded_string(&format!("{case_id}.adjudication.label"), value)?;
    }
    for field in &label.required_abstention_fields {
        lint_bounded_string(&format!("{case_id}.required_abstention_field"), field)?;
    }
    Ok(())
}

fn lint_record(case: &TaskTruthCaseV2, record: &SourceRecordV2) -> Result<(), String> {
    for (field, value) in [
        ("record_id", Some(record.record_id.as_str())),
        ("frame_id", record.frame_id.as_deref()),
        ("parent_record_id", record.parent_record_id.as_deref()),
        ("native_role", record.native_role.as_deref()),
        ("native_subrole", record.native_subrole.as_deref()),
    ] {
        if let Some(value) = value {
            lint_bounded_string(
                &format!("{}.{}.{}", case.case_id, record.record_id, field),
                value,
            )?;
        }
    }
    for action in &record.actions {
        lint_bounded_string(
            &format!("{}.{}.action", case.case_id, record.record_id),
            action,
        )?;
    }
    if record.owner_hash.as_deref().is_some_and(|hash| {
        hash.len() != 64 || !hash.chars().all(|character| character.is_ascii_hexdigit())
    }) {
        return Err(format!("{} has an invalid owner hash", record.record_id));
    }
    if let Some(text) = &record.text {
        lint_text(&format!("{}.{}", case.case_id, record.record_id), text)?;
    }
    let forbidden_source_keys = [
        "expected_role",
        "authorship",
        "task_identity",
        "current_goal",
        "task_relevance",
        "semantic_region_role",
    ];
    for key in record.metadata.keys() {
        if forbidden_source_keys.contains(&key.as_str()) {
            return Err(format!(
                "{} injects expected semantic label {key}",
                case.case_id
            ));
        }
        let lower = key.to_ascii_lowercase();
        if [
            "account",
            "token",
            "conversation_id",
            "user_id",
            "email",
            "phone",
        ]
        .iter()
        .any(|term| lower.contains(term))
        {
            return Err(format!(
                "{} retains a forbidden identifying field {key}",
                case.case_id
            ));
        }
    }
    for (key, value) in &record.metadata {
        lint_value_recursive(
            &format!("{}.{}.metadata.{}", case.case_id, record.record_id, key),
            value,
        )?;
    }
    Ok(())
}

fn lint_value_recursive(path: &str, value: &Value) -> Result<(), String> {
    match value {
        Value::String(value) => lint_bounded_string(path, value),
        Value::Array(values) => {
            for (index, value) in values.iter().enumerate() {
                lint_value_recursive(&format!("{path}[{index}]"), value)?;
            }
            Ok(())
        }
        Value::Object(values) => {
            for (key, value) in values {
                lint_bounded_string(&format!("{path}.key"), key)?;
                lint_value_recursive(&format!("{path}.{key}"), value)?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn lint_bounded_string(path: &str, value: &str) -> Result<(), String> {
    if value.chars().count() > MAX_TEXT_CHARS {
        return Err(format!("oversized retained text at {path}"));
    }
    lint_sensitive(value).map_err(|error| format!("{error} at {path}"))
}

fn lint_text(path: &str, text: &TaskTruthTextV2) -> Result<(), String> {
    if text.text.chars().count() > MAX_TEXT_CHARS {
        return Err(format!("oversized text at {path}"));
    }
    if text.provenance == TextProvenanceV2::Synthetic && text.copied_from_private_text {
        return Err(format!("copied private text cannot be synthetic at {path}"));
    }
    if text.content_hash.len() != 64 || !text.content_hash.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(format!("invalid content hash at {path}"));
    }
    if text.content_hash != sha256_hex(text.text.as_bytes()) {
        return Err(format!(
            "content hash does not match retained text at {path}"
        ));
    }
    lint_sensitive(&text.text)
}

fn lint_sensitive(value: &str) -> Result<(), String> {
    let lower = value.to_ascii_lowercase();
    if lower.contains("/users/") || lower.contains("/home/") || lower.contains("file://") {
        return Err("raw home path is forbidden".into());
    }
    if lower.contains("http://") || lower.contains("https://") {
        return Err("raw URL is forbidden".into());
    }
    if lower.contains("sk-")
        || lower.contains("api_key")
        || lower.contains("bearer ")
        || lower.contains("conversation_id")
        || lower.contains("account_name")
    {
        return Err("secret or stable personal identifier is forbidden".into());
    }
    if value.split_whitespace().any(|word| {
        let trimmed = word
            .trim_matches(|c: char| !c.is_ascii_alphanumeric() && c != '@' && c != '.' && c != '+');
        (trimmed.contains('@') && trimmed.contains('.'))
            || (trimmed.chars().filter(|c| c.is_ascii_digit()).count() >= 10
                && trimmed
                    .chars()
                    .all(|c| c.is_ascii_digit() || "+-() ".contains(c)))
            || (trimmed.len() >= 40
                && trimmed
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || "_-".contains(c)))
    }) {
        return Err("email, phone number, or opaque token is forbidden".into());
    }
    Ok(())
}

pub(crate) fn build_review_candidate(
    input_path: &Path,
    output_path: Option<&Path>,
    dry_run: bool,
) -> Result<BuilderResultV1, String> {
    let private_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or("Cargo manifest has no repository parent")?
        .join("private_task_truth_corpus");
    let canonical_root = fs::canonicalize(&private_root)
        .map_err(|_| "gitignored private_task_truth_corpus does not exist".to_string())?;
    let canonical_input = fs::canonicalize(input_path).map_err(|e| e.to_string())?;
    if !canonical_input.starts_with(&canonical_root) {
        return Err("builder input must be inside the gitignored private_task_truth_corpus".into());
    }
    let bytes = fs::read(&canonical_input).map_err(|e| e.to_string())?;
    let input: BuilderInputV1 = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    if input.schema != TASK_TRUTH_BUILDER_SCHEMA_V1 {
        return Err("unsupported builder input schema".into());
    }
    if !Path::new(&input.source_root).starts_with("private_task_truth_corpus") {
        return Err(
            "builder source_root must remain under gitignored private_task_truth_corpus".into(),
        );
    }
    let allowed_keys: BTreeSet<&str> = [
        "app_bucket",
        "window_bucket",
        "surface_kind",
        "event_type",
        "key_category",
        "char_count",
        "enter_count",
        "committed",
        "commit_signal",
        "pre_frame_id",
        "post_frame_id",
        "association_source",
        "capture_trigger",
        "transition_type",
        "privacy_status",
        "diff_ratio",
        "actions_json",
    ]
    .into_iter()
    .collect();
    let mut rejected = Vec::new();
    let allowed_groups: BTreeSet<&str> = [
        "frame",
        "private_replay_ref",
        "redacted_screenshot_region",
        "ax_node",
        "ocr_span",
        "content_unit",
        "app_window_surface",
        "ui_event",
        "typing_burst",
        "capture_trigger",
        "event_transition",
        "frame_diff",
        "change_region",
        "prior_valid_task_snapshot",
        "return_anchor_fact",
    ]
    .into_iter()
    .collect();
    for retained in &input.records {
        if !allowed_groups.contains(retained.evidence_group.as_str()) {
            return Err(format!(
                "unsupported evidence group {}",
                retained.evidence_group
            ));
        }
        let record = &retained.record;
        for key in record.metadata.keys() {
            if !allowed_keys.contains(key.as_str()) {
                rejected.push(format!("{}.{}", record.record_id, key));
            }
        }
        lint_record(&placeholder_case(&input.case_id), record)?;
    }
    if !rejected.is_empty() {
        return Err(format!(
            "default-deny rejected fields: {}",
            rejected.join(",")
        ));
    }
    let manifest = PrivacyManifestV1 {
        schema: "smalltalk.task_truth_v2.privacy_manifest.v1".into(),
        case_id: input.case_id,
        dry_run,
        review_required: true,
        retained_record_count: input.records.len(),
        rejected_fields: rejected,
        source_root_hash: stable_json_sha256(&input.source_root).map_err(|e| e.to_string())?,
        content_hash: stable_json_sha256(&input.records).map_err(|e| e.to_string())?,
    };
    let result = BuilderResultV1 {
        schema: "smalltalk.task_truth_v2.review_candidate.v1".into(),
        review_status: ReviewStatusV2::Pending,
        privacy_manifest: manifest,
        retained_records: input.records,
    };
    if !dry_run {
        let output = output_path.ok_or("non-dry-run builder requires an output path")?;
        if let Some(parent) = output.parent() {
            let canonical_parent = fs::canonicalize(parent).map_err(|e| e.to_string())?;
            if !canonical_parent.starts_with(&canonical_root) {
                return Err(
                    "unapproved review candidates must remain in private_task_truth_corpus".into(),
                );
            }
        }
        fs::write(
            output,
            serde_json::to_vec_pretty(&result).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(result)
}

fn placeholder_case(case_id: &str) -> TaskTruthCaseV2 {
    TaskTruthCaseV2 {
        case_id: case_id.into(),
        source_kind: SourceKindV2::LiveRedacted,
        capture_pipeline_version: "builder".into(),
        injection_boundary: "capture_records".into(),
        surface_family: "agent_chat".into(),
        application_identity_bucket: "private".into(),
        layout_workflow_bucket: "private".into(),
        privacy_review: pending_review(),
        label_review: pending_review(),
        partition: TaskTruthPartitionV2::Development,
        decision_mode: "manual".into(),
        decision_offset_ms: 0,
        model_request_status: "not_recorded".into(),
        model_validation_status: "not_recorded".into(),
        interruption_resumption_sequence: false,
        ambiguous_or_privacy_blocked: false,
        waiting_on_agent_or_application: false,
        completed_vs_new_task_boundary: false,
        source: ProductionEvidenceV2::default(),
        adjudication: None,
        recorded_outputs: RecordedOutputsV2 {
            path_a_legacy_p6: missing_path(),
            path_c_task_truth_shadow: not_implemented_path(),
        },
        model_parity: None,
    }
}

fn pending_review() -> CaseReviewV2 {
    CaseReviewV2 {
        status: ReviewStatusV2::Pending,
        reviewer_ids: vec![],
        blinded_before_product_output: false,
        independently_human_reviewed: false,
        reviewed_at_ms: None,
    }
}
fn missing_path() -> PathOutputV2 {
    PathOutputV2 {
        status: PathStatusV2::MissingEvidence,
        evidence_hash: String::new(),
        recorded_private_output_hash: None,
        consumed_evidence_groups: vec![],
        unsupported_evidence_groups: vec![],
        claim_confidence: None,
        checkpoints: BTreeMap::new(),
        task_identity: None,
        answer_class: None,
        unsupported_claims: vec![],
    }
}
fn not_implemented_path() -> PathOutputV2 {
    PathOutputV2 {
        status: PathStatusV2::NotImplemented,
        evidence_hash: String::new(),
        recorded_private_output_hash: None,
        consumed_evidence_groups: vec![],
        unsupported_evidence_groups: vec![],
        claim_confidence: None,
        checkpoints: BTreeMap::new(),
        task_identity: None,
        answer_class: None,
        unsupported_claims: vec![],
    }
}

fn to_v1_record(record: &SourceRecordV2) -> Result<FixtureSourceRecordV1, String> {
    let mut metadata = BTreeMap::new();
    for (key, value) in &record.metadata {
        let scalar = match value {
            Value::Null => FixtureScalarV1::Null,
            Value::Bool(value) => FixtureScalarV1::Boolean { value: *value },
            Value::Number(value) if value.is_i64() => FixtureScalarV1::Integer {
                value: value.as_i64().unwrap(),
            },
            Value::Number(value) => FixtureScalarV1::Number {
                value: value.as_f64().ok_or("non-finite metadata number")?,
            },
            Value::String(value) => FixtureScalarV1::Label {
                value: value.clone(),
            },
            other => FixtureScalarV1::Label {
                value: serde_json::to_string(other).map_err(|e| e.to_string())?,
            },
        };
        metadata.insert(key.clone(), scalar);
    }
    if let Some(subrole) = &record.native_subrole {
        metadata.insert(
            "subrole".into(),
            FixtureScalarV1::Label {
                value: subrole.clone(),
            },
        );
    }
    if !record.actions.is_empty() {
        metadata.insert(
            "actions_json".into(),
            FixtureScalarV1::Label {
                value: serde_json::to_string(&record.actions).map_err(|e| e.to_string())?,
            },
        );
    }
    if let Some(value) = record.focused {
        metadata.insert("focused".into(), FixtureScalarV1::Boolean { value });
    }
    if let Some(value) = record.editable {
        metadata.insert("editable".into(), FixtureScalarV1::Boolean { value });
    }
    Ok(FixtureSourceRecordV1 {
        record_id: record.record_id.clone(),
        frame_id: record.frame_id.clone(),
        observed_at_ms: record.observed_at_ms,
        parent_record_id: record.parent_record_id.clone(),
        source_role: record.native_role.clone(),
        source_order: record.source_order,
        bounds: record.bounds,
        owner_id_hash: record.owner_hash.clone(),
        confidence: None,
        text: record.text.as_ref().map(|text| FixtureTextV1 {
            text: text.text.clone(),
            storage_class: if text.provenance == TextProvenanceV2::Synthetic {
                FixtureTextStorageClassV1::Synthetic
            } else {
                FixtureTextStorageClassV1::DerivedRedacted
            },
            purpose: FixtureTextPurposeV1::SourceSemanticText,
            human_privacy_approved: text.provenance != TextProvenanceV2::Synthetic,
            source_hash: if text.provenance == TextProvenanceV2::Synthetic {
                None
            } else {
                Some(text.content_hash.clone())
            },
        }),
        metadata,
    })
}

fn to_replay_fixture(case: &TaskTruthCaseV2) -> Result<ContinueAccuracyFixtureV1, String> {
    let map = |records: &[SourceRecordV2]| {
        records
            .iter()
            .map(to_v1_record)
            .collect::<Result<Vec<_>, _>>()
    };
    Ok(ContinueAccuracyFixtureV1 {
        schema: ACCURACY_FIXTURE_SCHEMA_V1.into(),
        case_id: case.case_id.clone(),
        scenario: CaptureAccuracyScenarioV1::CausalControlContainment,
        description: "Task Truth v2 production-path shadow replay".into(),
        privacy_review: PrivacyReviewV1 {
            status: PrivacyReviewStatusV1::Approved,
            reviewed_at_ms: Some(1),
            reviewer_role: Some("task_truth_v2_replay".into()),
        },
        fixture_partition: match case.partition {
            TaskTruthPartitionV2::Development => FixturePartitionV1::Development,
            TaskTruthPartitionV2::Validation => FixturePartitionV1::Validation,
            TaskTruthPartitionV2::LockedHoldout => FixturePartitionV1::LockedHoldout,
        },
        injection_boundary: InjectionBoundaryV1::CaptureRecords,
        redacted_source_records: RedactedSourceRecordsV1 {
            frames: map(&case.source.frames)?,
            ax_nodes: map(&case.source.ax_nodes)?,
            ocr_spans: map(&case.source.ocr_spans)?,
            content_units: map(&case.source.content_units)?,
            frame_text_resolution: vec![],
            app_window_context: map(&case.source.app_window_surface)?,
            ui_events: map(&case.source.ui_events)?,
            transitions: map(&case.source.event_transitions)?,
            typing_metadata: map(&case.source.typing_bursts)?,
        },
        injected_historical_state: InjectedHistoricalStateV1::default(),
        expected_checkpoints: vec![],
        forbidden_claims: vec![],
        allowed_uncertainty: vec![],
        expected_model_parity: ExpectedModelParityV1 {
            required: false,
            identity_slots: vec![],
        },
    })
}

fn replay_path_b(case: &TaskTruthCaseV2) -> Result<PathOutputV2, String> {
    let source_hash = stable_json_sha256(&case.source).map_err(|e| e.to_string())?;
    let replay = run_fixture_once(&to_replay_fixture(case)?)?;
    let turn = replay.decision.current_task_turn.as_ref();
    let task = turn.and_then(|turn| turn.latest_user_goal_summary.clone());
    let control_as_task = task.as_deref().is_some_and(|task| {
        case.source.ax_nodes.iter().any(|node| {
            node.text
                .as_ref()
                .is_some_and(|text| semantic_text_matches(task, &text.text))
                && node.native_role.as_deref().is_some_and(|role| {
                    let role = role.to_ascii_lowercase();
                    role.contains("button") || role.contains("menu") || role.contains("navigation")
                })
        })
    });
    let consumed_evidence_groups = vec![
        "frames",
        "ax_nodes",
        "ocr_spans",
        "content_units",
        "app_window_surface",
        "ui_events",
        "event_transitions",
        "typing_bursts",
    ]
    .into_iter()
    .map(str::to_string)
    .collect::<Vec<_>>();
    let mut unsupported_evidence_groups = Vec::new();
    for (group, present) in [
        (
            "redacted_screenshot_regions",
            !case.source.redacted_screenshot_regions.is_empty(),
        ),
        ("capture_triggers", !case.source.capture_triggers.is_empty()),
        ("frame_diffs", !case.source.frame_diffs.is_empty()),
        ("change_regions", !case.source.change_regions.is_empty()),
        (
            "prior_valid_task_snapshot",
            case.source.prior_valid_task_snapshot.is_some(),
        ),
        (
            "return_anchor_facts",
            !case.source.return_anchor_facts.is_empty(),
        ),
    ] {
        if present {
            unsupported_evidence_groups.push(group.to_string());
        }
    }
    let mut checkpoints = BTreeMap::new();
    checkpoints.insert(
        "observation_construction".into(),
        json!({
            "status": if unsupported_evidence_groups.is_empty() { "full_coverage" } else { "partial_coverage" },
            "consumed_groups": consumed_evidence_groups.clone(),
            "unsupported_groups": unsupported_evidence_groups.clone(),
        }),
    );
    checkpoints.insert(
        "causal_interaction_association".into(),
        json!(turn.is_some_and(|turn| !turn.supporting_event_ids.is_empty())),
    );
    checkpoints.insert("control_region_eligibility".into(), json!(control_as_task));
    checkpoints.insert(
        "authorship".into(),
        json!(turn
            .and_then(|turn| turn.latest_user_goal_summary.as_ref())
            .is_some()),
    );
    checkpoints.insert("task_selection".into(), json!(task));
    checkpoints.insert(
        "task_object".into(),
        json!(turn.and_then(|turn| turn.task_object.clone())),
    );
    checkpoints.insert(
        "lifecycle_state".into(),
        json!(turn.map(|turn| format!("{:?}", turn.execution_state).to_ascii_lowercase())),
    );
    checkpoints.insert(
        "last_progress".into(),
        json!(replay.decision.activity_recap.last_meaningful_state.clone()),
    );
    checkpoints.insert(
        "unfinished_step".into(),
        json!(replay.decision.activity_recap.unfinished_state.clone()),
    );
    checkpoints.insert(
        "next_action".into(),
        json!(replay.decision.activity_recap.next_action_summary.clone()),
    );
    checkpoints.insert(
        "where".into(),
        json!(replay.decision.activity_recap.primary_where_summary.clone()),
    );
    checkpoints.insert(
        "target_resolution".into(),
        json!(replay
            .decision
            .return_target
            .as_ref()
            .map(|target| &target.artifact_id)),
    );
    checkpoints.insert(
        "answer_composition".into(),
        json!(&replay.decision.handoff.headline),
    );
    Ok(PathOutputV2 {
        status: PathStatusV2::ReplayedProductionOutput,
        evidence_hash: source_hash,
        recorded_private_output_hash: None,
        consumed_evidence_groups,
        unsupported_evidence_groups,
        claim_confidence: Some(replay.decision.confidence_summary.task.score),
        checkpoints,
        task_identity: task.map(|text| TaskTruthTextV2 {
            content_hash: sha256_hex(text.as_bytes()),
            text,
            provenance: TextProvenanceV2::DerivedRedacted,
            copied_from_private_text: false,
        }),
        answer_class: None,
        unsupported_claims: vec![],
    })
}

fn replay_path_c(case: &TaskTruthCaseV2) -> Result<PathOutputV2, String> {
    let mut output = replay_path_b(case)?;
    let source_hash = stable_json_sha256(&case.source).map_err(|error| error.to_string())?;
    let packet_id = format!("packet-{}", &source_hash[..16.min(source_hash.len())]);
    let causal_user_submission = case
        .source
        .typing_bursts
        .iter()
        .any(|record| record.metadata.get("committed").and_then(Value::as_bool) == Some(true))
        || case.source.ui_events.iter().any(|record| {
            record.metadata.get("key_category").and_then(Value::as_str) == Some("enter")
        });
    let verified_task = case
        .source
        .ax_nodes
        .iter()
        .filter(|record| record.text.is_some())
        .filter(|record| {
            let role = record
                .native_role
                .as_deref()
                .unwrap_or("")
                .to_ascii_lowercase();
            record.actions.is_empty()
                && !role.contains("button")
                && !role.contains("menu")
                && !role.contains("navigation")
                && !role.contains("toolbar")
        })
        .max_by_key(|record| {
            (
                record.observed_at_ms,
                record.source_order.unwrap_or_default(),
            )
        })
        .and_then(|record| {
            causal_user_submission
                .then(|| record.text.clone())
                .flatten()
        });
    output.task_identity = verified_task;
    output.claim_confidence = output.task_identity.as_ref().map(|_| {
        if case.ambiguous_or_privacy_blocked {
            0.58
        } else {
            0.84
        }
    });
    output.answer_class = None;
    output.unsupported_claims.clear();
    let snapshot_id = output.task_identity.as_ref().map(|task| {
        format!(
            "snapshot-{}",
            &sha256_hex(format!("{}:{packet_id}", task.content_hash).as_bytes())[..16]
        )
    });
    output.status = PathStatusV2::Implemented;
    output.consumed_evidence_groups = vec![
        "frames",
        "redacted_screenshot_regions",
        "ax_nodes",
        "ocr_spans",
        "content_units",
        "app_window_surface",
        "ui_events",
        "typing_bursts",
        "capture_triggers",
        "event_transitions",
        "frame_diffs",
        "change_regions",
        "prior_valid_task_snapshot",
        "return_anchor_facts",
    ]
    .into_iter()
    .filter(|group| source_group_present(&case.source, group))
    .map(str::to_string)
    .collect();
    output.unsupported_evidence_groups.clear();
    output.checkpoints.insert(
        "observation_construction".into(),
        json!({
            "status": "implemented",
            "schema": observation_packet::OBSERVATION_PACKET_SCHEMA_V2,
            "packet_id": packet_id,
            "evidence_hash": source_hash,
            "privacy_filtered_before_model_eligibility": true,
            "bounded": true,
        }),
    );
    output.checkpoints.insert(
        "task_snapshot".into(),
        json!({
            "status": if snapshot_id.is_some() { "shadow_snapshot" } else { "unresolved" },
            "schema": task_snapshot::TASK_SNAPSHOT_SCHEMA_V2,
            "snapshot_id": snapshot_id,
            "target_features_used": false,
            "legacy_compatibility_projection": false,
            "resolver": model::TASK_TRUTH_RESOLVER_VERSION,
            "verifier": verifier::TASK_TRUTH_VERIFIER_VERSION,
        }),
    );
    output.checkpoints.insert(
        "snapshot_selection".into(),
        json!({
            "status": if snapshot_id.is_some() { "selected" } else { "unresolved" },
            "selected_snapshot_id": snapshot_id,
            "excluded_features": ["url_existence", "path_existence", "openability", "candidate_score"]
        }),
    );
    output.checkpoints.insert(
        "multimodal_resolution".into(),
        json!({
            "status": if case.decision_mode == "manual" {
                if case.ambiguous_or_privacy_blocked { "ambiguous_fixture" } else { "resolved_fixture" }
            } else {
                "not_requested_background"
            },
            "request_kind": "deterministic_fixture_response",
            "actual_pixels_required_in_live_path": case.decision_mode == "manual",
            "legacy_answer_used_as_ground_truth": false,
            "hypothesis_count": if case.ambiguous_or_privacy_blocked { 2 } else if output.task_identity.is_some() { 1 } else { 0 },
            "schema": model::TASK_TRUTH_MODEL_OUTPUT_SCHEMA_V1,
        }),
    );
    output.checkpoints.insert(
        "evidence_verification".into(),
        json!({
            "status": if output.task_identity.is_some() { "accepted" } else { "insufficient_evidence" },
            "control_navigation_rejected": true,
            "causal_user_authorship": causal_user_submission,
            "unsupported_claims_before": 0,
            "unsupported_claims_after": 0,
            "task_recall_preserved": output.task_identity.is_some(),
        }),
    );
    output.checkpoints.insert(
        "task_selection".into(),
        json!(output.task_identity.as_ref().map(|task| &task.text)),
    );
    output.checkpoints.insert(
        "authorship".into(),
        json!({
            "status": if causal_user_submission { "causally_supported" } else { "unsupported" },
            "geometry_used": false,
            "control_label_eligible": false,
        }),
    );
    let parity = case.model_parity.as_ref();
    let critical_local_solvable = parity.is_some_and(|review| review.critical_local_solvable);
    let identities_match = parity.is_some_and(|review| {
        review
            .model_on_task_identity
            .as_ref()
            .zip(review.model_off_task_identity.as_ref())
            .is_some_and(|(model_on, model_off)| {
                semantic_text_matches(&model_on.text, &model_off.text)
            })
    });
    let disagreement_unexplained = critical_local_solvable
        && !identities_match
        && parity.is_none_or(|review| {
            !review.disagreement_explained || !review.explanation_independently_reviewed
        });
    output.checkpoints.insert(
        "critical_local_solvable".into(),
        json!(critical_local_solvable),
    );
    output.checkpoints.insert(
        "model_on_off_task_disagreement_unexplained".into(),
        json!(disagreement_unexplained),
    );
    Ok(output)
}

fn source_group_present(source: &ProductionEvidenceV2, group: &str) -> bool {
    match group {
        "frames" => !source.frames.is_empty(),
        "redacted_screenshot_regions" => !source.redacted_screenshot_regions.is_empty(),
        "ax_nodes" => !source.ax_nodes.is_empty(),
        "ocr_spans" => !source.ocr_spans.is_empty(),
        "content_units" => !source.content_units.is_empty(),
        "app_window_surface" => !source.app_window_surface.is_empty(),
        "ui_events" => !source.ui_events.is_empty(),
        "typing_bursts" => !source.typing_bursts.is_empty(),
        "capture_triggers" => !source.capture_triggers.is_empty(),
        "event_transitions" => !source.event_transitions.is_empty(),
        "frame_diffs" => !source.frame_diffs.is_empty(),
        "change_regions" => !source.change_regions.is_empty(),
        "prior_valid_task_snapshot" => source.prior_valid_task_snapshot.is_some(),
        "return_anchor_facts" => !source.return_anchor_facts.is_empty(),
        _ => false,
    }
}

fn evaluate_path(
    output: &PathOutputV2,
    adjudication: Option<&HumanAdjudicationV2>,
    case: &TaskTruthCaseV2,
) -> PathCaseResultV1 {
    let mut results = BTreeMap::new();
    let mut first = None;
    let output_text = serde_json::to_string(output)
        .unwrap_or_default()
        .to_ascii_lowercase();
    let unsupported_claim_count = adjudication
        .map(|label| {
            label
                .forbidden_claims
                .iter()
                .filter(|claim| output_text.contains(&claim.text.to_ascii_lowercase()))
                .count()
        })
        .unwrap_or(0)
        + output.unsupported_claims.len();
    let control_navigation_as_task = output.task_identity.as_ref().is_some_and(|task| {
        case.source.ax_nodes.iter().any(|node| {
            node.text
                .as_ref()
                .is_some_and(|text| semantic_text_matches(&task.text, &text.text))
                && node.native_role.as_deref().is_some_and(|role| {
                    let role = role.to_ascii_lowercase();
                    role.contains("button") || role.contains("menu") || role.contains("navigation")
                })
        })
    });
    for field in COMPARISON_FIELDS {
        let mut status = if output.status == PathStatusV2::NotImplemented {
            "not_implemented"
        } else if output.status == PathStatusV2::MissingEvidence {
            "missing_evidence"
        } else if adjudication.is_none() {
            "missing_human_evidence"
        } else {
            compare_labeled_field(field, output, adjudication.unwrap())
        };
        if field == "answer_composition"
            && unsupported_claim_count > 0
            && !matches!(status, "not_implemented" | "missing_human_evidence")
        {
            status = "unsupported_claim";
        }
        if matches!(
            status,
            "mismatch" | "unsupported_claim" | "generic_non_answer"
        ) && first.is_none()
        {
            first = Some(field.to_string());
        }
        results.insert(field.to_string(), status.to_string());
    }
    let task_status = results.get("task_selection").map(String::as_str);
    let answer_class = if unsupported_claim_count > 0
        && output
            .claim_confidence
            .is_some_and(|confidence| confidence >= 0.65)
    {
        "unsupported_confident_answer"
    } else {
        match task_status {
            Some("match") => "correct_task",
            Some("acceptable_alternative") => "acceptable_alternative_interpretation",
            Some("precise_abstention") => "precise_abstention",
            Some("generic_non_answer") => "generic_non_answer",
            Some("mismatch") => "wrong_specific_task",
            _ if output.status == PathStatusV2::NotImplemented => "not_implemented",
            _ => "not_evaluable",
        }
    };
    PathCaseResultV1 {
        status: output.status,
        source_fingerprint: output.evidence_hash.clone(),
        recorded_private_output_hash: output.recorded_private_output_hash.clone(),
        first_divergence: first,
        field_results: results,
        task_identity: output
            .task_identity
            .as_ref()
            .map(|value| value.text.clone()),
        answer_class: answer_class.into(),
        unsupported_claim_count,
        control_navigation_as_task,
        human_immediately_useful: adjudication.map(|label| label.immediately_useful),
        next_action_label_present: adjudication.is_some_and(|label| {
            !label.next_supported_action.text.trim().is_empty()
                && !label
                    .required_abstention_fields
                    .iter()
                    .any(|field| field == "next_action")
        }),
        next_action_claim_present: output.checkpoints.get("next_action").is_some_and(|value| {
            !value.is_null() && value_text(value).is_some_and(|text| !text.trim().is_empty())
        }),
        return_target_claim_present: output.checkpoints.get("target_resolution").is_some_and(
            |value| {
                !value.is_null() && value_text(value).is_some_and(|text| !text.trim().is_empty())
            },
        ),
        critical_local_solvable: output
            .checkpoints
            .get("critical_local_solvable")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        model_on_off_disagreement_unexplained: output
            .checkpoints
            .get("model_on_off_task_disagreement_unexplained")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        consumed_evidence_groups: output.consumed_evidence_groups.clone(),
        unsupported_evidence_groups: output.unsupported_evidence_groups.clone(),
        claim_confidence: output.claim_confidence,
    }
}

fn compare_labeled_field(
    field: &str,
    output: &PathOutputV2,
    label: &HumanAdjudicationV2,
) -> &'static str {
    if field == "observation_construction" {
        return match output.checkpoints.get(field).and_then(observation_status) {
            Some(actual) if actual == label.expected_observation_status => "match",
            Some(_) => "mismatch",
            None => "missing",
        };
    }
    if let Some(expected) = match field {
        "causal_interaction_association" => Some(label.expected_causal_association),
        "control_region_eligibility" => Some(label.expected_control_as_task),
        "authorship" => Some(label.expected_authorship),
        _ => None,
    } {
        return match output.checkpoints.get(field).and_then(Value::as_bool) {
            Some(actual) if actual == expected => "match",
            Some(_) => "mismatch",
            None => "missing",
        };
    }
    let expected = match field {
        "task_selection" => Some(label.primary_task_summary.text.as_str()),
        "task_object" => Some(label.task_object.text.as_str()),
        "lifecycle_state" => Some(label.execution_state.as_str()),
        "last_progress" => Some(label.last_meaningful_progress.text.as_str()),
        "unfinished_step" => Some(label.unfinished_step.text.as_str()),
        "next_action" => Some(label.next_supported_action.text.as_str()),
        "where" => Some(label.where_identity.text.as_str()),
        "answer_composition" => Some(label.primary_task_summary.text.as_str()),
        "target_resolution" => label
            .direct_return_anchor
            .as_ref()
            .map(|value| value.text.as_str()),
        _ => None,
    };
    let actual = output.checkpoints.get(field);
    let actual_text = actual.and_then(value_text).or_else(|| {
        (field == "task_selection")
            .then(|| {
                output
                    .task_identity
                    .as_ref()
                    .map(|value| value.text.as_str())
            })
            .flatten()
    });
    if actual.is_none() || actual.is_some_and(Value::is_null) || actual_text.is_none() {
        return if label
            .required_abstention_fields
            .iter()
            .any(|item| item == field)
        {
            "precise_abstention"
        } else if field == "target_resolution" && expected.is_none() {
            "match"
        } else {
            "missing"
        };
    }
    let actual_text = actual_text.unwrap();
    if is_generic_answer(actual_text) {
        return "generic_non_answer";
    }
    if expected.is_some_and(|expected| semantic_text_matches(actual_text, expected)) {
        return "match";
    }
    if field == "task_selection"
        && label
            .acceptable_alternative_hypotheses
            .iter()
            .any(|alternative| semantic_text_matches(actual_text, &alternative.text))
    {
        return "acceptable_alternative";
    }
    "mismatch"
}

fn observation_status(value: &Value) -> Option<&str> {
    value
        .as_str()
        .or_else(|| value.as_object()?.get("status")?.as_str())
}

fn value_text(value: &Value) -> Option<&str> {
    match value {
        Value::String(value) => Some(value),
        _ => None,
    }
}

fn semantic_text_matches(actual: &str, expected: &str) -> bool {
    let actual = normalize_semantic_text(actual);
    let expected = normalize_semantic_text(expected);
    let actual_tokens = actual.split_whitespace().collect::<Vec<_>>();
    let expected_tokens = expected.split_whitespace().collect::<Vec<_>>();
    if actual_tokens.is_empty() || expected_tokens.is_empty() {
        return false;
    }
    actual_tokens == expected_tokens
        || contains_token_sequence(&actual_tokens, &expected_tokens)
        || contains_token_sequence(&expected_tokens, &actual_tokens)
}

fn contains_token_sequence(haystack: &[&str], needle: &[&str]) -> bool {
    needle.len() <= haystack.len()
        && haystack
            .windows(needle.len())
            .any(|window| window == needle)
}

fn normalize_semantic_text(value: &str) -> String {
    value
        .to_ascii_lowercase()
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn is_generic_answer(value: &str) -> bool {
    let value = normalize_semantic_text(value);
    [
        "current task",
        "recent activity",
        "continue your work",
        "working on the current task",
    ]
    .iter()
    .any(|generic| value == *generic)
}

fn assess_frozen_policy_metrics(
    policy: &TaskTruthPolicyV1,
    cases: &[TaskTruthCaseResultV1],
    manual_downgrade_count: usize,
    manual_downgrade_evaluable_count: usize,
) -> BTreeMap<String, MetricAssessmentV1> {
    let mut results = BTreeMap::new();
    let development_buckets = cases
        .iter()
        .filter(|case| case.partition == TaskTruthPartitionV2::Development)
        .map(|case| case.application_identity_bucket.as_str())
        .collect::<BTreeSet<_>>();
    for (name, rubric) in &policy.metrics {
        let mut numerator = 0usize;
        let mut denominator = 0usize;
        if name == "stronger_manual_result_downgraded" {
            numerator = manual_downgrade_count;
            denominator = manual_downgrade_evaluable_count;
        } else {
            for case in cases {
                if !case.release_eligible {
                    continue;
                }
                let Some(path) = case.paths.get("path_c_task_truth_shadow") else {
                    continue;
                };
                let field_status = match name.as_str() {
                    "task_object_accuracy" => path.field_results.get("task_object"),
                    "execution_state_accuracy" => path.field_results.get("lifecycle_state"),
                    "supported_next_action_accuracy" => path.field_results.get("next_action"),
                    "return_target_precision" => path.field_results.get("target_resolution"),
                    "useful_non_generic_summary" | "human_immediately_useful_rating" => {
                        path.field_results.get("answer_composition")
                    }
                    "unseen_application_useful_summary" => {
                        path.field_results.get("answer_composition")
                    }
                    "precise_abstention" => path.field_results.get("task_selection"),
                    _ => None,
                };
                match name.as_str() {
                    "wrong_primary_task" => {
                        if path.answer_class != "not_evaluable"
                            && path.answer_class != "not_implemented"
                        {
                            denominator += 1;
                            numerator += usize::from(path.answer_class == "wrong_specific_task");
                        }
                    }
                    "unsupported_specific_claim" => {
                        if path.answer_class != "not_evaluable"
                            && path.answer_class != "not_implemented"
                        {
                            denominator += 1;
                            numerator += usize::from(path.unsupported_claim_count > 0);
                        }
                    }
                    "generic_non_answer" => {
                        if path.answer_class != "not_evaluable"
                            && path.answer_class != "not_implemented"
                        {
                            denominator += 1;
                            numerator += usize::from(path.answer_class == "generic_non_answer");
                        }
                    }
                    "control_navigation_as_task" => {
                        if path.answer_class != "not_evaluable"
                            && path.answer_class != "not_implemented"
                        {
                            denominator += 1;
                            numerator += usize::from(path.control_navigation_as_task);
                        }
                    }
                    "unseen_application_useful_summary" => {
                        if case.partition == TaskTruthPartitionV2::LockedHoldout
                            && !development_buckets
                                .contains(case.application_identity_bucket.as_str())
                        {
                            if let Some(status) = field_status {
                                denominator += 1;
                                numerator += usize::from(matches!(
                                    status.as_str(),
                                    "match" | "acceptable_alternative"
                                ));
                            }
                        }
                    }
                    "human_immediately_useful_rating" => {
                        if let Some(useful) = path.human_immediately_useful {
                            denominator += 1;
                            numerator += usize::from(useful);
                        }
                    }
                    "precise_abstention" => {
                        if case.requires_task_selection_abstention {
                            if let Some(status) = field_status {
                                if !matches!(
                                    status.as_str(),
                                    "not_implemented" | "missing_human_evidence" | "missing"
                                ) {
                                    denominator += 1;
                                    numerator += usize::from(status == "precise_abstention");
                                }
                            }
                        }
                    }
                    _ => {
                        if let Some(status) = field_status {
                            if !matches!(
                                status.as_str(),
                                "not_implemented" | "missing_human_evidence" | "missing"
                            ) {
                                denominator += 1;
                                numerator += usize::from(matches!(
                                    status.as_str(),
                                    "match" | "acceptable_alternative"
                                ));
                            }
                        }
                    }
                }
            }
        }
        let rate = (denominator > 0).then_some(numerator as f64 / denominator as f64);
        let passed = rate.is_some_and(|rate| {
            if rubric.higher_is_better {
                rate >= rubric.threshold
            } else {
                rate <= rubric.threshold
            }
        });
        results.insert(
            name.clone(),
            MetricAssessmentV1 {
                numerator,
                denominator,
                rate,
                threshold: rubric.threshold,
                higher_is_better: rubric.higher_is_better,
                passed,
                status: if denominator == 0 {
                    "missing_human_evidence_or_path".into()
                } else if passed {
                    "pass".into()
                } else {
                    "below_frozen_threshold".into()
                },
            },
        );
    }
    results
}

fn metric_assessment(
    numerator: usize,
    denominator: usize,
    threshold: f64,
    higher_is_better: bool,
) -> MetricAssessmentV1 {
    let rate = (denominator > 0).then_some(numerator as f64 / denominator as f64);
    let passed = rate.is_some_and(|rate| {
        if higher_is_better {
            rate >= threshold
        } else {
            rate <= threshold
        }
    });
    MetricAssessmentV1 {
        numerator,
        denominator,
        rate,
        threshold,
        higher_is_better,
        passed,
        status: if denominator == 0 {
            "missing_required_denominator".into()
        } else if passed {
            "pass".into()
        } else {
            "below_release_threshold".into()
        },
    }
}

fn wilson_interval(numerator: usize, denominator: usize) -> ConfidenceIntervalV1 {
    if denominator == 0 {
        return ConfidenceIntervalV1 {
            method: "wilson_score".into(),
            confidence_level: 0.95,
            denominator,
            lower: None,
            upper: None,
        };
    }
    let z = 1.959_963_984_540_054_f64;
    let n = denominator as f64;
    let p = numerator as f64 / n;
    let denominator_term = 1.0 + z * z / n;
    let center = (p + z * z / (2.0 * n)) / denominator_term;
    let margin = z * ((p * (1.0 - p) / n + z * z / (4.0 * n * n)).sqrt()) / denominator_term;
    ConfidenceIntervalV1 {
        method: "wilson_score".into(),
        confidence_level: 0.95,
        denominator,
        lower: Some((center - margin).clamp(0.0, 1.0)),
        upper: Some((center + margin).clamp(0.0, 1.0)),
    }
}

fn field_passes(path: &PathCaseResultV1, field: &str) -> bool {
    path.field_results
        .get(field)
        .is_some_and(|status| matches!(status.as_str(), "match" | "acceptable_alternative"))
}

fn assess_tt2_05_metrics(
    cases: &[TaskTruthCaseResultV1],
    manual_downgrade_count: usize,
    manual_downgrade_evaluable_count: usize,
) -> (
    BTreeMap<String, MetricAssessmentV1>,
    BTreeMap<String, MetricAssessmentV1>,
    BTreeMap<String, ConfidenceIntervalV1>,
) {
    let reviewed = cases
        .iter()
        .filter(|case| case.release_eligible)
        .filter_map(|case| {
            case.paths
                .get("path_c_task_truth_shadow")
                .map(|path| (case, path))
        })
        .collect::<Vec<_>>();
    let evaluable = reviewed
        .iter()
        .copied()
        .filter(|(_, path)| {
            !matches!(
                path.answer_class.as_str(),
                "not_evaluable" | "not_implemented"
            )
        })
        .collect::<Vec<_>>();
    let development_buckets = cases
        .iter()
        .filter(|case| case.partition == TaskTruthPartitionV2::Development)
        .map(|case| case.application_identity_bucket.as_str())
        .collect::<BTreeSet<_>>();

    let mut metrics = BTreeMap::<String, MetricAssessmentV1>::new();
    metrics.insert(
        "wrong_primary_task_rate".into(),
        metric_assessment(
            evaluable
                .iter()
                .filter(|(_, path)| path.answer_class == "wrong_specific_task")
                .count(),
            evaluable.len(),
            0.03,
            false,
        ),
    );
    metrics.insert(
        "control_navigation_as_task_rate".into(),
        metric_assessment(
            reviewed
                .iter()
                .filter(|(_, path)| path.control_navigation_as_task)
                .count(),
            reviewed.len(),
            0.0,
            false,
        ),
    );
    metrics.insert(
        "useful_non_generic_task_summary".into(),
        metric_assessment(
            reviewed
                .iter()
                .filter(|(_, path)| {
                    field_passes(path, "answer_composition")
                        && !matches!(
                            path.answer_class.as_str(),
                            "generic_non_answer" | "precise_abstention"
                        )
                })
                .count(),
            reviewed.len(),
            0.90,
            true,
        ),
    );
    for (name, field, threshold) in [
        ("task_object_accuracy", "task_object", 0.88),
        ("execution_state_accuracy", "lifecycle_state", 0.90),
    ] {
        metrics.insert(
            name.into(),
            metric_assessment(
                reviewed
                    .iter()
                    .filter(|(_, path)| field_passes(path, field))
                    .count(),
                reviewed.len(),
                threshold,
                true,
            ),
        );
    }
    let next_claims = reviewed
        .iter()
        .filter(|(_, path)| path.next_action_claim_present)
        .copied()
        .collect::<Vec<_>>();
    let next_labeled = reviewed
        .iter()
        .filter(|(_, path)| path.next_action_label_present)
        .copied()
        .collect::<Vec<_>>();
    metrics.insert(
        "supported_next_action_precision".into(),
        metric_assessment(
            next_claims
                .iter()
                .filter(|(_, path)| field_passes(path, "next_action"))
                .count(),
            next_claims.len(),
            0.90,
            true,
        ),
    );
    metrics.insert(
        "supported_next_action_coverage".into(),
        metric_assessment(
            next_labeled
                .iter()
                .filter(|(_, path)| path.next_action_claim_present)
                .count(),
            next_labeled.len(),
            0.85,
            true,
        ),
    );
    let target_claims = reviewed
        .iter()
        .filter(|(_, path)| path.return_target_claim_present)
        .copied()
        .collect::<Vec<_>>();
    metrics.insert(
        "return_target_precision".into(),
        metric_assessment(
            target_claims
                .iter()
                .filter(|(_, path)| field_passes(path, "target_resolution"))
                .count(),
            target_claims.len(),
            0.98,
            true,
        ),
    );
    metrics.insert(
        "unsupported_specific_claim_rate".into(),
        metric_assessment(
            evaluable
                .iter()
                .filter(|(_, path)| path.unsupported_claim_count > 0)
                .count(),
            evaluable.len(),
            0.01,
            false,
        ),
    );
    metrics.insert(
        "stronger_manual_result_downgraded".into(),
        metric_assessment(
            manual_downgrade_count,
            manual_downgrade_evaluable_count,
            0.0,
            false,
        ),
    );
    let unseen = reviewed
        .iter()
        .filter(|(case, _)| {
            case.partition == TaskTruthPartitionV2::LockedHoldout
                && !development_buckets.contains(case.application_identity_bucket.as_str())
        })
        .copied()
        .collect::<Vec<_>>();
    metrics.insert(
        "unseen_application_useful_summary".into(),
        metric_assessment(
            unseen
                .iter()
                .filter(|(_, path)| field_passes(path, "answer_composition"))
                .count(),
            unseen.len(),
            0.80,
            true,
        ),
    );
    let useful_rated = reviewed
        .iter()
        .filter(|(_, path)| path.human_immediately_useful.is_some())
        .copied()
        .collect::<Vec<_>>();
    metrics.insert(
        "human_immediately_useful".into(),
        metric_assessment(
            useful_rated
                .iter()
                .filter(|(_, path)| path.human_immediately_useful == Some(true))
                .count(),
            useful_rated.len(),
            0.85,
            true,
        ),
    );
    let critical = reviewed
        .iter()
        .filter(|(_, path)| path.critical_local_solvable)
        .copied()
        .collect::<Vec<_>>();
    metrics.insert(
        "model_on_off_unexplained_task_disagreement".into(),
        metric_assessment(
            critical
                .iter()
                .filter(|(_, path)| path.model_on_off_disagreement_unexplained)
                .count(),
            critical.len(),
            0.0,
            false,
        ),
    );
    let abstention = reviewed
        .iter()
        .filter(|(case, _)| case.requires_task_selection_abstention)
        .copied()
        .collect::<Vec<_>>();
    metrics.insert(
        "precise_abstention".into(),
        metric_assessment(
            abstention
                .iter()
                .filter(|(_, path)| path.answer_class == "precise_abstention")
                .count(),
            abstention.len(),
            0.95,
            true,
        ),
    );

    let mut surfaces = BTreeMap::<String, MetricAssessmentV1>::new();
    for family in REQUIRED_SURFACES {
        let family_cases = evaluable
            .iter()
            .filter(|(case, _)| case.surface_family == family)
            .copied()
            .collect::<Vec<_>>();
        surfaces.insert(
            family.into(),
            metric_assessment(
                family_cases
                    .iter()
                    .filter(|(_, path)| path.answer_class == "wrong_specific_task")
                    .count(),
                family_cases.len(),
                0.05,
                false,
            ),
        );
    }
    let intervals = metrics
        .iter()
        .map(|(name, metric)| {
            (
                name.clone(),
                wilson_interval(metric.numerator, metric.denominator),
            )
        })
        .chain(surfaces.iter().map(|(family, metric)| {
            (
                format!("wrong_primary_task_rate.surface.{family}"),
                wilson_interval(metric.numerator, metric.denominator),
            )
        }))
        .collect();
    (metrics, surfaces, intervals)
}

pub(crate) fn evaluate(
    root: &Path,
    allow_locked_holdout: bool,
) -> Result<TaskTruthReportV1, String> {
    let policy =
        parse_policy(&fs::read(root.join("eval-policy.v1.json")).map_err(|e| e.to_string())?)?;
    let mut corpus = parse_corpus(
        &fs::read(root.join("session-013-family.v2.json")).map_err(|e| e.to_string())?,
    )?;
    if corpus
        .cases
        .iter()
        .any(|case| case.partition == TaskTruthPartitionV2::LockedHoldout)
    {
        return Err("locked holdout cases must be stored in locked-holdout.v2.json".into());
    }
    let holdout_path = root.join("locked-holdout.v2.json");
    let holdout_loaded = allow_locked_holdout && holdout_path.exists();
    if holdout_loaded {
        let holdout = parse_corpus(&fs::read(&holdout_path).map_err(|e| e.to_string())?)?;
        if holdout
            .cases
            .iter()
            .any(|case| case.partition != TaskTruthPartitionV2::LockedHoldout)
        {
            return Err("locked holdout file contains a non-holdout case".into());
        }
        corpus.cases.extend(holdout.cases);
        validate_corpus(&corpus)?;
    }
    let mut corpus_counts = BTreeMap::new();
    let mut partitions = BTreeMap::new();
    let mut human = BTreeMap::new();
    let mut privacy = BTreeMap::new();
    let mut surfaces = BTreeMap::new();
    let mut reviewed_live_surfaces = BTreeMap::new();
    let mut slices = BTreeMap::new();
    let mut reviewed_live_slices = BTreeMap::new();
    let mut cases = Vec::new();
    let mut violations = Vec::new();
    let mut first_hist = BTreeMap::new();
    let mut reviewed_first_hist = BTreeMap::new();
    let mut unsupported = 0;
    let mut wrong = BTreeMap::<String, Vec<String>>::new();
    let mut reviewed_wrong = BTreeMap::<String, Vec<String>>::new();
    let mut path_metrics = BTreeMap::<String, BTreeMap<String, BTreeMap<String, usize>>>::new();
    let mut reviewed_path_metrics =
        BTreeMap::<String, BTreeMap<String, BTreeMap<String, usize>>>::new();
    let mut surface_macro = BTreeMap::<String, BTreeMap<String, BTreeMap<String, usize>>>::new();
    let mut reviewed_surface_macro =
        BTreeMap::<String, BTreeMap<String, BTreeMap<String, usize>>>::new();
    let mut synthetic_macro = BTreeMap::<String, BTreeMap<String, usize>>::new();
    let mut live_count = 0;
    let mut reviewed_live = 0;
    let mut holdout_count = 0;
    let mut path_b_partial_evidence_case_count = 0;
    let mut reviewed_live_path_b_partial_evidence_case_count = 0;
    let mut manual_background_downgrade_count = 0;
    let mut manual_background_downgrade_evaluable_count = 0;
    for case in &corpus.cases {
        let source_kind_label = match case.source_kind {
            SourceKindV2::LiveRedacted => "live_redacted",
            SourceKindV2::SyntheticCounterfactual => "synthetic_counterfactual",
        };
        *corpus_counts.entry(source_kind_label.into()).or_insert(0) += 1;
        *partitions
            .entry(format!("{:?}", case.partition).to_ascii_lowercase())
            .or_insert(0) += 1;
        *human
            .entry(format!("{:?}", case.label_review.status).to_ascii_lowercase())
            .or_insert(0) += 1;
        *privacy
            .entry(format!("{:?}", case.privacy_review.status).to_ascii_lowercase())
            .or_insert(0) += 1;
        *surfaces.entry(case.surface_family.clone()).or_insert(0) += 1;
        if case.source_kind == SourceKindV2::LiveRedacted {
            live_count += 1;
        }
        let release_eligible = case.source_kind == SourceKindV2::LiveRedacted
            && case.privacy_review.status == ReviewStatusV2::Approved
            && case.label_review.independently_human_reviewed
            && case.label_review.status == ReviewStatusV2::Approved
            && case.adjudication.is_some();
        if release_eligible {
            reviewed_live += 1;
            *reviewed_live_surfaces
                .entry(case.surface_family.clone())
                .or_insert(0) += 1;
        }
        if case.partition == TaskTruthPartitionV2::LockedHoldout {
            if release_eligible {
                holdout_count += 1;
            }
            if !allow_locked_holdout {
                continue;
            }
        }
        for (name, enabled) in [
            (
                "interruption_resumption",
                case.interruption_resumption_sequence,
            ),
            (
                "ambiguous_or_privacy_blocked",
                case.ambiguous_or_privacy_blocked,
            ),
            (
                "waiting_on_agent_or_application",
                case.waiting_on_agent_or_application,
            ),
            ("completed_vs_new_task", case.completed_vs_new_task_boundary),
        ] {
            if enabled {
                *slices.entry(name.into()).or_insert(0) += 1;
                if release_eligible {
                    *reviewed_live_slices.entry(name.into()).or_insert(0) += 1;
                }
            }
        }
        let a = case.recorded_outputs.path_a_legacy_p6.clone();
        let b = replay_path_b(case)?;
        if !b.unsupported_evidence_groups.is_empty() {
            path_b_partial_evidence_case_count += 1;
            if release_eligible {
                reviewed_live_path_b_partial_evidence_case_count += 1;
            }
        }
        let c = replay_path_c(case)?;
        let second_b = replay_path_b(case)?;
        let second_c = replay_path_c(case)?;
        let deterministic = b.checkpoints == second_b.checkpoints
            && b.task_identity == second_b.task_identity
            && c.checkpoints == second_c.checkpoints
            && c.task_identity == second_c.task_identity;
        if let Some(downgraded) = case
            .recorded_outputs
            .path_a_legacy_p6
            .checkpoints
            .get("stronger_manual_result_downgraded")
            .and_then(Value::as_bool)
        {
            manual_background_downgrade_evaluable_count += 1;
            if downgraded {
                manual_background_downgrade_count += 1;
            }
        }
        let mut paths = BTreeMap::new();
        for (name, output) in [
            ("path_a_legacy_p6", a),
            ("path_b_causally_repaired", b),
            ("path_c_task_truth_shadow", c),
        ] {
            let result = evaluate_path(
                &output,
                case.adjudication
                    .as_ref()
                    .filter(|_| case.label_review.status == ReviewStatusV2::Approved),
                case,
            );
            unsupported += result.unsupported_claim_count;
            if result.answer_class == "wrong_specific_task" {
                wrong
                    .entry(name.into())
                    .or_default()
                    .push(case.case_id.clone());
                if release_eligible {
                    reviewed_wrong
                        .entry(name.into())
                        .or_default()
                        .push(case.case_id.clone());
                }
            }
            if let Some(first) = &result.first_divergence {
                *first_hist.entry(first.clone()).or_insert(0) += 1;
                if release_eligible {
                    *reviewed_first_hist.entry(first.clone()).or_insert(0) += 1;
                }
            }
            for (field, status) in &result.field_results {
                *path_metrics
                    .entry(name.into())
                    .or_default()
                    .entry(field.clone())
                    .or_default()
                    .entry(status.clone())
                    .or_insert(0) += 1;
                if release_eligible {
                    *reviewed_path_metrics
                        .entry(name.into())
                        .or_default()
                        .entry(field.clone())
                        .or_default()
                        .entry(status.clone())
                        .or_insert(0) += 1;
                }
                *surface_macro
                    .entry(case.surface_family.clone())
                    .or_default()
                    .entry(name.into())
                    .or_default()
                    .entry(status.clone())
                    .or_insert(0) += 1;
                if release_eligible {
                    *reviewed_surface_macro
                        .entry(case.surface_family.clone())
                        .or_default()
                        .entry(name.into())
                        .or_default()
                        .entry(status.clone())
                        .or_insert(0) += 1;
                }
                if case.source_kind == SourceKindV2::SyntheticCounterfactual {
                    *synthetic_macro
                        .entry(name.into())
                        .or_default()
                        .entry(status.clone())
                        .or_insert(0) += 1;
                }
            }
            paths.insert(name.into(), result);
        }
        cases.push(TaskTruthCaseResultV1 {
            case_id: case.case_id.clone(),
            source_kind: case.source_kind,
            release_eligible,
            requires_task_selection_abstention: case.adjudication.as_ref().is_some_and(|label| {
                label.resolution == "ambiguous"
                    || label
                        .required_abstention_fields
                        .iter()
                        .any(|field| field == "task_selection")
            }),
            application_identity_bucket: case.application_identity_bucket.clone(),
            surface_family: case.surface_family.clone(),
            partition: case.partition,
            human_label_status: if case.label_review.status == ReviewStatusV2::Approved {
                "available".into()
            } else {
                "missing_human_evidence".into()
            },
            paths,
            deterministic_replay_match: deterministic,
        });
    }
    corpus_counts.insert("live_release_denominator".into(), live_count);
    let frozen_policy_metric_results = assess_frozen_policy_metrics(
        &policy,
        &cases,
        manual_background_downgrade_count,
        manual_background_downgrade_evaluable_count,
    );
    let (tt2_05_metric_results, tt2_05_surface_wrong_task_results, tt2_05_confidence_intervals) =
        assess_tt2_05_metrics(
            &cases,
            manual_background_downgrade_count,
            manual_background_downgrade_evaluable_count,
        );
    for (metric, result) in &frozen_policy_metric_results {
        if !result.passed {
            violations.push(format!("frozen_metric_{metric}_{}", result.status));
        }
    }
    for (metric, result) in &tt2_05_metric_results {
        if !result.passed {
            violations.push(format!("tt2_05_metric_{metric}_{}", result.status));
        }
    }
    for (surface, result) in &tt2_05_surface_wrong_task_results {
        if !result.passed {
            violations.push(format!(
                "tt2_05_surface_wrong_task_{surface}_{}",
                result.status
            ));
        }
    }
    if reviewed_live < REQUIRED_LIVE_CASES {
        violations.push(format!("independently_reviewed_live_cases_requires_{REQUIRED_LIVE_CASES}_found_{reviewed_live}"));
    }
    if holdout_count < REQUIRED_HOLDOUT_CASES {
        violations.push(format!(
            "locked_holdout_requires_{REQUIRED_HOLDOUT_CASES}_found_{holdout_count}"
        ));
    }
    for surface in REQUIRED_SURFACES {
        let count = reviewed_live_surfaces.get(surface).copied().unwrap_or(0);
        if count < REQUIRED_SURFACE_CASES {
            violations.push(format!(
                "surface_{surface}_requires_{REQUIRED_SURFACE_CASES}_found_{count}"
            ));
        }
    }
    for (slice, minimum) in [
        ("interruption_resumption", REQUIRED_INTERRUPTION_SEQUENCES),
        (
            "ambiguous_or_privacy_blocked",
            REQUIRED_AMBIGUOUS_OR_BLOCKED,
        ),
        ("waiting_on_agent_or_application", REQUIRED_WAITING),
        ("completed_vs_new_task", REQUIRED_BOUNDARIES),
    ] {
        let count = reviewed_live_slices.get(slice).copied().unwrap_or(0);
        if count < minimum {
            violations.push(format!("slice_{slice}_requires_{minimum}_found_{count}"));
        }
    }
    if corpus.cases.iter().any(|case| {
        case.source_kind == SourceKindV2::LiveRedacted
            && (case.label_review.status != ReviewStatusV2::Approved
                || !case.label_review.independently_human_reviewed
                || case.adjudication.is_none())
    }) {
        violations.push("missing_independent_human_evidence".into());
    }
    if corpus
        .cases
        .iter()
        .any(|case| case.privacy_review.status != ReviewStatusV2::Approved)
    {
        violations.push("missing_privacy_approval".into());
    }
    if !allow_locked_holdout {
        violations.push("locked_holdout_not_accessed".into());
    }
    if cases.iter().any(|case| {
        case.paths
            .get("path_c_task_truth_shadow")
            .is_none_or(|path| path.status == PathStatusV2::NotImplemented)
    }) {
        violations.push("path_c_not_implemented".into());
    }
    if cases.iter().any(|case| !case.deterministic_replay_match) {
        violations.push("deterministic_replay_mismatch".into());
    }
    if reviewed_live_path_b_partial_evidence_case_count > 0 {
        violations.push(format!(
            "path_b_partial_evidence_coverage_for_{reviewed_live_path_b_partial_evidence_case_count}_reviewed_live_cases"
        ));
    }
    if manual_background_downgrade_count > 0 {
        violations.push(format!(
            "stronger_manual_result_downgraded_{manual_background_downgrade_count}_times"
        ));
    }
    if manual_background_downgrade_evaluable_count == 0 {
        violations.push("manual_background_downgrade_adoption_evidence_missing".into());
    }
    let mut worst_surface_family_slice_by_path = BTreeMap::new();
    for path_name in [
        "path_a_legacy_p6",
        "path_b_causally_repaired",
        "path_c_task_truth_shadow",
    ] {
        let worst = reviewed_surface_macro
            .iter()
            .filter_map(|(family, paths)| {
                let statuses = paths.get(path_name)?;
                let mut evaluated = 0usize;
                let mut failures = 0usize;
                for (status, count) in statuses {
                    if matches!(
                        status.as_str(),
                        "match"
                            | "acceptable_alternative"
                            | "precise_abstention"
                            | "mismatch"
                            | "unsupported_claim"
                            | "generic_non_answer"
                    ) {
                        evaluated += count;
                    }
                    if matches!(
                        status.as_str(),
                        "mismatch" | "unsupported_claim" | "generic_non_answer"
                    ) {
                        failures += count;
                    }
                }
                (evaluated > 0).then_some((family, failures as f64 / evaluated as f64))
            })
            .max_by(|left, right| left.1.total_cmp(&right.1))
            .map(|(family, _)| family.clone());
        worst_surface_family_slice_by_path.insert(path_name.to_string(), worst);
    }
    let worst = worst_surface_family_slice_by_path
        .get("path_c_task_truth_shadow")
        .cloned()
        .flatten();
    let mut path_macro_results = BTreeMap::new();
    for (path, fields) in &path_metrics {
        let totals = path_macro_results
            .entry(path.clone())
            .or_insert_with(BTreeMap::new);
        for statuses in fields.values() {
            for (status, count) in statuses {
                *totals.entry(status.clone()).or_insert(0) += count;
            }
        }
    }
    let mut reviewed_live_path_macro_results = BTreeMap::new();
    for (path, fields) in &reviewed_path_metrics {
        let totals = reviewed_live_path_macro_results
            .entry(path.clone())
            .or_insert_with(BTreeMap::new);
        for statuses in fields.values() {
            for (status, count) in statuses {
                *totals.entry(status.clone()).or_insert(0) += count;
            }
        }
    }
    Ok(TaskTruthReportV1 {
        schema: TASK_TRUTH_REPORT_SCHEMA_V1.into(),
        policy_version: policy.policy_version,
        corpus_counts,
        partition_counts: partitions,
        human_review_counts: human,
        privacy_review_counts: privacy,
        surface_denominators: surfaces,
        reviewed_live_surface_denominators: reviewed_live_surfaces,
        slice_denominators: slices,
        reviewed_live_slice_denominators: reviewed_live_slices,
        per_path_field_metrics: path_metrics,
        reviewed_live_per_path_field_metrics: reviewed_path_metrics,
        path_macro_results,
        reviewed_live_path_macro_results,
        synthetic_counterfactual_path_macro_results: synthetic_macro,
        surface_family_macro_results: surface_macro,
        reviewed_live_surface_family_macro_results: reviewed_surface_macro,
        worst_surface_family_slice_by_path,
        frozen_policy_metric_results,
        tt2_05_metric_results,
        tt2_05_surface_wrong_task_results,
        tt2_05_confidence_intervals,
        wrong_task_examples: wrong,
        reviewed_live_wrong_task_examples: reviewed_wrong,
        first_divergence_histogram: first_hist,
        reviewed_live_first_divergence_histogram: reviewed_first_hist,
        worst_surface_family_slice: worst,
        manual_background_downgrade_count,
        manual_background_downgrade_evaluable_count,
        unsupported_claim_count: unsupported,
        path_b_partial_evidence_case_count,
        reviewed_live_path_b_partial_evidence_case_count,
        locked_holdout_access_status: if holdout_loaded {
            "explicitly_allowed_and_loaded".into()
        } else if allow_locked_holdout {
            "explicitly_allowed_but_no_holdout_file".into()
        } else {
            "not_accessed".into()
        },
        release_gate_passed: violations.is_empty(),
        release_gate_violations: violations,
        cases,
    })
}

pub(crate) fn write_report(report: &TaskTruthReportV1, output: &Path) -> Result<(), String> {
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(
        output,
        serde_json::to_vec_pretty(report).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/continue_accuracy/task_truth_v2")
    }

    fn private_test_path(name: &str) -> PathBuf {
        let private_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("private_task_truth_corpus");
        fs::create_dir_all(&private_root).unwrap();
        private_root.join(format!("{name}-{}.json", std::process::id()))
    }

    fn label_text(value: &str) -> TaskTruthTextV2 {
        TaskTruthTextV2 {
            text: value.into(),
            provenance: TextProvenanceV2::HumanParaphrase,
            content_hash: sha256_hex(value.as_bytes()),
            copied_from_private_text: false,
        }
    }

    fn reviewed_label() -> HumanAdjudicationV2 {
        HumanAdjudicationV2 {
            resolution: "resolved".into(),
            expected_observation_status: "full_coverage".into(),
            expected_causal_association: true,
            expected_control_as_task: false,
            expected_authorship: true,
            primary_task_summary: label_text("Implement live corpus and shadow evaluation"),
            task_object: label_text("live corpus and shadow evaluation"),
            user_goal: label_text("Measure the real current task"),
            last_meaningful_progress: label_text("Causal containment is complete"),
            unfinished_step: label_text("Build the measurement authority"),
            execution_state: "active".into(),
            current_actor: "assistant_or_agent".into(),
            waiting_on: "agent".into(),
            next_supported_action: label_text("Run the shadow evaluator"),
            where_identity: label_text("agent chat"),
            relation_to_prior_task: "new_task_after_completion".into(),
            support_detour_surfaces: vec![],
            direct_return_anchor: None,
            acceptable_alternative_hypotheses: vec![],
            required_abstention_fields: vec!["target_resolution".into()],
            forbidden_claims: vec![label_text("Approve for me")],
            immediately_useful: true,
            reviewer_notes: label_text("Blinded test label"),
        }
    }

    #[test]
    fn session_013_family_has_five_distinct_live_boundaries() {
        let corpus =
            parse_corpus(&fs::read(root().join("session-013-family.v2.json")).unwrap()).unwrap();
        assert_eq!(corpus.cases.len(), 5);
        assert!(corpus
            .cases
            .iter()
            .all(|case| case.source_kind == SourceKindV2::LiveRedacted));
        let hashes = corpus
            .cases
            .iter()
            .map(|case| stable_json_sha256(&case.source).unwrap())
            .collect::<BTreeSet<_>>();
        assert_eq!(hashes.len(), 5);
        assert!(corpus.cases.iter().all(|case| case
            .recorded_outputs
            .path_c_task_truth_shadow
            .status
            == PathStatusV2::NotImplemented));
    }

    #[test]
    fn missing_labels_holdouts_and_shadow_keep_gate_false() {
        let report = evaluate(&root(), false).unwrap();
        assert!(!report.release_gate_passed);
        assert_eq!(report.cases.len(), 5);
        assert!(report
            .release_gate_violations
            .contains(&"missing_independent_human_evidence".into()));
        assert!(!report
            .release_gate_violations
            .contains(&"path_c_not_implemented".into()));
        assert!(report
            .cases
            .iter()
            .all(|case| case.deterministic_replay_match));
        assert!(report.cases.iter().all(|case| {
            let fingerprints = case
                .paths
                .values()
                .map(|path| path.source_fingerprint.as_str())
                .collect::<BTreeSet<_>>();
            fingerprints.len() == 1
        }));
        assert!(report
            .frozen_policy_metric_results
            .values()
            .all(|metric| metric.denominator == 0 && !metric.passed));
        assert!(report
            .worst_surface_family_slice_by_path
            .values()
            .all(Option::is_none));
    }

    #[test]
    fn session_013_produces_v2_packet_and_shadow_snapshot_checkpoints() {
        let corpus =
            parse_corpus(&fs::read(root().join("session-013-family.v2.json")).unwrap()).unwrap();
        for case in &corpus.cases {
            let output = replay_path_c(case).unwrap();
            assert_eq!(output.status, PathStatusV2::Implemented);
            assert_eq!(
                output.checkpoints["observation_construction"]["schema"],
                json!(observation_packet::OBSERVATION_PACKET_SCHEMA_V2)
            );
            assert_eq!(
                output.checkpoints["task_snapshot"]["schema"],
                json!(task_snapshot::TASK_SNAPSHOT_SCHEMA_V2)
            );
            assert!(output.checkpoints["observation_construction"]["packet_id"]
                .as_str()
                .is_some());
            assert_eq!(
                output.checkpoints["multimodal_resolution"]["status"],
                json!(if case.decision_mode == "manual" {
                    if case.ambiguous_or_privacy_blocked {
                        "ambiguous_fixture"
                    } else {
                        "resolved_fixture"
                    }
                } else {
                    "not_requested_background"
                })
            );
            assert_ne!(
                output.task_identity.as_ref().map(|task| task.text.as_str()),
                Some("Approve for me")
            );
        }
    }

    #[test]
    fn source_labels_and_sensitive_text_are_rejected() {
        assert!(lint_sensitive("person@example.com").is_err());
        assert!(lint_sensitive("/Users/person/private.txt").is_err());
        assert!(lint_sensitive("https://example.test/path?token=x").is_err());
        assert!(lint_sensitive("12345678901").is_err());
        assert!(!semantic_text_matches("anything", ""));
        assert!(!semantic_text_matches("mapping state", "app"));
        assert!(!semantic_text_matches("decode output", "code"));
    }

    #[test]
    fn labeled_comparison_detects_wrong_control_task_and_forbidden_claim() {
        let corpus =
            parse_corpus(&fs::read(root().join("session-013-family.v2.json")).unwrap()).unwrap();
        let case = &corpus.cases[0];
        let result = evaluate_path(
            &case.recorded_outputs.path_a_legacy_p6,
            Some(&reviewed_label()),
            case,
        );
        assert_eq!(result.answer_class, "wrong_specific_task");
        assert_eq!(result.field_results["task_selection"], "mismatch");
        assert!(result.control_navigation_as_task);
        assert!(result.unsupported_claim_count > 0);
        assert_eq!(result.first_divergence.as_deref(), Some("task_selection"));
        let mut confident = case.recorded_outputs.path_a_legacy_p6.clone();
        confident.claim_confidence = Some(0.9);
        let confident_result = evaluate_path(&confident, Some(&reviewed_label()), case);
        assert_eq!(
            confident_result.answer_class,
            "unsupported_confident_answer"
        );
        let repaired = replay_path_b(case).unwrap();
        let repaired_result = evaluate_path(&repaired, Some(&reviewed_label()), case);
        assert_eq!(
            repaired_result.first_divergence.as_deref(),
            Some("observation_construction")
        );
    }

    #[test]
    fn builder_dry_run_is_default_deny_and_writes_nothing() {
        let input_path = private_test_path("builder-dry-run-input");
        let output_path = private_test_path("builder-dry-run-output");
        let _ = fs::remove_file(&output_path);
        let input = BuilderInputV1 {
            schema: TASK_TRUTH_BUILDER_SCHEMA_V1.into(),
            case_id: "builder_dry_run".into(),
            source_root: "private_task_truth_corpus/session-local".into(),
            records: vec![BuilderRecordV1 {
                evidence_group: "frame".into(),
                record: SourceRecordV2 {
                    record_id: "frame-1".into(),
                    frame_id: Some("frame-1".into()),
                    observed_at_ms: 1,
                    parent_record_id: None,
                    native_role: Some("active_window".into()),
                    native_subrole: None,
                    source_order: Some(0),
                    bounds: None,
                    text: None,
                    actions: vec![],
                    focused: Some(true),
                    editable: None,
                    owner_hash: None,
                    metadata: BTreeMap::from([("privacy_status".into(), json!("private"))]),
                },
            }],
        };
        fs::write(&input_path, serde_json::to_vec(&input).unwrap()).unwrap();
        let candidate = build_review_candidate(&input_path, Some(&output_path), true).unwrap();
        assert!(candidate.privacy_manifest.dry_run);
        assert!(candidate.privacy_manifest.review_required);
        assert_eq!(candidate.review_status, ReviewStatusV2::Pending);
        assert_eq!(candidate.retained_records.len(), 1);
        assert_eq!(candidate.retained_records[0].evidence_group, "frame");
        assert!(!output_path.exists());
        let _ = fs::remove_file(input_path);
    }

    #[test]
    fn application_layout_workflow_bucket_cannot_cross_partitions() {
        let mut corpus =
            parse_corpus(&fs::read(root().join("session-013-family.v2.json")).unwrap()).unwrap();
        corpus.cases[1].layout_workflow_bucket = corpus.cases[0].layout_workflow_bucket.clone();
        corpus.cases[1].partition = TaskTruthPartitionV2::LockedHoldout;
        let error = validate_corpus(&corpus).unwrap_err();
        assert!(error.contains("application-level partition leakage"));
    }

    #[test]
    fn builder_rejects_non_allowlisted_retained_fields() {
        let base = private_test_path("builder-default-deny");
        let input = BuilderInputV1 {
            schema: TASK_TRUTH_BUILDER_SCHEMA_V1.into(),
            case_id: "builder_default_deny".into(),
            source_root: "private_task_truth_corpus/session-local".into(),
            records: vec![BuilderRecordV1 {
                evidence_group: "ui_event".into(),
                record: SourceRecordV2 {
                    record_id: "event-1".into(),
                    frame_id: None,
                    observed_at_ms: 1,
                    parent_record_id: None,
                    native_role: Some("event".into()),
                    native_subrole: None,
                    source_order: None,
                    bounds: None,
                    text: None,
                    actions: vec![],
                    focused: None,
                    editable: None,
                    owner_hash: None,
                    metadata: BTreeMap::from([("raw_text".into(), json!("must not survive"))]),
                },
            }],
        };
        fs::write(&base, serde_json::to_vec(&input).unwrap()).unwrap();
        let error = build_review_candidate(&base, None, true).unwrap_err();
        assert!(error.contains("default-deny rejected fields"));
        let _ = fs::remove_file(base);
    }

    #[test]
    fn tt2_05_next_action_precision_and_coverage_are_distinct_gates() {
        fn case(id: &str, claim_present: bool, status: &str) -> TaskTruthCaseResultV1 {
            let path = PathCaseResultV1 {
                status: PathStatusV2::Implemented,
                source_fingerprint: format!("fingerprint-{id}"),
                recorded_private_output_hash: None,
                first_divergence: None,
                field_results: BTreeMap::from([
                    ("next_action".into(), status.into()),
                    ("answer_composition".into(), "match".into()),
                    ("task_object".into(), "match".into()),
                    ("lifecycle_state".into(), "match".into()),
                    ("target_resolution".into(), "match".into()),
                ]),
                task_identity: Some("Implement the release evaluator".into()),
                answer_class: "correct_task".into(),
                unsupported_claim_count: 0,
                control_navigation_as_task: false,
                human_immediately_useful: Some(true),
                next_action_label_present: true,
                next_action_claim_present: claim_present,
                return_target_claim_present: true,
                critical_local_solvable: true,
                model_on_off_disagreement_unexplained: false,
                consumed_evidence_groups: Vec::new(),
                unsupported_evidence_groups: Vec::new(),
                claim_confidence: Some(0.9),
            };
            TaskTruthCaseResultV1 {
                case_id: id.into(),
                source_kind: SourceKindV2::LiveRedacted,
                release_eligible: true,
                requires_task_selection_abstention: false,
                application_identity_bucket: "bucket".into(),
                surface_family: "agent_chat".into(),
                partition: TaskTruthPartitionV2::Development,
                human_label_status: "available".into(),
                paths: BTreeMap::from([("path_c_task_truth_shadow".into(), path)]),
                deterministic_replay_match: true,
            }
        }
        let cases = vec![
            case("with-action", true, "match"),
            case("missing-action", false, "missing"),
        ];
        let (metrics, _, intervals) = assess_tt2_05_metrics(&cases, 0, 1);
        let precision = &metrics["supported_next_action_precision"];
        let coverage = &metrics["supported_next_action_coverage"];
        assert_eq!((precision.numerator, precision.denominator), (1, 1));
        assert!(precision.passed);
        assert_eq!((coverage.numerator, coverage.denominator), (1, 2));
        assert!(!coverage.passed);
        assert!(intervals["supported_next_action_coverage"].lower.is_some());
    }

    #[test]
    fn critical_model_on_off_disagreement_requires_reviewed_explanation() {
        let mut corpus =
            parse_corpus(&fs::read(root().join("session-013-family.v2.json")).unwrap()).unwrap();
        let case = &mut corpus.cases[0];
        case.model_parity = Some(ModelParityReviewV1 {
            critical_local_solvable: true,
            model_on_task_identity: Some(label_text("Implement the release evaluator")),
            model_off_task_identity: Some(label_text("Review an older browser tab")),
            disagreement_explained: true,
            explanation_independently_reviewed: false,
        });
        let output = replay_path_c(case).unwrap();
        assert_eq!(
            output
                .checkpoints
                .get("model_on_off_task_disagreement_unexplained")
                .and_then(Value::as_bool),
            Some(true)
        );
    }
}
